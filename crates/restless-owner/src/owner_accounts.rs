//! Settings › Accounts (S98-01): the accounts the owner holds and the companies granted them.
//! Owner only, except a company's own grants, which its members see. No response carries a
//! credential value or its vault reference.
use super::*;
use restless_engine::accounts::{self, GrantInput, NewAccount};

fn refuse(error: anyhow::Error) -> Response<Body> {
    // Validation errors are already sentences for the owner; storage errors are not shown raw.
    let message = format!("{error:#}");
    // A plane with no vault cannot keep a password or key at all; retrying would not help.
    if message.contains("INFISICAL_") && !restless_engine::credential::infisical_configured() {
        tracing::warn!(error = %message, "account change failed: no vault");
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "vault_unavailable",
            "This plane has no vault to keep passwords and keys, so nothing was saved. A browser sign-in needs no vault; for the rest, set up the vault (see self-hosting).",
        );
    }
    if message.ends_with('.') && message.chars().count() <= 200 && !message.contains("error") {
        api_error(StatusCode::BAD_REQUEST, "accounts", message)
    } else {
        tracing::warn!(error = %message, "account change failed");
        api_error(StatusCode::SERVICE_UNAVAILABLE, "accounts", "Accounts could not be saved. Try again.")
    }
}

fn owner_only(principal: &RequestPrincipal) -> Option<Response<Body>> {
    (!principal.is_account_owner()).then(|| {
        api_error(StatusCode::FORBIDDEN, "accounts", "Only the account owner manages accounts.")
    })
}

pub(super) async fn list(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let mut held = match accounts::list(state.daemon.authority.pool()).await {
        Ok(held) => held,
        Err(error) => return refuse(error),
    };
    // AI accounts are listed from their own store until they move here.
    if let Ok(ai) = accounts::ai_provider_accounts(&state.daemon.root) {
        held.splice(0..0, ai);
    }
    Json(serde_json::json!({ "accounts": held })).into_response()
}

pub(super) async fn create(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    Json(input): Json<NewAccount>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    match accounts::create(state.daemon.authority.pool(), input).await {
        Ok(account) => (StatusCode::CREATED, Json(account)).into_response(),
        Err(error) => refuse(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AccountPatch {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    frozen: Option<bool>,
}

pub(super) async fn update(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(id): AxumPath<String>,
    Json(input): Json<AccountPatch>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    match accounts::update(state.daemon.authority.pool(), &id, input.label.as_deref(), input.frozen).await {
        Ok(Some(account)) => Json(account).into_response(),
        Ok(None) => api_error(StatusCode::NOT_FOUND, "accounts", "That account no longer exists."),
        Err(error) => refuse(error),
    }
}

pub(super) async fn remove(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(id): AxumPath<String>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    match accounts::delete(state.daemon.authority.pool(), &id).await {
        Ok(true) => {
            // Its signed-in browser goes with it.
            restless_engine::account_browser::remove(&id).await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => api_error(StatusCode::NOT_FOUND, "accounts", "That account no longer exists."),
        Err(error) => refuse(error),
    }
}

pub(super) async fn grant(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(id): AxumPath<String>,
    Json(input): Json<GrantInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    if let Some(company) = input.company.as_deref() {
        if runtime::CompanyConfig::load(&state.daemon.root, company).is_err() {
            return api_error(StatusCode::BAD_REQUEST, "accounts", "Choose one of your companies.");
        }
    }
    match accounts::create_grant(state.daemon.authority.pool(), &id, input, principal.actor_id()).await {
        Ok(grant) => (StatusCode::CREATED, Json(grant)).into_response(),
        Err(error) => refuse(error),
    }
}

pub(super) async fn update_grant(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((id, grant)): AxumPath<(String, String)>,
    Json(input): Json<GrantInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    if input.company.is_some() {
        return api_error(StatusCode::BAD_REQUEST, "accounts", "A grant stays with its company; grant the account to the other company instead.");
    }
    match accounts::update_grant(state.daemon.authority.pool(), &id, &grant, input).await {
        Ok(Some(grant)) => Json(grant).into_response(),
        Ok(None) => api_error(StatusCode::NOT_FOUND, "accounts", "That grant no longer exists."),
        Err(error) => refuse(error),
    }
}

pub(super) async fn remove_grant(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((id, grant)): AxumPath<(String, String)>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    match accounts::delete_grant(state.daemon.authority.pool(), &id, &grant).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => api_error(StatusCode::NOT_FOUND, "accounts", "That grant no longer exists."),
        Err(error) => refuse(error),
    }
}

/// The account's browser identity, for the owner's sign-in (S98-11). Answers the account's
/// settings when it is a browser account.
async fn browser_account(state: &OwnerState, id: &str) -> std::result::Result<(accounts::Account, serde_json::Value), Response<Body>> {
    let pool = state.daemon.authority.pool();
    let account = match accounts::get(pool, id).await {
        Ok(Some(account)) => account,
        Ok(None) => return Err(api_error(StatusCode::NOT_FOUND, "accounts", "That account no longer exists.")),
        Err(error) => return Err(refuse(error)),
    };
    if !restless_engine::account_browser::BROWSER_KINDS.contains(&account.kind.as_str()) {
        return Err(api_error(StatusCode::BAD_REQUEST, "accounts", "Only website logins and browser profiles sign in through a browser."));
    }
    match accounts::credential_settings(pool, id).await {
        Ok(Some(settings)) => Ok((account, settings)),
        Ok(None) => Err(api_error(StatusCode::NOT_FOUND, "accounts", "That account has no sign-in details.")),
        Err(error) => Err(refuse(error)),
    }
}

/// Start signing an account in: its browser opens at the sign-in page, and the owner signs in by
/// hand in the desktop this answers.
pub(super) async fn sign_in(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(id): AxumPath<String>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let (_, settings) = match browser_account(&state, &id).await {
        Ok(found) => found,
        Err(refusal) => return refusal,
    };
    let Some(page) = restless_engine::account_browser::sign_in_page(&settings) else {
        return api_error(StatusCode::BAD_REQUEST, "accounts", "The account has no sign-in page.");
    };
    match restless_engine::account_browser::begin_sign_in(&id, &page).await {
        Ok(session) => Json(serde_json::json!({
            "session": session,
            "desktop_url": format!(
                "/api/accounts/{id}/desktop/vnc.html?autoconnect=1&reconnect=1&reconnect_delay=1000&resize=remote&path=api/accounts/{id}/desktop/websockify"
            ),
        }))
        .into_response(),
        Err(error) => {
            tracing::warn!(account = %id, "identity browser did not start: {error:#}");
            api_error(StatusCode::SERVICE_UNAVAILABLE, "accounts", "The account's browser did not start. Try again in a moment.")
        }
    }
}

/// The owner finished signing in: the browser keeps its profile, and the account's health says
/// whether the sign-in took.
pub(super) async fn sign_in_done(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((id, session)): AxumPath<(String, String)>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let (_, settings) = match browser_account(&state, &id).await {
        Ok(found) => found,
        Err(refusal) => return refusal,
    };
    if !restless_engine::account_browser::end_sign_in(&id, &session).await {
        return api_error(StatusCode::NOT_FOUND, "accounts", "That sign-in has ended; start it again.");
    }
    let domains = restless_engine::account_browser::account_domains(&settings);
    let health = match restless_engine::account_browser::signed_in(&id, &domains).await {
        Ok(true) => "signed_in",
        Ok(false) => "signed_out",
        Err(_) => "unknown",
    };
    let pool = state.daemon.authority.pool();
    if let Err(error) = accounts::set_health(pool, &id, health).await {
        return refuse(error);
    }
    match accounts::get(pool, &id).await {
        Ok(Some(account)) => Json(account).into_response(),
        Ok(None) => api_error(StatusCode::NOT_FOUND, "accounts", "That account no longer exists."),
        Err(error) => refuse(error),
    }
}

pub(super) async fn sign_in_cancel(
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((id, session)): AxumPath<(String, String)>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    restless_engine::account_browser::end_sign_in(&id, &session).await;
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CodeAnswer {
    code: String,
}

/// The owner gives a sign-in code an agent is waiting on (S98-11): it is typed into the waiting
/// page on the account's own domains and never stored or logged.
pub(super) async fn answer_code(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, id)): AxumPath<(String, String)>,
    Json(input): Json<CodeAnswer>,
) -> Response<Body> {
    use restless_engine::account_browser as identity;
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let Some(code) = identity::clean_code(&input.code) else {
        return api_error(StatusCode::BAD_REQUEST, "accounts", "Type the code as the site sent it: 4 to 12 letters or digits.");
    };
    let Some(request) = identity::take_code_request(&company, &id).await else {
        return api_error(StatusCode::NOT_FOUND, "accounts", "That sign-in no longer waits for a code.");
    };
    let typed = identity::fill(&request.account_id, &request.domains, request.target.as_deref(), &code, "code").await;
    let pool = state.daemon.authority.pool();
    let usable = restless_engine::account_broker::usable(pool, &company, &request.actor)
        .await
        .ok()
        .and_then(|accounts| accounts.into_iter().find(|usable| usable.account.id == request.account_id));
    if let Some(usable) = &usable {
        let _ = restless_engine::account_broker::record_use(
            pool,
            usable,
            &company,
            &request.actor,
            Some(principal.actor_id()),
            "sign in (code from the owner)",
            "act",
            request.domains.first().map(String::as_str),
            if typed.is_ok() { "ok" } else { "failed" },
            None,
        )
        .await;
    }
    match typed {
        Ok(_) => Json(serde_json::json!({ "typed": true, "message": format!("Typed into {}'s sign-in.", request.account_label) })).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            identity::return_code_request(request).await;
            api_error(StatusCode::CONFLICT, "accounts", format!("The code was not typed: {message} Try again once the page shows the code field."))
        }
    }
}

/// The account browser's desktop page and assets, for the owner only.
pub(super) async fn desktop_asset(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((id, asset)): AxumPath<(String, String)>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    if let Err(refusal) = browser_account(&state, &id).await {
        return refusal;
    }
    let key = restless_engine::account_browser::identity_key(&id);
    match runtime::desktop_asset(&key, &asset).await {
        Ok(bytes) => {
            let mut response = Response::new(Body::from(bytes));
            response.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static(super::desktop_api::desktop_content_type(&asset)),
            );
            response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        Err(_) => api_error(StatusCode::BAD_GATEWAY, "desktop", "The account's browser is not running; start its sign-in again."),
    }
}

/// The account browser's desktop socket: full control, for the owner signing in by hand.
pub(super) async fn desktop_socket(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(id): AxumPath<String>,
    upgrade: WebSocketUpgrade,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    if let Err(refusal) = browser_account(&state, &id).await {
        return refusal;
    }
    let key = restless_engine::account_browser::identity_key(&id);
    upgrade
        .on_upgrade(move |socket| async move {
            if let Err(error) = relay_identity_desktop(socket, &key).await {
                tracing::debug!(identity = %key, "identity desktop ended: {error:#}");
            }
        })
        .into_response()
}

async fn relay_identity_desktop(browser: WebSocket, key: &str) -> anyhow::Result<()> {
    use futures_util::{SinkExt as _, StreamExt as _};
    let stream = runtime::desktop_stream(key).await?;
    let (desktop, _) = client_async("ws://127.0.0.1:6080/websockify", stream).await?;
    let (mut to_browser, mut from_browser) = browser.split();
    let (mut to_desktop, mut from_desktop) = desktop.split();
    loop {
        tokio::select! {
            message = from_browser.next() => {
                let Some(Ok(message)) = message else { break };
                let forwarded = match message {
                    AxumMessage::Binary(bytes) => tungstenite::Message::Binary(bytes),
                    AxumMessage::Ping(bytes) => tungstenite::Message::Ping(bytes),
                    AxumMessage::Pong(bytes) => tungstenite::Message::Pong(bytes),
                    AxumMessage::Text(_) => continue,
                    AxumMessage::Close(_) => break,
                };
                to_desktop.send(forwarded).await?;
            }
            message = from_desktop.next() => {
                let Some(Ok(message)) = message else { break };
                let forwarded = match message {
                    tungstenite::Message::Binary(bytes) => AxumMessage::Binary(bytes),
                    tungstenite::Message::Ping(bytes) => AxumMessage::Ping(bytes),
                    tungstenite::Message::Pong(bytes) => AxumMessage::Pong(bytes),
                    tungstenite::Message::Close(_) => break,
                    _ => continue,
                };
                to_browser.send(forwarded).await?;
            }
        }
    }
    Ok(())
}

#[derive(Deserialize)]
pub(super) struct UseQuery {
    company: Option<String>,
    actor: Option<String>,
    directed_by: Option<String>,
    tier: Option<String>,
    before: Option<chrono::DateTime<chrono::Utc>>,
    limit: Option<i64>,
}

async fn use_page(state: &OwnerState, filter: accounts::UseFilter<'_>, limit: i64) -> Response<Body> {
    match accounts::uses(state.daemon.authority.pool(), filter, limit).await {
        Ok(uses) => {
            let next_before = (uses.len() as i64 == limit.clamp(1, 200)).then(|| uses.last().map(|last| last.at)).flatten();
            Json(serde_json::json!({ "uses": uses, "next_before": next_before })).into_response()
        }
        Err(error) => refuse(error),
    }
}

pub(super) async fn uses(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(id): AxumPath<String>,
    Query(query): Query<UseQuery>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let filter = accounts::UseFilter {
        account: Some(&id),
        company: query.company.as_deref(),
        actor: query.actor.as_deref(),
        directed_by: query.directed_by.as_deref(),
        tier: query.tier.as_deref(),
        before: query.before,
    };
    use_page(&state, filter, query.limit.unwrap_or(50)).await
}

/// One company's recent uses of the owner's accounts, for anyone in it (S98-09).
pub(super) async fn company_uses(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<UseQuery>,
) -> Response<Body> {
    if !principal.permits_company(&company) {
        return api_error(StatusCode::FORBIDDEN, "accounts", "You are not in that company.");
    }
    let filter = accounts::UseFilter {
        company: Some(&company),
        actor: query.actor.as_deref(),
        directed_by: query.directed_by.as_deref(),
        tier: query.tier.as_deref(),
        before: query.before,
        ..Default::default()
    };
    use_page(&state, filter, query.limit.unwrap_or(50)).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NoticeDecision {
    decision: String,
}

/// The owner settles a notice: Fine, or Freeze (which freezes that company's grant). Answers the use.
pub(super) async fn decide_notice(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((id, notice)): AxumPath<(String, String)>,
    Json(input): Json<NoticeDecision>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let pool = state.daemon.authority.pool();
    match restless_engine::account_broker::decide_notice(pool, &id, &notice, &input.decision, principal.actor_id()).await {
        Ok(Some(use_id)) => {
            let found = accounts::uses(pool, accounts::UseFilter { account: Some(&id), ..Default::default() }, 200)
                .await
                .ok()
                .and_then(|uses| uses.into_iter().find(|row| row.id == use_id));
            match found {
                Some(row) => Json(row).into_response(),
                None => StatusCode::NO_CONTENT.into_response(),
            }
        }
        Ok(None) => api_error(StatusCode::NOT_FOUND, "accounts", "That notice no longer exists."),
        Err(error) => refuse(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NewRequest {
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    abilities: Option<accounts::Abilities>,
    #[serde(default)]
    reason: String,
}

/// A company asks the owner for an account, or for more on one (S98-08). Admins and the owner.
pub(super) async fn request(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<NewRequest>,
) -> Response<Body> {
    if !principal.permits_company(&company) || !matches!(principal.membership_role(), "owner" | "admin") {
        return api_error(StatusCode::FORBIDDEN, "accounts", "Only the company's admins ask the owner for accounts.");
    }
    match restless_engine::account_broker::create_request(
        state.daemon.authority.pool(),
        &company,
        input.account_id.as_deref(),
        input.kind.as_deref(),
        input.abilities,
        &input.reason,
        principal.actor_id(),
    )
    .await
    {
        Ok(request) => (StatusCode::CREATED, Json(request)).into_response(),
        Err(error) => refuse(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RequestDecision {
    decision: String,
    #[serde(default)]
    grant_id: Option<String>,
}

pub(super) async fn settle_request(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(request): AxumPath<String>,
    Json(input): Json<RequestDecision>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    match restless_engine::account_broker::settle_request(
        state.daemon.authority.pool(),
        &request,
        &input.decision,
        input.grant_id.as_deref(),
        principal.actor_id(),
    )
    .await
    {
        Ok(Some(request)) => Json(request).into_response(),
        Ok(None) => api_error(StatusCode::NOT_FOUND, "accounts", "That request no longer exists."),
        Err(error) => refuse(error),
    }
}

/// The accounts granted to one company, for anyone in it.
pub(super) async fn company(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if !principal.permits_company(&company) {
        return api_error(StatusCode::FORBIDDEN, "accounts", "You are not in that company.");
    }
    match accounts::company_grants(state.daemon.authority.pool(), &company).await {
        Ok(grants) => Json(serde_json::json!({
            "grants": grants.into_iter().map(|(grant, account)| {
                let mut value = serde_json::to_value(&grant).unwrap_or_default();
                value["account"] = serde_json::json!({
                    "id": account.id, "kind": account.kind, "label": account.label,
                    "provider": account.provider, "health": account.health,
                });
                value
            }).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(error) => refuse(error),
    }
}
