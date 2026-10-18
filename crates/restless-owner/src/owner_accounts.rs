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
    before: Option<chrono::DateTime<chrono::Utc>>,
    limit: Option<i64>,
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
    let limit = query.limit.unwrap_or(50);
    match accounts::uses(state.daemon.authority.pool(), &id, query.company.as_deref(), query.before, limit).await {
        Ok(uses) => {
            let next_before = (uses.len() as i64 == limit.clamp(1, 200)).then(|| uses.last().map(|last| last.at)).flatten();
            Json(serde_json::json!({ "uses": uses, "next_before": next_before })).into_response()
        }
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
