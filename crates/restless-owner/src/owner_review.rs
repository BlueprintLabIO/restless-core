//! Isolated review origins: tickets that open a Runtime-served outcome under its own origin, and the read-only proxy behind them.

use super::*;

pub(crate) const REVIEW_TTL: Duration = Duration::from_secs(30 * 60);

/// Where a plane serves review tickets on its own host: `/review/{ticket}/{path}`.
pub(crate) const REVIEW_PATH_PREFIX: &str = "/review/";

/// The route for every ticket path, the empty `{path}` included. A site recorded at its root
/// (`http://127.0.0.1:<port>/`) becomes `/review/{ticket}/`, which `/review/{ticket}/{*path}` never
/// matched (a wildcard needs one character), so the root fell through to the cockpit's own 404.
pub(crate) const REVIEW_PATH_ROUTE: &str = "/review/{*rest}";
/// Set by the one-address edge when it forwards a review subdomain to the plane. A browser cannot
/// add it to a navigation, and the plane grants no cross-origin request that carries it.
pub(crate) const REVIEW_HOST_HEADER: &str = "x-restless-review-host";

/// Where the browser sees a review served from, which decides how it is isolated.
pub(crate) enum Isolation {
    /// Its own origin, such as a local `<ticket>.localhost` host: nothing else lives there.
    OwnOrigin,
    /// The plane's own host, shared with the cockpit: sandboxed into an opaque origin.
    PlaneHost,
    /// Its own `<ticket>-<plane>-review` subdomain through the edge: its own origin, so a site's
    /// root-relative assets load, framed only by the cockpit's public origin.
    ReviewHost { cockpit: String },
}

/// A hosted plane `owner-<hex>.<domain>` gives each review ticket its own subdomain
/// `<ticket>-<hex>-review.<domain>`, one label deep so the zone's wildcard certificate covers it.
pub(crate) fn review_subdomain(plane_host: &str, ticket: &str) -> Option<String> {
    let (label, domain) = plane_host.split_once('.')?;
    let plane = label.strip_prefix("owner-")?;
    let hex = |value: &str| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_hexdigit());
    (hex(plane) && hex(ticket) && !domain.is_empty()).then(|| format!("{ticket}-{plane}-review.{domain}"))
}

/// The review subdomain for this ticket, when this plane sits behind the one-address edge.
fn review_host_for(state: &OwnerState, ticket: &str) -> Option<String> {
    if std::env::var("RESTLESS_REVIEW_ON_PLANE_HOST").as_deref() == Ok("1") {
        return None;
    }
    let network = state.entry.network()?;
    network.public_host()?;
    review_subdomain(network.host(), ticket)
}

/// A request for a ticket served on the plane's own host. The ticket is its whole authority, as
/// on the separate review origin, so it bypasses browser sessions.
pub(crate) fn is_review_path(path: &str) -> bool {
    path.starts_with(REVIEW_PATH_PREFIX)
}

/// Where a review ticket is served, and the Host it must arrive on (empty: the plane-host path).
///
/// A plane reached over the network cannot hand a remote browser its loopback review origin
/// (`http://{ticket}.localhost:<port>`): that URL resolves on the owner's own machine, and an https
/// cockpit refuses it as mixed content, so every review and Library file opened blank on Cloud.
/// Such a plane serves the ticket on its own host under `/review/{ticket}/`, with a sandbox policy
/// that gives the content an opaque origin: company code still never shares the cockpit's origin,
/// cookies or storage. A plane whose browser is on the same machine keeps its separate origin,
/// which also serves root-relative assets and live services intact. `RESTLESS_REVIEW_ON_PLANE_HOST=1`
/// opts a local plane into the plane-host path when it is reached from another device.
pub(crate) fn review_location(
    state: &OwnerState,
    ticket: &str,
    path_and_query: &str,
) -> Result<(String, String)> {
    if review_on_plane_host(state) {
        let path = path_and_query.trim_start_matches('/');
        if let Some(host) = review_host_for(state, ticket) {
            return Ok((format!("https://{host}/{path}"), String::new()));
        }
        return Ok((format!("{REVIEW_PATH_PREFIX}{ticket}/{path}"), String::new()));
    }
    materialize_review_url(&state.review_public_url, ticket, path_and_query)
}

fn review_on_plane_host(state: &OwnerState) -> bool {
    if std::env::var("RESTLESS_REVIEW_ON_PLANE_HOST").as_deref() == Ok("1") {
        return true;
    }
    let loopback = url::Url::parse(&state.review_public_url.replace("{ticket}", "ticket"))
        .ok()
        .and_then(|url| {
            url.host_str()
                .map(|host| host == "localhost" || host.ends_with(".localhost"))
        })
        .unwrap_or(true);
    state.entry.network().is_some() && loopback
}

#[derive(Clone)]
pub(crate) struct ReviewSession {
    pub(crate) company: String,
    pub(crate) generation: String,
    pub(crate) item_id: String,
    pub(crate) source: ReviewSource,
    pub(crate) expected_host: String,
    pub(crate) expires_at: SystemTime,
}

/// Where an isolated review origin reads from. Both are ordinary Runtime truth
/// observed read-only through the owner gateway; neither copies the outcome.
#[derive(Clone)]
pub(crate) enum ReviewSource {
    /// A project service already listening inside the company computer.
    Service { port: u16 },
    /// One produced file and the directory it needs, e.g. a rendered page with
    /// its own stylesheet and images (S19-T5).
    Files { root: PathBuf, entry: String },
}

#[derive(Debug, Deserialize)]
pub(crate) struct OwnerReviewInput {
    pub(crate) decision: String,
    #[serde(default)]
    pub(crate) feedback: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReviewTicketRequest {
    pub(crate) item_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReviewTicketResponse {
    pub(crate) review_url: String,
    pub(crate) expires_in_seconds: u64,
}

pub(crate) async fn review_outcome(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, handoff)): AxumPath<(String, Uuid)>,
    Json(input): Json<OwnerReviewInput>,
) -> impl IntoResponse {
    let decision = match input.decision.trim() {
        "accept" => restless_orgintel::OwnerReviewDecision::Accepted,
        "request_changes" => restless_orgintel::OwnerReviewDecision::ChangesRequested,
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "review",
                "decision must be accept or request_changes",
            )
        }
    };
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    match org
        // The owner reviews what is theirs; the person who owns the Work's team reviews what
        // Exec prepared for them. OrgIntel checks whose review it is.
        .decide_review_as(handoff, handoff_resolver(&principal), decision, &input.feedback)
        .await
    {
        Ok(()) => Json(serde_json::json!({
            "handoff_id": handoff,
            "decision": input.decision,
            "recorded": true,
        }))
        .into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "review", error.plain_message()),
    }
}

/// The owner opening a file is real demand: a computer that went to sleep while idle wakes for
/// it, as it does for a message to the Exec. One the owner stopped stays stopped, and says so in
/// a sentence rather than as Docker's error.
pub(crate) async fn wake_for_owner_read(
    state: &OwnerState,
    company: &str,
    reason: &str,
) -> std::result::Result<(), Response<Body>> {
    if restless_engine::runtime_sleep::wake_for(&state.daemon, company, reason).await {
        return Ok(());
    }
    Err(api_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "runtime",
        "The company computer is stopped, so this file cannot be opened. Start it from Company → Computer.",
    ))
}

pub(crate) async fn issue_review_ticket(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<ReviewTicketRequest>,
) -> impl IntoResponse {
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
    };
    // Wake before projecting, so the projection observes the file on a running computer.
    if let Err(refusal) =
        wake_for_owner_read(&state, &company, "the owner opened a review").await
    {
        return refusal;
    }
    let org = state.daemon.orgintel.get(&company).await.ok();
    let view = match attention::project(&config, &state.daemon.authority, org.as_ref()).await {
        Ok(view) => view,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "projection",
                format!("{error:#}"),
            )
        }
    };
    let Some(item) = view.items.iter().find(|item| item.id == input.item_id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "attention",
            "review is no longer outstanding",
        );
    };
    let Some(reference) = item.review_target.as_ref() else {
        return api_error(
            StatusCode::CONFLICT,
            "review",
            "this item has no prepared outcome the cockpit can open",
        );
    };
    // A Markdown or text outcome has no origin to open: the projection carries it, and now that
    // the computer is awake the next projection reads it.
    if reference.kind == "runtime-text" {
        return Json(serde_json::json!({ "review_url": null, "refresh": true })).into_response();
    }
    let current = runtime::generation(&company).await.ok().flatten();
    if current.as_deref() != Some(reference.generation.as_str()) {
        return api_error(
            StatusCode::CONFLICT,
            "runtime",
            "runtime generation changed; refresh the review",
        );
    }
    match open_review_origin(
        &state,
        &company,
        &reference.generation,
        &reference.uri,
        reference.kind == "runtime-file",
        &item.id,
    )
    .await
    {
        Ok(review_url) => Json(ReviewTicketResponse {
            review_url,
            expires_in_seconds: REVIEW_TTL.as_secs(),
        })
        .into_response(),
        Err(refusal) => refusal,
    }
}

/// One Library file to open: a reference Work recorded (`artifact_id`), or any file in the
/// company home by its exact path (`path`), as a link in a message or document names it.
#[derive(Debug, Deserialize)]
pub(crate) struct LibraryTicketRequest {
    #[serde(default)]
    pub(crate) artifact_id: Option<Uuid>,
    #[serde(default)]
    pub(crate) path: Option<String>,
}

/// Open one file, from Library, in the same isolated, read-only origin a review uses (S64-T5).
/// A recorded reference must still be the available version at its locator; a path must name a
/// file beneath `/company` that resolves, through any symlink, inside it. Either way the file is
/// probed before the ticket is issued, exactly as a review's is. Owner only: this route is not
/// among the collaboration families a member may reach.
///
/// A small text file also comes back as text, so the cockpit reads Markdown formatted and a CSV
/// as a table; the ticket still serves Download and Open in a tab.
pub(crate) async fn issue_library_ticket(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<LibraryTicketRequest>,
) -> impl IntoResponse {
    if let Err(error) = runtime::CompanyConfig::load(&state.daemon.root, &company) {
        return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}"));
    }
    // A teammate opens the files of the Work they can see, not any path on the computer.
    if principal.membership_role() != "owner" {
        let visible = match teammate_library(&state, &company, principal.actor_id()).await {
            Ok(visible) => visible,
            Err(refusal) => return refusal,
        };
        let allowed = match (input.artifact_id, input.path.as_deref().map(str::trim)) {
            (Some(artifact_id), None) => visible.ids.contains(&artifact_id),
            (None, Some(path)) => visible.uris.contains(path),
            _ => false,
        };
        if !allowed {
            return api_error(
                StatusCode::NOT_FOUND,
                "library",
                "this file is not one of the Work files you can open",
            );
        }
    }
    let (uri, item_id) = match (input.artifact_id, input.path.as_deref().map(str::trim)) {
        (Some(artifact_id), None) => match recorded_library_file(&state, &company, artifact_id).await
        {
            Ok(uri) => (uri, format!("artifact:{artifact_id}")),
            Err(refusal) => return refusal,
        },
        (None, Some(path)) => {
            if !path.starts_with("/company/") || runtime::runtime_review_file_root(path).is_err() {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "library",
                    "this is not a file in the company home",
                );
            }
            (path.to_string(), format!("path:{path}"))
        }
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "library",
                "name one file: an artifact_id or a path",
            )
        }
    };
    if let Err(refusal) =
        wake_for_owner_read(&state, &company, "the owner opened a Library file").await
    {
        return refusal;
    }
    let Some(generation) = runtime::generation(&company).await.ok().flatten() else {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime",
            "The company computer is still starting. Try again in a moment.",
        );
    };
    let is_file = runtime::runtime_review_file_root(&uri).is_ok();
    if !is_file && runtime::runtime_http_target(&uri).is_err() {
        return api_error(
            StatusCode::CONFLICT,
            "library",
            "this file cannot be shown in the cockpit",
        );
    }
    let review_url = match open_review_origin(&state, &company, &generation, &uri, is_file, &item_id)
        .await
    {
        Ok(review_url) => review_url,
        Err(refusal) => return refusal,
    };
    let mut body = serde_json::json!({
        "review_url": review_url,
        "expires_in_seconds": REVIEW_TTL.as_secs(),
    });
    if let Some(format) = is_file.then(|| runtime::runtime_text_format(&uri)).flatten() {
        // Too large or not UTF-8 still opens: on the review origin, as before.
        if let Ok(text) = runtime::read_runtime_text_file(&company, &uri).await {
            body["text"] = text.into();
            body["format"] = format.into();
        }
    }
    Json(body).into_response()
}

/// The locator of a file Work recorded, if it is still the available version.
/// The Work files a teammate can open: those of the Work their collaboration view shows.
struct TeammateLibrary {
    ids: std::collections::HashSet<Uuid>,
    uris: std::collections::HashSet<String>,
}

async fn teammate_library(
    state: &OwnerState,
    company: &str,
    actor_id: &str,
) -> std::result::Result<TeammateLibrary, Response<Body>> {
    let unavailable = |error: anyhow::Error| {
        api_error(StatusCode::SERVICE_UNAVAILABLE, "orgintel", format!("{error:#}"))
    };
    let org = state.daemon.orgintel.get(company).await.map_err(unavailable)?;
    let graph = org
        .collaborator_work_graph_snapshot(actor_id)
        .await
        .map_err(|error| unavailable(error.into()))?;
    let work: std::collections::HashSet<Uuid> = graph.work.iter().map(|work| work.id).collect();
    let mut library = TeammateLibrary { ids: Default::default(), uris: Default::default() };
    for artifact in graph.artifacts {
        if artifact.work_id.is_some_and(|work_id| work.contains(&work_id))
            && super::member_collaboration_api::member_library_uri(&artifact.uri).is_some()
        {
            library.ids.insert(artifact.id);
            library.uris.insert(artifact.uri);
        }
    }
    Ok(library)
}

async fn recorded_library_file(
    state: &OwnerState,
    company: &str,
    artifact_id: Uuid,
) -> std::result::Result<String, Response<Body>> {
    let org = match state.daemon.orgintel.get(company).await {
        Ok(org) => org,
        Err(error) => {
            return Err(api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            ))
        }
    };
    let artifact = match org.get_artifact_ref(artifact_id).await {
        Ok(Some(artifact)) if artifact.work_id.is_some() => artifact,
        Ok(_) => {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                "library",
                "this file is not one the company's Work recorded",
            ))
        }
        Err(error) => {
            return Err(api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            ))
        }
    };
    if artifact.state != restless_orgintel::ArtifactRefState::Available {
        return Err(api_error(
            StatusCode::CONFLICT,
            "library",
            "this file was replaced by a newer version or is missing",
        ));
    }
    Ok(artifact.uri)
}

#[derive(Debug, Deserialize)]
pub(crate) struct LibraryContentQuery {
    pub(crate) path: String,
}

/// The bytes of one Word, Excel or PowerPoint file in the company home, for the cockpit to render in place
/// (browsers render neither). Owner only, like opening it; bounded like a review read, served
/// as an inert download with no-sniff so the cockpit origin never renders it.
pub(crate) async fn library_file_content(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<LibraryContentQuery>,
) -> Response<Body> {
    let path = query.path.trim();
    if principal.membership_role() != "owner" {
        match teammate_library(&state, &company, principal.actor_id()).await {
            Ok(visible) if visible.uris.contains(path) => {}
            Ok(_) => {
                return api_error(
                    StatusCode::NOT_FOUND,
                    "library",
                    "this file is not one of the Work files you can open",
                )
            }
            Err(refusal) => return refusal,
        }
    }
    let office = std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "docx" | "xlsx" | "pptx"));
    if !office || !path.starts_with("/company/") || runtime::runtime_review_file_root(path).is_err()
    {
        return api_error(
            StatusCode::BAD_REQUEST,
            "library",
            "only a Word, Excel or PowerPoint file in the company home is read here",
        );
    }
    if let Err(error) = runtime::CompanyConfig::load(&state.daemon.root, &company) {
        return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}"));
    }
    if let Err(refusal) =
        wake_for_owner_read(&state, &company, "the owner opened a Library file").await
    {
        return refusal;
    }
    // Streamed, so a large workbook reads as a small one does.
    let file = match runtime::stream_runtime_review_file(
        &company,
        std::path::Path::new(path),
        None,
    )
    .await
    {
        Ok(file) => file,
        Err(error) if error.is::<runtime::ReviewFileTooLarge>() => {
            return api_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "library",
                "This file is larger than the 16 GiB the cockpit can open or download.",
            )
        }
        Err(error) => {
            return api_error(
                StatusCode::NOT_FOUND,
                "library",
                format!("the file could not be read: {error:#}"),
            )
        }
    };
    let mut response = Response::new(Body::from_stream(file.body));
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(CONTENT_DISPOSITION, HeaderValue::from_static("attachment"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static("sandbox; default-src 'none'"),
    );
    response
}

/// Probe one Runtime target and issue a ticket for its isolated origin, returning its URL. The
/// cockpit never frames a target it has not just observed.
async fn open_review_origin(
    state: &OwnerState,
    company: &str,
    generation: &str,
    uri: &str,
    is_file: bool,
    item_id: &str,
) -> std::result::Result<String, Response<Body>> {
    let company = company.to_string();
    let (source, path_and_query) = if is_file {
        let (root, entry) = match runtime::runtime_review_file_root(uri) {
            Ok(value) => value,
            Err(error) => {
                return Err(api_error(
                    StatusCode::BAD_REQUEST,
                    "review",
                    format!("invalid review target: {error:#}"),
                ))
            }
        };
        // Observe the exact file before claiming the outcome opens. The cockpit
        // must never frame a target it has not seen.
        if let Err(error) = runtime::probe_runtime_review_file(&company, uri).await {
            if error.is::<runtime::ReviewFileTooLarge>() {
                return Err(api_error(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "review",
                    "This file is larger than the 16 GiB the cockpit can open or download.",
                ));
            }
            return Err(api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "review",
                format!("prepared outcome is unavailable: {error:#}"),
            ));
        }
        let path = format!("/{}", percent_encode_segment(&entry));
        (ReviewSource::Files { root, entry }, path)
    } else {
        let target = match runtime::runtime_http_target(uri) {
            Ok(target) => target,
            Err(error) => {
                return Err(api_error(
                    StatusCode::BAD_REQUEST,
                    "review",
                    format!("invalid review target: {error:#}"),
                ))
            }
        };
        if let Err(error) = runtime::probe_runtime_http(&company, uri).await {
            return Err(api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "review",
                format!("live outcome is unavailable: {error:#}"),
            ));
        }
        let path = target.path_and_query.clone();
        (ReviewSource::Service { port: target.port }, path)
    };

    let ticket = Uuid::new_v4().simple().to_string();
    let (review_url, expected_host) =
        match review_location(state, &ticket, &path_and_query) {
            Ok(value) => value,
            Err(error) => {
                return Err(api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "review",
                    format!("review origin is invalid: {error:#}"),
                ))
            }
        };
    state.reviews.lock().expect("review registry").insert(
        ticket,
        ReviewSession {
            company: company.clone(),
            generation: generation.to_string(),
            item_id: item_id.to_string(),
            source,
            expected_host,
            expires_at: SystemTime::now() + REVIEW_TTL,
        },
    );
    Ok(review_url)
}

pub(crate) async fn review_proxy(
    State(state): State<OwnerState>,
    method: Method,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response<Body> {
    if !matches!(method, Method::GET | Method::HEAD) {
        return api_error(
            StatusCode::METHOD_NOT_ALLOWED,
            "review",
            "review previews are read-only",
        );
    }
    let host = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let token = host
        .split(':')
        .next()
        .and_then(|hostname| hostname.split('.').next())
        .unwrap_or_default();
    if token.len() != 32 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "review",
            "review ticket is invalid",
        );
    }
    let session = {
        let mut reviews = state.reviews.lock().expect("review registry");
        reviews.retain(|_, review| review.expires_at > SystemTime::now());
        reviews
            .get(token)
            .filter(|review| review.expected_host.eq_ignore_ascii_case(&host))
            .cloned()
    };
    let Some(session) = session else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "review",
            "review ticket is absent or expired",
        );
    };
    let path = uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");
    serve_review(session, method, path, &headers, Isolation::OwnOrigin).await
}

/// A ticket served on the plane's own host: `/review/{ticket}/{path}`. The ticket in the path is the
/// whole authority, exactly as the ticket in the separate origin's hostname is; the response is
/// sandboxed into an opaque origin (see [`finish_review_response`]).
pub(crate) async fn review_path_proxy(
    State(state): State<OwnerState>,
    method: Method,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response<Body> {
    if !matches!(method, Method::GET | Method::HEAD) {
        return api_error(
            StatusCode::METHOD_NOT_ALLOWED,
            "review",
            "review previews are read-only",
        );
    }
    let rest = uri.path().strip_prefix(REVIEW_PATH_PREFIX).unwrap_or_default();
    let (token, file) = rest.split_once('/').unwrap_or((rest, ""));
    if token.len() != 32 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return api_error(StatusCode::UNAUTHORIZED, "review", "review ticket is invalid");
    }
    let session = {
        let mut reviews = state.reviews.lock().expect("review registry");
        reviews.retain(|_, review| review.expires_at > SystemTime::now());
        reviews
            .get(token)
            .filter(|review| review.expected_host.is_empty())
            .cloned()
    };
    let Some(session) = session else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "review",
            "review ticket is absent or expired",
        );
    };
    let path = match uri.query() {
        Some(query) => format!("/{file}?{query}"),
        None => format!("/{file}"),
    };
    let isolation = match headers.get(REVIEW_HOST_HEADER).and_then(|value| value.to_str().ok()) {
        Some(host) if review_host_for(&state, token).as_deref() == Some(host) => Isolation::ReviewHost {
            cockpit: state
                .entry
                .network()
                .and_then(|network| network.public_host())
                .map(|public| format!("https://{public}"))
                .unwrap_or_else(|| "'self'".into()),
        },
        _ => Isolation::PlaneHost,
    };
    serve_review(session, method, &path, &headers, isolation).await
}

/// Read one request's target for a resolved ticket, wherever the ticket was presented.
async fn serve_review(
    session: ReviewSession,
    method: Method,
    path: &str,
    headers: &HeaderMap,
    isolation: Isolation,
) -> Response<Body> {
    let current = runtime::generation(&session.company).await.ok().flatten();
    if current.as_deref() != Some(session.generation.as_str()) {
        return api_error(
            StatusCode::CONFLICT,
            "runtime",
            "review points at a replaced company computer",
        );
    }
    let upstream = match &session.source {
        ReviewSource::Service { port } => {
            match runtime::runtime_http_request(&session.company, *port, method, path, headers)
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    return api_error(
                        StatusCode::BAD_GATEWAY,
                        "review",
                        format!("live outcome bridge: {error:#}"),
                    )
                }
            }
        }
        ReviewSource::Files { root, entry } => {
            // The page being served is company-authored, so this is exactly
            // where a traversal out of the prepared outcome would be tried.
            let resolved = match runtime::resolve_review_file(root, entry, path) {
                Ok(resolved) => resolved,
                Err(error) => {
                    return api_error(StatusCode::NOT_FOUND, "review", format!("{error:#}"))
                }
            };
            // The ticket's own file streams, a page as much as a recording or a long PDF, and
            // answers a single Range request with just that slice (Safari and iPhone play no
            // video without one). The assets a page loads are read whole.
            let streamed = resolved == root.join(entry);
            let range = headers
                .get(axum::http::header::RANGE)
                .and_then(|value| value.to_str().ok());
            let read = if streamed {
                runtime::stream_runtime_review_file(&session.company, &resolved, range)
                    .await
                    .map(|file| {
                        let body = if method == Method::HEAD {
                            Body::empty()
                        } else {
                            Body::from_stream(file.body)
                        };
                        (file.media_type, body, Some((file.size, file.range)))
                    })
            } else {
                runtime::read_runtime_review_file(&session.company, &resolved)
                    .await
                    .map(|(media_type, bytes)| {
                        let body = if method == Method::HEAD {
                            Body::empty()
                        } else {
                            Body::from(bytes)
                        };
                        (media_type, body, None)
                    })
            };
            match read {
                Ok((media_type, body, streamed)) => {
                    let mut response = Response::new(body);
                    if let Some((size, range)) = streamed {
                        let headers = response.headers_mut();
                        headers.insert(
                            axum::http::header::ACCEPT_RANGES,
                            HeaderValue::from_static("bytes"),
                        );
                        let length = match range {
                            Some((first, last)) => {
                                headers.insert(
                                    axum::http::header::CONTENT_RANGE,
                                    HeaderValue::from_str(&format!("bytes {first}-{last}/{size}"))
                                        .expect("a byte range is a valid header"),
                                );
                                last - first + 1
                            }
                            None => size,
                        };
                        headers.insert(
                            axum::http::header::CONTENT_LENGTH,
                            HeaderValue::from(length),
                        );
                        if range.is_some() {
                            *response.status_mut() = StatusCode::PARTIAL_CONTENT;
                        }
                    }
                    response
                        .headers_mut()
                        .insert(CONTENT_TYPE, HeaderValue::from_static(media_type));
                    if runtime::is_runtime_review_download(&resolved) || wants_download(path) {
                        let filename = resolved
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(safe_attachment_name)
                            .unwrap_or_else(|| "review-file".into())
                            .replace(['\\', '"'], "_");
                        let disposition = format!("attachment; filename=\"{filename}\"");
                        response.headers_mut().insert(
                            CONTENT_DISPOSITION,
                            HeaderValue::from_str(&disposition)
                                .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
                        );
                        response.headers_mut().insert(
                            HeaderName::from_static("x-content-type-options"),
                            HeaderValue::from_static("nosniff"),
                        );
                    }
                    return finish_review_response(response, &session, path, &isolation);
                }
                Err(error) => {
                    return api_error(StatusCode::NOT_FOUND, "review", format!("{error:#}"))
                }
            }
        }
    };
    let (parts, body) = upstream.into_parts();
    let response = Response::from_parts(parts, Body::new(body));
    finish_review_response(response, &session, path, &isolation)
}

/// `download=1` in a ticket URL's query asks for the file itself, saved under its own name,
/// rather than shown.
fn wants_download(path_and_query: &str) -> bool {
    path_and_query
        .split_once('?')
        .is_some_and(|(_, query)| query.split('&').any(|pair| pair == "download=1"))
}

/// A file name as one URL path segment. The review read decodes it again, so a name with a
/// space, `#` or `?` still reaches exactly that file.
pub(crate) fn percent_encode_segment(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// One header policy for every isolated review origin, whatever it read from.
///
/// On the plane's own host the content would otherwise share the cockpit's origin, so it is
/// sandboxed: without `allow-same-origin` the browser gives it an opaque origin, in the cockpit's
/// frame or opened on its own, so it cannot read the cockpit's cookies, storage or DOM, and its
/// requests to the plane are cross-site (the `SameSite=Lax` session is not sent). A PDF file is the
/// one exception: the browser's reader refuses a sandboxed response, and a PDF runs no page script.
/// Only a file's type is Core's own, chosen from its extension; a service names its own, and a
/// browser reads `application/pdf, text/html` or `application/pdf+xml` as a page, so a service is
/// always sandboxed.
pub(crate) fn finish_review_response(
    mut response: Response<Body>,
    session: &ReviewSession,
    path: &str,
    isolation: &Isolation,
) -> Response<Body> {
    for name in [
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "set-cookie",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
        // The preview iframe is already isolated on its own origin and
        // sandboxed by the cockpit. Upstream anti-framing headers describe a
        // public deployment, not this owner-only review projection.
        "content-security-policy",
        "x-frame-options",
    ] {
        response.headers_mut().remove(name);
    }
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    if let Isolation::ReviewHost { cockpit } = isolation {
        if let Ok(value) = HeaderValue::from_str(&format!("frame-ancestors {cockpit}")) {
            response.headers_mut().insert("content-security-policy", value);
        }
    }
    if matches!(isolation, Isolation::PlaneHost) {
        let pdf = matches!(session.source, ReviewSource::Files { .. })
            && response
                .headers()
                .get(CONTENT_TYPE)
                .is_some_and(|value| value == "application/pdf");
        response.headers_mut().insert(
            "content-security-policy",
            HeaderValue::from_static(if pdf {
                "frame-ancestors 'self'"
            } else {
                "sandbox allow-scripts allow-forms allow-popups allow-modals allow-downloads; frame-ancestors 'self'"
            }),
        );
    }
    tracing::debug!(
        company = %session.company,
        item = %session.item_id,
        path,
        "served isolated live review"
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_review_subdomain_is_its_own_origin_framed_only_by_the_cockpit() {
        let ticket = "0123456789abcdef0123456789abcdef";
        assert_eq!(
            review_subdomain("owner-ee54fb58bcde23cf.restless.run", ticket).as_deref(),
            Some("0123456789abcdef0123456789abcdef-ee54fb58bcde23cf-review.restless.run")
        );
        assert_eq!(review_subdomain("app.restless.run", ticket), None);
        assert_eq!(review_subdomain("owner-zz.restless.run", ticket), None);
        assert_eq!(review_subdomain("owner-ab.restless.run", "../x"), None);

        let mut response = Response::new(Body::empty());
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));
        response
            .headers_mut()
            .insert("set-cookie", HeaderValue::from_static("a=b; Domain=restless.run"));
        let response = finish_review_response(
            response,
            &session(),
            "/",
            &Isolation::ReviewHost { cockpit: "https://app.restless.run".into() },
        );
        assert_eq!(
            response.headers().get("content-security-policy").unwrap(),
            "frame-ancestors https://app.restless.run"
        );
        assert!(response.headers().get("set-cookie").is_none());
    }

    fn session() -> ReviewSession {
        ReviewSession {
            company: "acme_test".into(),
            generation: "g1".into(),
            item_id: "artifact:1".into(),
            source: ReviewSource::Files {
                root: PathBuf::from("/company/outputs/site"),
                entry: "index.html".into(),
            },
            expected_host: String::new(),
            expires_at: SystemTime::now() + REVIEW_TTL,
        }
    }

    fn served(content_type: &'static str, upstream_csp: Option<&'static str>, on_plane_host: bool) -> String {
        served_from(&session(), content_type, upstream_csp, on_plane_host)
    }

    fn served_from(
        session: &ReviewSession,
        content_type: &'static str,
        upstream_csp: Option<&'static str>,
        on_plane_host: bool,
    ) -> String {
        let mut response = Response::new(Body::empty());
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
        if let Some(csp) = upstream_csp {
            response
                .headers_mut()
                .insert("content-security-policy", HeaderValue::from_static(csp));
        }
        let isolation = if on_plane_host { Isolation::PlaneHost } else { Isolation::OwnOrigin };
        let response = finish_review_response(response, session, "/index.html", &isolation);
        response
            .headers()
            .get_all("content-security-policy")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect::<Vec<_>>()
            .join(" | ")
    }

    /// Company content served on the plane's own host must never share the cockpit's origin: it is
    /// sandboxed without `allow-same-origin`, whatever policy the company's own service sent.
    #[test]
    fn plane_host_content_is_sandboxed_into_an_opaque_origin() {
        for content_type in ["text/html; charset=utf-8", "image/svg+xml", "text/plain; charset=utf-8"] {
            let csp = served(content_type, Some("sandbox allow-same-origin allow-scripts"), true);
            assert!(csp.starts_with("sandbox "), "{content_type}: {csp}");
            assert!(!csp.contains("allow-same-origin"), "{content_type}: {csp}");
            assert!(csp.contains("frame-ancestors 'self'"), "{content_type}: {csp}");
        }
    }

    /// The browser's PDF reader refuses a sandboxed response; a PDF runs no page script, and
    /// `nosniff` keeps anything else from being read as one.
    #[test]
    fn plane_host_pdf_is_framed_only_by_the_cockpit() {
        assert_eq!(served("application/pdf", None, true), "frame-ancestors 'self'");
    }

    /// A service names its own type, and a browser renders `application/pdf, text/html` or
    /// `application/pdf+xml` as a page that would run on the cockpit's origin. Only the exact PDF
    /// type Core chose for a file is left unsandboxed.
    #[test]
    fn plane_host_service_is_sandboxed_whatever_type_it_claims() {
        let service = ReviewSession {
            source: ReviewSource::Service { port: 8080 },
            ..session()
        };
        for content_type in ["application/pdf", "application/pdf, text/html", "application/pdf+xml"] {
            let csp = served_from(&service, content_type, None, true);
            assert!(csp.starts_with("sandbox "), "{content_type}: {csp}");
            assert!(!csp.contains("allow-same-origin"), "{content_type}: {csp}");
        }
        assert!(served("application/pdf+xml", None, true).starts_with("sandbox "));
    }

    /// On the separate review origin the origin itself isolates the content; no policy is added
    /// and upstream anti-framing is dropped, as before.
    #[test]
    fn separate_origin_adds_no_policy() {
        assert_eq!(served("text/html; charset=utf-8", Some("frame-ancestors 'none'"), false), "");
    }

    /// A site recorded at its root URL is issued as `/review/{ticket}/`. That page, like every file
    /// beneath it, must reach the ticket rather than the cockpit's own "page does not exist".
    #[tokio::test]
    async fn plane_host_route_reaches_the_ticket_for_a_root_target() {
        use tower::ServiceExt as _;
        let app = Router::new()
            .route(REVIEW_PATH_ROUTE, any(|| async { "ticket" }))
            .fallback(|| async { "cockpit" });
        let ticket = "a".repeat(32);
        for path in [
            format!("/review/{ticket}/"),
            format!("/review/{ticket}/?tab=2"),
            format!("/review/{ticket}/assets/site.css"),
        ] {
            let request = Request::builder().uri(&path).body(Body::empty()).unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert_eq!(&body[..], b"ticket", "{path}");
        }
    }
}
