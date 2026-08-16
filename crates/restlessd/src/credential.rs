//! Host-side credential indirection.
//!
//! Company configuration stores a `credential_reference`, never raw secret
//! material (`authority-plane §8.2`). Provider adapters resolve the reference
//! at the point of use on the trusted host. The Runtime receives neither
//! consequential provider credentials nor Infisical machine-identity access.
//!
//! `env:` remains a local bootstrap/migration backend. `infisical:` is the
//! default durable backend from `ARCHITECTURE.md §3.2`: the daemon exchanges a
//! Universal Auth machine identity for a short-lived access token, then reads
//! or writes only the referenced secret through Infisical's v4 API.

use anyhow::{bail, Context as _, Result};
use serde::Deserialize;
use url::Url;

use crate::runtime::CompanyConfig;

const DEFAULT_INFISICAL_API_URL: &str = "https://us.infisical.com";
const DEFAULT_INFISICAL_ENVIRONMENT: &str = "prod";

#[derive(Debug)]
enum CredentialReference<'a> {
    Env(&'a str),
    Infisical(InfisicalLocator<'a>),
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

/// Resolve the credential a capability needs. The returned value is handed
/// directly to the provider client and is never logged, persisted, or placed
/// in an effect receipt.
pub async fn resolve(config: &CompanyConfig, capability: &str) -> Result<String> {
    let reference = config.credentials.get(capability).with_context(|| {
        format!(
            "company {} has no credential_reference for {capability}; add one with `restless credential set` before this capability can reach a real provider",
            config.name
        )
    })?;
    resolve_reference(reference).await
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
    }
}

/// Forward secret material to the referenced backend. Restless remains a
/// conduit, not a second store: only the reference is persisted by the caller.
pub(crate) async fn store_reference(reference: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        bail!("refusing to store an empty secret value");
    }
    match parse_reference(reference)? {
        CredentialReference::Env(locator) => bail!(
            "the env: backend cannot accept writes from Restless; set {locator} in the daemon environment or use an infisical: reference"
        ),
        CredentialReference::Infisical(locator) => {
            let settings = InfisicalSettings::from_env()?;
            infisical_upsert(&settings, &locator, value).await
        }
    }
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
    }
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
        other => bail!(
            "unknown credential scheme {other:?} in {reference:?}; supported schemes are env: and infisical:"
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
    let client = reqwest::Client::new();
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
    let client = reqwest::Client::new();
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
            name: "aris".to_string(),
            mission: String::new(),
            spend_ceiling_usd: 30.0,
            model: "moonshot/kimi-k3".to_string(),
            providers: std::collections::BTreeMap::new(),
            from_address: None,
            credentials,
            approved_parties: Vec::new(),
        }
    }

    #[tokio::test]
    async fn a_missing_credential_names_what_is_missing() {
        let config = config_with(&[("email.send", "env:DEFINITELY_NOT_SET_12345")]);
        let error = format!("{:#}", resolve(&config, "email.send").await.unwrap_err());
        assert!(error.contains("DEFINITELY_NOT_SET_12345"), "{error}");
    }

    #[tokio::test]
    async fn no_reference_is_an_error_not_a_default() {
        let config = config_with(&[]);
        let error = format!("{:#}", resolve(&config, "email.send").await.unwrap_err());
        assert!(error.contains("credential_reference"), "{error}");
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
}
