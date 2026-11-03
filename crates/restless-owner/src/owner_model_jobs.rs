//! Models by job (S102, S107): the everyday model that takes Staff chat and routine first tasks,
//! and the decision model that lets a routine result skip an Exec wake. Written exactly as
//! `restless company set` writes them; owner only, like every other model setting.

use super::*;

pub(super) const PATH: &str = "/companies/{company}/model-jobs";

pub(super) fn view(config: &runtime::CompanyConfig) -> serde_json::Value {
    let route = restless_engine::decisions::decision_route(config);
    let relay = restless_engine::decisions::DecisionClient::from_env().is_some();
    let decision_mode = match route {
        restless_engine::decisions::DecisionRoute::OpenRouter(_) => "active",
        _ if !relay => "unavailable",
        route if route.active() => "active",
        _ => "shadow",
    };
    serde_json::json!({
        "revision": company_setup_view(config)["revision"],
        "primary_model": config.for_agent("default").configured_model(),
        "everyday_model": config.everyday_model,
        "everyday": config.everyday(),
        "decision_model": config.decision_model,
        "decision_mode": decision_mode,
        "included_decisions": relay,
    })
}

pub(super) async fn read(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => Json(view(&config)).into_response(),
        Err(_) => api_error(StatusCode::NOT_FOUND, "company", "Company does not exist."),
    }
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
    if let Err(error) = runtime::CompanyConfig::save(&state.daemon.root, &config) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "model_jobs",
            logged(error, "The models were not saved. Try again."),
        );
    }
    Json(view(&config)).into_response()
}
