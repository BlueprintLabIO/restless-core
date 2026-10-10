//! Isolated review origins: tickets that open a Runtime-served outcome under its own origin, and the read-only proxy behind them.

use super::*;

pub(crate) const REVIEW_TTL: Duration = Duration::from_secs(30 * 60);

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
        .decide_owner_review(handoff, decision, &input.feedback)
        .await
    {
        Ok(()) => Json(serde_json::json!({
            "handoff_id": handoff,
            "decision": input.decision,
            "recorded": true,
        }))
        .into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "review", format!("{error:#}")),
    }
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
    let current = runtime::generation(&company).await.ok().flatten();
    if current.as_deref() != Some(reference.generation.as_str()) {
        return api_error(
            StatusCode::CONFLICT,
            "runtime",
            "runtime generation changed; refresh the review",
        );
    }
    open_review_origin(
        &state,
        &company,
        &reference.generation,
        &reference.uri,
        reference.kind == "runtime-file",
        &item.id,
    )
    .await
}

#[derive(Debug, Deserialize)]
pub(crate) struct LibraryTicketRequest {
    pub(crate) artifact_id: Uuid,
}

/// Open one file Work recorded, from Library, in the same isolated, read-only origin a review
/// uses (S64-T5). The reference must still be the available version at its locator; the file is
/// probed before the ticket is issued, exactly as a review's is.
pub(crate) async fn issue_library_ticket(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<LibraryTicketRequest>,
) -> impl IntoResponse {
    if let Err(error) = runtime::CompanyConfig::load(&state.daemon.root, &company) {
        return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}"));
    }
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
    let artifact = match org.get_artifact_ref(input.artifact_id).await {
        Ok(Some(artifact)) if artifact.work_id.is_some() => artifact,
        Ok(_) => {
            return api_error(
                StatusCode::NOT_FOUND,
                "library",
                "this file is not one the company's Work recorded",
            )
        }
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    if artifact.state != restless_orgintel::ArtifactRefState::Available {
        return api_error(
            StatusCode::CONFLICT,
            "library",
            "this file was replaced by a newer version or is missing",
        );
    }
    let Some(generation) = runtime::generation(&company).await.ok().flatten() else {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime",
            "the company computer is not running",
        );
    };
    let is_file = runtime::runtime_review_file_root(&artifact.uri).is_ok();
    if !is_file && runtime::runtime_http_target(&artifact.uri).is_err() {
        return api_error(
            StatusCode::CONFLICT,
            "library",
            "this file cannot be shown in the cockpit",
        );
    }
    open_review_origin(
        &state,
        &company,
        &generation,
        &artifact.uri,
        is_file,
        &format!("artifact:{}", artifact.id),
    )
    .await
}

/// Probe one Runtime target and issue a ticket for its isolated origin. The cockpit never frames
/// a target it has not just observed.
async fn open_review_origin(
    state: &OwnerState,
    company: &str,
    generation: &str,
    uri: &str,
    is_file: bool,
    item_id: &str,
) -> Response<Body> {
    let company = company.to_string();
    let (source, path_and_query) = if is_file {
        let (root, entry) = match runtime::runtime_review_file_root(uri) {
            Ok(value) => value,
            Err(error) => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "review",
                    format!("invalid review target: {error:#}"),
                )
            }
        };
        // Observe the exact file before claiming the outcome opens. The cockpit
        // must never frame a target it has not seen.
        if let Err(error) = runtime::probe_runtime_review_file(&company, uri).await {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "review",
                format!("prepared outcome is unavailable: {error:#}"),
            );
        }
        let path = format!("/{entry}");
        (ReviewSource::Files { root, entry }, path)
    } else {
        let target = match runtime::runtime_http_target(uri) {
            Ok(target) => target,
            Err(error) => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "review",
                    format!("invalid review target: {error:#}"),
                )
            }
        };
        if let Err(error) = runtime::probe_runtime_http(&company, uri).await {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "review",
                format!("live outcome is unavailable: {error:#}"),
            );
        }
        let path = target.path_and_query.clone();
        (ReviewSource::Service { port: target.port }, path)
    };

    let ticket = Uuid::new_v4().simple().to_string();
    let (review_url, expected_host) =
        match materialize_review_url(&state.review_public_url, &ticket, &path_and_query) {
            Ok(value) => value,
            Err(error) => {
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "review",
                    format!("review origin is invalid: {error:#}"),
                )
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
    Json(ReviewTicketResponse {
        review_url,
        expires_in_seconds: REVIEW_TTL.as_secs(),
    })
    .into_response()
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
    let current = runtime::generation(&session.company).await.ok().flatten();
    if current.as_deref() != Some(session.generation.as_str()) {
        return api_error(
            StatusCode::CONFLICT,
            "runtime",
            "review points at a replaced company computer",
        );
    }
    let path = uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");
    let upstream = match &session.source {
        ReviewSource::Service { port } => {
            match runtime::runtime_http_request(&session.company, *port, method, path, &headers)
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
            match runtime::read_runtime_review_file(&session.company, &resolved).await {
                Ok((media_type, bytes)) => {
                    let body = if method == Method::HEAD {
                        Body::empty()
                    } else {
                        Body::from(bytes)
                    };
                    let mut response = Response::new(body);
                    response
                        .headers_mut()
                        .insert(CONTENT_TYPE, HeaderValue::from_static(media_type));
                    if runtime::is_runtime_review_download(&resolved) {
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
                    return finish_review_response(response, &session, path);
                }
                Err(error) => {
                    return api_error(StatusCode::NOT_FOUND, "review", format!("{error:#}"))
                }
            }
        }
    };
    let (parts, body) = upstream.into_parts();
    let response = Response::from_parts(parts, Body::new(body));
    finish_review_response(response, &session, path)
}

/// One header policy for every isolated review origin, whatever it read from.
pub(crate) fn finish_review_response(
    mut response: Response<Body>,
    session: &ReviewSession,
    path: &str,
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
    tracing::debug!(
        company = %session.company,
        item = %session.item_id,
        path,
        "served isolated live review"
    );
    response
}
