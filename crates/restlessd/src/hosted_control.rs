//! Authenticated Cloud-to-Core control endpoints for a hosted account plane.
//!
//! Fleet may ask this released Core process to prove its own readiness and to
//! initialise a company identity. It cannot create, resize, stop, replace or
//! inspect a Runtime: those powers remain exclusively in Cloud's private
//! Runtime Supervisor.

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    io::Read as _,
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, OriginalUri, Path as AxumPath, State, WebSocketUpgrade},
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, COOKIE, HOST, ORIGIN},
        HeaderMap, HeaderValue, Method, Response, StatusCode,
    },
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use futures_util::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sqlx::Row as _;
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use uuid::Uuid;

use restlessd::hosted_runtime::{
    secret_file_access_is_restricted, BootstrapSecret, CompanyBootstrapRequest,
    HostedCompanyProvisioner, HostedCompanyReadiness, HostedCompanyScope, HostedPlaneConfig,
    HostedPlaneValues, HostedRuntimeAdmission, HostedRuntimeError, HostedRuntimeIdentity,
    RuntimeBootstrapRequest, RuntimeBridgeBootstrap, RuntimeBridgeCapabilityKey,
    RuntimeBridgeGrant,
};
use restlessd::runtime_bridge::{
    RuntimeBridgeAuthority, RuntimeBridgeRegistry, RuntimeGrantConsumption,
};

use crate::{runtime, Daemon};

const CONTRACT_VERSION: u32 = 1;
const READINESS_LEASE_SECONDS: i64 = 20;
const MAX_INTERNAL_BODY: usize = 16 * 1024;
const MAX_MODEL_REQUEST_BODY: usize = 2 * 1024 * 1024;
const MAX_COORDINATION_FRAME: usize = 1024 * 1024;
const MAX_JWKS_BODY: usize = 64 * 1024;
const EXTERNAL_PROBE_TIMEOUT: Duration = Duration::from_millis(1_200);
const REQUIRED_CHECKS: [&str; 6] = [
    "authority",
    "credential_custody",
    "company_directory",
    "identity_handoff",
    "cockpit",
    "plane_database",
];

/// An environment credential that cannot be accidentally printed or encoded.
#[derive(Clone, PartialEq, Eq)]
struct BearerSecret(Arc<[u8]>);

impl std::fmt::Debug for BearerSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("BearerSecret([REDACTED])")
    }
}

impl BearerSecret {
    fn from_environment(name: &'static str) -> Result<Self> {
        let value = required_environment(name)?;
        if value.len() < 32
            || value.len() > 256
            || !value.bytes().all(|byte| byte.is_ascii_graphic())
        {
            anyhow::bail!("{name} must contain 32-256 printable non-whitespace bytes");
        }
        Ok(Self(Arc::from(value.into_bytes())))
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() < 32 || bytes.len() > 256 || !bytes.iter().all(u8::is_ascii_graphic) {
            anyhow::bail!("hosted bearer credentials must contain 32-256 printable bytes");
        }
        Ok(Self(Arc::from(bytes)))
    }

    fn authorizes(&self, headers: &HeaderMap) -> bool {
        let mut values = headers.get_all(AUTHORIZATION).iter();
        let Some(value) = values.next() else {
            return false;
        };
        if values.next().is_some() {
            return false;
        }
        let Ok(value) = value.to_str() else {
            return false;
        };
        let Some(candidate) = value.strip_prefix("Bearer ") else {
            return false;
        };
        constant_time_equal(&self.0, candidate.as_bytes())
    }

    fn same_as(&self, other: &Self) -> bool {
        constant_time_equal(&self.0, &other.0)
    }

    fn bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }
}

/// Exact immutable identity of one deployed hosted account plane.
///
/// The type intentionally has no `Debug` or `Serialize` implementation: it
/// also retains scoped credentials used by the endpoint handlers.
#[derive(Clone)]
pub(crate) struct HostedDeploymentConfig {
    owner_id: Uuid,
    plane_id: Uuid,
    hostname: String,
    desired_revision: i64,
    account_plane_image: String,
    company_runtime_image: String,
    release_manifest_digest: String,
    core_release: String,
    core_source_revision: String,
    cockpit_dir: PathBuf,
    jwks_url: reqwest::Url,
    model_credential_reference: String,
    plane_readiness_token: BearerSecret,
    #[allow(dead_code)]
    cell_readiness_token: BearerSecret,
    #[allow(dead_code)]
    activity_token: BearerSecret,
    #[allow(dead_code)]
    deletion_token: BearerSecret,
    runtime_bootstrap_token: BearerSecret,
    runtime_bootstrap_token_file: PathBuf,
}

struct HostedDeploymentValues {
    owner_id: String,
    plane_id: String,
    hostname: String,
    desired_revision: String,
    account_plane_image: String,
    company_runtime_image: String,
    release_manifest_digest: String,
    core_release: String,
    core_source_revision: String,
    cockpit_dir: String,
    entry_issuer: String,
    entry_jwks_url: String,
    model_credential_reference: String,
    model_relay_url: String,
    model_api_base_url: String,
    model_relay_token: BearerSecret,
    plane_readiness_token: BearerSecret,
    cell_readiness_token: BearerSecret,
    activity_token: BearerSecret,
    deletion_token: BearerSecret,
    runtime_bootstrap_token: BearerSecret,
    runtime_bootstrap_token_file: String,
}

impl HostedDeploymentConfig {
    /// Parse every hosted value together. One missing or mutable input is a
    /// startup error; local Core does not expose Cloud-only endpoints.
    pub(crate) fn from_environment() -> Result<Option<Self>> {
        let mode = std::env::var("RESTLESS_ENTRY_MODE").unwrap_or_else(|_| "local".into());
        if mode == "local" {
            return Ok(None);
        }
        if mode != "network" {
            anyhow::bail!("RESTLESS_ENTRY_MODE must be `local` or `network`");
        }
        if std::env::var("RESTLESS_ENTRY_ALLOW_INSECURE_HTTP")
            .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        {
            anyhow::bail!("hosted account planes require HTTPS entry and JWKS origins");
        }

        let bootstrap_file = required_environment("RESTLESS_RUNTIME_BOOTSTRAP_TOKEN_FILE")?;
        let bootstrap_bytes = read_bootstrap_secret_file(Path::new(&bootstrap_file))?;
        let config = Self::from_values(HostedDeploymentValues {
            owner_id: required_environment("RESTLESS_ENTRY_OWNER_ID")?,
            plane_id: required_environment("RESTLESS_ENTRY_PLANE_ID")?,
            hostname: required_environment("RESTLESS_ENTRY_HOST")?,
            desired_revision: required_environment("RESTLESS_DESIRED_REVISION")?,
            account_plane_image: required_environment("RESTLESS_ACCOUNT_PLANE_IMAGE")?,
            company_runtime_image: required_environment("RESTLESS_COMPANY_IMAGE")?,
            release_manifest_digest: required_environment("RESTLESS_RELEASE_MANIFEST_DIGEST")?,
            core_release: crate::release::CORE_VERSION.to_owned(),
            core_source_revision: crate::release::SOURCE_REVISION.to_owned(),
            cockpit_dir: required_environment("RESTLESS_COCKPIT_DIR")?,
            entry_issuer: required_environment("RESTLESS_ENTRY_ISSUER")?,
            entry_jwks_url: required_environment("RESTLESS_ENTRY_JWKS_URL")?,
            model_credential_reference: required_environment(
                "RESTLESS_HOSTED_MODEL_CREDENTIAL_REFERENCE",
            )?,
            model_relay_url: required_environment("RESTLESS_HOSTED_MODEL_RELAY_URL")?,
            model_api_base_url: required_environment("GPT_BASE_URL")?,
            model_relay_token: BearerSecret::from_environment("RESTLESS_HOSTED_MODEL_RELAY_TOKEN")?,
            plane_readiness_token: BearerSecret::from_environment(
                "RESTLESS_PLANE_READINESS_TOKEN",
            )?,
            cell_readiness_token: BearerSecret::from_environment("RESTLESS_CELL_READINESS_TOKEN")?,
            activity_token: BearerSecret::from_environment("RESTLESS_ACTIVITY_TOKEN")?,
            deletion_token: BearerSecret::from_environment("RESTLESS_DELETION_TOKEN")?,
            runtime_bootstrap_token: BearerSecret::from_bytes(bootstrap_bytes)?,
            runtime_bootstrap_token_file: bootstrap_file,
        })?;
        Ok(Some(config))
    }

    fn from_values(values: HostedDeploymentValues) -> Result<Self> {
        let owner_id = non_nil_uuid("RESTLESS_ENTRY_OWNER_ID", &values.owner_id)?;
        let plane_id = non_nil_uuid("RESTLESS_ENTRY_PLANE_ID", &values.plane_id)?;
        validate_hostname(&values.hostname)?;
        let desired_revision = values
            .desired_revision
            .parse::<i64>()
            .context("RESTLESS_DESIRED_REVISION must be a positive integer")?;
        if desired_revision < 1 {
            anyhow::bail!("RESTLESS_DESIRED_REVISION must be a positive integer");
        }
        validate_immutable_image("RESTLESS_ACCOUNT_PLANE_IMAGE", &values.account_plane_image)?;
        validate_immutable_image("RESTLESS_COMPANY_IMAGE", &values.company_runtime_image)?;
        if values.account_plane_image == values.company_runtime_image {
            anyhow::bail!("account-plane and company-runtime images must be distinct artifacts");
        }
        validate_digest(
            "RESTLESS_RELEASE_MANIFEST_DIGEST",
            &values.release_manifest_digest,
        )?;
        validate_release(&values.core_release)?;
        validate_source_revision(&values.core_source_revision)?;
        let cockpit_dir = absolute_normal_path("RESTLESS_COCKPIT_DIR", &values.cockpit_dir)?;
        let runtime_bootstrap_token_file = absolute_normal_path(
            "RESTLESS_RUNTIME_BOOTSTRAP_TOKEN_FILE",
            &values.runtime_bootstrap_token_file,
        )?;
        validate_entry_urls(&values.entry_issuer, &values.entry_jwks_url)?;
        validate_model_relay_binding(
            &values.model_credential_reference,
            &values.model_relay_url,
            &values.model_api_base_url,
        )?;
        let jwks_url = reqwest::Url::parse(&values.entry_jwks_url)
            .expect("entry URL validation parsed the JWKS URL");

        let credentials = [
            &values.plane_readiness_token,
            &values.cell_readiness_token,
            &values.activity_token,
            &values.deletion_token,
            &values.runtime_bootstrap_token,
            &values.model_relay_token,
        ];
        for left in 0..credentials.len() {
            for right in (left + 1)..credentials.len() {
                if credentials[left].same_as(credentials[right]) {
                    anyhow::bail!(
                        "hosted plane readiness, cell readiness, activity, deletion, Runtime bootstrap and model relay credentials must all be distinct"
                    );
                }
            }
        }
        if values.runtime_bootstrap_token.0.len() != 43
            || URL_SAFE_NO_PAD
                .decode(&*values.runtime_bootstrap_token.0)
                .ok()
                .is_none_or(|decoded| decoded.len() != 32)
        {
            anyhow::bail!("Runtime bootstrap secret must be an exact base64url 32-byte value");
        }

        Ok(Self {
            owner_id,
            plane_id,
            hostname: values.hostname,
            desired_revision,
            account_plane_image: values.account_plane_image,
            company_runtime_image: values.company_runtime_image,
            release_manifest_digest: values.release_manifest_digest,
            core_release: values.core_release,
            core_source_revision: values.core_source_revision,
            cockpit_dir,
            jwks_url,
            model_credential_reference: values.model_credential_reference,
            plane_readiness_token: values.plane_readiness_token,
            cell_readiness_token: values.cell_readiness_token,
            activity_token: values.activity_token,
            deletion_token: values.deletion_token,
            runtime_bootstrap_token: values.runtime_bootstrap_token,
            runtime_bootstrap_token_file,
        })
    }

    fn runtime_plane_config(&self) -> std::result::Result<HostedPlaneConfig, HostedRuntimeError> {
        HostedPlaneConfig::from_values(HostedPlaneValues {
            owner_id: self.owner_id.to_string(),
            plane_id: self.plane_id.to_string(),
            hostname: self.hostname.clone(),
            runtime_image: self.company_runtime_image.clone(),
            core_source_revision: self.core_source_revision.clone(),
            bootstrap_token_file: self
                .runtime_bootstrap_token_file
                .to_string_lossy()
                .into_owned(),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaneReadinessRequest {
    contract_version: u32,
    owner_id: Uuid,
    plane_id: Uuid,
    hostname: String,
    account_plane_image: String,
    desired_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PlaneReadinessCheck {
    kind: String,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaneReadinessObservation {
    contract_version: u32,
    owner_id: Uuid,
    plane_id: Uuid,
    hostname: String,
    account_plane_image: String,
    desired_revision: i64,
    core_release: String,
    release_manifest_digest: String,
    status: String,
    ready: bool,
    checks: Vec<PlaneReadinessCheck>,
    observed_at: DateTime<Utc>,
    valid_until: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct InternalError {
    error: &'static str,
    message: &'static str,
}

#[async_trait]
trait HostedControlBackend: Send + Sync {
    async fn readiness_checks(&self) -> [bool; 6];
    async fn ensure_company(
        &self,
        request: &CompanyBootstrapRequest,
    ) -> std::result::Result<bool, HostedRuntimeError>;
    async fn exact_company_is_ready(
        &self,
        scope: &HostedCompanyScope,
    ) -> std::result::Result<bool, HostedRuntimeError>;
    async fn admit_runtime(
        &self,
        identity: &HostedRuntimeIdentity,
        desired_revision: i64,
    ) -> std::result::Result<bool, HostedRuntimeError>;
    async fn exact_runtime_is_current(&self, identity: &HostedRuntimeIdentity) -> Result<bool>;
    async fn consume_registration_grant(
        &self,
        grant: &RuntimeBridgeGrant,
    ) -> Result<RuntimeGrantConsumption>;
}

#[derive(Clone)]
struct BackendAdapter(Arc<dyn HostedControlBackend>);

#[async_trait]
impl HostedCompanyProvisioner for BackendAdapter {
    async fn ensure_company(
        &self,
        request: &CompanyBootstrapRequest,
    ) -> std::result::Result<bool, HostedRuntimeError> {
        self.0.ensure_company(request).await
    }
}

#[async_trait]
impl HostedCompanyReadiness for BackendAdapter {
    async fn exact_company_is_ready(
        &self,
        scope: &HostedCompanyScope,
    ) -> std::result::Result<bool, HostedRuntimeError> {
        self.0.exact_company_is_ready(scope).await
    }
}

#[async_trait]
impl HostedRuntimeAdmission for BackendAdapter {
    async fn admit_runtime(
        &self,
        identity: &HostedRuntimeIdentity,
        desired_revision: i64,
    ) -> std::result::Result<bool, HostedRuntimeError> {
        self.0.admit_runtime(identity, desired_revision).await
    }
}

#[async_trait]
impl RuntimeBridgeAuthority for BackendAdapter {
    async fn exact_runtime_is_current(&self, identity: &HostedRuntimeIdentity) -> Result<bool> {
        self.0.exact_runtime_is_current(identity).await
    }

    async fn consume_registration_grant(
        &self,
        grant: &RuntimeBridgeGrant,
    ) -> Result<RuntimeGrantConsumption> {
        self.0.consume_registration_grant(grant).await
    }
}

#[derive(Clone)]
struct HostedModelProxy {
    client: reqwest::Client,
    loopback_origin: String,
}

impl HostedModelProxy {
    fn new(loopback_origin: String) -> Result<Self> {
        let origin = reqwest::Url::parse(&loopback_origin)
            .context("hosted model relay loopback origin is invalid")?;
        if origin.scheme() != "http"
            || origin.host_str() != Some("127.0.0.1")
            || origin.port().is_none()
            || origin.path() != "/"
            || origin.query().is_some()
            || origin.fragment().is_some()
            || !origin.username().is_empty()
            || origin.password().is_some()
        {
            anyhow::bail!("hosted model relay must use one exact loopback origin");
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15 * 60))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .context("build hosted model relay client")?,
            loopback_origin: loopback_origin.trim_end_matches('/').to_owned(),
        })
    }
}

#[derive(Clone)]
pub(crate) struct HostedControl {
    config: Arc<HostedDeploymentConfig>,
    runtime_bootstrap: RuntimeBridgeBootstrap,
    backend: BackendAdapter,
    runtime_bridge: RuntimeBridgeRegistry,
    model_proxy: HostedModelProxy,
    daemon: Option<Arc<Daemon>>,
}

impl HostedControl {
    pub(crate) async fn open(
        config: HostedDeploymentConfig,
        daemon: Arc<Daemon>,
        model_admission: crate::model_gateway::HostedModelAdmission,
    ) -> Result<Self> {
        ensure_hosted_schema(daemon.authority.pool()).await?;
        let runtime_plane = config
            .runtime_plane_config()
            .map_err(|error| anyhow::anyhow!(error))?;
        let bootstrap_secret = BootstrapSecret::from_bytes(config.runtime_bootstrap_token.bytes())
            .map_err(|error| anyhow::anyhow!(error))?;
        let capability_key = RuntimeBridgeCapabilityKey::from_installation_key(
            &daemon.root.join(crate::capability::KEY_FILE),
        )
        .map_err(|error| anyhow::anyhow!(error))?;
        let model_proxy =
            HostedModelProxy::new(crate::model_gateway::hosted_relay_loopback_url()?)?;
        let config = Arc::new(config);
        let backend: Arc<dyn HostedControlBackend> = Arc::new(DaemonHostedBackend {
            daemon: daemon.clone(),
            config: config.clone(),
            model_admission,
            bootstrap_lock: tokio::sync::Mutex::new(()),
        });
        let backend = BackendAdapter(backend);
        let runtime_bridge =
            RuntimeBridgeRegistry::new(capability_key.clone(), Arc::new(backend.clone()));
        Ok(Self {
            config,
            runtime_bootstrap: RuntimeBridgeBootstrap::new(
                runtime_plane,
                bootstrap_secret,
                capability_key,
            ),
            backend,
            runtime_bridge,
            model_proxy,
            daemon: Some(daemon),
        })
    }

    #[cfg(test)]
    fn for_test(config: HostedDeploymentConfig, backend: Arc<dyn HostedControlBackend>) -> Self {
        Self::for_test_with_model_origin(config, backend, "http://127.0.0.1:1".into())
    }

    #[cfg(test)]
    fn for_test_with_model_origin(
        config: HostedDeploymentConfig,
        backend: Arc<dyn HostedControlBackend>,
        model_origin: String,
    ) -> Self {
        let runtime_plane = config.runtime_plane_config().unwrap();
        let bootstrap_secret =
            BootstrapSecret::from_bytes(config.runtime_bootstrap_token.bytes()).unwrap();
        let backend = BackendAdapter(backend);
        let capability_key = RuntimeBridgeCapabilityKey::from_bytes([19; 32]);
        Self {
            config: Arc::new(config),
            runtime_bootstrap: RuntimeBridgeBootstrap::new(
                runtime_plane,
                bootstrap_secret,
                capability_key.clone(),
            ),
            runtime_bridge: RuntimeBridgeRegistry::new(capability_key, Arc::new(backend.clone())),
            backend,
            model_proxy: HostedModelProxy::new(model_origin).unwrap(),
            daemon: None,
        }
    }

    pub(crate) fn runtime_transport(&self) -> RuntimeBridgeRegistry {
        self.runtime_bridge.clone()
    }
}

pub(crate) fn router(control: Arc<HostedControl>) -> Router {
    let control_routes = Router::new()
        .route(
            "/internal/v1/planes/{plane_id}/readiness",
            post(plane_readiness),
        )
        .route("/internal/v1/companies/bootstrap", post(company_bootstrap))
        .route(
            "/internal/v1/runtime-bridge/bootstrap",
            post(runtime_bridge_bootstrap),
        )
        .route("/internal/v1/runtime-bridge", get(runtime_bridge_socket))
        .route("/internal/v1/coordination", get(coordination_socket))
        .layer(DefaultBodyLimit::max(MAX_INTERNAL_BODY));
    let model_routes = Router::new()
        .route("/internal/v1/model/v1/models", get(proxy_model_catalogue))
        .route(
            "/internal/v1/model/v1/pi/stream",
            post(proxy_model_pi_stream),
        )
        .route(
            "/internal/v1/model/v1/responses",
            post(proxy_model_responses),
        )
        .layer(DefaultBodyLimit::max(MAX_MODEL_REQUEST_BODY));
    control_routes
        .merge(model_routes)
        .route("/internal/{*path}", axum::routing::any(internal_not_found))
        .layer(middleware::from_fn(harden_internal_route))
        .with_state(control)
}

async fn harden_internal_route(request: axum::extract::Request, next: Next) -> Response<Body> {
    let mut response = next.run(request).await;
    harden_internal_response(&mut response);
    response
}

async fn proxy_model_catalogue(
    State(control): State<Arc<HostedControl>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    if !body.is_empty() {
        return internal_error(
            StatusCode::BAD_REQUEST,
            "model_relay_invalid",
            "the model catalogue request body must be empty",
        );
    }
    proxy_model_request(control, uri, headers, Method::GET, "/v1/models", body).await
}

async fn proxy_model_pi_stream(
    State(control): State<Arc<HostedControl>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    proxy_model_request(control, uri, headers, Method::POST, "/v1/pi/stream", body).await
}

async fn proxy_model_responses(
    State(control): State<Arc<HostedControl>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    proxy_model_request(control, uri, headers, Method::POST, "/v1/responses", body).await
}

async fn proxy_model_request(
    control: Arc<HostedControl>,
    uri: axum::http::Uri,
    headers: HeaderMap,
    method: Method,
    upstream_path: &'static str,
    body: Bytes,
) -> Response<Body> {
    if uri.query().is_some() {
        return internal_error(
            StatusCode::BAD_REQUEST,
            "query_not_allowed",
            "model relay authority must not appear in the URL",
        );
    }
    let mut upstream = control.model_proxy.client.request(
        method,
        format!("{}{upstream_path}", control.model_proxy.loopback_origin),
    );
    for name in [AUTHORIZATION, ACCEPT, CONTENT_TYPE] {
        let mut values = headers.get_all(&name).iter();
        if let Some(value) = values.next() {
            if values.next().is_some() {
                return internal_error(
                    StatusCode::BAD_REQUEST,
                    "model_relay_invalid",
                    "model relay headers must be singular",
                );
            }
            upstream = upstream.header(name.as_str(), value.as_bytes());
        }
    }
    let upstream = match upstream.body(body).send().await {
        Ok(response) => response,
        Err(_) => {
            return internal_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "model_relay_unavailable",
                "the admitted model relay is unavailable",
            );
        }
    };
    let status = upstream.status();
    let content_type = upstream.headers().get(CONTENT_TYPE).cloned();
    let mut response = Response::builder().status(status);
    if let Some(content_type) = content_type {
        response = response.header(CONTENT_TYPE, content_type);
    }
    response
        .body(Body::from_stream(upstream.bytes_stream()))
        .unwrap_or_else(|_| {
            internal_error(
                StatusCode::BAD_GATEWAY,
                "model_relay_invalid_response",
                "the admitted model relay returned an invalid response",
            )
        })
}

async fn plane_readiness(
    State(control): State<Arc<HostedControl>>,
    AxumPath(path_plane_id): AxumPath<Uuid>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    Json(request): Json<PlaneReadinessRequest>,
) -> Response<Body> {
    if uri.query().is_some() {
        return internal_error(
            StatusCode::BAD_REQUEST,
            "query_not_allowed",
            "internal credentials and identity must not appear in the URL",
        );
    }
    if !control.config.plane_readiness_token.authorizes(&headers) {
        return internal_error(
            StatusCode::UNAUTHORIZED,
            "readiness_unauthorized",
            "plane readiness authentication failed",
        );
    }
    let config = &control.config;
    if request.contract_version != CONTRACT_VERSION
        || path_plane_id != config.plane_id
        || request.owner_id != config.owner_id
        || request.plane_id != config.plane_id
        || request.hostname != config.hostname
        || request.account_plane_image != config.account_plane_image
        || request.desired_revision != config.desired_revision
    {
        return internal_error(
            StatusCode::FORBIDDEN,
            "readiness_identity_mismatch",
            "readiness request does not identify this exact deployed plane",
        );
    }
    let observed_checks = control.backend.0.readiness_checks().await;
    let checks = REQUIRED_CHECKS
        .into_iter()
        .zip(observed_checks)
        .map(|(kind, ready)| PlaneReadinessCheck {
            kind: kind.to_owned(),
            status: if ready { "ready" } else { "failed" }.to_owned(),
        })
        .collect::<Vec<_>>();
    let ready = exact_check_set_is_ready(&checks);
    let observed_at = Utc::now();
    internal_json(
        StatusCode::OK,
        &PlaneReadinessObservation {
            contract_version: CONTRACT_VERSION,
            owner_id: config.owner_id,
            plane_id: config.plane_id,
            hostname: config.hostname.clone(),
            account_plane_image: config.account_plane_image.clone(),
            desired_revision: config.desired_revision,
            core_release: config.core_release.clone(),
            release_manifest_digest: config.release_manifest_digest.clone(),
            status: if ready { "ready" } else { "degraded" }.to_owned(),
            ready,
            checks,
            observed_at,
            valid_until: observed_at + ChronoDuration::seconds(READINESS_LEASE_SECONDS),
        },
    )
}

async fn company_bootstrap(
    State(control): State<Arc<HostedControl>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    Json(request): Json<CompanyBootstrapRequest>,
) -> Response<Body> {
    if uri.query().is_some() {
        return internal_error(
            StatusCode::BAD_REQUEST,
            "query_not_allowed",
            "internal credentials and identity must not appear in the URL",
        );
    }
    match control
        .runtime_bootstrap
        .bootstrap_company(single_authorization(&headers), request, &control.backend)
        .await
    {
        Ok(response) => internal_json(StatusCode::OK, &response),
        Err(error) => hosted_runtime_error(error),
    }
}

async fn runtime_bridge_bootstrap(
    State(control): State<Arc<HostedControl>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    Json(request): Json<RuntimeBootstrapRequest>,
) -> Response<Body> {
    if uri.query().is_some() {
        return internal_error(
            StatusCode::BAD_REQUEST,
            "query_not_allowed",
            "internal credentials and identity must not appear in the URL",
        );
    }
    match control
        .runtime_bootstrap
        .issue_runtime_capability(
            single_authorization(&headers),
            request,
            &control.backend,
            Utc::now(),
        )
        .await
    {
        Ok(response) => internal_json(StatusCode::OK, &response),
        Err(error) => hosted_runtime_error(error),
    }
}

async fn runtime_bridge_socket(
    State(control): State<Arc<HostedControl>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response<Body> {
    if !runtime_upgrade_is_allowed(&uri, &headers, &control.config.hostname) {
        return internal_error(
            StatusCode::BAD_REQUEST,
            "runtime_bridge_upgrade_invalid",
            "Runtime Agent upgrades require the exact plane host and no browser or URL authority",
        );
    }
    let bridge = control.runtime_bridge.clone();
    upgrade
        .on_upgrade(move |socket| async move {
            if let Err(error) = bridge.accept_socket(socket).await {
                tracing::warn!("Runtime Agent bridge closed: {error}");
            }
        })
        .into_response()
}

async fn coordination_socket(
    State(control): State<Arc<HostedControl>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response<Body> {
    let Some(daemon) = control.daemon.clone() else {
        return internal_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "coordination_unavailable",
            "hosted Runtime coordination is unavailable",
        );
    };
    if !runtime_upgrade_is_allowed(&uri, &headers, &control.config.hostname) {
        return internal_error(
            StatusCode::BAD_REQUEST,
            "coordination_upgrade_invalid",
            "Runtime coordination upgrades require the exact plane host and no browser or URL authority",
        );
    }
    upgrade
        .max_message_size(MAX_COORDINATION_FRAME)
        .max_frame_size(MAX_COORDINATION_FRAME)
        .on_upgrade(move |socket| serve_coordination_socket(socket, daemon))
        .into_response()
}

fn runtime_upgrade_is_allowed(
    uri: &axum::http::Uri,
    headers: &HeaderMap,
    expected_host: &str,
) -> bool {
    let mut hosts = headers.get_all(HOST).iter();
    let host_is_exact = hosts
        .next()
        .is_some_and(|value| value.to_str().ok() == Some(expected_host))
        && hosts.next().is_none();
    uri.query().is_none()
        && host_is_exact
        && !headers.contains_key(ORIGIN)
        && !headers.contains_key(COOKIE)
        && !headers.contains_key(AUTHORIZATION)
        && !headers.contains_key("sec-websocket-protocol")
}

async fn serve_coordination_socket(socket: axum::extract::ws::WebSocket, daemon: Arc<Daemon>) {
    let (daemon_io, relay_io) = tokio::io::duplex(64 * 1024);
    let daemon_task = tokio::spawn(async move {
        crate::serve(daemon_io, &daemon, crate::ConnectionOrigin::RuntimeTcp).await
    });
    let (relay_read, mut relay_write) = tokio::io::split(relay_io);
    let mut responses = BufReader::new(relay_read).lines();
    let (mut sink, mut source) = socket.split();
    loop {
        tokio::select! {
            incoming = source.next() => {
                match incoming {
                    Some(Ok(axum::extract::ws::Message::Text(text)))
                        if valid_coordination_frame(text.as_str()) =>
                    {
                        if relay_write.write_all(text.as_bytes()).await.is_err()
                            || relay_write.write_all(b"\n").await.is_err()
                        {
                            break;
                        }
                    }
                    Some(Ok(axum::extract::ws::Message::Ping(payload))) => {
                        if sink.send(axum::extract::ws::Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(axum::extract::ws::Message::Pong(_))) => {}
                    Some(Ok(axum::extract::ws::Message::Close(_))) | None => break,
                    Some(Ok(_)) | Some(Err(_)) => break,
                }
            }
            response = responses.next_line() => {
                match response {
                    Ok(Some(line))
                        if valid_coordination_frame(&line) =>
                    {
                        if sink
                            .send(axum::extract::ws::Message::Text(line.into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        }
    }
    drop(relay_write);
    daemon_task.abort();
    let _ = daemon_task.await;
}

fn valid_coordination_frame(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_COORDINATION_FRAME
        && !value.contains(['\r', '\n'])
        && serde_json::from_str::<serde_json::Value>(value)
            .is_ok_and(|document| document.is_object())
}

async fn internal_not_found() -> Response<Body> {
    internal_error(
        StatusCode::NOT_FOUND,
        "internal_route_not_found",
        "the requested internal contract does not exist",
    )
}

fn single_authorization(headers: &HeaderMap) -> Option<&str> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let first = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    Some(first)
}

fn hosted_runtime_error(error: HostedRuntimeError) -> Response<Body> {
    match error {
        HostedRuntimeError::Unauthorized => internal_error(
            StatusCode::UNAUTHORIZED,
            "runtime_bootstrap_unauthorized",
            "Runtime bootstrap authentication failed",
        ),
        HostedRuntimeError::InvalidRequest(_) => internal_error(
            StatusCode::BAD_REQUEST,
            "runtime_bootstrap_invalid",
            "Runtime bootstrap request is invalid",
        ),
        HostedRuntimeError::IdentityMismatch => internal_error(
            StatusCode::FORBIDDEN,
            "runtime_bootstrap_identity_mismatch",
            "Runtime bootstrap identity does not match this plane",
        ),
        HostedRuntimeError::CompanyNotReady => internal_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "company_not_ready",
            "the exact company is not durably ready",
        ),
        _ => internal_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime_bootstrap_unavailable",
            "Runtime bootstrap is unavailable",
        ),
    }
}

fn internal_json<T: Serialize>(status: StatusCode, value: &T) -> Response<Body> {
    let mut response = (status, Json(value)).into_response();
    harden_internal_response(&mut response);
    response
}

fn internal_error(
    status: StatusCode,
    error: &'static str,
    message: &'static str,
) -> Response<Body> {
    internal_json(status, &InternalError { error, message })
}

fn harden_internal_response(response: &mut Response<Body>) {
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        axum::http::header::PRAGMA,
        HeaderValue::from_static("no-cache"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
}

fn exact_check_set_is_ready(checks: &[PlaneReadinessCheck]) -> bool {
    let kinds: HashSet<_> = checks.iter().map(|check| check.kind.as_str()).collect();
    checks.len() == REQUIRED_CHECKS.len()
        && kinds.len() == REQUIRED_CHECKS.len()
        && REQUIRED_CHECKS.iter().all(|kind| kinds.contains(kind))
        && checks.iter().all(|check| check.status == "ready")
}

struct DaemonHostedBackend {
    daemon: Arc<Daemon>,
    config: Arc<HostedDeploymentConfig>,
    model_admission: crate::model_gateway::HostedModelAdmission,
    bootstrap_lock: tokio::sync::Mutex<()>,
}

impl DaemonHostedBackend {
    async fn database_ready(&self) -> bool {
        matches!(
            sqlx::query_scalar::<_, i64>("SELECT 1::bigint")
                .fetch_one(self.daemon.authority.pool())
                .await,
            Ok(1)
        )
    }

    async fn authority_ready(&self) -> bool {
        sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass('restless_authority.records') IS NOT NULL \
             AND to_regclass('restless_authority.company_migrations') IS NOT NULL \
             AND to_regclass('restless_authority.hosted_companies') IS NOT NULL \
             AND to_regclass('restless_authority.hosted_runtime_identities') IS NOT NULL \
             AND to_regclass('restless_authority.consumed_runtime_bridge_grants') IS NOT NULL",
        )
        .fetch_one(self.daemon.authority.pool())
        .await
        .unwrap_or(false)
    }

    async fn company_directory_ready(&self) -> bool {
        let root = self.daemon.root.clone();
        let filesystem = tokio::task::spawn_blocking(move || probe_company_directory(&root))
            .await
            .unwrap_or(false);
        if !filesystem {
            return false;
        }
        let rows = sqlx::query(
            "SELECT core_company, model, reasoning_effort \
             FROM restless_authority.hosted_companies WHERE status = 'ready'",
        )
        .fetch_all(self.daemon.authority.pool())
        .await;
        let Ok(rows) = rows else {
            return false;
        };
        rows.into_iter().all(|row| {
            let company: String = row.get("core_company");
            let model: String = row.get("model");
            let reasoning: String = row.get("reasoning_effort");
            runtime::CompanyConfig::load(&self.daemon.root, &company).is_ok_and(|config| {
                config.model == model
                    && config.reasoning_effort == reasoning
                    && hosted_model_credentials(
                        &config.model,
                        &self.config.model_credential_reference,
                    )
                    .is_ok_and(|expected| hosted_model_credentials_match(&config, &expected))
            })
        })
    }

    async fn identity_handoff_ready(&self) -> bool {
        let client = match reqwest::Client::builder()
            .timeout(EXTERNAL_PROBE_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            Ok(client) => client,
            Err(_) => return false,
        };
        let mut response = match client.get(self.config.jwks_url.clone()).send().await {
            Ok(response) if response.status().is_success() => response,
            _ => return false,
        };
        if response
            .content_length()
            .is_some_and(|length| length > MAX_JWKS_BODY as u64)
        {
            return false;
        }
        let mut bytes = Vec::new();
        loop {
            let chunk = match response.chunk().await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(_) => return false,
            };
            if bytes.len().saturating_add(chunk.len()) > MAX_JWKS_BODY {
                return false;
            }
            bytes.extend_from_slice(&chunk);
        }
        valid_jwks(&bytes)
    }

    async fn cockpit_ready(&self) -> bool {
        let root = self.config.cockpit_dir.clone();
        tokio::task::spawn_blocking(move || verify_cockpit_artifact(&root))
            .await
            .unwrap_or(false)
    }

    async fn ensure_company_inner(&self, request: &CompanyBootstrapRequest) -> Result<bool> {
        let _guard = self.bootstrap_lock.lock().await;
        let company = core_company_name(request.company_id);
        let pool = self.daemon.authority.pool();
        let model_credentials =
            match hosted_model_credentials(&request.model, &self.config.model_credential_reference)
            {
                Ok(credentials) => credentials,
                Err(_) => return Ok(false),
            };

        let existing = hosted_company_row(pool, request.company_id).await?;
        if let Some(existing) = &existing {
            if !existing.matches(request, &company) {
                return Ok(false);
            }
        } else {
            if self
                .daemon
                .root
                .join("companies")
                .join(format!("{company}.toml"))
                .exists()
                || self
                    .daemon
                    .root
                    .join("archived-companies")
                    .join(format!("{company}.toml"))
                    .exists()
            {
                // Never adopt an unbound local company merely because its
                // deterministic-looking name collides with a Cloud UUID.
                return Ok(false);
            }
            sqlx::query(
                "INSERT INTO restless_authority.hosted_companies \
                 (company_id, owner_id, plane_id, cell_id, core_company, model, reasoning_effort, status) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,'provisioning')",
            )
            .bind(request.company_id)
            .bind(request.owner_id)
            .bind(request.plane_id)
            .bind(request.cell_id)
            .bind(&company)
            .bind(&request.model)
            .bind(&request.reasoning_effort)
            .execute(pool)
            .await
            .context("record hosted company identity before provisioning")?;
        }

        let path = self
            .daemon
            .root
            .join("companies")
            .join(format!("{company}.toml"));
        if path.exists() {
            let mut config = runtime::CompanyConfig::load(&self.daemon.root, &company)?;
            if config.model != request.model || config.reasoning_effort != request.reasoning_effort
            {
                return Ok(false);
            }
            if !config.credentials.iter().all(|(binding, reference)| {
                !binding.starts_with("model.inference")
                    || model_credentials.get(binding) == Some(reference)
            }) {
                return Ok(false);
            }
            if !hosted_model_credentials_match(&config, &model_credentials) {
                if existing
                    .as_ref()
                    .is_some_and(|row| row.status != "provisioning")
                {
                    return Ok(false);
                }
                config.credentials.extend(model_credentials.clone());
                runtime::CompanyConfig::save(&self.daemon.root, &config)?;
            }
        } else {
            runtime::CompanyConfig::save(
                &self.daemon.root,
                &runtime::CompanyConfig {
                    name: company.clone(),
                    mission: String::new(),
                    spend_ceiling_usd: runtime::SpendCeiling::from_micro_usd(10_000_000),
                    outcome_standard: Default::default(),
                    model: request.model.clone(),
                    worker_runtime: runtime::WorkerRuntime::Omp,
                    reasoning_effort: request.reasoning_effort.clone(),
                    model_failover: Vec::new(),
                    credentials: model_credentials,
                    approved_parties: Vec::new(),
                },
            )?;
        }

        self.daemon
            .authority
            .initialise_company(&company, &[])
            .await?;
        let org = self.daemon.orgintel.get(&company).await?;
        crate::ensure_standing_actors(&org, Some(&request.model)).await?;
        if !standing_actors_match(&org, &request.model).await {
            return Ok(false);
        }
        let config = runtime::CompanyConfig::load(&self.daemon.root, &company)?;
        if !matches!(
            self.model_admission.admit(config).await,
            crate::model_gateway::HostedAdmissionOutcome::Admitted
        ) {
            return Ok(false);
        }
        let updated = sqlx::query(
            "UPDATE restless_authority.hosted_companies \
             SET status='ready', ready_at=now(), updated_at=now() \
             WHERE company_id=$1 AND owner_id=$2 AND plane_id=$3 AND cell_id=$4 \
             AND core_company=$5 AND model=$6 AND reasoning_effort=$7",
        )
        .bind(request.company_id)
        .bind(request.owner_id)
        .bind(request.plane_id)
        .bind(request.cell_id)
        .bind(&company)
        .bind(&request.model)
        .bind(&request.reasoning_effort)
        .execute(pool)
        .await
        .context("mark exact hosted company ready")?;
        if updated.rows_affected() != 1 {
            return Ok(false);
        }
        self.exact_company_ready_inner(&HostedCompanyScope {
            owner_id: request.owner_id,
            plane_id: request.plane_id,
            company_id: request.company_id,
            cell_id: request.cell_id,
        })
        .await
    }

    async fn exact_company_state_ready_inner(&self, scope: &HostedCompanyScope) -> Result<bool> {
        let Some(row) = hosted_company_row(self.daemon.authority.pool(), scope.company_id).await?
        else {
            return Ok(false);
        };
        if row.owner_id != scope.owner_id
            || row.plane_id != scope.plane_id
            || row.cell_id != scope.cell_id
            || row.status != "ready"
            || row.core_company != core_company_name(scope.company_id)
        {
            return Ok(false);
        }
        let config = match runtime::CompanyConfig::load(&self.daemon.root, &row.core_company) {
            Ok(config)
                if config.model == row.model
                    && config.reasoning_effort == row.reasoning_effort
                    && hosted_model_credentials(
                        &config.model,
                        &self.config.model_credential_reference,
                    )
                    .is_ok_and(|expected| hosted_model_credentials_match(&config, &expected)) =>
            {
                config
            }
            _ => return Ok(false),
        };
        let org = match self.daemon.orgintel.get(&row.core_company).await {
            Ok(org) if org.is_live().await => org,
            _ => return Ok(false),
        };
        Ok(standing_actors_match(&org, &config.model).await)
    }

    async fn exact_company_ready_inner(&self, scope: &HostedCompanyScope) -> Result<bool> {
        if !self.exact_company_state_ready_inner(scope).await? {
            return Ok(false);
        }
        let Some(row) = hosted_company_row(self.daemon.authority.pool(), scope.company_id).await?
        else {
            return Ok(false);
        };
        Ok(crate::model_gateway::company_model_is_admitted(
            &row.core_company,
            &row.model,
        ))
    }

    async fn admit_runtime_inner(
        &self,
        identity: &HostedRuntimeIdentity,
        desired_revision: i64,
    ) -> Result<bool> {
        if desired_revision < 1 {
            return Ok(false);
        }
        let pool = self.daemon.authority.pool();
        let mut transaction = pool.begin().await.context("begin Runtime admission")?;
        let company = sqlx::query(
            "SELECT owner_id, plane_id, cell_id, status \
             FROM restless_authority.hosted_companies WHERE company_id=$1 FOR UPDATE",
        )
        .bind(identity.company_id)
        .fetch_optional(&mut *transaction)
        .await
        .context("lock hosted company for Runtime admission")?;
        let Some(company) = company else {
            transaction.rollback().await.ok();
            return Ok(false);
        };
        if company.get::<Uuid, _>("owner_id") != identity.owner_id
            || company.get::<Uuid, _>("plane_id") != identity.plane_id
            || company.get::<Uuid, _>("cell_id") != identity.cell_id
            || company.get::<String, _>("status") != "ready"
        {
            transaction.rollback().await.ok();
            return Ok(false);
        }

        let rows = sqlx::query(
            "SELECT owner_id, plane_id, company_id, cell_id, runtime_id, runtime_generation, \
                    runtime_image, volume_name, source_revision, desired_revision, state \
             FROM restless_authority.hosted_runtime_identities \
             WHERE company_id=$1 AND state IN ('current','pending') FOR UPDATE",
        )
        .bind(identity.company_id)
        .fetch_all(&mut *transaction)
        .await
        .context("lock current hosted Runtime identities")?;
        let mut current = None;
        let mut pending = None;
        for row in rows {
            let row = HostedRuntimeRow::from_row(&row);
            match row.state.as_str() {
                "current" => current = Some(row),
                "pending" => pending = Some(row),
                _ => {}
            }
        }

        if let Some(current) = &current {
            if identity.runtime_generation < current.identity.runtime_generation {
                transaction.rollback().await.ok();
                return Ok(false);
            }
            if identity.runtime_generation == current.identity.runtime_generation {
                if !current.matches(identity) || desired_revision < current.desired_revision {
                    transaction.rollback().await.ok();
                    return Ok(false);
                }
                sqlx::query(
                    "UPDATE restless_authority.hosted_runtime_identities \
                     SET desired_revision=$3, updated_at=now() \
                     WHERE company_id=$1 AND runtime_generation=$2 AND state='current'",
                )
                .bind(identity.company_id)
                .bind(identity.runtime_generation)
                .bind(desired_revision)
                .execute(&mut *transaction)
                .await
                .context("advance hot Runtime desired revision")?;
                transaction
                    .commit()
                    .await
                    .context("commit hot Runtime admission")?;
                return Ok(true);
            }
        }

        if let Some(pending) = &pending {
            if identity.runtime_generation < pending.identity.runtime_generation
                || (identity.runtime_generation == pending.identity.runtime_generation
                    && (!pending.matches(identity) || desired_revision < pending.desired_revision))
            {
                transaction.rollback().await.ok();
                return Ok(false);
            }
            if identity.runtime_generation == pending.identity.runtime_generation {
                sqlx::query(
                    "UPDATE restless_authority.hosted_runtime_identities \
                     SET desired_revision=$3, updated_at=now() \
                     WHERE company_id=$1 AND runtime_generation=$2 AND state='pending'",
                )
                .bind(identity.company_id)
                .bind(identity.runtime_generation)
                .bind(desired_revision)
                .execute(&mut *transaction)
                .await
                .context("advance pending Runtime desired revision")?;
                transaction
                    .commit()
                    .await
                    .context("commit pending Runtime admission")?;
                return Ok(true);
            }
            sqlx::query(
                "UPDATE restless_authority.hosted_runtime_identities \
                 SET state='superseded', updated_at=now() \
                 WHERE company_id=$1 AND state='pending'",
            )
            .bind(identity.company_id)
            .execute(&mut *transaction)
            .await
            .context("supersede older pending Runtime identity")?;
        }

        sqlx::query(
            "INSERT INTO restless_authority.hosted_runtime_identities \
             (company_id, owner_id, plane_id, cell_id, runtime_id, runtime_generation, \
              runtime_image, volume_name, source_revision, desired_revision, state) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'pending')",
        )
        .bind(identity.company_id)
        .bind(identity.owner_id)
        .bind(identity.plane_id)
        .bind(identity.cell_id)
        .bind(&identity.runtime_id)
        .bind(identity.runtime_generation)
        .bind(&identity.runtime_image)
        .bind(&identity.volume_name)
        .bind(&identity.source_revision)
        .bind(desired_revision)
        .execute(&mut *transaction)
        .await
        .context("stage pending hosted Runtime identity")?;
        transaction
            .commit()
            .await
            .context("commit Runtime admission")?;
        Ok(true)
    }

    async fn exact_runtime_is_current_inner(
        &self,
        identity: &HostedRuntimeIdentity,
    ) -> Result<bool> {
        let row = sqlx::query(
            "SELECT owner_id, plane_id, company_id, cell_id, runtime_id, runtime_generation, \
                    runtime_image, volume_name, source_revision, desired_revision, state \
             FROM restless_authority.hosted_runtime_identities \
             WHERE company_id=$1 AND state='current'",
        )
        .bind(identity.company_id)
        .fetch_optional(self.daemon.authority.pool())
        .await
        .context("read current hosted Runtime identity")?;
        Ok(row
            .as_ref()
            .map(HostedRuntimeRow::from_row)
            .is_some_and(|row| row.matches(identity)))
    }

    async fn consume_runtime_grant_inner(
        &self,
        grant: &RuntimeBridgeGrant,
    ) -> Result<RuntimeGrantConsumption> {
        let pool = self.daemon.authority.pool();
        let mut transaction = pool
            .begin()
            .await
            .context("begin Runtime grant consumption")?;
        let locked = sqlx::query(
            "SELECT company_id FROM restless_authority.hosted_companies \
             WHERE company_id=$1 FOR UPDATE",
        )
        .bind(grant.identity.company_id)
        .fetch_optional(&mut *transaction)
        .await
        .context("lock hosted company for Runtime registration")?;
        if locked.is_none() {
            transaction.rollback().await.ok();
            return Ok(RuntimeGrantConsumption::IdentityMismatch);
        }
        let row = sqlx::query(
            "SELECT owner_id, plane_id, company_id, cell_id, runtime_id, runtime_generation, \
                    runtime_image, volume_name, source_revision, desired_revision, state \
             FROM restless_authority.hosted_runtime_identities \
             WHERE company_id=$1 AND runtime_generation=$2 AND state IN ('current','pending') \
             FOR UPDATE",
        )
        .bind(grant.identity.company_id)
        .bind(grant.identity.runtime_generation)
        .fetch_optional(&mut *transaction)
        .await
        .context("lock admitted Runtime identity")?;
        let Some(row) = row else {
            transaction.rollback().await.ok();
            return Ok(RuntimeGrantConsumption::IdentityMismatch);
        };
        let row = HostedRuntimeRow::from_row(&row);
        if !row.matches(&grant.identity) {
            transaction.rollback().await.ok();
            return Ok(RuntimeGrantConsumption::IdentityMismatch);
        }
        let inserted = sqlx::query(
            "INSERT INTO restless_authority.consumed_runtime_bridge_grants \
             (jti, company_id, cell_id, runtime_generation, expires_at) \
             VALUES ($1,$2,$3,$4,$5) ON CONFLICT (jti) DO NOTHING",
        )
        .bind(grant.jti)
        .bind(grant.identity.company_id)
        .bind(grant.identity.cell_id)
        .bind(grant.identity.runtime_generation)
        .bind(grant.expires_at)
        .execute(&mut *transaction)
        .await
        .context("consume Runtime bridge grant")?;
        if inserted.rows_affected() != 1 {
            transaction.rollback().await.ok();
            return Ok(RuntimeGrantConsumption::Replayed);
        }
        if row.state == "pending" {
            sqlx::query(
                "UPDATE restless_authority.hosted_runtime_identities \
                 SET state='superseded', updated_at=now() \
                 WHERE company_id=$1 AND state='current'",
            )
            .bind(grant.identity.company_id)
            .execute(&mut *transaction)
            .await
            .context("supersede prior current Runtime identity")?;
            let promoted = sqlx::query(
                "UPDATE restless_authority.hosted_runtime_identities \
                 SET state='current', updated_at=now() \
                 WHERE company_id=$1 AND runtime_generation=$2 AND state='pending'",
            )
            .bind(grant.identity.company_id)
            .bind(grant.identity.runtime_generation)
            .execute(&mut *transaction)
            .await
            .context("promote connected Runtime identity")?;
            if promoted.rows_affected() != 1 {
                transaction.rollback().await.ok();
                return Ok(RuntimeGrantConsumption::IdentityMismatch);
            }
        }
        sqlx::query(
            "DELETE FROM restless_authority.consumed_runtime_bridge_grants \
             WHERE expires_at < now() - interval '1 hour'",
        )
        .execute(&mut *transaction)
        .await
        .context("expire old Runtime bridge replay records")?;
        transaction
            .commit()
            .await
            .context("commit Runtime grant consumption")?;
        Ok(RuntimeGrantConsumption::Accepted)
    }
}

struct HostedRuntimeRow {
    identity: HostedRuntimeIdentity,
    desired_revision: i64,
    state: String,
}

impl HostedRuntimeRow {
    fn from_row(row: &sqlx::postgres::PgRow) -> Self {
        Self {
            identity: HostedRuntimeIdentity {
                owner_id: row.get("owner_id"),
                plane_id: row.get("plane_id"),
                company_id: row.get("company_id"),
                cell_id: row.get("cell_id"),
                runtime_id: row.get("runtime_id"),
                runtime_generation: row.get("runtime_generation"),
                runtime_image: row.get("runtime_image"),
                volume_name: row.get("volume_name"),
                source_revision: row.get("source_revision"),
            },
            desired_revision: row.get("desired_revision"),
            state: row.get("state"),
        }
    }

    fn matches(&self, identity: &HostedRuntimeIdentity) -> bool {
        &self.identity == identity
    }
}

#[async_trait]
impl HostedControlBackend for DaemonHostedBackend {
    async fn readiness_checks(&self) -> [bool; 6] {
        let (authority, credential, directory, identity, cockpit, database) = tokio::join!(
            self.authority_ready(),
            crate::credential::probe_custody(),
            self.company_directory_ready(),
            self.identity_handoff_ready(),
            self.cockpit_ready(),
            self.database_ready(),
        );
        [
            authority, credential, directory, identity, cockpit, database,
        ]
    }

    async fn ensure_company(
        &self,
        request: &CompanyBootstrapRequest,
    ) -> std::result::Result<bool, HostedRuntimeError> {
        match self.ensure_company_inner(request).await {
            Ok(ready) => Ok(ready),
            Err(_) => {
                tracing::warn!(
                    company = %request.company_id,
                    "hosted company bootstrap did not reach durable readiness"
                );
                Ok(false)
            }
        }
    }

    async fn exact_company_is_ready(
        &self,
        scope: &HostedCompanyScope,
    ) -> std::result::Result<bool, HostedRuntimeError> {
        match self.exact_company_ready_inner(scope).await {
            Ok(ready) => Ok(ready),
            Err(_) => Ok(false),
        }
    }

    async fn admit_runtime(
        &self,
        identity: &HostedRuntimeIdentity,
        desired_revision: i64,
    ) -> std::result::Result<bool, HostedRuntimeError> {
        match self.admit_runtime_inner(identity, desired_revision).await {
            Ok(admitted) => Ok(admitted),
            Err(error) => {
                tracing::warn!(
                    company = %identity.company_id,
                    generation = identity.runtime_generation,
                    "hosted Runtime admission failed: {error:#}"
                );
                Ok(false)
            }
        }
    }

    async fn exact_runtime_is_current(&self, identity: &HostedRuntimeIdentity) -> Result<bool> {
        self.exact_runtime_is_current_inner(identity).await
    }

    async fn consume_registration_grant(
        &self,
        grant: &RuntimeBridgeGrant,
    ) -> Result<RuntimeGrantConsumption> {
        self.consume_runtime_grant_inner(grant).await
    }
}

#[derive(Debug)]
struct HostedCompanyRow {
    owner_id: Uuid,
    plane_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    core_company: String,
    model: String,
    reasoning_effort: String,
    status: String,
}

impl HostedCompanyRow {
    fn matches(&self, request: &CompanyBootstrapRequest, company: &str) -> bool {
        self.owner_id == request.owner_id
            && self.plane_id == request.plane_id
            && self.company_id == request.company_id
            && self.cell_id == request.cell_id
            && self.core_company == company
            && self.model == request.model
            && self.reasoning_effort == request.reasoning_effort
            && matches!(self.status.as_str(), "provisioning" | "ready")
    }
}

async fn hosted_company_row(
    pool: &sqlx::PgPool,
    company_id: Uuid,
) -> Result<Option<HostedCompanyRow>> {
    let row = sqlx::query(
        "SELECT owner_id, plane_id, company_id, cell_id, core_company, model, reasoning_effort, status \
         FROM restless_authority.hosted_companies WHERE company_id=$1",
    )
    .bind(company_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| HostedCompanyRow {
        owner_id: row.get("owner_id"),
        plane_id: row.get("plane_id"),
        company_id: row.get("company_id"),
        cell_id: row.get("cell_id"),
        core_company: row.get("core_company"),
        model: row.get("model"),
        reasoning_effort: row.get("reasoning_effort"),
        status: row.get("status"),
    }))
}

async fn ensure_hosted_schema(pool: &sqlx::PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.hosted_companies (\
           company_id UUID PRIMARY KEY, \
           owner_id UUID NOT NULL, \
           plane_id UUID NOT NULL, \
           cell_id UUID NOT NULL UNIQUE, \
           core_company TEXT NOT NULL UNIQUE, \
           model TEXT NOT NULL, \
           reasoning_effort TEXT NOT NULL, \
           status TEXT NOT NULL CHECK (status IN ('provisioning','ready')), \
           ready_at TIMESTAMPTZ, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           CHECK (company_id <> '00000000-0000-0000-0000-000000000000'), \
           CHECK (owner_id <> '00000000-0000-0000-0000-000000000000'), \
           CHECK (plane_id <> '00000000-0000-0000-0000-000000000000'), \
           CHECK (cell_id <> '00000000-0000-0000-0000-000000000000')\
         )",
    )
    .execute(pool)
    .await
    .context("create hosted company identity registry")?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.hosted_runtime_identities (\
           company_id UUID NOT NULL REFERENCES restless_authority.hosted_companies(company_id) ON DELETE CASCADE, \
           owner_id UUID NOT NULL, plane_id UUID NOT NULL, cell_id UUID NOT NULL, \
           runtime_id TEXT NOT NULL, runtime_generation BIGINT NOT NULL CHECK (runtime_generation > 0), \
           runtime_image TEXT NOT NULL, volume_name TEXT NOT NULL, source_revision TEXT NOT NULL, \
           desired_revision BIGINT NOT NULL CHECK (desired_revision > 0), \
           state TEXT NOT NULL CHECK (state IN ('pending','current','superseded')), \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           PRIMARY KEY (company_id, runtime_generation), \
           CHECK (owner_id <> '00000000-0000-0000-0000-000000000000'), \
           CHECK (plane_id <> '00000000-0000-0000-0000-000000000000'), \
           CHECK (cell_id <> '00000000-0000-0000-0000-000000000000')\
         )",
    )
    .execute(pool)
    .await
    .context("create hosted Runtime identity registry")?;
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS hosted_runtime_one_current \
         ON restless_authority.hosted_runtime_identities(company_id) WHERE state='current'",
    )
    .execute(pool)
    .await
    .context("enforce one current hosted Runtime identity")?;
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS hosted_runtime_one_pending \
         ON restless_authority.hosted_runtime_identities(company_id) WHERE state='pending'",
    )
    .execute(pool)
    .await
    .context("enforce one pending hosted Runtime identity")?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.consumed_runtime_bridge_grants (\
           jti UUID PRIMARY KEY, company_id UUID NOT NULL, cell_id UUID NOT NULL, \
           runtime_generation BIGINT NOT NULL CHECK (runtime_generation > 0), \
           expires_at TIMESTAMPTZ NOT NULL, consumed_at TIMESTAMPTZ NOT NULL DEFAULT now()\
         )",
    )
    .execute(pool)
    .await
    .context("create durable Runtime bridge replay registry")?;
    Ok(())
}

async fn standing_actors_match(org: &restless_orgintel::OrgIntel, model: &str) -> bool {
    let (owner, exec) = tokio::join!(org.active_actor("owner"), org.active_actor("exec"));
    matches!(owner, Ok(Some(actor)) if actor.kind == "owner" && actor.role == "owner")
        && matches!(exec, Ok(Some(actor)) if actor.kind == "exec" && actor.role == "exec" && actor.model.as_deref() == Some(model))
}

fn core_company_name(company_id: Uuid) -> String {
    format!("c{}", company_id.simple())
}

fn hosted_model_credentials(model: &str, reference: &str) -> Result<BTreeMap<String, String>> {
    let Some(model_id) = model.strip_prefix("litellm/") else {
        anyhow::bail!("hosted companies may use only the Cloud model relay provider");
    };
    if !matches!(model_id, "gpt-5.6-sol" | "gpt-5.6-terra" | "gpt-5.6-luna") {
        anyhow::bail!("hosted company model is outside the pinned metered allowlist");
    }
    if reference != crate::credential::HOSTED_MODEL_RELAY_REFERENCE {
        anyhow::bail!("hosted company model reference is not the fixed Cloud relay capability");
    }
    Ok(BTreeMap::from([(
        "model.inference.litellm".to_owned(),
        reference.to_owned(),
    )]))
}

fn hosted_model_credentials_match(
    config: &runtime::CompanyConfig,
    expected: &BTreeMap<String, String>,
) -> bool {
    expected
        .iter()
        .all(|(binding, reference)| config.credentials.get(binding) == Some(reference))
        && config.credentials.iter().all(|(binding, reference)| {
            let is_model_binding =
                binding == "model.inference" || binding.starts_with("model.inference.");
            !is_model_binding || expected.get(binding) == Some(reference)
        })
}

fn probe_company_directory(root: &Path) -> bool {
    let directory = root.join("companies");
    let marker = directory.join(format!(".hosted-readiness-{}", Uuid::new_v4().simple()));
    let write_probe = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&marker)
        .and_then(|file| file.sync_all());
    if write_probe.is_err() {
        return false;
    }
    if fs::remove_file(&marker).is_err() {
        return false;
    }
    let Ok(entries) = fs::read_dir(&directory) else {
        return false;
    };
    for entry in entries {
        let Ok(entry) = entry else {
            return false;
        };
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            return false;
        };
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return false;
        }
        let Some(name) = path.file_stem().and_then(|value| value.to_str()) else {
            return false;
        };
        if runtime::CompanyConfig::load(root, name).is_err() {
            return false;
        }
    }
    true
}

#[derive(Deserialize)]
struct JwkSet {
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct Jwk {
    kty: String,
    crv: String,
    alg: String,
    #[serde(rename = "use")]
    usage: String,
    kid: String,
    x: String,
}

fn valid_jwks(bytes: &[u8]) -> bool {
    let Ok(set) = serde_json::from_slice::<JwkSet>(bytes) else {
        return false;
    };
    if set.keys.is_empty() || set.keys.len() > 16 {
        return false;
    }
    let mut kids = HashSet::new();
    set.keys.into_iter().all(|key| {
        key.kty == "OKP"
            && key.crv == "Ed25519"
            && key.alg == "EdDSA"
            && key.usage == "sig"
            && !key.kid.is_empty()
            && key.kid.len() <= 128
            && key.kid.bytes().all(|byte| byte.is_ascii_graphic())
            && kids.insert(key.kid)
            && URL_SAFE_NO_PAD
                .decode(key.x)
                .ok()
                .is_some_and(|decoded| decoded.len() == 32)
    })
}

#[derive(Deserialize)]
struct UiArtifact {
    schema: String,
    artifact_sha256: String,
    entrypoint: String,
    manifest: UiManifestIdentity,
    payload: UiPayload,
    routes: UiRoutes,
}

#[derive(Deserialize)]
struct UiManifestIdentity {
    path: String,
    excluded_from_payload: bool,
}

#[derive(Deserialize)]
struct UiPayload {
    sha256: String,
    file_count: usize,
    byte_count: u64,
    files: Vec<UiFile>,
}

#[derive(Deserialize)]
struct UiFile {
    path: String,
    size: u64,
    sha256: String,
}

#[derive(Deserialize)]
struct UiRoutes {
    format: String,
    sha256: String,
    count: usize,
    items: Vec<String>,
}

fn verify_cockpit_artifact(root: &Path) -> bool {
    let mut actual_files = BTreeSet::new();
    if !collect_artifact_files(root, Path::new(""), &mut actual_files) {
        return false;
    }
    let manifest_path = root.join("core-ui-manifest.json");
    let Ok(bytes) = fs::read(&manifest_path) else {
        return false;
    };
    let Ok(artifact) = serde_json::from_slice::<UiArtifact>(&bytes) else {
        return false;
    };
    if artifact.schema != "restless.core-ui-artifact/v1"
        || artifact.entrypoint != "index.html"
        || artifact.manifest.path != "core-ui-manifest.json"
        || !artifact.manifest.excluded_from_payload
        || artifact.payload.file_count != artifact.payload.files.len()
        || artifact.routes.count != artifact.routes.items.len()
        || artifact.routes.format != "sveltekit-url-pattern"
        || !artifact.routes.items.iter().any(|route| route == "/")
        || artifact.routes.items.iter().any(|route| {
            route.len() > 1_024
                || !route.starts_with('/')
                || route.contains(['\\', '\r', '\n', '\0'])
        })
        || !artifact
            .routes
            .items
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    {
        return false;
    }
    let mut paths = BTreeSet::new();
    let mut aggregate = Sha256::new();
    aggregate.update(b"restless.core-ui-payload/v1\0");
    let mut byte_count = 0_u64;
    for file in &artifact.payload.files {
        if !safe_relative_artifact_path(&file.path) || !paths.insert(file.path.clone()) {
            return false;
        }
        let path = root.join(&file.path);
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            return false;
        };
        if !metadata.file_type().is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() != file.size
        {
            return false;
        }
        let Ok(contents) = fs::read(path) else {
            return false;
        };
        let digest = format!("{:x}", Sha256::digest(&contents));
        if digest != file.sha256 {
            return false;
        }
        byte_count = match byte_count.checked_add(file.size) {
            Some(value) => value,
            None => return false,
        };
        aggregate.update(file.path.as_bytes());
        aggregate.update(b"\0");
        aggregate.update(file.size.to_string().as_bytes());
        aggregate.update(b"\0");
        aggregate.update(file.sha256.as_bytes());
        aggregate.update(b"\0");
    }
    let payload_digest = format!("{:x}", aggregate.finalize());
    if byte_count != artifact.payload.byte_count || payload_digest != artifact.payload.sha256 {
        return false;
    }
    let mut expected_files = paths.clone();
    expected_files.insert(artifact.manifest.path.clone());
    if actual_files != expected_files {
        return false;
    }
    let mut route_digest = Sha256::new();
    route_digest.update(b"restless.core-ui-routes/v1\0");
    for route in &artifact.routes.items {
        route_digest.update(route.as_bytes());
        route_digest.update(b"\0");
    }
    if format!("{:x}", route_digest.finalize()) != artifact.routes.sha256 {
        return false;
    }
    let mut artifact_digest = Sha256::new();
    artifact_digest
        .update(format!("{}\0{}\0", artifact.schema, artifact.payload.sha256).as_bytes());
    for route in &artifact.routes.items {
        artifact_digest.update(route.as_bytes());
        artifact_digest.update(b"\0");
    }
    format!("{:x}", artifact_digest.finalize()) == artifact.artifact_sha256
        && paths.contains(&artifact.entrypoint)
}

fn collect_artifact_files(root: &Path, relative: &Path, files: &mut BTreeSet<String>) -> bool {
    let directory = root.join(relative);
    let Ok(metadata) = fs::symlink_metadata(&directory) else {
        return false;
    };
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return false;
    }
    let Ok(entries) = fs::read_dir(&directory) else {
        return false;
    };
    for entry in entries {
        let Ok(entry) = entry else {
            return false;
        };
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            return false;
        };
        let child_relative = relative.join(name);
        let Ok(child_metadata) = fs::symlink_metadata(root.join(&child_relative)) else {
            return false;
        };
        if child_metadata.file_type().is_symlink() {
            return false;
        }
        if child_metadata.file_type().is_dir() {
            if !collect_artifact_files(root, &child_relative, files) {
                return false;
            }
        } else if child_metadata.file_type().is_file() {
            let Some(path) = child_relative.to_str() else {
                return false;
            };
            if !files.insert(path.replace(std::path::MAIN_SEPARATOR, "/")) {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn safe_relative_artifact_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn required_environment(name: &'static str) -> Result<String> {
    let value = std::env::var_os(name)
        .with_context(|| format!("{name} is required in network entry mode"))?
        .into_string()
        .map_err(|_| anyhow::anyhow!("{name} must be valid UTF-8"))?;
    if value.is_empty() || value.trim() != value || value.contains(['\r', '\n', '\0']) {
        anyhow::bail!("{name} must be one exact bounded value");
    }
    Ok(value)
}

fn non_nil_uuid(label: &'static str, value: &str) -> Result<Uuid> {
    let parsed = Uuid::parse_str(value).with_context(|| format!("{label} must be a UUID"))?;
    if parsed.is_nil() {
        anyhow::bail!("{label} must not be the nil UUID");
    }
    Ok(parsed)
}

fn validate_hostname(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 253
        || value.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
    {
        anyhow::bail!("RESTLESS_ENTRY_HOST must be a bounded lowercase DNS hostname");
    }
    Ok(())
}

fn validate_immutable_image(label: &'static str, value: &str) -> Result<()> {
    let Some((repository, digest)) = value.rsplit_once("@sha256:") else {
        anyhow::bail!("{label} must be an immutable sha256 OCI reference");
    };
    if repository.is_empty()
        || repository.contains(char::is_whitespace)
        || digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        anyhow::bail!("{label} must be an immutable sha256 OCI reference");
    }
    Ok(())
}

fn validate_digest(label: &'static str, value: &str) -> Result<()> {
    let Some(digest) = value.strip_prefix("sha256:") else {
        anyhow::bail!("{label} must be a sha256 digest");
    };
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        anyhow::bail!("{label} must be a sha256 digest");
    }
    Ok(())
}

fn validate_release(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
    {
        anyhow::bail!("Core release must be one bounded release identifier");
    }
    Ok(())
}

fn validate_source_revision(value: &str) -> Result<()> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        anyhow::bail!("hosted Core must be built from an exact clean 40-character source revision");
    }
    Ok(())
}

fn absolute_normal_path(label: &'static str, value: &str) -> Result<PathBuf> {
    let path = PathBuf::from(value);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        anyhow::bail!("{label} must be one absolute normalized path");
    }
    Ok(path)
}

fn validate_entry_urls(issuer: &str, jwks: &str) -> Result<()> {
    let issuer = reqwest::Url::parse(issuer).context("RESTLESS_ENTRY_ISSUER must be a URL")?;
    let jwks = reqwest::Url::parse(jwks).context("RESTLESS_ENTRY_JWKS_URL must be a URL")?;
    if issuer.scheme() != "https"
        || issuer.host_str().is_none()
        || !issuer.username().is_empty()
        || issuer.password().is_some()
        || issuer.path() != "/"
        || issuer.query().is_some()
        || issuer.fragment().is_some()
        || jwks.scheme() != "https"
        || jwks.origin() != issuer.origin()
        || !jwks.username().is_empty()
        || jwks.password().is_some()
        || jwks.path() != "/.well-known/jwks.json"
        || jwks.query().is_some()
        || jwks.fragment().is_some()
    {
        anyhow::bail!(
            "RESTLESS_ENTRY_JWKS_URL must be the HTTPS issuer's exact /.well-known/jwks.json"
        );
    }
    Ok(())
}

fn validate_model_relay_binding(
    reference: &str,
    relay_origin: &str,
    model_api_base: &str,
) -> Result<()> {
    if reference != crate::credential::HOSTED_MODEL_RELAY_REFERENCE {
        anyhow::bail!("RESTLESS_HOSTED_MODEL_CREDENTIAL_REFERENCE must be hosted-model-relay:v1");
    }
    let relay = reqwest::Url::parse(relay_origin)
        .context("RESTLESS_HOSTED_MODEL_RELAY_URL must be a URL")?;
    if relay.scheme() != "https"
        || relay.host_str().is_none()
        || !relay.username().is_empty()
        || relay.password().is_some()
        || relay.path() != "/"
        || relay.query().is_some()
        || relay.fragment().is_some()
        || relay.as_str().trim_end_matches('/') != relay_origin
        || model_api_base != format!("{relay_origin}/v1")
    {
        anyhow::bail!("GPT_BASE_URL must be the fixed HTTPS hosted model relay origin plus /v1");
    }
    Ok(())
}

fn read_bootstrap_secret_file(path: &Path) -> Result<Vec<u8>> {
    if !path.is_absolute() {
        anyhow::bail!("RESTLESS_RUNTIME_BOOTSTRAP_TOKEN_FILE must be an absolute path");
    }
    let link_metadata = fs::symlink_metadata(path)
        .context("Runtime bootstrap secret file is unavailable or unsafe")?;
    if !link_metadata.file_type().is_file() || link_metadata.file_type().is_symlink() {
        anyhow::bail!("Runtime bootstrap secret file is unavailable or unsafe");
    }
    #[cfg(unix)]
    {
        if !secret_file_access_is_restricted(path, &link_metadata) {
            anyhow::bail!("Runtime bootstrap secret file is unavailable or unsafe");
        }
    }
    if link_metadata.len() != 43 {
        anyhow::bail!("Runtime bootstrap secret file is unavailable or unsafe");
    }
    let file = fs::File::open(path).context("Runtime bootstrap secret file is unavailable")?;
    let metadata = file
        .metadata()
        .context("Runtime bootstrap secret file is unavailable")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if metadata.dev() != link_metadata.dev() || metadata.ino() != link_metadata.ino() {
            anyhow::bail!("Runtime bootstrap secret file changed while it was opened");
        }
    }
    let mut bytes = Vec::with_capacity(43);
    file.take(44)
        .read_to_end(&mut bytes)
        .context("Runtime bootstrap secret file is unavailable")?;
    if bytes.len() != 43
        || URL_SAFE_NO_PAD
            .decode(&bytes)
            .ok()
            .is_none_or(|decoded| decoded.len() != 32)
    {
        anyhow::bail!("Runtime bootstrap secret file is unavailable or unsafe");
    }
    Ok(bytes)
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    let length = left.len().max(right.len());
    let mut difference = left.len() ^ right.len();
    for index in 0..length {
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

    use axum::http::{header::CONTENT_TYPE, Method, Request};
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;

    use super::*;

    const OWNER: &str = "018f47de-5708-7c87-90f8-f0a9a9fb2f31";
    const PLANE: &str = "018f47df-8fc7-72ea-9675-4363f793bb39";
    const COMPANY: &str = "018f47e0-4e21-7a92-bb31-b1f24221c13c";
    const CELL: &str = "018f47e1-12e3-7d7d-a946-2439690b3e42";
    const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";
    const PLANE_IMAGE: &str = "ghcr.io/restless/core-plane@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const RUNTIME_IMAGE: &str = "ghcr.io/restless/core-runtime@sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const MANIFEST: &str =
        "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn encoded_secret(seed: u8) -> String {
        URL_SAFE_NO_PAD.encode([seed; 32])
    }

    fn secret(seed: u8) -> BearerSecret {
        BearerSecret::from_bytes(encoded_secret(seed).into_bytes()).unwrap()
    }

    #[test]
    fn runtime_websockets_require_exact_non_browser_authority() {
        let uri: axum::http::Uri = "/internal/v1/runtime-bridge".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("owner.core.example.test"));
        assert!(runtime_upgrade_is_allowed(
            &uri,
            &headers,
            "owner.core.example.test"
        ));

        for (name, value) in [
            (ORIGIN, "https://owner.core.example.test"),
            (COOKIE, "session=browser"),
            (AUTHORIZATION, "Bearer not-a-url-secret"),
        ] {
            let mut refused = headers.clone();
            refused.insert(name, HeaderValue::from_static(value));
            assert!(!runtime_upgrade_is_allowed(
                &uri,
                &refused,
                "owner.core.example.test"
            ));
        }
        let query: axum::http::Uri = "/internal/v1/runtime-bridge?token=no".parse().unwrap();
        assert!(!runtime_upgrade_is_allowed(
            &query,
            &headers,
            "owner.core.example.test"
        ));
        headers.insert(HOST, HeaderValue::from_static("another.example.test"));
        assert!(!runtime_upgrade_is_allowed(
            &uri,
            &headers,
            "owner.core.example.test"
        ));

        assert!(valid_coordination_frame(r#"{"cmd":"status"}"#));
        assert!(!valid_coordination_frame(""));
        assert!(!valid_coordination_frame("{}\n{}"));
        assert!(!valid_coordination_frame("not-json"));
        assert!(!valid_coordination_frame("[]"));
        assert!(!valid_coordination_frame(
            &"x".repeat(MAX_COORDINATION_FRAME + 1)
        ));
    }

    fn values() -> HostedDeploymentValues {
        HostedDeploymentValues {
            owner_id: OWNER.into(),
            plane_id: PLANE.into(),
            hostname: "owner.core.example.test".into(),
            desired_revision: "7".into(),
            account_plane_image: PLANE_IMAGE.into(),
            company_runtime_image: RUNTIME_IMAGE.into(),
            release_manifest_digest: MANIFEST.into(),
            core_release: "1.2.3".into(),
            core_source_revision: REVISION.into(),
            cockpit_dir: "/opt/restless/cockpit".into(),
            entry_issuer: "https://fleet.example.test".into(),
            entry_jwks_url: "https://fleet.example.test/.well-known/jwks.json".into(),
            model_credential_reference: crate::credential::HOSTED_MODEL_RELAY_REFERENCE.into(),
            model_relay_url: "https://models.example.test".into(),
            model_api_base_url: "https://models.example.test/v1".into(),
            model_relay_token: secret(6),
            plane_readiness_token: secret(1),
            cell_readiness_token: secret(2),
            activity_token: secret(3),
            deletion_token: secret(4),
            runtime_bootstrap_token: secret(5),
            runtime_bootstrap_token_file: "/run/secrets/runtime_bootstrap_token".into(),
        }
    }

    fn config() -> HostedDeploymentConfig {
        HostedDeploymentConfig::from_values(values()).unwrap()
    }

    #[derive(Default)]
    struct FakeBackend {
        ensure_calls: AtomicUsize,
        checks: Option<[bool; 6]>,
        company_ready: bool,
    }

    #[async_trait]
    impl HostedControlBackend for FakeBackend {
        async fn readiness_checks(&self) -> [bool; 6] {
            self.checks.unwrap_or([true; 6])
        }

        async fn ensure_company(
            &self,
            _request: &CompanyBootstrapRequest,
        ) -> std::result::Result<bool, HostedRuntimeError> {
            self.ensure_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.company_ready)
        }

        async fn exact_company_is_ready(
            &self,
            _scope: &HostedCompanyScope,
        ) -> std::result::Result<bool, HostedRuntimeError> {
            Ok(self.company_ready)
        }

        async fn admit_runtime(
            &self,
            _identity: &HostedRuntimeIdentity,
            desired_revision: i64,
        ) -> std::result::Result<bool, HostedRuntimeError> {
            Ok(self.company_ready && desired_revision > 0)
        }

        async fn exact_runtime_is_current(
            &self,
            _identity: &HostedRuntimeIdentity,
        ) -> Result<bool> {
            Ok(self.company_ready)
        }

        async fn consume_registration_grant(
            &self,
            _grant: &RuntimeBridgeGrant,
        ) -> Result<RuntimeGrantConsumption> {
            Ok(if self.company_ready {
                RuntimeGrantConsumption::Accepted
            } else {
                RuntimeGrantConsumption::IdentityMismatch
            })
        }
    }

    fn test_app(backend: Arc<FakeBackend>) -> Router {
        router(Arc::new(HostedControl::for_test(config(), backend)))
    }

    #[derive(Default)]
    struct ModelProxyProbe {
        calls: AtomicUsize,
        headers: tokio::sync::Mutex<Option<HeaderMap>>,
        body: tokio::sync::Mutex<Vec<u8>>,
    }

    async fn fake_model_upstream(
        State(probe): State<Arc<ModelProxyProbe>>,
        headers: HeaderMap,
        body: Bytes,
    ) -> Response<Body> {
        probe.calls.fetch_add(1, Ordering::SeqCst);
        *probe.headers.lock().await = Some(headers);
        *probe.body.lock().await = body.to_vec();
        let chunks = futures_util::stream::iter([
            Ok::<_, std::io::Error>(Bytes::from_static(b"event: first\n\n")),
            Ok(Bytes::from_static(b"event: second\n\n")),
        ]);
        Response::builder()
            .status(StatusCode::ACCEPTED)
            .header(CONTENT_TYPE, "text/event-stream")
            .header("x-upstream-private", "must-not-cross")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    async fn test_app_with_model_upstream(
        probe: Arc<ModelProxyProbe>,
    ) -> (Router, tokio::task::JoinHandle<()>) {
        let upstream = Router::new()
            .route("/v1/models", get(fake_model_upstream))
            .route("/v1/pi/stream", axum::routing::post(fake_model_upstream))
            .route("/v1/responses", axum::routing::post(fake_model_upstream))
            .with_state(probe);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, upstream).await.unwrap();
        });
        let backend = Arc::new(FakeBackend {
            company_ready: true,
            ..Default::default()
        });
        let control = HostedControl::for_test_with_model_origin(config(), backend, origin);
        (router(Arc::new(control)), server)
    }

    fn readiness_body() -> serde_json::Value {
        serde_json::json!({
            "contract_version": 1,
            "owner_id": OWNER,
            "plane_id": PLANE,
            "hostname": "owner.core.example.test",
            "account_plane_image": PLANE_IMAGE,
            "desired_revision": 7
        })
    }

    fn company_body() -> serde_json::Value {
        serde_json::json!({
            "contract_version": 1,
            "owner_id": OWNER,
            "plane_id": PLANE,
            "company_id": COMPANY,
            "cell_id": CELL,
            "model": "litellm/gpt-5.6-terra",
            "reasoning_effort": "high"
        })
    }

    fn runtime_body() -> serde_json::Value {
        serde_json::json!({
            "contract_version": 1,
            "owner_id": OWNER,
            "plane_id": PLANE,
            "company_id": COMPANY,
            "cell_id": CELL,
            "runtime_id": format!("restless-cell-{CELL}"),
            "runtime_generation": 3,
            "desired_revision": 7,
            "runtime_image": RUNTIME_IMAGE,
            "volume_name": format!("restless-cell-{CELL}-data"),
            "source_revision": REVISION
        })
    }

    fn post(uri: String, token: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    }

    async fn json_body(response: Response<Body>) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[test]
    fn immutable_plane_configuration_rejects_mutability_and_shared_credentials() {
        let mut mutable_plane = values();
        mutable_plane.account_plane_image = "ghcr.io/restless/core-plane:latest".into();
        assert!(HostedDeploymentConfig::from_values(mutable_plane).is_err());

        let mut mutable_runtime = values();
        mutable_runtime.company_runtime_image = "ghcr.io/restless/core-runtime:latest".into();
        assert!(HostedDeploymentConfig::from_values(mutable_runtime).is_err());

        let mut duplicate = values();
        duplicate.cell_readiness_token = duplicate.plane_readiness_token.clone();
        assert!(HostedDeploymentConfig::from_values(duplicate).is_err());

        let mut shared_model_token = values();
        shared_model_token.model_relay_token = shared_model_token.activity_token.clone();
        assert!(HostedDeploymentConfig::from_values(shared_model_token).is_err());

        let mut redirected_model = values();
        redirected_model.model_api_base_url = "https://attacker.example.test/v1".into();
        assert!(HostedDeploymentConfig::from_values(redirected_model).is_err());

        let mut arbitrary_reference = values();
        arbitrary_reference.model_credential_reference = "env:OWNER_CHOSEN_KEY".into();
        assert!(HostedDeploymentConfig::from_values(arbitrary_reference).is_err());

        let mut dirty = values();
        dirty.core_source_revision = format!("{REVISION}-dirty");
        assert!(HostedDeploymentConfig::from_values(dirty).is_err());
    }

    #[test]
    fn hosted_company_model_binding_is_cloud_relay_only() {
        let expected = hosted_model_credentials(
            "litellm/gpt-5.6-terra",
            crate::credential::HOSTED_MODEL_RELAY_REFERENCE,
        )
        .unwrap();
        assert_eq!(
            expected.get("model.inference.litellm").map(String::as_str),
            Some(crate::credential::HOSTED_MODEL_RELAY_REFERENCE)
        );
        assert!(hosted_model_credentials(
            "openai/gpt-5.6-terra",
            crate::credential::HOSTED_MODEL_RELAY_REFERENCE
        )
        .is_err());
        assert!(hosted_model_credentials(
            "litellm/unpriced-model",
            crate::credential::HOSTED_MODEL_RELAY_REFERENCE
        )
        .is_err());
    }

    #[test]
    fn hosted_model_proxy_cannot_be_redirected_beyond_explicit_loopback() {
        assert!(HostedModelProxy::new("http://127.0.0.1:7790".into()).is_ok());
        for refused in [
            "https://127.0.0.1:7790",
            "http://localhost:7790",
            "http://127.0.0.1",
            "http://127.0.0.1:7790/v1",
            "http://127.0.0.1:7790?target=provider",
            "http://user:secret@127.0.0.1:7790",
            "http://169.254.169.254:80",
        ] {
            assert!(
                HostedModelProxy::new(refused.into()).is_err(),
                "accepted unsafe model proxy origin {refused}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn runtime_bootstrap_secret_must_be_a_private_regular_file_without_newline() {
        use std::os::unix::fs::{symlink, PermissionsExt as _};

        let directory = std::env::temp_dir().join(format!(
            "restless-hosted-control-secret-{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("bootstrap");
        fs::write(&path, encoded_secret(8)).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(read_bootstrap_secret_file(&path).unwrap().len(), 43);

        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        assert!(read_bootstrap_secret_file(&path).is_err());

        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let link = directory.join("bootstrap-link");
        symlink(&path, &link).unwrap();
        assert!(read_bootstrap_secret_file(&link).is_err());

        fs::write(&path, format!("{}\n", encoded_secret(8))).unwrap();
        assert!(read_bootstrap_secret_file(&path).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn readiness_requires_its_distinct_token_and_exact_deployment_identity() {
        let backend = Arc::new(FakeBackend {
            company_ready: true,
            ..Default::default()
        });
        let app = test_app(backend);
        let uri = format!("/internal/v1/planes/{PLANE}/readiness");

        let refused = app
            .clone()
            .oneshot(post(uri.clone(), &encoded_secret(2), readiness_body()))
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            refused.headers().get("cache-control").unwrap(),
            HeaderValue::from_static("no-store")
        );

        let mut wrong = readiness_body();
        wrong["desired_revision"] = serde_json::json!(8);
        let mismatch = app
            .clone()
            .oneshot(post(uri.clone(), &encoded_secret(1), wrong))
            .await
            .unwrap();
        assert_eq!(mismatch.status(), StatusCode::FORBIDDEN);

        let query = app
            .oneshot(post(
                format!("{uri}?token=forbidden"),
                &encoded_secret(1),
                readiness_body(),
            ))
            .await
            .unwrap();
        assert_eq!(query.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn model_proxy_is_streaming_exact_path_and_header_scoped() {
        let probe = Arc::new(ModelProxyProbe::default());
        let (app, server) = test_app_with_model_upstream(probe.clone()).await;
        let payload = br#"{"model":"moonshot/kimi-k3"}"#;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/internal/v1/model/v1/pi/stream")
                    .header(AUTHORIZATION, "Bearer signed-model-capability")
                    .header(ACCEPT, "text/event-stream")
                    .header(CONTENT_TYPE, "application/json")
                    .header("cookie", "owner-session=must-not-cross")
                    .header("x-provider-key", "must-not-cross")
                    .body(Body::from(payload.as_slice()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/event-stream")
        );
        assert_eq!(
            response.headers().get("cache-control").unwrap(),
            HeaderValue::from_static("no-store")
        );
        assert!(!response.headers().contains_key("x-upstream-private"));
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body, b"event: first\n\nevent: second\n\n".as_slice());

        assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
        {
            let headers = probe.headers.lock().await;
            let headers = headers.as_ref().unwrap();
            assert_eq!(
                headers.get(AUTHORIZATION).unwrap(),
                HeaderValue::from_static("Bearer signed-model-capability")
            );
            assert_eq!(
                headers.get(ACCEPT).unwrap(),
                HeaderValue::from_static("text/event-stream")
            );
            assert_eq!(
                headers.get(CONTENT_TYPE).unwrap(),
                HeaderValue::from_static("application/json")
            );
            assert!(!headers.contains_key("cookie"));
            assert!(!headers.contains_key("x-provider-key"));
        }
        assert_eq!(*probe.body.lock().await, payload);

        let query = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/internal/v1/model/v1/models?credential=forbidden")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(query.status(), StatusCode::BAD_REQUEST);

        let unknown = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/internal/v1/model/v1/chat/completions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unknown.status(), StatusCode::NOT_FOUND);

        let mut duplicate = Request::builder()
            .method(Method::POST)
            .uri("/internal/v1/model/v1/responses")
            .header(AUTHORIZATION, "Bearer first")
            .body(Body::from("{}"))
            .unwrap();
        duplicate
            .headers_mut()
            .append(AUTHORIZATION, HeaderValue::from_static("Bearer second"));
        let duplicate = app.clone().oneshot(duplicate).await.unwrap();
        assert_eq!(duplicate.status(), StatusCode::BAD_REQUEST);

        let oversized = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/internal/v1/model/v1/responses")
                    .body(Body::from(vec![b'x'; MAX_MODEL_REQUEST_BODY + 1]))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
        server.abort();
    }

    #[tokio::test]
    async fn readiness_response_is_the_exact_fresh_six_check_cloud_contract() {
        let backend = Arc::new(FakeBackend {
            company_ready: true,
            ..Default::default()
        });
        let response = test_app(backend)
            .oneshot(post(
                format!("/internal/v1/planes/{PLANE}/readiness"),
                &encoded_secret(1),
                readiness_body(),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        let expected_keys = [
            "account_plane_image",
            "checks",
            "contract_version",
            "core_release",
            "desired_revision",
            "hostname",
            "observed_at",
            "owner_id",
            "plane_id",
            "ready",
            "release_manifest_digest",
            "status",
            "valid_until",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        assert_eq!(
            body.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            expected_keys
        );
        assert_eq!(body["ready"], true);
        assert_eq!(body["status"], "ready");
        assert_eq!(body["release_manifest_digest"], MANIFEST);
        let checks = body["checks"].as_array().unwrap();
        assert_eq!(checks.len(), 6);
        assert_eq!(
            checks
                .iter()
                .map(|check| check["kind"].as_str().unwrap())
                .collect::<Vec<_>>(),
            REQUIRED_CHECKS
        );
        assert!(checks.iter().all(|check| check["status"] == "ready"));
        let observed = DateTime::parse_from_rfc3339(body["observed_at"].as_str().unwrap()).unwrap();
        let valid = DateTime::parse_from_rfc3339(body["valid_until"].as_str().unwrap()).unwrap();
        assert_eq!((valid - observed).num_seconds(), READINESS_LEASE_SECONDS);
    }

    #[tokio::test]
    async fn readiness_degrades_when_any_real_probe_fails() {
        let backend = Arc::new(FakeBackend {
            checks: Some([true, true, true, false, true, true]),
            company_ready: true,
            ..Default::default()
        });
        let response = test_app(backend)
            .oneshot(post(
                format!("/internal/v1/planes/{PLANE}/readiness"),
                &encoded_secret(1),
                readiness_body(),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["ready"], false);
        assert_eq!(body["status"], "degraded");
        assert_eq!(body["checks"][3]["kind"], "identity_handoff");
        assert_eq!(body["checks"][3]["status"], "failed");
    }

    #[tokio::test]
    async fn company_bootstrap_and_runtime_capability_match_fleet_shapes() {
        let backend = Arc::new(FakeBackend {
            company_ready: true,
            ..Default::default()
        });
        let app = test_app(backend.clone());
        let token = encoded_secret(5);
        let company = app
            .clone()
            .oneshot(post(
                "/internal/v1/companies/bootstrap".into(),
                &token,
                company_body(),
            ))
            .await
            .unwrap();
        assert_eq!(company.status(), StatusCode::OK);
        let company = json_body(company).await;
        assert_eq!(
            company
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            [
                "cell_id",
                "company_id",
                "contract_version",
                "model",
                "owner_id",
                "plane_id",
                "reasoning_effort",
                "status",
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(company["status"], "ready");
        assert_eq!(backend.ensure_calls.load(Ordering::SeqCst), 1);

        let runtime = app
            .oneshot(post(
                "/internal/v1/runtime-bridge/bootstrap".into(),
                &token,
                runtime_body(),
            ))
            .await
            .unwrap();
        assert_eq!(runtime.status(), StatusCode::OK);
        let runtime = json_body(runtime).await;
        assert_eq!(
            runtime
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            [
                "capability",
                "cell_id",
                "company_id",
                "contract_version",
                "desired_revision",
                "runtime_generation",
                "valid_for_seconds",
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(runtime["runtime_generation"], 3);
        assert_eq!(runtime["desired_revision"], 7);
        assert_eq!(runtime["valid_for_seconds"], 900);
        assert!(!runtime["capability"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn bootstrap_refuses_wrong_token_identity_and_mutable_runtime() {
        let backend = Arc::new(FakeBackend {
            company_ready: true,
            ..Default::default()
        });
        let app = test_app(backend.clone());
        let refused = app
            .clone()
            .oneshot(post(
                "/internal/v1/companies/bootstrap".into(),
                &encoded_secret(1),
                company_body(),
            ))
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(backend.ensure_calls.load(Ordering::SeqCst), 0);

        let mut wrong_plane = company_body();
        wrong_plane["plane_id"] = serde_json::json!(Uuid::new_v4());
        let mismatch = app
            .clone()
            .oneshot(post(
                "/internal/v1/companies/bootstrap".into(),
                &encoded_secret(5),
                wrong_plane,
            ))
            .await
            .unwrap();
        assert_eq!(mismatch.status(), StatusCode::FORBIDDEN);

        let mut mutable = runtime_body();
        mutable["runtime_image"] = serde_json::json!("restless-runtime:latest");
        let mutable = app
            .oneshot(post(
                "/internal/v1/runtime-bridge/bootstrap".into(),
                &encoded_secret(5),
                mutable,
            ))
            .await
            .unwrap();
        assert_eq!(mutable.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn jwks_probe_requires_unique_ed25519_signing_keys() {
        let x = URL_SAFE_NO_PAD.encode([7; 32]);
        assert!(valid_jwks(
            serde_json::json!({
                "keys": [{
                    "kty": "OKP",
                    "crv": "Ed25519",
                    "alg": "EdDSA",
                    "use": "sig",
                    "kid": "fleet-2026-09",
                    "x": x
                }]
            })
            .to_string()
            .as_bytes()
        ));
        assert!(!valid_jwks(br#"{"keys":[]}"#));
        assert!(!valid_jwks(
            serde_json::json!({
                "keys": [{
                    "kty": "RSA",
                    "crv": "Ed25519",
                    "alg": "EdDSA",
                    "use": "sig",
                    "kid": "wrong",
                    "x": URL_SAFE_NO_PAD.encode([7; 32])
                }]
            })
            .to_string()
            .as_bytes()
        ));
    }

    #[test]
    fn cockpit_probe_covers_the_exact_artifact_inventory() {
        let directory = std::env::temp_dir().join(format!(
            "restless-hosted-control-cockpit-{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir(&directory).unwrap();
        let contents = b"canonical core cockpit";
        fs::write(directory.join("index.html"), contents).unwrap();

        let file_digest = format!("{:x}", Sha256::digest(contents));
        let mut payload_digest = Sha256::new();
        payload_digest.update(b"restless.core-ui-payload/v1\0");
        payload_digest.update(b"index.html\0");
        payload_digest.update(contents.len().to_string().as_bytes());
        payload_digest.update(b"\0");
        payload_digest.update(file_digest.as_bytes());
        payload_digest.update(b"\0");
        let payload_digest = format!("{:x}", payload_digest.finalize());

        let routes = ["/", "/[companyId]/work"];
        let mut route_digest = Sha256::new();
        route_digest.update(b"restless.core-ui-routes/v1\0");
        for route in routes {
            route_digest.update(route.as_bytes());
            route_digest.update(b"\0");
        }
        let route_digest = format!("{:x}", route_digest.finalize());

        let mut artifact_digest = Sha256::new();
        artifact_digest
            .update(format!("restless.core-ui-artifact/v1\0{payload_digest}\0").as_bytes());
        for route in routes {
            artifact_digest.update(route.as_bytes());
            artifact_digest.update(b"\0");
        }
        let artifact_digest = format!("{:x}", artifact_digest.finalize());
        let manifest = serde_json::json!({
            "schema": "restless.core-ui-artifact/v1",
            "artifact_sha256": artifact_digest,
            "entrypoint": "index.html",
            "manifest": {
                "path": "core-ui-manifest.json",
                "excluded_from_payload": true
            },
            "payload": {
                "sha256": payload_digest,
                "file_count": 1,
                "byte_count": contents.len(),
                "files": [{
                    "path": "index.html",
                    "size": contents.len(),
                    "sha256": file_digest
                }]
            },
            "routes": {
                "format": "sveltekit-url-pattern",
                "sha256": route_digest,
                "count": routes.len(),
                "items": routes
            }
        });
        fs::write(
            directory.join("core-ui-manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify_cockpit_artifact(&directory));

        fs::write(directory.join("unlisted.js"), b"not in the release").unwrap();
        assert!(!verify_cockpit_artifact(&directory));
        fs::remove_dir_all(directory).unwrap();
    }
}
