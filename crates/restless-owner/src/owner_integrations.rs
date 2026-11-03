//! Company › Integrations (S109 H2): one status per integration, in the words agents read too.
use axum::http::StatusCode;
use axum::{
    extract::{Path as AxumPath, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::Deserialize;

use crate::{credential, runtime};
use restless_engine::email_integration;

use super::{api_error, OwnerState, RequestPrincipal};

/// The Email status. Everyone in the company may read it; only the owner and admins see which
/// Vault name holds the key.
pub(super) async fn integrations(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response {
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(_) => return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist."),
    };
    // An unbound secret with a Resend name still sends (credential::resolve_resend_send).
    let inventory = if credential::infisical_configured()
        && !config
            .credentials
            .contains_key(credential::RESEND_SEND_BINDING)
    {
        credential::company_vault_inventory(&company)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let unbound: Vec<&str> = inventory
        .iter()
        .filter_map(|row| row["name"].as_str())
        .collect();
    let key = email_integration::key_name(&config, &unbound);
    let mut email = match email_integration::status(&state.daemon.authority, &company, key).await {
        Ok(status) => status,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "integrations",
                "Email status could not be read. Try again.",
            )
        }
    };
    if !matches!(principal.membership_role(), "owner" | "admin") {
        email.key = None;
    }
    Json(serde_json::json!({ "email": email })).into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TestInput {
    from: String,
    to: String,
}

/// The owner sends one test email; its outcome becomes the status.
pub(super) async fn send_test(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<TestInput>,
) -> Response {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "integrations",
            "Only the owner sends a test email.",
        );
    }
    match email_integration::send_test(
        &state.daemon.root,
        &state.daemon.authority,
        &company,
        input.from.trim(),
        input.to.trim(),
        principal.actor_id(),
    )
    .await
    {
        Ok(sent) => Json(serde_json::json!({ "last_send": sent })).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "integrations", error.to_string()),
    }
}
