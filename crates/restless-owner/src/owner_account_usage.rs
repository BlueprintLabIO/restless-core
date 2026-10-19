//! Account headroom (S100): how full each AI account's rate-limit windows are, when they reset,
//! which company used them, and when they run out at today's pace. Numbers only: no credential or
//! vault reference ever leaves the plane.
use super::*;
use restless_engine::account_usage;

/// Every account the owner holds that has reported its limits. Owner only.
pub(super) async fn accounts(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
) -> Response<Body> {
    if !principal.is_account_owner() {
        return api_error(StatusCode::FORBIDDEN, "usage", "Only the account owner sees account usage.");
    }
    match account_usage::accounts(state.daemon.authority.pool(), Utc::now()).await {
        Ok(accounts) => Json(serde_json::json!({ "accounts": accounts, "now": Utc::now() })).into_response(),
        Err(error) => {
            tracing::warn!(error = %format!("{error:#}"), "account usage unavailable");
            api_error(StatusCode::SERVICE_UNAVAILABLE, "usage", "Account usage could not be read. Try again.")
        }
    }
}

/// The headroom of the account behind a company's main model, for its members.
pub(super) async fn company(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if !principal.permits_company(&company) {
        return api_error(StatusCode::FORBIDDEN, "usage", "You are not in that company.");
    }
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(_) => return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist."),
    };
    let model = config.configured_model().map(str::to_string);
    let account = model.as_deref().and_then(|model| {
        let provider = model.split('/').next()?;
        let reference = config.credentials.get(&format!("model.inference.{provider}"))?;
        account_usage::account_of(reference)
    });
    let windows = match &account {
        Some(account) => match account_usage::accounts(state.daemon.authority.pool(), Utc::now()).await {
            Ok(accounts) => accounts.into_iter().find(|view| &view.account == account).map(|view| view.windows),
            Err(_) => None,
        },
        None => None,
    };
    Json(serde_json::json!({
        "model": model,
        "account": account,
        "windows": windows.unwrap_or_default(),
        "now": Utc::now(),
    }))
    .into_response()
}
