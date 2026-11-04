//! Models by job (S102, S107): the everyday model that takes Staff chat, routine first tasks and
//! (owner, 4 Nov) team leads' routine turns, whether leads may use it, and the decision model that
//! lets a routine result skip an Exec wake. Written exactly as `restless company set` writes them;
//! owner only, like every other model setting.

use super::*;

pub(super) const PATH: &str = "/companies/{company}/model-jobs";

pub(super) fn view(config: &runtime::CompanyConfig) -> serde_json::Value {
    use restless_engine::decisions::{decision_route, DecisionClient, DecisionRoute};
    let primary = config.for_agent("default");
    let relay = DecisionClient::from_env().is_some();
    let openrouter = config
        .credentials
        .contains_key("model.inference.openrouter")
        || (config.model.starts_with("openrouter/")
            && config.credentials.contains_key("model.inference"));
    let decision_mode = match decision_route(config) {
        DecisionRoute::Off => "off",
        DecisionRoute::OpenRouter(_) if openrouter => "active",
        DecisionRoute::OpenRouter(_) => "unavailable",
        _ if !relay => "unavailable",
        DecisionRoute::Relay => "active",
        DecisionRoute::RelayShadow => "shadow",
    };
    serde_json::json!({
        "revision": company_setup_view(config)["revision"],
        "primary_model": primary.configured_model(),
        "everyday_model": config.everyday_model,
        "everyday": primary.everyday(),
        "suggested_everyday": runtime::everyday_sibling(&primary.model),
        "leads_use_everyday": config.leads_may_use_everyday(),
        "decision_model": config.decision_model,
        "decision_mode": decision_mode,
        "included_decisions": relay,
        "openrouter_connected": openrouter,
    })
}

pub(super) async fn read(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(_) => return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist."),
    };
    let mut body = view(&config);
    // What decisions did lately; absent rather than failing the page when OrgIntel is down.
    if let Ok(org) = state.daemon.orgintel.get(&company).await {
        if let Ok(rows) = org.recent_decisions(None, 500).await {
            body["decisions"] = super::decisions_api::summary(&rows);
        }
    }
    Json(body).into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ModelJobsInput {
    /// A provider/model, `off` (same as the main model) or empty for the default.
    #[serde(default)]
    everyday_model: Option<String>,
    /// `included`, an OpenRouter provider/model, or empty for the default (the relay in shadow).
    #[serde(default)]
    decision_model: Option<String>,
    /// Whether team leads' routine turns may use the everyday model.
    #[serde(default)]
    leads_use_everyday: Option<bool>,
    revision: String,
}

pub(super) async fn update(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<ModelJobsInput>,
) -> Response<Body> {
    let _write = state.charter_writes.lock().await;
    let mut config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(_) => return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist."),
    };
    if company_setup_view(&config)["revision"].as_str() != Some(input.revision.as_str()) {
        return api_error(
            StatusCode::CONFLICT,
            "provider_revision",
            "Company settings changed. Refresh and try again.",
        );
    }
    if let Some(value) = input.everyday_model.as_deref() {
        if let Err(error) = config.set_everyday_model(value) {
            return api_error(
                StatusCode::BAD_REQUEST,
                "everyday_model",
                logged(
                    error,
                    "Choose a model as provider/model, or the main model.",
                ),
            );
        }
    }
    if let Some(value) = input.decision_model.as_deref() {
        if let Err(error) = config.set_decision_model(value) {
            return api_error(
                StatusCode::BAD_REQUEST,
                "decision_model",
                logged(
                    error,
                    "Choose the included decision model or an OpenRouter model.",
                ),
            );
        }
    }
    if let Some(allowed) = input.leads_use_everyday {
        // Unset is the default (on); only "off" is stored.
        config.leads_use_everyday = (!allowed).then_some(false);
    }
    if let Err(error) = runtime::CompanyConfig::save(&state.daemon.root, &config) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "model_jobs",
            logged(error, "The models were not saved. Try again."),
        );
    }
    Json(view(&config)).into_response()
}
