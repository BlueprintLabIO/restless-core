use axum::http::StatusCode;
use axum::{
    extract::{Path as AxumPath, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::Deserialize;

use crate::{credential, runtime};

use super::{api_error, company_setup_view, OwnerState, RequestPrincipal};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoreSecretInput {
    #[serde(default)]
    binding: String,
    secret: String,
    revision: String,
    #[serde(default)]
    create: bool,
    /// The owner confirmed that a new secret may take over an existing name.
    #[serde(default)]
    replace: bool,
    /// Replace the value of a secret already in this company's vault, by its row, whether or not
    /// a connection uses it. Its bindings stay as they are.
    #[serde(default)]
    reference: Option<String>,
}

/// A secret this company's vault owns: under `infisical:/companies/{company}/`, every path
/// segment plain. Anything else (account connections, env, sign-ins) is not the Vault's to change.
fn company_secret_reference(company: &str, reference: &str) -> bool {
    let prefix = format!("infisical:/companies/{company}/");
    reference.strip_prefix(&prefix).is_some_and(|rest| {
        rest.split('/').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
                && segment != "."
                && segment != ".."
        })
    })
}

fn valid_secret_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

pub(super) async fn store_secret(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<StoreSecretInput>,
) -> Response {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "vault",
            "Only the owner can change the vault.",
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
    let mut config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
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

    if let Some(reference) = input.reference.as_deref() {
        if !company_secret_reference(&company, reference) {
            return api_error(
                StatusCode::BAD_REQUEST,
                "vault_reference",
                "This secret is not stored in this company's vault.",
            );
        }
        return store_and_confirm(reference, &input.secret, None).await;
    }

    let binding = credential::canonical_binding(&input.binding);
    if !valid_secret_name(binding) || binding.starts_with("model.inference") {
        return api_error(
            StatusCode::BAD_REQUEST,
            "vault_binding",
            "Choose an existing non-model Vault binding.",
        );
    }
    let existing = config.credentials.get(binding).cloned();
    if input.create && existing.is_some() && !input.replace {
        return api_error(
            StatusCode::CONFLICT,
            "vault_exists",
            format!("A secret named {binding} already exists. Saving would replace its value."),
        );
    }
    let reference = if let Some(reference) = existing {
        reference
    } else if input.create
        && binding.len() <= 64
        && binding
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        format!("infisical:/companies/{company}/{binding}")
    } else {
        return api_error(StatusCode::BAD_REQUEST, "vault_binding", "Choose a connection, or name a new secret using letters, numbers, dots, underscores and hyphens.");
    };
    let new_binding = !config.credentials.contains_key(binding);
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
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return api_error(
            StatusCode::BAD_REQUEST,
            "vault_reference",
            "This binding does not use a supported company secret name.",
        );
    }

    if new_binding {
        // The secret may already exist in the Vault without a company binding.
        // Do not turn a stale list or an unbound secret into an unconfirmed overwrite.
        match credential::probe_reference(&reference).await.status {
            credential::ProbeStatus::Present if !input.replace => {
                return api_error(
                    StatusCode::CONFLICT,
                    "vault_exists",
                    "A secret with that name already exists. Refresh the Vault before replacing it.",
                );
            }
            credential::ProbeStatus::Invalid => {
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "vault",
                    "Could not check whether that secret already exists. Try again.",
                );
            }
            _ => {}
        }
        // The binding is saved only once the value is confirmed below; a failed store leaves the
        // company as it was.
        let stored = store_and_confirm(&reference, &input.secret, Some(binding)).await;
        if !stored.status().is_success() {
            return stored;
        }
        config
            .credentials
            .insert(binding.to_string(), reference.clone());
        if runtime::CompanyConfig::save(&state.daemon.root, &config).is_err() {
            return api_error(StatusCode::SERVICE_UNAVAILABLE, "vault", "The secret was stored, but its company binding could not be saved. Retry after checking host storage.");
        }
        return stored;
    }
    store_and_confirm(&reference, &input.secret, Some(binding)).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RenameSecretInput {
    reference: String,
    name: String,
    revision: String,
}

/// Keep the old value until every consumer points at the confirmed new copy. An interrupted
/// rename can leave an unbound copy, but cannot strand a consumer with a deleted credential.
pub(super) async fn rename_secret(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<RenameSecretInput>,
) -> Response {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "vault",
            "Only the owner can change the vault.",
        );
    }
    let name = credential::canonical_binding(&input.name);
    if !company_secret_reference(&company, &input.reference)
        || !valid_secret_name(name)
        || name.starts_with("model.inference")
    {
        return api_error(StatusCode::BAD_REQUEST, "vault_reference", "Choose a company secret and a name using letters, numbers, dots, underscores and hyphens.");
    }
    let _write = state.charter_writes.lock().await;
    let mut config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
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
    let consumers: Vec<String> = config
        .credentials
        .iter()
        .filter(|(_, reference)| **reference == input.reference)
        .map(|(binding, _)| binding.clone())
        .collect();
    if consumers
        .iter()
        .any(|binding| binding.starts_with("model.inference") || binding.starts_with("finance."))
        || config.native_harnesses.values().any(|connection| {
            connection.credential_reference.as_deref() == Some(input.reference.as_str())
        })
    {
        return api_error(StatusCode::CONFLICT, "vault_in_use", "This connection requires a fixed secret location. Replace its value here or change the connection in its settings.");
    }
    let (directory, _) = input
        .reference
        .rsplit_once('/')
        .expect("validated company reference");
    let destination = format!("{directory}/{name}");
    if destination == input.reference {
        return no_store(Json(
            serde_json::json!({"renamed":true,"cleanup_pending":false}),
        ));
    }
    if config
        .credentials
        .get(name)
        .is_some_and(|reference| reference != &input.reference)
    {
        return api_error(
            StatusCode::CONFLICT,
            "vault_exists",
            "That name is already used by another connection. Choose another name.",
        );
    }
    match credential::probe_reference(&destination).await.status {
        credential::ProbeStatus::Absent => {}
        credential::ProbeStatus::Present => {
            return api_error(
                StatusCode::CONFLICT,
                "vault_exists",
                "A secret with that name already exists. Choose another name.",
            )
        }
        credential::ProbeStatus::Invalid => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "vault",
                "Could not check the destination in the Vault. Try again.",
            )
        }
    }
    let value = match credential::resolve_reference(&input.reference).await {
        Ok(value) => value,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "vault",
                "Could not read the secret to rename it. Refresh the Vault and try again.",
            )
        }
    };
    let stored = store_and_confirm(&destination, &value, None).await;
    if !stored.status().is_success() {
        return stored;
    }
    for reference in config.credentials.values_mut() {
        if *reference == input.reference {
            *reference = destination.clone();
        }
    }
    // Consumers keep their binding names while the row's name and storage location change.
    config
        .credentials
        .insert(name.to_string(), destination.clone());
    if runtime::CompanyConfig::save(&state.daemon.root, &config).is_err() {
        let _ = credential::delete_reference(&destination).await;
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "vault",
            "The rename could not be saved. The original secret and its connections are unchanged.",
        );
    }
    let cleanup_pending = !credential::delete_reference(&input.reference).await;
    no_store(Json(
        serde_json::json!({"renamed":true,"cleanup_pending":cleanup_pending}),
    ))
}

/// Store the value where the reference points, and answer only once Infisical confirms it.
async fn store_and_confirm(reference: &str, secret: &str, binding: Option<&str>) -> Response {
    if credential::store_reference(reference, secret)
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
    no_store(Json(serde_json::json!({
        "binding": binding,
        "reference": reference,
        "status": "present",
    })))
}

fn no_store(body: impl IntoResponse) -> Response {
    let mut response = body.into_response();
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeleteSecretInput {
    reference: String,
    revision: String,
}

/// Delete a secret from this company's vault. Connections that used it lose their binding in the
/// same step, so nothing keeps pointing at a value that is gone; the owner was shown them first.
pub(super) async fn delete_secret(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<DeleteSecretInput>,
) -> Response {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "vault",
            "Only the owner can change the vault.",
        );
    }
    if !company_secret_reference(&company, &input.reference) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "vault_reference",
            "This secret is not stored in this company's vault.",
        );
    }
    let _write = state.charter_writes.lock().await;
    let mut config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
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
    let unbound: Vec<String> = config
        .credentials
        .iter()
        .filter(|(_, reference)| **reference == input.reference)
        .map(|(name, _)| name.clone())
        .collect();
    if unbound
        .iter()
        .any(|name| name.starts_with("model.inference"))
        || config.native_harnesses.values().any(|connection| {
            connection.credential_reference.as_deref() == Some(input.reference.as_str())
        })
    {
        return api_error(
            StatusCode::CONFLICT,
            "vault_in_use",
            "A model connection uses this secret. Change the model connection in Intelligence first.",
        );
    }
    if !credential::delete_reference(&input.reference).await {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "vault",
            "Could not delete the secret in Infisical. Check the local Vault service and try again.",
        );
    }
    if !unbound.is_empty() {
        config
            .credentials
            .retain(|_, reference| *reference != input.reference);
        if runtime::CompanyConfig::save(&state.daemon.root, &config).is_err() {
            return api_error(StatusCode::SERVICE_UNAVAILABLE, "vault", "The secret was deleted, but the connections that used it could not be updated. Retry after checking host storage.");
        }
    }
    no_store(Json(
        serde_json::json!({ "deleted": input.reference, "unbound": unbound }),
    ))
}

#[cfg(test)]
mod tests {
    use super::{company_secret_reference, valid_secret_name};

    #[test]
    fn vault_names_accept_service_bindings_but_never_paths() {
        assert!(valid_secret_name("resend.production"));
        for name in ["", ".", "..", "../other", "mail/key", "a b"] {
            assert!(!valid_secret_name(name), "{name}");
        }
    }

    #[test]
    fn only_this_companys_vault_secrets_can_be_changed_by_reference() {
        assert!(company_secret_reference(
            "aris",
            "infisical:/companies/aris/stripe-key"
        ));
        assert!(company_secret_reference(
            "aris",
            "infisical:/companies/aris/apps/mail_token"
        ));
        assert!(!company_secret_reference(
            "aris",
            "infisical:/companies/other/stripe-key"
        ));
        assert!(!company_secret_reference(
            "aris",
            "infisical:/companies/aris/../other/key"
        ));
        assert!(!company_secret_reference(
            "aris",
            "infisical:/companies/aris/"
        ));
        assert!(!company_secret_reference(
            "aris",
            "infisical:/companies/aris2/key"
        ));
        assert!(!company_secret_reference("aris", "env:STRIPE_KEY"));
    }
}
