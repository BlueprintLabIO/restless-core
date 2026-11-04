//! Company › Integrations (S109 H2): one status per integration, in the words agents read too.
use axum::http::StatusCode;
use axum::{
    extract::{Path as AxumPath, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::Deserialize;

use crate::{credential, runtime};
use restless_engine::{email_integration, hooks, incoming_email};

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
    let incoming = match hooks::endpoint(
        state.daemon.authority.pool(),
        &company,
        hooks::Provider::Resend,
    )
    .await
    {
        Ok(endpoint) => incoming_email::status(endpoint.as_ref(), chrono::Utc::now()),
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "integrations",
                "Email status could not be read. Try again.",
            )
        }
    };
    Json(serde_json::json!({ "email": email, "incoming": incoming })).into_response()
}

/// The owner connects incoming email, or connects it again: Restless registers the company's
/// Resend webhook (replacing one that exists) and records the domains it receives for.
pub(super) async fn connect_incoming(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "integrations",
            "Only the owner connects incoming email.",
        );
    }
    if runtime::CompanyConfig::load(&state.daemon.root, &company).is_err() {
        return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist.");
    }
    match incoming_email::connect(&state.daemon, &company, true).await {
        Ok(incoming) => Json(serde_json::json!({ "incoming": incoming })).into_response(),
        Err(_) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "integrations",
            "Incoming email could not be connected. Try again.",
        ),
    }
}

/// The owner's test: the company emails itself and the status shows when it came back.
pub(super) async fn test_incoming(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "integrations",
            "Only the owner tests incoming email.",
        );
    }
    match incoming_email::send_check(&state.daemon, &company).await {
        Ok(incoming) => Json(serde_json::json!({ "incoming": incoming })).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "integrations", error.to_string()),
    }
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
