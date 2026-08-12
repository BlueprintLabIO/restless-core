use std::{
    collections::BTreeMap,
    fmt,
    fs::{self, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::{StreamExt as _, stream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    GatewayError, GatewayResult, PurposeTokenClaims, PurposeTokenCodec, SecretBytes, SpendStore,
    UsageStore,
    spend::{CeilingMap, ModelRate, SpendRecord, TokenUsage, parse_token_usage},
};

/// Response-tail buffer ceiling for usage extraction (Sprint 01 T2). The
/// `response.completed` SSE event is the final event; 256 KiB of tail covers
/// it without buffering the whole body.
const USAGE_TAIL_BYTES: usize = 256 * 1024;

const MAXIMUM_TOKEN_HEADER_BYTES: usize = 64 * 1024;
const PURPOSE_REQUEST_HEADER: &str = "authorization";
const PROVIDER_AUTHORIZATION: &str = "authorization";
const RECEIPT_HEADER: &str = "x-company-gateway-request-id";

/// Non-secret, attributable gateway audit event.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuditEvent {
    pub request_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub kind: AuditEventKind,
    pub token_id: Option<Uuid>,
    pub company_id: Option<String>,
    pub actor_id: Option<String>,
    pub execution_id: Option<Uuid>,
    pub path: String,
    /// Model identity presented by the governed adapter and authorised by the
    /// purpose token. Kept as `model` for audit-spool compatibility.
    pub model: Option<String>,
    /// Exact model identity sent to the fixed upstream after applying an
    /// explicit installation-owned route. Equal to `model` for passthrough.
    #[serde(default)]
    pub upstream_model: Option<String>,
    pub byte_count: Option<u64>,
    pub upstream_status: Option<u16>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AuditEventKind {
    Admitted,
    Rejected,
    UpstreamResponded,
    StreamCompleted,
    StreamFailed,
    StreamLimitExceeded,
}

#[async_trait::async_trait]
pub trait AuditSink: Send + Sync + fmt::Debug + 'static {
    /// Whether records survive process restart before `record` returns.
    /// Production gateway construction requires this.
    fn is_durable(&self) -> bool {
        false
    }

    async fn record(&self, event: AuditEvent) -> GatewayResult<()>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoopAuditSink;

#[async_trait::async_trait]
impl AuditSink for NoopAuditSink {
    async fn record(&self, _event: AuditEvent) -> GatewayResult<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct MemoryAuditSink {
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl MemoryAuditSink {
    #[must_use]
    pub fn events(&self) -> Vec<AuditEvent> {
        self.events
            .lock()
            .map_or_else(|_| Vec::new(), |events| events.clone())
    }
}

#[async_trait::async_trait]
impl AuditSink for MemoryAuditSink {
    async fn record(&self, event: AuditEvent) -> GatewayResult<()> {
        self.events
            .lock()
            .map_err(|_| GatewayError::Upstream)?
            .push(event);
        Ok(())
    }
}

/// Crash-durable receipt spool. Each non-secret event is a create-new JSON
/// object that companyd can ingest through its typed command boundary.
#[derive(Clone)]
pub struct FileAuditSink {
    root: PathBuf,
}

impl fmt::Debug for FileAuditSink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileAuditSink")
            .field("root", &"[CONFIGURED]")
            .finish()
    }
}

impl FileAuditSink {
    /// Open an existing owner-private, real receipt directory.
    ///
    /// # Errors
    ///
    /// Returns an error for a relative, linked, non-directory, or group/world
    /// accessible path.
    pub fn new(root: &Path) -> GatewayResult<Self> {
        Ok(Self {
            root: validate_audit_root(root)?,
        })
    }
}

#[async_trait::async_trait]
impl AuditSink for FileAuditSink {
    fn is_durable(&self) -> bool {
        true
    }

    async fn record(&self, event: AuditEvent) -> GatewayResult<()> {
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || persist_audit_event(&root, &event))
            .await
            .map_err(|_| GatewayError::Upstream)?
    }
}

fn persist_audit_event(root: &Path, event: &AuditEvent) -> GatewayResult<()> {
    validate_audit_root(root)?;
    let timestamp = event
        .occurred_at
        .timestamp_nanos_opt()
        .ok_or_else(|| GatewayError::Configuration("audit timestamp is out of range".into()))?;
    let name = format!(
        "{timestamp}-{}-{}.json",
        event.request_id.simple(),
        audit_kind_name(event.kind)
    );
    let path = root.join(&name);
    let temporary_path = root.join(format!(".{name}.{}.tmp", Uuid::new_v4().simple()));
    let bytes = serde_json::to_vec(event)
        .map_err(|_| GatewayError::Configuration("serialize gateway audit event".into()))?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&temporary_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::hard_link(&temporary_path, &path)?;
        fs::File::open(root)?.sync_all()?;
        fs::remove_file(&temporary_path)?;
        fs::File::open(root)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn validate_audit_root(root: &Path) -> GatewayResult<PathBuf> {
    if !root.is_absolute() {
        return Err(GatewayError::Configuration(
            "gateway audit root must be absolute".into(),
        ));
    }
    let metadata = fs::symlink_metadata(root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(GatewayError::Configuration(
            "gateway audit root must be a real directory".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(GatewayError::Configuration(
                "gateway audit root must be owner-private".into(),
            ));
        }
    }
    root.canonicalize().map_err(GatewayError::Io)
}

const fn audit_kind_name(kind: AuditEventKind) -> &'static str {
    match kind {
        AuditEventKind::Admitted => "admitted",
        AuditEventKind::Rejected => "rejected",
        AuditEventKind::UpstreamResponded => "upstream-responded",
        AuditEventKind::StreamCompleted => "stream-completed",
        AuditEventKind::StreamFailed => "stream-failed",
        AuditEventKind::StreamLimitExceeded => "stream-limit-exceeded",
    }
}

/// Fixed provider boundary. The upstream origin and key are installation
/// configuration, never selected by the sandbox or purpose token.
#[derive(Clone)]
pub struct GatewayConfig {
    pub upstream_origin: Url,
    /// Fixed path inserted before the client-facing `/v1/...` route. Empty by
    /// default; an upstream mounted below `/api` uses exactly `/api`.
    pub upstream_path_prefix: String,
    /// Explicit adapter-model to upstream-model routes. Absence means exact
    /// passthrough; the gateway never guesses provider prefixes or aliases.
    pub model_routes: BTreeMap<String, String>,
    /// Per-call output-token ceiling, enforced on `max_output_tokens` in the
    /// request body — clamped when over, SET when absent (providers
    /// pre-authorize the model maximum, e.g. 64000, when the field is
    /// missing, and a key with limited credit 402s every call). The cap is
    /// the same class of transform as the model route — bounded,
    /// deterministic, installation-owned spend policy (T2).
    pub max_output_tokens_cap: u64,
    /// Rate table keyed by upstream model identity (post-route). A request
    /// whose routed model has no rate is refused: if we cannot price a call
    /// we cannot bound its spend (T2 fail-closed).
    pub rates: BTreeMap<String, ModelRate>,
    pub provider_key: SecretBytes,
    pub token_codec: PurposeTokenCodec,
}

impl fmt::Debug for GatewayConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GatewayConfig")
            .field("upstream_origin", &"[CONFIGURED]")
            .field("upstream_path_prefix", &self.upstream_path_prefix)
            .field("model_route_count", &self.model_routes.len())
            .field("provider_key", &"[REDACTED]")
            .field("token_codec", &self.token_codec)
            .finish()
    }
}

impl GatewayConfig {
    /// Validate one fixed HTTPS upstream origin.
    ///
    /// # Errors
    ///
    /// Returns an error for credentials, query/fragment, or a non-origin URL.
    pub fn validate(&self) -> GatewayResult<()> {
        if self.upstream_origin.scheme() != "https"
            || self.upstream_origin.host_str().is_none()
            || !self.upstream_origin.username().is_empty()
            || self.upstream_origin.password().is_some()
            || !matches!(self.upstream_origin.path(), "" | "/")
            || self.upstream_origin.query().is_some()
            || self.upstream_origin.fragment().is_some()
        {
            return Err(GatewayError::Configuration(
                "upstream must be one credential-free HTTPS origin".into(),
            ));
        }
        validate_upstream_path_prefix(&self.upstream_path_prefix)?;
        validate_model_routes(&self.model_routes)?;
        let key = self.provider_key.expose();
        if key.len() < 16 || key.iter().any(u8::is_ascii_whitespace) {
            return Err(GatewayError::Configuration(
                "provider credential is malformed".into(),
            ));
        }
        Ok(())
    }

    fn upstream_url(&self, request_path: &str) -> Url {
        let mut upstream = self.upstream_origin.clone();
        upstream.set_path(&format!("{}{}", self.upstream_path_prefix, request_path));
        upstream
    }
}

/// Parse a comma-delimited set of exact `adapter=upstream` model routes.
/// Empty input selects byte-for-byte model passthrough.
///
/// # Errors
///
/// Returns an error for duplicates, malformed entries, or unsafe identifiers.
pub fn parse_model_routes(value: &str) -> GatewayResult<BTreeMap<String, String>> {
    if value.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut routes = BTreeMap::new();
    for entry in value.split(',') {
        let (adapter_model, upstream_model) = entry.split_once('=').ok_or_else(|| {
            GatewayError::Configuration(
                "model routes must use comma-delimited adapter=upstream entries".into(),
            )
        })?;
        if routes
            .insert(adapter_model.to_owned(), upstream_model.to_owned())
            .is_some()
        {
            return Err(GatewayError::Configuration(
                "model routes contain a duplicate adapter identity".into(),
            ));
        }
    }
    validate_model_routes(&routes)?;
    Ok(routes)
}

fn validate_upstream_path_prefix(prefix: &str) -> GatewayResult<()> {
    let valid = prefix.is_empty()
        || (prefix.starts_with('/')
            && !prefix.ends_with('/')
            && prefix.len() <= 256
            && prefix.split('/').skip(1).all(|segment| {
                !segment.is_empty()
                    && !matches!(segment, "." | "..")
                    && segment.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'~' | b'-')
                    })
            }));
    if valid {
        Ok(())
    } else {
        Err(GatewayError::Configuration(
            "upstream path prefix must be empty or one bounded absolute path without a trailing slash"
                .into(),
        ))
    }
}

fn validate_model_routes(routes: &BTreeMap<String, String>) -> GatewayResult<()> {
    if routes.len() > 64
        || routes.iter().any(|(adapter_model, upstream_model)| {
            !valid_adapter_model(adapter_model) || !valid_upstream_model(upstream_model)
        })
    {
        return Err(GatewayError::Configuration(
            "model routes contain an invalid adapter or upstream model identity".into(),
        ));
    }
    Ok(())
}

fn valid_adapter_model(model: &str) -> bool {
    !model.is_empty()
        && model.len() <= 160
        && model
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_upstream_model(model: &str) -> bool {
    !model.is_empty()
        && model.len() <= 256
        && model.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':')
        })
}

#[derive(Clone)]
pub struct GatewayState {
    config: Arc<GatewayConfig>,
    client: Client,
    usage: Arc<dyn UsageStore>,
    audit: Arc<dyn AuditSink>,
    spend: Arc<SpendStore>,
    ceilings: CeilingMap,
}

impl fmt::Debug for GatewayState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GatewayState")
            .field("config", &self.config)
            .field("usage", &self.usage)
            .finish_non_exhaustive()
    }
}

impl GatewayState {
    /// Construct a gateway with a bounded, redirect-free HTTP client.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid configuration or TLS-client setup.
    pub fn new(
        config: GatewayConfig,
        usage: Arc<dyn UsageStore>,
        audit: Arc<dyn AuditSink>,
        spend: Arc<SpendStore>,
        ceilings: CeilingMap,
    ) -> GatewayResult<Self> {
        config.validate()?;
        if !usage.is_durable() || !audit.is_durable() {
            return Err(GatewayError::Configuration(
                "production gateway requires durable usage and audit stores".into(),
            ));
        }
        Self::build(config, usage, audit, spend, ceilings, true)
    }

    fn build(
        config: GatewayConfig,
        usage: Arc<dyn UsageStore>,
        audit: Arc<dyn AuditSink>,
        spend: Arc<SpendStore>,
        ceilings: CeilingMap,
        https_only: bool,
    ) -> GatewayResult<Self> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .referer(false)
            .no_proxy()
            .https_only(https_only)
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|_| GatewayError::Configuration("build provider client".into()))?;
        Ok(Self {
            config: Arc::new(config),
            client,
            usage,
            audit,
            spend,
            ceilings,
        })
    }

    #[cfg(test)]
    fn new_for_http_fixture(
        config: GatewayConfig,
        usage: Arc<dyn UsageStore>,
        audit: Arc<dyn AuditSink>,
    ) -> GatewayResult<Self> {
        validate_upstream_path_prefix(&config.upstream_path_prefix)?;
        validate_model_routes(&config.model_routes)?;
        if config.upstream_origin.scheme() != "http"
            || config.upstream_origin.host_str().is_none()
            || !config.upstream_origin.username().is_empty()
            || config.upstream_origin.password().is_some()
            || !matches!(config.upstream_origin.path(), "" | "/")
            || config.upstream_origin.query().is_some()
            || config.upstream_origin.fragment().is_some()
        {
            return Err(GatewayError::Configuration(
                "test upstream must be one credential-free HTTP origin".into(),
            ));
        }
        let root = tempfile::tempdir().map_err(|_| GatewayError::Upstream)?;
        let spend = SpendStore::open(root.path())?;
        // Fixture requests all carry company-1 (see `claims`); seed a ceiling
        // so the spend pre-flight admits them. Ceiling refusal paths are
        // covered by a dedicated fail-closed test.
        let ceilings = crate::spend::ceiling_map();
        ceilings
            .write()
            .map_err(|_| GatewayError::Upstream)?
            .insert("company-1".to_owned(), u64::MAX);
        Self::build(config, usage, audit, Arc::new(spend), ceilings, false)
        // The tempdir must outlive the gateway under test: leak it (test-only).
        .map(|state| {
            std::mem::forget(root);
            state
        })
    }
}

pub fn router(state: GatewayState) -> Router {
    Router::new()
        .route("/v1/responses", post(proxy))
        .route("/v1/responses/compact", post(proxy))
        .fallback(reject_route)
        .with_state(state)
}

/// Every refused request is audited and logged with its method and path —
/// a rejected call is exactly the one we need to see (Sprint 01 friction:
/// the fallback used to answer without leaving a trace).
async fn reject_route(State(state): State<GatewayState>, request: Request) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    tracing::warn!(%method, %path, "model-gateway rejected request: unsupported method or path");
    if state
        .audit
        .record(AuditEvent {
            request_id: Uuid::new_v4(),
            occurred_at: Utc::now(),
            kind: AuditEventKind::Rejected,
            token_id: None,
            company_id: None,
            actor_id: None,
            execution_id: None,
            path: format!("{method} {path}"),
            model: None,
            upstream_model: None,
            byte_count: None,
            upstream_status: None,
        })
        .await
        .is_err()
    {
        tracing::error!("failed to persist rejected-route audit event");
    }
    GatewayError::InvalidRequest("unsupported method or path".into()).into_response()
}

#[allow(clippy::too_many_lines)]
async fn proxy(State(state): State<GatewayState>, request: Request) -> Response {
    let request_id = Uuid::new_v4();
    let path = request.uri().path().to_owned();
    match proxy_inner(&state, request_id, &path, request).await {
        Ok(response) => response,
        Err(error) => {
            if state
                .audit
                .record(AuditEvent {
                    request_id,
                    occurred_at: Utc::now(),
                    kind: AuditEventKind::Rejected,
                    token_id: None,
                    company_id: None,
                    actor_id: None,
                    execution_id: None,
                    path,
                    model: None,
                    upstream_model: None,
                    byte_count: None,
                    upstream_status: None,
                })
                .await
                .is_err()
            {
                tracing::error!(
                    request_id = %request_id,
                    "failed to persist rejected model-gateway request"
                );
            }
            error.into_response()
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn proxy_inner(
    state: &GatewayState,
    request_id: Uuid,
    path: &str,
    request: Request,
) -> GatewayResult<Response> {
    if request.method() != Method::POST || request.uri().query().is_some() {
        return Err(GatewayError::InvalidRequest(
            "only exact POST routes without queries are supported".into(),
        ));
    }
    let token = bearer(request.headers())?;
    let claims = state.config.token_codec.verify_at(token, Utc::now())?;
    if !claims.allowed_paths.contains(path) {
        return Err(GatewayError::Forbidden);
    }
    let _reservation = state
        .usage
        .reserve(&claims, request_id, Utc::now())
        .map_err(|error| {
            if matches!(error, GatewayError::LimitExceeded) {
                GatewayError::LimitExceeded
            } else {
                GatewayError::Upstream
            }
        })?;
    let request_limit = usize::try_from(claims.limits.maximum_request_bytes)
        .map_err(|_| GatewayError::LimitExceeded)?;
    let (parts, body) = request.into_parts();
    let body = to_bytes(body, request_limit)
        .await
        .map_err(|_| GatewayError::LimitExceeded)?;
    let routed = route_body(
        body,
        &claims,
        &state.config.model_routes,
        state.config.max_output_tokens_cap,
        request_limit,
    )?;
    let models = routed.models;
    let body = routed.body;

    // T2 spend fuse: the routed model must be priced, and the company's
    // durable counter must be below its ceiling. Both fail closed.
    if !state.config.rates.contains_key(&models.upstream) {
        return Err(GatewayError::Configuration(format!(
            "no rate table entry for upstream model {}",
            models.upstream
        )));
    }
    let ceiling = state
        .ceilings
        .read()
        .ok()
        .and_then(|map| map.get(&claims.company_id).copied())
        .ok_or(GatewayError::Forbidden)?;
    if state.spend.spent_micro_usd(&claims.company_id) >= ceiling {
        return Err(GatewayError::SpendCeilingExceeded);
    }

    state
        .audit
        .record(attributed_event(
            request_id,
            AuditEventKind::Admitted,
            &claims,
            path,
            Some(&models),
            Some(u64::try_from(body.len()).unwrap_or(u64::MAX)),
            None,
        ))
        .await
        .map_err(|_| GatewayError::Upstream)?;

    let upstream = state.config.upstream_url(path);
    let mut builder = state.client.post(upstream);
    for name in [
        header::CONTENT_TYPE,
        header::ACCEPT,
        header::USER_AGENT,
        HeaderName::from_static("openai-beta"),
        HeaderName::from_static("x-client-feature-id"),
    ] {
        if let Some(value) = parts.headers.get(&name) {
            builder = builder.header(name, value);
        }
    }
    let mut provider_authorization_bytes = b"Bearer ".to_vec();
    provider_authorization_bytes.extend_from_slice(state.config.provider_key.expose());
    let mut provider_authorization = HeaderValue::from_bytes(&provider_authorization_bytes)
        .map_err(|_| {
            GatewayError::Configuration("provider credential is not a header value".into())
        })?;
    provider_authorization.set_sensitive(true);
    provider_authorization_bytes.fill(0);
    let remaining = (claims.expires_at - Utc::now())
        .to_std()
        .map_err(|_| GatewayError::Forbidden)?;
    let upstream_response = builder
        .header(PROVIDER_AUTHORIZATION, provider_authorization)
        .timeout(remaining)
        .body(body)
        .send()
        .await
        .map_err(|_| GatewayError::Upstream)?;
    let status = upstream_response.status();
    let headers = upstream_response.headers().clone();
    state
        .audit
        .record(attributed_event(
            request_id,
            AuditEventKind::UpstreamResponded,
            &claims,
            path,
            Some(&models),
            None,
            Some(status.as_u16()),
        ))
        .await
        .map_err(|_| GatewayError::Upstream)?;

    let response_limit = claims.limits.maximum_response_bytes;
    let audit = Arc::clone(&state.audit);
    let spend = Arc::clone(&state.spend);
    let rate = state.config.rates.get(&models.upstream).copied();
    let stream_claims = claims.clone();
    let stream_path = path.to_owned();
    let upstream_stream = upstream_response.bytes_stream();
    let guarded = stream::unfold(
        (upstream_stream, 0_u64, Vec::new(), false),
        move |(mut upstream, consumed, mut tail, finished)| {
            let audit = Arc::clone(&audit);
            let spend = Arc::clone(&spend);
            let claims = stream_claims.clone();
            let path = stream_path.clone();
            let models = models.clone();
            async move {
                if finished {
                    return None;
                }
                match upstream.next().await {
                    Some(Ok(chunk)) => {
                        let next =
                            consumed.saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
                        if next > response_limit {
                            let audit_failed = audit
                                .record(attributed_event(
                                    request_id,
                                    AuditEventKind::StreamLimitExceeded,
                                    &claims,
                                    &path,
                                    Some(&models),
                                    Some(consumed),
                                    Some(status.as_u16()),
                                ))
                                .await
                                .is_err();
                            Some((
                                Err(std::io::Error::other(if audit_failed {
                                    "model response and audit limit boundary failed"
                                } else {
                                    "model response limit exceeded"
                                })),
                                (upstream, consumed, tail, true),
                            ))
                        } else {
                            tail.extend_from_slice(&chunk);
                            if tail.len() > USAGE_TAIL_BYTES {
                                let overflow = tail.len() - USAGE_TAIL_BYTES;
                                tail.drain(..overflow);
                            }
                            Some((Ok::<Bytes, std::io::Error>(chunk), (upstream, next, tail, false)))
                        }
                    }
                    Some(Err(_)) => {
                        let audit_failed = audit
                            .record(attributed_event(
                                request_id,
                                AuditEventKind::StreamFailed,
                                &claims,
                                &path,
                                Some(&models),
                                Some(consumed),
                                Some(status.as_u16()),
                            ))
                            .await
                            .is_err();
                        Some((
                            Err(std::io::Error::other(if audit_failed {
                                "model response and audit stream boundary failed"
                            } else {
                                "model provider stream failed"
                            })),
                            (upstream, consumed, tail, true),
                        ))
                    }
                    None => {
                        // T2: account the call against the rate table. Usage
                        // comes from the response tail; a miss is observable
                        // (never silently zero-cost).
                        if status.is_success() {
                            match parse_token_usage(&tail) {
                                Some(TokenUsage { input_tokens, output_tokens }) => {
                                    if let Some(rate) = rate {
                                        let record = SpendRecord {
                                            request_id,
                                            company_id: claims.company_id.clone(),
                                            model: models.upstream.clone(),
                                            input_tokens,
                                            output_tokens,
                                            cost_micro_usd: rate.cost_micro_usd(
                                                input_tokens,
                                                output_tokens,
                                            ),
                                            occurred_at: Utc::now(),
                                        };
                                        if spend.record(&record).is_err() {
                                            spend.poison(&claims.company_id);
                                            tracing::error!(
                                                request_id = %request_id,
                                                "spend record failed; company poisoned fail-closed"
                                            );
                                        }
                                    }
                                }
                                None => {
                                    tracing::warn!(
                                        request_id = %request_id,
                                        company = %claims.company_id,
                                        "upstream usage unparsed from response tail"
                                    );
                                }
                            }
                        }
                        let audit_result = audit
                            .record(attributed_event(
                                request_id,
                                AuditEventKind::StreamCompleted,
                                &claims,
                                &path,
                                Some(&models),
                                Some(consumed),
                                Some(status.as_u16()),
                            ))
                            .await;
                        audit_result.err().map(|_| {
                            (
                                Err(std::io::Error::other(
                                    "model response audit persistence failed",
                                )),
                                (upstream, consumed, tail, true),
                            )
                        })
                    }
                }
            }
        },
    );
    let mut response = Response::new(Body::from_stream(guarded));
    *response.status_mut() =
        StatusCode::from_u16(status.as_u16()).map_err(|_| GatewayError::Upstream)?;
    copy_response_headers(&headers, response.headers_mut());
    response.headers_mut().insert(
        HeaderName::from_static(RECEIPT_HEADER),
        HeaderValue::from_str(&request_id.to_string()).map_err(|_| GatewayError::Upstream)?,
    );
    Ok(response)
}

fn bearer(headers: &HeaderMap) -> GatewayResult<&str> {
    if headers.get_all(PURPOSE_REQUEST_HEADER).iter().count() != 1 {
        return Err(GatewayError::Unauthorized);
    }
    let value = headers
        .get(PURPOSE_REQUEST_HEADER)
        .ok_or(GatewayError::Unauthorized)?;
    if value.as_bytes().len() > MAXIMUM_TOKEN_HEADER_BYTES {
        return Err(GatewayError::Unauthorized);
    }
    value
        .to_str()
        .ok()
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| !token.is_empty())
        .ok_or(GatewayError::Unauthorized)
}

#[derive(Clone)]
struct RoutedModels {
    requested: String,
    upstream: String,
}

struct RoutedBody {
    models: RoutedModels,
    body: Bytes,
}

fn route_body(
    body: Bytes,
    claims: &PurposeTokenClaims,
    model_routes: &BTreeMap<String, String>,
    max_output_tokens_cap: u64,
    request_limit: usize,
) -> GatewayResult<RoutedBody> {
    #[derive(Deserialize)]
    struct ModelRequest {
        model: String,
    }
    let parsed = serde_json::from_slice::<ModelRequest>(&body)
        .map_err(|_| GatewayError::InvalidRequest("body must name an exact model".into()))?;
    if !claims.allowed_models.contains(&parsed.model) {
        return Err(GatewayError::Forbidden);
    }
    let upstream_model = model_routes.get(&parsed.model);
    let requested_cap = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| value.get("max_output_tokens")?.as_u64());
    // Absent means the provider pre-authorizes the model maximum — enforce
    // the cap whenever the field is missing or over it.
    let needs_cap = requested_cap.is_none_or(|requested| requested > max_output_tokens_cap);
    if upstream_model.is_none() && !needs_cap {
        return Ok(RoutedBody {
            models: RoutedModels {
                upstream: parsed.model.clone(),
                requested: parsed.model,
            },
            body,
        });
    }
    let mut value = serde_json::from_slice::<serde_json::Value>(&body)
        .map_err(|_| GatewayError::InvalidRequest("body must be one JSON object".into()))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| GatewayError::InvalidRequest("body must be one JSON object".into()))?;
    if let Some(upstream_model) = upstream_model {
        drop(object.insert(
            "model".into(),
            serde_json::Value::String(upstream_model.clone()),
        ));
    }
    if needs_cap {
        drop(object.insert(
            "max_output_tokens".into(),
            serde_json::Value::from(max_output_tokens_cap),
        ));
    }
    let rewritten = serde_json::to_vec(&value)
        .map_err(|_| GatewayError::InvalidRequest("body could not be routed".into()))?;
    if rewritten.len() > request_limit {
        return Err(GatewayError::LimitExceeded);
    }
    Ok(RoutedBody {
        models: RoutedModels {
            upstream: upstream_model.cloned().unwrap_or_else(|| parsed.model.clone()),
            requested: parsed.model,
        },
        body: Bytes::from(rewritten),
    })
}

fn copy_response_headers(source: &HeaderMap, destination: &mut HeaderMap) {
    for name in [
        header::CONTENT_TYPE,
        header::CACHE_CONTROL,
        HeaderName::from_static("openai-processing-ms"),
        HeaderName::from_static("x-request-id"),
    ] {
        if let Some(value) = source.get(&name) {
            destination.insert(name, value.clone());
        }
    }
}

fn attributed_event(
    request_id: Uuid,
    kind: AuditEventKind,
    claims: &PurposeTokenClaims,
    path: &str,
    models: Option<&RoutedModels>,
    byte_count: Option<u64>,
    upstream_status: Option<u16>,
) -> AuditEvent {
    AuditEvent {
        request_id,
        occurred_at: Utc::now(),
        kind,
        token_id: Some(claims.token_id),
        company_id: Some(claims.company_id.clone()),
        actor_id: Some(claims.actor_id.clone()),
        execution_id: Some(claims.execution_id),
        path: path.into(),
        model: models.map(|models| models.requested.clone()),
        upstream_model: models.map(|models| models.upstream.clone()),
        byte_count,
        upstream_status,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        sync::atomic::{AtomicBool, Ordering},
    };

    use axum::{Json, body::Body, http::Request as HttpRequest};
    use chrono::Duration;
    use tower::ServiceExt as _;

    use super::*;
    use crate::{MemoryUsageStore, PURPOSE_TOKEN_VERSION, PurposeTokenLimits, UsageStore};

    fn claims(now: DateTime<Utc>) -> PurposeTokenClaims {
        PurposeTokenClaims {
            schema_version: PURPOSE_TOKEN_VERSION,
            token_id: Uuid::new_v4(),
            company_id: "company-1".into(),
            actor_id: "actor-1".into(),
            execution_id: Uuid::new_v4(),
            audience: "model-gateway".into(),
            issued_at: now - Duration::seconds(2),
            not_before: now - Duration::seconds(1),
            expires_at: now + Duration::minutes(5),
            allowed_paths: BTreeSet::from(["/v1/responses".into()]),
            allowed_models: BTreeSet::from(["gpt-test".into()]),
            limits: PurposeTokenLimits {
                maximum_requests: 1,
                maximum_request_bytes: 256,
                maximum_response_bytes: 256,
            },
        }
    }

    /// Rate table covering every upstream model the fixtures route to.
    fn fixture_rates() -> BTreeMap<String, ModelRate> {
        BTreeMap::from([
            ("gpt-test".to_owned(), test_rate()),
            ("vendor/frontier-model:free".to_owned(), test_rate()),
        ])
    }

    fn test_rate() -> ModelRate {
        ModelRate {
            input_usd_per_mtok: 3.0,
            output_usd_per_mtok: 15.0,
        }
    }

    fn private_directory() -> tempfile::TempDir {
        let temporary = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        }
        temporary
    }

    fn audit_event(kind: AuditEventKind) -> AuditEvent {
        AuditEvent {
            request_id: Uuid::new_v4(),
            occurred_at: Utc::now(),
            kind,
            token_id: Some(Uuid::new_v4()),
            company_id: Some("company-1".into()),
            actor_id: Some("actor-1".into()),
            execution_id: Some(Uuid::new_v4()),
            path: "/v1/responses".into(),
            model: Some("gpt-test".into()),
            upstream_model: Some("gpt-test".into()),
            byte_count: Some(42),
            upstream_status: Some(200),
        }
    }

    fn audit_files(root: &Path) -> Vec<PathBuf> {
        let mut paths = fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    #[tokio::test]
    async fn file_audit_sink_persists_complete_events_across_reopen() {
        let temporary = private_directory();
        let event = audit_event(AuditEventKind::Admitted);
        FileAuditSink::new(temporary.path())
            .unwrap()
            .record(event.clone())
            .await
            .unwrap();

        let paths = audit_files(temporary.path());
        assert_eq!(paths.len(), 1);
        assert_eq!(
            paths[0].extension().and_then(|value| value.to_str()),
            Some("json")
        );
        let persisted =
            serde_json::from_slice::<AuditEvent>(&fs::read(&paths[0]).unwrap()).unwrap();
        assert_eq!(persisted, event);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                fs::metadata(&paths[0]).unwrap().permissions().mode() & 0o077,
                0
            );
        }

        let reopened = FileAuditSink::new(temporary.path()).unwrap();
        let mut completed = event;
        completed.kind = AuditEventKind::StreamCompleted;
        reopened.record(completed.clone()).await.unwrap();
        let paths = audit_files(temporary.path());
        assert_eq!(paths.len(), 2);
        assert!(
            paths
                .iter()
                .all(|path| !path.to_string_lossy().ends_with(".tmp"))
        );
        assert!(paths.iter().any(|path| {
            serde_json::from_slice::<AuditEvent>(&fs::read(path).unwrap()).unwrap() == completed
        }));
    }

    #[tokio::test]
    async fn file_audit_sink_never_overwrites_a_colliding_receipt() {
        let temporary = private_directory();
        let event = audit_event(AuditEventKind::Admitted);
        let first = FileAuditSink::new(temporary.path()).unwrap();
        first.record(event.clone()).await.unwrap();
        let path = audit_files(temporary.path()).pop().unwrap();
        let original = fs::read(&path).unwrap();

        let reopened = FileAuditSink::new(temporary.path()).unwrap();
        let error = reopened.record(event).await.unwrap_err();
        assert!(matches!(
            error,
            GatewayError::Io(ref error) if error.kind() == std::io::ErrorKind::AlreadyExists
        ));
        assert_eq!(audit_files(temporary.path()), vec![path.clone()]);
        assert_eq!(fs::read(path).unwrap(), original);
    }

    #[tokio::test]
    async fn file_audit_sink_fails_closed_if_its_root_is_replaced() {
        let temporary = private_directory();
        let root = temporary.path().to_path_buf();
        let moved = root.with_extension("moved-audit-root");
        let sink = FileAuditSink::new(&root).unwrap();
        fs::rename(&root, &moved).unwrap();
        fs::write(&root, b"not a directory").unwrap();

        let error = sink
            .record(audit_event(AuditEventKind::Admitted))
            .await
            .unwrap_err();
        assert!(matches!(error, GatewayError::Configuration(_)));
        assert_eq!(fs::read_dir(&moved).unwrap().count(), 0);

        fs::remove_file(&root).unwrap();
        fs::rename(moved, root).unwrap();
    }

    async fn fake_provider() -> (Url, tokio::task::JoinHandle<()>) {
        let app = Router::new().route(
            "/v1/responses",
            post(
                |headers: HeaderMap, Json(body): Json<serde_json::Value>| async move {
                    assert_eq!(
                        headers.get(header::AUTHORIZATION).unwrap(),
                        "Bearer provider-secret-key"
                    );
                    assert_eq!(body["model"], "gpt-test");
                    (
                        [(header::CONTENT_TYPE, "application/json")],
                        Json(serde_json::json!({"ok": true})),
                    )
                },
            ),
        );
        serve_provider(app).await
    }

    async fn prefixed_routed_provider() -> (Url, tokio::task::JoinHandle<()>) {
        let app = Router::new().route(
            "/api/v1/responses",
            post(
                |headers: HeaderMap, Json(body): Json<serde_json::Value>| async move {
                    assert_eq!(
                        headers.get(header::AUTHORIZATION).unwrap(),
                        "Bearer provider-secret-key"
                    );
                    assert_eq!(body["model"], "vendor/frontier-model:free");
                    assert_eq!(body["input"], "hello");
                    (
                        [(header::CONTENT_TYPE, "application/json")],
                        Json(serde_json::json!({"ok": true})),
                    )
                },
            ),
        );
        serve_provider(app).await
    }

    async fn redirecting_provider(
        redirected: Arc<AtomicBool>,
    ) -> (Url, tokio::task::JoinHandle<()>) {
        let catcher = Arc::clone(&redirected);
        let app = Router::new()
            .route(
                "/v1/responses",
                post(|headers: HeaderMap| async move {
                    assert_eq!(
                        headers.get(header::AUTHORIZATION).unwrap(),
                        "Bearer provider-secret-key"
                    );
                    (
                        StatusCode::TEMPORARY_REDIRECT,
                        [(header::LOCATION, "/credential-catcher")],
                    )
                }),
            )
            .route(
                "/credential-catcher",
                post(move || {
                    let catcher = Arc::clone(&catcher);
                    async move {
                        catcher.store(true, Ordering::SeqCst);
                        StatusCode::NO_CONTENT
                    }
                }),
            );
        serve_provider(app).await
    }

    async fn oversized_provider() -> (Url, tokio::task::JoinHandle<()>) {
        let app = Router::new().route(
            "/v1/responses",
            post(|| async { (StatusCode::OK, vec![b'x'; 512]) }),
        );
        serve_provider(app).await
    }

    async fn failing_stream_provider() -> (Url, tokio::task::JoinHandle<()>) {
        let app = Router::new().route(
            "/v1/responses",
            post(|| async {
                let chunks = futures::stream::unfold(0_u8, |step| async move {
                    match step {
                        0 => Some((
                            Ok::<Bytes, std::io::Error>(Bytes::from_static(b"partial")),
                            1,
                        )),
                        1 => {
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                            Some((Err(std::io::Error::other("fixture stream failure")), 2))
                        }
                        _ => None,
                    }
                });
                Response::new(Body::from_stream(chunks))
            }),
        );
        serve_provider(app).await
    }

    async fn serve_provider(app: Router) -> (Url, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (
            Url::parse(&format!("http://{address}"))
                .unwrap()
                .join("/")
                .unwrap(),
            handle,
        )
    }

    #[tokio::test]
    async fn missing_and_forged_tokens_fail_without_secret_diagnostics() {
        let codec = PurposeTokenCodec::new(SecretBytes::new(vec![3; 32]).unwrap(), "model-gateway")
            .unwrap();
        let config = GatewayConfig {
            upstream_origin: Url::parse("https://api.openai.com").unwrap(),
            upstream_path_prefix: String::new(),
            model_routes: BTreeMap::new(),
            provider_key: SecretBytes::new(b"provider-secret-key".to_vec()).unwrap(),
            token_codec: codec,
            max_output_tokens_cap: 16_384,
            rates: BTreeMap::new(),
        };
        let root = tempfile::tempdir().unwrap();
        let app = router(
            GatewayState::build(
                config,
                Arc::new(MemoryUsageStore::default()),
                Arc::new(NoopAuditSink),
                Arc::new(SpendStore::open(root.path()).unwrap()),
                crate::spend::ceiling_map(),
                true,
            )
            .unwrap(),
        );
        let response = app
            .oneshot(
                HttpRequest::post("/v1/responses")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"model":"gpt-test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), 4_096).await.unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("provider-secret-key"));
    }

    #[test]
    fn config_rejects_non_https_or_pathful_upstreams() {
        for url in [
            "http://api.openai.com",
            "https://api.openai.com/v1",
            "https://user:password@api.openai.com",
            "https://api.openai.com?credential=value",
            "https://api.openai.com#credential",
        ] {
            let config = GatewayConfig {
                upstream_origin: Url::parse(url).unwrap(),
                upstream_path_prefix: String::new(),
                model_routes: BTreeMap::new(),
                provider_key: SecretBytes::new(b"provider-secret-key".to_vec()).unwrap(),
                token_codec: PurposeTokenCodec::new(
                    SecretBytes::new(vec![3; 32]).unwrap(),
                    "model-gateway",
                )
                .unwrap(),
                max_output_tokens_cap: 16_384,
                rates: BTreeMap::new(),
            };
            assert!(config.validate().is_err());
        }
    }

    #[test]
    fn path_prefix_and_model_routes_are_explicit_and_bounded() {
        let parsed = parse_model_routes(
            "adapter-model=vendor/frontier-model:free,other-model=vendor/other-model",
        )
        .unwrap();
        assert_eq!(
            parsed.get("adapter-model").map(String::as_str),
            Some("vendor/frontier-model:free")
        );
        assert!(parse_model_routes("").unwrap().is_empty());
        for invalid in [
            "implicit-model",
            "adapter-model=",
            "adapter/model=vendor/model",
            "adapter-model=vendor/model?variant",
            "adapter-model=vendor/model,adapter-model=vendor/other",
        ] {
            assert!(parse_model_routes(invalid).is_err(), "accepted {invalid}");
        }
        for invalid in ["api", "/api/", "//api", "/api/../escape", "/api?query"] {
            assert!(validate_upstream_path_prefix(invalid).is_err());
        }
        validate_upstream_path_prefix("").unwrap();
        validate_upstream_path_prefix("/api").unwrap();
        validate_upstream_path_prefix("/gateway/api-v1").unwrap();
    }

    #[test]
    fn rewrite_is_rechecked_against_the_signed_request_limit() {
        let now = Utc::now();
        let claims = claims(now);
        let original = Bytes::from_static(br#"{"model":"gpt-test"}"#);
        let routes = BTreeMap::from([(
            "gpt-test".into(),
            "vendor/a-much-longer-upstream-model:free".into(),
        )]);
        // Route + cap insertion grows the body past the signed limit.
        assert!(matches!(
            route_body(original.clone(), &claims, &routes, 16_384, original.len()),
            Err(GatewayError::LimitExceeded)
        ));

        // A body already carrying an in-cap max_output_tokens and needing no
        // route passes through byte-for-byte.
        let capped = Bytes::from_static(br#"{"model":"gpt-test","max_output_tokens":100}"#);
        let passthrough =
            route_body(capped.clone(), &claims, &BTreeMap::new(), 16_384, capped.len()).unwrap();
        assert_eq!(passthrough.body, capped);
        assert_eq!(passthrough.models.requested, "gpt-test");
        assert_eq!(passthrough.models.upstream, "gpt-test");
    }

    #[test]
    fn max_output_tokens_is_clamped_to_the_installation_cap() {
        let now = Utc::now();
        let claims = claims(now);
        // Over the cap: rewritten down, model untouched when no route exists.
        let over = Bytes::from_static(
            br#"{"model":"gpt-test","input":"hello","max_output_tokens":64000}"#,
        );
        let clamped = route_body(over, &claims, &BTreeMap::new(), 16_384, 4_096).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&clamped.body).unwrap();
        assert_eq!(value["max_output_tokens"], 16_384);
        assert_eq!(value["model"], "gpt-test");
        assert_eq!(clamped.models.upstream, "gpt-test");

        // At or under the cap: byte-for-byte passthrough. Absent: the cap is
        // SET (an absent field means the provider pre-authorizes the model
        // maximum — that is the whole reason this transform exists).
        for body in [
            &br#"{"model":"gpt-test","max_output_tokens":16384}"#[..],
            &br#"{"model":"gpt-test","max_output_tokens":100}"#[..],
        ] {
            let routed =
                route_body(Bytes::copy_from_slice(body), &claims, &BTreeMap::new(), 16_384, 4_096)
                    .unwrap();
            assert_eq!(routed.body.as_ref(), body);
        }
        let absent = Bytes::from_static(br#"{"model":"gpt-test","input":"hello"}"#);
        let capped = route_body(absent, &claims, &BTreeMap::new(), 16_384, 4_096).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&capped.body).unwrap();
        assert_eq!(value["max_output_tokens"], 16_384);
        assert_eq!(value["model"], "gpt-test");

        // Clamp composes with an explicit model route.
        let routes = BTreeMap::from([("gpt-test".into(), "vendor/frontier-model:free".into())]);
        let over = Bytes::from_static(br#"{"model":"gpt-test","max_output_tokens":99999}"#);
        let both = route_body(over, &claims, &routes, 16_384, 4_096).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&both.body).unwrap();
        assert_eq!(value["max_output_tokens"], 16_384);
        assert_eq!(value["model"], "vendor/frontier-model:free");
    }

    #[test]
    fn production_state_rejects_ephemeral_persistence_and_redacts_config() {
        let config = GatewayConfig {
            upstream_origin: Url::parse("https://user:password@api.openai.com").unwrap(),
            upstream_path_prefix: String::new(),
            model_routes: BTreeMap::new(),
            provider_key: SecretBytes::new(b"provider-secret-key".to_vec()).unwrap(),
            token_codec: PurposeTokenCodec::new(
                SecretBytes::new(vec![3; 32]).unwrap(),
                "model-gateway",
            )
            .unwrap(),
            max_output_tokens_cap: 16_384,
            rates: BTreeMap::new(),
        };
        let debug = format!("{config:?}");
        assert!(!debug.contains("password"));
        assert!(!debug.contains("provider-secret-key"));

        let valid_config = GatewayConfig {
            upstream_origin: Url::parse("https://api.openai.com").unwrap(),
            upstream_path_prefix: String::new(),
            model_routes: BTreeMap::new(),
            provider_key: SecretBytes::new(b"provider-secret-key".to_vec()).unwrap(),
            token_codec: PurposeTokenCodec::new(
                SecretBytes::new(vec![3; 32]).unwrap(),
                "model-gateway",
            )
            .unwrap(),
            max_output_tokens_cap: 16_384,
            rates: BTreeMap::new(),
        };
        let root = tempfile::tempdir().unwrap();
        assert!(matches!(
            GatewayState::new(
                valid_config,
                Arc::new(MemoryUsageStore::default()),
                Arc::new(NoopAuditSink),
                Arc::new(SpendStore::open(root.path()).unwrap()),
                crate::spend::ceiling_map(),
            ),
            Err(GatewayError::Configuration(_))
        ));
    }

    #[test]
    fn request_counter_is_atomic_and_fail_closed() {
        let now = Utc::now();
        let claims = claims(now);
        let usage = MemoryUsageStore::default();
        usage.reserve(&claims, Uuid::new_v4(), now).unwrap();
        assert!(matches!(
            usage.reserve(&claims, Uuid::new_v4(), now),
            Err(GatewayError::LimitExceeded)
        ));
    }

    // Kept as a compile-time construction check; TLS interception belongs in
    // the end-to-end gateway validation harness rather than weakening the
    // production HTTPS-only invariant for an in-process HTTP fixture.
    #[tokio::test]
    async fn fake_provider_fixture_starts() {
        let (_url, handle) = fake_provider().await;
        handle.abort();
    }

    #[tokio::test]
    async fn purpose_token_proxies_once_with_injected_provider_key_and_audit() {
        let (upstream_origin, provider) = fake_provider().await;
        let now = Utc::now();
        let codec = PurposeTokenCodec::new(SecretBytes::new(vec![9; 32]).unwrap(), "model-gateway")
            .unwrap();
        let token = codec.issue_at(&claims(now), now).unwrap();
        let audit = MemoryAuditSink::default();
        let state = GatewayState::new_for_http_fixture(
            GatewayConfig {
                upstream_origin,
                upstream_path_prefix: String::new(),
                model_routes: BTreeMap::new(),
                provider_key: SecretBytes::new(b"provider-secret-key".to_vec()).unwrap(),
                token_codec: codec,
                max_output_tokens_cap: 16_384,
                rates: fixture_rates(),
            },
            Arc::new(MemoryUsageStore::default()),
            Arc::new(audit.clone()),
        )
        .unwrap();
        let app = router(state);
        let request = || {
            HttpRequest::post("/v1/responses")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"model":"gpt-test","input":"hello"}"#))
                .unwrap()
        };

        let response = app.clone().oneshot(request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key(RECEIPT_HEADER));
        let body = to_bytes(response.into_body(), 4_096).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({"ok": true})
        );
        tokio::task::yield_now().await;
        let events = audit.events();
        assert!(
            events
                .iter()
                .any(|event| event.kind == AuditEventKind::Admitted)
        );
        assert!(
            events
                .iter()
                .any(|event| event.kind == AuditEventKind::StreamCompleted)
        );
        assert!(
            events
                .iter()
                .filter(|event| event.model.is_some())
                .all(|event| {
                    event.model.as_deref() == Some("gpt-test")
                        && event.upstream_model.as_deref() == Some("gpt-test")
                })
        );

        let exhausted = app.oneshot(request()).await.unwrap();
        assert_eq!(exhausted.status(), StatusCode::PAYLOAD_TOO_LARGE);
        provider.abort();
    }

    #[tokio::test]
    async fn fixed_prefix_and_explicit_model_route_rewrite_the_upstream_only() {
        let (upstream_origin, provider) = prefixed_routed_provider().await;
        let now = Utc::now();
        let codec = PurposeTokenCodec::new(SecretBytes::new(vec![9; 32]).unwrap(), "model-gateway")
            .unwrap();
        let token = codec.issue_at(&claims(now), now).unwrap();
        let audit = MemoryAuditSink::default();
        let state = GatewayState::new_for_http_fixture(
            GatewayConfig {
                upstream_origin,
                upstream_path_prefix: "/api".into(),
                model_routes: BTreeMap::from([(
                    "gpt-test".into(),
                    "vendor/frontier-model:free".into(),
                )]),
                provider_key: SecretBytes::new(b"provider-secret-key".to_vec()).unwrap(),
                token_codec: codec,
                max_output_tokens_cap: 16_384,
                rates: fixture_rates(),
            },
            Arc::new(MemoryUsageStore::default()),
            Arc::new(audit.clone()),
        )
        .unwrap();
        let response = router(state)
            .oneshot(
                HttpRequest::post("/v1/responses")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"model":"gpt-test","input":"hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _ = to_bytes(response.into_body(), 4_096).await.unwrap();
        let events = audit.events();
        assert!(
            events
                .iter()
                .filter(|event| event.model.is_some())
                .all(|event| {
                    event.model.as_deref() == Some("gpt-test")
                        && event.upstream_model.as_deref() == Some("vendor/frontier-model:free")
                })
        );
        provider.abort();
    }

    #[tokio::test]
    async fn duplicate_authorization_headers_are_rejected_without_upstream_access() {
        let now = Utc::now();
        let codec = PurposeTokenCodec::new(SecretBytes::new(vec![9; 32]).unwrap(), "model-gateway")
            .unwrap();
        let token = codec.issue_at(&claims(now), now).unwrap();
        let root = tempfile::tempdir().unwrap();
        let state = GatewayState::build(
            GatewayConfig {
                upstream_origin: Url::parse("https://api.openai.com").unwrap(),
                upstream_path_prefix: String::new(),
                model_routes: BTreeMap::new(),
                provider_key: SecretBytes::new(b"provider-secret-key".to_vec()).unwrap(),
                token_codec: codec,
                max_output_tokens_cap: 16_384,
                rates: fixture_rates(),
            },
            Arc::new(MemoryUsageStore::default()),
            Arc::new(MemoryAuditSink::default()),
            Arc::new(SpendStore::open(root.path()).unwrap()),
            crate::spend::ceiling_map(),
            true,
        )
        .unwrap();
        let response = router(state)
            .oneshot(
                HttpRequest::post("/v1/responses")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"model":"gpt-test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn provider_redirects_are_returned_without_being_followed() {
        let redirected = Arc::new(AtomicBool::new(false));
        let (upstream_origin, provider) = redirecting_provider(Arc::clone(&redirected)).await;
        let now = Utc::now();
        let codec = PurposeTokenCodec::new(SecretBytes::new(vec![9; 32]).unwrap(), "model-gateway")
            .unwrap();
        let token = codec.issue_at(&claims(now), now).unwrap();
        let state = GatewayState::new_for_http_fixture(
            GatewayConfig {
                upstream_origin,
                upstream_path_prefix: String::new(),
                model_routes: BTreeMap::new(),
                provider_key: SecretBytes::new(b"provider-secret-key".to_vec()).unwrap(),
                token_codec: codec,
                max_output_tokens_cap: 16_384,
                rates: fixture_rates(),
            },
            Arc::new(MemoryUsageStore::default()),
            Arc::new(MemoryAuditSink::default()),
        )
        .unwrap();
        let response = router(state)
            .oneshot(
                HttpRequest::post("/v1/responses")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"model":"gpt-test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert!(!response.headers().contains_key(header::LOCATION));
        let _ = to_bytes(response.into_body(), 4_096).await.unwrap();
        assert!(!redirected.load(Ordering::SeqCst));
        provider.abort();
    }

    #[tokio::test]
    async fn provider_stream_is_bounded_before_oversized_bytes_reach_the_caller() {
        let (upstream_origin, provider) = oversized_provider().await;
        let now = Utc::now();
        let codec = PurposeTokenCodec::new(SecretBytes::new(vec![9; 32]).unwrap(), "model-gateway")
            .unwrap();
        let mut bounded_claims = claims(now);
        bounded_claims.limits.maximum_response_bytes = 8;
        let token = codec.issue_at(&bounded_claims, now).unwrap();
        let audit = MemoryAuditSink::default();
        let state = GatewayState::new_for_http_fixture(
            GatewayConfig {
                upstream_origin,
                upstream_path_prefix: String::new(),
                model_routes: BTreeMap::new(),
                provider_key: SecretBytes::new(b"provider-secret-key".to_vec()).unwrap(),
                token_codec: codec,
                max_output_tokens_cap: 16_384,
                rates: fixture_rates(),
            },
            Arc::new(MemoryUsageStore::default()),
            Arc::new(audit.clone()),
        )
        .unwrap();
        let response = router(state)
            .oneshot(
                HttpRequest::post("/v1/responses")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"model":"gpt-test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(to_bytes(response.into_body(), 4_096).await.is_err());
        assert!(
            audit
                .events()
                .iter()
                .any(|event| event.kind == AuditEventKind::StreamLimitExceeded)
        );
        provider.abort();
    }

    #[tokio::test]
    async fn provider_stream_failures_receive_a_terminal_audit_record() {
        let (upstream_origin, provider) = failing_stream_provider().await;
        let now = Utc::now();
        let codec = PurposeTokenCodec::new(SecretBytes::new(vec![9; 32]).unwrap(), "model-gateway")
            .unwrap();
        let token = codec.issue_at(&claims(now), now).unwrap();
        let audit = MemoryAuditSink::default();
        let state = GatewayState::new_for_http_fixture(
            GatewayConfig {
                upstream_origin,
                upstream_path_prefix: String::new(),
                model_routes: BTreeMap::new(),
                provider_key: SecretBytes::new(b"provider-secret-key".to_vec()).unwrap(),
                token_codec: codec,
                max_output_tokens_cap: 16_384,
                rates: fixture_rates(),
            },
            Arc::new(MemoryUsageStore::default()),
            Arc::new(audit.clone()),
        )
        .unwrap();
        let response = router(state)
            .oneshot(
                HttpRequest::post("/v1/responses")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"model":"gpt-test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(to_bytes(response.into_body(), 4_096).await.is_err());
        assert!(
            audit
                .events()
                .iter()
                .any(|event| event.kind == AuditEventKind::StreamFailed)
        );
        provider.abort();
    }

    #[tokio::test]
    async fn spend_fuse_refuses_closed_at_missing_or_reached_ceiling() {
        let (upstream_origin, provider) = fake_provider().await;
        let now = Utc::now();
        let codec = PurposeTokenCodec::new(SecretBytes::new(vec![9; 32]).unwrap(), "model-gateway")
            .unwrap();
        let mut many = claims(now);
        many.limits.maximum_requests = 10;
        let token = codec.issue_at(&many, now).unwrap();
        let root = tempfile::tempdir().unwrap();
        let spend = Arc::new(SpendStore::open(root.path()).unwrap());
        let ceilings = crate::spend::ceiling_map();
        let state = GatewayState::build(
            GatewayConfig {
                upstream_origin,
                upstream_path_prefix: String::new(),
                model_routes: BTreeMap::new(),
                provider_key: SecretBytes::new(b"provider-secret-key".to_vec()).unwrap(),
                token_codec: codec,
                max_output_tokens_cap: 16_384,
                rates: fixture_rates(),
            },
            Arc::new(MemoryUsageStore::default()),
            Arc::new(MemoryAuditSink::default()),
            Arc::clone(&spend),
            ceilings.clone(),
            false,
        )
        .unwrap();
        let app = router(state);
        let request = || {
            HttpRequest::post("/v1/responses")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"model":"gpt-test","input":"hello"}"#))
                .unwrap()
        };

        // No ceiling entry for the company: refuse closed, upstream untouched.
        let response = app.clone().oneshot(request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Under the ceiling: proxies.
        ceilings
            .write()
            .unwrap()
            .insert("company-1".to_owned(), 1_000_000);
        let response = app.clone().oneshot(request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let _ = to_bytes(response.into_body(), 4_096).await.unwrap();

        // At the ceiling: the typed fuse error, still no upstream call.
        ceilings.write().unwrap().insert("company-1".to_owned(), 0);
        let response = app.clone().oneshot(request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = to_bytes(response.into_body(), 4_096).await.unwrap();
        assert!(String::from_utf8_lossy(&body).contains("spend_ceiling_exceeded"));

        // Poisoned accounting beats any ceiling, even the maximum.
        ceilings
            .write()
            .unwrap()
            .insert("company-1".to_owned(), u64::MAX);
        spend.poison("company-1");
        let response = app.oneshot(request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        provider.abort();
    }
}
