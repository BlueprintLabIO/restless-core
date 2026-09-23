//! Owner-only custom intelligence harness settings and runtime operations.
use super::*;
use crate::custom_harness;
use serde_json::{json, Value};

fn authorize(state: &OwnerState, principal: &RequestPrincipal) -> Option<Response<Body>> {
    if principal.membership_role() != "owner" {
        Some(api_error(
            StatusCode::FORBIDDEN,
            "harness",
            "Only the owner can manage intelligence harnesses.",
        ))
    } else if state.entry.network().is_some() {
        Some(api_error(
            StatusCode::FORBIDDEN,
            "harness",
            "Manage intelligence on the account host.",
        ))
    } else {
        None
    }
}

pub(super) async fn list(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if let Some(error) = authorize(&state, &principal) {
        return error;
    }
    let result: Result<Value> = async {
        runtime::CompanyConfig::load(&state.daemon.root,&company)?;
        let registry=custom_harness::load(&state.daemon.root,&company)?;
        let revision=custom_harness::revision(&registry);
        let company_ref=&company;
        let root_ref=&state.daemon.root;
        let rows=futures_util::future::join_all(registry.iter().map(|(id,harness)| async move {
            let status=custom_harness::status(company_ref,id).await.unwrap_or_else(|_|json!({"state":"unavailable","message":"Start the company computer to check installation."}));
            let checking=custom_harness::refresh_if_due(root_ref,company_ref,id,harness,status["state"]=="installed");
            json!({"id":id,"config":harness,"installation":status,"checking":checking,"discovery":custom_harness::cached_probe(root_ref,company_ref,id,harness)})
        })).await;
        Ok(json!({"revision":revision,"harnesses":rows,"presets":custom_harness::presets()}))
    }.await;
    match result {
        Ok(value) => Json(value).into_response(),
        Err(_) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "harness",
            "Could not read custom harness settings.",
        ),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SaveInput {
    revision: String,
    config: custom_harness::Harness,
}
pub(super) async fn save(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, id)): AxumPath<(String, String)>,
    Json(input): Json<SaveInput>,
) -> Response<Body> {
    if let Some(error) = authorize(&state, &principal) {
        return error;
    }
    let _write = state.charter_writes.lock().await;
    let result: Result<Value> = async {
        runtime::CompanyConfig::load(&state.daemon.root, &company)?;
        custom_harness::validate_id(&id)?;
        input.config.validate()?;
        let mut registry = custom_harness::load(&state.daemon.root, &company)?;
        if custom_harness::revision(&registry) != input.revision {
            bail!("Settings changed. Refresh and try again.");
        }
        if registry.len() >= 24 && !registry.contains_key(&id) {
            bail!("At most 24 custom harnesses may be configured per company.");
        }
        if registry.contains_key(&id)
            && custom_harness::status(&company, &id).await?["state"] == "installing"
        {
            bail!("Wait for this installation to finish before changing its settings.");
        }
        registry.insert(id, input.config);
        custom_harness::save(&state.daemon.root, &company, &registry)?;
        Ok(json!({"saved":true,"revision":custom_harness::revision(&registry)}))
    }
    .await;
    match result {
        Ok(value) => Json(value).into_response(),
        Err(e) => api_error(StatusCode::BAD_REQUEST, "harness", e.to_string()),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ActionInput {
    action: String,
    revision: String,
    model: Option<String>,
    secret: Option<String>,
    environment_name: Option<String>,
}
pub(super) async fn action(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, id)): AxumPath<(String, String)>,
    Json(input): Json<ActionInput>,
) -> Response<Body> {
    if let Some(error) = authorize(&state, &principal) {
        return error;
    }
    let _write = state.charter_writes.lock().await;
    let result: Result<Value> = async {
        runtime::CompanyConfig::load(&state.daemon.root, &company)?;
        let mut registry = custom_harness::load(&state.daemon.root, &company)?;
        if custom_harness::revision(&registry) != input.revision {
            bail!("Settings changed. Refresh and try again.");
        }
        let harness = registry.get(&id).context("Save this harness first")?.clone();
        match input.action.as_str() {
            "install" => {
                if custom_harness::status(&company, &id).await?["state"] == "installing" {
                    bail!("Installation is already running");
                }
                custom_harness::install(&company, &id, &harness).await?;
                Ok(json!({"state":"starting"}))
            }
            "probe" => {
                let result =
                    custom_harness::probe(&company, &id, &harness, input.model.as_deref()).await?;
                custom_harness::cache_probe(&state.daemon.root, &company, &id, &harness, &result)?;
                Ok(result)
            }
            "setup" => {
                if custom_harness::status(&company,&id).await?["state"]!="installed" {bail!("Install this harness first");}
                custom_harness::setup(&company,&id,&harness).await?;
                custom_harness::invalidate_probe(&state.daemon.root,&company,&id)?;
                Ok(json!({"state":"setup_opened","href":format!("/{company}/company/computer?focus=desktop")}))
            }
            "api_key" => {
                let key=input.environment_name.as_deref().context("Choose the API key environment name")?;
                let secret=input.secret.as_deref().filter(|value| !value.trim().is_empty() && value.len()<=32768).context("Enter an API key")?;
                let mut updated=harness;
                let reference=format!("infisical:/companies/{company}/CUSTOM_HARNESS_{}_{}",id.replace('-',"_"),key);
                updated.environment.remove(key);
                updated.credentials.insert(key.into(),reference.clone());
                updated.validate()?;
                credential::store_reference(&reference,secret).await.map_err(|_|anyhow::anyhow!("Could not save the harness key in Infisical"))?;
                registry.insert(id,updated);custom_harness::save(&state.daemon.root,&company,&registry)?;
                Ok(json!({"saved":true,"revision":custom_harness::revision(&registry)}))
            }
            _ => bail!("Unknown harness action"),
        }
    }
    .await;
    match result {
        Ok(value) => Json(value).into_response(),
        Err(e) => api_error(StatusCode::BAD_REQUEST, "harness", e.to_string()),
    }
}
