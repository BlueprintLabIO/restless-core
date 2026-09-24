//! Host-side credential indirection.
//!
//! Company configuration stores a `credential_reference`, never raw secret
//! material (`authority-plane §8.2`). The model gateway or generic governed
//! process resolves the named reference at the point of use on the trusted
//! host. The Runtime receives neither consequential external-tool credentials
//! nor Infisical machine-identity access.
//!
//! `env:` remains a local bootstrap/migration backend. `infisical:` is the
//! default durable backend from `ARCHITECTURE.md §3.2`: the daemon exchanges a
//! Universal Auth machine identity for a short-lived access token, then reads
//! or writes only the referenced secret through Infisical's v4 API.

use anyhow::{bail, Context as _, Result};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use std::io::Read as _;
use url::Url;
use uuid::Uuid;

use crate::runtime::CompanyConfig;

const DEFAULT_INFISICAL_API_URL: &str = "https://us.infisical.com";
const DEFAULT_INFISICAL_ENVIRONMENT: &str = "prod";

#[derive(Debug)]
enum CredentialReference<'a> {
    Env(&'a str),
    Infisical(InfisicalLocator<'a>),
    /// A subscription OAuth credential held by OMP's host-side Restless
    /// broker. It is a reference to broker custody, never a plaintext value
    /// that this generic resolver may return.
    OmpOauth(&'a str),
}

#[derive(Debug)]
struct InfisicalLocator<'a> {
    path: &'a str,
    name: &'a str,
}

#[derive(Debug)]
struct InfisicalSettings {
    base_url: Url,
    project_id: String,
    environment: String,
    client_id: String,
    client_secret: String,
    organization_slug: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct SecretResponse {
    secret: SecretValue,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecretValue {
    secret_value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbeStatus {
    Present,
    Absent,
    Invalid,
}

impl ProbeStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Present => "present",
            Self::Absent => "absent",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Debug)]
pub(crate) struct Probe {
    pub(crate) status: ProbeStatus,
    pub(crate) detail: Option<String>,
}

/// Resolve one named binding for one governed child process.
pub async fn resolve(config: &CompanyConfig, binding: &str) -> Result<String> {
    if finance_binding(binding) {
        bail!(
            "finance credential bindings may only be resolved by the host-side Authority adapter"
        );
    }
    let reference = config.credentials.get(binding).with_context(|| {
        format!(
            "company {} has no credential reference for binding {binding}; add it with `restless credential set`",
            config.name
        )
    })?;
    resolve_reference(reference).await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FinanceCredential {
    Read,
    Submit,
    Webhook,
}

/// Resolve one finance secret only from its canonical company/provider path.
/// This function is not reachable from the generic effect runner.
pub(crate) async fn resolve_finance(
    config: &CompanyConfig,
    kind: FinanceCredential,
) -> Result<String> {
    let (binding, suffix) = match kind {
        FinanceCredential::Read => ("finance.airwallex.read", "/read/api-key"),
        FinanceCredential::Submit => ("finance.airwallex.submit", "/submit/api-key"),
        FinanceCredential::Webhook => ("finance.airwallex.webhook", "/webhook/signing-secret"),
    };
    let reference = config.credentials.get(binding).with_context(|| {
        format!(
            "company {} has no {binding} credential reference",
            config.name
        )
    })?;
    let expected = format!(
        "infisical:/companies/{}/finance/airwallex{suffix}",
        config.name
    );
    if reference != &expected {
        bail!("{binding} must use the canonical Infisical finance path {expected}");
    }
    resolve_reference(reference).await
}

fn finance_binding(value: &str) -> bool {
    value == "finance"
        || value.starts_with("finance.")
        || value.starts_with("finance/")
        || value.contains("/finance/")
}

/// Resolve one `scheme:locator` reference.
///
/// Infisical locators are absolute secret paths whose final segment is the
/// secret name, for example `infisical:/companies/aris/RESEND_API_KEY`.
pub(crate) async fn resolve_reference(reference: &str) -> Result<String> {
    match parse_reference(reference)? {
        CredentialReference::Env(locator) => read_env(locator),
        CredentialReference::Infisical(locator) => {
            let settings = InfisicalSettings::from_env()?;
            infisical_get(&settings, &locator)
                .await?
                .with_context(|| format!("Infisical secret {reference:?} was not found"))
        }
        CredentialReference::OmpOauth(provider) => bail!(
            "omp-oauth:{provider} is broker-held model access and cannot be resolved as a raw credential"
        ),
    }
}

/// Forward secret material to the referenced backend. Restless remains a
/// conduit, not a second store: only the reference is persisted by the caller.
pub(crate) async fn store_reference(reference: &str, value: &str) -> Result<()> {
    let value = normalize_secret_value(value)?;
    match parse_reference(reference)? {
        CredentialReference::Env(locator) => bail!(
            "the env: backend cannot accept writes from Restless; set {locator} in the daemon environment or use an infisical: reference"
        ),
        CredentialReference::Infisical(locator) => {
            let settings = InfisicalSettings::from_env()?;
            infisical_ensure_folder(&settings, locator.path).await?;
            infisical_upsert(&settings, &locator, &value).await
        }
        CredentialReference::OmpOauth(provider) => bail!(
            "omp-oauth:{provider} is created through the owner OAuth handover, not by storing a raw value"
        ),
    }
}

/// Put the generated per-cell native Documents database capability into the
/// exact secret location consumed by Cloud's company-cell provider. The raw
/// URL is read only inside the account plane, forwarded to Infisical, and
/// checked by digest after readback. Bootstrap must not become ready if this
/// custody handoff is absent or ambiguous.
pub(crate) async fn publish_native_documents_store(
    plane_id: Uuid,
    cell_id: Uuid,
    credential_path: &std::path::Path,
) -> Result<()> {
    if plane_id.is_nil() || cell_id.is_nil() {
        bail!("native Documents credential needs non-nil plane and cell identities");
    }
    let link = std::fs::symlink_metadata(credential_path).with_context(|| {
        format!(
            "inspect native Documents credential at {}",
            credential_path.display()
        )
    })?;
    if link.file_type().is_symlink()
        || !link.file_type().is_file()
        || !(32..=2048).contains(&link.len())
    {
        bail!("native Documents credential must be one bounded regular non-symlink file");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        if link.permissions().mode() & 0o077 != 0 || link.nlink() != 1 {
            bail!("native Documents credential must be private and unlinked");
        }
    }
    let file = std::fs::File::open(credential_path).with_context(|| {
        format!(
            "open native Documents credential at {}",
            credential_path.display()
        )
    })?;
    let opened = file
        .metadata()
        .context("read opened native Documents credential metadata")?;
    if !opened.is_file() || opened.len() != link.len() {
        bail!("native Documents credential changed before it was opened");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if opened.dev() != link.dev() || opened.ino() != link.ino() {
            bail!("native Documents credential changed identity before it was opened");
        }
    }
    let value =
        std::io::read_to_string(file.take(2049)).context("read native Documents credential")?;
    if value.trim() != value || value.lines().count() != 1 {
        bail!("native Documents credential must be one normalized URL");
    }
    let parsed = Url::parse(&value).context("parse native Documents credential URL")?;
    if !matches!(parsed.scheme(), "postgres" | "postgresql")
        || parsed.username().is_empty()
        || parsed.password().is_none()
        || parsed.path().len() <= 1
        || parsed.fragment().is_some()
    {
        bail!("native Documents credential URL is invalid");
    }

    let reference = format!("/plane-credentials/{plane_id}/cells/{cell_id}/native-documents-store");
    let locator = parse_infisical_locator(&reference)?;
    let settings = InfisicalSettings::from_env()?;
    infisical_publish_verified(&settings, &locator, &value).await
}

/// Prove that this account plane can reach and use only its own durable
/// credential-custody namespace. The marker contains no secret material. It
/// is created once when the namespace is empty and read on every readiness
/// observation, so an expired machine identity or unavailable Infisical
/// backend makes the plane non-ready instead of relying on startup intent.
pub(crate) async fn probe_plane_custody(plane_id: Uuid) -> Result<()> {
    if plane_id.is_nil() {
        bail!("credential custody probe needs a non-nil plane identity");
    }
    let reference = format!("/plane-credentials/{plane_id}/readiness-marker");
    let locator = parse_infisical_locator(&reference)?;
    let settings = InfisicalSettings::from_env()?;
    let expected = format!("restless-plane-custody-v1:{plane_id}");
    match infisical_get(&settings, &locator)
        .await
        .context("read account-plane credential custody marker")?
    {
        Some(value) if Sha256::digest(value.as_bytes()) == Sha256::digest(expected.as_bytes()) => {
            Ok(())
        }
        Some(_) => bail!("account-plane credential custody marker has unexpected content"),
        None => {
            infisical_ensure_folder(&settings, locator.path)
                .await
                .context("prepare account-plane credential custody namespace")?;
            infisical_upsert(&settings, &locator, &expected)
                .await
                .context("create account-plane credential custody marker")?;
            let readback = infisical_get(&settings, &locator)
                .await
                .context("verify account-plane credential custody marker")?
                .context("account-plane credential custody marker remained absent")?;
            if Sha256::digest(readback.as_bytes()) != Sha256::digest(expected.as_bytes()) {
                bail!("account-plane credential custody marker readback did not match");
            }
            Ok(())
        }
    }
}

async fn infisical_publish_verified(
    settings: &InfisicalSettings,
    locator: &InfisicalLocator<'_>,
    value: &str,
) -> Result<()> {
    infisical_ensure_folder(settings, locator.path)
        .await
        .context("prepare native Documents credential custody")?;
    infisical_upsert(settings, locator, value)
        .await
        .context("publish native Documents credential")?;
    let readback = infisical_get(settings, locator)
        .await
        .context("verify native Documents credential custody")?
        .context("native Documents credential was absent after publication")?;
    if Sha256::digest(value.as_bytes()) != Sha256::digest(readback.as_bytes()) {
        bail!("native Documents credential readback did not match its published digest");
    }
    Ok(())
}

/// Move a bootstrap-only daemon environment credential into durable Infisical
/// custody. The value is resolved and forwarded entirely inside the trusted
/// daemon; callers receive only success or a sanitized error. Deliberately do
/// not make this a generic secret-copy primitive: only the documented `env:`
/// migration source and the durable `infisical:` destination are permitted.
pub(crate) async fn promote_env_to_infisical(
    source_reference: &str,
    destination_reference: &str,
) -> Result<()> {
    let source = match parse_reference(source_reference)? {
        CredentialReference::Env(locator) => locator,
        _ => bail!("credential promotion source must use the bootstrap env: backend"),
    };
    let destination = match parse_reference(destination_reference)? {
        CredentialReference::Infisical(locator) => locator,
        _ => bail!("credential promotion destination must use the durable infisical: backend"),
    };
    let value = read_env(source)?;
    let settings = InfisicalSettings::from_env()?;
    // Infisical requires a folder to exist before a secret can be written at a
    // nested path. Folder creation happens only on this explicit owner
    // promotion path, never implicitly while a company resolves a secret.
    infisical_ensure_folder(&settings, destination.path).await?;
    infisical_upsert(&settings, &destination, &value).await
}

/// Files and clipboard pipes commonly add one or more line endings. Those are
/// transport delimiters, not part of an API key. Other control characters are
/// rejected rather than silently mutated.
fn normalize_secret_value(value: &str) -> Result<String> {
    let normalized = value.trim_end_matches(['\r', '\n']);
    if normalized.is_empty() {
        bail!("refusing to store an empty secret value");
    }
    if normalized.chars().any(char::is_control) {
        bail!("secret value contains a control character other than a trailing line ending");
    }
    Ok(normalized.to_string())
}

/// Probe presence without leaking the value. `absent` is reserved for a
/// well-formed reference whose target does not exist; malformed references,
/// authentication failures, and backend outages remain distinct `invalid`
/// results instead of being collapsed into absence.
pub(crate) async fn probe_reference(reference: &str) -> Probe {
    let parsed = match parse_reference(reference) {
        Ok(parsed) => parsed,
        Err(error) => {
            return Probe {
                status: ProbeStatus::Invalid,
                detail: Some(format!("{error:#}")),
            };
        }
    };
    match parsed {
        CredentialReference::Env(locator) => match std::env::var(locator) {
            Ok(value) if !value.trim().is_empty() => Probe {
                status: ProbeStatus::Present,
                detail: None,
            },
            Ok(_) => Probe {
                status: ProbeStatus::Absent,
                detail: Some(format!("{locator} is set but empty")),
            },
            Err(_) => Probe {
                status: ProbeStatus::Absent,
                detail: Some(format!("{locator} is not set in the daemon environment")),
            },
        },
        CredentialReference::Infisical(locator) => {
            let settings = match InfisicalSettings::from_env() {
                Ok(settings) => settings,
                Err(error) => {
                    return Probe {
                        status: ProbeStatus::Invalid,
                        detail: Some(format!("{error:#}")),
                    };
                }
            };
            match infisical_get(&settings, &locator).await {
                Ok(Some(_)) => Probe {
                    status: ProbeStatus::Present,
                    detail: None,
                },
                Ok(None) => Probe {
                    status: ProbeStatus::Absent,
                    detail: Some(format!("Infisical secret {reference:?} was not found")),
                },
                Err(error) => Probe {
                    status: ProbeStatus::Invalid,
                    detail: Some(format!("{error:#}")),
                },
            }
        }
        CredentialReference::OmpOauth(provider) => {
            match crate::model_gateway::oauth_is_loaded(provider) {
                Ok(true) => Probe {
                    status: ProbeStatus::Present,
                    detail: None,
                },
                Ok(false) => Probe {
                    status: ProbeStatus::Absent,
                    detail: Some(format!(
                        "host OMP broker has no active OAuth credential for {provider}"
                    )),
                },
                Err(error) => Probe {
                    status: ProbeStatus::Invalid,
                    detail: Some(format!("{error:#}")),
                },
            }
        }
    }
}

/// Return the provider named by a host-broker OAuth reference. Other valid
/// credential backends return `None`; malformed references remain errors.
pub(crate) fn omp_oauth_provider(reference: &str) -> Result<Option<&str>> {
    Ok(match parse_reference(reference)? {
        CredentialReference::OmpOauth(provider) => Some(provider),
        CredentialReference::Env(_) | CredentialReference::Infisical(_) => None,
    })
}

fn parse_reference(reference: &str) -> Result<CredentialReference<'_>> {
    let (scheme, locator) = reference.split_once(':').with_context(|| {
        format!(
            "credential reference {reference:?} must be `scheme:locator`, e.g. infisical:/companies/aris/RESEND_API_KEY"
        )
    })?;
    match scheme {
        "env" => {
            if locator.is_empty() {
                bail!("env: credential reference needs a variable name");
            }
            Ok(CredentialReference::Env(locator))
        }
        "infisical" => Ok(CredentialReference::Infisical(parse_infisical_locator(
            locator,
        )?)),
        "omp-oauth" => {
            if locator.is_empty()
                || !locator.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                })
            {
                bail!("omp-oauth: locator must be a provider identifier such as anthropic");
            }
            Ok(CredentialReference::OmpOauth(locator))
        }
        other => bail!(
            "unknown credential scheme {other:?} in {reference:?}; supported schemes are env:, infisical:, and omp-oauth:"
        ),
    }
}

fn parse_infisical_locator(locator: &str) -> Result<InfisicalLocator<'_>> {
    if !locator.starts_with('/') {
        bail!("infisical: locator must be an absolute secret path");
    }
    if locator.contains("//")
        || locator
            .split('/')
            .any(|segment| matches!(segment, "." | ".."))
    {
        bail!("infisical: locator must be a normalized secret path");
    }
    let (path, name) = locator
        .rsplit_once('/')
        .context("infisical: locator must end with a secret name")?;
    if name.is_empty() {
        bail!("infisical: locator must end with a secret name");
    }
    Ok(InfisicalLocator {
        path: if path.is_empty() { "/" } else { path },
        name,
    })
}

fn read_env(locator: &str) -> Result<String> {
    let value = std::env::var(locator)
        .with_context(|| format!("{locator} is not set in the daemon environment"))?;
    if value.trim().is_empty() {
        bail!("{locator} is set but empty");
    }
    Ok(value)
}

/// Authenticate and check the configured project, never infer connectivity from env vars.
pub(crate) async fn infisical_health() -> Probe {
    let check = async {
        let settings = InfisicalSettings::from_env()?;
        let client = infisical_client()?;
        let token = infisical_login(&client, &settings).await?;
        let url = infisical_endpoint(
            &settings.base_url,
            &["api", "v1", "projects", &settings.project_id],
        )?;
        let response = client.get(url).bearer_auth(token).send().await?;
        if !response.status().is_success() {
            bail!(
                "Infisical project check returned HTTP {}",
                response.status().as_u16()
            );
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;
    match check {
        Ok(()) => Probe {
            status: ProbeStatus::Present,
            detail: None,
        },
        Err(error) => Probe {
            status: ProbeStatus::Invalid,
            detail: Some(format!("{error:#}")),
        },
    }
}

pub(crate) fn infisical_configured() -> bool {
    InfisicalSettings::from_env().is_ok()
}

impl InfisicalSettings {
    fn from_env() -> Result<Self> {
        let base_url = std::env::var("INFISICAL_API_URL")
            .unwrap_or_else(|_| DEFAULT_INFISICAL_API_URL.to_string());
        let base_url = Url::parse(&base_url).context("INFISICAL_API_URL is not a valid URL")?;
        if !matches!(base_url.scheme(), "https" | "http") {
            bail!("INFISICAL_API_URL must use http or https");
        }
        Ok(Self {
            base_url,
            project_id: required_env("INFISICAL_PROJECT_ID")?,
            environment: std::env::var("INFISICAL_ENVIRONMENT")
                .unwrap_or_else(|_| DEFAULT_INFISICAL_ENVIRONMENT.to_string()),
            client_id: required_env("INFISICAL_UNIVERSAL_AUTH_CLIENT_ID")?,
            client_secret: required_env("INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET")?,
            organization_slug: std::env::var("INFISICAL_ORGANIZATION_SLUG")
                .ok()
                .filter(|value| !value.trim().is_empty()),
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} is not set"))?;
    if value.trim().is_empty() {
        bail!("{name} is set but empty");
    }
    Ok(value)
}

fn infisical_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("build bounded Infisical client")
}

async fn infisical_login(client: &reqwest::Client, settings: &InfisicalSettings) -> Result<String> {
    let endpoint = infisical_endpoint(
        &settings.base_url,
        &["api", "v1", "auth", "universal-auth", "login"],
    )?;
    let mut body = serde_json::json!({
        "clientId": settings.client_id,
        "clientSecret": settings.client_secret,
    });
    if let Some(slug) = &settings.organization_slug {
        body["organizationSlug"] = serde_json::Value::String(slug.clone());
    }
    let response = client
        .post(endpoint)
        .json(&body)
        .send()
        .await
        .context("authenticate Infisical machine identity")?;
    let status = response.status();
    if !status.is_success() {
        bail!(
            "Infisical Universal Auth login returned HTTP {}",
            status.as_u16()
        );
    }
    let login: LoginResponse = response
        .json()
        .await
        .context("decode Infisical Universal Auth response")?;
    if login.access_token.trim().is_empty() {
        bail!("Infisical Universal Auth returned an empty access token");
    }
    Ok(login.access_token)
}

async fn infisical_get(
    settings: &InfisicalSettings,
    locator: &InfisicalLocator<'_>,
) -> Result<Option<String>> {
    let client = infisical_client()?;
    let access_token = infisical_login(&client, settings).await?;
    let endpoint = infisical_endpoint(&settings.base_url, &["api", "v4", "secrets", locator.name])?;
    let response = client
        .get(endpoint)
        .bearer_auth(access_token)
        .query(&[
            ("projectId", settings.project_id.as_str()),
            ("environment", settings.environment.as_str()),
            ("secretPath", locator.path),
            ("type", "shared"),
            ("viewSecretValue", "true"),
            ("expandSecretReferences", "true"),
        ])
        .send()
        .await
        .context("retrieve secret from Infisical")?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        // Never include the provider body. A badly behaved backend must not be
        // able to reflect secret material into a Restless error or log.
        bail!(
            "Infisical secret retrieval returned HTTP {}",
            status.as_u16()
        );
    }
    let value: SecretResponse = response
        .json()
        .await
        .context("decode Infisical secret response")?;
    if value.secret.secret_value.is_empty() {
        bail!("Infisical returned an empty secret value");
    }
    Ok(Some(value.secret.secret_value))
}

async fn infisical_upsert(
    settings: &InfisicalSettings,
    locator: &InfisicalLocator<'_>,
    value: &str,
) -> Result<()> {
    let client = infisical_client()?;
    let access_token = infisical_login(&client, settings).await?;
    let endpoint = infisical_endpoint(&settings.base_url, &["api", "v4", "secrets", "batch"])?;
    let response = client
        .patch(endpoint)
        .bearer_auth(access_token)
        .json(&serde_json::json!({
            "projectId": settings.project_id,
            "environment": settings.environment,
            "secretPath": locator.path,
            "mode": "upsert",
            "secrets": [{
                "secretKey": locator.name,
                "secretValue": value,
            }],
        }))
        .send()
        .await
        .context("store secret in Infisical")?;
    let status = response.status();
    if !status.is_success() {
        bail!("Infisical secret upsert returned HTTP {}", status.as_u16());
    }
    Ok(())
}

/// Ensure the destination directory exists for an explicit credential
/// promotion. Creating a directory that already exists is harmless; any
/// other backend error remains visible to the owner without a response body
/// that might contain secret material.
async fn infisical_ensure_folder(settings: &InfisicalSettings, path: &str) -> Result<()> {
    if path == "/" {
        return Ok(());
    }
    let client = infisical_client()?;
    let access_token = infisical_login(&client, settings).await?;
    let endpoint = infisical_endpoint(&settings.base_url, &["api", "v2", "folders"])?;
    let mut parent = "/".to_string();
    for name in path
        .trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
    {
        let response = client
            .post(endpoint.clone())
            .bearer_auth(&access_token)
            .json(&serde_json::json!({
                "projectId": settings.project_id,
                "environment": settings.environment,
                "name": name,
                "path": parent,
            }))
            .send()
            .await
            .context("create Infisical secret folder")?;
        let status = response.status();
        // The Infisical folder API returns 400 for an already-existing folder
        // on this host. The subsequent secret upsert is still authoritative:
        // if this was any other bad request, it will fail without persisting a
        // company reference.
        if !status.is_success()
            && status != reqwest::StatusCode::CONFLICT
            && status != reqwest::StatusCode::BAD_REQUEST
        {
            bail!(
                "Infisical folder creation returned HTTP {}",
                status.as_u16()
            );
        }
        parent = if parent == "/" {
            format!("/{name}")
        } else {
            format!("{parent}/{name}")
        };
    }
    Ok(())
}

fn infisical_endpoint(base: &Url, segments: &[&str]) -> Result<Url> {
    let mut endpoint = base.clone();
    {
        let mut path = endpoint
            .path_segments_mut()
            .map_err(|_| anyhow::anyhow!("INFISICAL_API_URL cannot be used as an API base"))?;
        path.pop_if_empty();
        for segment in segments {
            path.push(segment);
        }
    }
    endpoint.set_query(None);
    endpoint.set_fragment(None);
    Ok(endpoint)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::{
        extract::Path,
        http::StatusCode,
        routing::{get, patch, post},
        Json, Router,
    };

    use super::*;

    fn config_with(entries: &[(&str, &str)]) -> CompanyConfig {
        let mut credentials = std::collections::BTreeMap::new();
        for (capability, reference) in entries {
            credentials.insert((*capability).to_string(), (*reference).to_string());
        }
        CompanyConfig {
            agent_intelligence: Default::default(),
            native_harnesses: Default::default(),
            display_name: None,
            name: "aris".to_string(),
            internal_network: false,
            mission: String::new(),
            spend_ceiling_usd: crate::runtime::SpendCeiling::from_micro_usd(30_000_000),
            monthly_runtime_cap_hours: None,
            auto_sleep_after_minutes: None,
            outcome_standard: Default::default(),
            model: "moonshot/kimi-k3".to_string(),
            coordination_harness: crate::runtime::AgentHarness::RestlessManaged,
            worker_harness: crate::runtime::AgentHarness::RestlessManaged,
            reasoning_effort: crate::acp::DEFAULT_REASONING_EFFORT.to_string(),
            model_failover: Vec::new(),
            credentials,
            approved_parties: Vec::new(),
        }
    }

    #[tokio::test]
    async fn a_missing_credential_names_what_is_missing() {
        let config = config_with(&[("resend.production", "env:DEFINITELY_NOT_SET_12345")]);
        let error = format!(
            "{:#}",
            resolve(&config, "resend.production").await.unwrap_err()
        );
        assert!(error.contains("DEFINITELY_NOT_SET_12345"), "{error}");
    }

    #[tokio::test]
    async fn no_reference_is_an_error_not_a_default() {
        let config = config_with(&[]);
        let error = format!(
            "{:#}",
            resolve(&config, "resend.production").await.unwrap_err()
        );
        assert!(error.contains("binding resend.production"), "{error}");
    }

    #[tokio::test]
    async fn generic_resolution_cannot_cross_the_finance_boundary() {
        let config = config_with(&[(
            "finance.airwallex.submit",
            "env:FINANCE_SECRET_MUST_NOT_ENTER_RUNTIME",
        )]);
        let error = format!(
            "{:#}",
            resolve(&config, "finance.airwallex.submit")
                .await
                .unwrap_err()
        );
        assert!(error.contains("host-side Authority adapter"), "{error}");
        assert!(
            !error.contains("FINANCE_SECRET_MUST_NOT_ENTER_RUNTIME"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn finance_resolution_requires_the_exact_company_provider_path() {
        let config = config_with(&[(
            "finance.airwallex.submit",
            "infisical:/companies/other/finance/airwallex/submit/api-key",
        )]);
        let error = format!(
            "{:#}",
            resolve_finance(&config, FinanceCredential::Submit)
                .await
                .unwrap_err()
        );
        assert!(
            error.contains("canonical Infisical finance path"),
            "{error}"
        );
    }

    #[test]
    fn references_are_bounded_and_normalized() {
        let parsed = parse_reference("infisical:/companies/aris/RESEND_API_KEY").unwrap();
        let CredentialReference::Infisical(locator) = parsed else {
            panic!("expected Infisical reference")
        };
        assert_eq!(locator.path, "/companies/aris");
        assert_eq!(locator.name, "RESEND_API_KEY");
        assert!(parse_reference("infisical:relative/KEY").is_err());
        assert!(parse_reference("infisical:/companies/../KEY").is_err());
        assert!(parse_reference("no-scheme-at-all").is_err());
    }

    #[tokio::test]
    async fn env_scheme_reads_the_daemon_environment() {
        // SAFETY: this variable is test-local and no other test names it.
        unsafe { std::env::set_var("RESTLESS_TEST_CRED", "sk-test-value") };
        assert_eq!(
            resolve_reference("env:RESTLESS_TEST_CRED").await.unwrap(),
            "sk-test-value"
        );
        unsafe { std::env::remove_var("RESTLESS_TEST_CRED") };
    }

    #[tokio::test]
    async fn promotion_is_limited_to_bootstrap_env_and_infisical() {
        let source_error = format!(
            "{:#}",
            promote_env_to_infisical("omp-oauth:zai", "infisical:/providers/zai/ZAI_API_KEY")
                .await
                .unwrap_err()
        );
        assert!(
            source_error.contains("source must use the bootstrap env:"),
            "{source_error}"
        );

        let destination_error = format!(
            "{:#}",
            promote_env_to_infisical("env:RESTLESS_TEST_CRED", "env:ZAI_API_KEY")
                .await
                .unwrap_err()
        );
        assert!(
            destination_error.contains("destination must use the durable infisical:"),
            "{destination_error}"
        );
    }

    #[tokio::test]
    async fn infisical_machine_identity_reads_and_upserts_without_a_cli() {
        let writes = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
        let app = Router::new()
            .route(
                "/api/v1/auth/universal-auth/login",
                post(|| async { Json(serde_json::json!({ "accessToken": "short-lived" })) }),
            )
            .route(
                "/api/v4/secrets/{name}",
                get(|Path(name): Path<String>| async move {
                    assert_eq!(name, "RESEND_API_KEY");
                    Json(serde_json::json!({
                        "secret": { "secretValue": "provider-secret" }
                    }))
                }),
            )
            .route(
                "/api/v4/secrets/batch",
                patch({
                    let writes = Arc::clone(&writes);
                    move |Json(body): Json<serde_json::Value>| {
                        let writes = Arc::clone(&writes);
                        async move {
                            writes.lock().unwrap().push(body);
                            Json(serde_json::json!({ "secrets": [] }))
                        }
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let settings = InfisicalSettings {
            base_url: Url::parse(&format!("http://{address}")).unwrap(),
            project_id: "project".into(),
            environment: "prod".into(),
            client_id: "client".into(),
            client_secret: "bootstrap-secret".into(),
            organization_slug: None,
        };
        let locator = parse_infisical_locator("/companies/aris/RESEND_API_KEY").unwrap();

        assert_eq!(
            infisical_get(&settings, &locator).await.unwrap().as_deref(),
            Some("provider-secret")
        );
        infisical_upsert(&settings, &locator, "new-provider-secret")
            .await
            .unwrap();
        let writes = writes.lock().unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0]["mode"], "upsert");
        assert_eq!(writes[0]["secretPath"], "/companies/aris");
        assert_eq!(writes[0]["secrets"][0]["secretKey"], "RESEND_API_KEY");
        assert_eq!(
            writes[0]["secrets"][0]["secretValue"],
            "new-provider-secret"
        );
        server.abort();
    }

    #[tokio::test]
    async fn native_documents_publication_creates_exact_custody_and_verifies_readback() {
        let folders = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
        let stored = Arc::new(Mutex::new(String::new()));
        let app = Router::new()
            .route(
                "/api/v1/auth/universal-auth/login",
                post(|| async { Json(serde_json::json!({ "accessToken": "short-lived" })) }),
            )
            .route(
                "/api/v2/folders",
                post({
                    let folders = Arc::clone(&folders);
                    move |Json(body): Json<serde_json::Value>| {
                        let folders = Arc::clone(&folders);
                        async move {
                            folders.lock().unwrap().push(body);
                            Json(serde_json::json!({ "folder": {} }))
                        }
                    }
                }),
            )
            .route(
                "/api/v4/secrets/batch",
                patch({
                    let stored = Arc::clone(&stored);
                    move |Json(body): Json<serde_json::Value>| {
                        let stored = Arc::clone(&stored);
                        async move {
                            *stored.lock().unwrap() = body["secrets"][0]["secretValue"]
                                .as_str()
                                .unwrap()
                                .to_string();
                            Json(serde_json::json!({ "secrets": [] }))
                        }
                    }
                }),
            )
            .route(
                "/api/v4/secrets/{name}",
                get({
                    let stored = Arc::clone(&stored);
                    move |Path(name): Path<String>| {
                        let stored = Arc::clone(&stored);
                        async move {
                            assert_eq!(name, "native-documents-store");
                            Json(serde_json::json!({
                                "secret": { "secretValue": stored.lock().unwrap().clone() }
                            }))
                        }
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let settings = InfisicalSettings {
            base_url: Url::parse(&format!("http://{address}")).unwrap(),
            project_id: "project".into(),
            environment: "prod".into(),
            client_id: "client".into(),
            client_secret: "bootstrap-secret".into(),
            organization_slug: None,
        };
        let plane_id = Uuid::parse_str("33333333-3333-7333-8333-333333333333").unwrap();
        let cell_id = Uuid::parse_str("44444444-4444-7444-8444-444444444444").unwrap();
        let path = format!("/plane-credentials/{plane_id}/cells/{cell_id}/native-documents-store");
        let locator = parse_infisical_locator(&path).unwrap();
        let secret = "postgresql://docs:secret@postgres.internal/restless_cell_company";

        infisical_publish_verified(&settings, &locator, secret)
            .await
            .unwrap();
        assert_eq!(stored.lock().unwrap().as_str(), secret);
        let folders = folders.lock().unwrap();
        assert_eq!(folders.len(), 4);
        assert_eq!(folders[0]["name"], "plane-credentials");
        assert_eq!(folders[0]["path"], "/");
        assert_eq!(folders[1]["name"], plane_id.to_string());
        assert_eq!(folders[1]["path"], "/plane-credentials");
        assert_eq!(folders[2]["name"], "cells");
        assert_eq!(folders[2]["path"], format!("/plane-credentials/{plane_id}"));
        assert_eq!(folders[3]["name"], cell_id.to_string());
        assert_eq!(
            folders[3]["path"],
            format!("/plane-credentials/{plane_id}/cells")
        );
        server.abort();
    }

    #[tokio::test]
    async fn a_backend_error_cannot_reflect_secret_material_into_logs() {
        let app = Router::new()
            .route(
                "/api/v1/auth/universal-auth/login",
                post(|| async { Json(serde_json::json!({ "accessToken": "short-lived" })) }),
            )
            .route(
                "/api/v4/secrets/{name}",
                get(|| async {
                    (
                        StatusCode::BAD_GATEWAY,
                        "reflected-provider-secret-must-not-escape",
                    )
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let settings = InfisicalSettings {
            base_url: Url::parse(&format!("http://{address}")).unwrap(),
            project_id: "project".into(),
            environment: "prod".into(),
            client_id: "client".into(),
            client_secret: "bootstrap-secret".into(),
            organization_slug: None,
        };
        let locator = parse_infisical_locator("/companies/aris/RESEND_API_KEY").unwrap();
        let error = format!(
            "{:#}",
            infisical_get(&settings, &locator).await.unwrap_err()
        );
        assert!(error.contains("502"), "{error}");
        assert!(!error.contains("reflected-provider-secret"), "{error}");
        server.abort();
    }

    #[test]
    fn piped_line_endings_are_not_stored_as_part_of_a_secret() {
        assert_eq!(
            normalize_secret_value("re_example\n").unwrap(),
            "re_example"
        );
        assert_eq!(
            normalize_secret_value("re_example\r\n\r\n").unwrap(),
            "re_example"
        );
        assert!(normalize_secret_value("\r\n").is_err());
        assert!(normalize_secret_value("re_bad\tvalue").is_err());
    }
}

/// Metadata-only inventory, strictly bounded to this company's vault directory.
pub(crate) async fn company_vault_inventory(company: &str) -> Result<Vec<serde_json::Value>> {
    crate::runtime::validate_company_name(company)?;
    let settings = InfisicalSettings::from_env()?;
    let client = infisical_client()?;
    let token = infisical_login(&client, &settings).await?;
    let path = format!("/companies/{company}");
    let response = client
        .get(infisical_endpoint(
            &settings.base_url,
            &["api", "v4", "secrets"],
        )?)
        .bearer_auth(token)
        .query(&[
            ("projectId", settings.project_id.as_str()),
            ("environment", settings.environment.as_str()),
            ("secretPath", path.as_str()),
            ("recursive", "true"),
            ("viewSecretValue", "false"),
            ("expandSecretReferences", "false"),
        ])
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    if !response.status().is_success() {
        bail!(
            "Vault inventory returned HTTP {}",
            response.status().as_u16()
        );
    }
    let body: serde_json::Value = response.json().await?;
    vault_metadata(&path, &body)
}

fn vault_metadata(base: &str, body: &serde_json::Value) -> Result<Vec<serde_json::Value>> {
    let mut rows = Vec::new();
    for secret in body["secrets"]
        .as_array()
        .context("Vault returned an invalid inventory")?
    {
        let path = secret["secretPath"]
            .as_str()
            .context("Vault secret has no directory")?;
        if path != base && !path.starts_with(&format!("{base}/")) {
            bail!("Vault returned a secret outside this company");
        }
        let name = secret["secretKey"]
            .as_str()
            .context("Vault secret has no name")?;
        rows.push(serde_json::json!({"name":name,"path":path,"reference":format!("infisical:{path}/{name}"),"updated_at":secret["updatedAt"]}));
    }
    rows.sort_by_key(|row| row["reference"].as_str().unwrap_or_default().to_owned());
    Ok(rows)
}

#[cfg(test)]
mod vault_inventory_tests {
    #[test]
    fn inventory_omits_values_and_rejects_other_company_paths() {
        let body = serde_json::json!({"secrets":[{"secretKey":"KEY","secretPath":"/companies/one_test/nested","secretValue":"never-expose-this","secretComment":"also-sensitive"}]});
        let rows = super::vault_metadata("/companies/one_test", &body).unwrap();
        let text = serde_json::to_string(&rows).unwrap();
        assert!(!text.contains("never-expose-this"));
        assert!(!text.contains("also-sensitive"));
        assert_eq!(
            rows[0]["reference"],
            "infisical:/companies/one_test/nested/KEY"
        );
        assert!(super::vault_metadata("/companies/one", &body).is_err());
    }
}
