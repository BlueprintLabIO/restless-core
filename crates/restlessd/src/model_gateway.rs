//! Host-side model credential isolation through OMP's imported auth broker
//! and auth gateway.
//!
//! Restless does not implement another model proxy here. It supervises the
//! open-source proxy shipped by the ACP runtime we already use, places only
//! credentials for providers named by configured companies into its host-side
//! vault, and gives company processes the narrower gateway bearer. Provider
//! keys (and Infisical machine-identity credentials) never cross into the
//! Company Runtime.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use tokio::process::{Child, Command};

use crate::runtime::CompanyConfig;

const BROKER_PROFILE: &str = "restless-model-broker";
const GATEWAY_PROFILE: &str = "restless-model-gateway";
const BROKER_URL: &str = "http://127.0.0.1:7789";
const BROKER_BIND: &str = "127.0.0.1:7789";
const GATEWAY_HOST_URL: &str = "http://127.0.0.1:7790";
const GATEWAY_RUNTIME_URL: &str = "http://host.docker.internal:7790";
const GATEWAY_BIND: &str = "0.0.0.0:7790";
const GATEWAY_TOKEN_ENV: &str = "RESTLESS_MODEL_GATEWAY_TOKEN";

static CLIENT: OnceLock<ClientConfig> = OnceLock::new();

#[derive(Clone)]
pub struct ClientConfig {
    gateway_token: String,
    providers: BTreeSet<String>,
}

impl ClientConfig {
    pub fn auth_for(&self, model: &str) -> Result<AgentGatewayAuth> {
        let (provider, _) = split_model(model)?;
        if !self.providers.contains(provider) {
            bail!(
                "model provider {provider} was not loaded into the host gateway at daemon boot; configure its credential and restart restlessd"
            );
        }
        Ok(AgentGatewayAuth {
            provider: provider.to_string(),
            token_env: GATEWAY_TOKEN_ENV.to_string(),
            token: self.gateway_token.clone(),
            runtime_url: GATEWAY_RUNTIME_URL.to_string(),
        })
    }
}

pub struct AgentGatewayAuth {
    pub provider: String,
    pub token_env: String,
    pub token: String,
    pub runtime_url: String,
}

/// Child handles are deliberately ordinary supervised processes. Dropping the
/// daemon drops these handles and requests process termination; their durable
/// credential vault remains host-side in OMP's Restless-only profile.
pub struct Processes {
    broker: Child,
    gateway: Child,
}

impl Drop for Processes {
    fn drop(&mut self) {
        let _ = self.gateway.start_kill();
        let _ = self.broker.start_kill();
    }
}

/// Start the imported broker/gateway pair and install its narrow client
/// configuration for ACP and world-model processes.
pub async fn start(configs: &[CompanyConfig]) -> Result<Processes> {
    let provider_keys = provider_keys(configs).await?;
    if provider_keys.is_empty() {
        bail!("no configured company model provider is available for the model gateway");
    }

    let omp = std::env::var("RESTLESS_OMP_BIN").unwrap_or_else(|_| "omp".to_string());
    let broker_token = token(&omp, BROKER_PROFILE, "auth-broker").await?;
    let mut broker = Command::new(&omp)
        .env("OMP_PROFILE", BROKER_PROFILE)
        .args(["auth-broker", "serve", "--bind", BROKER_BIND])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .context("start OMP model credential broker")?;
    wait_for_broker(&mut broker, &broker_token).await?;

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("build model credential sync client")?;
    prune_unconfigured_credentials(
        &http,
        &broker_token,
        provider_keys.keys().map(String::as_str).collect(),
    )
    .await?;
    for (provider, key) in &provider_keys {
        let response = http
            .post(format!("{BROKER_URL}/v1/credential"))
            .bearer_auth(&broker_token)
            .json(&serde_json::json!({
                "provider": provider,
                "credential": { "type": "api_key", "key": key }
            }))
            .send()
            .await
            .with_context(|| format!("sync {provider} credential to host model broker"))?;
        if !response.status().is_success() {
            // Never include a provider response body here: a hostile or buggy
            // backend can reflect request material, including the secret.
            bail!(
                "host model broker refused the {provider} credential with HTTP {}",
                response.status()
            );
        }
    }
    prune_superseded_credentials(&http, &broker_token, &provider_keys).await?;

    let gateway_token = token(&omp, GATEWAY_PROFILE, "auth-gateway").await?;
    let mut gateway = Command::new(&omp)
        .env("OMP_PROFILE", GATEWAY_PROFILE)
        .env("OMP_AUTH_BROKER_URL", BROKER_URL)
        .env("OMP_AUTH_BROKER_TOKEN", &broker_token)
        .args(["auth-gateway", "serve", "--bind", GATEWAY_BIND])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .context("start OMP model auth gateway")?;
    let providers = provider_keys.keys().cloned().collect::<BTreeSet<_>>();
    wait_for_gateway(&mut gateway, &gateway_token, &providers).await?;

    CLIENT
        .set(ClientConfig {
            gateway_token,
            providers,
        })
        .map_err(|_| anyhow::anyhow!("model gateway client was already installed"))?;
    Ok(Processes { broker, gateway })
}

#[derive(Deserialize)]
struct BrokerSnapshot {
    credentials: Vec<BrokerCredential>,
}

#[derive(Deserialize)]
struct BrokerCredential {
    id: i64,
    provider: String,
    credential: serde_json::Value,
}

impl BrokerCredential {
    fn api_key(&self) -> Option<&str> {
        (self.credential.get("type")?.as_str()? == "api_key")
            .then(|| self.credential.get("key")?.as_str())?
    }
}

async fn broker_snapshot(http: &reqwest::Client, broker_token: &str) -> Result<BrokerSnapshot> {
    let response = http
        .get(format!("{BROKER_URL}/v1/snapshot"))
        .bearer_auth(broker_token)
        .send()
        .await
        .context("read host model broker credential snapshot")?;
    if !response.status().is_success() {
        bail!(
            "host model broker snapshot failed with HTTP {}",
            response.status()
        );
    }
    response
        .json::<BrokerSnapshot>()
        .await
        .context("parse redacted host model broker snapshot")
}

async fn disable_credential(
    http: &reqwest::Client,
    broker_token: &str,
    credential: &BrokerCredential,
    cause: &str,
) -> Result<()> {
    let response = http
        .post(format!(
            "{BROKER_URL}/v1/credential/{}/disable",
            credential.id
        ))
        .bearer_auth(broker_token)
        .json(&serde_json::json!({ "cause": cause }))
        .send()
        .await
        .with_context(|| format!("disable stale {} model credential", credential.provider))?;
    if !response.status().is_success() {
        bail!(
            "host model broker refused stale {} credential removal with HTTP {}",
            credential.provider,
            response.status()
        );
    }
    Ok(())
}

/// The OMP profile survives daemon restarts, so changing a company from one
/// provider to another must not leave the old provider routable as a silent
/// fallback. Disable every active row not named by the current company set
/// before the gateway snapshots its catalogue.
async fn prune_unconfigured_credentials(
    http: &reqwest::Client,
    broker_token: &str,
    configured: BTreeSet<&str>,
) -> Result<()> {
    let snapshot = broker_snapshot(http, broker_token).await?;
    for credential in snapshot
        .credentials
        .into_iter()
        .filter(|credential| !configured.contains(credential.provider.as_str()))
    {
        disable_credential(
            http,
            broker_token,
            &credential,
            "provider is not configured by any Restless company at daemon boot",
        )
        .await?;
    }
    Ok(())
}

/// API-key rows have no broker identity key, so OMP intentionally treats a
/// changed key as a second account. Restless V0 has one credential per provider,
/// however: after uploading the current Infisical value, disable every other
/// active row for that provider. This both applies rotation and prevents daemon
/// restarts from accumulating equally privileged fallback keys.
async fn prune_superseded_credentials(
    http: &reqwest::Client,
    broker_token: &str,
    provider_keys: &BTreeMap<String, String>,
) -> Result<()> {
    let snapshot = broker_snapshot(http, broker_token).await?;
    for (provider, expected_key) in provider_keys {
        let (keep_id, superseded_ids) = canonical_credential(&snapshot, provider, expected_key)?;
        for credential in snapshot
            .credentials
            .iter()
            .filter(|credential| superseded_ids.contains(&credential.id))
        {
            disable_credential(
                http,
                broker_token,
                credential,
                "superseded by the current Restless credential reference",
            )
            .await?;
        }

        let verified = broker_snapshot(http, broker_token).await?;
        let active = verified
            .credentials
            .iter()
            .filter(|credential| credential.provider == *provider)
            .collect::<Vec<_>>();
        if active.len() != 1
            || active[0].id != keep_id
            || active[0].api_key() != Some(expected_key.as_str())
        {
            bail!("host model broker did not converge {provider} to one current credential");
        }
    }
    Ok(())
}

fn canonical_credential(
    snapshot: &BrokerSnapshot,
    provider: &str,
    expected_key: &str,
) -> Result<(i64, Vec<i64>)> {
    let provider_rows = snapshot
        .credentials
        .iter()
        .filter(|credential| credential.provider == provider)
        .collect::<Vec<_>>();
    let matching = provider_rows
        .iter()
        .filter(|credential| credential.api_key() == Some(expected_key))
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        bail!(
            "host model broker has {} current {provider} credentials after sync; expected exactly one",
            matching.len()
        );
    }
    let keep_id = matching[0].id;
    let superseded = provider_rows
        .into_iter()
        .filter_map(|credential| (credential.id != keep_id).then_some(credential.id))
        .collect();
    Ok((keep_id, superseded))
}

pub fn client() -> Result<&'static ClientConfig> {
    CLIENT
        .get()
        .context("host model gateway is not installed; restlessd did not finish booting")
}

pub fn models_config(provider: &str, runtime_url: &str, token_env: &str) -> Result<String> {
    if !provider
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        bail!("invalid model provider identifier {provider:?}");
    }
    if runtime_url != GATEWAY_RUNTIME_URL || token_env != GATEWAY_TOKEN_ENV {
        bail!("refusing an unrecognised model gateway route");
    }
    Ok(format!(
        "# Managed by Restless. Contains a gateway route, never a provider credential.\n\
providers:\n  {provider}:\n    baseUrl: {runtime_url}\n    apiKey: {token_env}\n    transport: pi-native\n"
    ))
}

async fn provider_keys(configs: &[CompanyConfig]) -> Result<BTreeMap<String, String>> {
    let mut keys = BTreeMap::<String, String>::new();
    for config in configs {
        let (provider, _) = split_model(&config.model)?;
        let key = match config.credentials.get("model.inference") {
            Some(reference) => crate::credential::resolve_reference(reference)
                .await
                .with_context(|| format!("resolve model credential for {}", config.name))?,
            None => {
                let env = provider_key_env(provider)?;
                std::env::var(env).with_context(|| {
                    format!(
                        "{env} must be set for configured model {} (or set credentials.model.inference)",
                        config.model
                    )
                })?
            }
        };
        if let Some(existing) = keys.get(provider) {
            if existing != &key {
                bail!(
                    "V0 model gateway refuses different {provider} credentials across companies; separate provider custody before multi-account use"
                );
            }
        } else {
            keys.insert(provider.to_string(), key);
        }
    }
    Ok(keys)
}

fn split_model(model: &str) -> Result<(&str, &str)> {
    let (provider, id) = model.split_once('/').with_context(|| {
        format!("model {model} must be provider-qualified, e.g. moonshot/kimi-k3")
    })?;
    if provider.is_empty() || id.is_empty() {
        bail!("model {model} must contain a provider and model id");
    }
    Ok((provider, id))
}

fn provider_key_env(provider: &str) -> Result<&'static str> {
    match provider {
        "zai" => Ok("ZAI_API_KEY"),
        "anthropic" => Ok("ANTHROPIC_API_KEY"),
        "openai" => Ok("OPENAI_API_KEY"),
        "moonshot" => Ok("MOONSHOT_API_KEY"),
        "openrouter" => Ok("OPENROUTER_API_KEY"),
        other => bail!("no credential mapping for model provider {other}"),
    }
}

async fn token(omp: &str, profile: &str, command: &str) -> Result<String> {
    let output = Command::new(omp)
        .env("OMP_PROFILE", profile)
        .args([command, "token"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await
        .with_context(|| format!("create OMP {command} bearer"))?;
    if !output.status.success() {
        bail!("OMP {command} token command failed");
    }
    let value = String::from_utf8(output.stdout)
        .context("OMP bearer was not UTF-8")?
        .trim()
        .to_string();
    if value.len() < 32 || value.bytes().any(|byte| byte.is_ascii_whitespace()) {
        bail!("OMP {command} returned an invalid bearer");
    }
    Ok(value)
}

async fn wait_for_broker(child: &mut Child, token: &str) -> Result<()> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()?;
    for _ in 0..100 {
        if let Some(status) = child.try_wait().context("inspect OMP broker")? {
            bail!("OMP model credential broker exited during boot ({status})");
        }
        if http
            .get(format!("{BROKER_URL}/v1/healthz"))
            .bearer_auth(token)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!("OMP model credential broker did not become ready")
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelRow>,
}

#[derive(Deserialize)]
struct ModelRow {
    id: String,
}

async fn wait_for_gateway(
    child: &mut Child,
    token: &str,
    providers: &BTreeSet<String>,
) -> Result<()> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    for _ in 0..150 {
        if let Some(status) = child.try_wait().context("inspect OMP gateway")? {
            bail!("OMP model auth gateway exited during boot ({status})");
        }
        if let Ok(response) = http
            .get(format!("{GATEWAY_HOST_URL}/v1/models"))
            .bearer_auth(token)
            .send()
            .await
        {
            if response.status().is_success() {
                if let Ok(list) = response.json::<ModelList>().await {
                    let ready = providers.iter().all(|provider| {
                        let prefix = format!("{provider}/");
                        list.data.iter().any(|model| model.id.starts_with(&prefix))
                    });
                    if ready {
                        return Ok(());
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!("OMP model auth gateway did not expose every configured provider")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_model_config_contains_only_the_narrow_gateway_route() {
        let config = models_config("moonshot", GATEWAY_RUNTIME_URL, GATEWAY_TOKEN_ENV).unwrap();
        assert!(config.contains("\n  moonshot:\n    baseUrl:"));
        assert!(config.contains("transport: pi-native"));
        assert!(config.contains("apiKey: RESTLESS_MODEL_GATEWAY_TOKEN"));
        assert!(!config.contains("MOONSHOT_API_KEY"));
        assert!(!config.contains("api.kimi.com"));
    }

    #[test]
    fn provider_and_route_are_not_open_ended_injection_points() {
        assert!(
            models_config("moonshot\nheaders", GATEWAY_RUNTIME_URL, GATEWAY_TOKEN_ENV).is_err()
        );
        assert!(models_config("moonshot", "https://example.invalid", GATEWAY_TOKEN_ENV).is_err());
    }

    #[test]
    fn credential_rotation_keeps_only_the_referenced_provider_key() {
        let snapshot: BrokerSnapshot = serde_json::from_value(serde_json::json!({
            "credentials": [
                {"id": 1, "provider": "moonshot", "credential": {"type": "api_key", "key": "old"}},
                {"id": 2, "provider": "moonshot", "credential": {"type": "api_key", "key": "current"}},
                {"id": 3, "provider": "openai", "credential": {"type": "api_key", "key": "unrelated"}}
            ]
        }))
        .unwrap();
        assert_eq!(
            canonical_credential(&snapshot, "moonshot", "current").unwrap(),
            (2, vec![1])
        );
        assert!(canonical_credential(&snapshot, "moonshot", "missing").is_err());
    }
}
