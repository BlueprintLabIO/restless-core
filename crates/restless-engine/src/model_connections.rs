//! The owner's account-level model connections: API keys and OAuth sign-ins
//! registered once and assignable to companies. A private, file-backed
//! registry beside the plane state; the owner API edits it, and the model
//! gateway and Exec read it.

use std::path::PathBuf;

use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::runtime;

pub fn valid_provider_id(provider: &str) -> bool {
    !provider.is_empty()
        && provider.len() <= 80
        && provider
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
}

pub fn account_oauth_key(
    root: &std::path::Path,
    provider: &str,
    reference: &str,
) -> Result<Option<String>> {
    let registry = load_owner_connections(root)?;
    Ok(registry
        .connections
        .iter()
        .find(|connection| {
            connection.kind == "oauth"
                && connection.provider == provider
                && owner_connection_reference(connection) == reference
        })
        .and_then(|connection| connection.account_key.clone()))
}

pub fn account_assignment_is_granted(
    root: &std::path::Path,
    config: &runtime::CompanyConfig,
    provider: &str,
    id: &str,
) -> Result<bool> {
    let registry = load_owner_connections(root)?;
    let Some(connection) = registry
        .connections
        .iter()
        .find(|connection| connection.provider == provider && connection.id == id)
    else {
        return Ok(false);
    };
    let reference = owner_connection_reference(connection);
    Ok(config
        .credentials
        .get(&format!("model.inference.{provider}"))
        == Some(&reference))
}

/// The company's current connection for `provider`, by label, when it is one
/// of the owner's live connections. An agent whose own assigned connection was
/// removed or replaced falls back to it rather than failing every turn
/// (Sprint 104 A5): the gateway serves the provider through this credential.
pub fn provider_fallback(
    root: &std::path::Path,
    config: &runtime::CompanyConfig,
    provider: &str,
) -> Result<Option<String>> {
    let Some(current) = config
        .credentials
        .get(&format!("model.inference.{provider}"))
    else {
        return Ok(None);
    };
    let registry = load_owner_connections(root)?;
    Ok(registry
        .connections
        .iter()
        .find(|connection| {
            connection.provider == provider && &owner_connection_reference(connection) == current
        })
        .map(|connection| connection.label.clone()))
}

/// Why an agent's assigned connection no longer reaches the company, in the owner's words, with
/// where to fix it. Replacing a provider's connection used to fail every agent still assigned to
/// the old one with only "no longer granted", and nobody could tell who or what had done it.
pub fn why_not_granted(
    root: &std::path::Path,
    config: &runtime::CompanyConfig,
    provider: &str,
    id: &str,
) -> String {
    let Ok(registry) = load_owner_connections(root) else {
        return "This agent's model connection could not be checked. Try again in a moment.".into();
    };
    let Some(assigned) = registry
        .connections
        .iter()
        .find(|connection| connection.provider == provider && connection.id == id)
    else {
        return format!(
            "This agent's {provider} connection was removed from the account. Give the agent another model in Company › Intelligence."
        );
    };
    let current = config
        .credentials
        .get(&format!("model.inference.{provider}"));
    match current.and_then(|reference| {
        registry.connections.iter().find(|connection| &owner_connection_reference(connection) == reference)
    }) {
        Some(other) => format!(
            "This agent still uses {}, but the company now uses {} for {provider}: granting it replaced the earlier one. \
             Give the agent a model on {} in Company › Intelligence, or connect {} again.",
            assigned.label, other.label, other.label, assigned.label
        ),
        None => format!(
            "{} was removed from the company. Connect it again, or give the agent another model, in Company › Intelligence.",
            assigned.label
        ),
    }
}

pub fn account_connection_matches(
    root: &std::path::Path,
    provider: &str,
    reference: &str,
) -> Result<bool> {
    let registry = load_owner_connections(root)?;
    Ok(registry.connections.iter().any(|connection| {
        connection.provider == provider && owner_connection_reference(connection) == reference
    }))
}

pub fn owner_connection_reference(connection: &OwnerModelConnection) -> String {
    if connection.kind == "oauth" {
        format!("omp-oauth:{}@{}", connection.provider, connection.id)
    } else {
        model_connection_reference(&connection.id)
    }
}

pub fn model_connection_reference(id: &str) -> String {
    format!("infisical:/owner/model-connections/{id}/API_KEY")
}

pub fn save_owner_connections(
    root: &std::path::Path,
    registry: &OwnerModelConnections,
) -> Result<()> {
    let path = owner_connections_path(root);
    let bytes = serde_json::to_vec_pretty(registry)?;
    let temporary = root.join(format!(".owner-model-connections-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<()> {
        use std::io::Write as _;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, &path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

pub fn load_owner_connections(root: &std::path::Path) -> Result<OwnerModelConnections> {
    let path = owner_connections_path(root);
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(OwnerModelConnections::default())
        }
        Err(error) => Err(error.into()),
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            bail!("owner connection registry must be a regular file")
        }
        Ok(metadata) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                if metadata.permissions().mode() & 0o077 != 0 {
                    bail!("owner connection registry permissions must be private")
                }
            }
            let bytes = std::fs::read(&path)?;
            let registry: OwnerModelConnections =
                serde_json::from_slice(&bytes).context("parse owner connection registry")?;
            if registry.connections.iter().any(|connection| {
                !matches!(connection.kind.as_str(), "api_key" | "oauth")
                    || !valid_provider_id(&connection.provider)
                    || connection.id.is_empty()
                    || connection.account_key.as_ref().is_some_and(|key| {
                        connection.kind != "oauth"
                            || key.is_empty()
                            || key.len() > 200
                            || key.chars().any(char::is_control)
                    })
            }) {
                bail!(
                    "owner connection registry contains an unsupported connection kind or identity"
                );
            }
            let mut oauth_providers = std::collections::BTreeSet::new();
            if registry.connections.iter().any(|connection| {
                connection.kind == "oauth" && !oauth_providers.insert(connection.provider.as_str())
            }) {
                bail!("owner connection registry contains duplicate OAuth providers");
            }
            Ok(registry)
        }
    }
}

/// A verified provider sign-in replaced its credential. Reload an unadmitted
/// route even when the provider name stayed the same, and lift only credential
/// rejections belonging to companies granted this exact account connection.
pub async fn oauth_connection_restored(
    root: &std::path::Path,
    provider: &str,
    authority: &crate::authority::AuthorityStore,
) -> Result<()> {
    let registry = load_owner_connections(root)?;
    let connection = registry
        .connections
        .iter()
        .find(|connection| connection.provider == provider && connection.kind == "oauth")
        .context("restored OAuth connection was not registered")?;
    let reference = owner_connection_reference(connection);
    crate::model_gateway::note_credential_replaced();
    for company in crate::configured_companies(root)? {
        let config = match runtime::CompanyConfig::load(root, &company) {
            Ok(config) => config,
            Err(error) => {
                tracing::warn!(company, %error, "could not inspect restored model access");
                continue;
            }
        };
        if config
            .credentials
            .get(&format!("model.inference.{provider}"))
            != Some(&reference)
        {
            continue;
        }
        if let Err(error) = authority
            .clear_provider_credential_cooldowns(&company, provider)
            .await
        {
            tracing::warn!(company, %error, "could not clear restored credential rejection");
        }
    }
    Ok(())
}

pub fn account_oauth_providers(root: &std::path::Path) -> Result<Vec<String>> {
    Ok(load_owner_connections(root)?
        .connections
        .into_iter()
        .filter(|connection| connection.kind == "oauth")
        .map(|connection| connection.provider)
        .collect())
}

pub fn owner_connections_path(root: &std::path::Path) -> PathBuf {
    root.join("owner-model-connections.json")
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerModelConnections {
    pub connections: Vec<OwnerModelConnection>,
}

pub fn default_owner_connection_kind() -> String {
    "api_key".into()
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerModelConnection {
    pub id: String,
    pub label: String,
    pub provider: String,
    #[serde(default = "default_owner_connection_kind")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_key: Option<String>,
}

/// What a provider said when shown an API key, before the key is saved. Probe, never guess: a key
/// the provider explicitly refuses is not stored as if it worked.
#[derive(Debug, PartialEq, Eq)]
pub enum KeyCheck {
    Accepted,
    /// The provider answered 401 or 403; the text is its own short reason.
    Rejected(String),
    /// No answer this check can rely on (unknown provider, network, an unexpected status).
    Unverified,
}

/// The cheapest authenticated read each known provider offers, and how the key is presented.
fn key_check_request(provider: &str) -> Option<(&'static str, &'static str)> {
    Some(match provider {
        "anthropic" => ("https://api.anthropic.com/v1/models?limit=1", "x-api-key"),
        "openai" => ("https://api.openai.com/v1/models", "bearer"),
        "openrouter" => ("https://openrouter.ai/api/v1/key", "bearer"),
        "google" => (
            "https://generativelanguage.googleapis.com/v1beta/models?pageSize=1",
            "x-goog-api-key",
        ),
        "groq" => ("https://api.groq.com/openai/v1/models", "bearer"),
        "mistral" => ("https://api.mistral.ai/v1/models", "bearer"),
        "xai" => ("https://api.x.ai/v1/models", "bearer"),
        // Meta's Model API (Muse), as OMP's "meta" provider validates it.
        "meta" => ("https://api.meta.ai/v1/models", "bearer"),
        "deepseek" => ("https://api.deepseek.com/models", "bearer"),
        _ => return None,
    })
}

/// Ask the provider whether it accepts this key. Spends no tokens. When the gateway relays
/// Anthropic to another base URL (a scripted test model, `tools/mock-model`), the key is checked
/// there too, the same place every turn will go.
pub async fn check_api_key(provider: &str, key: &str) -> KeyCheck {
    if provider == "anthropic" {
        if let Ok(base) = std::env::var("RESTLESS_ANTHROPIC_BASE_URL") {
            let url = format!("{}/models?limit=1", base.trim_end_matches('/'));
            return check_api_key_at(&url, "x-api-key", key).await;
        }
    }
    match key_check_request(provider) {
        Some((url, style)) => check_api_key_at(url, style, key).await,
        None => KeyCheck::Unverified,
    }
}

pub async fn check_api_key_at(url: &str, style: &str, key: &str) -> KeyCheck {
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    else {
        return KeyCheck::Unverified;
    };
    let request = match style {
        "bearer" => client.get(url).bearer_auth(key),
        header => client
            .get(url)
            .header(header, key)
            .header("anthropic-version", "2023-06-01"),
    };
    let Ok(response) = request.send().await else {
        return KeyCheck::Unverified;
    };
    let status = response.status();
    if status.is_success() {
        return KeyCheck::Accepted;
    }
    let body = response.text().await.unwrap_or_default();
    let message = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value["error"]["message"]
                .as_str()
                .or_else(|| value["error"].as_str())
                .or_else(|| value["message"].as_str())
                .or_else(|| value["detail"].as_str())
                .map(str::to_owned)
        })
        .unwrap_or_default();
    let lower = message.to_lowercase();
    // Most providers refuse with 401 or 403; Google and xAI answer 400 naming the key.
    let refused = status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::FORBIDDEN
        || (status == reqwest::StatusCode::BAD_REQUEST
            && lower.contains("api key")
            && ["not valid", "invalid", "incorrect"]
                .iter()
                .any(|word| lower.contains(word)));
    if !refused {
        return KeyCheck::Unverified;
    }
    let reason = if message.is_empty() || lower.contains("missing authentication") {
        "the key was not recognised".to_owned()
    } else {
        // The provider's words, bounded and on one line; never the key.
        let message = if key.len() >= 8 {
            message.replace(key, "…")
        } else {
            message
        };
        message.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    KeyCheck::Rejected(reason.chars().take(160).collect())
}

#[cfg(test)]
mod key_check_tests {
    use super::*;

    async fn fake_provider() -> String {
        use axum::{http::HeaderMap, routing::get, Json, Router};
        let app = Router::new()
            .route(
                "/v1beta/models",
                get(|| async {
                    (
                        axum::http::StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error":{"code":400,"message":"API key not valid. Please pass a valid API key."}})),
                    )
                }),
            )
            .route(
            "/v1/models",
            get(|headers: HeaderMap| async move {
                if headers.get("x-api-key").and_then(|value| value.to_str().ok()) == Some("good-key") {
                    (axum::http::StatusCode::OK, Json(serde_json::json!({"data": []})))
                } else {
                    (
                        axum::http::StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}})),
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}/v1/models")
    }

    #[tokio::test]
    async fn a_refused_key_is_rejected_with_the_providers_reason_and_a_good_one_accepted() {
        let url = fake_provider().await;
        assert_eq!(
            check_api_key_at(&url, "x-api-key", "good-key").await,
            KeyCheck::Accepted
        );
        assert_eq!(
            check_api_key_at(&url, "x-api-key", "sk-ant-wrong").await,
            KeyCheck::Rejected("invalid x-api-key".into())
        );
        // Nothing answers: unverified, so a network blip never blocks saving a key.
        assert_eq!(
            check_api_key_at("http://127.0.0.1:9/v1/models", "x-api-key", "k").await,
            KeyCheck::Unverified
        );
        assert_eq!(
            check_api_key("some-new-provider", "k").await,
            KeyCheck::Unverified
        );
        // Google refuses with 400 naming the key.
        let google = url.replace("/v1/models", "/v1beta/models");
        assert_eq!(
            check_api_key_at(&google, "x-goog-api-key", "k").await,
            KeyCheck::Rejected("API key not valid. Please pass a valid API key.".into())
        );
    }
}

#[cfg(test)]
mod provider_fallback_tests {
    use super::*;

    #[test]
    fn an_agent_on_a_replaced_connection_falls_back_to_the_companys_current_one() {
        let root = std::env::temp_dir().join(format!("restless-fallback-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let connection = |label: &str| OwnerModelConnection {
            id: Uuid::new_v4().simple().to_string(),
            label: label.into(),
            provider: "anthropic".into(),
            kind: "api_key".into(),
            account_key: None,
        };
        let (old, new) = (connection("Old key"), connection("New key"));
        save_owner_connections(
            &root,
            &OwnerModelConnections {
                connections: vec![new.clone()],
            },
        )
        .unwrap();
        let config = |reference: Option<String>| -> runtime::CompanyConfig {
            serde_json::from_value(serde_json::json!({
                "name": "fallback_test", "mission": "Fallback fixture",
                "credentials": reference
                    .map(|reference| serde_json::json!({"model.inference.anthropic": reference}))
                    .unwrap_or_else(|| serde_json::json!({}))
            }))
            .unwrap()
        };
        let current = config(Some(owner_connection_reference(&new)));
        assert!(!account_assignment_is_granted(&root, &current, "anthropic", &old.id).unwrap());
        assert_eq!(
            provider_fallback(&root, &current, "anthropic").unwrap(),
            Some("New key".into())
        );
        assert_eq!(
            provider_fallback(&root, &config(None), "anthropic").unwrap(),
            None
        );
        assert_eq!(
            provider_fallback(&root, &config(Some("env:ELSEWHERE".into())), "anthropic").unwrap(),
            None,
            "a credential that is not one of the owner's connections is no fallback"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod oauth_restoration_tests {
    use super::*;

    #[tokio::test]
    async fn restoring_oauth_clears_only_exact_account_credential_rejections() {
        let Ok(database) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping OAuth restoration scenario");
            return;
        };
        struct Root(PathBuf);
        impl Drop for Root {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root =
            Root(std::env::temp_dir().join(format!("restless-oauth-restore-{}", Uuid::new_v4())));
        std::fs::create_dir_all(root.0.join("companies")).unwrap();
        let connection = OwnerModelConnection {
            id: Uuid::new_v4().simple().to_string(),
            label: "Restored test account".into(),
            provider: "openai-codex".into(),
            kind: "oauth".into(),
            account_key: Some("original-account".into()),
        };
        save_owner_connections(
            &root.0,
            &OwnerModelConnections {
                connections: vec![connection.clone()],
            },
        )
        .unwrap();
        let granted = format!("oauth_granted_{}_test", Uuid::new_v4().simple());
        let other = format!("oauth_other_{}_test", Uuid::new_v4().simple());
        let unrelated = format!("oauth_unrelated_{}_test", Uuid::new_v4().simple());
        for (company, reference) in [
            (&granted, owner_connection_reference(&connection)),
            (&other, "env:OTHER_ACCOUNT_KEY".into()),
        ] {
            let config: runtime::CompanyConfig = serde_json::from_value(serde_json::json!({
                "name": company, "mission": "Owned OAuth recovery fixture",
                "credentials": {"model.inference.openai-codex": reference}
            }))
            .unwrap();
            runtime::CompanyConfig::save(&root.0, &config).unwrap();
        }
        let unrelated_config: runtime::CompanyConfig = serde_json::from_value(serde_json::json!({
            "name": unrelated, "mission": "Unrelated credential field",
            "credentials": {"external.service": owner_connection_reference(&connection)}
        }))
        .unwrap();
        runtime::CompanyConfig::save(&root.0, &unrelated_config).unwrap();
        let authority = crate::authority::AuthorityStore::connect(&database)
            .await
            .unwrap();
        let later = chrono::Utc::now() + chrono::Duration::hours(24);
        for (company, model, kind) in [
            (&granted, "openai-codex/gpt-6-sol", "credential"),
            (&granted, "openai-codex/gpt-5.6-sol", "quota"),
            (&granted, "anthropic/claude-sonnet-5-5", "credential"),
            (&other, "openai-codex/gpt-6-sol", "credential"),
            (&unrelated, "openai-codex/gpt-6-sol", "credential"),
        ] {
            authority
                .set_model_cooldown(company, model, kind, "fixture rejection", later)
                .await
                .unwrap();
        }
        oauth_connection_restored(&root.0, "openai-codex", &authority)
            .await
            .unwrap();
        let mut left = authority
            .active_model_cooldowns(&granted)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.model)
            .collect::<Vec<_>>();
        left.sort();
        assert_eq!(
            left,
            ["anthropic/claude-sonnet-5-5", "openai-codex/gpt-5.6-sol"]
        );
        assert_eq!(
            authority
                .active_model_cooldowns(&other)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            authority
                .active_model_cooldowns(&unrelated)
                .await
                .unwrap()
                .len(),
            1
        );
    }
}
