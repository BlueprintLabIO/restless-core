//! Owner API for connected tools: add, sign in, probe, grant by class,
//! freeze, disconnect and receipts. Every write is an Authority-owner act; the
//! engine's `connections` module owns the rules.

use super::*;
use restless_engine::connections::{self, NewConnection, ToolDecision};

/// The provider redirects the owner's browser here. It is a top-level
/// cross-site navigation, so it bypasses the browser boundary; the single-use,
/// server-generated OAuth state bound to a sign-in the owner started is its
/// authority.
pub(super) const OAUTH_CALLBACK_PATH: &str = "/connections/tools/oauth/callback";

fn connection_error(error: anyhow::Error) -> Response<Body> {
    api_error(StatusCode::BAD_REQUEST, "connection", format!("{error:#}"))
}

async fn owner_gate(
    state: &OwnerState,
    company: &str,
    principal: &RequestPrincipal,
) -> Result<(), Response<Body>> {
    if let Err(error) = runtime::CompanyConfig::load(&state.daemon.root, company) {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            "company",
            format!("{error:#}"),
        ));
    }
    require_authority_owner(state, company, principal).await
}

/// Connected tools reach agents through the tool gateway on this plane's
/// Runtime relay, which a hosted (externally managed) Runtime does not use.
/// Say so instead of adding an app that no agent could ever call.
/// A hosted plane reaches online apps through its tool gateway, but has no
/// Docker of its own to run an app's local command in.
fn refuse_when_hosted(state: &OwnerState) -> Result<(), Response<Body>> {
    if state.daemon.runtime_bridges.is_hosted() {
        return Err(api_error(
            StatusCode::CONFLICT,
            "connection",
            "This app runs as a command on a computer, which Restless Cloud can't do yet. \
             Online apps (a link starting https://) work here.",
        ));
    }
    Ok(())
}

#[derive(Serialize)]
struct ConnectionView {
    #[serde(flatten)]
    connection: connections::Connection,
    grants: Vec<connections::ConnectionGrant>,
    /// Exec's starting point for each observed tool; the owner adjusts it.
    proposed: Vec<connections::GrantedTool>,
    /// Tools whose upstream definition changed since they were granted.
    changed: Vec<String>,
    /// What a local tool last printed to its error output, secrets removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    last_output: Option<String>,
}

pub(super) async fn list(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    let pool = state.daemon.authority.pool();
    let result = async {
        let all = connections::list(pool, &company).await?;
        let grants = connections::grants(pool, &company, None).await?;
        let changed = connections::changed_tools(pool, &company).await?;
        Ok::<_, anyhow::Error>(
            all.into_iter()
                .map(|connection| ConnectionView {
                    grants: grants
                        .iter()
                        .filter(|grant| grant.connection == connection.name)
                        .cloned()
                        .collect(),
                    proposed: connection
                        .tools
                        .iter()
                        .map(connections::proposed_grant)
                        .collect(),
                    changed: changed
                        .iter()
                        .filter(|(name, _)| name == &connection.name)
                        .map(|(_, tool)| tool.clone())
                        .collect(),
                    last_output: connections::worker_log_tail(&company, &connection.name),
                    connection,
                })
                .collect::<Vec<_>>(),
        )
    }
    .await;
    match result {
        Ok(views) => Json(serde_json::json!({ "connections": views })).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "connection",
            format!("{error:#}"),
        ),
    }
}

pub(super) async fn add(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<NewConnection>,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    let pool = state.daemon.authority.pool();
    let added = match connections::add(
        pool,
        &state.daemon.authority,
        &company,
        input,
        principal.actor_id(),
    )
    .await
    {
        Ok(added) => added,
        Err(error) => return connection_error(error),
    };
    // Probe now when nothing waits on a sign-in: the owner sees the observed
    // tools or the honest failure, never a hopeful "connected".
    let connection = if added.status == "awaiting_probe" {
        match connections::probe(pool, &state.daemon.root, &company, &added.name).await {
            Ok(connection) => connection,
            Err(error) => return connection_error(error),
        }
    } else {
        added
    };
    Json(serde_json::json!({ "connection": connection })).into_response()
}

pub(super) async fn probe(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, name)): AxumPath<(String, String)>,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    match connections::probe(
        state.daemon.authority.pool(),
        &state.daemon.root,
        &company,
        &name,
    )
    .await
    {
        Ok(connection) => Json(serde_json::json!({ "connection": connection })).into_response(),
        Err(error) => connection_error(error),
    }
}

pub(super) async fn sign_in(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, name)): AxumPath<(String, String)>,
    headers: HeaderMap,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    // The owner boundary already requires a same-origin Origin on writes, so
    // it is the origin the owner's browser will return to.
    let Some(origin) = headers.get(ORIGIN).and_then(|value| value.to_str().ok()) else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "connection",
            "sign-in must start from the cockpit",
        );
    };
    let redirect = format!("{}{OAUTH_CALLBACK_PATH}", origin.trim_end_matches('/'));
    match connections::begin_sign_in(
        state.daemon.authority.pool(),
        &state.daemon.root,
        &company,
        &name,
        &redirect,
    )
    .await
    {
        Ok(url) => Json(serde_json::json!({ "authorization_url": url })).into_response(),
        Err(error) => connection_error(error),
    }
}

#[derive(Deserialize)]
pub(super) struct OauthCallback {
    code: Option<String>,
    state: Option<String>,
    iss: Option<String>,
    error: Option<String>,
}

/// A sign-in that did not complete says so, with a way back to start it again:
/// the app's own page while the sign-in is still known, otherwise home.
fn callback_page(
    status: StatusCode,
    message: &str,
    sign_in: Option<&(String, String)>,
) -> Response<Body> {
    let escaped = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let (href, label) = match sign_in {
        Some((company, name)) => (app_path(company, name), "Back to Apps"),
        None => ("/".to_string(), "Back to your companies"),
    };
    let mut response = Response::new(Body::from(format!(
        "<!doctype html><meta charset=utf-8><title>Sign-in</title><body style=\"font:16px system-ui;margin:3rem\"><p>{escaped}</p><p><a href=\"{href}\">{label}</a></p>"
    )));
    *response.status_mut() = status;
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response
}

pub(super) async fn oauth_callback(
    State(state): State<OwnerState>,
    axum::extract::Query(query): axum::extract::Query<OauthCallback>,
) -> Response<Body> {
    // The company and app of a sign-in still pending, so a failed one leads
    // back to that app.
    let sign_in = match query.state.as_deref() {
        Some(oauth_state) => connections::pending_sign_in(oauth_state).await,
        None => None,
    };
    if let Some(error) = query.error.as_deref() {
        let error = error
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .take(64)
            .collect::<String>();
        return callback_page(StatusCode::BAD_REQUEST, &format!("The provider did not complete the sign-in ({error}). Start it again from Apps."), sign_in.as_ref());
    }
    let (Some(code), Some(oauth_state)) = (query.code.as_deref(), query.state.as_deref()) else {
        return callback_page(
            StatusCode::BAD_REQUEST,
            "This sign-in link is incomplete. Start it again from Apps.",
            sign_in.as_ref(),
        );
    };
    if sign_in.is_none() {
        return callback_page(
            StatusCode::BAD_REQUEST,
            "This sign-in link expired or was already used. Start it again from Apps.",
            None,
        );
    }
    match connections::complete_sign_in(
        state.daemon.authority.pool(),
        &state.daemon.authority,
        &state.daemon.root,
        code,
        oauth_state,
        query.iss.as_deref(),
    )
    .await
    {
        Ok((company, name)) => {
            // Signing in to an app the company may already use resumes the Work
            // waiting on it. A new app lands where the owner allows it, beside
            // the request it answers, since only then can that Work resume.
            let request = if resolve_app_requests(&state, &company, &name).await == 0 {
                pending_app_request(&state, &company, &name).await
            } else {
                None
            };
            Redirect::to(&format!(
                "{}{}",
                app_path(&company, &name),
                request.map(|id| format!("?request={id}")).unwrap_or_default()
            ))
            .into_response()
        }
        Err(error) => callback_page(
            StatusCode::BAD_REQUEST,
            &format!("{error:#}"),
            sign_in.as_ref(),
        ),
    }
}

/// The cockpit page of one company's app, where the owner signs in and allows it.
fn app_path(company: &str, name: &str) -> String {
    format!(
        "/{}/apps/c-{}",
        urlencoding_path(company),
        urlencoding_path(name)
    )
}

fn urlencoding_path(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GrantInput {
    /// `*` for every actor, or one actor id.
    #[serde(default = "every_actor")]
    grantee: String,
    tools: Vec<ToolDecision>,
    #[serde(default)]
    expires_at: Option<DateTime<Utc>>,
}

fn every_actor() -> String {
    "*".into()
}

pub(super) async fn grant(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, name)): AxumPath<(String, String)>,
    Json(input): Json<GrantInput>,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    match connections::grant(
        state.daemon.authority.pool(),
        &state.daemon.authority,
        &company,
        &name,
        &input.grantee,
        input.tools,
        principal.actor_id(),
        input.expires_at,
    )
    .await
    {
        Ok(grant) => {
            let resumed = resolve_app_requests(&state, &company, &name).await;
            Json(serde_json::json!({ "grant": grant, "resumed_requests": resumed })).into_response()
        }
        Err(error) => connection_error(error),
    }
}

/// The names an app request may give this app: its own, its address or its
/// catalogue key.
fn requested_names(connection: &connections::Connection) -> Vec<String> {
    let mut names = vec![connection.name.clone()];
    if let Some(endpoint) = connection.endpoint.as_deref() {
        names.push(endpoint.to_string());
        names.push(endpoint.trim_end_matches('/').to_string());
    }
    if let Some(key) = connection
        .source
        .as_deref()
        .and_then(|source| source.strip_prefix("catalogue:"))
    {
        names.push(key.to_string());
    }
    names
}

/// A working, allowed app is the observable resume condition of every pending
/// app request it satisfies, whether a grant or a later sign-in completed it:
/// resolve them as an observation, so the Work resumes and the owner never
/// reports that the sign-in is done. A failure here never undoes the grant;
/// the request stays visible in Apps.
async fn resolve_app_requests(state: &OwnerState, company: &str, name: &str) -> usize {
    let pool = state.daemon.authority.pool();
    let Ok(connection) = connections::require(pool, company, name).await else {
        return 0;
    };
    let allowed = connections::grants(pool, company, Some(name))
        .await
        .is_ok_and(|grants| !grants.is_empty());
    if connection.status != "working" || !allowed {
        return 0;
    }
    let Ok(org) = state.daemon.orgintel.get(company).await else {
        return 0;
    };
    let Ok(requests) = org.app_requests_for(&requested_names(&connection)).await else {
        return 0;
    };
    let mut resolved = 0;
    for id in requests {
        let outcome = org
            .resolve_observed_handoff(
                id,
                "daemon",
                &format!(
                    "The owner connected {} and allowed what the company may do with it. New sessions receive its tools.",
                    connection.name
                ),
            )
            .await;
        match outcome {
            Ok(true) => resolved += 1,
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(company, handoff = %id, error = %error, "app request was not resolved")
            }
        }
    }
    resolved
}

/// The oldest pending app request this app satisfies once it is allowed.
async fn pending_app_request(state: &OwnerState, company: &str, name: &str) -> Option<Uuid> {
    let connection = connections::require(state.daemon.authority.pool(), company, name)
        .await
        .ok()?;
    let org = state.daemon.orgintel.get(company).await.ok()?;
    org.app_requests_for(&requested_names(&connection))
        .await
        .ok()?
        .into_iter()
        .next()
}

/// Pending sign-in handoffs that name an app: Exec's requests in Apps.
pub(super) async fn app_requests(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    // A request the owner set aside stays out of the way until it is restored.
    let requests = match state.daemon.orgintel.get(&company).await {
        Ok(org) => match (org.app_requests().await, org.attention_dismissals().await) {
            (Ok(requests), dismissed) => {
                let dismissed = dismissed.unwrap_or_default();
                Ok(requests
                    .into_iter()
                    .filter(|request| !dismissed.contains_key(&format!("orgintel:handoff:{}", request.handoff_id)))
                    .collect::<Vec<_>>())
            }
            (Err(error), _) => Err(anyhow::Error::from(error)),
        },
        Err(error) => Err(error),
    };
    match requests {
        Ok(requests) => Json(serde_json::json!({ "requests": requests })).into_response(),
        Err(error) => api_error(StatusCode::SERVICE_UNAVAILABLE, "apps", format!("{error:#}")),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RevokeInput {
    #[serde(default = "every_actor")]
    grantee: String,
}

pub(super) async fn revoke(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, name)): AxumPath<(String, String)>,
    Json(input): Json<RevokeInput>,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    match connections::revoke_grant(
        state.daemon.authority.pool(),
        &state.daemon.authority,
        &company,
        &name,
        &input.grantee,
        principal.actor_id(),
    )
    .await
    {
        Ok(revoked) => Json(serde_json::json!({ "revoked": revoked })).into_response(),
        Err(error) => connection_error(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FreezeInput {
    frozen: bool,
}

pub(super) async fn freeze(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, name)): AxumPath<(String, String)>,
    Json(input): Json<FreezeInput>,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    match connections::set_frozen(
        state.daemon.authority.pool(),
        &state.daemon.authority,
        &company,
        &name,
        input.frozen,
        principal.actor_id(),
    )
    .await
    {
        Ok(connection) => Json(serde_json::json!({ "connection": connection })).into_response(),
        Err(error) => connection_error(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BrowserInput {
    browser: bool,
}

/// Let one local tool drive the company computer's browser, or stop it.
pub(super) async fn browser(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, name)): AxumPath<(String, String)>,
    Json(input): Json<BrowserInput>,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    match connections::set_browser(
        state.daemon.authority.pool(),
        &state.daemon.authority,
        &company,
        &name,
        input.browser,
        principal.actor_id(),
    )
    .await
    {
        Ok(connection) => Json(serde_json::json!({ "connection": connection })).into_response(),
        Err(error) => connection_error(error),
    }
}

pub(super) async fn disconnect(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, name)): AxumPath<(String, String)>,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    match connections::disconnect(
        state.daemon.authority.pool(),
        &state.daemon.authority,
        &state.daemon.root,
        &company,
        &name,
        principal.actor_id(),
    )
    .await
    {
        Ok(connection) => Json(serde_json::json!({ "connection": connection })).into_response(),
        Err(error) => connection_error(error),
    }
}

/// Recent receipts lead the connection view: reads from the gateway's read
/// receipts, effects from the Authority record.
pub(super) async fn receipts(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, name)): AxumPath<(String, String)>,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    let authority = &state.daemon.authority;
    let result = async {
        let reads =
            connections::recent_read_receipts(authority.pool(), &company, &name, 50).await?;
        let class = restless_engine::effect::tool_effect_class(&name);
        let effects = authority
            .recent_records_of_kind(&company, "effect", 200)
            .await?
            .into_iter()
            .filter(|record| {
                record.body.get("effect_class").and_then(|v| v.as_str()) == Some(class.as_str())
            })
            .take(50)
            .map(|record| record.body)
            .collect::<Vec<_>>();
        Ok::<_, anyhow::Error>(serde_json::json!({ "reads": reads, "effects": effects }))
    }
    .await;
    match result {
        Ok(body) => Json(body).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "connection",
            format!("{error:#}"),
        ),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PluginInput {
    url: String,
}

/// Import a plugin bundle: its MCP servers become connections awaiting
/// probe and grant, and Exec is asked to add its skills as candidates in the
/// company computer, where skills live.
pub(super) async fn import_plugin(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<PluginInput>,
) -> Response<Body> {
    if let Err(refusal) = owner_gate(&state, &company, &principal).await {
        return refusal;
    }
    if let Err(refusal) = refuse_when_hosted(&state) {
        return refusal;
    }
    let pool = state.daemon.authority.pool();
    let mut imported = match connections::import_plugin(
        pool,
        &state.daemon.authority,
        &company,
        input.url.trim(),
        principal.actor_id(),
    )
    .await
    {
        Ok(imported) => imported,
        Err(error) => return connection_error(error),
    };
    for connection in &mut imported.connections {
        if connection.status == "awaiting_probe" {
            if let Ok(probed) =
                connections::probe(pool, &state.daemon.root, &company, &connection.name).await
            {
                *connection = probed;
            }
        }
    }
    let mut skills_requested = false;
    if !imported.skills.is_empty() {
        if let Ok(org) = state.daemon.orgintel.get(&company).await {
            let sources = imported
                .skills
                .iter()
                .map(|source| format!("- `restless skill add {source}`"))
                .collect::<Vec<_>>()
                .join("\n");
            let body = format!(
                "I imported the plugin {} ({}). Add its skills as candidates for me to review:\n{sources}\nThen tell me in one line which were added.",
                imported.plugin, imported.source
            );
            skills_requested = org
                .send_message(principal.actor_id(), Some("exec"), &body)
                .await
                .is_ok();
        }
    }
    Json(serde_json::json!({ "import": imported, "skills_requested": skills_requested }))
        .into_response()
}
