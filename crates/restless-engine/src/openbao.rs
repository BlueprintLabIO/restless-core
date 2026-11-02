//! The OpenBao backend for a Cloud plane's vault (restless-cloud ADR 0013).
//!
//! A Cloud plane's settings choose it: `OPENBAO_ADDR` set means every `infisical:` reference
//! resolves against the plane's own OpenBao namespace instead of an Infisical project. Paths map
//! one to one: `infisical:/companies/aris/RESEND_API_KEY` is the KV v2 entry
//! `kv/data/companies/aris/RESEND_API_KEY`, field `value`. Self-hosted Core keeps Infisical.
//!
//! A short vault outage costs writes, not work (ADR §11):
//! - one pooled client, trusting only the vault CA, and one AppRole token reused for 80% of its
//!   lease (a 403 logs in again once);
//! - a memory-only cache of resolved values, served only while the vault is unreachable
//!   (transport failure, timeout, 5xx including 503 sealed), never after a 403 or 404 and never
//!   older than 24 hours;
//! - writes fail loudly while the vault is unreachable.
//!
//! No response body ever reaches an error: a misbehaving backend cannot reflect secret material
//! into a log.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use reqwest::{Method, StatusCode};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use url::Url;
use uuid::Uuid;

pub(crate) const ADDR_ENV: &str = "OPENBAO_ADDR";
const NAMESPACE_ENV: &str = "OPENBAO_NAMESPACE";
const ROLE_ID_ENV: &str = "OPENBAO_ROLE_ID";
const SECRET_ID_ENV: &str = "OPENBAO_SECRET_ID";
const CACERT_ENV: &str = "OPENBAO_CACERT";
const KV_MOUNT: &str = "kv";
/// How long a cached value may stand in for an unreachable vault.
pub(crate) const STALE_LIMIT: Duration = Duration::from_secs(24 * 60 * 60);
const UNAVAILABLE: &str = "The vault is unavailable; try again in a few minutes";
const MAX_INVENTORY_DEPTH: usize = 8;
const MAX_INVENTORY_ENTRIES: usize = 1000;
const MAX_MANIFEST_ENTRIES: usize = 10_000;
const MAX_REPORTED_MISMATCHES: usize = 100;
/// After the vault did not answer, calls fail at once for this long instead of each waiting out
/// its own connect timeout: reads are served from memory meanwhile.
const FAIL_FAST: Duration = Duration::from_secs(5);
/// Fleet's readiness client waits 2 s for the whole observation.
const CUSTODY_BUDGET: Duration = Duration::from_millis(1500);

/// Whether this plane's settings choose OpenBao.
pub(crate) fn configured() -> bool {
    std::env::var(ADDR_ENV).is_ok_and(|value| !value.trim().is_empty())
}

/// The process-wide vault, built once from the plane's settings.
pub(crate) fn vault() -> Result<Arc<Vault>> {
    static VAULT: OnceLock<std::result::Result<Arc<Vault>, String>> = OnceLock::new();
    VAULT
        .get_or_init(|| {
            Settings::from_env()
                .and_then(Vault::new)
                .map(Arc::new)
                .map_err(|error| format!("{error:#}"))
        })
        .clone()
        .map_err(anyhow::Error::msg)
}

#[derive(Debug, Clone)]
pub(crate) struct Settings {
    base_url: Url,
    namespace: String,
    role_id: String,
    secret_id: String,
    /// The only CA the vault client trusts. `None` only in tests against a plain-HTTP mock.
    ca_pem: Option<Vec<u8>>,
}

impl Settings {
    fn from_env() -> Result<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let required = |name: &str| -> Result<String> {
            let value = lookup(name).with_context(|| format!("{name} is not set"))?;
            if value.trim().is_empty() || value.trim() != value {
                bail!("{name} must be one non-empty value");
            }
            Ok(value)
        };
        let base_url =
            Url::parse(&required(ADDR_ENV)?).with_context(|| format!("{ADDR_ENV} is not a URL"))?;
        if base_url.scheme() != "https"
            || base_url.host().is_none()
            || !base_url.username().is_empty()
            || base_url.password().is_some()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
        {
            bail!("{ADDR_ENV} must be a plain https URL");
        }
        let namespace = required(NAMESPACE_ENV)?;
        if namespace.starts_with('/')
            || namespace.ends_with('/')
            || namespace.split('/').any(|part| {
                part.is_empty()
                    || !part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            })
        {
            bail!("{NAMESPACE_ENV} must be a namespace path such as tenants/<plane-id>");
        }
        let ca_path = required(CACERT_ENV)?;
        let ca_pem = std::fs::read(&ca_path)
            .with_context(|| format!("read the vault CA certificate named by {CACERT_ENV}"))?;
        if ca_pem.len() > 64 * 1024 || !ca_pem.starts_with(b"-----BEGIN CERTIFICATE-----") {
            bail!("{CACERT_ENV} must name one PEM CA certificate");
        }
        Ok(Self {
            base_url,
            namespace,
            role_id: required(ROLE_ID_ENV)?,
            secret_id: required(SECRET_ID_ENV)?,
            ca_pem: Some(ca_pem),
        })
    }

    #[cfg(test)]
    pub(crate) fn for_test(base_url: &str) -> Self {
        Self {
            base_url: Url::parse(base_url).unwrap(),
            namespace: "tenants/test-plane".into(),
            role_id: "role".into(),
            secret_id: "secret-id-value".into(),
            ca_pem: None,
        }
    }
}

/// Why a vault call failed, without anything the vault said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureKind {
    /// No answer, a timeout or a 5xx (503 is sealed): a cached value may stand in.
    Unreachable,
    /// The vault answered and refused (401/403, or a login refused).
    Refused,
    /// Any other answer this client does not accept.
    Invalid,
}

#[derive(Debug, Clone)]
pub(crate) struct Failure {
    pub(crate) kind: FailureKind,
    message: String,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Failure {}

impl Failure {
    fn new(kind: FailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn status(operation: &str, status: StatusCode) -> Self {
        let kind = if status.is_server_error() {
            FailureKind::Unreachable
        } else if matches!(status.as_u16(), 401 | 403) {
            FailureKind::Refused
        } else {
            FailureKind::Invalid
        };
        Self::new(
            kind,
            format!("OpenBao {operation} returned HTTP {}", status.as_u16()),
        )
    }

    fn transport(operation: &str, error: &reqwest::Error) -> Self {
        // reqwest's error text names the URL and the cause, never a response body.
        let cause = if error.is_timeout() {
            "timed out"
        } else if error.is_connect() {
            "could not connect"
        } else {
            "failed in transport"
        };
        Self::new(
            FailureKind::Unreachable,
            format!("OpenBao {operation} {cause}"),
        )
    }

    /// The owner-facing error for a write that could not happen.
    fn for_write(self) -> anyhow::Error {
        if self.kind == FailureKind::Unreachable {
            anyhow::Error::new(self).context(UNAVAILABLE)
        } else {
            anyhow::Error::new(self)
        }
    }
}

struct Token {
    value: String,
    refresh_at: Instant,
}

impl Drop for Token {
    fn drop(&mut self) {
        wipe(&mut self.value);
    }
}

/// One resolved value, kept only in memory and wiped when dropped.
struct Cached {
    value: String,
    fetched_at: Instant,
}

impl Drop for Cached {
    fn drop(&mut self) {
        wipe(&mut self.value);
    }
}

fn wipe(value: &mut str) {
    // SAFETY: zero bytes are valid UTF-8, and the string is not used again after this.
    let bytes = unsafe { value.as_bytes_mut() };
    for byte in bytes.iter_mut() {
        // Volatile so the compiler cannot drop the writes to memory about to be freed.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

/// The migration check of ADR 0013 §14 step 5, paths only.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MigrationReport {
    /// `verified`, `mismatch`, `absent` (no manifest: a plane that was never migrated) or
    /// `unknown` (the vault did not answer).
    pub status: &'static str,
    pub entries: usize,
    pub mismatched: Vec<String>,
}

/// How the vault answers right now.
#[derive(Debug, Clone, Serialize)]
pub struct VaultHealth {
    /// `ok`, `degraded` (unreachable) or `refused`.
    pub vault: &'static str,
    pub last_success_at: Option<DateTime<Utc>>,
}

pub(crate) struct Vault {
    settings: Settings,
    client: reqwest::Client,
    token: tokio::sync::Mutex<Option<Token>>,
    cache: Mutex<HashMap<String, Cached>>,
    last_success: Mutex<Option<(Instant, DateTime<Utc>)>>,
    verified: Mutex<Option<(u64, MigrationReport)>>,
    unreachable_until: Mutex<Option<Instant>>,
    fail_fast: Duration,
}

/// The answer to one data call: status and, on success, the decoded body.
struct Answer {
    status: StatusCode,
    body: Option<serde_json::Value>,
}

impl Vault {
    pub(crate) fn new(settings: Settings) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .pool_idle_timeout(Duration::from_secs(90));
        if let Some(pem) = &settings.ca_pem {
            // Only the vault CA: no built-in roots, so no public CA can issue a certificate
            // this client accepts.
            builder = builder
                .use_rustls_tls()
                .tls_built_in_root_certs(false)
                .add_root_certificate(
                    reqwest::Certificate::from_pem(pem)
                        .context("parse the vault CA certificate")?,
                )
                .https_only(true);
        }
        Ok(Self {
            client: builder
                .build()
                .context("build the bounded OpenBao client")?,
            settings,
            token: tokio::sync::Mutex::new(None),
            cache: Mutex::new(HashMap::new()),
            last_success: Mutex::new(None),
            verified: Mutex::new(None),
            unreachable_until: Mutex::new(None),
            fail_fast: FAIL_FAST,
        })
    }

    fn endpoint(&self, segments: &[&str]) -> std::result::Result<Url, Failure> {
        let mut endpoint = self.settings.base_url.clone();
        {
            let mut path = endpoint.path_segments_mut().map_err(|_| {
                Failure::new(FailureKind::Invalid, "OPENBAO_ADDR cannot be an API base")
            })?;
            path.pop_if_empty();
            path.push("v1");
            for segment in segments {
                path.push(segment);
            }
        }
        endpoint.set_query(None);
        endpoint.set_fragment(None);
        Ok(endpoint)
    }

    /// `kv/<kind>/<locator segments>`, where the locator is an already-normalized absolute path.
    fn kv(&self, kind: &str, locator: &str) -> std::result::Result<Url, Failure> {
        let mut segments = vec![KV_MOUNT, kind];
        segments.extend(locator.split('/').filter(|part| !part.is_empty()));
        self.endpoint(&segments)
    }

    fn succeeded(&self) {
        *self.last_success.lock().unwrap() = Some((Instant::now(), Utc::now()));
    }

    /// The last time the vault answered, if within the stale limit.
    pub(crate) fn recently_reachable(&self) -> bool {
        self.last_success
            .lock()
            .unwrap()
            .is_some_and(|(at, _)| at.elapsed() < STALE_LIMIT)
    }

    fn last_success_at(&self) -> Option<DateTime<Utc>> {
        self.last_success.lock().unwrap().map(|(_, at)| at)
    }

    async fn login(&self) -> std::result::Result<Token, Failure> {
        let response = self
            .client
            .post(self.endpoint(&["auth", "approle", "login"])?)
            .header("X-Vault-Namespace", &self.settings.namespace)
            .json(&serde_json::json!({
                "role_id": self.settings.role_id,
                "secret_id": self.settings.secret_id,
            }))
            .send()
            .await
            .map_err(|error| Failure::transport("login", &error))?;
        let status = response.status();
        if !status.is_success() {
            let mut failure = Failure::status("AppRole login", status);
            if matches!(status.as_u16(), 400 | 401 | 403) {
                failure.kind = FailureKind::Refused;
            }
            return Err(failure);
        }
        let body: serde_json::Value = response.json().await.map_err(|_| {
            Failure::new(FailureKind::Invalid, "OpenBao login answer did not decode")
        })?;
        let value = body["auth"]["client_token"]
            .as_str()
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| Failure::new(FailureKind::Invalid, "OpenBao login returned no token"))?
            .to_string();
        let lease = body["auth"]["lease_duration"].as_u64().unwrap_or(0).max(1);
        self.succeeded();
        Ok(Token {
            value,
            refresh_at: Instant::now() + Duration::from_millis(lease * 800),
        })
    }

    /// One authenticated call. A token is reused until 80% of its lease; a 403 logs in once more
    /// and retries once. Only a success decodes the body.
    async fn call(
        &self,
        operation: &str,
        method: Method,
        url: Url,
        body: Option<&serde_json::Value>,
    ) -> std::result::Result<Answer, Failure> {
        if self
            .unreachable_until
            .lock()
            .unwrap()
            .is_some_and(|until| Instant::now() < until)
        {
            return Err(Failure::new(
                FailureKind::Unreachable,
                format!("OpenBao {operation} skipped: the vault did not answer moments ago"),
            ));
        }
        let result = self.call_once(operation, method, url, body).await;
        *self.unreachable_until.lock().unwrap() = match &result {
            Err(failure) if failure.kind == FailureKind::Unreachable => {
                Some(Instant::now() + self.fail_fast)
            }
            _ => None,
        };
        result
    }

    async fn call_once(
        &self,
        operation: &str,
        method: Method,
        url: Url,
        body: Option<&serde_json::Value>,
    ) -> std::result::Result<Answer, Failure> {
        for attempt in 0..2 {
            let token = {
                let mut held = self.token.lock().await;
                if held
                    .as_ref()
                    .is_none_or(|token| Instant::now() >= token.refresh_at)
                {
                    *held = Some(self.login().await?);
                }
                held.as_ref()
                    .map(|token| token.value.clone())
                    .unwrap_or_default()
            };
            let mut request = self
                .client
                .request(method.clone(), url.clone())
                .header("X-Vault-Token", &token)
                .header("X-Vault-Namespace", &self.settings.namespace);
            if let Some(body) = body {
                request = request.json(body);
            }
            let mut token = token;
            let sent = request.send().await;
            wipe(&mut token);
            let response = sent.map_err(|error| Failure::transport(operation, &error))?;
            let status = response.status();
            if status == StatusCode::FORBIDDEN && attempt == 0 {
                *self.token.lock().await = None;
                continue;
            }
            if status.is_server_error() {
                return Err(Failure::status(operation, status));
            }
            self.succeeded();
            let body = if status.is_success() && status != StatusCode::NO_CONTENT {
                Some(response.json().await.map_err(|_| {
                    Failure::new(
                        FailureKind::Invalid,
                        format!("OpenBao {operation} answer did not decode"),
                    )
                })?)
            } else {
                None
            };
            return Ok(Answer { status, body });
        }
        Err(Failure::new(
            FailureKind::Refused,
            format!("OpenBao {operation} returned HTTP 403"),
        ))
    }

    /// Read one entry, at a version when given. `Ok(None)` is absent (or soft-deleted).
    /// Returns the value and its KV version.
    async fn read_entry(
        &self,
        locator: &str,
        version: Option<u64>,
    ) -> std::result::Result<Option<(String, u64)>, Failure> {
        let mut url = self.kv("data", locator)?;
        if let Some(version) = version {
            url.query_pairs_mut()
                .append_pair("version", &version.to_string());
        }
        let answer = self.call("read", Method::GET, url, None).await?;
        match answer.status {
            StatusCode::NOT_FOUND => Ok(None),
            status if status.is_success() => {
                let body = answer.body.unwrap_or_default();
                let value = body["data"]["data"]["value"].as_str().ok_or_else(|| {
                    Failure::new(FailureKind::Invalid, "OpenBao entry has no string value")
                })?;
                if value.is_empty() {
                    return Err(Failure::new(
                        FailureKind::Invalid,
                        "OpenBao returned an empty secret value",
                    ));
                }
                let version = body["data"]["metadata"]["version"].as_u64().unwrap_or(0);
                Ok(Some((value.to_string(), version)))
            }
            status => Err(Failure::status("read", status)),
        }
    }

    /// Resolve one reference for use. Served from memory only while the vault is unreachable.
    pub(crate) async fn get(&self, locator: &str) -> Result<Option<String>> {
        match self.read_entry(locator, None).await {
            Ok(Some((value, _))) => {
                self.cache.lock().unwrap().insert(
                    locator.to_string(),
                    Cached {
                        value: value.clone(),
                        fetched_at: Instant::now(),
                    },
                );
                Ok(Some(value))
            }
            Ok(None) => {
                self.cache.lock().unwrap().remove(locator);
                Ok(None)
            }
            Err(failure) if failure.kind == FailureKind::Unreachable => {
                let cache = self.cache.lock().unwrap();
                match cache.get(locator) {
                    Some(cached) if cached.fetched_at.elapsed() < STALE_LIMIT => {
                        tracing::warn!(
                            error = %failure,
                            "vault degraded: serving a value resolved earlier from memory"
                        );
                        Ok(Some(cached.value.clone()))
                    }
                    _ => Err(failure.into()),
                }
            }
            Err(failure) => {
                // Revoked or refused: the change takes effect at once.
                self.cache.lock().unwrap().remove(locator);
                Err(failure.into())
            }
        }
    }

    /// Write one entry. With `cas = Some(0)` it is created only if absent; `Ok(None)` then means
    /// another value is already there. Returns the new KV version.
    pub(crate) async fn put(
        &self,
        locator: &str,
        value: &str,
        cas: Option<u64>,
    ) -> Result<Option<u64>> {
        let mut body = serde_json::json!({ "data": { "value": value } });
        if let Some(cas) = cas {
            body["options"] = serde_json::json!({ "cas": cas });
        }
        let url = self.kv("data", locator).map_err(Failure::for_write)?;
        let answer = self
            .call("write", Method::POST, url, Some(&body))
            .await
            .map_err(Failure::for_write)?;
        if answer.status == StatusCode::BAD_REQUEST && cas.is_some() {
            return Ok(None);
        }
        if !answer.status.is_success() {
            return Err(Failure::status("write", answer.status).for_write());
        }
        self.cache.lock().unwrap().insert(
            locator.to_string(),
            Cached {
                value: value.to_string(),
                fetched_at: Instant::now(),
            },
        );
        Ok(Some(
            answer
                .body
                .as_ref()
                .and_then(|body| body["data"]["version"].as_u64())
                .unwrap_or(0),
        ))
    }

    /// Soft-delete the latest version (recoverable by an operator).
    pub(crate) async fn delete(&self, locator: &str) -> Result<()> {
        self.cache.lock().unwrap().remove(locator);
        let url = self.kv("data", locator).map_err(Failure::for_write)?;
        let answer = self
            .call("delete", Method::DELETE, url, None)
            .await
            .map_err(Failure::for_write)?;
        if !answer.status.is_success() && answer.status != StatusCode::NOT_FOUND {
            return Err(Failure::status("delete", answer.status).for_write());
        }
        Ok(())
    }

    async fn list(&self, folder: &str) -> std::result::Result<Vec<String>, Failure> {
        let mut url = self.kv("metadata", folder)?;
        url.query_pairs_mut().append_pair("list", "true");
        let answer = self.call("list", Method::GET, url, None).await?;
        match answer.status {
            StatusCode::NOT_FOUND => Ok(Vec::new()),
            status if status.is_success() => Ok(answer.body.unwrap_or_default()["data"]["keys"]
                .as_array()
                .map(|keys| {
                    keys.iter()
                        .filter_map(|key| key.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default()),
            status => Err(Failure::status("list", status)),
        }
    }

    /// Names and update times under one company's folder, never values.
    pub(crate) async fn inventory(&self, base: &str) -> Result<Vec<serde_json::Value>> {
        let mut rows = Vec::new();
        let mut folders = vec![(base.to_string(), 0usize)];
        while let Some((folder, depth)) = folders.pop() {
            for key in self.list(&folder).await? {
                let name = key.trim_end_matches('/');
                if name.is_empty() || matches!(name, "." | "..") || name.contains('/') {
                    bail!("Vault returned an invalid inventory");
                }
                if key.ends_with('/') {
                    if depth + 1 >= MAX_INVENTORY_DEPTH {
                        bail!("Vault inventory is deeper than {MAX_INVENTORY_DEPTH} folders");
                    }
                    folders.push((format!("{folder}/{name}"), depth + 1));
                    continue;
                }
                if rows.len() >= MAX_INVENTORY_ENTRIES {
                    bail!("Vault inventory has more than {MAX_INVENTORY_ENTRIES} secrets");
                }
                let answer = self
                    .call(
                        "metadata",
                        Method::GET,
                        self.kv("metadata", &format!("{folder}/{name}"))?,
                        None,
                    )
                    .await?;
                if !answer.status.is_success() {
                    // Listed a moment ago and gone now, or destroyed: not a current secret.
                    continue;
                }
                let metadata = answer.body.unwrap_or_default();
                let current = metadata["data"]["current_version"].as_u64().unwrap_or(0);
                let latest = &metadata["data"]["versions"][current.to_string()];
                if latest["destroyed"].as_bool() == Some(true)
                    || latest["deletion_time"]
                        .as_str()
                        .is_some_and(|time| !time.is_empty())
                {
                    // Soft-deleted: not a current secret.
                    continue;
                }
                let updated = metadata["data"]["updated_time"].clone();
                if folder != base && !folder.starts_with(&format!("{base}/")) {
                    bail!("Vault returned a secret outside this company");
                }
                rows.push(serde_json::json!({
                    "name": name,
                    "path": folder,
                    "reference": format!("infisical:{folder}/{name}"),
                    "updated_at": updated,
                }));
            }
        }
        rows.sort_by_key(|row| row["reference"].as_str().unwrap_or_default().to_owned());
        Ok(rows)
    }

    /// Log in (reusing a held token) and look the token up.
    pub(crate) async fn health(&self) -> VaultHealth {
        let vault = match self.lookup_self().await {
            Ok(()) => "ok",
            Err(failure) if failure.kind == FailureKind::Unreachable => "degraded",
            Err(_) => "refused",
        };
        VaultHealth {
            vault,
            last_success_at: self.last_success_at(),
        }
    }

    async fn lookup_self(&self) -> std::result::Result<(), Failure> {
        let url = self.endpoint(&["auth", "token", "lookup-self"])?;
        let answer = self.call("token lookup", Method::GET, url, None).await?;
        if answer.status.is_success() {
            Ok(())
        } else {
            Err(Failure::status("token lookup", answer.status))
        }
    }

    pub(crate) async fn check(&self) -> Result<()> {
        self.lookup_self().await.map_err(anyhow::Error::new)
    }

    /// Read (or create once) the plane's custody marker. An unreachable vault keeps the plane
    /// ready for up to 24 hours after it last answered; a refusal or a wrong marker never does.
    pub(crate) async fn probe_custody(&self, plane_id: Uuid) -> Result<()> {
        let locator = format!("/plane-credentials/{plane_id}/readiness-marker");
        let expected = format!("restless-plane-custody-v1:{plane_id}");
        let outcome = tokio::time::timeout(CUSTODY_BUDGET, async {
            match self.read_entry(&locator, None).await? {
                Some((value, _)) if same_digest(&value, &expected) => Ok(()),
                Some(_) => Err(Failure::new(
                    FailureKind::Invalid,
                    "account-plane credential custody marker has unexpected content",
                )),
                None => {
                    let url = self.kv("data", &locator)?;
                    let body = serde_json::json!({
                        "options": { "cas": 0 },
                        "data": { "value": expected },
                    });
                    let answer = self.call("write", Method::POST, url, Some(&body)).await?;
                    // 400 is a concurrent creation; the readback decides.
                    if !answer.status.is_success() && answer.status != StatusCode::BAD_REQUEST {
                        return Err(Failure::status("write", answer.status));
                    }
                    match self.read_entry(&locator, None).await? {
                        Some((value, _)) if same_digest(&value, &expected) => Ok(()),
                        _ => Err(Failure::new(
                            FailureKind::Invalid,
                            "account-plane credential custody marker readback did not match",
                        )),
                    }
                }
            }
        })
        .await
        .unwrap_or_else(|_| {
            Err(Failure::new(
                FailureKind::Unreachable,
                "OpenBao custody check did not answer in time",
            ))
        });
        match outcome {
            Ok(()) => Ok(()),
            Err(failure)
                if failure.kind == FailureKind::Unreachable && self.recently_reachable() =>
            {
                tracing::warn!(error = %failure, "vault degraded: custody kept from the last answer");
                Ok(())
            }
            Err(failure) => {
                Err(anyhow::Error::new(failure)
                    .context("read account-plane credential custody marker"))
            }
        }
    }

    /// Compare the migration manifest's digests with the copies at the versions it names.
    pub(crate) async fn verify_migration(&self, plane_id: Uuid) -> MigrationReport {
        let manifest_path = format!("/plane-credentials/{plane_id}/migration-manifest");
        let unknown = MigrationReport {
            status: "unknown",
            entries: 0,
            mismatched: Vec::new(),
        };
        let (manifest, manifest_version) = match self.read_entry(&manifest_path, None).await {
            Ok(Some(found)) => found,
            Ok(None) => {
                return MigrationReport {
                    status: "absent",
                    entries: 0,
                    mismatched: Vec::new(),
                }
            }
            Err(failure) if failure.kind == FailureKind::Invalid => {
                return MigrationReport {
                    status: "mismatch",
                    entries: 0,
                    mismatched: vec![manifest_path],
                }
            }
            Err(_) => return unknown,
        };
        if let Some((version, report)) = self.verified.lock().unwrap().as_ref() {
            if *version == manifest_version {
                return report.clone();
            }
        }
        let Some(entries) = parse_manifest(&manifest) else {
            return MigrationReport {
                status: "mismatch",
                entries: 0,
                mismatched: vec![manifest_path],
            };
        };
        let mut mismatched = Vec::new();
        for entry in &entries {
            match self.read_entry(&entry.path, Some(entry.kv_version)).await {
                Ok(Some((value, _))) if digest_matches(&value, &entry.sha256) => {}
                Ok(_) => mismatched.push(entry.path.clone()),
                Err(failure) if failure.kind == FailureKind::Unreachable => return unknown,
                Err(_) => mismatched.push(entry.path.clone()),
            }
        }
        let report = MigrationReport {
            status: if mismatched.is_empty() {
                "verified"
            } else {
                "mismatch"
            },
            entries: entries.len(),
            mismatched: mismatched
                .into_iter()
                .take(MAX_REPORTED_MISMATCHES)
                .collect(),
        };
        *self.verified.lock().unwrap() = Some((manifest_version, report.clone()));
        report
    }
}

struct ManifestEntry {
    path: String,
    kv_version: u64,
    sha256: String,
}

fn parse_manifest(text: &str) -> Option<Vec<ManifestEntry>> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    if value["version"].as_u64() != Some(1) {
        return None;
    }
    let entries = value["entries"].as_array()?;
    if entries.len() > MAX_MANIFEST_ENTRIES {
        return None;
    }
    entries
        .iter()
        .map(|entry| {
            let path = entry["path"].as_str()?;
            let normalized = path.starts_with('/')
                && !path.contains("//")
                && !path.ends_with('/')
                && !path.split('/').any(|part| matches!(part, "." | ".."));
            let kv_version = entry["kv_version"]
                .as_u64()
                .filter(|version| *version > 0)?;
            let sha256 = entry["sha256"].as_str()?;
            let hex = sha256.len() == 64
                && sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
            (normalized && hex).then(|| ManifestEntry {
                path: path.to_string(),
                kv_version,
                sha256: sha256.to_string(),
            })
        })
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn same_digest(left: &str, right: &str) -> bool {
    constant_time_equal(
        &Sha256::digest(left.as_bytes()),
        &Sha256::digest(right.as_bytes()),
    )
}

fn digest_matches(value: &str, expected_hex: &str) -> bool {
    constant_time_equal(
        hex(&Sha256::digest(value.as_bytes())).as_bytes(),
        expected_hex.as_bytes(),
    )
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        difference |= usize::from(
            left.get(index).copied().unwrap_or_default()
                ^ right.get(index).copied().unwrap_or_default(),
        );
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::{
        extract::{Path, Query, State},
        http::{HeaderMap, StatusCode as AxumStatus},
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };

    use super::*;

    fn test_plane() -> Uuid {
        Uuid::parse_str("0190aaaa-0000-7000-8000-00000000abcd").unwrap()
    }

    /// A small in-memory KV v2 with an AppRole login, a sealed switch and a revocation switch.
    #[derive(Clone, Default)]
    struct Mock {
        logins: Arc<AtomicUsize>,
        /// path → every version's value (None is soft-deleted)
        entries: Arc<Mutex<HashMap<String, Vec<Option<String>>>>>,
        sealed: Arc<std::sync::atomic::AtomicBool>,
        revoked: Arc<std::sync::atomic::AtomicBool>,
        bodies: Arc<Mutex<Vec<serde_json::Value>>>,
    }

    fn guard(mock: &Mock, headers: &HeaderMap) -> Option<axum::response::Response> {
        if mock.sealed.load(Ordering::SeqCst) {
            return Some(
                (
                    AxumStatus::SERVICE_UNAVAILABLE,
                    "sealed-body-must-not-escape",
                )
                    .into_response(),
            );
        }
        if mock.revoked.load(Ordering::SeqCst)
            || headers.get("x-vault-token").and_then(|v| v.to_str().ok()) != Some("tok")
            || headers
                .get("x-vault-namespace")
                .and_then(|v| v.to_str().ok())
                != Some("tenants/test-plane")
        {
            return Some((AxumStatus::FORBIDDEN, "denied-body").into_response());
        }
        None
    }

    async fn login(
        State(mock): State<Mock>,
        Json(body): Json<serde_json::Value>,
    ) -> impl IntoResponse {
        mock.logins.fetch_add(1, Ordering::SeqCst);
        if mock.sealed.load(Ordering::SeqCst) {
            return (AxumStatus::SERVICE_UNAVAILABLE, Json(serde_json::json!({}))).into_response();
        }
        if mock.revoked.load(Ordering::SeqCst) || body["secret_id"] != "secret-id-value" {
            return (AxumStatus::BAD_REQUEST, Json(serde_json::json!({}))).into_response();
        }
        Json(serde_json::json!({"auth": {"client_token": "tok", "lease_duration": 600}}))
            .into_response()
    }

    async fn data(
        State(mock): State<Mock>,
        Path(path): Path<String>,
        Query(query): Query<HashMap<String, String>>,
        headers: HeaderMap,
        method: Method,
        body: axum::body::Bytes,
    ) -> axum::response::Response {
        if let Some(refusal) = guard(&mock, &headers) {
            return refusal;
        }
        let mut entries = mock.entries.lock().unwrap();
        match method {
            Method::GET => {
                let Some(versions) = entries.get(&path) else {
                    return AxumStatus::NOT_FOUND.into_response();
                };
                let version = query
                    .get("version")
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(versions.len());
                match versions.get(version.wrapping_sub(1)).cloned().flatten() {
                    Some(value) => Json(serde_json::json!({"data": {
                        "data": {"value": value},
                        "metadata": {"version": version},
                    }}))
                    .into_response(),
                    None => AxumStatus::NOT_FOUND.into_response(),
                }
            }
            Method::POST => {
                let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
                mock.bodies.lock().unwrap().push(body.clone());
                let versions = entries.entry(path).or_default();
                if let Some(cas) = body["options"]["cas"].as_u64() {
                    if cas as usize != versions.len() {
                        return (AxumStatus::BAD_REQUEST, "check-and-set failed").into_response();
                    }
                }
                versions.push(Some(body["data"]["value"].as_str().unwrap().to_string()));
                Json(serde_json::json!({"data": {"version": versions.len()}})).into_response()
            }
            Method::DELETE => match entries.get_mut(&path) {
                Some(versions) => {
                    versions.push(None);
                    AxumStatus::NO_CONTENT.into_response()
                }
                None => AxumStatus::NOT_FOUND.into_response(),
            },
            _ => AxumStatus::METHOD_NOT_ALLOWED.into_response(),
        }
    }

    async fn metadata(
        State(mock): State<Mock>,
        Path(path): Path<String>,
        Query(query): Query<HashMap<String, String>>,
        headers: HeaderMap,
    ) -> axum::response::Response {
        if let Some(refusal) = guard(&mock, &headers) {
            return refusal;
        }
        let entries = mock.entries.lock().unwrap();
        if query.get("list").map(String::as_str) == Some("true") {
            let prefix = format!("{}/", path.trim_end_matches('/'));
            let mut keys: Vec<String> = entries
                .keys()
                .filter_map(|key| key.strip_prefix(&prefix))
                .map(|rest| match rest.split_once('/') {
                    Some((folder, _)) => format!("{folder}/"),
                    None => rest.to_string(),
                })
                .collect();
            keys.sort();
            keys.dedup();
            if keys.is_empty() {
                return AxumStatus::NOT_FOUND.into_response();
            }
            return Json(serde_json::json!({"data": {"keys": keys}})).into_response();
        }
        match entries.get(&path) {
            Some(_) => Json(serde_json::json!({"data": {"updated_time": "2026-11-02T00:00:00Z"}}))
                .into_response(),
            None => AxumStatus::NOT_FOUND.into_response(),
        }
    }

    async fn lookup(State(mock): State<Mock>, headers: HeaderMap) -> axum::response::Response {
        if let Some(refusal) = guard(&mock, &headers) {
            return refusal;
        }
        Json(serde_json::json!({"data": {}})).into_response()
    }

    async fn serve(mock: Mock) -> (Vault, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route("/v1/auth/approle/login", post(login))
            .route("/v1/auth/token/lookup-self", get(lookup))
            .route("/v1/kv/data/{*path}", get(data).post(data).delete(data))
            .route("/v1/kv/metadata/{*path}", get(metadata))
            .with_state(mock);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let mut vault = Vault::new(Settings::for_test(&format!("http://{address}"))).unwrap();
        vault.fail_fast = Duration::ZERO;
        (vault, server)
    }

    #[tokio::test]
    async fn reads_writes_creates_and_deletes_with_one_reused_login() {
        let mock = Mock::default();
        let (vault, server) = serve(mock.clone()).await;
        let path = "/companies/aris/RESEND_API_KEY";

        assert_eq!(vault.get(path).await.unwrap(), None);
        assert_eq!(vault.put(path, "re_one", None).await.unwrap(), Some(1));
        assert_eq!(vault.get(path).await.unwrap().as_deref(), Some("re_one"));
        assert_eq!(
            mock.bodies.lock().unwrap()[0],
            serde_json::json!({"data": {"value": "re_one"}})
        );

        // Create-only: occupied is not an error and changes nothing.
        assert_eq!(vault.put(path, "re_two", Some(0)).await.unwrap(), None);
        assert_eq!(vault.get(path).await.unwrap().as_deref(), Some("re_one"));
        assert_eq!(
            vault
                .put("/companies/aris/NEW", "fresh", Some(0))
                .await
                .unwrap(),
            Some(1)
        );

        vault.delete(path).await.unwrap();
        assert_eq!(
            vault.get(path).await.unwrap(),
            None,
            "a soft delete is absent"
        );
        vault.delete("/companies/aris/NEVER").await.unwrap();
        assert_eq!(mock.logins.load(Ordering::SeqCst), 1, "the token is reused");
        server.abort();
    }

    #[tokio::test]
    async fn an_unreachable_vault_serves_memory_but_a_refusal_never_does() {
        let mock = Mock::default();
        let (vault, server) = serve(mock.clone()).await;
        let path = "/companies/aris/RESEND_API_KEY";
        vault.put(path, "re_cached", None).await.unwrap();
        assert_eq!(vault.get(path).await.unwrap().as_deref(), Some("re_cached"));

        mock.sealed.store(true, Ordering::SeqCst);
        assert_eq!(
            vault.get(path).await.unwrap().as_deref(),
            Some("re_cached"),
            "sealed: the value resolved earlier keeps the company working"
        );
        assert!(vault.get("/companies/aris/UNSEEN").await.is_err());
        let write = format!("{:#}", vault.put(path, "re_new", None).await.unwrap_err());
        assert!(write.contains("The vault is unavailable"), "{write}");
        assert!(!write.contains("sealed-body"), "{write}");
        vault
            .probe_custody(test_plane())
            .await
            .expect("degraded, still ready");

        mock.sealed.store(false, Ordering::SeqCst);
        mock.revoked.store(true, Ordering::SeqCst);
        let refused = format!("{:#}", vault.get(path).await.unwrap_err());
        assert!(!refused.contains("denied-body"), "{refused}");
        mock.revoked.store(false, Ordering::SeqCst);
        mock.sealed.store(true, Ordering::SeqCst);
        assert!(
            vault.get(path).await.is_err(),
            "a refusal evicted the value: it is not served again"
        );
        assert!(vault.probe_custody(test_plane()).await.is_ok());
        mock.sealed.store(false, Ordering::SeqCst);
        assert_eq!(vault.health().await.vault, "ok");
        server.abort();
    }

    #[tokio::test]
    async fn without_a_recent_answer_an_unreachable_vault_is_not_ready() {
        let mock = Mock::default();
        mock.sealed.store(true, Ordering::SeqCst);
        let (vault, server) = serve(mock.clone()).await;
        assert!(vault.probe_custody(test_plane()).await.is_err());
        assert_eq!(vault.health().await.vault, "degraded");
        mock.sealed.store(false, Ordering::SeqCst);
        mock.revoked.store(true, Ordering::SeqCst);
        assert_eq!(vault.health().await.vault, "refused");
        server.abort();
    }

    #[tokio::test]
    async fn after_no_answer_calls_fail_fast_and_reads_come_from_memory() {
        let mock = Mock::default();
        let (mut vault, server) = serve(mock.clone()).await;
        vault.fail_fast = Duration::from_secs(60);
        vault.put("/a/B", "v", None).await.unwrap();
        vault.get("/a/B").await.unwrap();
        mock.sealed.store(true, Ordering::SeqCst);
        assert!(vault.get("/a/C").await.is_err());
        mock.sealed.store(false, Ordering::SeqCst);
        let logins = mock.logins.load(Ordering::SeqCst);
        // Within the window nothing reaches the vault, even though it answers again.
        assert_eq!(vault.get("/a/B").await.unwrap().as_deref(), Some("v"));
        assert!(vault.get("/a/C").await.is_err());
        assert_eq!(mock.logins.load(Ordering::SeqCst), logins);
        server.abort();
    }

    #[tokio::test]
    async fn a_403_logs_in_again_once() {
        let mock = Mock::default();
        let (vault, server) = serve(mock.clone()).await;
        vault.put("/a/B", "v", None).await.unwrap();
        // The token the vault knows changes under the client: it is dropped and replaced once.
        *vault.token.lock().await = Some(Token {
            value: "stale".into(),
            refresh_at: Instant::now() + Duration::from_secs(60),
        });
        assert_eq!(vault.get("/a/B").await.unwrap().as_deref(), Some("v"));
        assert_eq!(mock.logins.load(Ordering::SeqCst), 2);
        server.abort();
    }

    #[tokio::test]
    async fn custody_marker_is_created_once_and_checked() {
        let mock = Mock::default();
        let (vault, server) = serve(mock.clone()).await;
        let plane = test_plane();
        vault.probe_custody(plane).await.unwrap();
        vault.probe_custody(plane).await.unwrap();
        let key = format!("plane-credentials/{plane}/readiness-marker");
        assert_eq!(mock.entries.lock().unwrap()[&key].len(), 1);
        mock.entries
            .lock()
            .unwrap()
            .insert(key, vec![Some("wrong".into())]);
        assert!(vault.probe_custody(plane).await.is_err());
        server.abort();
    }

    #[tokio::test]
    async fn inventory_lists_names_never_values_within_one_company() {
        let mock = Mock::default();
        let (vault, server) = serve(mock.clone()).await;
        vault
            .put("/companies/reno/RESEND_API_KEY", "never-expose", None)
            .await
            .unwrap();
        vault
            .put("/companies/reno/nested/deep/KEY", "x", None)
            .await
            .unwrap();
        vault.put("/companies/other/KEY", "y", None).await.unwrap();
        let rows = vault.inventory("/companies/reno").await.unwrap();
        let text = serde_json::to_string(&rows).unwrap();
        assert!(!text.contains("never-expose"));
        let references: Vec<_> = rows
            .iter()
            .map(|row| row["reference"].as_str().unwrap())
            .collect();
        assert_eq!(
            references,
            [
                "infisical:/companies/reno/RESEND_API_KEY",
                "infisical:/companies/reno/nested/deep/KEY"
            ]
        );
        assert_eq!(rows[1]["path"], "/companies/reno/nested/deep");
        assert_eq!(rows[1]["name"], "KEY");
        assert_eq!(rows[0]["updated_at"], "2026-11-02T00:00:00Z");
        assert!(vault.inventory("/companies/none").await.unwrap().is_empty());

        let deep = format!("/companies/deep/{}/KEY", ["d"; 9].join("/"));
        vault.put(&deep, "z", None).await.unwrap();
        assert!(
            vault.inventory("/companies/deep").await.is_err(),
            "depth is bounded"
        );
        server.abort();
    }

    fn sha(value: &str) -> String {
        hex(&Sha256::digest(value.as_bytes()))
    }

    #[tokio::test]
    async fn migration_is_verified_at_the_copied_versions() {
        let mock = Mock::default();
        let (vault, server) = serve(mock.clone()).await;
        let plane = test_plane();
        assert_eq!(vault.verify_migration(plane).await.status, "absent");

        let a = vault
            .put("/companies/reno/A", "alpha", None)
            .await
            .unwrap()
            .unwrap();
        let b = vault
            .put("/companies/reno/B", "beta", None)
            .await
            .unwrap()
            .unwrap();
        let manifest = format!("/plane-credentials/{plane}/migration-manifest");
        let body = serde_json::json!({"version": 1, "entries": [
            {"path": "/companies/reno/A", "kv_version": a, "sha256": sha("alpha")},
            {"path": "/companies/reno/B", "kv_version": b, "sha256": sha("beta")},
        ]});
        vault.put(&manifest, &body.to_string(), None).await.unwrap();
        // The owner changes a value after the cutover: the copy is still what was verified.
        vault
            .put("/companies/reno/A", "alpha-2", None)
            .await
            .unwrap();
        let report = vault.verify_migration(plane).await;
        assert_eq!((report.status, report.entries), ("verified", 2));

        let wrong = serde_json::json!({"version": 1, "entries": [
            {"path": "/companies/reno/A", "kv_version": a, "sha256": sha("not-alpha")},
            {"path": "/companies/reno/MISSING", "kv_version": 1, "sha256": sha("x")},
        ]});
        vault
            .put(&manifest, &wrong.to_string(), None)
            .await
            .unwrap();
        let report = vault.verify_migration(plane).await;
        assert_eq!(
            report.status, "mismatch",
            "a rewritten manifest is checked again"
        );
        assert_eq!(
            report.mismatched,
            ["/companies/reno/A", "/companies/reno/MISSING"]
        );
        assert!(!serde_json::to_string(&report).unwrap().contains("alpha"));

        vault.put(&manifest, "not json", None).await.unwrap();
        assert_eq!(vault.verify_migration(plane).await.status, "mismatch");
        mock.sealed.store(true, Ordering::SeqCst);
        assert_eq!(vault.verify_migration(plane).await.status, "unknown");
        server.abort();
    }

    #[test]
    fn production_settings_require_https_and_only_the_vault_ca() {
        let ca = std::env::temp_dir().join(format!("restless-openbao-ca-{}.crt", test_plane()));
        std::fs::write(&ca, "not a certificate").unwrap();
        let settings = |addr: &'static str, cacert: Option<String>| {
            move |name: &str| match name {
                ADDR_ENV => Some(addr.to_string()),
                NAMESPACE_ENV => Some("tenants/0190aaaa-0000-7000-8000-000000000000".to_string()),
                ROLE_ID_ENV => Some("role".to_string()),
                SECRET_ID_ENV => Some("secret".to_string()),
                CACERT_ENV => cacert.clone(),
                _ => None,
            }
        };
        let error = |lookup| format!("{:#}", Settings::from_lookup(lookup).unwrap_err());
        assert!(error(settings("http://159.69.14.247:8200", None)).contains("https"));
        assert!(error(settings("https://159.69.14.247:8200", None)).contains(CACERT_ENV));
        assert!(error(settings(
            "https://159.69.14.247:8200",
            Some(ca.display().to_string())
        ))
        .contains("PEM"));
        let _ = std::fs::remove_file(ca);
    }
}
