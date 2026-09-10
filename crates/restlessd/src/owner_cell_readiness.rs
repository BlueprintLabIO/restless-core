//! Fleet's short-lived, authenticated readiness view of one hosted Company cell.
//!
//! Runtime Supervisor receipts prove that Cloud asked a host to materialise a
//! container. They do not prove that the exact released company can accept
//! Work. This endpoint closes that gap from Core's live state. It deliberately
//! returns only six fixed checks and immutable release/Runtime identity; no
//! company content or free-form diagnostic crosses into Fleet.

use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::{Context as _, Result};
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, Extension, OriginalUri, Path as AxumPath};
use axum::http::header::{
    AUTHORIZATION, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, COOKIE, HOST, ORIGIN,
};
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode};
use axum::routing::post;
use axum::Router;
use chrono::{Duration, Utc};
use restless_runtime_bridge_protocol::RuntimeIdentity;
use serde::{Deserialize, Serialize};
use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;

use crate::entry::EntryMode;
use crate::{release, Daemon};

pub(crate) const CELL_READINESS_PATH_PREFIX: &str = "/internal/v1/cells/";
const CELL_READINESS_PATH_SUFFIX: &str = "/readiness";
const CONTRACT_VERSION: u32 = 1;
const MAX_REQUEST_BYTES: usize = 4 * 1024;
const OBSERVATION_LIFETIME_SECONDS: i64 = 20;
const MINIMUM_VOLUME_HEADROOM_BYTES: u64 = 16 * 1024 * 1024;
const AUTHORITY_PROBE_TIMEOUT: StdDuration = StdDuration::from_secs(3);
const COMPANY_STORE_PROBE_TIMEOUT: StdDuration = StdDuration::from_secs(5);
const RUNTIME_PROBE_TIMEOUT: StdDuration = StdDuration::from_secs(12);
const TOKEN_FILE_ENV: &str = "RESTLESS_CELL_READINESS_TOKEN_FILE";
const RELEASE_MANIFEST_DIGEST_ENV: &str = "RESTLESS_RELEASE_MANIFEST_DIGEST";

const CHECK_KINDS: [&str; 6] = [
    "runtime",
    "authority_record",
    "company_database",
    "persistent_volume",
    "orgintel",
    "runtime_bridge",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CellReadinessRequest {
    contract_version: u32,
    owner_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    runtime_id: String,
    runtime_image: String,
    desired_revision: i64,
}

#[derive(Debug, Serialize)]
struct CellReadinessObservation {
    contract_version: u32,
    owner_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    runtime_id: String,
    runtime_image: String,
    desired_revision: i64,
    core_release: String,
    release_manifest_digest: String,
    status: &'static str,
    ready: bool,
    checks: Vec<CellReadinessCheck>,
    observed_at: chrono::DateTime<Utc>,
    valid_until: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct CellReadinessCheck {
    kind: &'static str,
    status: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckStatus {
    Ready,
    Pending,
    Failed,
}

impl CheckStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Pending => "pending",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone)]
enum CellReadinessEndpoint {
    Disabled,
    Enabled(Arc<CellReadinessService>),
    #[cfg(test)]
    GateOnly(Arc<CellReadinessGate>),
}

#[derive(Clone)]
struct CellReadinessGate {
    owner_id: Uuid,
    hostname: String,
    secret: ReadinessSecret,
}

#[derive(Clone)]
struct CellReadinessService {
    gate: CellReadinessGate,
    plane_id: Uuid,
    core_release: String,
    release_manifest_digest: String,
    daemon: Arc<Daemon>,
}

#[derive(Clone)]
struct ReadinessSecret(Arc<PathBuf>);

impl std::fmt::Debug for ReadinessSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ReadinessSecret([REDACTED])")
    }
}

impl ReadinessSecret {
    fn read(path: &Path) -> Result<Self> {
        read_secret_bytes(path)?;
        Ok(Self(Arc::new(path.to_path_buf())))
    }

    fn authorizes(&self, headers: &HeaderMap) -> bool {
        // Secret projection is rotated with atomic rename. Reopening it here
        // avoids pinning an obsolete inode or credential until Core restarts.
        let Ok(secret) = read_secret_bytes(&self.0) else {
            return false;
        };
        let mut values = headers.get_all(AUTHORIZATION).iter();
        let candidate = values
            .next()
            .and_then(|value| value.as_bytes().strip_prefix(b"Bearer "));
        candidate.is_some_and(|candidate| constant_time_equal(candidate, &secret))
            && values.next().is_none()
    }
}

fn read_secret_bytes(path: &Path) -> Result<Vec<u8>> {
    let linked = fs::symlink_metadata(path)
        .with_context(|| format!("inspect cell-readiness secret at {}", path.display()))?;
    if linked.file_type().is_symlink()
        || !linked.file_type().is_file()
        || !(32..=513).contains(&linked.len())
    {
        anyhow::bail!("cell-readiness secret must be one bounded regular non-symlink file");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if linked.permissions().mode() & 0o077 != 0 {
            anyhow::bail!("cell-readiness secret must not be group- or world-accessible");
        }
    }
    let file = fs::File::open(path)
        .with_context(|| format!("open cell-readiness secret at {}", path.display()))?;
    let opened = file
        .metadata()
        .context("read opened cell-readiness secret metadata")?;
    if !opened.is_file() || opened.len() != linked.len() {
        anyhow::bail!("cell-readiness secret changed before it was opened");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if opened.dev() != linked.dev() || opened.ino() != linked.ino() {
            anyhow::bail!("cell-readiness secret changed identity before it was opened");
        }
    }
    let mut bytes = Vec::with_capacity(514);
    file.take(514)
        .read_to_end(&mut bytes)
        .context("read cell-readiness secret")?;
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    if !(32..=512).contains(&bytes.len())
        || bytes
            .iter()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
    {
        anyhow::bail!("cell-readiness secret must be one normalized 32-512 byte bearer");
    }
    Ok(bytes)
}

impl CellReadinessService {
    fn from_environment(daemon: &Arc<Daemon>, entry: &EntryMode) -> Result<CellReadinessEndpoint> {
        let Some((owner_id, plane_id, hostname)) = entry.network_coordinates() else {
            if std::env::var_os(TOKEN_FILE_ENV).is_some() {
                anyhow::bail!("{TOKEN_FILE_ENV} is only valid in hosted network entry mode");
            }
            return Ok(CellReadinessEndpoint::Disabled);
        };
        let path = std::env::var_os(TOKEN_FILE_ENV)
            .with_context(|| format!("{TOKEN_FILE_ENV} is required in hosted network mode"))?;
        let path = PathBuf::from(path);
        if !path.is_absolute() {
            anyhow::bail!("{TOKEN_FILE_ENV} must be an absolute secret-file path");
        }
        let release_manifest_digest = required_environment(RELEASE_MANIFEST_DIGEST_ENV)?;
        if !valid_sha256_digest(&release_manifest_digest) {
            anyhow::bail!("{RELEASE_MANIFEST_DIGEST_ENV} is not one SHA-256 manifest identity");
        }
        Ok(CellReadinessEndpoint::Enabled(Arc::new(Self {
            gate: CellReadinessGate {
                owner_id,
                hostname: hostname.to_string(),
                secret: ReadinessSecret::read(&path)?,
            },
            plane_id,
            core_release: release::CORE_VERSION.to_string(),
            release_manifest_digest,
            daemon: Arc::clone(daemon),
        })))
    }

    async fn expected_runtime(
        &self,
        request: &CellReadinessRequest,
    ) -> Result<Option<RuntimeIdentity>> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Uuid,
                String,
                String,
                i64,
                String,
                String,
                String,
            ),
        >(
            "SELECT owner_id,plane_id,company_id,company_handle,runtime_id,runtime_generation,\
                    runtime_image,volume_name,source_revision \
             FROM restless_authority.runtime_bridge_generations WHERE cell_id=$1",
        )
        .bind(request.cell_id)
        .fetch_optional(self.daemon.authority.pool())
        .await
        .context("read exact cell Runtime identity")?;
        let Some(row) = row else {
            return Ok(None);
        };
        let identity = RuntimeIdentity {
            owner_id: row.0,
            plane_id: row.1,
            company_id: row.2,
            cell_id: request.cell_id,
            company: row.3,
            runtime_id: row.4,
            runtime_generation: row.5,
            runtime_image: row.6,
            volume_name: row.7,
            source_revision: row.8,
        };
        if identity.owner_id != request.owner_id
            || identity.plane_id != self.plane_id
            || identity.company_id != request.company_id
            || identity.runtime_id != request.runtime_id
            || identity.runtime_image != request.runtime_image
            || identity.runtime_generation < 1
            || crate::runtime::validate_company_name(&identity.company).is_err()
            || !valid_volume_name(&identity.volume_name)
            || identity.source_revision != release::SOURCE_REVISION
        {
            return Ok(None);
        }
        Ok(Some(identity))
    }
}

impl CellReadinessEndpoint {
    fn gate(&self) -> Option<&CellReadinessGate> {
        match self {
            Self::Disabled => None,
            Self::Enabled(service) => Some(&service.gate),
            #[cfg(test)]
            Self::GateOnly(gate) => Some(gate),
        }
    }

    fn service(&self) -> Option<&CellReadinessService> {
        match self {
            Self::Enabled(service) => Some(service),
            Self::Disabled => None,
            #[cfg(test)]
            Self::GateOnly(_) => None,
        }
    }
}

fn request_is_well_formed(
    owner_id: Uuid,
    path_cell_id: Uuid,
    request: &CellReadinessRequest,
) -> bool {
    request.contract_version == CONTRACT_VERSION
        && !request.owner_id.is_nil()
        && !request.company_id.is_nil()
        && !request.cell_id.is_nil()
        && request.cell_id == path_cell_id
        && request.owner_id == owner_id
        && request.desired_revision > 0
        && valid_runtime_id(&request.runtime_id)
        && valid_immutable_image(&request.runtime_image)
}

pub(crate) fn routes<S>(daemon: &Arc<Daemon>, entry: &EntryMode) -> Result<Router<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let endpoint = Arc::new(CellReadinessService::from_environment(daemon, entry)?);
    Ok(router_with_endpoint(endpoint))
}

fn router_with_endpoint<S>(endpoint: Arc<CellReadinessEndpoint>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::<S>::new()
        .route(
            "/internal/v1/cells/{cell_id}/readiness",
            post(observe_cell_readiness),
        )
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .layer(Extension(endpoint))
}

pub(crate) fn is_cell_readiness_path(path: &str) -> bool {
    path.strip_prefix(CELL_READINESS_PATH_PREFIX)
        .and_then(|rest| rest.strip_suffix(CELL_READINESS_PATH_SUFFIX))
        .is_some_and(|cell| cell.parse::<Uuid>().is_ok())
}

async fn observe_cell_readiness(
    Extension(endpoint): Extension<Arc<CellReadinessEndpoint>>,
    OriginalUri(uri): OriginalUri,
    AxumPath(path_cell_id): AxumPath<Uuid>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let Some(gate) = endpoint.gate() else {
        return refusal(StatusCode::NOT_FOUND, "cell_readiness_unavailable");
    };
    if uri.path()
        != format!("{CELL_READINESS_PATH_PREFIX}{path_cell_id}{CELL_READINESS_PATH_SUFFIX}")
        || uri.query().is_some()
        || !valid_machine_envelope(&headers, &gate.hostname)
    {
        return refusal(StatusCode::BAD_REQUEST, "cell_readiness_envelope");
    }
    if !gate.secret.authorizes(&headers) {
        return refusal(StatusCode::UNAUTHORIZED, "cell_readiness_unauthorized");
    }
    if body.is_empty() || body.len() > MAX_REQUEST_BYTES {
        return refusal(StatusCode::PAYLOAD_TOO_LARGE, "cell_readiness_body_size");
    }
    let request = match serde_json::from_slice::<CellReadinessRequest>(&body) {
        Ok(request) => request,
        Err(_) => return refusal(StatusCode::BAD_REQUEST, "cell_readiness_request_invalid"),
    };
    if !request_is_well_formed(gate.owner_id, path_cell_id, &request) {
        return refusal(StatusCode::CONFLICT, "cell_readiness_identity_mismatch");
    }
    let Some(service) = endpoint.service() else {
        return refusal(
            StatusCode::SERVICE_UNAVAILABLE,
            "cell_readiness_unavailable",
        );
    };
    let identity =
        match tokio::time::timeout(AUTHORITY_PROBE_TIMEOUT, service.expected_runtime(&request))
            .await
        {
            Ok(Ok(Some(identity))) => identity,
            Ok(Ok(None)) => {
                return refusal(StatusCode::CONFLICT, "cell_readiness_identity_mismatch")
            }
            Ok(Err(error)) => {
                tracing::warn!(%error, "cell readiness could not resolve durable Runtime identity");
                return refusal(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "cell_readiness_unavailable",
                );
            }
            Err(_) => {
                return refusal(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "cell_readiness_unavailable",
                )
            }
        };

    let (authority, store, runtime) = tokio::join!(
        bounded_authority_probe(service, &identity),
        bounded_store_probe(&service.daemon.root, &identity),
        bounded_runtime_probe(service, &identity),
    );
    let checks = vec![
        checked("runtime", runtime.runtime),
        checked("authority_record", authority),
        checked("company_database", store.database),
        checked("persistent_volume", runtime.persistent_volume),
        checked("orgintel", store.orgintel),
        checked("runtime_bridge", runtime.runtime_bridge),
    ];
    debug_assert_eq!(
        checks.iter().map(|check| check.kind).collect::<Vec<_>>(),
        CHECK_KINDS
    );
    let observation = build_observation(service, request, checks);
    let bytes = serde_json::to_vec(&observation).expect("bounded cell-readiness observation");
    machine_json(StatusCode::OK, bytes)
}

async fn bounded_authority_probe(
    service: &CellReadinessService,
    identity: &RuntimeIdentity,
) -> CheckStatus {
    tokio::time::timeout(
        AUTHORITY_PROBE_TIMEOUT,
        probe_authority_record(service, identity),
    )
    .await
    .unwrap_or(CheckStatus::Pending)
}

async fn probe_authority_record(
    service: &CellReadinessService,
    identity: &RuntimeIdentity,
) -> CheckStatus {
    let rows = match sqlx::query_as::<_, (Uuid, Uuid, Uuid, String, String, String, String)>(
        "SELECT owner_id,plane_id,company_id,company_handle,core_release,\
                release_manifest_digest,status \
         FROM restless_authority.company_bootstrap_operations \
         WHERE company_id=$1 OR cell_id=$2 OR company_handle=$3 LIMIT 2",
    )
    .bind(identity.company_id)
    .bind(identity.cell_id)
    .bind(&identity.company)
    .fetch_all(service.daemon.authority.pool())
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            tracing::warn!(%error, "cell-readiness Authority probe unavailable");
            return CheckStatus::Pending;
        }
    };
    if rows.len() != 1 {
        return if rows.is_empty() {
            CheckStatus::Pending
        } else {
            CheckStatus::Failed
        };
    }
    let row = &rows[0];
    if row.0 != identity.owner_id
        || row.1 != identity.plane_id
        || row.2 != identity.company_id
        || row.3 != identity.company
        || row.4 != service.core_release
        || row.5 != service.release_manifest_digest
    {
        return CheckStatus::Failed;
    }
    match row.6.as_str() {
        "ready" => CheckStatus::Ready,
        "provisioning" => CheckStatus::Pending,
        _ => CheckStatus::Failed,
    }
}

struct StoreChecks {
    database: CheckStatus,
    orgintel: CheckStatus,
}

async fn bounded_store_probe(root: &Path, identity: &RuntimeIdentity) -> StoreChecks {
    tokio::time::timeout(
        COMPANY_STORE_PROBE_TIMEOUT,
        probe_company_store(root, identity),
    )
    .await
    .unwrap_or(StoreChecks {
        database: CheckStatus::Pending,
        orgintel: CheckStatus::Pending,
    })
}

async fn probe_company_store(root: &Path, identity: &RuntimeIdentity) -> StoreChecks {
    let database_url = match read_cell_database_url(root, &identity.company) {
        Ok(Some(url)) => url,
        Ok(None) => {
            return StoreChecks {
                database: CheckStatus::Pending,
                orgintel: CheckStatus::Pending,
            }
        }
        Err(error) => {
            tracing::warn!(company = %identity.company, %error, "cell-readiness database credential is invalid");
            return StoreChecks {
                database: CheckStatus::Failed,
                orgintel: CheckStatus::Pending,
            };
        }
    };
    let parsed = match url::Url::parse(&database_url) {
        Ok(parsed) => parsed,
        Err(_) => {
            return StoreChecks {
                database: CheckStatus::Failed,
                orgintel: CheckStatus::Pending,
            }
        }
    };
    let expected_database = parsed.path().strip_prefix('/').unwrap_or_default();
    if !matches!(parsed.scheme(), "postgres" | "postgresql")
        || parsed.host_str().is_none()
        || parsed.username().is_empty()
        || parsed.password().is_none()
        || expected_database.is_empty()
        || expected_database.contains('/')
        || parsed.username() != expected_database
        || parsed.fragment().is_some()
    {
        return StoreChecks {
            database: CheckStatus::Failed,
            orgintel: CheckStatus::Pending,
        };
    }
    let mut connection = match PgConnection::connect(&database_url).await {
        Ok(connection) => connection,
        Err(error) => {
            tracing::warn!(company = %identity.company, %error, "cell-readiness company database is unavailable");
            return StoreChecks {
                database: CheckStatus::Pending,
                orgintel: CheckStatus::Pending,
            };
        }
    };
    let database_ready = sqlx::query_as::<_, (String, String, bool, bool)>(
        "SELECT current_database(),current_user,NOT pg_is_in_recovery(),\
                current_setting('transaction_read_only')='off'",
    )
    .fetch_one(&mut connection)
    .await
    .is_ok_and(|row| row.0 == expected_database && row.1 == parsed.username() && row.2 && row.3);
    if !database_ready {
        return StoreChecks {
            database: CheckStatus::Failed,
            orgintel: CheckStatus::Pending,
        };
    }
    let search_path_set =
        sqlx::query_scalar::<_, String>("SELECT set_config('search_path',$1,false)")
            .bind(&identity.company)
            .fetch_one(&mut connection)
            .await;
    if !search_path_set
        .as_deref()
        .is_ok_and(|value| value == identity.company.as_str())
    {
        return StoreChecks {
            database: CheckStatus::Ready,
            orgintel: CheckStatus::Failed,
        };
    }
    let orgintel_ready = sqlx::query_as::<_, (Uuid, Uuid, bool)>(
        "SELECT company_id,cell_id,\
                to_regclass('actors') IS NOT NULL \
                AND to_regclass('work') IS NOT NULL \
                AND to_regclass('rooms') IS NOT NULL \
                AND to_regclass('native_documents') IS NOT NULL \
         FROM company_access_identity WHERE singleton=TRUE",
    )
    .fetch_optional(&mut connection)
    .await
    .ok()
    .flatten()
    .is_some_and(|row| row.0 == identity.company_id && row.1 == identity.cell_id && row.2);
    StoreChecks {
        database: CheckStatus::Ready,
        orgintel: if orgintel_ready {
            CheckStatus::Ready
        } else {
            CheckStatus::Failed
        },
    }
}

struct RuntimeChecks {
    runtime: CheckStatus,
    persistent_volume: CheckStatus,
    runtime_bridge: CheckStatus,
}

async fn bounded_runtime_probe(
    service: &CellReadinessService,
    identity: &RuntimeIdentity,
) -> RuntimeChecks {
    tokio::time::timeout(RUNTIME_PROBE_TIMEOUT, probe_runtime(service, identity))
        .await
        .unwrap_or(RuntimeChecks {
            runtime: CheckStatus::Pending,
            persistent_volume: CheckStatus::Pending,
            runtime_bridge: CheckStatus::Pending,
        })
}

async fn probe_runtime(
    service: &CellReadinessService,
    identity: &RuntimeIdentity,
) -> RuntimeChecks {
    use crate::runtime_bridge::BridgeReadiness;

    let runtime_bridge = match service.daemon.runtime_bridges.readiness(identity) {
        BridgeReadiness::Ready => CheckStatus::Ready,
        BridgeReadiness::Missing | BridgeReadiness::Stale => CheckStatus::Pending,
        BridgeReadiness::IdentityMismatch => CheckStatus::Failed,
    };
    if runtime_bridge != CheckStatus::Ready {
        return RuntimeChecks {
            runtime: CheckStatus::Pending,
            persistent_volume: CheckStatus::Pending,
            runtime_bridge,
        };
    }
    let observed = match crate::runtime_bridge::preflight_observation(
        &service.daemon.runtime_bridges,
        identity,
    )
    .await
    {
        Ok(observed) => observed,
        Err(error) => {
            tracing::warn!(company = %identity.company, %error, "cell-readiness Runtime preflight unavailable");
            return RuntimeChecks {
                runtime: CheckStatus::Pending,
                persistent_volume: CheckStatus::Pending,
                runtime_bridge,
            };
        }
    };
    let now = Utc::now();
    let preflight_fresh = observed.observed_at <= now + Duration::seconds(5)
        && observed.observed_at >= now - Duration::seconds(15);
    let runtime = if observed.release.core_version == service.core_release
        && observed.release.source_revision == identity.source_revision
        && observed.release.api_contract_version == release::API_CONTRACT_VERSION
        && observed.release.assertion_contract_version == crate::entry::ASSERTION_CONTRACT_VERSION
        && observed.release.schema_version == i64::from(release::SCHEMA_VERSION)
        && preflight_fresh
    {
        CheckStatus::Ready
    } else {
        CheckStatus::Failed
    };
    let persistent_volume = if runtime == CheckStatus::Ready
        && observed.disk_available_bytes >= MINIMUM_VOLUME_HEADROOM_BYTES
    {
        CheckStatus::Ready
    } else if runtime == CheckStatus::Ready {
        CheckStatus::Failed
    } else {
        CheckStatus::Pending
    };
    RuntimeChecks {
        runtime,
        persistent_volume,
        runtime_bridge,
    }
}

fn read_cell_database_url(root: &Path, company: &str) -> Result<Option<String>> {
    let path = root.join("cells").join(company).join("database.url");
    let linked = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("inspect {}", path.display())),
    };
    if linked.file_type().is_symlink()
        || !linked.file_type().is_file()
        || !(1..=2_048).contains(&linked.len())
    {
        anyhow::bail!("cell database credential is not one bounded regular file");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        if linked.permissions().mode() & 0o077 != 0 || linked.nlink() != 1 {
            anyhow::bail!("cell database credential is not private and singly linked");
        }
    }
    let file = fs::File::open(&path).with_context(|| format!("open {}", path.display()))?;
    let opened = file
        .metadata()
        .context("inspect opened cell database credential")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if opened.dev() != linked.dev() || opened.ino() != linked.ino() {
            anyhow::bail!("cell database credential changed identity before it was opened");
        }
    }
    let mut value = String::new();
    file.take(2_049)
        .read_to_string(&mut value)
        .context("read cell database credential")?;
    if value.trim() != value || value.lines().count() != 1 {
        anyhow::bail!("cell database credential must be one normalized URL");
    }
    Ok(Some(value))
}

fn checked(kind: &'static str, status: CheckStatus) -> CellReadinessCheck {
    CellReadinessCheck {
        kind,
        status: status.as_str(),
    }
}

fn build_observation(
    service: &CellReadinessService,
    request: CellReadinessRequest,
    checks: Vec<CellReadinessCheck>,
) -> CellReadinessObservation {
    let all_ready = checks.iter().all(|check| check.status == "ready");
    let any_failed = checks.iter().any(|check| check.status == "failed");
    let status = observation_status(
        all_ready,
        any_failed,
        service.daemon.lifecycle.is_recovering(),
        service.daemon.lifecycle.is_draining(),
    );
    let observed_at = Utc::now();
    CellReadinessObservation {
        contract_version: CONTRACT_VERSION,
        owner_id: request.owner_id,
        company_id: request.company_id,
        cell_id: request.cell_id,
        runtime_id: request.runtime_id,
        runtime_image: request.runtime_image,
        desired_revision: request.desired_revision,
        core_release: service.core_release.clone(),
        release_manifest_digest: service.release_manifest_digest.clone(),
        status,
        ready: status == "ready" && all_ready,
        checks,
        observed_at,
        valid_until: observed_at + Duration::seconds(OBSERVATION_LIFETIME_SECONDS),
    }
}

fn observation_status(
    all_ready: bool,
    any_failed: bool,
    recovering: bool,
    draining: bool,
) -> &'static str {
    if draining {
        "draining"
    } else if any_failed {
        "degraded"
    } else if recovering || !all_ready {
        "starting"
    } else {
        "ready"
    }
}

fn valid_machine_envelope(headers: &HeaderMap, expected_host: &str) -> bool {
    if headers.contains_key(COOKIE)
        || headers.contains_key(ORIGIN)
        || headers.contains_key("forwarded")
        || headers.contains_key("x-original-host")
        || headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            != Some("application/json")
    {
        return false;
    }
    let mut hosts = headers.get_all(HOST).iter();
    let host_matches = hosts
        .next()
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<axum::http::uri::Authority>().ok())
        .is_some_and(|host| {
            host.host().eq_ignore_ascii_case(expected_host)
                && host.port_u16().is_none_or(|port| port == 443)
        })
        && hosts.next().is_none();
    let optional_exact = |name: &'static str, expected: &str| {
        let mut values = headers.get_all(name).iter();
        let matches = values
            .next()
            .map(|value| value.to_str().ok() == Some(expected))
            .unwrap_or(true);
        matches && values.next().is_none()
    };
    host_matches
        && optional_exact("x-forwarded-host", expected_host)
        && optional_exact("x-forwarded-proto", "https")
        && single_bounded_content_length(headers)
}

fn single_bounded_content_length(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(CONTENT_LENGTH).iter();
    let first = values.next();
    if values.next().is_some() {
        return false;
    }
    first
        .map(|value| {
            value
                .to_str()
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .is_some_and(|length| (1..=MAX_REQUEST_BYTES).contains(&length))
        })
        .unwrap_or(true)
}

fn valid_runtime_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value.trim() == value
        && !value.bytes().any(|byte| byte.is_ascii_control())
}

fn valid_volume_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

fn valid_immutable_image(value: &str) -> bool {
    value
        .rsplit_once("@sha256:")
        .is_some_and(|(repository, digest)| {
            !repository.is_empty()
                && value.len() <= 512
                && !repository.contains(char::is_whitespace)
                && digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

fn valid_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn required_environment(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} is required"))?;
    if value.is_empty() || value.trim() != value || value.contains(['\r', '\n']) {
        anyhow::bail!("{name} must be one normalized value");
    }
    Ok(value)
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    let length = left.len().max(right.len());
    for index in 0..length {
        difference |= usize::from(
            left.get(index).copied().unwrap_or_default()
                ^ right.get(index).copied().unwrap_or_default(),
        );
    }
    difference == 0
}

fn machine_json(status: StatusCode, bytes: Vec<u8>) -> Response<Body> {
    let mut response = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .header(CONTENT_LENGTH, bytes.len().to_string())
        .body(Body::from(bytes))
        .expect("bounded cell-readiness response");
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn refusal(status: StatusCode, code: &'static str) -> Response<Body> {
    machine_json(
        status,
        serde_json::to_vec(&serde_json::json!({ "error": code }))
            .expect("bounded cell-readiness refusal"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;
    use tower::ServiceExt as _;

    fn request() -> CellReadinessRequest {
        CellReadinessRequest {
            contract_version: CONTRACT_VERSION,
            owner_id: Uuid::parse_str("11111111-1111-7111-8111-111111111111").unwrap(),
            company_id: Uuid::parse_str("22222222-2222-7222-8222-222222222222").unwrap(),
            cell_id: Uuid::parse_str("33333333-3333-7333-8333-333333333333").unwrap(),
            runtime_id: "restless-cell-33333333-3333-7333-8333-333333333333".into(),
            runtime_image: format!("registry.example.test/runtime@sha256:{}", "a".repeat(64)),
            desired_revision: 7,
        }
    }

    #[test]
    fn path_match_is_exact() {
        let cell = request().cell_id;
        assert!(is_cell_readiness_path(&format!(
            "/internal/v1/cells/{cell}/readiness"
        )));
        assert!(!is_cell_readiness_path(&format!(
            "/internal/v1/cells/{cell}/readiness/extra"
        )));
        assert!(!is_cell_readiness_path(&format!(
            "/api/internal/v1/cells/{cell}/readiness"
        )));
    }

    #[tokio::test]
    async fn route_enforces_exact_envelope_bearer_and_identity_before_live_probes() {
        let exact = request();
        let directory = std::env::temp_dir().join(format!(
            "restless-cell-readiness-route-{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir(&directory).unwrap();
        let secret_path = directory.join("token");
        fs::write(&secret_path, b"readiness-secret-that-is-long-enough").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(&secret_path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let app = router_with_endpoint::<()>(Arc::new(CellReadinessEndpoint::GateOnly(Arc::new(
            CellReadinessGate {
                owner_id: exact.owner_id,
                hostname: "owner.example.test".into(),
                secret: ReadinessSecret::read(&secret_path).unwrap(),
            },
        ))));
        let body = serde_json::json!({
            "contract_version": exact.contract_version,
            "owner_id": exact.owner_id,
            "company_id": exact.company_id,
            "cell_id": exact.cell_id,
            "runtime_id": exact.runtime_id,
            "runtime_image": exact.runtime_image,
            "desired_revision": exact.desired_revision,
        });
        let call =
            |host: &'static str, bearer: &'static str, cell_id: Uuid, body: serde_json::Value| {
                HttpRequest::builder()
                    .method("POST")
                    .uri(format!("/internal/v1/cells/{cell_id}/readiness"))
                    .header(HOST, host)
                    .header(CONTENT_TYPE, "application/json")
                    .header(AUTHORIZATION, format!("Bearer {bearer}"))
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap()
            };

        assert_eq!(
            app.clone()
                .oneshot(call(
                    "attacker.example.test",
                    "readiness-secret-that-is-long-enough",
                    exact.cell_id,
                    body.clone(),
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            app.clone()
                .oneshot(call(
                    "owner.example.test",
                    "wrong-readiness-secret-that-is-long-enough",
                    exact.cell_id,
                    body.clone(),
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            app.clone()
                .oneshot(call(
                    "owner.example.test",
                    "readiness-secret-that-is-long-enough",
                    Uuid::new_v4(),
                    body.clone(),
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            app.oneshot(call(
                "owner.example.test",
                "readiness-secret-that-is-long-enough",
                exact.cell_id,
                body,
            ))
            .await
            .unwrap()
            .status(),
            // Passing all three gates reaches the deliberately absent live
            // service in this route-only fixture.
            StatusCode::SERVICE_UNAVAILABLE
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn machine_envelope_requires_exact_host_json_and_direct_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("owner.example.test"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(CONTENT_LENGTH, HeaderValue::from_static("100"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer readiness-secret-that-is-long-enough"),
        );
        let directory = std::env::temp_dir().join(format!(
            "restless-cell-readiness-auth-{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("token");
        fs::write(&path, b"readiness-secret-that-is-long-enough").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let secret = ReadinessSecret::read(&path).unwrap();
        assert!(valid_machine_envelope(&headers, "owner.example.test"));
        assert!(secret.authorizes(&headers));

        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("attacker.test"),
        );
        assert!(!valid_machine_envelope(&headers, "owner.example.test"));
        headers.remove("x-forwarded-host");
        headers.insert(COOKIE, HeaderValue::from_static("session=browser"));
        assert!(!valid_machine_envelope(&headers, "owner.example.test"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn file_backed_bearer_refuses_symlink_and_public_permissions() {
        let directory = std::env::temp_dir().join(format!(
            "restless-cell-readiness-secret-{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("token");
        fs::write(&path, b"0123456789abcdef0123456789abcdef\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        assert_eq!(read_secret_bytes(&path).unwrap().len(), 32);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(ReadinessSecret::read(&path).is_err());
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            std::os::unix::fs::symlink(&path, directory.join("link")).unwrap();
            assert!(ReadinessSecret::read(&directory.join("link")).is_err());
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn six_checks_and_short_lease_are_contract_exact() {
        let checks = CHECK_KINDS
            .into_iter()
            .map(|kind| checked(kind, CheckStatus::Ready))
            .collect::<Vec<_>>();
        assert_eq!(checks.len(), 6);
        assert!(checks.iter().all(|check| check.status == "ready"));

        let observed_at = Utc::now();
        let valid_until = observed_at + Duration::seconds(OBSERVATION_LIFETIME_SECONDS);
        assert_eq!((valid_until - observed_at).num_seconds(), 20);
        assert_eq!(observation_status(true, false, false, false), "ready");
        assert_eq!(observation_status(false, false, false, false), "starting");
        assert_eq!(observation_status(false, true, false, false), "degraded");
        assert_eq!(observation_status(true, false, false, true), "draining");
    }

    #[test]
    fn image_runtime_and_manifest_validation_fail_closed() {
        let exact = request();
        assert!(valid_runtime_id(&exact.runtime_id));
        assert!(valid_immutable_image(&exact.runtime_image));
        assert!(valid_sha256_digest(&format!("sha256:{}", "b".repeat(64))));
        assert!(!valid_immutable_image(
            "registry.example.test/runtime:latest"
        ));
        assert!(!valid_immutable_image(&format!(
            "registry.example.test/runtime@sha256:{}",
            "A".repeat(64)
        )));
        assert!(!valid_sha256_digest("sha256:not-a-digest"));
    }

    #[tokio::test]
    async fn real_postgres_probe_requires_exact_database_and_orgintel_identity() {
        let Ok(admin_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            return;
        };
        let suffix = Uuid::new_v4().simple().to_string();
        let company = format!("readiness_{}_test", &suffix[..8]);
        let root = std::env::temp_dir().join(format!("restless-cell-readiness-pg-{suffix}"));
        fs::create_dir_all(&root).unwrap();
        let company_id = Uuid::new_v4();
        let cell_id = Uuid::new_v4();
        let cell_url = crate::cell::ensure_database(&root, &admin_url, &company)
            .await
            .unwrap();
        let org = restless_orgintel::OrgIntel::ensure(&cell_url, &company)
            .await
            .unwrap();
        org.ensure_company_access_identity(restless_orgintel::CompanyAccessIdentity {
            company_id,
            cell_id,
        })
        .await
        .unwrap();
        let identity = RuntimeIdentity {
            owner_id: Uuid::new_v4(),
            plane_id: Uuid::new_v4(),
            company_id,
            cell_id,
            company: company.clone(),
            runtime_id: format!("restless-cell-{cell_id}"),
            runtime_generation: 1,
            runtime_image: format!("registry.example.test/runtime@sha256:{}", "a".repeat(64)),
            volume_name: format!("restless-cell-{cell_id}"),
            source_revision: "0123456789abcdef".into(),
        };

        let exact = probe_company_store(&root, &identity).await;
        let mut wrong = identity.clone();
        wrong.company_id = Uuid::new_v4();
        let mismatched = probe_company_store(&root, &wrong).await;

        org.close().await;
        crate::cell::destroy_database(&root, &admin_url, &company)
            .await
            .unwrap();
        fs::remove_dir_all(root).unwrap();
        assert_eq!(exact.database, CheckStatus::Ready);
        assert_eq!(exact.orgintel, CheckStatus::Ready);
        assert_eq!(mismatched.database, CheckStatus::Ready);
        assert_eq!(mismatched.orgintel, CheckStatus::Failed);
    }
}
