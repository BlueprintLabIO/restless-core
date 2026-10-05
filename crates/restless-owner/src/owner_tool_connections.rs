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

#[derive(Serialize)]
struct ConnectionView {
    #[serde(flatten)]
    connection: connections::Connection,
    grants: Vec<connections::ConnectionGrant>,
    /// Exec's starting point for each observed tool; the owner adjusts it.
    proposed: Vec<connections::GrantedTool>,
    /// Tools whose upstream definition changed since they were granted.
    changed: Vec<String>,
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

fn callback_page(status: StatusCode, message: &str) -> Response<Body> {
    let escaped = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let mut response = Response::new(Body::from(format!(
        "<!doctype html><meta charset=utf-8><title>Sign-in</title><p style=\"font:16px system-ui;margin:3rem\">{escaped}</p>"
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
    if let Some(error) = query.error.as_deref() {
        let error = error
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .take(64)
            .collect::<String>();
        return callback_page(StatusCode::BAD_REQUEST, &format!("The provider did not complete the sign-in ({error}). Start it again from Connections."));
    }
    let (Some(code), Some(oauth_state)) = (query.code.as_deref(), query.state.as_deref()) else {
        return callback_page(
            StatusCode::BAD_REQUEST,
            "This sign-in link is incomplete. Start it again from Connections.",
        );
    };
    if connections::pending_sign_in_company(oauth_state)
        .await
        .is_none()
    {
        return callback_page(
            StatusCode::BAD_REQUEST,
            "This sign-in link expired or was already used. Start it again from Connections.",
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
        Ok((company, _connection)) => {
            Redirect::to(&format!("/{}/company", urlencoding_path(&company))).into_response()
        }
        Err(error) => callback_page(StatusCode::BAD_REQUEST, &format!("{error:#}")),
    }
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
        Ok(grant) => Json(serde_json::json!({ "grant": grant })).into_response(),
        Err(error) => connection_error(error),
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
