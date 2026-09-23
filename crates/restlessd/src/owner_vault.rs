use axum::http::StatusCode;
use axum::{
    extract::{Path as AxumPath, State},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::{credential, runtime};

use super::{api_error, company_setup_view, OwnerState};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoreSecretInput {
    binding: String,
    secret: String,
    revision: String,
}

pub(super) async fn store_secret(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<StoreSecretInput>,
) -> Response {
    if state.entry.network().is_some() {
        return api_error(
            StatusCode::FORBIDDEN,
            "vault",
            "Manage the vault on the account host.",
        );
    }
    if input.secret.trim().is_empty() || input.secret.len() > 32_768 {
        return api_error(
            StatusCode::BAD_REQUEST,
            "vault_secret",
            "Enter a secret up to 32,768 bytes.",
        );
    }

    let _write = state.charter_writes.lock().await;
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(_) => return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist."),
    };
    if company_setup_view(&config)["revision"].as_str() != Some(input.revision.as_str()) {
        return api_error(
            StatusCode::CONFLICT,
            "vault_revision",
            "Company settings changed. Refresh the Vault and try again.",
        );
    }

    let binding = input.binding.trim();
    if binding.is_empty() || binding.starts_with("model.inference") {
        return api_error(
            StatusCode::BAD_REQUEST,
            "vault_binding",
            "Choose an existing non-model Vault binding.",
        );
    }
    let Some(reference) = config.credentials.get(binding) else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "vault_binding",
            "Choose an existing company credential binding.",
        );
    };
    let prefix = format!("infisical:/companies/{company}/");
    let Some(secret_name) = reference.strip_prefix(&prefix) else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "vault_reference",
            "This binding is not stored in this company's Infisical scope.",
        );
    };
    if secret_name.is_empty()
        || !secret_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return api_error(
            StatusCode::BAD_REQUEST,
            "vault_reference",
            "This binding does not use a supported company secret name.",
        );
    }

    if credential::store_reference(reference, &input.secret)
        .await
        .is_err()
    {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "vault",
            "Could not save the secret in Infisical. Check the local Vault service and try again.",
        );
    }
    let probe = credential::probe_reference(reference).await;
    if probe.status != credential::ProbeStatus::Present {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "vault",
            "Infisical did not confirm the stored secret.",
        );
    }

    let mut response = Json(serde_json::json!({
        "binding": binding,
        "reference": reference,
        "status": "present",
    }))
    .into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}
