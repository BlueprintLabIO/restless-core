//! Core's client for a company's native Documents sidecar: where it runs (per
//! cell DNS, or the local appliance's service), its readiness contract, and
//! agent body commands carried under a short-lived, document-scoped token.

#[cfg(test)]
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use axum::http::header::ACCEPT;
use axum::http::HeaderMap;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::document_collaboration_token::{
    DocumentCollaborationAccess, DocumentCollaborationTokenInput, DEFAULT_TTL_SECONDS,
};
use crate::owner_cell_readiness::ReadinessSecret;
use crate::owner_config::OwnerConfig;
use crate::Daemon;

pub const NATIVE_DOCUMENTS_READINESS_PATH: &str = "/internal/v1/native-documents/ready";

pub const NATIVE_DOCUMENTS_SERVICE_PORT: u16 = 6688;

pub const NATIVE_DOCUMENTS_PROTOCOL_VERSION: u32 = 1;

pub const NATIVE_DOCUMENTS_SCHEMA_VERSION: u32 = 1;

pub const MAX_NATIVE_DOCUMENTS_HEALTH_BYTES: usize = 2 * 1024;

pub const CELL_READINESS_TOKEN_FILE_ENV: &str = "RESTLESS_CELL_READINESS_TOKEN_FILE";

#[derive(Clone)]
pub struct NativeDocumentsProxy {
    client: reqwest::Client,
    readiness_secret: Option<ReadinessSecret>,
    service: NativeDocumentsService,
}

impl NativeDocumentsProxy {
    pub fn from_environment() -> Result<Self> {
        if std::env::var_os("RESTLESS_CELL_READINESS_TOKEN").is_some() {
            anyhow::bail!("native Documents readiness requires {CELL_READINESS_TOKEN_FILE_ENV}, not an environment bearer");
        }
        let readiness_secret = match std::env::var_os(CELL_READINESS_TOKEN_FILE_ENV) {
            Some(raw) => {
                let path = std::path::PathBuf::from(raw);
                if !path.is_absolute() {
                    anyhow::bail!(
                        "{CELL_READINESS_TOKEN_FILE_ENV} must be an absolute secret-file path"
                    );
                }
                Some(ReadinessSecret::read(&path)?)
            }
            None => None,
        };
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(1_500))
            .timeout(Duration::from_secs(2))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("build native Documents readiness client")?;
        Ok(Self {
            client,
            readiness_secret,
            service: NativeDocumentsService::PerCellDns,
        })
    }

    pub fn disabled_for_test() -> Self {
        Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("test native Documents client"),
            readiness_secret: None,
            service: NativeDocumentsService::PerCellDns,
        }
    }

    pub fn use_local_services(&mut self, root: std::path::PathBuf) {
        self.service = NativeDocumentsService::Local { root };
    }

    pub async fn agent_body(
        &self,
        root: &std::path::Path,
        org: &restless_orgintel::OrgIntel,
        actor: &str,
        document: Uuid,
        issuer: &str,
        payload: &Value,
    ) -> Result<Value> {
        let access = match org.document_access_for_actor(document, actor).await? {
            Some(restless_orgintel::DocumentAccess::Edit) => DocumentCollaborationAccess::Write,
            Some(_) if payload["action"] == "read" => DocumentCollaborationAccess::Read,
            _ => anyhow::bail!("native document is unavailable"),
        };
        let identity = org
            .company_access_identity()
            .await?
            .context("Documents collaboration is not provisioned")?;
        let signer =
            crate::document_collaboration_token::DocumentCollaborationTokenIssuer::open(root)?;
        let token = signer.issue(DocumentCollaborationTokenInput {
            issuer,
            session_principal: &format!("agent:{actor}"),
            company_id: identity.company_id,
            document_id: document,
            actor_id: actor,
            access,
            now: Utc::now(),
            ttl_seconds: DEFAULT_TTL_SECONDS,
        })?;
        let (host, port) = self.service_address(identity.cell_id)?;
        let url = format!(
            "http://{host}:{port}/api/companies/{}/documents/{document}/collaboration/body",
            identity.company_id
        );
        let mut response = self
            .client
            .post(&url)
            .timeout(Duration::from_secs(30))
            .bearer_auth(token)
            .json(payload)
            .send()
            .await
            .context("reach live document body")?;
        if response.status() != reqwest::StatusCode::OK {
            anyhow::bail!(
                "live document command failed (HTTP {}); retain the same edit key on retry",
                response.status().as_u16()
            );
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            anyhow::ensure!(
                bytes.len() + chunk.len() <= 12 * 1024 * 1024,
                "live document response exceeds limit"
            );
            bytes.extend_from_slice(&chunk);
        }
        Ok(serde_json::from_slice(&bytes).context("decode live document response")?)
    }

    pub fn service_address(&self, cell_id: Uuid) -> Result<(String, u16)> {
        Ok(match &self.service {
            NativeDocumentsService::Local { root } => {
                return crate::local_documents::address(root, cell_id)
            }
            NativeDocumentsService::PerCellDns => (
                format!("restless-docs-{cell_id}"),
                NATIVE_DOCUMENTS_SERVICE_PORT,
            ),
            #[cfg(test)]
            NativeDocumentsService::Fixed { host, port } => (host.to_string(), *port),
        })
    }

    pub fn readiness_authorized(&self, headers: &HeaderMap) -> bool {
        self.readiness_secret
            .as_ref()
            .is_some_and(|secret| secret.authorizes(headers))
    }

    pub async fn observe_readiness(&self, cell_id: Uuid) -> Result<NativeDocumentsHealth> {
        let (host, port) = self.service_address(cell_id)?;
        let url = format!("http://{host}:{port}{NATIVE_DOCUMENTS_READINESS_PATH}");
        let mut response = self
            .client
            .get(&url)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .context("reach native Documents sidecar readiness")?;
        if response.status() != reqwest::StatusCode::OK || response.url().as_str() != url {
            anyhow::bail!("native Documents sidecar is not ready");
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .split(';')
            .next()
            .unwrap_or_default()
            .trim();
        if content_type != "application/json" {
            anyhow::bail!("native Documents readiness response is not JSON");
        }
        if response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok())
            .is_some_and(|length| length == 0 || length > MAX_NATIVE_DOCUMENTS_HEALTH_BYTES)
        {
            anyhow::bail!("native Documents readiness response is outside its bound");
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .context("read native Documents readiness response")?
        {
            if body.len() + chunk.len() > MAX_NATIVE_DOCUMENTS_HEALTH_BYTES {
                anyhow::bail!("native Documents readiness response is outside its bound");
            }
            body.extend_from_slice(&chunk);
        }
        let observation: NativeDocumentsHealth =
            serde_json::from_slice(&body).context("decode native Documents readiness response")?;
        if observation
            != (NativeDocumentsHealth {
                status: "ready".into(),
                protocol_version: NATIVE_DOCUMENTS_PROTOCOL_VERSION,
                schema_version: NATIVE_DOCUMENTS_SCHEMA_VERSION,
            })
        {
            anyhow::bail!(
                "native Documents readiness response does not match the released contract"
            );
        }
        Ok(observation)
    }
}

#[derive(Clone)]
pub enum NativeDocumentsService {
    PerCellDns,
    Local {
        root: std::path::PathBuf,
    },
    #[cfg(test)]
    Fixed {
        host: Arc<str>,
        port: u16,
    },
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeDocumentsHealth {
    status: String,
    protocol_version: u32,
    schema_version: u32,
}

pub async fn agent_document_body(
    root: &std::path::Path,
    org: &restless_orgintel::OrgIntel,
    actor: &str,
    document: Uuid,
    payload: &serde_json::Value,
) -> Result<serde_json::Value> {
    let config = OwnerConfig::from_env()?;
    let issuer = config.document_issuer();
    let mut proxy = NativeDocumentsProxy::from_environment()?;
    if !config.hosted_runtime() {
        proxy.use_local_services(root.to_owned());
    }
    proxy
        .agent_body(root, org, actor, document, &issuer, payload)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use axum::routing::get;
    use axum::{Json, Router};

    #[test]
    fn native_documents_uses_the_cell_secret_file_and_observes_rotation() {
        use axum::http::header::AUTHORIZATION;
        if std::env::var_os("RESTLESS_DOCS_READINESS_TEST_CHILD").is_some() {
            let proxy = NativeDocumentsProxy::from_environment().unwrap();
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_static("Bearer native-documents-readiness-secret-123456"),
            );
            assert!(proxy.readiness_authorized(&headers));
            println!("MOUNTED_READINESS_AUTHORIZED");
            return;
        }
        struct Scratch(std::path::PathBuf);
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let directory =
            std::env::temp_dir().join(format!("restless-docs-readiness-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let directory = Scratch(directory);
        let path = directory.0.join("cell_readiness_token");
        let secret = b"native-documents-readiness-secret-123456".to_vec();
        std::fs::write(&path, &secret).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let proxy = NativeDocumentsProxy {
            client: reqwest::Client::new(),
            readiness_secret: Some(ReadinessSecret::read(&path).unwrap()),
            service: NativeDocumentsService::PerCellDns,
        };
        // Exercise the actual environment loader in a child so parallel tests
        // never see a process-wide secret environment mutation.
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "documents_service::tests::native_documents_uses_the_cell_secret_file_and_observes_rotation", "--nocapture"])
            .env("RESTLESS_DOCS_READINESS_TEST_CHILD", "1")
            .env(CELL_READINESS_TOKEN_FILE_ENV, &path)
            .env_remove("RESTLESS_CELL_READINESS_TOKEN")
            .output().unwrap();
        assert!(
            child.status.success(),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        assert!(String::from_utf8_lossy(&child.stdout).contains("MOUNTED_READINESS_AUTHORIZED"));
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!(
                "Bearer {}",
                String::from_utf8(secret.clone()).unwrap()
            ))
            .unwrap(),
        );
        assert!(proxy.readiness_authorized(&headers));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer native-documents-readiness-secret-123457"),
        );
        assert!(!proxy.readiness_authorized(&headers));
        let replacement = directory.0.join("replacement");
        std::fs::write(&replacement, b"native-documents-readiness-secret-123457").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&replacement, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        std::fs::rename(&replacement, &path).unwrap();
        assert!(proxy.readiness_authorized(&headers));
        headers.append(AUTHORIZATION, HeaderValue::from_static("Bearer duplicate"));
        assert!(!proxy.readiness_authorized(&headers));
        headers.remove(AUTHORIZATION);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer native-documents-readiness-secret-123456"),
        );
        assert!(!proxy.readiness_authorized(&headers));
        std::fs::remove_file(&path).unwrap();
        assert!(!proxy.readiness_authorized(&headers));
    }

    #[tokio::test]
    async fn sidecar_readiness_requires_the_exact_released_observation() {
        async fn serve(value: serde_json::Value) -> (u16, tokio::task::JoinHandle<()>) {
            let app = Router::new().route(
                NATIVE_DOCUMENTS_READINESS_PATH,
                get(move || {
                    let value = value.clone();
                    async move { Json(value) }
                }),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            (port, task)
        }

        let (port, task) = serve(serde_json::json!({
            "status": "ready",
            "protocol_version": NATIVE_DOCUMENTS_PROTOCOL_VERSION,
            "schema_version": NATIVE_DOCUMENTS_SCHEMA_VERSION,
        }))
        .await;
        let proxy = NativeDocumentsProxy {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            readiness_secret: None,
            service: NativeDocumentsService::Fixed {
                host: "127.0.0.1".into(),
                port,
            },
        };
        assert_eq!(
            proxy.observe_readiness(Uuid::new_v4()).await.unwrap(),
            NativeDocumentsHealth {
                status: "ready".into(),
                protocol_version: NATIVE_DOCUMENTS_PROTOCOL_VERSION,
                schema_version: NATIVE_DOCUMENTS_SCHEMA_VERSION,
            }
        );
        task.abort();

        let (port, task) = serve(serde_json::json!({
            "status": "ready",
            "protocol_version": NATIVE_DOCUMENTS_PROTOCOL_VERSION,
            "schema_version": NATIVE_DOCUMENTS_SCHEMA_VERSION,
            "invented": true,
        }))
        .await;
        let proxy = NativeDocumentsProxy {
            service: NativeDocumentsService::Fixed {
                host: "127.0.0.1".into(),
                port,
            },
            ..proxy
        };
        assert!(proxy.observe_readiness(Uuid::new_v4()).await.is_err());
        task.abort();
    }
}

/// Observe the live Docs service; never provision or mutate company content.
pub async fn document_service_doctor(daemon: &Daemon, company: &str) -> serde_json::Value {
    let observation = async {
        let org = daemon.orgintel.get(company).await?;
        let identity = org
            .company_access_identity()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Documents identity has not been provisioned"))?;
        let mut proxy = crate::documents_service::NativeDocumentsProxy::from_environment()?;
        if !daemon.runtime_bridges.is_hosted() {
            proxy.use_local_services(daemon.root.clone());
        }
        proxy.observe_readiness(identity.cell_id).await
    }
    .await;
    match observation {
        Ok(health) => serde_json::json!({
            "status": "available", "health": health,
            "detail": "Live collaboration service and its storage capability respond; editing and delivery require separate workflow probes."
        }),
        Err(error) => {
            tracing::warn!(%company, %error, "Doctor could not observe Documents readiness");
            serde_json::json!({"status": "unavailable", "detail": "Documents collaboration service or its storage capability is unavailable."})
        }
    }
}
