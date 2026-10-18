//! Settings › Accounts (S98-01): the accounts the owner holds and the companies granted them.
//! Owner only, except a company's own grants, which its members see. No response carries a
//! credential value or its vault reference.
use super::*;
use restless_engine::accounts::{self, GrantInput, NewAccount};

fn refuse(error: anyhow::Error) -> Response<Body> {
    // Validation errors are already sentences for the owner; storage errors are not shown raw.
    let message = format!("{error:#}");
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
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
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
