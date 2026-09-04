//! Bounded artifact launch broker.
//!
//! UI resources carry exact descriptors, never arbitrary URLs or executable
//! paths. The broker resolves those descriptors from Authority/OrgIntel again
//! at Open time, keeps publication material in memory, and launches only the
//! three released shapes below.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use axum::body::Body;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use restless_orgintel::{ArtifactRefRow, ArtifactRefState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::publication::PreparedPublicationAccess;
use crate::{runtime, Daemon};
use restlessd::published_service_contract::ServiceProfile;

pub(crate) const CONTRACT_VERSION: &str = "artifact-launch.v1";
const SESSION_TTL_SECONDS: i64 = 5 * 60;
const MAX_NATIVE_ARCHIVE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_NATIVE_ARCHIVE_ENTRIES: usize = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LaunchShape {
    EmbeddedWeb,
    NativeClient,
    CompanyComputer,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ArtifactLaunchDescriptor {
    pub(crate) contract_version: &'static str,
    pub(crate) shape: LaunchShape,
    pub(crate) availability: String,
    pub(crate) detail: String,
    pub(crate) open_endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) artifact_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) candidate_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) work_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) attempt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) audience: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) publication_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) runtime_generation: Option<String>,
}

impl ArtifactLaunchDescriptor {
    pub(crate) fn computer(company: &str, ready: bool) -> Self {
        Self {
            contract_version: CONTRACT_VERSION,
            shape: LaunchShape::CompanyComputer,
            availability: if ready { "ready" } else { "unavailable" }.into(),
            detail: if ready {
                "Open the private Company Computer stream.".into()
            } else {
                "Start the Company computer before opening its streamed desktop.".into()
            },
            open_endpoint: format!("/api/companies/{company}/resources/company-computer/open"),
            artifact_digest: None,
            candidate_digest: None,
            work_id: None,
            attempt_id: None,
            audience: Some("owner-only".into()),
            expires_at: None,
            platform: None,
            publication_id: None,
            runtime_generation: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeClientRelease {
    pub(crate) contract_version: String,
    pub(crate) platform: String,
    /// Exact zip file in the company computer. The artifact row owns its digest.
    pub(crate) archive_path: String,
    pub(crate) executable_relative_path: String,
    pub(crate) publication_id: String,
}

impl NativeClientRelease {
    pub(crate) fn parse(artifact: &ArtifactRefRow) -> Result<Self> {
        if artifact.kind != "native_client_release" || artifact.state != ArtifactRefState::Available
        {
            bail!("artifact is not an available native_client_release");
        }
        let value: Self = serde_json::from_str(&artifact.note)
            .context("decode native client release manifest")?;
        if value.contract_version != CONTRACT_VERSION {
            bail!("unsupported native client release contract");
        }
        if !matches!(
            value.platform.as_str(),
            "macos-arm64" | "macos-x86_64" | "macos-universal"
        ) {
            bail!("native client platform is not a released macOS target");
        }
        validate_company_path(&value.archive_path, ".zip")?;
        validate_relative_executable(&value.executable_relative_path)?;
        if value.publication_id.is_empty() || value.publication_id.chars().any(char::is_whitespace)
        {
            bail!("native client release has an invalid publication id");
        }
        let digest = artifact
            .digest
            .as_deref()
            .context("native client artifact has no digest")?;
        validate_sha256(digest)?;
        if artifact.work_id.is_none()
            || artifact.attempt_id.is_none()
            || artifact.runtime_generation.is_none()
        {
            bail!("native client release lacks Work, Attempt or Runtime provenance");
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum OpenOutcome {
    Embedded {
        href: String,
        expires_at: DateTime<Utc>,
        reused: bool,
    },
    Native {
        state: String,
        handle: String,
        expires_at: DateTime<Utc>,
        reused: bool,
    },
    CompanyComputer {
        href: String,
    },
    External {
        href: String,
        reason: String,
    },
}

#[derive(Debug, Clone)]
enum SessionKind {
    Web,
    Native,
}

#[derive(Debug, Clone)]
struct LaunchSession {
    handle: String,
    company: String,
    resource_id: String,
    kind: SessionKind,
    target: String,
    token: Option<String>,
    subject: Option<String>,
    candidate_digest: String,
    expires_at: DateTime<Utc>,
    allow_invalid_local_tls: bool,
}

#[derive(Clone)]
pub(crate) struct LaunchBroker {
    root: PathBuf,
    sessions: Arc<Mutex<HashMap<String, LaunchSession>>>,
    by_resource: Arc<Mutex<HashMap<String, String>>>,
}

impl LaunchBroker {
    pub(crate) fn new(root: &Path) -> Result<Self> {
        let root = root.join("launch-cache");
        std::fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            by_resource: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub(crate) fn active_native_resources(&self) -> Vec<String> {
        let mut resources = self
            .sessions
            .lock()
            .map(|sessions| {
                sessions
                    .values()
                    .filter(|session| {
                        matches!(session.kind, SessionKind::Native)
                            && session.expires_at > Utc::now()
                    })
                    .map(|session| format!("{}/{}", session.company, session.resource_id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|_| vec!["registry-unavailable".into()]);
        resources.sort();
        resources.dedup();
        resources
    }

    pub(crate) async fn open(
        &self,
        daemon: &Daemon,
        company: &str,
        resource_id: &str,
    ) -> Result<OpenOutcome> {
        let _lifecycle_lease = daemon.lifecycle.try_enter().context(
            "the stable appliance is draining for replacement; retry Open after activation",
        )?;
        self.prune_expired();
        if resource_id == "company-computer" {
            if runtime::status(company).await? != runtime::ContainerStatus::Running {
                bail!("Company Computer is unavailable; start the company first");
            }
            return Ok(OpenOutcome::CompanyComputer {
                href: format!("/{company}/company/computer"),
            });
        }
        if let Some(publication_id) = resource_id.strip_prefix("published-service:") {
            return self
                .open_published_web(daemon, company, resource_id, publication_id)
                .await;
        }
        if let Some(artifact_id) = resource_id.strip_prefix("artifact:") {
            return self
                .open_native(daemon, company, resource_id, artifact_id)
                .await;
        }
        bail!("resource does not have a released launch profile")
    }

    async fn open_published_web(
        &self,
        daemon: &Daemon,
        company: &str,
        resource_id: &str,
        publication_id: &str,
    ) -> Result<OpenOutcome> {
        if let Some(session) = self.reusable(company, resource_id, SessionKind::Web) {
            return Ok(OpenOutcome::Embedded {
                href: format!("/api/launches/{}", session.handle),
                expires_at: session.expires_at,
                reused: true,
            });
        }
        let org = daemon.orgintel.get(company).await?;
        let access = daemon
            .publication
            .prepare_owner_access(&org, company, publication_id)
            .await?;
        if access.endpoint.profile != ServiceProfile::HttpsWebsocketDemo {
            bail!("this publication is a native server; open its exact native client artifact")
        }
        if let Some(reason) = probe_embedding_policy(&access).await? {
            if access.token.is_none() {
                return Ok(OpenOutcome::External {
                    href: access.endpoint.public_endpoint,
                    reason,
                });
            }
            bail!("the artifact refuses embedding and its private access cannot be placed in an external URL: {reason}");
        }
        let session = self.insert_session(company, resource_id, SessionKind::Web, access)?;
        Ok(OpenOutcome::Embedded {
            href: format!("/api/launches/{}", session.handle),
            expires_at: session.expires_at,
            reused: false,
        })
    }

    async fn open_native(
        &self,
        daemon: &Daemon,
        company: &str,
        resource_id: &str,
        artifact_id: &str,
    ) -> Result<OpenOutcome> {
        if let Some(session) = self.reusable(company, resource_id, SessionKind::Native) {
            return Ok(OpenOutcome::Native {
                state: "running".into(),
                handle: session.handle,
                expires_at: session.expires_at,
                reused: true,
            });
        }
        let artifact_id =
            Uuid::parse_str(artifact_id).context("native artifact id is malformed")?;
        let org = daemon.orgintel.get(company).await?;
        let artifact = org
            .get_artifact_ref(artifact_id)
            .await?
            .context("native artifact does not exist")?;
        let manifest = NativeClientRelease::parse(&artifact)?;
        let expected_platform = if cfg!(target_arch = "aarch64") {
            "macos-arm64"
        } else {
            "macos-x86_64"
        };
        if !cfg!(target_os = "macos")
            || (manifest.platform != expected_platform && manifest.platform != "macos-universal")
        {
            bail!(
                "native client is for {}, this machine requires {expected_platform}",
                manifest.platform
            );
        }
        let generation = runtime::generation(company)
            .await?
            .context("Company Computer has no running generation")?;
        if artifact.runtime_generation.as_deref() != Some(generation.as_str()) {
            bail!("native client belongs to a different Company Runtime generation");
        }
        let access = daemon
            .publication
            .prepare_owner_access(&org, company, &manifest.publication_id)
            .await?;
        if access.endpoint.profile != ServiceProfile::GodotEnetUdp {
            bail!("native client publication does not expose the required Godot ENet profile");
        }
        let executable = self
            .materialize_native(company, &artifact, &manifest)
            .await?;
        let session = self.insert_session(company, resource_id, SessionKind::Native, access)?;
        let owner_port = crate::port_with_offset(7788)?;
        let mut child = tokio::process::Command::new(&executable)
            .env("RESTLESS_LAUNCH_HANDLE", &session.handle)
            .env(
                "RESTLESS_LAUNCH_BROKER_ORIGIN",
                format!("http://127.0.0.1:{owner_port}"),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("launch verified native client {}", executable.display()))?;
        let broker = self.clone();
        let handle = session.handle.clone();
        let until_expiry = (session.expires_at - Utc::now())
            .to_std()
            .unwrap_or_default();
        tokio::spawn(async move {
            tokio::select! {
                _ = child.wait() => {}
                _ = tokio::time::sleep(until_expiry) => {
                    // This task owns the exact Child object it spawned. Never
                    // signal a numeric PID which may have been recycled.
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
            }
            broker.remove_session(&handle);
        });
        Ok(OpenOutcome::Native {
            state: "launched".into(),
            handle: session.handle,
            expires_at: session.expires_at,
            reused: false,
        })
    }

    fn insert_session(
        &self,
        company: &str,
        resource_id: &str,
        kind: SessionKind,
        access: PreparedPublicationAccess,
    ) -> Result<LaunchSession> {
        let handle = Uuid::new_v4().simple().to_string();
        let expires_at = access
            .expires_at
            .min(Utc::now() + chrono::Duration::seconds(SESSION_TTL_SECONDS));
        let session = LaunchSession {
            handle: handle.clone(),
            company: company.to_string(),
            resource_id: resource_id.to_string(),
            kind,
            target: access.endpoint.public_endpoint,
            token: access.token,
            subject: access.subject,
            candidate_digest: access.candidate_digest,
            expires_at,
            allow_invalid_local_tls: access.local_self_signed_tls,
        };
        self.sessions
            .lock()
            .expect("launch sessions")
            .insert(handle.clone(), session.clone());
        self.by_resource
            .lock()
            .expect("launch resource index")
            .insert(format!("{company}\0{resource_id}"), handle);
        Ok(session)
    }

    fn reusable(
        &self,
        company: &str,
        resource_id: &str,
        kind: SessionKind,
    ) -> Option<LaunchSession> {
        let key = format!("{company}\0{resource_id}");
        let handle = self.by_resource.lock().ok()?.get(&key)?.clone();
        let session = self.sessions.lock().ok()?.get(&handle)?.clone();
        let same_kind = matches!(
            (&session.kind, &kind),
            (SessionKind::Web, SessionKind::Web) | (SessionKind::Native, SessionKind::Native)
        );
        (same_kind && session.expires_at > Utc::now()).then_some(session)
    }

    pub(crate) async fn proxy_web(
        &self,
        handle: &str,
        asset: &str,
        headers: &HeaderMap,
    ) -> Response {
        self.prune_expired();
        let Some(session) = self
            .sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(handle).cloned())
        else {
            return (StatusCode::GONE, "launch session expired").into_response();
        };
        if !matches!(session.kind, SessionKind::Web) {
            return (StatusCode::BAD_REQUEST, "launch session is not web content").into_response();
        }
        let asset = asset.trim_start_matches('/');
        if asset.split('/').any(|part| part == "..") {
            return (StatusCode::BAD_REQUEST, "invalid launch asset path").into_response();
        }
        let mut target = match reqwest::Url::parse(&session.target) {
            Ok(url) => url,
            Err(_) => {
                return (StatusCode::BAD_GATEWAY, "released endpoint is malformed").into_response()
            }
        };
        target.set_path(&format!("/{asset}"));
        let client = match reqwest::Client::builder()
            .danger_accept_invalid_certs(session.allow_invalid_local_tls)
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            Ok(client) => client,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "launch transport unavailable",
                )
                    .into_response()
            }
        };
        let mut request = client.get(target);
        if let Some(token) = session.token.as_deref() {
            request = request.bearer_auth(token);
        }
        if let Some(value) = headers.get(header::ACCEPT) {
            request = request.header(header::ACCEPT, value);
        }
        match request.send().await {
            Ok(upstream) => {
                let status = upstream.status();
                if let Some(reason) = embedding_denial(upstream.headers()) {
                    return (
                        StatusCode::CONFLICT,
                        format!("released artifact refuses embedding: {reason}"),
                    )
                        .into_response();
                }
                let content_type = upstream.headers().get(header::CONTENT_TYPE).cloned();
                match upstream.bytes().await {
                    Ok(bytes) if bytes.len() <= 32 * 1024 * 1024 => {
                        let mut response = Response::builder().status(status);
                        if let Some(content_type) = content_type {
                            response = response.header(header::CONTENT_TYPE, content_type);
                        }
                        response
                            .header("content-security-policy", "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self'")
                            .body(Body::from(bytes))
                            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
                    }
                    Ok(_) => (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "launch response exceeds 32 MiB",
                    )
                        .into_response(),
                    Err(_) => (
                        StatusCode::BAD_GATEWAY,
                        "could not read released web artifact",
                    )
                        .into_response(),
                }
            }
            Err(error) => (
                StatusCode::BAD_GATEWAY,
                format!("released web artifact is unavailable: {error}"),
            )
                .into_response(),
        }
    }

    pub(crate) fn exchange_native(&self, handle: &str) -> Result<serde_json::Value> {
        self.prune_expired();
        let mut sessions = self.sessions.lock().expect("launch sessions");
        let session = sessions
            .get_mut(handle)
            .context("launch handle is absent or expired")?;
        if !matches!(session.kind, SessionKind::Native) {
            bail!("launch handle is not a native client session");
        }
        let token = session
            .token
            .take()
            .context("launch handle was already exchanged")?;
        let subject = session
            .subject
            .take()
            .context("launch handle has no invitation subject")?;
        Ok(serde_json::json!({
            "endpoint": session.target,
            "token": token,
            "subject": subject,
            "candidate_digest": session.candidate_digest,
            "expires_at": session.expires_at,
        }))
    }

    async fn materialize_native(
        &self,
        company: &str,
        artifact: &ArtifactRefRow,
        manifest: &NativeClientRelease,
    ) -> Result<PathBuf> {
        let digest = artifact
            .digest
            .as_deref()
            .context("native artifact has no digest")?;
        validate_sha256(digest)?;
        let key = digest.trim_start_matches("sha256:");
        let release = self.root.join("native").join(key);
        let executable = release.join(&manifest.executable_relative_path);
        if executable.is_file() {
            return Ok(executable);
        }
        let staging = self
            .root
            .join("native")
            .join(format!(".{key}.staging-{}", std::process::id()));
        if staging.exists() {
            std::fs::remove_dir_all(&staging)?;
        }
        std::fs::create_dir_all(&staging)?;
        let archive = staging.join("client.zip");
        let source = format!(
            "{}:{}",
            runtime::container_name(company),
            manifest.archive_path
        );
        let output = tokio::process::Command::new("docker")
            .args(["cp", &source])
            .arg(&archive)
            .output()
            .await
            .context("copy exact native client archive from Company Computer")?;
        if !output.status.success() {
            std::fs::remove_dir_all(&staging).ok();
            bail!(
                "native client archive copy failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let metadata = std::fs::metadata(&archive)?;
        if metadata.len() > MAX_NATIVE_ARCHIVE_BYTES {
            std::fs::remove_dir_all(&staging).ok();
            bail!("native client archive exceeds 2 GiB");
        }
        let observed = sha256_file(&archive)?;
        if observed != digest {
            std::fs::remove_dir_all(&staging).ok();
            bail!("native client digest mismatch: expected {digest}, observed {observed}");
        }
        let unpacked = staging.join("unpacked");
        extract_zip(&archive, &unpacked)?;
        let staged_executable = unpacked.join(&manifest.executable_relative_path);
        if !staged_executable.is_file() {
            std::fs::remove_dir_all(&staging).ok();
            bail!("native client archive lacks its declared executable");
        }
        let mut permissions = std::fs::metadata(&staged_executable)?.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            permissions.set_mode(0o755);
            std::fs::set_permissions(&staged_executable, permissions)?;
        }
        std::fs::create_dir_all(release.parent().expect("native release has parent"))?;
        std::fs::rename(unpacked, &release)?;
        std::fs::remove_dir_all(staging).ok();
        Ok(executable)
    }

    fn prune_expired(&self) {
        let expired = self
            .sessions
            .lock()
            .map(|sessions| {
                sessions
                    .values()
                    .filter(|session| session.expires_at <= Utc::now())
                    .map(|session| session.handle.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for handle in expired {
            self.expire_session(&handle);
        }
    }

    fn expire_session(&self, handle: &str) {
        let session = self
            .sessions
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.remove(handle));
        if let Some(session) = session {
            self.by_resource.lock().ok().map(|mut index| {
                index.remove(&format!("{}\0{}", session.company, session.resource_id))
            });
        }
    }

    fn remove_session(&self, handle: &str) {
        let session = self
            .sessions
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.remove(handle));
        if let Some(session) = session {
            self.by_resource.lock().ok().map(|mut index| {
                index.remove(&format!("{}\0{}", session.company, session.resource_id))
            });
        }
    }
}

async fn probe_embedding_policy(access: &PreparedPublicationAccess) -> Result<Option<String>> {
    let target = reqwest::Url::parse(&access.endpoint.public_endpoint)
        .context("released endpoint is malformed")?;
    if target.scheme() != "https"
        || !target.username().is_empty()
        || target.password().is_some()
        || target.query().is_some()
        || target.fragment().is_some()
    {
        bail!("embedded artifacts require one credential-free HTTPS endpoint");
    }
    if access.local_self_signed_tls
        && !matches!(target.host_str(), Some("127.0.0.1" | "localhost" | "::1"))
    {
        bail!("self-signed launch transport is permitted only for the exact local fixture");
    }
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(access.local_self_signed_tls)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let mut request = client.get(target);
    if let Some(token) = access.token.as_deref() {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .await
        .context("probe released artifact embedding policy")?;
    if response.status().is_redirection() {
        bail!("released artifact redirects outside its exact endpoint");
    }
    if !response.status().is_success() {
        bail!("released artifact root returned {}", response.status());
    }
    Ok(embedding_denial(response.headers()))
}

fn embedding_denial(headers: &HeaderMap) -> Option<String> {
    if headers
        .get("x-frame-options")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case("deny"))
        })
    {
        return Some("X-Frame-Options is DENY".into());
    }
    let csp = headers
        .get("content-security-policy")
        .and_then(|value| value.to_str().ok())?;
    let ancestors = csp.split(';').map(str::trim).find(|directive| {
        directive
            .to_ascii_lowercase()
            .starts_with("frame-ancestors")
    })?;
    let values = ancestors
        .split_ascii_whitespace()
        .skip(1)
        .collect::<Vec<_>>();
    if values
        .iter()
        .any(|value| value.eq_ignore_ascii_case("'none'"))
    {
        return Some("Content-Security-Policy sets frame-ancestors 'none'".into());
    }
    if !values
        .iter()
        .any(|value| *value == "*" || value.eq_ignore_ascii_case("'self'"))
    {
        return Some(
            "Content-Security-Policy does not allow its brokered same-origin frame".into(),
        );
    }
    None
}

fn validate_company_path(value: &str, suffix: &str) -> Result<()> {
    let path = Path::new(value);
    let mut parts = path.components();
    if parts.next() != Some(Component::RootDir)
        || parts.next() != Some(Component::Normal("company".as_ref()))
        || parts.any(|part| !matches!(part, Component::Normal(_)))
        || !value.ends_with(suffix)
    {
        bail!("native archive must be an exact file under /company ending in {suffix}");
    }
    Ok(())
}

fn validate_relative_executable(value: &str) -> Result<()> {
    let path = Path::new(value);
    if path.is_absolute()
        || value.is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        bail!("native executable must be a safe relative path");
    }
    Ok(())
}

fn validate_sha256(value: &str) -> Result<()> {
    let digest = value
        .strip_prefix("sha256:")
        .context("digest must use sha256")?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        bail!("digest must be sha256 plus 64 lowercase hexadecimal characters");
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("sha256:{:x}", digest.finalize()))
}

fn extract_zip(archive: &Path, destination: &Path) -> Result<()> {
    let file = File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file).context("read native client zip")?;
    if zip.len() > MAX_NATIVE_ARCHIVE_ENTRIES {
        bail!("native client archive has too many entries");
    }
    std::fs::create_dir_all(destination)?;
    let mut total = 0u64;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let relative = entry
            .enclosed_name()
            .context("native client archive contains a path traversal")?
            .to_path_buf();
        total = total.saturating_add(entry.size());
        if total > MAX_NATIVE_ARCHIVE_BYTES {
            bail!("expanded native client exceeds 2 GiB");
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("native client archive may not contain symbolic links");
        }
        let output = destination.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&output)?;
            continue;
        }
        std::fs::create_dir_all(output.parent().context("archive file has no parent")?)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&output)?;
        std::io::copy(&mut entry, &mut file)?;
        file.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_manifest_refuses_arbitrary_paths_and_digests() {
        assert!(validate_company_path("/company/build/SwiftArrival.zip", ".zip").is_ok());
        assert!(validate_company_path("/tmp/SwiftArrival.zip", ".zip").is_err());
        assert!(
            validate_relative_executable("SwiftArrival.app/Contents/MacOS/SwiftArrival").is_ok()
        );
        assert!(validate_relative_executable("../escape").is_err());
        assert!(validate_sha256(&format!("sha256:{}", "a".repeat(64))).is_ok());
        assert!(validate_sha256("latest").is_err());
    }

    #[test]
    fn computer_descriptor_never_contains_a_runtime_port() {
        let descriptor = ArtifactLaunchDescriptor::computer("swift_arrival_test", true);
        let value = serde_json::to_string(&descriptor).unwrap();
        assert!(!value.contains("5901"));
        assert!(!value.contains("6080"));
        assert_eq!(descriptor.shape, LaunchShape::CompanyComputer);
    }

    #[test]
    fn embed_denial_is_explicit_and_allow_self_remains_embeddable() {
        let mut deny = HeaderMap::new();
        deny.insert("x-frame-options", "DENY".parse().unwrap());
        assert!(embedding_denial(&deny).unwrap().contains("DENY"));

        let mut csp_deny = HeaderMap::new();
        csp_deny.insert(
            "content-security-policy",
            "default-src 'self'; frame-ancestors https://another.example"
                .parse()
                .unwrap(),
        );
        assert!(embedding_denial(&csp_deny).is_some());

        let mut allow = HeaderMap::new();
        allow.insert(
            "content-security-policy",
            "default-src 'self'; frame-ancestors 'self'"
                .parse()
                .unwrap(),
        );
        assert!(embedding_denial(&allow).is_none());
    }

    #[test]
    fn native_exchange_is_one_time_and_keeps_the_token_out_of_the_handle() {
        let root = std::env::temp_dir().join(format!("restless-launch-{}", Uuid::new_v4()));
        let broker = LaunchBroker::new(&root).unwrap();
        let session = broker
            .insert_session(
                "swift_arrival_test",
                "artifact:release",
                SessionKind::Native,
                PreparedPublicationAccess {
                    endpoint: restlessd::published_service_contract::ProviderEndpoint {
                        profile: ServiceProfile::GodotEnetUdp,
                        public_endpoint: "udp://127.0.0.1:24565".into(),
                        bound_port: 24_565,
                        transport_security: "publication-invitation".into(),
                    },
                    token: Some("private-test-token".into()),
                    subject: Some("owner".into()),
                    candidate_digest: format!("sha256:{}", "a".repeat(64)),
                    expires_at: Utc::now() + chrono::Duration::minutes(5),
                    local_self_signed_tls: false,
                },
            )
            .unwrap();
        assert!(!session.handle.contains("private-test-token"));
        let exchange = broker.exchange_native(&session.handle).unwrap();
        assert_eq!(exchange["token"], "private-test-token");
        assert!(broker.exchange_native(&session.handle).is_err());
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn native_zip_extraction_refuses_path_traversal() {
        let nonce = Uuid::new_v4().simple().to_string();
        let root = std::env::temp_dir().join(format!("restless-zip-{nonce}"));
        std::fs::create_dir_all(&root).unwrap();
        let archive = root.join("bad.zip");
        let file = File::create(&archive).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file(
                format!("../outside-{nonce}"),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"bad").unwrap();
        writer.finish().unwrap();
        assert!(extract_zip(&archive, &root.join("unpacked")).is_err());
        assert!(!root
            .parent()
            .unwrap()
            .join(format!("outside-{nonce}"))
            .exists());
        std::fs::remove_dir_all(root).ok();
    }
}
