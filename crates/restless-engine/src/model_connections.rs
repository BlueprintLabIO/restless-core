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

pub fn account_oauth_key(root: &std::path::Path, provider: &str, reference: &str) -> Result<Option<String>> {
    let registry = load_owner_connections(root)?;
    Ok(registry.connections.iter().find(|connection| {
        connection.kind == "oauth" && connection.provider == provider && owner_connection_reference(connection) == reference
    }).and_then(|connection| connection.account_key.clone()))
}

pub fn account_assignment_is_granted(
    root: &std::path::Path,
    config: &runtime::CompanyConfig,
    provider: &str,
    id: &str,
) -> Result<bool> {
    let registry = load_owner_connections(root)?;
    let Some(connection) = registry.connections.iter().find(|connection| {
        connection.provider == provider && connection.id == id
    }) else { return Ok(false); };
    let reference = owner_connection_reference(connection);
    Ok(config.credentials.get(&format!("model.inference.{provider}")) == Some(&reference))
}

pub fn account_connection_matches(root: &std::path::Path, provider: &str, reference: &str) -> Result<bool> {
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

pub fn save_owner_connections(root: &std::path::Path, registry: &OwnerModelConnections) -> Result<()> {
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
                    || connection.account_key.as_ref().is_some_and(|key| connection.kind != "oauth" || key.is_empty() || key.len() > 200 || key.chars().any(char::is_control))
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

pub fn account_oauth_providers(root: &std::path::Path) -> Result<Vec<String>> {
    Ok(load_owner_connections(root)?
        .connections.into_iter().filter(|connection| connection.kind == "oauth")
        .map(|connection| connection.provider).collect())
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
