//! Fleet's authenticated, live readiness view of one hosted account plane.
//!
//! A provider receipt proves that a container was materialised. This endpoint
//! proves the exact Core plane can currently use the six subsystems required
//! before Fleet issues owner-entry assertions. No check is inferred from an
//! environment declaration: unknown live state is reported distinctly and is
//! never accepted as readiness.

use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result};
use axum::{
    extract::{DefaultBodyLimit, Path as AxumPath, State},
    http::{
        header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, COOKIE, HOST, ORIGIN},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::OwnerState;
use crate::{credential, entry::EntryMode, release, runtime, Daemon};

const CONTRACT_VERSION: u32 = 1;
const TOKEN_ENV: &str = "RESTLESS_PLANE_READINESS_TOKEN";
const ACCOUNT_PLANE_IMAGE_ENV: &str = "RESTLESS_ACCOUNT_PLANE_IMAGE";
const DESIRED_REVISION_ENV: &str = "RESTLESS_DESIRED_REVISION";
const RELEASE_MANIFEST_DIGEST_ENV: &str = "RESTLESS_RELEASE_MANIFEST_DIGEST";
const MAX_REQUEST_BYTES: usize = 4 * 1024;
const MAX_COMPANIES: usize = 4_096;
const MAX_COCKPIT_INDEX_BYTES: u64 = 1024 * 1024;

#[derive(Clone)]
pub(super) enum PlaneReadinessService {
    Disabled,
    Enabled(Arc<PlaneReadinessDeployment>),
}

#[derive(Clone)]
pub(super) struct PlaneReadinessDeployment {
    identity: PlaneIdentity,
    token: Arc<[u8]>,
    daemon: Arc<Daemon>,
    entry: EntryMode,
    cockpit: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlaneIdentity {
    owner_id: Uuid,
    plane_id: Uuid,
    hostname: String,
    account_plane_image: String,
    desired_revision: i64,
    core_release: String,
    release_manifest_digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaneReadinessRequest {
    contract_version: u32,
    owner_id: Uuid,
    plane_id: Uuid,
    hostname: String,
    account_plane_image: String,
    desired_revision: i64,
}

#[derive(Debug, Serialize)]
struct PlaneReadinessObservation {
    contract_version: u32,
    owner_id: Uuid,
    plane_id: Uuid,
    hostname: String,
    account_plane_image: String,
    desired_revision: i64,
    core_release: String,
    release_manifest_digest: String,
    status: &'static str,
    ready: bool,
    checks: Vec<PlaneReadinessCheck>,
    observed_at: chrono::DateTime<Utc>,
    valid_until: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct PlaneReadinessCheck {
    kind: &'static str,
    status: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckStatus {
    Ready,
    Failed,
    Unknown,
}

impl CheckStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

impl PlaneReadinessService {
    pub(super) fn from_environment(daemon: &Arc<Daemon>, entry: &EntryMode) -> Result<Self> {
        let Some((owner_id, plane_id, hostname)) = entry.network_coordinates() else {
            return Ok(Self::Disabled);
        };
        let token = required_environment(TOKEN_ENV)?;
        if !(32..=512).contains(&token.len())
            || token.trim() != token
            || token.bytes().any(|byte| byte.is_ascii_control())
        {
            anyhow::bail!("{TOKEN_ENV} must be one bounded bearer value");
        }
        let identity = PlaneIdentity {
            owner_id,
            plane_id,
            hostname: hostname.to_string(),
            account_plane_image: required_environment(ACCOUNT_PLANE_IMAGE_ENV)?,
            desired_revision: required_environment(DESIRED_REVISION_ENV)?
                .parse()
                .with_context(|| format!("parse {DESIRED_REVISION_ENV}"))?,
            core_release: release::CORE_VERSION.to_string(),
            release_manifest_digest: required_environment(RELEASE_MANIFEST_DIGEST_ENV)?,
        };
        identity.validate()?;
        let cockpit = match std::env::var_os("RESTLESS_COCKPIT_DIR") {
            Some(path) => PathBuf::from(path),
            None => runtime::source_root()?.join("web/build"),
        };
        Ok(Self::Enabled(Arc::new(PlaneReadinessDeployment {
            identity,
            token: Arc::from(token.into_bytes()),
            daemon: daemon.clone(),
            entry: entry.clone(),
            cockpit,
        })))
    }

    fn deployment(&self) -> Option<&PlaneReadinessDeployment> {
        match self {
            Self::Disabled => None,
            Self::Enabled(deployment) => Some(deployment),
        }
    }
}

impl PlaneIdentity {
    fn validate(&self) -> Result<()> {
        if self.owner_id.is_nil()
            || self.plane_id.is_nil()
            || !valid_hostname(&self.hostname)
            || !valid_immutable_image(&self.account_plane_image)
            || self.desired_revision < 1
            || !valid_release(&self.core_release)
            || !valid_sha256_digest(&self.release_manifest_digest)
        {
            anyhow::bail!("plane-readiness deployment identity is incomplete or mutable");
        }
        Ok(())
    }

    fn matches(&self, path_plane_id: Uuid, request: &PlaneReadinessRequest) -> bool {
        request.contract_version == CONTRACT_VERSION
            && request.owner_id == self.owner_id
            && request.plane_id == self.plane_id
            && path_plane_id == self.plane_id
            && request.hostname == self.hostname
            && request.account_plane_image == self.account_plane_image
            && request.desired_revision == self.desired_revision
    }
}

pub(super) fn routes() -> Router<OwnerState> {
    Router::new()
        .route(
            "/internal/v1/planes/{plane_id}/readiness",
            post(observe_plane_readiness),
        )
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
}

pub(super) fn is_plane_readiness_path(path: &str) -> bool {
    path.strip_prefix("/internal/v1/planes/")
        .and_then(|rest| rest.strip_suffix("/readiness"))
        .is_some_and(|plane| plane.parse::<Uuid>().is_ok())
}

async fn observe_plane_readiness(
    State(state): State<OwnerState>,
    AxumPath(path_plane_id): AxumPath<Uuid>,
    headers: HeaderMap,
    Json(request): Json<PlaneReadinessRequest>,
) -> Response {
    let Some(deployment) = state.plane_readiness.deployment() else {
        return refusal(StatusCode::NOT_FOUND, "plane_readiness_unavailable");
    };
    if !authorized(&headers, deployment) {
        return refusal(StatusCode::UNAUTHORIZED, "plane_readiness_unauthorized");
    }
    if !deployment.identity.matches(path_plane_id, &request) {
        return refusal(StatusCode::CONFLICT, "plane_readiness_identity_mismatch");
    }

    let checks = live_checks(deployment).await;
    let recovering = deployment.daemon.lifecycle.is_recovering();
    let draining = deployment.daemon.lifecycle.is_draining();
    let observation = build_observation(&deployment.identity, checks, recovering, draining);
    let mut response = Json(observation).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn live_checks(deployment: &PlaneReadinessDeployment) -> Vec<PlaneReadinessCheck> {
    let (authority, custody, identity_handoff, plane_database) = tokio::join!(
        probe_authority(&deployment.daemon),
        credential::probe_plane_custody(deployment.identity.plane_id),
        probe_identity_handoff(&deployment.entry),
        probe_plane_database(&deployment.daemon),
    );
    vec![
        checked("authority", authority),
        checked("credential_custody", custody),
        checked_known(
            "company_directory",
            probe_company_directory(&deployment.daemon.root),
        ),
        checked("identity_handoff", identity_handoff),
        checked_known("cockpit", probe_cockpit(&deployment.cockpit)),
        checked("plane_database", plane_database),
    ]
}

fn checked(kind: &'static str, result: Result<()>) -> PlaneReadinessCheck {
    match result {
        Ok(()) => PlaneReadinessCheck {
            kind,
            status: CheckStatus::Ready.as_str(),
        },
        Err(error) => {
            tracing::warn!(check = kind, %error, "account-plane readiness check is unknown");
            PlaneReadinessCheck {
                kind,
                status: CheckStatus::Unknown.as_str(),
            }
        }
    }
}

fn checked_known(kind: &'static str, result: Result<CheckStatus>) -> PlaneReadinessCheck {
    match result {
        Ok(status) => PlaneReadinessCheck {
            kind,
            status: status.as_str(),
        },
        Err(error) => {
            tracing::warn!(check = kind, %error, "account-plane readiness check failed");
            PlaneReadinessCheck {
                kind,
                status: CheckStatus::Failed.as_str(),
            }
        }
    }
}

fn build_observation(
    identity: &PlaneIdentity,
    checks: Vec<PlaneReadinessCheck>,
    recovering: bool,
    draining: bool,
) -> PlaneReadinessObservation {
    let all_ready = checks.iter().all(|check| check.status == "ready");
    let any_failed = checks
        .iter()
        .any(|check| matches!(check.status, "failed" | "unknown"));
    let status = if draining {
        "draining"
    } else if any_failed {
        "degraded"
    } else if recovering || !all_ready {
        "starting"
    } else {
        "ready"
    };
    let observed_at = Utc::now();
    PlaneReadinessObservation {
        contract_version: CONTRACT_VERSION,
        owner_id: identity.owner_id,
        plane_id: identity.plane_id,
        hostname: identity.hostname.clone(),
        account_plane_image: identity.account_plane_image.clone(),
        desired_revision: identity.desired_revision,
        core_release: identity.core_release.clone(),
        release_manifest_digest: identity.release_manifest_digest.clone(),
        status,
        ready: status == "ready" && all_ready,
        checks,
        observed_at,
        valid_until: observed_at + Duration::seconds(20),
    }
}

async fn probe_authority(daemon: &Daemon) -> Result<()> {
    let schema_ready: bool = sqlx::query_scalar(
        "SELECT to_regclass('restless_authority.records') IS NOT NULL \
         AND to_regclass('restless_authority.company_bootstrap_operations') IS NOT NULL",
    )
    .fetch_one(daemon.authority.pool())
    .await
    .context("probe Authority schema")?;
    anyhow::ensure!(schema_ready, "required Authority relations are absent");
    Ok(())
}

async fn probe_plane_database(daemon: &Daemon) -> Result<()> {
    let expected_database = url::Url::parse(&daemon.orgintel.database_url)
        .context("parse configured plane database URL")?
        .path()
        .strip_prefix('/')
        .filter(|name| !name.is_empty() && !name.contains('/'))
        .context("configured plane database URL has no exact database")?
        .to_string();
    let (observed_database, primary): (String, bool) =
        sqlx::query_as("SELECT current_database(), NOT pg_is_in_recovery()")
            .fetch_one(daemon.authority.pool())
            .await
            .context("probe account-plane database")?;
    anyhow::ensure!(
        observed_database == expected_database && primary,
        "account-plane database identity or writable-primary state differs"
    );
    Ok(())
}

async fn probe_identity_handoff(entry: &EntryMode) -> Result<()> {
    let network = entry
        .network()
        .context("identity handoff is unavailable outside network mode")?;
    network
        .prepare()
        .await
        .context("refresh Fleet identity handoff keys")
}

fn probe_company_directory(root: &Path) -> Result<CheckStatus> {
    let directory = root.join("companies");
    let metadata = fs::symlink_metadata(&directory)
        .with_context(|| format!("inspect {}", directory.display()))?;
    anyhow::ensure!(
        metadata.file_type().is_dir() && !metadata.file_type().is_symlink(),
        "company directory is not a real directory"
    );
    let companies = crate::configured_companies(root)?;
    anyhow::ensure!(
        companies.len() <= MAX_COMPANIES,
        "company directory exceeds its bounded inventory"
    );
    for company in companies {
        let path = directory.join(format!("{company}.toml"));
        let metadata =
            fs::symlink_metadata(&path).with_context(|| format!("inspect {}", path.display()))?;
        anyhow::ensure!(
            metadata.file_type().is_file() && !metadata.file_type().is_symlink(),
            "company configuration is not a regular file"
        );
        runtime::CompanyConfig::load(root, &company)
            .with_context(|| format!("load company configuration {company}"))?;
    }
    Ok(CheckStatus::Ready)
}

fn probe_cockpit(cockpit: &Path) -> Result<CheckStatus> {
    let index = cockpit.join("index.html");
    let metadata = fs::symlink_metadata(&index)
        .with_context(|| format!("inspect cockpit entry {}", index.display()))?;
    anyhow::ensure!(
        metadata.file_type().is_file()
            && !metadata.file_type().is_symlink()
            && (1..=MAX_COCKPIT_INDEX_BYTES).contains(&metadata.len()),
        "cockpit entry is not one bounded regular file"
    );
    let mut file = fs::File::open(&index)
        .with_context(|| format!("open cockpit entry {}", index.display()))?;
    let mut head = Vec::with_capacity(metadata.len().min(64 * 1024) as usize);
    file.by_ref()
        .take(64 * 1024)
        .read_to_end(&mut head)
        .context("read cockpit entry")?;
    let head = std::str::from_utf8(&head).context("cockpit entry is not UTF-8")?;
    anyhow::ensure!(
        head.to_ascii_lowercase().contains("<html"),
        "cockpit entry is not an HTML document"
    );
    Ok(CheckStatus::Ready)
}

fn authorized(headers: &HeaderMap, deployment: &PlaneReadinessDeployment) -> bool {
    authorized_for(
        headers,
        &deployment.identity.hostname,
        deployment.token.as_ref(),
    )
}

fn authorized_for(headers: &HeaderMap, hostname: &str, token: &[u8]) -> bool {
    if headers.contains_key(COOKIE)
        || headers.contains_key(ORIGIN)
        || headers.contains_key("forwarded")
        || headers.contains_key("x-forwarded-host")
        || headers.contains_key("x-forwarded-proto")
        || headers.contains_key("x-forwarded-for")
        || !headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.eq_ignore_ascii_case("application/json"))
    {
        return false;
    }
    let host_matches = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<axum::http::uri::Authority>().ok())
        .is_some_and(|authority| {
            authority.host().eq_ignore_ascii_case(hostname)
                && authority.port_u16().is_none_or(|port| port == 443)
        });
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let bearer_matches = values
        .next()
        .and_then(|value| value.as_bytes().strip_prefix(b"Bearer "))
        .is_some_and(|candidate| constant_time_equal(candidate, token));
    host_matches && bearer_matches && values.next().is_none()
}

fn valid_hostname(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.contains('.')
        && !value.contains(['/', ':'])
        && value.parse::<std::net::IpAddr>().is_err()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

fn valid_immutable_image(value: &str) -> bool {
    value
        .rsplit_once("@sha256:")
        .is_some_and(|(repository, digest)| {
            !repository.is_empty()
                && !repository.contains(char::is_whitespace)
                && digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
}

fn valid_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn valid_release(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
}

fn required_environment(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} is required"))?;
    if value.trim() != value || value.is_empty() || value.contains(['\r', '\n']) {
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

fn refusal(status: StatusCode, code: &'static str) -> Response {
    let mut response = (status, Json(json!({ "error": code }))).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> PlaneIdentity {
        PlaneIdentity {
            owner_id: Uuid::parse_str("11111111-1111-7111-8111-111111111111").unwrap(),
            plane_id: Uuid::parse_str("22222222-2222-7222-8222-222222222222").unwrap(),
            hostname: "owner.example.test".into(),
            account_plane_image: format!(
                "registry.example.test/account-plane@sha256:{}",
                "a".repeat(64)
            ),
            desired_revision: 3,
            core_release: "1.2.3".into(),
            release_manifest_digest: format!("sha256:{}", "b".repeat(64)),
        }
    }

    fn request(identity: &PlaneIdentity) -> PlaneReadinessRequest {
        PlaneReadinessRequest {
            contract_version: CONTRACT_VERSION,
            owner_id: identity.owner_id,
            plane_id: identity.plane_id,
            hostname: identity.hostname.clone(),
            account_plane_image: identity.account_plane_image.clone(),
            desired_revision: identity.desired_revision,
        }
    }

    fn checks() -> Vec<PlaneReadinessCheck> {
        [
            "authority",
            "credential_custody",
            "company_directory",
            "identity_handoff",
            "cockpit",
            "plane_database",
        ]
        .into_iter()
        .map(|kind| PlaneReadinessCheck {
            kind,
            status: "ready",
        })
        .collect()
    }

    #[test]
    fn machine_path_accepts_only_one_exact_plane_route() {
        let plane = identity().plane_id;
        assert!(is_plane_readiness_path(&format!(
            "/internal/v1/planes/{plane}/readiness"
        )));
        assert!(!is_plane_readiness_path(&format!(
            "/api/internal/v1/planes/{plane}/readiness"
        )));
        assert!(!is_plane_readiness_path(&format!(
            "/internal/v1/planes/{plane}/readiness/extra"
        )));
    }

    #[test]
    fn exact_deployment_tuple_is_required() {
        let identity = identity();
        identity.validate().unwrap();
        let exact = request(&identity);
        assert!(identity.matches(identity.plane_id, &exact));

        let mut wrong_release_image = request(&identity);
        wrong_release_image.account_plane_image = format!(
            "registry.example.test/account-plane@sha256:{}",
            "c".repeat(64)
        );
        assert!(!identity.matches(identity.plane_id, &wrong_release_image));

        let mut mutable = identity.clone();
        mutable.account_plane_image = "registry.example.test/account-plane:latest".into();
        assert!(mutable.validate().is_err());
        let mut wrong_manifest = identity;
        wrong_manifest.release_manifest_digest = "sha256:not-a-release".into();
        assert!(wrong_manifest.validate().is_err());
    }

    #[test]
    fn unavailable_database_or_custody_can_never_be_called_ready() {
        for (kind, status) in [
            ("plane_database", CheckStatus::Unknown),
            ("credential_custody", CheckStatus::Failed),
        ] {
            let mut checks = checks();
            checks
                .iter_mut()
                .find(|check| check.kind == kind)
                .unwrap()
                .status = status.as_str();
            let observation = build_observation(&identity(), checks, false, false);
            assert_eq!(observation.status, "degraded");
            assert!(!observation.ready);
        }
    }

    #[test]
    fn all_six_live_checks_and_non_draining_lifecycle_are_required() {
        let exact = build_observation(&identity(), checks(), false, false);
        assert_eq!(exact.status, "ready");
        assert!(exact.ready);

        let recovering = build_observation(&identity(), checks(), true, false);
        assert_eq!(recovering.status, "starting");
        assert!(!recovering.ready);
        let draining = build_observation(&identity(), checks(), false, true);
        assert_eq!(draining.status, "draining");
        assert!(!draining.ready);
    }

    #[test]
    fn host_token_and_direct_machine_headers_are_all_mandatory() {
        let identity = identity();
        let token = b"readiness-token-with-at-least-32-bytes";
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_str(&identity.hostname).unwrap());
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer readiness-token-with-at-least-32-bytes"),
        );
        assert!(authorized_for(&headers, &identity.hostname, token));

        headers.insert(HOST, HeaderValue::from_static("attacker.example.test"));
        assert!(!authorized_for(&headers, &identity.hostname, token));
        headers.insert(HOST, HeaderValue::from_str(&identity.hostname).unwrap());
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer wrong-token-with-at-least-32-bytes"),
        );
        assert!(!authorized_for(&headers, &identity.hostname, token));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer readiness-token-with-at-least-32-bytes"),
        );
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("owner.example.test"),
        );
        assert!(!authorized_for(&headers, &identity.hostname, token));
    }
}
