//! Authenticated Fleet-to-Core company bootstrap for one hosted account plane.
//!
//! This endpoint owns only the durable company substrate: the deterministic
//! Core handle, CompanyConfig, Authority binding, isolated cell schema,
//! company/cell access identity, and standing Owner/Exec actors. It never
//! starts a Company Runtime and never issues a Runtime Bridge capability.

use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result};
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, Extension, OriginalUri};
use axum::http::header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, HOST};
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode};
use axum::routing::post;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::entry::EntryMode;
use crate::{authority::AuthorityStore, cell, credential, release, runtime, Daemon};

pub(crate) const COMPANY_BOOTSTRAP_PATH: &str = "/internal/v1/companies/bootstrap";
pub(crate) const COMPANY_BOOTSTRAP_CONTRACT_VERSION: u32 = 1;
const MAX_REQUEST_BYTES: usize = 4 * 1024;
const MAX_RECEIPT_BYTES: usize = 2 * 1024;
const TOKEN_FILE_ENV: &str = "RESTLESS_COMPANY_BOOTSTRAP_TOKEN_FILE";
const DESIRED_REVISION_ENV: &str = "RESTLESS_DESIRED_REVISION";
const ACCOUNT_PLANE_IMAGE_ENV: &str = "RESTLESS_ACCOUNT_PLANE_IMAGE";
const RELEASE_MANIFEST_DIGEST_ENV: &str = "RESTLESS_RELEASE_MANIFEST_DIGEST";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompanyBootstrapRequest {
    pub contract_version: u32,
    pub operation_id: Uuid,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub plane_hostname: String,
    pub plane_desired_revision: i64,
    pub account_plane_image: String,
    pub core_release: String,
    pub release_manifest_digest: String,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub model: String,
    pub reasoning_effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompanyBootstrapReceipt {
    pub contract_version: u32,
    pub operation_id: Uuid,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub plane_hostname: String,
    pub plane_desired_revision: i64,
    pub account_plane_image: String,
    pub core_release: String,
    pub release_manifest_digest: String,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub model: String,
    pub reasoning_effort: String,
    pub status: String,
}

impl From<&CompanyBootstrapRequest> for CompanyBootstrapReceipt {
    fn from(request: &CompanyBootstrapRequest) -> Self {
        Self {
            contract_version: request.contract_version,
            operation_id: request.operation_id,
            owner_id: request.owner_id,
            plane_id: request.plane_id,
            plane_hostname: request.plane_hostname.clone(),
            plane_desired_revision: request.plane_desired_revision,
            account_plane_image: request.account_plane_image.clone(),
            core_release: request.core_release.clone(),
            release_manifest_digest: request.release_manifest_digest.clone(),
            company_id: request.company_id,
            cell_id: request.cell_id,
            model: request.model.clone(),
            reasoning_effort: request.reasoning_effort.clone(),
            status: "ready".into(),
        }
    }
}

#[derive(Clone)]
struct BootstrapSecret(Arc<[u8]>);

impl std::fmt::Debug for BootstrapSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("BootstrapSecret([REDACTED])")
    }
}

impl BootstrapSecret {
    fn read(path: &Path) -> Result<Self> {
        let link = fs::symlink_metadata(path).with_context(|| {
            format!(
                "read company-bootstrap secret metadata at {}",
                path.display()
            )
        })?;
        if link.file_type().is_symlink()
            || !link.file_type().is_file()
            || !(43..=44).contains(&link.len())
        {
            anyhow::bail!("company-bootstrap secret must be one bounded regular non-symlink file");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            if link.permissions().mode() & 0o077 != 0 {
                anyhow::bail!("company-bootstrap secret must not be group- or world-accessible");
            }
        }
        let file = fs::File::open(path)
            .with_context(|| format!("open company-bootstrap secret at {}", path.display()))?;
        let opened = file
            .metadata()
            .context("read opened company-bootstrap secret metadata")?;
        if !opened.is_file() || opened.len() != link.len() {
            anyhow::bail!("company-bootstrap secret changed before it was opened");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            if opened.dev() != link.dev() || opened.ino() != link.ino() {
                anyhow::bail!("company-bootstrap secret changed identity before it was opened");
            }
        }
        let mut bytes = Vec::with_capacity(45);
        file.take(45)
            .read_to_end(&mut bytes)
            .context("read company-bootstrap secret")?;
        // Some secret materialisers append one terminal newline. It is the
        // only tolerated formatting byte; all other whitespace is refused.
        if bytes.len() == 44 && bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        Self::from_bytes(bytes)
    }

    fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() != 43
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || URL_SAFE_NO_PAD.decode(&bytes).ok().map(|value| value.len()) != Some(32)
        {
            anyhow::bail!("company-bootstrap secret must be one 32-byte base64url token");
        }
        Ok(Self(Arc::from(bytes)))
    }

    fn authorizes(&self, headers: &HeaderMap) -> bool {
        let mut values = headers.get_all(AUTHORIZATION).iter();
        let Some(value) = values.next().and_then(|value| value.to_str().ok()) else {
            return false;
        };
        if values.next().is_some() {
            return false;
        }
        value
            .strip_prefix("Bearer ")
            .is_some_and(|candidate| constant_time_equal(&self.0, candidate.as_bytes()))
    }
}

/// Non-secret deployment coordinates used by consumers of bootstrap
/// admission. Readers never select a historic ready operation while a newer
/// deployment is still provisioning.
#[derive(Clone)]
pub(crate) struct CompanyAdmissionDeployment {
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub plane_hostname: String,
    pub plane_desired_revision: i64,
    pub account_plane_image: String,
    pub core_release: String,
    pub release_manifest_digest: String,
}

impl CompanyAdmissionDeployment {
    pub(crate) fn from_environment(owner_id: Uuid, plane_id: Uuid, hostname: &str) -> Result<Self> {
        let identity = Self {
            owner_id,
            plane_id,
            plane_hostname: hostname.to_string(),
            plane_desired_revision: required_environment(DESIRED_REVISION_ENV)?.parse()?,
            account_plane_image: required_environment(ACCOUNT_PLANE_IMAGE_ENV)?,
            core_release: release::CORE_VERSION.to_string(),
            release_manifest_digest: required_environment(RELEASE_MANIFEST_DIGEST_ENV)?,
        };
        if identity.plane_desired_revision < 1
            || !valid_immutable_image(&identity.account_plane_image)
            || !valid_sha256_digest(&identity.release_manifest_digest)
        {
            anyhow::bail!("company admission deployment is incomplete or mutable");
        }
        Ok(identity)
    }
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct CurrentCompanyAdmission {
    pub operation_id: Uuid,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub plane_hostname: String,
    pub plane_desired_revision: i64,
    pub account_plane_image: String,
    pub core_release: String,
    pub release_manifest_digest: String,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub company_handle: String,
    pub status: String,
    pub custody_conflict: bool,
}

impl CurrentCompanyAdmission {
    pub(crate) fn matches_deployment(&self, expected: &CompanyAdmissionDeployment) -> bool {
        !self.custody_conflict
            && !self.operation_id.is_nil()
            && self.owner_id == expected.owner_id
            && self.plane_id == expected.plane_id
            && self.plane_hostname == expected.plane_hostname
            && self.plane_desired_revision == expected.plane_desired_revision
            && self.account_plane_image == expected.account_plane_image
            && self.core_release == expected.core_release
            && self.release_manifest_digest == expected.release_manifest_digest
    }
}

/// One statement observes both newest admission and all-history custody under
/// one PostgreSQL snapshot. Never filter status before selecting newest:
/// provisioning is an authoritative fence, including equal-revision repairs.
pub(crate) async fn current_company_admission<'c, E>(
    executor: E,
    company_id: Option<Uuid>,
    cell_id: Option<Uuid>,
    handle: &str,
) -> std::result::Result<Option<CurrentCompanyAdmission>, sqlx::Error>
where
    E: sqlx::Executor<'c, Database = Postgres>,
{
    sqlx::query_as(
        "WITH latest AS (SELECT * FROM restless_authority.company_bootstrap_operations \
          WHERE company_id=$1 OR cell_id=$2 OR company_handle=$3 \
          ORDER BY plane_desired_revision DESC,admission_order DESC LIMIT 1) \
         SELECT latest.operation_id,latest.owner_id,latest.plane_id,latest.plane_hostname, \
           latest.plane_desired_revision,latest.account_plane_image,latest.core_release, \
           latest.release_manifest_digest,latest.company_id,latest.cell_id,latest.company_handle, \
           latest.status, EXISTS (SELECT 1 FROM restless_authority.company_bootstrap_operations history \
             WHERE (history.company_id=latest.company_id OR history.cell_id=latest.cell_id \
               OR history.company_handle=latest.company_handle) AND ( \
               history.company_id IS DISTINCT FROM latest.company_id \
               OR history.cell_id IS DISTINCT FROM latest.cell_id \
               OR history.company_handle IS DISTINCT FROM latest.company_handle \
               OR history.owner_id IS DISTINCT FROM latest.owner_id \
               OR history.plane_id IS DISTINCT FROM latest.plane_id \
               OR history.plane_hostname IS DISTINCT FROM latest.plane_hostname \
               OR history.config_fingerprint IS DISTINCT FROM latest.config_fingerprint \
               OR history.model IS DISTINCT FROM latest.model \
               OR history.reasoning_effort IS DISTINCT FROM latest.reasoning_effort \
               OR (history.plane_desired_revision=latest.plane_desired_revision AND ( \
                 history.account_plane_image IS DISTINCT FROM latest.account_plane_image \
                 OR history.core_release IS DISTINCT FROM latest.core_release \
                 OR history.release_manifest_digest IS DISTINCT FROM latest.release_manifest_digest)))) \
           AS custody_conflict FROM latest",
    )
    .bind(company_id)
    .bind(cell_id)
    .bind(handle)
    .fetch_optional(executor)
    .await
}

pub(crate) async fn lock_company_admission(
    tx: &mut Transaction<'_, Postgres>,
    company_id: Uuid,
    cell_id: Uuid,
) -> std::result::Result<(), sqlx::Error> {
    let mut keys = [
        format!("company-bootstrap:company:{company_id}"),
        format!("company-bootstrap:cell:{cell_id}"),
        format!("company-bootstrap:handle:{}", company_handle(company_id)),
    ];
    keys.sort();
    for key in keys {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(key)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

#[derive(Clone)]
struct BootstrapDeployment {
    owner_id: Uuid,
    plane_id: Uuid,
    plane_hostname: String,
    plane_desired_revision: i64,
    account_plane_image: String,
    core_release: String,
    release_manifest_digest: String,
    source_revision: String,
    secret: BootstrapSecret,
}

impl BootstrapDeployment {
    fn from_environment(entry: &EntryMode) -> Result<Option<Self>> {
        let Some(path) = std::env::var_os(TOKEN_FILE_ENV) else {
            return Ok(None);
        };
        let path = path
            .into_string()
            .map_err(|_| anyhow::anyhow!("{TOKEN_FILE_ENV} must be UTF-8"))?;
        if path.is_empty() || path.contains(['\r', '\n']) {
            anyhow::bail!("{TOKEN_FILE_ENV} must name one secret file");
        }
        let (owner_id, plane_id, plane_hostname) = entry
            .network_coordinates()
            .context("RESTLESS_COMPANY_BOOTSTRAP_TOKEN_FILE is only valid in network entry mode")?;
        let desired_revision = required_environment(DESIRED_REVISION_ENV)?
            .parse::<i64>()
            .with_context(|| format!("parse {DESIRED_REVISION_ENV}"))?;
        let deployment = Self {
            owner_id,
            plane_id,
            plane_hostname: plane_hostname.to_string(),
            plane_desired_revision: desired_revision,
            account_plane_image: required_environment(ACCOUNT_PLANE_IMAGE_ENV)?,
            core_release: release::CORE_VERSION.to_string(),
            release_manifest_digest: required_environment(RELEASE_MANIFEST_DIGEST_ENV)?,
            source_revision: release::SOURCE_REVISION.to_string(),
            secret: BootstrapSecret::read(Path::new(&path))?,
        };
        deployment.validate_configuration()?;
        Ok(Some(deployment))
    }

    fn validate_configuration(&self) -> Result<()> {
        if self.owner_id.is_nil()
            || self.plane_id.is_nil()
            || !valid_hostname(&self.plane_hostname)
            || self.plane_desired_revision < 1
            || !valid_immutable_image(&self.account_plane_image)
            || !valid_release(&self.core_release)
            || !valid_sha256_digest(&self.release_manifest_digest)
            || !exact_source_revision(&self.source_revision)
        {
            anyhow::bail!("company-bootstrap deployment identity is incomplete or mutable");
        }
        Ok(())
    }

    fn validate_request(&self, request: &CompanyBootstrapRequest) -> BootstrapResult<()> {
        if request.contract_version != COMPANY_BOOTSTRAP_CONTRACT_VERSION
            || request.operation_id.is_nil()
            || request.owner_id.is_nil()
            || request.plane_id.is_nil()
            || request.company_id.is_nil()
            || request.cell_id.is_nil()
            || !valid_hostname(&request.plane_hostname)
            || request.plane_desired_revision < 1
            || !valid_immutable_image(&request.account_plane_image)
            || !valid_release(&request.core_release)
            || !valid_sha256_digest(&request.release_manifest_digest)
            || !valid_model(&request.model)
            || !valid_reasoning_effort(&request.reasoning_effort)
        {
            return Err(BootstrapFailure::Invalid);
        }
        if request.owner_id != self.owner_id
            || request.plane_id != self.plane_id
            || request.plane_hostname != self.plane_hostname
            || request.plane_desired_revision != self.plane_desired_revision
            || request.account_plane_image != self.account_plane_image
            || request.core_release != self.core_release
            || request.release_manifest_digest != self.release_manifest_digest
        {
            return Err(BootstrapFailure::IdentityMismatch);
        }
        Ok(())
    }
}

#[derive(Clone)]
enum NativeDocumentsCredentialPublisher {
    Infisical,
    #[cfg(test)]
    Recorded(Arc<std::sync::Mutex<Vec<(Uuid, Uuid)>>>),
}

impl NativeDocumentsCredentialPublisher {
    async fn publish(&self, plane_id: Uuid, cell_id: Uuid, path: &Path) -> Result<()> {
        match self {
            Self::Infisical => {
                credential::publish_native_documents_store(plane_id, cell_id, path).await
            }
            #[cfg(test)]
            Self::Recorded(published) => {
                let metadata = fs::symlink_metadata(path)
                    .context("inspect recorded native Documents credential")?;
                if !metadata.is_file() || metadata.file_type().is_symlink() {
                    anyhow::bail!("recorded native Documents credential is not a regular file");
                }
                published
                    .lock()
                    .expect("credential publisher")
                    .push((plane_id, cell_id));
                Ok(())
            }
        }
    }
}

#[derive(Clone)]
struct CompanyBootstrapService {
    deployment: BootstrapDeployment,
    root: PathBuf,
    database_url: String,
    authority: AuthorityStore,
    native_documents_credential_publisher: NativeDocumentsCredentialPublisher,
}

impl CompanyBootstrapService {
    async fn execute(&self, request: CompanyBootstrapRequest) -> BootstrapResult<Vec<u8>> {
        self.deployment.validate_request(&request)?;
        let company_handle = company_handle(request.company_id);
        let desired_config = desired_company_config(&request, &company_handle);
        let rendered_config = canonical_company_config(&desired_config)?;
        let request_fingerprint = digest(&serde_json::to_vec(&request).map_err(unavailable)?);
        let config_fingerprint = digest(rendered_config.as_bytes());

        let _reservation = reserve_operation(
            &self.authority,
            &request,
            &company_handle,
            &request_fingerprint,
            &config_fingerprint,
        )
        .await?;

        // One database transaction holds the canonical advisory lock through
        // every idempotent external step. A second process may reserve/retry,
        // but it cannot race the shared temporary config path or acknowledge
        // readiness before this attempt finishes verification.
        let mut completion = self.authority.pool().begin().await.map_err(unavailable)?;
        lock_operation(&mut completion, &request).await?;
        validate_company_lineage(
            &mut completion,
            &request,
            &company_handle,
            &config_fingerprint,
        )
        .await?;
        let stored = stored_operation(&mut completion, request.operation_id)
            .await?
            .ok_or_else(|| {
                unavailable(anyhow::anyhow!("company-bootstrap reservation vanished"))
            })?;
        if stored.request_fingerprint != request_fingerprint
            || stored.config_fingerprint != config_fingerprint
        {
            return Err(BootstrapFailure::Conflict);
        }
        let ready_receipt = if stored.status == "ready" {
            Some(validate_receipt_bytes(
                &request,
                stored.receipt_bytes.ok_or_else(|| {
                    unavailable(anyhow::anyhow!("ready bootstrap has no receipt"))
                })?,
            )?)
        } else {
            None
        };

        ensure_company_config(&self.root, &desired_config, &rendered_config).await?;
        AuthorityStore::initialise_company_in_transaction(&mut completion, &company_handle, &[])
            .await
            .map_err(unavailable)?;

        let cell_url = cell::ensure_database(&self.root, &self.database_url, &company_handle)
            .await
            .map_err(unavailable)?;
        let org = restless_orgintel::OrgIntel::ensure(&cell_url, &company_handle)
            .await
            .map_err(unavailable)?;
        let expected_access = restless_orgintel::CompanyAccessIdentity {
            company_id: request.company_id,
            cell_id: request.cell_id,
        };
        match org.company_access_identity().await.map_err(unavailable)? {
            Some(bound) if bound != expected_access => return Err(BootstrapFailure::Conflict),
            _ => org
                .ensure_company_access_identity(expected_access)
                .await
                .map_err(company_substrate_error)?,
        }
        org.ensure_actor("owner", "owner", "owner", "The Owner")
            .await
            .map_err(company_substrate_error)?;
        org.ensure_actor_with_model("exec", "exec", "exec", "The Exec", Some(&request.model))
            .await
            .map_err(company_substrate_error)?;
        let native_documents_credential =
            cell::ensure_native_documents_store(&self.root, &self.database_url, &company_handle)
                .await
                .map_err(unavailable)?;
        self.native_documents_credential_publisher
            .publish(
                request.plane_id,
                request.cell_id,
                &native_documents_credential,
            )
            .await
            .map_err(unavailable)?;
        verify_company_substrate(&org, &request).await?;
        drop(org);

        // A durable ready receipt proves a past handoff, not present
        // readiness. Exact retries re-run the idempotent substrate checks and
        // credential-custody handoff before returning the original receipt.
        // This repairs lost/expired external custody without minting a new
        // bootstrap identity.
        if let Some(receipt) = ready_receipt {
            completion.commit().await.map_err(unavailable)?;
            return Ok(receipt);
        }

        let receipt = CompanyBootstrapReceipt::from(&request);
        let receipt_bytes = serde_json::to_vec(&receipt).map_err(unavailable)?;
        if receipt_bytes.len() > MAX_RECEIPT_BYTES {
            return Err(BootstrapFailure::Unavailable(anyhow::anyhow!(
                "company-bootstrap receipt exceeds {MAX_RECEIPT_BYTES} bytes"
            )));
        }
        let updated = sqlx::query(
            "UPDATE restless_authority.company_bootstrap_operations \
             SET status='ready',receipt_bytes=$2,ready_at=now(),updated_at=now() \
             WHERE operation_id=$1 AND request_fingerprint=$3 AND status='provisioning'",
        )
        .bind(request.operation_id)
        .bind(&receipt_bytes)
        .bind(&request_fingerprint)
        .execute(&mut *completion)
        .await
        .map_err(unavailable)?;
        if updated.rows_affected() != 1 {
            return Err(BootstrapFailure::Conflict);
        }
        completion.commit().await.map_err(unavailable)?;
        Ok(receipt_bytes)
    }
}

#[derive(Clone)]
enum BootstrapEndpoint {
    Disabled,
    Enabled(Arc<CompanyBootstrapService>),
}

pub(crate) fn routes<S>(daemon: &Arc<Daemon>, entry: &EntryMode) -> Result<Router<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let endpoint = match BootstrapDeployment::from_environment(entry)? {
        Some(deployment) => BootstrapEndpoint::Enabled(Arc::new(CompanyBootstrapService {
            deployment,
            root: daemon.root.clone(),
            database_url: daemon.orgintel.database_url.clone(),
            authority: daemon.authority.clone(),
            native_documents_credential_publisher: NativeDocumentsCredentialPublisher::Infisical,
        })),
        None => BootstrapEndpoint::Disabled,
    };
    Ok(Router::<S>::new()
        .route(COMPANY_BOOTSTRAP_PATH, post(company_bootstrap))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .layer(Extension(Arc::new(endpoint))))
}

async fn company_bootstrap(
    Extension(endpoint): Extension<Arc<BootstrapEndpoint>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let BootstrapEndpoint::Enabled(service) = endpoint.as_ref() else {
        return bootstrap_error(
            StatusCode::NOT_FOUND,
            "company_bootstrap_disabled",
            "the company-bootstrap contract is not enabled on this plane",
        );
    };
    if uri.path() != COMPANY_BOOTSTRAP_PATH
        || uri.query().is_some()
        || headers.contains_key("forwarded")
        || headers.contains_key("x-forwarded-host")
        || headers.contains_key("x-original-host")
        || headers.contains_key("origin")
        || headers.contains_key("cookie")
        || !single_content_length_is_bounded(&headers)
    {
        return bootstrap_error(
            StatusCode::BAD_REQUEST,
            "company_bootstrap_envelope",
            "company bootstrap requires the exact bounded direct endpoint",
        );
    }
    if !single_host_matches(&headers, &service.deployment.plane_hostname) {
        return bootstrap_error(
            StatusCode::FORBIDDEN,
            "company_bootstrap_identity_mismatch",
            "company bootstrap does not identify this exact account plane",
        );
    }
    if !service.deployment.secret.authorizes(&headers) {
        return bootstrap_error(
            StatusCode::UNAUTHORIZED,
            "company_bootstrap_unauthorized",
            "company bootstrap authentication failed",
        );
    }
    if headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        != Some("application/json")
    {
        return bootstrap_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "company_bootstrap_content_type",
            "company bootstrap requires application/json",
        );
    }
    if body.is_empty() || body.len() > MAX_REQUEST_BYTES {
        return bootstrap_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "company_bootstrap_body_size",
            "company bootstrap body is outside the contract bound",
        );
    }
    let request = match serde_json::from_slice::<CompanyBootstrapRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return bootstrap_error(
                StatusCode::BAD_REQUEST,
                "company_bootstrap_invalid",
                "company bootstrap request is invalid",
            )
        }
    };
    match service.execute(request).await {
        Ok(receipt) => bootstrap_json(StatusCode::OK, receipt),
        Err(BootstrapFailure::Invalid) => bootstrap_error(
            StatusCode::BAD_REQUEST,
            "company_bootstrap_invalid",
            "company bootstrap request is invalid",
        ),
        Err(BootstrapFailure::IdentityMismatch) => bootstrap_error(
            StatusCode::FORBIDDEN,
            "company_bootstrap_identity_mismatch",
            "company bootstrap does not identify this exact account plane release",
        ),
        Err(BootstrapFailure::Conflict) => bootstrap_error(
            StatusCode::CONFLICT,
            "company_bootstrap_conflict",
            "company bootstrap operation or company identity conflicts with durable state",
        ),
        Err(BootstrapFailure::Unavailable(error)) => {
            tracing::warn!(%error, "company bootstrap did not reach durable readiness");
            bootstrap_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "company_bootstrap_unavailable",
                "company bootstrap has not reached durable readiness",
            )
        }
    }
}

#[derive(Debug)]
enum BootstrapFailure {
    Invalid,
    IdentityMismatch,
    Conflict,
    Unavailable(anyhow::Error),
}

type BootstrapResult<T> = std::result::Result<T, BootstrapFailure>;

fn unavailable(error: impl Into<anyhow::Error>) -> BootstrapFailure {
    BootstrapFailure::Unavailable(error.into())
}

fn company_substrate_error(error: restless_orgintel::OrgIntelError) -> BootstrapFailure {
    match error {
        restless_orgintel::OrgIntelError::CompanyAccessMismatch(_)
        | restless_orgintel::OrgIntelError::InvalidWork(_) => BootstrapFailure::Conflict,
        error => unavailable(error),
    }
}

#[derive(Debug)]
struct StoredOperation {
    request_fingerprint: Vec<u8>,
    config_fingerprint: Vec<u8>,
    status: String,
    receipt_bytes: Option<Vec<u8>>,
}

enum Reservation {
    Provisioning,
    Ready,
}

async fn reserve_operation(
    authority: &AuthorityStore,
    request: &CompanyBootstrapRequest,
    company_handle: &str,
    request_fingerprint: &[u8],
    config_fingerprint: &[u8],
) -> BootstrapResult<Reservation> {
    let mut tx = authority.pool().begin().await.map_err(unavailable)?;
    lock_operation(&mut tx, request).await?;
    validate_company_lineage(&mut tx, request, company_handle, config_fingerprint).await?;
    if let Some(stored) = stored_operation(&mut tx, request.operation_id).await? {
        if stored.request_fingerprint != request_fingerprint
            || stored.config_fingerprint != config_fingerprint
        {
            return Err(BootstrapFailure::Conflict);
        }
        let reservation = match stored.status.as_str() {
            "provisioning" => Reservation::Provisioning,
            "ready" => {
                if stored.receipt_bytes.is_none() {
                    return Err(unavailable(anyhow::anyhow!(
                        "ready bootstrap has no receipt"
                    )));
                }
                Reservation::Ready
            }
            _ => {
                return Err(unavailable(anyhow::anyhow!(
                    "company bootstrap contains an unknown state"
                )))
            }
        };
        tx.commit().await.map_err(unavailable)?;
        return Ok(reservation);
    }
    sqlx::query(
        "INSERT INTO restless_authority.company_bootstrap_operations \
         (operation_id,request_fingerprint,owner_id,plane_id,plane_hostname, \
          plane_desired_revision,account_plane_image,core_release,release_manifest_digest, \
          company_id,cell_id,company_handle,model,reasoning_effort,config_fingerprint,status) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,'provisioning')",
    )
    .bind(request.operation_id)
    .bind(request_fingerprint)
    .bind(request.owner_id)
    .bind(request.plane_id)
    .bind(&request.plane_hostname)
    .bind(request.plane_desired_revision)
    .bind(&request.account_plane_image)
    .bind(&request.core_release)
    .bind(&request.release_manifest_digest)
    .bind(request.company_id)
    .bind(request.cell_id)
    .bind(company_handle)
    .bind(&request.model)
    .bind(&request.reasoning_effort)
    .bind(config_fingerprint)
    .execute(&mut *tx)
    .await
    .map_err(unavailable)?;
    tx.commit().await.map_err(unavailable)?;
    Ok(Reservation::Provisioning)
}

/// Re-admission may replace a deployment or repair external custody, never
/// replace a company's identity or configuration. Even a provisioning newer
/// reservation fences an old daemon before it can republish credentials.
/// Both reservation and completion call this under the canonical locks.
async fn validate_company_lineage(
    tx: &mut Transaction<'_, Postgres>,
    request: &CompanyBootstrapRequest,
    company_handle: &str,
    config_fingerprint: &[u8],
) -> BootstrapResult<()> {
    let conflict: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM restless_authority.company_bootstrap_operations \
         WHERE (company_id=$1 OR cell_id=$2 OR company_handle=$3) AND ( \
           company_id IS DISTINCT FROM $1 OR cell_id IS DISTINCT FROM $2 \
           OR company_handle IS DISTINCT FROM $3 OR owner_id IS DISTINCT FROM $4 \
           OR plane_id IS DISTINCT FROM $5 OR plane_hostname IS DISTINCT FROM $6 \
           OR config_fingerprint IS DISTINCT FROM $7 OR model IS DISTINCT FROM $8 \
           OR reasoning_effort IS DISTINCT FROM $9 OR plane_desired_revision > $10 \
           OR (plane_desired_revision = $10 AND (account_plane_image IS DISTINCT FROM $11 \
             OR core_release IS DISTINCT FROM $12 OR release_manifest_digest IS DISTINCT FROM $13))))",
    )
    .bind(request.company_id)
    .bind(request.cell_id)
    .bind(company_handle)
    .bind(request.owner_id)
    .bind(request.plane_id)
    .bind(&request.plane_hostname)
    .bind(config_fingerprint)
    .bind(&request.model)
    .bind(&request.reasoning_effort)
    .bind(request.plane_desired_revision)
    .bind(&request.account_plane_image)
    .bind(&request.core_release)
    .bind(&request.release_manifest_digest)
    .fetch_one(&mut **tx)
    .await
    .map_err(unavailable)?;
    if conflict {
        return Err(BootstrapFailure::Conflict);
    }
    Ok(())
}

async fn stored_operation(
    tx: &mut Transaction<'_, Postgres>,
    operation_id: Uuid,
) -> BootstrapResult<Option<StoredOperation>> {
    let row = sqlx::query_as::<_, (Vec<u8>, Vec<u8>, String, Option<Vec<u8>>)>(
        "SELECT request_fingerprint,config_fingerprint,status,receipt_bytes \
         FROM restless_authority.company_bootstrap_operations \
         WHERE operation_id=$1 FOR UPDATE",
    )
    .bind(operation_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(unavailable)?;
    Ok(row.map(
        |(request_fingerprint, config_fingerprint, status, receipt_bytes)| StoredOperation {
            request_fingerprint,
            config_fingerprint,
            status,
            receipt_bytes,
        },
    ))
}

async fn lock_operation(
    tx: &mut Transaction<'_, Postgres>,
    request: &CompanyBootstrapRequest,
) -> BootstrapResult<()> {
    // Every path takes the same sorted lock set. The operation lock fences
    // semantic drift; company/cell locks fence two different operation ids
    // racing for one durable identity.
    lock_company_admission(tx, request.company_id, request.cell_id)
        .await
        .map_err(unavailable)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!(
            "company-bootstrap:operation:{}",
            request.operation_id
        ))
        .execute(&mut **tx)
        .await
        .map_err(unavailable)?;
    Ok(())
}

async fn ensure_company_config(
    root: &Path,
    desired: &runtime::CompanyConfig,
    rendered: &str,
) -> BootstrapResult<()> {
    let root = root.to_path_buf();
    let desired = desired.clone();
    let rendered = rendered.to_string();
    tokio::task::spawn_blocking(move || {
        fs::create_dir_all(root.join("companies"))
            .with_context(|| format!("create company directory at {}", root.display()))
            .map_err(unavailable)?;
        let archived = root
            .join("archived-companies")
            .join(format!("{}.toml", desired.name));
        if archived.exists() {
            return Err(BootstrapFailure::Conflict);
        }
        let path = root
            .join("companies")
            .join(format!("{}.toml", desired.name));
        if path.exists() {
            let existing =
                runtime::CompanyConfig::load(&root, &desired.name).map_err(unavailable)?;
            if canonical_company_config(&existing)? != rendered {
                return Err(BootstrapFailure::Conflict);
            }
        } else {
            runtime::CompanyConfig::save(&root, &desired).map_err(unavailable)?;
        }
        let persisted = runtime::CompanyConfig::load(&root, &desired.name).map_err(unavailable)?;
        if canonical_company_config(&persisted)? != rendered {
            return Err(BootstrapFailure::Conflict);
        }
        Ok(())
    })
    .await
    .map_err(|error| unavailable(anyhow::anyhow!("company config task failed: {error}")))?
}

async fn verify_company_substrate(
    org: &restless_orgintel::OrgIntel,
    request: &CompanyBootstrapRequest,
) -> BootstrapResult<()> {
    if !org.is_live().await.map_err(unavailable)?
        || !org
            .collaboration_surfaces_ready()
            .await
            .map_err(unavailable)?
        || org.company_access_identity().await.map_err(unavailable)?
            != Some(restless_orgintel::CompanyAccessIdentity {
                company_id: request.company_id,
                cell_id: request.cell_id,
            })
    {
        return Err(BootstrapFailure::Unavailable(anyhow::anyhow!(
            "company schema or collaboration surfaces are not ready"
        )));
    }
    let owner = org.active_actor("owner").await.map_err(unavailable)?;
    let exec = org.active_actor("exec").await.map_err(unavailable)?;
    if owner.as_ref().is_none_or(|actor| {
        actor.kind != "owner" || actor.actor_class != "human" || actor.role != "owner"
    }) || exec.as_ref().is_none_or(|actor| {
        actor.kind != "exec"
            || actor.actor_class != "agent"
            || actor.role != "exec"
            || actor.model.as_deref() != Some(request.model.as_str())
    }) {
        return Err(BootstrapFailure::Conflict);
    }
    Ok(())
}

fn desired_company_config(
    request: &CompanyBootstrapRequest,
    company_handle: &str,
) -> runtime::CompanyConfig {
    runtime::CompanyConfig {
        agent_intelligence: Default::default(),
        native_harnesses: Default::default(),
        display_name: None,
        name: company_handle.to_string(),
        internal_network: false,
        mission: String::new(),
        spend_ceiling_usd: runtime::SpendCeiling::from_micro_usd(10_000_000),
        monthly_runtime_cap_hours: None,
        auto_sleep_after_minutes: None,
        outcome_standard: Default::default(),
        model: request.model.clone(),
        coordination_harness: runtime::AgentHarness::RestlessManaged,
        worker_harness: runtime::AgentHarness::RestlessManaged,
        reasoning_effort: request.reasoning_effort.clone(),
        model_failover: Vec::new(),
        credentials: hosted_model_credentials(&request.model),
        approved_parties: Vec::new(),
    }
}

/// A hosted company starts with its plane's model-relay capability for the provider it was
/// bootstrapped on, so it can think immediately. The owner may later connect their own
/// provider (bring your own credentials); those references sit beside this one.
fn hosted_model_credentials(model: &str) -> std::collections::BTreeMap<String, String> {
    let mut credentials = std::collections::BTreeMap::new();
    let hosted = std::env::var("RESTLESS_HOSTED_MODEL_CREDENTIAL_REFERENCE").ok();
    if hosted.as_deref() == Some(crate::credential::HOSTED_MODEL_RELAY_REFERENCE) {
        if let Some((provider, _)) = model.split_once('/') {
            credentials.insert(
                format!("model.inference.{provider}"),
                crate::credential::HOSTED_MODEL_RELAY_REFERENCE.to_string(),
            );
        }
    }
    credentials
}

fn canonical_company_config(config: &runtime::CompanyConfig) -> BootstrapResult<String> {
    toml::to_string_pretty(config).map_err(unavailable)
}

/// The one derivation of a hosted company's Core handle: bootstrap stores it and the Runtime
/// bridge must recognise the same value.
pub(crate) fn company_handle(company_id: Uuid) -> String {
    format!("company_{}", company_id.simple())
}

fn digest(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

fn validate_receipt_bytes(
    request: &CompanyBootstrapRequest,
    bytes: Vec<u8>,
) -> BootstrapResult<Vec<u8>> {
    if bytes.is_empty() || bytes.len() > MAX_RECEIPT_BYTES {
        return Err(unavailable(anyhow::anyhow!(
            "stored bootstrap receipt is unbounded"
        )));
    }
    let expected = CompanyBootstrapReceipt::from(request);
    let canonical = serde_json::to_vec(&expected).map_err(unavailable)?;
    let observed =
        serde_json::from_slice::<CompanyBootstrapReceipt>(&bytes).map_err(unavailable)?;
    if observed != expected || bytes != canonical {
        return Err(unavailable(anyhow::anyhow!(
            "stored bootstrap receipt is not canonical"
        )));
    }
    Ok(bytes)
}

fn required_environment(name: &'static str) -> Result<String> {
    std::env::var(name)
        .with_context(|| format!("{name} is required when company bootstrap is enabled"))
        .and_then(|value| {
            if value.is_empty()
                || value.trim() != value
                || value.contains(['\r', '\n'])
                || value.len() > 512
            {
                anyhow::bail!("{name} must be one exact bounded value");
            }
            Ok(value)
        })
}

fn valid_hostname(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.contains('.')
        && value == value.to_ascii_lowercase()
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
}

fn valid_immutable_image(value: &str) -> bool {
    let Some((repository, sha)) = value.rsplit_once("@sha256:") else {
        return false;
    };
    !repository.is_empty()
        && repository.len() <= 439
        && repository.contains('/')
        && repository.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'-' | b'_' | b'/' | b':')
        })
        && sha.len() == 64
        && sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_release(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
}

fn valid_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn exact_source_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_model(value: &str) -> bool {
    value.len() <= 160
        && runtime::validate_company_model_selection(value).is_ok()
        && value.split_once('/').is_some_and(|(provider, model)| {
            !provider.is_empty()
                && !model.is_empty()
                && provider
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && model.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(byte, b'-' | b'_' | b'.' | b'+' | b':' | b'/')
                })
        })
}

fn valid_reasoning_effort(value: &str) -> bool {
    matches!(
        value,
        "none" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
    )
}

fn single_host_matches(headers: &HeaderMap, expected: &str) -> bool {
    let mut hosts = headers.get_all(HOST).iter();
    hosts
        .next()
        .and_then(|value| value.to_str().ok())
        .is_some_and(|host| host == expected)
        && hosts.next().is_none()
}

fn single_content_length_is_bounded(headers: &HeaderMap) -> bool {
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

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn bootstrap_json(status: StatusCode, bytes: Vec<u8>) -> Response<Body> {
    let length = bytes.len();
    let mut response = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .header(CONTENT_LENGTH, length.to_string())
        .body(Body::from(bytes))
        .expect("bounded bootstrap response");
    harden_response(&mut response);
    response
}

fn bootstrap_error(
    status: StatusCode,
    error: &'static str,
    message: &'static str,
) -> Response<Body> {
    bootstrap_json(
        status,
        serde_json::to_vec(&serde_json::json!({"error": error, "message": message}))
            .expect("bootstrap error JSON"),
    )
}

fn harden_response(response: &mut Response<Body>) {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, Request as HttpRequest};
    use tower::ServiceExt as _;

    type RequestMutation = Box<dyn Fn(&mut CompanyBootstrapRequest)>;

    fn token(byte: u8) -> BootstrapSecret {
        BootstrapSecret::from_bytes(URL_SAFE_NO_PAD.encode([byte; 32]).into_bytes()).unwrap()
    }

    fn deployment() -> BootstrapDeployment {
        BootstrapDeployment {
            owner_id: Uuid::parse_str("22222222-2222-7222-8222-222222222222").unwrap(),
            plane_id: Uuid::parse_str("33333333-3333-7333-8333-333333333333").unwrap(),
            plane_hostname: "plane.preview.restless.test".into(),
            plane_desired_revision: 7,
            account_plane_image: format!(
                "ghcr.io/blueprintlabio/restless-account-plane@sha256:{}",
                "a".repeat(64)
            ),
            core_release: "0.1.0".into(),
            release_manifest_digest: format!("sha256:{}", "b".repeat(64)),
            source_revision: "c".repeat(40),
            secret: token(b'A'),
        }
    }

    fn request() -> CompanyBootstrapRequest {
        let deployment = deployment();
        CompanyBootstrapRequest {
            contract_version: 1,
            operation_id: Uuid::parse_str("11111111-1111-7111-8111-111111111111").unwrap(),
            owner_id: deployment.owner_id,
            plane_id: deployment.plane_id,
            plane_hostname: deployment.plane_hostname,
            plane_desired_revision: deployment.plane_desired_revision,
            account_plane_image: deployment.account_plane_image,
            core_release: deployment.core_release,
            release_manifest_digest: deployment.release_manifest_digest,
            company_id: Uuid::parse_str("44444444-4444-7444-8444-444444444444").unwrap(),
            cell_id: Uuid::parse_str("55555555-5555-7555-8555-555555555555").unwrap(),
            model: "openai/gpt-5.6".into(),
            reasoning_effort: "high".into(),
        }
    }

    #[test]
    fn request_and_ready_receipt_are_exact_current_contracts() {
        let request = request();
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 13);
        let mut extra = value.clone();
        extra["legacy_company"] = serde_json::json!("forbidden");
        assert!(serde_json::from_value::<CompanyBootstrapRequest>(extra).is_err());

        let receipt = CompanyBootstrapReceipt::from(&request);
        let value = serde_json::to_value(&receipt).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 14);
        assert_eq!(value["status"], "ready");
        assert_eq!(value["operation_id"], request.operation_id.to_string());
    }

    #[test]
    fn company_handle_is_total_deterministic_and_identifier_safe() {
        let id = request().company_id;
        assert_eq!(company_handle(id), company_handle(id));
        assert_eq!(company_handle(id).len(), 40);
        runtime::validate_company_name(&company_handle(id)).unwrap();
    }

    #[test]
    fn bootstrap_secret_rejects_runtime_audience_and_duplicate_authorization() {
        let company = token(b'A');
        let runtime = token(b'B');
        let company_value = URL_SAFE_NO_PAD.encode([b'A'; 32]);
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {company_value}")).unwrap(),
        );
        assert!(company.authorizes(&headers));
        assert!(!runtime.authorizes(&headers));
        headers.append(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {company_value}")).unwrap(),
        );
        assert!(!company.authorizes(&headers));
    }

    #[cfg(unix)]
    #[test]
    fn bootstrap_secret_file_requires_private_read_only_custody() {
        use std::os::unix::fs::PermissionsExt as _;

        let path = std::env::temp_dir().join(format!(
            "restless-company-bootstrap-secret-{}",
            Uuid::new_v4()
        ));
        let value = URL_SAFE_NO_PAD.encode([b'C'; 32]);
        fs::write(&path, format!("{value}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
        assert!(BootstrapSecret::read(&path).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        let secret = BootstrapSecret::read(&path).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {value}")).unwrap(),
        );
        assert!(secret.authorizes(&headers));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn exact_topology_and_release_tuple_is_required() {
        let deployment = deployment();
        deployment.validate_configuration().unwrap();
        let baseline = request();
        deployment.validate_request(&baseline).unwrap();
        let mutations: Vec<RequestMutation> = vec![
            Box::new(|value| value.owner_id = Uuid::new_v4()),
            Box::new(|value| value.plane_id = Uuid::new_v4()),
            Box::new(|value| value.plane_hostname = "other.restless.test".into()),
            Box::new(|value| value.plane_desired_revision += 1),
            Box::new(|value| {
                value.account_plane_image = format!(
                    "ghcr.io/blueprintlabio/restless-account-plane@sha256:{}",
                    "d".repeat(64)
                )
            }),
            Box::new(|value| value.core_release = "0.2.0".into()),
            Box::new(|value| value.release_manifest_digest = format!("sha256:{}", "d".repeat(64))),
        ];
        for mutate in mutations {
            let mut candidate = baseline.clone();
            mutate(&mut candidate);
            assert!(matches!(
                deployment.validate_request(&candidate),
                Err(BootstrapFailure::IdentityMismatch)
            ));
        }
    }

    #[test]
    fn malformed_payload_values_fail_before_any_durable_work() {
        let deployment = deployment();
        let baseline = request();
        let mutations: Vec<RequestMutation> = vec![
            Box::new(|value| value.contract_version = 0),
            Box::new(|value| value.operation_id = Uuid::nil()),
            Box::new(|value| value.company_id = Uuid::nil()),
            Box::new(|value| value.cell_id = Uuid::nil()),
            Box::new(|value| value.model = "unqualified".into()),
            Box::new(|value| value.reasoning_effort = "frontier".into()),
        ];
        for mutate in mutations {
            let mut candidate = baseline.clone();
            mutate(&mut candidate);
            assert!(matches!(
                deployment.validate_request(&candidate),
                Err(BootstrapFailure::Invalid)
            ));
        }
    }

    #[tokio::test]
    async fn readmission_preserves_company_and_receipts_and_fences_old_deployments() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping company-bootstrap re-admission");
            return;
        };
        let authority = AuthorityStore::connect(&database_url).await.unwrap();
        let mut original = request();
        original.operation_id = Uuid::new_v4();
        original.company_id = Uuid::new_v4();
        original.cell_id = Uuid::new_v4();
        let handle = company_handle(original.company_id);
        let root = std::env::temp_dir().join(format!(
            "restless-company-bootstrap-readmission-{}",
            original.operation_id
        ));
        let published = Arc::new(std::sync::Mutex::new(Vec::new()));
        let service_for = |request: &CompanyBootstrapRequest| {
            let mut deployment = deployment();
            deployment.owner_id = request.owner_id;
            deployment.plane_id = request.plane_id;
            deployment.plane_hostname = request.plane_hostname.clone();
            deployment.plane_desired_revision = request.plane_desired_revision;
            deployment.account_plane_image = request.account_plane_image.clone();
            deployment.core_release = request.core_release.clone();
            deployment.release_manifest_digest = request.release_manifest_digest.clone();
            Arc::new(CompanyBootstrapService {
                deployment,
                root: root.clone(),
                database_url: database_url.clone(),
                authority: authority.clone(),
                native_documents_credential_publisher: NativeDocumentsCredentialPublisher::Recorded(
                    Arc::clone(&published),
                ),
            })
        };
        let admission_deployment = |request: &CompanyBootstrapRequest| CompanyAdmissionDeployment {
            owner_id: request.owner_id,
            plane_id: request.plane_id,
            plane_hostname: request.plane_hostname.clone(),
            plane_desired_revision: request.plane_desired_revision,
            account_plane_image: request.account_plane_image.clone(),
            core_release: request.core_release.clone(),
            release_manifest_digest: request.release_manifest_digest.clone(),
        };
        let selected = |request: &CompanyBootstrapRequest| {
            current_company_admission(
                authority.pool(),
                Some(request.company_id),
                Some(request.cell_id),
                &handle,
            )
        };
        let original_service = service_for(&original);
        let original_receipt = original_service.execute(original.clone()).await.unwrap();
        let config_path = root.join("companies").join(format!("{handle}.toml"));
        let original_config = fs::read(&config_path).unwrap();
        let credential_path = cell::native_documents_store_credential_path(&root, &handle);
        let original_credential = fs::read(&credential_path).unwrap();
        let runtime_file = root
            .join("cells")
            .join(&handle)
            .join("runtime-owner-work.txt");
        fs::write(&runtime_file, b"owner work must survive").unwrap();
        let cell_url = cell::ensure_database(&root, &database_url, &handle)
            .await
            .unwrap();
        let org = restless_orgintel::OrgIntel::ensure(&cell_url, &handle)
            .await
            .unwrap();
        let document_content = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "attrs": {"block_id": "preserved"},
                "content": [{"type": "text", "text": "Owner document must survive"}],
            }],
        });
        let document = org
            .create_document(restless_orgintel::NewDocument {
                command_id: Uuid::new_v4(),
                title: "Owner document",
                kind: restless_orgintel::DocumentKind::Brief,
                visibility: restless_orgintel::DocumentVisibility::Company,
                linked_room_id: None,
                inherit_room_visibility: false,
                owner_actor_id: "owner",
                created_by_actor_id: "owner",
                content_json: &document_content,
                reason: "re-admission preservation test",
            })
            .await
            .unwrap()
            .document_id;
        let document_before =
            serde_json::to_value(org.get_document_for_actor(document, "owner").await.unwrap())
                .unwrap();
        let exec_before = org.active_actor("exec").await.unwrap().unwrap();

        // Recreate the old Authority uniqueness constraints over a real ready
        // receipt, then run the actual startup migration without dropping data.
        for (name, column) in [
            ("company_bootstrap_operations_company_id_key", "company_id"),
            ("company_bootstrap_operations_cell_id_key", "cell_id"),
            (
                "company_bootstrap_operations_company_handle_key",
                "company_handle",
            ),
        ] {
            sqlx::query(&format!(
                "ALTER TABLE restless_authority.company_bootstrap_operations \
                 ADD CONSTRAINT {name} UNIQUE ({column})"
            ))
            .execute(authority.pool())
            .await
            .unwrap();
        }
        sqlx::query("ALTER TABLE restless_authority.company_bootstrap_operations DROP COLUMN admission_order")
            .execute(authority.pool()).await.unwrap();
        AuthorityStore::connect(&database_url).await.unwrap();
        let current = selected(&original).await.unwrap().unwrap();
        assert_eq!(current.operation_id, original.operation_id);
        assert_eq!(current.status, "ready");
        assert!(current.matches_deployment(&admission_deployment(&original)));

        let mut same = original.clone();
        same.operation_id = Uuid::new_v4();
        let mut concurrent_same = same.clone();
        concurrent_same.operation_id = Uuid::new_v4();
        let rendered =
            canonical_company_config(&desired_company_config(&original, &handle)).unwrap();
        let config_fingerprint = digest(rendered.as_bytes());
        reserve_operation(
            &authority,
            &same,
            &handle,
            &digest(&serde_json::to_vec(&same).unwrap()),
            &config_fingerprint,
        )
        .await
        .unwrap();
        let current = selected(&same).await.unwrap().unwrap();
        assert_eq!(current.operation_id, same.operation_id);
        assert_eq!(
            current.status, "provisioning",
            "new repair must fence the historic ready receipt"
        );
        assert!(current.matches_deployment(&admission_deployment(&same)));
        // Publication's handle-only lookup observes the same current fence.
        assert_eq!(
            current_company_admission(authority.pool(), None, None, &handle)
                .await
                .unwrap()
                .unwrap()
                .operation_id,
            same.operation_id
        );
        let (first, second) = tokio::join!(
            original_service.execute(same.clone()),
            original_service.execute(concurrent_same.clone()),
        );
        let same_receipt = first.unwrap();
        let concurrent_receipt = second.unwrap();
        assert_eq!(
            serde_json::from_slice::<CompanyBootstrapReceipt>(&same_receipt)
                .unwrap()
                .operation_id,
            same.operation_id
        );
        assert_eq!(
            serde_json::from_slice::<CompanyBootstrapReceipt>(&concurrent_receipt)
                .unwrap()
                .operation_id,
            concurrent_same.operation_id
        );
        assert_eq!(
            original_service.execute(original.clone()).await.unwrap(),
            original_receipt
        );

        // Matching a different daemon's environment must not change custody,
        // configuration, or the already bound deployment at the same revision.
        let mutations: Vec<RequestMutation> = vec![
            Box::new(|value| value.owner_id = Uuid::new_v4()),
            Box::new(|value| value.plane_id = Uuid::new_v4()),
            Box::new(|value| value.plane_hostname = "other.restless.test".into()),
            Box::new(|value| value.company_id = Uuid::new_v4()),
            Box::new(|value| value.cell_id = Uuid::new_v4()),
            Box::new(|value| value.model = "anthropic/claude-sonnet-4-6".into()),
            Box::new(|value| value.reasoning_effort = "low".into()),
            Box::new(|value| {
                value.account_plane_image = format!(
                    "ghcr.io/blueprintlabio/restless-account-plane@sha256:{}",
                    "e".repeat(64)
                )
            }),
            Box::new(|value| value.core_release = "0.2.0".into()),
            Box::new(|value| value.release_manifest_digest = format!("sha256:{}", "d".repeat(64))),
            Box::new(|value| value.plane_desired_revision -= 1),
        ];
        for mutate in mutations {
            let mut candidate = original.clone();
            candidate.operation_id = Uuid::new_v4();
            mutate(&mut candidate);
            assert!(matches!(
                service_for(&candidate).execute(candidate).await,
                Err(BootstrapFailure::Conflict)
            ));
        }
        let rendered =
            canonical_company_config(&desired_company_config(&original, &handle)).unwrap();
        let config_fingerprint = digest(rendered.as_bytes());
        let mut wrong_handle = original.clone();
        wrong_handle.operation_id = Uuid::new_v4();
        assert!(matches!(
            reserve_operation(
                &authority,
                &wrong_handle,
                "different_handle_test",
                &digest(&serde_json::to_vec(&wrong_handle).unwrap()),
                &config_fingerprint
            )
            .await,
            Err(BootstrapFailure::Conflict)
        ));

        let mut upgraded = original.clone();
        upgraded.operation_id = Uuid::new_v4();
        upgraded.plane_desired_revision += 1;
        upgraded.account_plane_image = format!(
            "ghcr.io/blueprintlabio/restless-account-plane@sha256:{}",
            "e".repeat(64)
        );
        upgraded.release_manifest_digest = format!("sha256:{}", "f".repeat(64));
        let upgraded_service = service_for(&upgraded);

        // A lost final DB commit leaves a retryable reservation, never a
        // fabricated ready receipt; existing external custody and data survive.
        sqlx::query(&format!(
            "CREATE FUNCTION restless_authority.readmission_fault_test() RETURNS trigger LANGUAGE plpgsql AS $$ \
             BEGIN IF NEW.operation_id='{}'::uuid AND NEW.status='ready' THEN \
               RAISE EXCEPTION 'injected ready commit failure'; END IF; RETURN NEW; END $$",
            upgraded.operation_id,
        )).execute(authority.pool()).await.unwrap();
        sqlx::query(
            "CREATE TRIGGER readmission_fault_test BEFORE UPDATE \
             ON restless_authority.company_bootstrap_operations FOR EACH ROW \
             EXECUTE FUNCTION restless_authority.readmission_fault_test()",
        )
        .execute(authority.pool())
        .await
        .unwrap();
        assert!(matches!(
            upgraded_service.execute(upgraded.clone()).await,
            Err(BootstrapFailure::Unavailable(_))
        ));
        let failed: (String, Option<Vec<u8>>) = sqlx::query_as(
            "SELECT status,receipt_bytes FROM restless_authority.company_bootstrap_operations WHERE operation_id=$1"
        ).bind(upgraded.operation_id).fetch_one(authority.pool()).await.unwrap();
        assert_eq!(failed, ("provisioning".into(), None));
        let current = selected(&upgraded).await.unwrap().unwrap();
        assert_eq!(current.operation_id, upgraded.operation_id);
        assert_eq!(current.status, "provisioning");
        assert!(current.matches_deployment(&admission_deployment(&upgraded)));
        assert!(!current.matches_deployment(&admission_deployment(&original)));
        let before_old_replay = published.lock().unwrap().len();
        assert!(
            matches!(
                original_service.execute(original.clone()).await,
                Err(BootstrapFailure::Conflict)
            ),
            "an old daemon must not replay its exact historical operation after newer reservation"
        );
        assert_eq!(published.lock().unwrap().len(), before_old_replay);

        sqlx::query(
            "DROP TRIGGER readmission_fault_test ON restless_authority.company_bootstrap_operations"
        ).execute(authority.pool()).await.unwrap();
        sqlx::query("DROP FUNCTION restless_authority.readmission_fault_test()")
            .execute(authority.pool())
            .await
            .unwrap();
        let upgraded_receipt = upgraded_service.execute(upgraded.clone()).await.unwrap();
        let current = selected(&upgraded).await.unwrap().unwrap();
        assert_eq!(current.operation_id, upgraded.operation_id);
        assert_eq!(current.status, "ready");
        assert!(current.matches_deployment(&admission_deployment(&upgraded)));
        assert_eq!(
            upgraded_service.execute(upgraded.clone()).await.unwrap(),
            upgraded_receipt
        );

        // Simulate reservation/completion interleaving: a newer provisioning
        // row must fence the old completion before any external credential handoff.
        let mut newer = upgraded.clone();
        newer.operation_id = Uuid::new_v4();
        newer.plane_desired_revision += 1;
        reserve_operation(
            &authority,
            &newer,
            &handle,
            &digest(&serde_json::to_vec(&newer).unwrap()),
            &config_fingerprint,
        )
        .await
        .unwrap();
        let mut old_completion = authority.pool().begin().await.unwrap();
        lock_operation(&mut old_completion, &upgraded)
            .await
            .unwrap();
        assert!(matches!(
            validate_company_lineage(&mut old_completion, &upgraded, &handle, &config_fingerprint)
                .await,
            Err(BootstrapFailure::Conflict)
        ));
        old_completion.rollback().await.unwrap();
        let current = selected(&newer).await.unwrap().unwrap();
        assert_eq!(current.operation_id, newer.operation_id);
        assert_eq!(current.status, "provisioning");
        assert!(!current.matches_deployment(&admission_deployment(&upgraded)));
        // A corrupt historical identity cannot be hidden by a newer ready row.
        let mut conflicting_history = authority.pool().begin().await.unwrap();
        sqlx::query("UPDATE restless_authority.company_bootstrap_operations SET owner_id=$2 WHERE operation_id=$1")
            .bind(original.operation_id).bind(Uuid::new_v4())
            .execute(&mut *conflicting_history).await.unwrap();
        let current = current_company_admission(
            &mut *conflicting_history,
            Some(original.company_id),
            Some(original.cell_id),
            &handle,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(current.custody_conflict);
        assert!(!current.matches_deployment(&admission_deployment(&newer)));
        conflicting_history.rollback().await.unwrap();

        let history: Vec<(Uuid, String, Option<Vec<u8>>)> = sqlx::query_as(
            "SELECT operation_id,status,receipt_bytes FROM restless_authority.company_bootstrap_operations \
             WHERE company_id=$1 ORDER BY plane_desired_revision,created_at"
        ).bind(original.company_id).fetch_all(authority.pool()).await.unwrap();
        assert_eq!(history.len(), 5);
        for (operation, expected) in [
            (original.operation_id, original_receipt),
            (same.operation_id, same_receipt),
            (concurrent_same.operation_id, concurrent_receipt),
            (upgraded.operation_id, upgraded_receipt),
        ] {
            let entry = history.iter().find(|entry| entry.0 == operation).unwrap();
            assert_eq!(entry.1, "ready");
            assert_eq!(entry.2.as_ref(), Some(&expected));
        }
        assert_eq!(
            history
                .iter()
                .find(|entry| entry.0 == newer.operation_id)
                .unwrap()
                .1,
            "provisioning"
        );
        assert_eq!(fs::read(&config_path).unwrap(), original_config);
        assert!(
            fs::read(&credential_path).unwrap() == original_credential,
            "Documents credential must remain unchanged"
        );
        assert_eq!(fs::read(&runtime_file).unwrap(), b"owner work must survive");
        assert_eq!(
            org.company_access_identity().await.unwrap(),
            Some(restless_orgintel::CompanyAccessIdentity {
                company_id: original.company_id,
                cell_id: original.cell_id
            })
        );
        assert_eq!(
            org.active_actor("exec").await.unwrap().unwrap().created_at,
            exec_before.created_at
        );
        assert_eq!(
            serde_json::to_value(org.get_document_for_actor(document, "owner").await.unwrap())
                .unwrap(),
            document_before
        );
        assert!(published
            .lock()
            .unwrap()
            .iter()
            .all(|identity| identity == &(original.plane_id, original.cell_id)));

        org.close().await;
        cell::destroy_database(&root, &database_url, &handle)
            .await
            .unwrap();
        sqlx::query(
            "DELETE FROM restless_authority.company_bootstrap_operations WHERE company_id=$1",
        )
        .bind(original.company_id)
        .execute(authority.pool())
        .await
        .unwrap();
        sqlx::query("DELETE FROM restless_authority.company_migrations WHERE company=$1")
            .bind(&handle)
            .execute(authority.pool())
            .await
            .unwrap();
        fs::remove_dir_all(&root).unwrap();
    }

    #[tokio::test]
    async fn concurrent_lost_response_replays_one_durable_receipt() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping company-bootstrap scenario");
            return;
        };
        let authority = AuthorityStore::connect(&database_url).await.unwrap();
        let mut request = request();
        request.operation_id = Uuid::new_v4();
        request.company_id = Uuid::new_v4();
        request.cell_id = Uuid::new_v4();
        let root = std::env::temp_dir().join(format!(
            "restless-company-bootstrap-{}",
            request.operation_id
        ));
        fs::create_dir_all(root.join("companies")).unwrap();
        let published_credentials = Arc::new(std::sync::Mutex::new(Vec::new()));
        let service = Arc::new(CompanyBootstrapService {
            deployment: deployment(),
            root: root.clone(),
            database_url: database_url.clone(),
            authority: authority.clone(),
            native_documents_credential_publisher: NativeDocumentsCredentialPublisher::Recorded(
                Arc::clone(&published_credentials),
            ),
        });

        let mut attempts = Vec::new();
        for _ in 0..8 {
            let service = service.clone();
            let request = request.clone();
            attempts.push(tokio::spawn(async move { service.execute(request).await }));
        }
        let mut receipts = Vec::new();
        for attempt in attempts {
            receipts.push(attempt.await.unwrap().unwrap());
        }
        let first = receipts.first().unwrap().clone();
        assert!(receipts.iter().all(|receipt| receipt == &first));
        // Simulate a successful commit whose HTTP response was lost.
        drop(receipts);
        let replay = service.execute(request.clone()).await.unwrap();
        assert_eq!(replay, first);
        {
            let published = published_credentials.lock().unwrap();
            assert_eq!(published.len(), 9);
            assert!(published
                .iter()
                .all(|identity| identity == &(request.plane_id, request.cell_id)));
        }

        let endpoint = Arc::new(BootstrapEndpoint::Enabled(service.clone()));
        let app = Router::new()
            .route(COMPANY_BOOTSTRAP_PATH, post(company_bootstrap))
            .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
            .layer(Extension(endpoint));
        let request_bytes = serde_json::to_vec(&request).unwrap();
        let company_token = URL_SAFE_NO_PAD.encode([b'A'; 32]);
        let http_request = |uri: &str, host: &str, bearer: &str| {
            HttpRequest::builder()
                .method(Method::POST)
                .uri(uri)
                .header(HOST, host)
                .header(CONTENT_TYPE, "application/json")
                .header(AUTHORIZATION, format!("Bearer {bearer}"))
                .body(Body::from(request_bytes.clone()))
                .unwrap()
        };
        let response = app
            .clone()
            .oneshot(http_request(
                COMPANY_BOOTSTRAP_PATH,
                &service.deployment.plane_hostname,
                &company_token,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let http_receipt = axum::body::to_bytes(response.into_body(), MAX_RECEIPT_BYTES)
            .await
            .unwrap();
        assert_eq!(http_receipt.as_ref(), first.as_slice());
        assert_eq!(
            published_credentials.lock().unwrap().len(),
            10,
            "every exact retry must revalidate external credential custody"
        );

        for (candidate, status) in [
            (
                http_request(
                    COMPANY_BOOTSTRAP_PATH,
                    "other.restless.test",
                    &company_token,
                ),
                StatusCode::FORBIDDEN,
            ),
            (
                http_request(
                    &format!("{COMPANY_BOOTSTRAP_PATH}?retry=true"),
                    &service.deployment.plane_hostname,
                    &company_token,
                ),
                StatusCode::BAD_REQUEST,
            ),
            (
                http_request(
                    COMPANY_BOOTSTRAP_PATH,
                    &service.deployment.plane_hostname,
                    &URL_SAFE_NO_PAD.encode([b'B'; 32]),
                ),
                StatusCode::UNAUTHORIZED,
            ),
        ] {
            assert_eq!(
                app.clone().oneshot(candidate).await.unwrap().status(),
                status
            );
        }
        let forwarded = HttpRequest::builder()
            .method(Method::POST)
            .uri(COMPANY_BOOTSTRAP_PATH)
            .header(HOST, &service.deployment.plane_hostname)
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, format!("Bearer {company_token}"))
            .header("forwarded", "host=attacker.invalid")
            .body(Body::from(request_bytes))
            .unwrap();
        assert_eq!(
            app.oneshot(forwarded).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );

        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM restless_authority.company_bootstrap_operations \
             WHERE operation_id=$1 AND status='ready'",
        )
        .bind(request.operation_id)
        .fetch_one(authority.pool())
        .await
        .unwrap();
        assert_eq!(count, 1);

        let mut drift = request.clone();
        drift.model = "anthropic/claude-sonnet-4-6".into();
        assert!(matches!(
            service.execute(drift).await,
            Err(BootstrapFailure::Conflict)
        ));
        let mut other_operation = request.clone();
        other_operation.operation_id = Uuid::new_v4();
        other_operation.cell_id = Uuid::new_v4();
        assert!(matches!(
            service.execute(other_operation).await,
            Err(BootstrapFailure::Conflict)
        ));

        let handle = company_handle(request.company_id);
        let cell_url = cell::ensure_database(&root, &database_url, &handle)
            .await
            .unwrap();
        let org = restless_orgintel::OrgIntel::ensure(&cell_url, &handle)
            .await
            .unwrap();
        assert_eq!(
            org.company_access_identity().await.unwrap(),
            Some(restless_orgintel::CompanyAccessIdentity {
                company_id: request.company_id,
                cell_id: request.cell_id,
            })
        );
        assert!(org.collaboration_surfaces_ready().await.unwrap());
        assert!(org.active_actor("owner").await.unwrap().is_some());
        assert_eq!(
            org.active_actor("exec")
                .await
                .unwrap()
                .and_then(|actor| actor.model),
            Some(request.model.clone())
        );
        org.close().await;
        cell::destroy_database(&root, &database_url, &handle)
            .await
            .unwrap();
        sqlx::query(
            "DELETE FROM restless_authority.company_bootstrap_operations WHERE operation_id=$1",
        )
        .bind(request.operation_id)
        .execute(authority.pool())
        .await
        .unwrap();
        sqlx::query("DELETE FROM restless_authority.company_migrations WHERE company=$1")
            .bind(&handle)
            .execute(authority.pool())
            .await
            .unwrap();
        fs::remove_dir_all(&root).unwrap();
    }
}
