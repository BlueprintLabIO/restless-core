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
        return api_error(
            StatusCode::FORBIDDEN,
            "usage",
            "Only the account owner sees account usage.",
        );
    }
    refresh_from_broker(&state).await;
    match account_usage::accounts(state.daemon.authority.pool(), Utc::now()).await {
        Ok(accounts) => {
            Json(serde_json::json!({ "accounts": accounts, "now": Utc::now() })).into_response()
        }
        Err(error) => {
            tracing::warn!(error = %format!("{error:#}"), "account usage unavailable");
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "usage",
                "Account usage could not be read. Try again.",
            )
        }
    }
}

/// How often the broker is asked; it asks the providers in turn.
const BROKER_REFRESH: std::time::Duration = std::time::Duration::from_secs(60);
static LAST_BROKER_READ: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

/// Record each account sign-in's limits as the broker reports them, under the same account names
/// a company's calls would record them. A failure only leaves the last reading in place.
async fn refresh_from_broker(state: &OwnerState) {
    {
        let Ok(mut last) = LAST_BROKER_READ.lock() else {
            return;
        };
        if last.is_some_and(|at| at.elapsed() < BROKER_REFRESH) {
            return;
        }
        *last = Some(std::time::Instant::now());
    }
    let reports = match restless_engine::model_gateway::broker_usage().await {
        Ok(reports) => reports,
        Err(error) => {
            tracing::warn!(error = %format!("{error:#}"), "account usage: broker unreadable");
            return;
        }
    };
    let Ok(registry) = load_owner_connections(&state.daemon.root) else {
        return;
    };
    let now = Utc::now();
    for connection in registry
        .connections
        .iter()
        .filter(|connection| connection.kind == "oauth")
    {
        let Some(account) = account_usage::account_of(&owner_connection_reference(connection))
        else {
            continue;
        };
        let mine = reports
            .iter()
            .filter(|report| report.provider == connection.provider);
        // Fall back to the provider's report only when it is the only one: with two Claude
        // sign-ins, taking the first gave one account the other's numbers (21 October).
        let report = mine
            .clone()
            .find(|report| {
                connection.account_key.is_some()
                    && report.account_key.as_deref() == connection.account_key.as_deref()
            })
            .or_else(|| {
                (mine.clone().count() == 1)
                    .then(|| mine.clone().next())
                    .flatten()
            });
        let Some(report) = report else { continue };
        if let Err(error) = account_usage::record(
            state.daemon.authority.pool(),
            &account,
            "",
            &report.windows,
            "broker",
            now,
        )
        .await
        {
            tracing::warn!(%account, error = %format!("{error:#}"), "account usage not recorded");
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
        return api_error(
            StatusCode::FORBIDDEN,
            "usage",
            "You are not in that company.",
        );
    }
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(_) => return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist."),
    };
    let model = config.configured_model().map(str::to_string);
    let account = model.as_deref().and_then(|model| {
        let provider = model.split('/').next()?;
        let reference = config
            .credentials
            .get(&format!("model.inference.{provider}"))?;
        account_usage::account_of(reference)
    });
    if account.is_some() {
        refresh_from_broker(&state).await;
    }
    let windows = match &account {
        Some(account) => {
            match account_usage::accounts(state.daemon.authority.pool(), Utc::now()).await {
                Ok(accounts) => accounts
                    .into_iter()
                    .find(|view| &view.account == account)
                    .map(|view| view.windows),
                Err(_) => None,
            }
        }
        None => None,
    };
    // The sign-in's own name, so the meters say whose limits they are.
    let account_label = account.as_ref().and_then(|account| {
        load_owner_connections(&state.daemon.root)
            .ok()?
            .connections
            .into_iter()
            .find_map(|connection| {
                (account_usage::account_of(&owner_connection_reference(&connection)).as_ref()
                    == Some(account))
                .then_some(connection.label)
            })
    });
    Json(serde_json::json!({
        "model": model,
        "account": account,
        "account_label": account_label,
        "windows": windows.unwrap_or_default(),
        "now": Utc::now(),
    }))
    .into_response()
}
