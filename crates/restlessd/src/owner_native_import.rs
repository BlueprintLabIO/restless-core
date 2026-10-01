//! Explicit, owner-initiated migration of a company-local Codex sign-in.
use super::*;
use base64::Engine as _;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImportCompanyCodexInput {
    company: String,
}

pub(super) async fn import_company_codex(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    Json(input): Json<ImportCompanyCodexInput>,
) -> Response<Body> {
    if !principal.is_account_owner() {
        return api_error(
            StatusCode::FORBIDDEN,
            "connections",
            "Only the account owner can import a sign-in.",
        );
    }
    if state.entry.network().is_some() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "connections",
            "Company sign-in import is available only on a local appliance.",
        );
    }
    let company = input.company.trim();
    if runtime::validate_company_name(company).is_err() {
        return api_error(
            StatusCode::NOT_FOUND,
            "company",
            "Source company does not exist.",
        );
    }
    let source = match runtime::CompanyConfig::load(&state.daemon.root, company) {
        Ok(source) => source,
        _ => {
            return api_error(
                StatusCode::NOT_FOUND,
                "company",
                "Source company does not exist.",
            )
        }
    };
    if !source.native_harnesses.contains_key("codex") {
        return api_error(
            StatusCode::BAD_REQUEST,
            "connections",
            "This company has no Codex sign-in configured.",
        );
    }
    let _write = state.charter_writes.lock().await;
    let mut registry = match load_owner_connections(&state.daemon.root) {
        Ok(registry) => registry,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "connections",
                "Could not check account connections.",
            )
        }
    };
    if registry
        .connections
        .iter()
        .any(|row| row.provider == "openai-codex")
    {
        return api_error(StatusCode::CONFLICT, "connections", "The account already has a Codex connection. Use that connection or reconnect the same account.");
    }
    let companies = match crate::configured_companies(&state.daemon.root) {
        Ok(companies) => companies,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "connections",
                "Could not check company connections.",
            )
        }
    };
    for name in companies {
        let Ok(config) = runtime::CompanyConfig::load(&state.daemon.root, &name) else {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "connections",
                "Could not check company connections.",
            );
        };
        let current = config
            .credentials
            .get("model.inference.openai-codex")
            .or_else(|| {
                config
                    .model
                    .starts_with("openai-codex/")
                    .then(|| config.credentials.get("model.inference"))
                    .flatten()
            });
        if current.is_some_and(|reference| {
            credential::omp_oauth_provider(reference).ok().flatten() != Some("openai-codex")
        }) {
            return api_error(StatusCode::CONFLICT, "connections", "A company already uses a different Codex credential. Change that connection before importing a shared sign-in.");
        }
    }
    let output = tokio::time::timeout(
        Duration::from_secs(8),
        tokio::process::Command::new("docker")
            .args([
                "exec",
                "-u",
                "company",
                &runtime::container_name(company),
                "cat",
                "/company/home/.restless/harness-auth/codex/auth.json",
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await;
    let bytes = match output {
        Ok(Ok(output)) if output.status.success() && output.stdout.len() <= 65_536 => output.stdout,
        _ => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "connections",
                "Could not read this company's Codex sign-in. Check that its computer is running.",
            )
        }
    };
    let profile: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(profile) => profile,
        Err(_) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "connections",
                "The company Codex sign-in has an invalid format.",
            )
        }
    };
    let Some((access, refresh, account_id, expiry_ms)) = parse_codex_profile(&profile) else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "connections",
            "The company Codex sign-in is incomplete. Reconnect it in the company first.",
        );
    };
    match model_gateway::oauth_account_key("openai-codex").await {
        Ok(Some(existing)) if existing == account_id => {}
        Ok(Some(_)) => {
            return api_error(
                StatusCode::CONFLICT,
                "connections",
                "The account broker already holds a different Codex sign-in.",
            )
        }
        Ok(None) => {
            if let Err(error) =
                model_gateway::import_codex_oauth(access, refresh, expiry_ms, account_id).await
            {
                tracing::warn!(company, error = %error, "account Codex import refused");
                return api_error(StatusCode::CONFLICT, "connections", "Could not import this sign-in. Check whether the account already has a different Codex connection.");
            }
        }
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "connections",
                "The account broker is unavailable. Try again when it is running.",
            )
        }
    }
    if model_gateway::oauth_account_key("openai-codex")
        .await
        .ok()
        .flatten()
        .as_deref()
        != Some(account_id)
    {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "The imported Codex account identity could not be verified. No company access was changed.");
    }
    let connection = OwnerModelConnection {
        id: Uuid::new_v4().simple().to_string(),
        label: "ChatGPT / Codex".to_owned(),
        provider: "openai-codex".to_owned(),
        kind: "oauth".to_owned(),
        account_key: Some(account_id.to_owned()),
    };
    registry.connections.push(connection.clone());
    if save_owner_connections(&state.daemon.root, &registry).is_err() {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "connections", "The broker imported the sign-in, but the account connection could not be saved. No company access was changed.");
    }
    Json(serde_json::json!({"connection": safe_connection_summary(&connection, vec![])}))
        .into_response()
}

fn parse_codex_profile(profile: &serde_json::Value) -> Option<(&str, &str, &str, u64)> {
    if profile.get("auth_mode")?.as_str()? != "chatgpt" {
        return None;
    }
    let tokens = profile.get("tokens")?;
    let access = tokens.get("access_token")?.as_str()?;
    let refresh = tokens.get("refresh_token")?.as_str()?;
    let account_id = tokens.get("account_id")?.as_str()?;
    if access.is_empty()
        || refresh.is_empty()
        || account_id.is_empty()
        || account_id.len() > 200
        || account_id.chars().any(char::is_control)
    {
        return None;
    }
    let encoded = access.split('.').nth(1)?;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    let expiry_ms = claims.get("exp")?.as_u64()?.checked_mul(1000)?;
    Some((access, refresh, account_id, expiry_ms))
}
