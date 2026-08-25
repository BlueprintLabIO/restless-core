//! Host-side model credential isolation through OMP's imported auth broker
//! and auth gateway.
//!
//! Restless does not implement another model proxy here. It supervises the
//! open-source proxy shipped by the ACP runtime we already use, places only
//! credentials for providers named by configured companies into its host-side
//! vault, and gives company processes a short-lived signed relay capability.
//! Provider keys, OMP's root bearer, and Infisical machine-identity
//! credentials never cross into the Company Runtime.

use std::collections::{BTreeMap, BTreeSet};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::OnceLock;
use std::task::{Context as TaskContext, Poll};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::header::{ACCEPT, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, Response, StatusCode};
use axum::routing::post;
use axum::Router;
use futures_util::Stream;
use serde::Deserialize;
use tokio::process::{Child, Command};

use crate::runtime::CompanyConfig;

const BROKER_PROFILE: &str = "restless-model-broker";
const GATEWAY_PROFILE: &str = "restless-model-gateway";
const BROKER_URL: &str = "http://127.0.0.1:7789";
const BROKER_BIND: &str = "127.0.0.1:7789";
/// OMP itself keeps the root provider bearer on this loopback-only listener.
const OMP_GATEWAY_HOST_URL: &str = "http://127.0.0.1:7792";
const OMP_GATEWAY_BIND: &str = "127.0.0.1:7792";
/// The Runtime-facing relay owns the established container route. It never
/// accepts OMP's root bearer.
const RELAY_RUNTIME_URL: &str = "http://host.docker.internal:7790";
const RELAY_BIND: &str = "0.0.0.0:7790";
const MODEL_CAPABILITY_ENV: &str = "RESTLESS_MODEL_CAPABILITY";

static CLIENT: OnceLock<ClientConfig> = OnceLock::new();

#[derive(Clone)]
pub struct ClientConfig {
    providers: BTreeMap<String, ModelBilling>,
}

impl ClientConfig {
    pub fn auth_for(
        &self,
        model: &str,
        capabilities: &crate::capability::CapabilityIssuer,
        company: &str,
        actor: &str,
        session: &str,
    ) -> Result<AgentGatewayAuth> {
        let (provider, _) = split_model(model)?;
        let Some(billing) = self.providers.get(provider).copied() else {
            bail!(
                "model provider {provider} was not loaded into the host gateway at daemon boot; configure its credential and restart restlessd"
            );
        };
        Ok(AgentGatewayAuth {
            provider: provider.to_string(),
            token_env: MODEL_CAPABILITY_ENV.to_string(),
            token: capabilities.issue_model_session(
                company,
                actor,
                session,
                provider,
                model,
                billing.as_str(),
            )?,
            runtime_url: RELAY_RUNTIME_URL.to_string(),
            billing,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelBilling {
    MeteredApi,
    Subscription,
}

impl ModelBilling {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MeteredApi => "metered_api",
            Self::Subscription => "subscription",
        }
    }
}

pub struct AgentGatewayAuth {
    pub provider: String,
    pub token_env: String,
    pub token: String,
    pub runtime_url: String,
    pub billing: ModelBilling,
}

/// Apply the installation-wide cooldown facts to one company's ordered model
/// policy. This is shared by Exec and Staff so a dead provider is not retried
/// once per actor and wake.
pub async fn available_candidates(
    config: &CompanyConfig,
    preferred: Option<&str>,
    authority: &crate::authority::AuthorityStore,
) -> Result<Vec<String>> {
    let ordered = ordered_candidates(config, preferred)?;
    let cooldowns = authority.active_model_cooldowns(&config.name).await?;
    filter_cooling_candidates(ordered, &cooldowns)
}

/// An explicit non-Exec actor preference is an exact next-wake contract, not
/// an invitation to inherit the company's fallback chain. Exec may cross that
/// chain for portfolio continuity; a lead or worker must instead block
/// visibly so role/model experiments and specialist assignments cannot change
/// capability silently.
pub async fn available_actor_candidates(
    config: &CompanyConfig,
    preferred: Option<&str>,
    authority: &crate::authority::AuthorityStore,
) -> Result<Vec<String>> {
    let ordered = actor_candidates(config, preferred)?;
    let cooldowns = authority.active_model_cooldowns(&config.name).await?;
    filter_cooling_candidates(ordered, &cooldowns)
}

fn actor_candidates(config: &CompanyConfig, preferred: Option<&str>) -> Result<Vec<String>> {
    match preferred.map(str::trim).filter(|model| !model.is_empty()) {
        Some(model) => Ok(vec![model.to_string()]),
        None => ordered_candidates(config, None),
    }
}

fn ordered_candidates(config: &CompanyConfig, preferred: Option<&str>) -> Result<Vec<String>> {
    let mut ordered = Vec::new();
    if let Some(model) = preferred.map(str::trim).filter(|model| !model.is_empty()) {
        ordered.push(model.to_string());
    }
    for model in config.model_candidates()? {
        if !ordered.iter().any(|candidate| candidate == model) {
            ordered.push(model.to_string());
        }
    }
    Ok(ordered)
}

fn filter_cooling_candidates(
    mut ordered: Vec<String>,
    cooldowns: &[crate::authority::ModelCooldown],
) -> Result<Vec<String>> {
    ordered.retain(|model| !cooldowns.iter().any(|cooldown| &cooldown.model == model));
    if ordered.is_empty() {
        let next = cooldowns
            .first()
            .map(|cooldown| {
                format!(
                    "{} until {} ({})",
                    cooldown.model,
                    cooldown.retry_at.to_rfc3339(),
                    cooldown.kind
                )
            })
            .unwrap_or_else(|| "no configured candidates".into());
        bail!("all model candidates are cooling down; next: {next}");
    }
    Ok(ordered)
}

pub async fn record_cooldown(
    authority: &crate::authority::AuthorityStore,
    company: &str,
    model: &str,
    kind: crate::health::BlockKind,
    reason: &str,
) -> Result<()> {
    let duration = match kind {
        crate::health::BlockKind::Credential | crate::health::BlockKind::Model => {
            chrono::Duration::hours(24)
        }
        crate::health::BlockKind::Quota => chrono::Duration::hours(1),
        crate::health::BlockKind::NoOp => chrono::Duration::minutes(15),
        crate::health::BlockKind::Transport => chrono::Duration::minutes(2),
        _ => return Ok(()),
    };
    let retry_at = chrono::Utc::now() + duration;
    authority
        .set_model_cooldown(
            company,
            model,
            kind.as_str(),
            &reason.chars().take(500).collect::<String>(),
            retry_at,
        )
        .await?;
    authority
        .emit(
            company,
            "model_cooldown",
            None,
            serde_json::json!({
                "model": model, "kind": kind.as_str(), "retry_at": retry_at,
            }),
        )
        .await?;
    Ok(())
}

/// Child handles are deliberately ordinary supervised processes. Dropping the
/// daemon drops these handles and requests process termination; their durable
/// credential vault remains host-side in OMP's Restless-only profile.
pub struct Processes {
    broker: Child,
    gateway: Child,
    relay: tokio::task::JoinHandle<()>,
}

impl Drop for Processes {
    fn drop(&mut self) {
        self.relay.abort();
        let _ = self.gateway.start_kill();
        let _ = self.broker.start_kill();
    }
}

/// Start the imported broker/gateway pair and install its narrow client
/// configuration for ACP and world-model processes.
pub async fn start(
    configs: &[CompanyConfig],
    root: &std::path::Path,
    capabilities: crate::capability::CapabilityIssuer,
    spend: crate::spend::SpendLedger,
) -> Result<Processes> {
    let provider_credentials = provider_credentials(configs).await?;
    if provider_credentials.is_empty() {
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
        provider_credentials.keys().map(String::as_str).collect(),
    )
    .await?;
    for (provider, credential) in &provider_credentials {
        let ProviderCredential::ApiKey(key) = credential else {
            continue;
        };
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
    reconcile_credentials(&http, &broker_token, &provider_credentials).await?;

    let gateway_token = token(&omp, GATEWAY_PROFILE, "auth-gateway").await?;
    let mut gateway = Command::new(&omp)
        .env("OMP_PROFILE", GATEWAY_PROFILE)
        .env("OMP_AUTH_BROKER_URL", BROKER_URL)
        .env("OMP_AUTH_BROKER_TOKEN", &broker_token)
        .args(["auth-gateway", "serve", "--bind", OMP_GATEWAY_BIND])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .context("start OMP model auth gateway")?;
    let required = provider_credentials
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    wait_for_gateway(&mut gateway, &gateway_token, &required).await?;
    let providers = provider_credentials
        .into_iter()
        .map(|(provider, credential)| (provider, credential.billing()))
        .collect();

    CLIENT
        .set(ClientConfig { providers })
        .map_err(|_| anyhow::anyhow!("model gateway client was already installed"))?;
    let relay = start_runtime_relay(RelayState {
        root: root.to_path_buf(),
        capabilities,
        spend,
        upstream_token: gateway_token,
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(15 * 60))
            .build()
            .context("build Runtime model relay client")?,
    })
    .await?;
    Ok(Processes {
        broker,
        gateway,
        relay,
    })
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

    fn is_oauth(&self) -> bool {
        self.credential.get("type").and_then(|value| value.as_str()) == Some("oauth")
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
async fn reconcile_credentials(
    http: &reqwest::Client,
    broker_token: &str,
    provider_credentials: &BTreeMap<String, ProviderCredential>,
) -> Result<()> {
    let snapshot = broker_snapshot(http, broker_token).await?;
    for (provider, expected) in provider_credentials {
        let (keep_id, superseded_ids) = match expected {
            ProviderCredential::ApiKey(expected_key) => {
                canonical_api_key_credential(&snapshot, provider, expected_key)?
            }
            ProviderCredential::OmpOauth => canonical_oauth_credential(&snapshot, provider)?,
        };
        for credential in snapshot
            .credentials
            .iter()
            .filter(|credential| superseded_ids.contains(&credential.id))
        {
            disable_credential(
                http,
                broker_token,
                credential,
                "superseded by the current Restless model credential reference",
            )
            .await?;
        }

        let verified = broker_snapshot(http, broker_token).await?;
        let active = verified
            .credentials
            .iter()
            .filter(|credential| credential.provider == *provider)
            .collect::<Vec<_>>();
        let matches = active.len() == 1
            && active[0].id == keep_id
            && match expected {
                ProviderCredential::ApiKey(expected_key) => {
                    active[0].api_key() == Some(expected_key.as_str())
                }
                ProviderCredential::OmpOauth => active[0].is_oauth(),
            };
        if !matches {
            bail!("host model broker did not converge {provider} to one current credential");
        }
    }
    Ok(())
}

fn canonical_api_key_credential(
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

fn canonical_oauth_credential(
    snapshot: &BrokerSnapshot,
    provider: &str,
) -> Result<(i64, Vec<i64>)> {
    let provider_rows = snapshot
        .credentials
        .iter()
        .filter(|credential| credential.provider == provider)
        .collect::<Vec<_>>();
    let matching = provider_rows
        .iter()
        .filter(|credential| credential.is_oauth())
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        bail!(
            "host model broker has {} OAuth credentials for {provider}; expected exactly one owner-authenticated account",
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

#[derive(Clone)]
struct RelayState {
    root: std::path::PathBuf,
    capabilities: crate::capability::CapabilityIssuer,
    spend: crate::spend::SpendLedger,
    upstream_token: String,
    http: reqwest::Client,
}

async fn start_runtime_relay(state: RelayState) -> Result<tokio::task::JoinHandle<()>> {
    let listener = tokio::net::TcpListener::bind(RELAY_BIND)
        .await
        .with_context(|| format!("bind Runtime model relay {RELAY_BIND}"))?;
    let app = Router::new()
        .route("/v1/pi/stream", post(relay_pi_stream))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state);
    Ok(tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            tracing::error!("Runtime model relay stopped: {error}");
        }
    }))
}

async fn relay_pi_stream(
    State(state): State<RelayState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let token = match bearer_capability(&headers) {
        Ok(token) => token,
        Err(error) => return relay_error(StatusCode::UNAUTHORIZED, &error),
    };
    let grant = match state.capabilities.verify_model(token) {
        Ok(grant) => grant,
        Err(error) => {
            return relay_error(
                StatusCode::UNAUTHORIZED,
                &format!("invalid model capability: {error:#}"),
            )
        }
    };
    let request = match serde_json::from_slice::<serde_json::Value>(&body) {
        Ok(request) => request,
        Err(_) => return relay_error(StatusCode::BAD_REQUEST, "model request body is not JSON"),
    };
    let model = match requested_model(&request) {
        Some(model) => model,
        None => return relay_error(StatusCode::BAD_REQUEST, "model request has no modelId"),
    };
    let provider = match split_model(model) {
        Ok((provider, _)) => provider,
        Err(_) => {
            return relay_error(
                StatusCode::BAD_REQUEST,
                "model request must use a provider-qualified model id",
            )
        }
    };
    if provider != grant.provider || model != grant.model {
        return relay_error(
            StatusCode::FORBIDDEN,
            "model capability does not permit this exact model",
        );
    }
    let config = match CompanyConfig::load(&state.root, &grant.company) {
        Ok(config) => config,
        Err(_) => {
            return relay_error(
                StatusCode::FORBIDDEN,
                "model capability company is unavailable",
            )
        }
    };
    let billing = match grant.billing.as_str() {
        "metered_api" => ModelBilling::MeteredApi,
        "subscription" => ModelBilling::Subscription,
        _ => {
            return relay_error(
                StatusCode::UNAUTHORIZED,
                "model capability has an invalid billing policy",
            )
        }
    };
    if billing == ModelBilling::MeteredApi {
        let budget = state.spend.budget_state(&config);
        if !budget.is_available() {
            return relay_error(
                StatusCode::PAYMENT_REQUIRED,
                &budget.owner_message(&config.name),
            );
        }
    }

    let mut upstream_request = state
        .http
        .post(format!("{OMP_GATEWAY_HOST_URL}/v1/pi/stream"))
        .bearer_auth(&state.upstream_token)
        .header(CONTENT_TYPE, "application/json")
        .body(body);
    if let Some(accept) = headers.get(ACCEPT) {
        upstream_request = upstream_request.header(ACCEPT, accept);
    }
    let upstream = match upstream_request.send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(company = %grant.company, actor = %grant.actor, "model relay upstream transport: {error}");
            return relay_error(StatusCode::BAD_GATEWAY, "host model gateway is unavailable");
        }
    };
    if !upstream.status().is_success() {
        let status = upstream.status();
        tracing::warn!(
            company = %grant.company,
            actor = %grant.actor,
            upstream_status = %status,
            "host model gateway refused Runtime request"
        );
        return relay_error(status, "host model gateway refused the model request");
    }

    let status = upstream.status();
    let upstream_headers = upstream.headers().clone();
    let stream = MeteredStream::new(
        upstream.bytes_stream(),
        state.spend.meter(),
        MeteredRequest {
            company: grant.company,
            actor: grant.actor,
            session: grant.session,
            model: model.to_string(),
            billing,
        },
    );
    let mut response = Response::builder().status(status);
    for name in [CONTENT_TYPE, CACHE_CONTROL] {
        if let Some(value) = upstream_headers.get(&name) {
            response = response.header(name, value);
        }
    }
    response
        .body(Body::from_stream(stream))
        .unwrap_or_else(|_| relay_error(StatusCode::BAD_GATEWAY, "could not relay model stream"))
}

fn bearer_capability(headers: &HeaderMap) -> std::result::Result<&str, String> {
    let raw = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "model request has no bearer capability".to_string())?;
    raw.strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .ok_or_else(|| "model request bearer capability is malformed".to_string())
}

fn requested_model(request: &serde_json::Value) -> Option<&str> {
    request
        .get("modelId")
        .and_then(serde_json::Value::as_str)
        .filter(|model| !model.is_empty())
        .or_else(|| {
            request
                .get("model")
                .and_then(serde_json::Value::as_str)
                .filter(|model| !model.is_empty())
        })
        .or_else(|| {
            request
                .get("model")
                .and_then(serde_json::Value::as_object)
                .and_then(|model| model.get("id"))
                .and_then(serde_json::Value::as_str)
                .filter(|model| !model.is_empty())
        })
}

fn relay_error(status: StatusCode, message: &str) -> Response<Body> {
    let body = serde_json::json!({ "error": { "message": message } }).to_string();
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("static relay error response")
}

struct MeteredRequest {
    company: String,
    actor: String,
    session: String,
    model: String,
    billing: ModelBilling,
}

/// The relay forwards chunks unchanged but observes enough pi-native SSE to
/// make the terminal charged usage a host-side record. A provider error is a
/// valid terminal only when its canonical error message carries a provider
/// cost (including zero). A semantic pi-native `done` or `error` message is
/// the accounting boundary. The `[DONE]` sentinel is transport cleanup, and
/// a partial, Drop, or EOF without a semantic terminal remains ambiguous and
/// therefore poisons the company.
struct MeteredStream {
    inner: Pin<Box<dyn Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send>>,
    meter: crate::spend::TurnMeter,
    request: MeteredRequest,
    frame_buffer: Vec<u8>,
    settled: bool,
    failed: bool,
}

impl MeteredStream {
    fn new<S>(inner: S, meter: crate::spend::TurnMeter, request: MeteredRequest) -> Self
    where
        S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send + 'static,
    {
        Self {
            inner: Box::pin(inner),
            meter,
            request,
            frame_buffer: Vec::new(),
            settled: false,
            failed: false,
        }
    }

    fn observe(&mut self, bytes: &Bytes) {
        if self.settled || self.failed {
            return;
        }
        self.frame_buffer.extend_from_slice(bytes);
        if self.frame_buffer.len() > 256 * 1024 {
            self.failed = true;
            return;
        }
        while let Some((end, separator)) = sse_frame_end(&self.frame_buffer) {
            let frame = self.frame_buffer.drain(..end).collect::<Vec<_>>();
            self.frame_buffer.drain(..separator);
            self.observe_frame(&frame);
            if self.settled || self.failed {
                return;
            }
        }
    }

    /// The pi-native client accepts a final SSE event without the customary
    /// blank-line delimiter. Mirror that tolerant protocol behaviour here,
    /// but keep strict JSON parsing so a genuinely truncated terminal frame is
    /// still unaccountable and therefore fail-closed.
    fn finish(&mut self) {
        if self.settled || self.failed || self.frame_buffer.is_empty() {
            return;
        }
        let frame = std::mem::take(&mut self.frame_buffer);
        self.observe_frame(&frame);
    }

    fn observe_frame(&mut self, frame: &[u8]) {
        let Ok(frame) = std::str::from_utf8(frame) else {
            self.failed = true;
            return;
        };
        let data = frame
            .lines()
            .filter_map(|line| line.trim_end_matches('\r').strip_prefix("data:"))
            .map(str::trim_start)
            .collect::<Vec<_>>()
            .join("\n");
        if data.is_empty() {
            return;
        }
        if data == "[DONE]" {
            // A normal semantic `done` or `error` should already have settled
            // the request. A bare sentinel cannot prove spend and remains
            // fail-closed.
            self.record_terminal_usage(None);
            return;
        }
        let Ok(event) = serde_json::from_str::<serde_json::Value>(&data) else {
            self.failed = true;
            return;
        };
        match event.get("type").and_then(serde_json::Value::as_str) {
            Some("done") => self.record_terminal_usage(event.pointer("/message/usage")),
            Some("error") => {
                // pi-native error events are terminal messages too. A provider
                // refusal may legitimately cost $0, but accepting it requires
                // the same exact usage envelope as a success.
                self.record_terminal_usage(event.pointer("/error/usage"));
            }
            _ => {}
        }
    }

    fn record_terminal_usage(&mut self, usage: Option<&serde_json::Value>) {
        if self.settled || self.failed {
            return;
        }
        let tokens = usage.map(total_tokens).unwrap_or_default();
        let micro_usd = match self.request.billing {
            ModelBilling::MeteredApi => usage
                .and_then(|usage| usage.pointer("/cost/total"))
                .and_then(ceiling_micro_usd),
            ModelBilling::Subscription => Some(0),
        };
        let Some(micro_usd) = micro_usd else {
            self.failed = true;
            return;
        };
        self.meter.record_exact(
            &self.request.company,
            &self.request.actor,
            &self.request.session,
            &self.request.model,
            tokens,
            micro_usd,
        );
        self.settled = true;
    }

    fn fail_closed(&mut self, detail: &str) {
        if self.request.billing == ModelBilling::MeteredApi && !self.settled {
            self.meter.poison(&self.request.company);
            tracing::error!(
                company = %self.request.company,
                actor = %self.request.actor,
                session = %self.request.session,
                "metered model response had no terminal charged usage: {detail}"
            );
            self.settled = true;
        }
    }
}

impl Stream for MeteredStream {
    type Item = std::result::Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                this.observe(&bytes);
                Poll::Ready(Some(Ok(bytes)))
            }
            Poll::Ready(Some(Err(error))) => {
                this.fail_closed("upstream stream failed");
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                this.finish();
                if !this.settled || this.failed {
                    this.fail_closed("stream ended before a valid done event");
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for MeteredStream {
    fn drop(&mut self) {
        if !self.settled || self.failed {
            self.fail_closed("relay body dropped before a valid done event");
        }
    }
}

fn sse_frame_end(buffer: &[u8]) -> Option<(usize, usize)> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|end| (end, 4))
        .or_else(|| {
            buffer
                .windows(2)
                .position(|window| window == b"\n\n")
                .map(|end| (end, 2))
        })
}

fn total_tokens(usage: &serde_json::Value) -> u64 {
    if let Some(tokens) = usage.get("totalTokens").and_then(serde_json::Value::as_u64) {
        return tokens;
    }
    ["input", "output", "cacheRead", "cacheWrite"]
        .into_iter()
        .filter_map(|field| usage.get(field).and_then(serde_json::Value::as_u64))
        .fold(0_u64, u64::saturating_add)
}

/// Convert the provider's decimal charge into the ledger's coarser
/// micro-USD unit without ever understating spend. Providers may report
/// hundredths of a micro-dollar (for example `$0.01104072`); an upward
/// quantisation of less than one micro-dollar preserves the hard ceiling even
/// though the ledger does not store a fractional micro-dollar.
fn ceiling_micro_usd(value: &serde_json::Value) -> Option<u64> {
    let raw = value.as_number()?.to_string();
    let exponent_at = raw.find(['e', 'E']);
    let (significand, exponent) = match exponent_at {
        Some(index) => {
            let (significand, exponent) = raw.split_at(index);
            let exponent = exponent.get(1..)?.parse::<i64>().ok()?;
            if raw[index + 1..].contains(['e', 'E']) {
                return None;
            }
            (significand, exponent)
        }
        None => (raw.as_str(), 0),
    };
    let (whole, fraction) = significand.split_once('.').unwrap_or((significand, ""));
    if whole.is_empty()
        || whole.starts_with('-')
        || whole.starts_with('+')
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let digits = format!("{whole}{fraction}").parse::<u64>().ok()?;
    if digits == 0 {
        return Some(0);
    }
    // A JSON number might serialise in exponent form (for example 0.000001 as
    // 1e-6); normalise it with integer powers only. The ledger uses whole
    // micro-USD, so retain an exact value when possible and otherwise round
    // *up* rather than leave real spend outside the hard envelope.
    let shift = exponent
        .checked_sub(fraction.len() as i64)?
        .checked_add(6)?;
    if shift >= 0 {
        let power: u32 = shift.try_into().ok()?;
        digits.checked_mul(10_u64.checked_pow(power)?)
    } else {
        let scale = shift.checked_abs()?;
        // Any positive value smaller than 10^-19 USD still consumes one whole
        // micro-USD in a conservative integer ledger. Large positive shifts
        // cannot fit the ledger and stay fail-closed.
        if scale > 19 {
            return Some(u64::from(digits != 0));
        }
        let divisor = 10_u64.checked_pow(scale.try_into().ok()?)?;
        let whole = digits / divisor;
        whole.checked_add(u64::from(digits % divisor != 0))
    }
}

pub fn client() -> Result<&'static ClientConfig> {
    CLIENT
        .get()
        .context("host model gateway is not installed; restlessd did not finish booting")
}

pub fn oauth_is_loaded(provider: &str) -> Result<bool> {
    Ok(matches!(
        client()?.providers.get(provider),
        Some(ModelBilling::Subscription)
    ))
}

pub fn models_config(provider: &str, runtime_url: &str, token_env: &str) -> Result<String> {
    if !provider
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        bail!("invalid model provider identifier {provider:?}");
    }
    if runtime_url != RELAY_RUNTIME_URL || token_env != MODEL_CAPABILITY_ENV {
        bail!("refusing an unrecognised model gateway route");
    }
    Ok(format!(
        "# Managed by Restless. Contains a gateway route, never a provider credential.\n\
providers:\n  {provider}:\n    baseUrl: {runtime_url}\n    apiKey: {token_env}\n    transport: pi-native\n"
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProviderCredential {
    ApiKey(String),
    OmpOauth,
}

impl ProviderCredential {
    fn billing(&self) -> ModelBilling {
        match self {
            Self::ApiKey(_) => ModelBilling::MeteredApi,
            Self::OmpOauth => ModelBilling::Subscription,
        }
    }
}

async fn provider_credentials(
    configs: &[CompanyConfig],
) -> Result<BTreeMap<String, ProviderCredential>> {
    let mut credentials = BTreeMap::<String, ProviderCredential>::new();
    // Resolve only explicit host references on the first pass. Throwaway
    // configs deliberately carry none; they may use an installation route
    // authorised by another company, but iteration order must never decide it.
    for config in configs {
        let (primary_provider, _) = split_model(&config.model)?;
        for model in config.model_candidates()? {
            let (provider, _) = split_model(model)?;
            let provider_capability = format!("model.inference.{provider}");
            let reference = config.credentials.get(&provider_capability).or_else(|| {
                (provider == primary_provider)
                    .then(|| config.credentials.get("model.inference"))
                    .flatten()
            });
            let Some(reference) = reference else {
                continue;
            };
            let credential = match crate::credential::omp_oauth_provider(reference)? {
                Some(referenced_provider) => {
                    if referenced_provider != provider {
                        bail!(
                            "company {} maps {provider_capability} to OAuth provider {referenced_provider}",
                            config.name
                        );
                    }
                    ProviderCredential::OmpOauth
                }
                None => ProviderCredential::ApiKey(
                    crate::credential::resolve_reference(reference)
                        .await
                        .with_context(|| {
                            format!("resolve {provider} model credential for {}", config.name)
                        })?,
                ),
            };
            if let Some(existing) = credentials.get(provider) {
                if existing != &credential {
                    bail!(
                        "V0 model gateway refuses different {provider} credentials across companies; separate provider custody before multi-account use"
                    );
                }
            } else {
                credentials.insert(provider.to_string(), credential);
            }
        }
    }
    for config in configs {
        for model in config.model_candidates()? {
            let (provider, _) = split_model(model)?;
            if !credentials.contains_key(provider) {
                bail!(
                    "no configured company provides a host credential reference for model {model}; set credentials.model.inference.{provider}"
                );
            }
        }
    }
    Ok(credentials)
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
    let mut observed = BTreeSet::new();
    for _ in 0..300 {
        if let Some(status) = child.try_wait().context("inspect OMP gateway")? {
            bail!("OMP model auth gateway exited during boot ({status})");
        }
        if let Ok(response) = http
            .get(format!("{OMP_GATEWAY_HOST_URL}/v1/models"))
            .bearer_auth(token)
            .send()
            .await
        {
            if response.status().is_success() {
                if let Ok(list) = response.json::<ModelList>().await {
                    observed = list
                        .data
                        .iter()
                        .filter_map(|model| model.id.split_once('/').map(|(provider, _)| provider))
                        .map(str::to_string)
                        .collect();
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
    let missing = providers.difference(&observed).cloned().collect::<Vec<_>>();
    bail!(
        "OMP model auth gateway did not expose configured providers {:?} (observed {:?})",
        missing,
        observed
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "restless-model-relay-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        CompanyConfig::save(
            &root,
            &CompanyConfig {
                name: "acme_test".into(),
                mission: "relay test".into(),
                spend_ceiling_usd: crate::runtime::SpendCeiling::from_micro_usd(2),
                model: "moonshot/kimi-k3".into(),
                model_failover: Vec::new(),
                credentials: BTreeMap::new(),
                approved_parties: Vec::new(),
            },
        )
        .unwrap();
        root
    }

    fn test_relay_state(
        root: &std::path::Path,
    ) -> (
        crate::capability::CapabilityIssuer,
        crate::spend::SpendLedger,
        RelayState,
    ) {
        let capabilities = crate::capability::CapabilityIssuer::open(root).unwrap();
        let spend = crate::spend::SpendLedger::open(root).unwrap();
        let state = RelayState {
            root: root.to_path_buf(),
            capabilities: capabilities.clone(),
            spend: spend.clone(),
            upstream_token: "host-only-root-bearer".into(),
            http: reqwest::Client::new(),
        };
        (capabilities, spend, state)
    }

    fn bearer_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        headers
    }

    #[test]
    fn runtime_model_config_contains_only_the_narrow_gateway_route() {
        let config = models_config("moonshot", RELAY_RUNTIME_URL, MODEL_CAPABILITY_ENV).unwrap();
        assert!(config.contains("\n  moonshot:\n    baseUrl:"));
        assert!(config.contains("transport: pi-native"));
        assert!(config.contains("apiKey: RESTLESS_MODEL_CAPABILITY"));
        assert!(!config.contains("MOONSHOT_API_KEY"));
        assert!(!config.contains("api.kimi.com"));
    }

    #[test]
    fn provider_and_route_are_not_open_ended_injection_points() {
        assert!(
            models_config("moonshot\nheaders", RELAY_RUNTIME_URL, MODEL_CAPABILITY_ENV).is_err()
        );
        assert!(
            models_config("moonshot", "https://example.invalid", MODEL_CAPABILITY_ENV).is_err()
        );
    }

    #[test]
    fn agent_model_access_is_a_signed_company_actor_session_exact_model_grant() {
        let root = test_root();
        let (issuer, _spend, _state) = test_relay_state(&root);
        let client = ClientConfig {
            providers: BTreeMap::from([("moonshot".into(), ModelBilling::MeteredApi)]),
        };
        let access = client
            .auth_for(
                "moonshot/kimi-k3",
                &issuer,
                "acme_test",
                "delivery-lead",
                "session_123",
            )
            .unwrap();
        assert_eq!(access.token_env, MODEL_CAPABILITY_ENV);
        assert_eq!(access.runtime_url, RELAY_RUNTIME_URL);
        assert_eq!(
            issuer.verify_model(&access.token).unwrap(),
            crate::capability::ModelGrant {
                company: "acme_test".into(),
                actor: "delivery-lead".into(),
                session: "session_123".into(),
                provider: "moonshot".into(),
                model: "moonshot/kimi-k3".into(),
                billing: "metered_api".into(),
            }
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn relay_refuses_a_provider_mismatch_and_exhausted_company_before_forwarding() {
        let root = test_root();
        let (issuer, spend, state) = test_relay_state(&root);
        let token = issuer
            .issue_model_session(
                "acme_test",
                "delivery-lead",
                "session_123",
                "moonshot",
                "moonshot/kimi-k3",
                "metered_api",
            )
            .unwrap();
        let mismatched = relay_pi_stream(
            State(state.clone()),
            bearer_headers(&token),
            Bytes::from_static(br#"{"modelId":"openai/gpt-5"}"#),
        )
        .await;
        assert_eq!(mismatched.status(), StatusCode::FORBIDDEN);

        let same_provider_wrong_model = relay_pi_stream(
            State(state.clone()),
            bearer_headers(&token),
            Bytes::from_static(br#"{"modelId":"moonshot/another-model"}"#),
        )
        .await;
        assert_eq!(same_provider_wrong_model.status(), StatusCode::FORBIDDEN);

        spend.meter().record_exact(
            "acme_test",
            "delivery-lead",
            "prior_session",
            "moonshot/kimi-k3",
            1,
            2,
        );
        let exhausted = relay_pi_stream(
            State(state),
            bearer_headers(&token),
            Bytes::from_static(br#"{"modelId":"moonshot/kimi-k3"}"#),
        )
        .await;
        assert_eq!(exhausted.status(), StatusCode::PAYMENT_REQUIRED);

        drop(spend);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relay_records_one_exact_terminal_charge_and_poison_missing_terminal_usage() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        let event = serde_json::json!({
            "type": "done",
            "message": {
                "usage": {
                    "input": 3,
                    "output": 5,
                    "cost": { "total": 0.000001 }
                }
            }
        });
        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                MeteredRequest {
                    company: "acme_test".into(),
                    actor: "delivery-lead".into(),
                    session: "session_123".into(),
                    model: "moonshot/kimi-k3".into(),
                    billing: ModelBilling::MeteredApi,
                },
            );
            let frame = Bytes::from(format!("data: {event}\n\n"));
            stream.observe(&frame);
            stream.observe(&frame);
        }
        assert_eq!(
            ledger
                .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                .remaining_micro_usd(),
            Some(1),
            "one terminal event writes one exact micro-USD record"
        );
        let spool = std::fs::read_to_string(root.join("spend/spend.jsonl")).unwrap();
        let records = spool
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["actorId"], "delivery-lead");
        assert_eq!(records[0]["sessionId"], "session_123");
        assert_eq!(records[0]["totalTokens"], 8);
        assert_eq!(records[0]["costMicroUsd"], 1);

        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let _missing_terminal = MeteredStream::new(
                inner,
                ledger.meter(),
                MeteredRequest {
                    company: "acme_test".into(),
                    actor: "delivery-lead".into(),
                    session: "session_missing_usage".into(),
                    model: "moonshot/kimi-k3".into(),
                    billing: ModelBilling::MeteredApi,
                },
            );
        }
        assert_eq!(
            ledger.budget_state_for(
                "acme_test",
                crate::runtime::SpendCeiling::from_micro_usd(2)
            ).remaining_micro_usd(),
            None,
            "a metered stream without terminal charged usage leaves metering unknown and blocks further charged requests"
        );

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relay_accepts_an_explicitly_accounted_provider_error_but_not_an_ambiguous_one() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        let explicit_zero_cost_error = serde_json::json!({
            "type": "error",
            "reason": "error",
            "error": {
                "usage": {
                    "input": 0,
                    "output": 0,
                    "cost": { "total": 0 }
                }
            }
        });
        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                MeteredRequest {
                    company: "acme_test".into(),
                    actor: "delivery-lead".into(),
                    session: "provider_error_with_zero_cost".into(),
                    model: "moonshot/kimi-k3".into(),
                    billing: ModelBilling::MeteredApi,
                },
            );
            stream.observe(&Bytes::from(format!(
                "data: {explicit_zero_cost_error}\n\n"
            )));
        }
        assert_eq!(
            ledger
                .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                .remaining_micro_usd(),
            Some(2),
            "an error with an exact $0 usage envelope does not create unknown spend"
        );

        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                MeteredRequest {
                    company: "acme_test".into(),
                    actor: "delivery-lead".into(),
                    session: "ambiguous_provider_error".into(),
                    model: "moonshot/kimi-k3".into(),
                    billing: ModelBilling::MeteredApi,
                },
            );
            stream.observe(&Bytes::from_static(
                b"data: {\"type\":\"error\",\"reason\":\"error\"}\n\n",
            ));
        }
        assert_eq!(
            ledger
                .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                .remaining_micro_usd(),
            None,
            "an error with no exact usage remains fail-closed without a fake zero balance"
        );

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relay_poisoned_when_only_a_partial_and_wire_cleanup_arrive() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                MeteredRequest {
                    company: "acme_test".into(),
                    actor: "delivery-lead".into(),
                    session: "tool_use_partial".into(),
                    model: "moonshot/kimi-k3".into(),
                    billing: ModelBilling::MeteredApi,
                },
            );
            let partial = serde_json::json!({
                "type": "toolcall_end",
                "partial": {
                    "stopReason": "toolUse",
                    "usage": {
                        "input": 3,
                        "output": 5,
                        "cost": { "total": 0.000001 }
                    }
                }
            });
            stream.observe(&Bytes::from(format!("data: {partial}\n\n")));
            assert_eq!(
                ledger
                    .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                    .remaining_micro_usd(),
                Some(2),
                "a partial is not a provider-confirmed terminal usage record"
            );
            stream.observe(&Bytes::from_static(b"data: [DONE]\n\n"));
        }
        assert_eq!(
            ledger
                .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                .remaining_micro_usd(),
            None,
            "a bare wire sentinel cannot turn a partial into accounted provider spend"
        );

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relay_flushes_a_valid_terminal_frame_that_arrives_without_a_final_blank_line() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        let event = serde_json::json!({
            "type": "done",
            "message": {
                "usage": {
                    "input": 3,
                    "output": 5,
                    "cost": { "total": 0.000001 }
                }
            }
        });
        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                MeteredRequest {
                    company: "acme_test".into(),
                    actor: "delivery-lead".into(),
                    session: "trailing_terminal".into(),
                    model: "moonshot/kimi-k3".into(),
                    billing: ModelBilling::MeteredApi,
                },
            );
            stream.observe(&Bytes::from(format!("data: {event}\n")));
            assert_eq!(
                ledger
                    .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                    .remaining_micro_usd(),
                Some(2),
                "a trailing frame is held until EOF"
            );
            stream.finish();
        }
        assert_eq!(
            ledger
                .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                .remaining_micro_usd(),
            Some(1),
            "a valid trailing terminal frame records its exact cost"
        );

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn micro_usd_parser_preserves_or_conservatively_rounds_provider_decimals() {
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("0.000001").unwrap()),
            Some(1)
        );
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("12.34").unwrap()),
            Some(12_340_000)
        );
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("0.0000001").unwrap()),
            Some(1),
            "never round real provider spend down below one ledger unit"
        );
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("0.01104072").unwrap()),
            Some(11_041),
            "GLM's 8-place dollar prices are charged upward by less than one micro-dollar"
        );
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("1e-6").unwrap()),
            Some(1)
        );
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("1e-7").unwrap()),
            Some(1)
        );
        assert!(ceiling_micro_usd(&serde_json::from_str("-1").unwrap()).is_none());
    }

    #[test]
    fn relay_charges_a_glm_precision_terminal_without_poisoning() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        let event = serde_json::json!({
            "type": "done",
            "message": {
                "usage": {
                    "input": 7_568,
                    "output": 71,
                    "cacheRead": 512,
                    "cost": { "total": 0.01104072 }
                }
            }
        });
        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                MeteredRequest {
                    company: "acme_test".into(),
                    actor: "delivery-lead".into(),
                    session: "glm_precision".into(),
                    model: "zai/glm-5.3".into(),
                    billing: ModelBilling::MeteredApi,
                },
            );
            stream.observe(&Bytes::from(format!("data: {event}\n\n")));
        }
        assert_eq!(
            ledger
                .budget_state_for(
                    "acme_test",
                    crate::runtime::SpendCeiling::from_micro_usd(12_000)
                )
                .remaining_micro_usd(),
            Some(959),
            "the micro-USD ledger uses a conservative 11,041-micro charge"
        );

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
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
            canonical_api_key_credential(&snapshot, "moonshot", "current").unwrap(),
            (2, vec![1])
        );
        assert!(canonical_api_key_credential(&snapshot, "moonshot", "missing").is_err());
    }

    #[test]
    fn oauth_reference_keeps_exactly_one_authenticated_provider_row() {
        let snapshot: BrokerSnapshot = serde_json::from_value(serde_json::json!({
            "credentials": [
                {"id": 7, "provider": "anthropic", "credential": {"type": "oauth", "refresh": "<remote>", "access": "redacted", "expires": 1}},
                {"id": 8, "provider": "anthropic", "credential": {"type": "api_key", "key": "old"}}
            ]
        }))
        .unwrap();
        assert_eq!(
            canonical_oauth_credential(&snapshot, "anthropic").unwrap(),
            (7, vec![8])
        );
    }

    #[test]
    fn one_cooldown_read_drives_exec_and_staff_candidate_order() {
        let config = CompanyConfig {
            name: "continuity_test".into(),
            mission: String::new(),
            spend_ceiling_usd: crate::runtime::SpendCeiling::from_micro_usd(10_000_000),
            model: "moonshot/kimi-k3".into(),
            model_failover: vec!["anthropic/claude-sonnet-4-5".into()],
            credentials: BTreeMap::new(),
            approved_parties: Vec::new(),
        };
        let ordered = ordered_candidates(&config, Some("moonshot/kimi-k3")).unwrap();
        assert_eq!(ordered, ["moonshot/kimi-k3", "anthropic/claude-sonnet-4-5"]);
        let available = filter_cooling_candidates(
            ordered,
            &[crate::authority::ModelCooldown {
                model: "moonshot/kimi-k3".into(),
                kind: "quota".into(),
                reason: "allowance exhausted".into(),
                retry_at: chrono::Utc::now() + chrono::Duration::hours(1),
            }],
        )
        .unwrap();
        assert_eq!(available, ["anthropic/claude-sonnet-4-5"]);
    }

    #[test]
    fn explicit_staff_model_never_inherits_the_exec_fallback_chain() {
        let config = CompanyConfig {
            name: "exact_staff_test".into(),
            mission: String::new(),
            spend_ceiling_usd: crate::runtime::SpendCeiling::from_micro_usd(10_000_000),
            model: "openai-codex/gpt-5.6-sol".into(),
            model_failover: vec!["anthropic/claude-sonnet-4-6".into()],
            credentials: BTreeMap::new(),
            approved_parties: Vec::new(),
        };
        assert_eq!(
            actor_candidates(&config, Some("openai-codex/gpt-5.6-terra")).unwrap(),
            ["openai-codex/gpt-5.6-terra"]
        );
        assert_eq!(
            actor_candidates(&config, None).unwrap(),
            ["openai-codex/gpt-5.6-sol", "anthropic/claude-sonnet-4-6"]
        );
    }
}
