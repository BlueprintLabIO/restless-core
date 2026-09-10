use std::time::Duration;

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use url::Url;

use super::{CandidateBuildReceipt, CandidateBuildRequest, BUILD_CONTRACT_VERSION};
use restlessd::published_service_contract::{
    ProviderCleanupReceipt, ProviderEndpoint, ProviderReadyReceipt, PublishRequest, ServiceProfile,
    CONTRACT_VERSION,
};

pub(super) const HTTP_PROVIDER: &str = "cloud-http";
const URL_ENV: &str = "RESTLESS_PUBLISHED_SERVICE_PROVIDER_URL";
const TOKEN_ENV: &str = "RESTLESS_PUBLISHED_SERVICE_PROVIDER_TOKEN";
const TIMEOUT_ENV: &str = "RESTLESS_PUBLISHED_SERVICE_PROVIDER_TIMEOUT_MS";
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_DISPATCH_ATTEMPTS: usize = 3;

#[derive(Debug)]
pub(super) enum ProviderObservation {
    Ready(ProviderReadyReceipt),
    Provisioning(Value),
    Failed(Value),
    Cleaned(ProviderCleanupReceipt),
    Absent,
}

impl ProviderObservation {
    pub(super) fn status(&self) -> &'static str {
        match self {
            Self::Ready(_) => "ready",
            Self::Provisioning(_) => "provisioning",
            Self::Failed(_) => "failed",
            Self::Cleaned(_) => "cleaned",
            Self::Absent => "absent",
        }
    }

    pub(super) fn receipt(&self) -> Option<&ProviderReadyReceipt> {
        match self {
            Self::Ready(receipt) => Some(receipt),
            _ => None,
        }
    }

    pub(super) fn as_value(&self) -> Value {
        match self {
            Self::Ready(receipt) => {
                let mut value = serde_json::to_value(receipt).expect("receipt serializes");
                value["status"] = Value::String("ready".into());
                value
            }
            Self::Provisioning(value) | Self::Failed(value) => value.clone(),
            Self::Cleaned(receipt) => {
                let mut value = serde_json::to_value(receipt).expect("receipt serializes");
                value["status"] = Value::String("cleaned".into());
                value
            }
            Self::Absent => json!({ "status": "absent" }),
        }
    }
}

pub(super) struct HttpPublicationProvider {
    base_url: Url,
    token: String,
    request_timeout: Duration,
    client: reqwest::Client,
}

impl HttpPublicationProvider {
    pub(super) fn from_env() -> Result<Self> {
        let raw_url = std::env::var(URL_ENV).with_context(|| format!("{URL_ENV} is required"))?;
        let token = std::env::var(TOKEN_ENV).with_context(|| format!("{TOKEN_ENV} is required"))?;
        let timeout_ms = std::env::var(TIMEOUT_ENV)
            .ok()
            .map(|value| {
                value
                    .parse::<u64>()
                    .with_context(|| format!("{TIMEOUT_ENV} must be an integer"))
            })
            .transpose()?
            .unwrap_or(DEFAULT_TIMEOUT_MS);
        Self::new(&raw_url, token, Duration::from_millis(timeout_ms))
    }

    fn new(raw_url: &str, token: String, request_timeout: Duration) -> Result<Self> {
        let mut base_url = Url::parse(raw_url).context("parse published-service provider URL")?;
        if base_url.username() != "" || base_url.password().is_some() {
            bail!("published-service provider URL may not contain credentials");
        }
        if base_url.query().is_some() || base_url.fragment().is_some() {
            bail!("published-service provider URL may not contain a query or fragment");
        }
        if base_url.path() != "" && base_url.path() != "/" {
            bail!("published-service provider URL must be an origin without a path");
        }
        let loopback_http = base_url.scheme() == "http"
            && base_url
                .host_str()
                .and_then(|host| host.parse::<std::net::IpAddr>().ok())
                .is_some_and(|address| address.is_loopback());
        if base_url.scheme() != "https" && !loopback_http {
            bail!("published-service provider URL must use HTTPS (HTTP is restricted to loopback tests)");
        }
        if token.as_bytes().len() < 32 {
            bail!("{TOKEN_ENV} must contain at least 32 bytes");
        }
        if !(Duration::from_millis(25)..=Duration::from_secs(120)).contains(&request_timeout) {
            bail!("{TIMEOUT_ENV} must be in 25..=120000 milliseconds");
        }
        base_url.set_path("/");
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("build published-service provider HTTP client")?;
        Ok(Self {
            base_url,
            token,
            request_timeout,
            client,
        })
    }

    pub(super) async fn provision(
        &self,
        request: &PublishRequest,
        invitation_key: &[u8],
    ) -> Result<(ProviderReadyReceipt, bool)> {
        ensure_distributable_image(&request.candidate.image)?;
        let invitation_key_id = format!("sha256:{:x}", Sha256::digest(invitation_key));
        let envelope = json!({
            "request": request,
            "invitation_key_base64": base64::engine::general_purpose::STANDARD.encode(invitation_key),
            "invitation_key_id": invitation_key_id,
        });
        let deadline = request.start_deadline;
        let mut dispatch_attempts = 0;
        let mut ambiguous_dispatch = false;

        loop {
            ensure_before(
                deadline,
                "publication start deadline elapsed before provider readiness",
            )?;
            match self
                .observe_until(request, &invitation_key_id, deadline)
                .await?
            {
                ObserveCall::Observed(ProviderObservation::Ready(receipt)) => {
                    return Ok((receipt, ambiguous_dispatch || dispatch_attempts == 0));
                }
                ObserveCall::Observed(ProviderObservation::Cleaned(_)) => {
                    bail!(
                        "provider operation is already terminally cleaned and cannot be restarted"
                    )
                }
                ObserveCall::Observed(ProviderObservation::Provisioning(_)) => {
                    sleep_before(deadline).await?;
                    continue;
                }
                ObserveCall::Observed(ProviderObservation::Failed(value)) => {
                    if dispatch_attempts >= MAX_DISPATCH_ATTEMPTS {
                        bail!("provider operation failed: {}", bounded_failure(&value));
                    }
                }
                ObserveCall::Observed(ProviderObservation::Absent) => {
                    if dispatch_attempts >= MAX_DISPATCH_ATTEMPTS {
                        bail!("provider outcome remains absent after {MAX_DISPATCH_ATTEMPTS} idempotent dispatch attempts");
                    }
                }
                ObserveCall::Ambiguous if dispatch_attempts > 0 => {
                    ambiguous_dispatch = true;
                    sleep_before(deadline).await?;
                    continue;
                }
                ObserveCall::Ambiguous => {}
            }

            dispatch_attempts += 1;
            let path = "/v1/publications";
            match self
                .call(Method::POST, path, Some(&envelope), deadline)
                .await
            {
                CallResult::Response(status, body) if status == StatusCode::CREATED => {
                    match parse_observation(body, request, Some(&invitation_key_id)) {
                        Ok(ProviderObservation::Ready(receipt)) => {
                            return Ok((receipt, false));
                        }
                        Ok(_) | Err(_) => ambiguous_dispatch = true,
                    }
                }
                CallResult::Response(status, body) if status.is_client_error() => {
                    bail!(
                        "published-service provider rejected dispatch with {status}: {}",
                        bounded_failure(&body)
                    );
                }
                CallResult::Response(_, _) | CallResult::Ambiguous => {
                    ambiguous_dispatch = true;
                }
            }
            sleep_before(deadline).await?;
        }
    }

    pub(super) async fn build_candidate(
        &self,
        request: &CandidateBuildRequest,
    ) -> Result<(CandidateBuildReceipt, bool)> {
        let path = format!("/v1/builds/{}", request.operation_id);
        let body = serde_json::to_value(request).context("encode candidate build request")?;
        let mut dispatches = 0;
        let mut ambiguous = false;
        loop {
            ensure_before(
                request.deadline,
                "candidate build outcome remained ambiguous until its deadline",
            )?;
            match self.call(Method::GET, &path, None, request.deadline).await {
                CallResult::Response(StatusCode::OK, value) => {
                    if value.get("status").and_then(Value::as_str) == Some("ready") {
                        return Ok((
                            validate_build_receipt(value, request)?,
                            ambiguous || dispatches == 0,
                        ));
                    }
                    validate_build_progress(&value, request)?;
                }
                CallResult::Response(StatusCode::NOT_FOUND, _) => {}
                CallResult::Response(status, value) if status.is_client_error() => {
                    bail!(
                        "candidate build provider rejected observation with {status}: {}",
                        bounded_failure(&value)
                    )
                }
                CallResult::Response(_, _) | CallResult::Ambiguous => ambiguous = true,
            }
            if dispatches >= MAX_DISPATCH_ATTEMPTS {
                bail!("candidate build outcome remains ambiguous after {MAX_DISPATCH_ATTEMPTS} idempotent dispatch attempts");
            }
            dispatches += 1;
            match self
                .call(Method::POST, "/v1/builds", Some(&body), request.deadline)
                .await
            {
                CallResult::Response(StatusCode::CREATED, value) => {
                    return Ok((validate_build_receipt(value, request)?, ambiguous));
                }
                CallResult::Response(status, value) if status.is_client_error() => {
                    bail!(
                        "candidate build provider rejected dispatch with {status}: {}",
                        bounded_failure(&value)
                    )
                }
                CallResult::Response(_, _) | CallResult::Ambiguous => ambiguous = true,
            }
            sleep_before(request.deadline).await?;
        }
    }

    pub(super) async fn observe(
        &self,
        request: &PublishRequest,
        invitation_key: &[u8],
    ) -> Result<ProviderObservation> {
        let key_id = format!("sha256:{:x}", Sha256::digest(invitation_key));
        let deadline = Utc::now()
            + chrono::Duration::from_std(self.request_timeout)
                .context("published-service provider timeout is out of range")?;
        match self.observe_until(request, &key_id, deadline).await? {
            ObserveCall::Observed(observation) => Ok(observation),
            ObserveCall::Ambiguous => bail!("published-service provider observation was ambiguous"),
        }
    }

    pub(super) async fn cleanup(&self, request: &PublishRequest) -> Result<ProviderCleanupReceipt> {
        let deadline = Utc::now()
            + chrono::Duration::from_std(
                self.request_timeout * (MAX_DISPATCH_ATTEMPTS as u32 * 2 + 1),
            )
            .context("published-service provider timeout is out of range")?;
        let path = publication_path(&request.publication_id);
        let cleanup_envelope = json!({
            "contract_version": CONTRACT_VERSION,
            "publication_id": request.publication_id,
            "candidate_digest": request.candidate.manifest_digest,
            "expires_at": request.expires_at,
            "idempotency_key": request.idempotency_key,
        });
        let mut attempts = 0;
        loop {
            ensure_before(
                deadline,
                "provider cleanup outcome remained ambiguous until its deadline",
            )?;
            attempts += 1;
            match self
                .call(Method::DELETE, &path, Some(&cleanup_envelope), deadline)
                .await
            {
                CallResult::Response(status, body) if status == StatusCode::OK => {
                    match parse_observation(body, request, None) {
                        Ok(ProviderObservation::Cleaned(receipt)) => return Ok(receipt),
                        _ => {}
                    }
                }
                CallResult::Response(status, body)
                    if status.is_client_error() && status != StatusCode::NOT_FOUND =>
                {
                    bail!(
                        "published-service provider rejected cleanup with {status}: {}",
                        bounded_failure(&body)
                    );
                }
                _ => {}
            }

            match self.observe_until(request, "", deadline).await? {
                ObserveCall::Observed(ProviderObservation::Cleaned(receipt)) => return Ok(receipt),
                ObserveCall::Observed(ProviderObservation::Absent) => {
                    bail!("provider reports absence without the exact cleanup receipt required for terminal success")
                }
                ObserveCall::Observed(_) | ObserveCall::Ambiguous
                    if attempts < MAX_DISPATCH_ATTEMPTS =>
                {
                    sleep_before(deadline).await?;
                }
                ObserveCall::Observed(_) | ObserveCall::Ambiguous => {
                    bail!("provider cleanup outcome remains ambiguous after {MAX_DISPATCH_ATTEMPTS} idempotent attempts")
                }
            }
        }
    }

    async fn observe_until(
        &self,
        request: &PublishRequest,
        expected_key_id: &str,
        deadline: DateTime<Utc>,
    ) -> Result<ObserveCall> {
        let path = publication_path(&request.publication_id);
        match self.call(Method::GET, &path, None, deadline).await {
            CallResult::Response(StatusCode::NOT_FOUND, _) => {
                Ok(ObserveCall::Observed(ProviderObservation::Absent))
            }
            CallResult::Response(status, body) if status == StatusCode::OK => parse_observation(
                body,
                request,
                (!expected_key_id.is_empty()).then_some(expected_key_id),
            )
            .map(ObserveCall::Observed),
            CallResult::Response(status, body) if status.is_client_error() => {
                bail!(
                    "published-service provider rejected observation with {status}: {}",
                    bounded_failure(&body)
                )
            }
            CallResult::Response(status, _) if status.is_server_error() => {
                Ok(ObserveCall::Ambiguous)
            }
            CallResult::Response(status, _) => {
                bail!("published-service provider observation returned {status}")
            }
            CallResult::Ambiguous => Ok(ObserveCall::Ambiguous),
        }
    }

    async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
        deadline: DateTime<Utc>,
    ) -> CallResult {
        let remaining = match (deadline - Utc::now()).to_std() {
            Ok(value) if !value.is_zero() => value,
            _ => return CallResult::Ambiguous,
        };
        let timeout = remaining.min(self.request_timeout);
        let url = match self.base_url.join(path.trim_start_matches('/')) {
            Ok(url) => url,
            Err(_) => return CallResult::Ambiguous,
        };
        let mut request = self
            .client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
            .timeout(timeout);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(_) => return CallResult::Ambiguous,
        };
        let status = response.status();
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return CallResult::Ambiguous;
        }
        let bytes = match response.bytes().await {
            Ok(bytes) if bytes.len() <= MAX_RESPONSE_BYTES => bytes,
            _ => return CallResult::Ambiguous,
        };
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        CallResult::Response(status, body)
    }
}

fn validate_build_progress(value: &Value, request: &CandidateBuildRequest) -> Result<()> {
    let object = value
        .as_object()
        .context("candidate build progress must be an object")?;
    if object.get("contract_version").and_then(Value::as_str) != Some(BUILD_CONTRACT_VERSION)
        || object.get("operation_id").and_then(Value::as_str) != Some(&request.operation_id)
        || !matches!(
            object.get("status").and_then(Value::as_str),
            Some("reserved" | "snapshot" | "built" | "pushed")
        )
    {
        bail!("candidate build progress does not match the exact operation");
    }
    Ok(())
}

fn validate_build_receipt(
    value: Value,
    request: &CandidateBuildRequest,
) -> Result<CandidateBuildReceipt> {
    let receipt: CandidateBuildReceipt =
        serde_json::from_value(value).context("decode exact candidate build receipt")?;
    if receipt.status != "ready"
        || receipt.contract_version != BUILD_CONTRACT_VERSION
        || receipt.operation_id != request.operation_id
        || receipt.provider_operation_id != request.operation_id
        || receipt.source_commit != request.source_commit
        || receipt.work_id != request.work_id.to_string()
        || receipt.attempt_id != request.attempt_id.to_string()
        || receipt.source_artifact_ref_id != request.source_artifact_ref_id.to_string()
        || receipt.producing_actor != request.producing_actor
        || receipt.runtime_generation.is_empty()
        || receipt.runtime_generation.len() > 20
        || !receipt
            .runtime_generation
            .bytes()
            .all(|byte| byte.is_ascii_digit())
        || receipt.runtime_generation.starts_with('0')
        || !receipt
            .image
            .ends_with(&format!("@{}", receipt.image_digest))
        || !is_sha256(&receipt.image_digest)
        || !is_sha256(&receipt.context_digest)
        || receipt.built_at > receipt.pushed_at
        || receipt.pushed_at > receipt.verified_at
    {
        bail!("candidate build receipt does not match the exact source/build/digest");
    }
    ensure_distributable_image(&receipt.image)?;
    Ok(receipt)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

enum CallResult {
    Response(StatusCode, Value),
    Ambiguous,
}

enum ObserveCall {
    Observed(ProviderObservation),
    Ambiguous,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NonReadyObservation {
    status: String,
    contract_version: String,
    publication_id: String,
    candidate_digest: String,
    endpoint: ProviderEndpoint,
    artifact_running: bool,
    gateway_running: bool,
    failure: Option<String>,
}

fn parse_observation(
    mut value: Value,
    request: &PublishRequest,
    expected_key_id: Option<&str>,
) -> Result<ProviderObservation> {
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .context("provider response has no status")?;
    match status {
        "ready" => {
            value
                .as_object_mut()
                .context("provider ready response must be an object")?
                .remove("status");
            let receipt: ProviderReadyReceipt =
                serde_json::from_value(value).context("decode exact provider ready receipt")?;
            validate_ready(request, &receipt, expected_key_id)?;
            Ok(ProviderObservation::Ready(receipt))
        }
        "cleaned" => {
            value
                .as_object_mut()
                .context("provider cleanup response must be an object")?
                .remove("status");
            let receipt: ProviderCleanupReceipt =
                serde_json::from_value(value).context("decode exact provider cleanup receipt")?;
            validate_cleanup(request, &receipt)?;
            Ok(ProviderObservation::Cleaned(receipt))
        }
        "provisioning" | "failed" => {
            let observation: NonReadyObservation = serde_json::from_value(value.clone())
                .context("decode exact non-ready provider observation")?;
            if observation.contract_version != CONTRACT_VERSION
                || observation.status != status
                || observation.publication_id != request.publication_id
                || observation.candidate_digest != request.candidate.manifest_digest
                || observation.endpoint.profile != request.candidate.manifest.profile
            {
                bail!("provider observation does not match the exact authorized publication/build/profile");
            }
            let _ = (
                observation.artifact_running,
                observation.gateway_running,
                observation.failure,
            );
            if status == "provisioning" {
                Ok(ProviderObservation::Provisioning(value))
            } else {
                Ok(ProviderObservation::Failed(value))
            }
        }
        other => bail!("unsupported published-service provider status {other:?}"),
    }
}

fn validate_ready(
    request: &PublishRequest,
    receipt: &ProviderReadyReceipt,
    expected_key_id: Option<&str>,
) -> Result<()> {
    if receipt.contract_version != CONTRACT_VERSION
        || receipt.publication_id != request.publication_id
        || receipt.provider_operation_id != request.publication_id
        || receipt.candidate_digest != request.candidate.manifest_digest
        || receipt.endpoint.profile != request.candidate.manifest.profile
        || expected_key_id.is_some_and(|expected| receipt.invitation_key_id != expected)
        || receipt.endpoint.bound_port == 0
    {
        bail!("provider ready receipt does not match the exact authorized publication/build/profile/key");
    }
    let endpoint = Url::parse(&receipt.endpoint.public_endpoint)
        .context("provider ready receipt endpoint is not an absolute URL")?;
    if endpoint.host_str().is_none() || endpoint.username() != "" || endpoint.password().is_some() {
        bail!("provider ready receipt endpoint has no safe network authority");
    }
    match receipt.endpoint.profile {
        ServiceProfile::HttpsWebsocketDemo
            if endpoint.scheme() == "https"
                && receipt.endpoint.transport_security == "provider-tls" => {}
        ServiceProfile::GodotEnetUdp
            if endpoint.scheme() == "udp"
                && receipt.endpoint.transport_security == "signed-application-ticket" => {}
        _ => bail!("provider ready receipt endpoint transport does not match its released profile"),
    }
    Ok(())
}

fn validate_cleanup(request: &PublishRequest, receipt: &ProviderCleanupReceipt) -> Result<()> {
    if receipt.contract_version != CONTRACT_VERSION
        || receipt.publication_id != request.publication_id
        || receipt.candidate_digest != request.candidate.manifest_digest
    {
        bail!("provider cleanup receipt does not match the exact authorized publication/build");
    }
    if !receipt.provider_process_absent
        || !receipt.route_absent
        || !receipt.invitation_material_absent
        || !receipt.resource_lease_released
        || !receipt.temporary_files_absent
    {
        bail!("provider cleanup receipt does not prove terminal absence across every required surface");
    }
    Ok(())
}

fn ensure_distributable_image(image: &str) -> Result<()> {
    let repository = image
        .rsplit_once('@')
        .map(|(repository, _)| repository)
        .context("immutable candidate image has no digest separator")?;
    let registry = repository
        .split('/')
        .next()
        .context("immutable candidate image has no registry/repository")?;
    if matches!(registry, "localhost" | "host.docker.internal")
        || registry
            .split_once(':')
            .map(|(host, _)| host)
            .unwrap_or(registry)
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
    {
        bail!("cloud-http publication requires a remotely pullable immutable OCI digest, not a loopback image reference");
    }
    Ok(())
}

fn publication_path(publication_id: &str) -> String {
    format!("/v1/publications/{publication_id}")
}

fn bounded_failure(value: &Value) -> String {
    value
        .get("error")
        .or_else(|| value.get("failure"))
        .and_then(Value::as_str)
        .unwrap_or("provider returned no public diagnostic")
        .chars()
        .take(512)
        .collect()
}

fn ensure_before(deadline: DateTime<Utc>, message: &str) -> Result<()> {
    if Utc::now() >= deadline {
        bail!("{message}");
    }
    Ok(())
}

async fn sleep_before(deadline: DateTime<Utc>) -> Result<()> {
    ensure_before(deadline, "provider operation deadline elapsed")?;
    let remaining = (deadline - Utc::now())
        .to_std()
        .unwrap_or_default()
        .min(Duration::from_millis(200));
    tokio::time::sleep(remaining).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::{
        extract::State,
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };

    use super::*;

    const TOKEN: &str = "published-service-provider-test-token-0001";

    fn request() -> PublishRequest {
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../../docs/sprints/sprint-36/contract/v1/publish-request-https-websocket.json"
        ))
        .unwrap();
        let now = Utc::now();
        value["requested_at"] = json!(now);
        value["candidate"]["created_at"] = json!(now);
        value["start_deadline"] = json!(now + chrono::Duration::seconds(3));
        value["expires_at"] = json!(now + chrono::Duration::hours(1));
        serde_json::from_value(value).unwrap()
    }

    fn key() -> Vec<u8> {
        vec![23; 32]
    }

    fn ready(request: &PublishRequest, key: &[u8]) -> Value {
        json!({
            "status": "ready",
            "contract_version": CONTRACT_VERSION,
            "publication_id": request.publication_id,
            "candidate_digest": request.candidate.manifest_digest,
            "provider_operation_id": request.publication_id,
            "endpoint": {
                "profile": "https-websocket-demo",
                "public_endpoint": "https://publication.preview.restless.test",
                "bound_port": 443,
                "transport_security": "provider-tls"
            },
            "invitation_key_id": format!("sha256:{:x}", Sha256::digest(key)),
            "provider_process_id": 4312,
            "ready_at": Utc::now(),
        })
    }

    fn cleaned(request: &PublishRequest) -> Value {
        json!({
            "status": "cleaned",
            "contract_version": CONTRACT_VERSION,
            "publication_id": request.publication_id,
            "candidate_digest": request.candidate.manifest_digest,
            "provider_process_absent": true,
            "route_absent": true,
            "invitation_material_absent": true,
            "resource_lease_released": true,
            "temporary_files_absent": true,
            "cleaned_at": Utc::now(),
        })
    }

    fn assert_auth(headers: &HeaderMap) {
        let expected = format!("Bearer {TOKEN}");
        assert_eq!(
            headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some(expected.as_str())
        );
    }

    async fn serve(app: Router) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (format!("http://{address}"), server)
    }

    fn build_request() -> CandidateBuildRequest {
        let now = Utc::now();
        CandidateBuildRequest {
            contract_version: BUILD_CONTRACT_VERSION.into(),
            operation_id: "build-0123456789abcdefghijklmn".into(),
            owner_id: uuid::uuid!("01992bda-1e00-7000-8000-000000000001"),
            company_id: uuid::uuid!("01992bda-1e00-7000-8000-000000000002"),
            cell_id: uuid::uuid!("01992bda-1e00-7000-8000-000000000003"),
            company_handle: "c01992bda1e0070008000000000000002".into(),
            work_id: uuid::uuid!("01992bda-1e00-7000-8000-000000000004"),
            attempt_id: uuid::uuid!("01992bda-1e00-7000-8000-000000000005"),
            producing_actor: "game/backend".into(),
            source_artifact_ref_id: uuid::uuid!("01992bda-1e00-7000-8000-000000000006"),
            source_commit: "c".repeat(40),
            context_path: format!("reviews/git/{}/server", "c".repeat(40)),
            dockerfile: "Dockerfile".into(),
            idempotency_key: "game-server-build-7".into(),
            requested_at: now - chrono::Duration::minutes(1),
            deadline: now + chrono::Duration::seconds(3),
        }
    }

    fn build_ready(request: &CandidateBuildRequest) -> Value {
        let built = Utc::now() - chrono::Duration::seconds(2);
        let pushed = built + chrono::Duration::seconds(1);
        json!({
            "status": "ready",
            "contract_version": BUILD_CONTRACT_VERSION,
            "operation_id": request.operation_id,
            "provider_operation_id": request.operation_id,
            "image": format!("ghcr.io/blueprintlabio/restless-service-candidates@sha256:{}", "a".repeat(64)),
            "image_digest": format!("sha256:{}", "a".repeat(64)),
            "context_digest": format!("sha256:{}", "b".repeat(64)),
            "source_commit": request.source_commit,
            "runtime_generation": "7",
            "work_id": request.work_id,
            "attempt_id": request.attempt_id,
            "source_artifact_ref_id": request.source_artifact_ref_id,
            "producing_actor": request.producing_actor,
            "built_at": built,
            "pushed_at": pushed,
            "verified_at": pushed + chrono::Duration::seconds(1),
        })
    }

    #[tokio::test]
    async fn candidate_build_recovers_a_lost_http_response_by_stable_operation_id() {
        #[derive(Clone)]
        struct TestState {
            request: CandidateBuildRequest,
            dispatched: Arc<std::sync::atomic::AtomicBool>,
            captured: Arc<Mutex<Option<Value>>>,
        }
        async fn observe(State(state): State<TestState>, headers: HeaderMap) -> impl IntoResponse {
            assert_auth(&headers);
            if state.dispatched.load(std::sync::atomic::Ordering::SeqCst) {
                (StatusCode::OK, Json(build_ready(&state.request)))
            } else {
                (StatusCode::NOT_FOUND, Json(json!({"error": "not found"})))
            }
        }
        async fn build(
            State(state): State<TestState>,
            headers: HeaderMap,
            Json(body): Json<Value>,
        ) -> impl IntoResponse {
            assert_auth(&headers);
            *state.captured.lock().unwrap() = Some(body);
            state
                .dispatched
                .store(true, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(100)).await;
            (StatusCode::CREATED, Json(build_ready(&state.request)))
        }
        let request = build_request();
        let captured = Arc::new(Mutex::new(None));
        let state = TestState {
            request: request.clone(),
            dispatched: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            captured: Arc::clone(&captured),
        };
        let app = Router::new()
            .route("/v1/builds", post(build))
            .route("/v1/builds/{operation_id}", get(observe))
            .with_state(state);
        let (base, server) = serve(app).await;
        let provider =
            HttpPublicationProvider::new(&base, TOKEN.into(), Duration::from_millis(25)).unwrap();
        let (receipt, recovered) = provider.build_candidate(&request).await.unwrap();
        assert!(recovered);
        assert_eq!(receipt.operation_id, request.operation_id);
        assert_eq!(
            captured.lock().unwrap().clone().unwrap(),
            serde_json::to_value(&request).unwrap()
        );
        assert!(!serde_json::to_string(&receipt).unwrap().contains(TOKEN));
        server.abort();
    }

    #[test]
    fn candidate_build_rejects_adversarial_digest_lineage() {
        let request = build_request();
        let mut receipt = build_ready(&request);
        receipt["image_digest"] = json!(format!("sha256:{}", "f".repeat(64)));
        assert!(validate_build_receipt(receipt, &request).is_err());
    }

    #[tokio::test]
    async fn dispatches_the_exact_immutable_envelope_over_authenticated_http() {
        #[derive(Clone)]
        struct TestState {
            request: PublishRequest,
            key: Vec<u8>,
            captured: Arc<Mutex<Option<Value>>>,
        }
        async fn observe(State(state): State<TestState>, headers: HeaderMap) -> impl IntoResponse {
            assert_auth(&headers);
            if state.captured.lock().unwrap().is_some() {
                (StatusCode::OK, Json(ready(&state.request, &state.key)))
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "publication not found"})),
                )
            }
        }
        async fn provision(
            State(state): State<TestState>,
            headers: HeaderMap,
            Json(body): Json<Value>,
        ) -> impl IntoResponse {
            assert_auth(&headers);
            *state.captured.lock().unwrap() = Some(body);
            (StatusCode::CREATED, Json(ready(&state.request, &state.key)))
        }

        let request = request();
        let key = key();
        let captured = Arc::new(Mutex::new(None));
        let state = TestState {
            request: request.clone(),
            key: key.clone(),
            captured: Arc::clone(&captured),
        };
        let app = Router::new()
            .route("/v1/publications", post(provision))
            .route("/v1/publications/{publication_id}", get(observe))
            .with_state(state);
        let (base, server) = serve(app).await;
        let provider =
            HttpPublicationProvider::new(&base, TOKEN.into(), Duration::from_secs(1)).unwrap();
        let (receipt, adopted) = provider.provision(&request, &key).await.unwrap();
        assert!(!adopted);
        assert_eq!(receipt.publication_id, request.publication_id);
        let body = captured.lock().unwrap().clone().unwrap();
        assert_eq!(body["request"], serde_json::to_value(&request).unwrap());
        assert_eq!(
            body["request"]["candidate"]["image"],
            request.candidate.image
        );
        assert_eq!(
            body["invitation_key_id"],
            format!("sha256:{:x}", Sha256::digest(&key))
        );
        assert_eq!(
            body["invitation_key_base64"],
            base64::engine::general_purpose::STANDARD.encode(&key)
        );
        server.abort();
    }

    #[tokio::test]
    async fn a_lost_dispatch_response_is_recovered_by_stable_publication_identity() {
        #[derive(Clone)]
        struct TestState {
            request: PublishRequest,
            key: Vec<u8>,
            dispatched: Arc<std::sync::atomic::AtomicBool>,
        }
        async fn observe(State(state): State<TestState>, headers: HeaderMap) -> impl IntoResponse {
            assert_auth(&headers);
            if state.dispatched.load(std::sync::atomic::Ordering::SeqCst) {
                (StatusCode::OK, Json(ready(&state.request, &state.key)))
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "publication not found"})),
                )
            }
        }
        async fn provision(
            State(state): State<TestState>,
            headers: HeaderMap,
        ) -> impl IntoResponse {
            assert_auth(&headers);
            state
                .dispatched
                .store(true, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(100)).await;
            (StatusCode::CREATED, Json(ready(&state.request, &state.key)))
        }

        let request = request();
        let key = key();
        let state = TestState {
            request: request.clone(),
            key: key.clone(),
            dispatched: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        let app = Router::new()
            .route("/v1/publications", post(provision))
            .route("/v1/publications/{publication_id}", get(observe))
            .with_state(state);
        let (base, server) = serve(app).await;
        let provider =
            HttpPublicationProvider::new(&base, TOKEN.into(), Duration::from_millis(25)).unwrap();
        let (receipt, adopted) = provider.provision(&request, &key).await.unwrap();
        assert!(adopted);
        assert_eq!(receipt.provider_operation_id, request.publication_id);
        server.abort();
    }

    #[tokio::test]
    async fn adversarial_receipt_lineage_is_rejected() {
        #[derive(Clone)]
        struct TestState(PublishRequest, Vec<u8>);
        async fn observe(State(state): State<TestState>, headers: HeaderMap) -> impl IntoResponse {
            assert_auth(&headers);
            let mut response = ready(&state.0, &state.1);
            response["candidate_digest"] = json!(format!("sha256:{}", "f".repeat(64)));
            (StatusCode::OK, Json(response))
        }
        let request = request();
        let key = key();
        let app = Router::new()
            .route("/v1/publications/{publication_id}", get(observe))
            .with_state(TestState(request.clone(), key.clone()));
        let (base, server) = serve(app).await;
        let provider =
            HttpPublicationProvider::new(&base, TOKEN.into(), Duration::from_secs(1)).unwrap();
        let error = provider
            .provision(&request, &key)
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("does not match the exact authorized"),
            "{error}"
        );
        server.abort();
    }

    #[tokio::test]
    async fn a_lost_cleanup_response_recovers_the_exact_provider_tombstone() {
        #[derive(Clone)]
        struct TestState {
            request: PublishRequest,
            cleaned: Arc<std::sync::atomic::AtomicBool>,
            cleanup: Arc<Mutex<Option<Value>>>,
        }
        async fn observe(State(state): State<TestState>, headers: HeaderMap) -> impl IntoResponse {
            assert_auth(&headers);
            if state.cleaned.load(std::sync::atomic::Ordering::SeqCst) {
                (StatusCode::OK, Json(cleaned(&state.request)))
            } else {
                (StatusCode::OK, Json(ready(&state.request, &key())))
            }
        }
        async fn cleanup(
            State(state): State<TestState>,
            headers: HeaderMap,
            Json(body): Json<Value>,
        ) -> impl IntoResponse {
            assert_auth(&headers);
            *state.cleanup.lock().unwrap() = Some(body);
            state
                .cleaned
                .store(true, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(100)).await;
            (StatusCode::OK, Json(cleaned(&state.request)))
        }
        let request = request();
        let captured_cleanup = Arc::new(Mutex::new(None));
        let state = TestState {
            request: request.clone(),
            cleaned: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            cleanup: Arc::clone(&captured_cleanup),
        };
        let app = Router::new()
            .route(
                "/v1/publications/{publication_id}",
                get(observe).delete(cleanup),
            )
            .with_state(state);
        let (base, server) = serve(app).await;
        let provider =
            HttpPublicationProvider::new(&base, TOKEN.into(), Duration::from_millis(25)).unwrap();
        let receipt = provider.cleanup(&request).await.unwrap();
        assert!(receipt.provider_process_absent);
        assert!(receipt.route_absent);
        assert!(receipt.invitation_material_absent);
        let cleanup = captured_cleanup.lock().unwrap().clone().unwrap();
        assert_eq!(cleanup["publication_id"], request.publication_id);
        assert_eq!(
            cleanup["candidate_digest"],
            request.candidate.manifest_digest
        );
        assert_eq!(cleanup["idempotency_key"], request.idempotency_key);
        server.abort();
    }

    #[test]
    fn remote_provider_rejects_insecure_origins_and_loopback_candidate_images() {
        assert!(HttpPublicationProvider::new(
            "http://provider.example.test",
            TOKEN.into(),
            Duration::from_secs(1),
        )
        .is_err());
        let mut request = request();
        request.candidate.image = format!("127.0.0.1:5000/game@sha256:{}", "a".repeat(64));
        assert!(ensure_distributable_image(&request.candidate.image).is_err());
    }
}
