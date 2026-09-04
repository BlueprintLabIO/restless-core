//! Public Core contracts for a hosted company Runtime.
//!
//! Hosted Core deliberately has no Docker or Runtime-Supervisor credential.
//! Fleet owns lifecycle and capacity through the private supervisor. The
//! account plane owns the other direction: Fleet authenticates a bounded
//! bootstrap request, Core issues a company/generation-scoped capability, and
//! the Runtime Agent opens an outbound connection back to Core.
//!
//! This module contains that public, typed boundary. It does not make a
//! supervisor request and it cannot fall back to Docker when network entry is
//! enabled.

use std::{
    fmt, fs,
    io::Read as _,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac as _};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

const ENTRY_MODE_ENV: &str = "RESTLESS_ENTRY_MODE";
const OWNER_ID_ENV: &str = "RESTLESS_ENTRY_OWNER_ID";
const PLANE_ID_ENV: &str = "RESTLESS_ENTRY_PLANE_ID";
const PLANE_HOST_ENV: &str = "RESTLESS_ENTRY_HOST";
const RUNTIME_IMAGE_ENV: &str = "RESTLESS_COMPANY_IMAGE";
const BOOTSTRAP_TOKEN_FILE_ENV: &str = "RESTLESS_RUNTIME_BOOTSTRAP_TOKEN_FILE";
const TOKEN_VERSION: &str = "rb1";
const CAPABILITY_AUDIENCE: &str = "restless-core-runtime-bridge";
const CAPABILITY_TTL_SECONDS: i64 = 900;
pub const RUNTIME_BRIDGE_CONTRACT_VERSION: u32 = 1;

type HmacSha256 = Hmac<Sha256>;

/// The only two Runtime execution boundaries supported by released Core.
///
/// Callers must branch on this value. In particular, `HostedRuntimeBridge`
/// never means "try Docker and report whatever happens".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBackendKind {
    LocalDocker,
    HostedRuntimeBridge,
}

impl RuntimeBackendKind {
    pub fn from_entry_mode(value: Option<&str>) -> Result<Self, HostedRuntimeError> {
        match value.unwrap_or("local") {
            "local" => Ok(Self::LocalDocker),
            "network" => Ok(Self::HostedRuntimeBridge),
            _ => Err(HostedRuntimeError::InvalidConfiguration(
                "RESTLESS_ENTRY_MODE must be `local` or `network`",
            )),
        }
    }
}

/// Refuse every local-Docker operation in a network-reachable account plane.
///
/// This guard is intentionally tiny so legacy Core lifecycle functions can
/// put it immediately above process creation while they are migrated to the
/// Runtime Agent transport.
pub fn require_local_docker(entry_mode: Option<&str>) -> Result<(), HostedRuntimeError> {
    match RuntimeBackendKind::from_entry_mode(entry_mode)? {
        RuntimeBackendKind::LocalDocker => Ok(()),
        RuntimeBackendKind::HostedRuntimeBridge => Err(HostedRuntimeError::HostedDockerForbidden),
    }
}

pub fn require_local_docker_from_environment() -> Result<(), HostedRuntimeError> {
    let mode = optional_env_value(ENTRY_MODE_ENV)?;
    require_local_docker(mode.as_deref())
}

/// Stable deployment identity supplied by Cloud's reviewed plane template.
/// No Fleet or supervisor credential is part of this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedPlaneConfig {
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub hostname: String,
    pub runtime_image: String,
    pub core_source_revision: String,
    pub bootstrap_token_file: PathBuf,
}

impl HostedPlaneConfig {
    pub fn from_environment() -> Result<Option<Self>, HostedRuntimeError> {
        let entry_mode = optional_env_value(ENTRY_MODE_ENV)?;
        let mode = RuntimeBackendKind::from_entry_mode(entry_mode.as_deref())?;
        if mode == RuntimeBackendKind::LocalDocker {
            return Ok(None);
        }
        Self::from_values(HostedPlaneValues {
            owner_id: env_value(OWNER_ID_ENV)?,
            plane_id: env_value(PLANE_ID_ENV)?,
            hostname: env_value(PLANE_HOST_ENV)?,
            runtime_image: env_value(RUNTIME_IMAGE_ENV)?,
            bootstrap_token_file: env_value(BOOTSTRAP_TOKEN_FILE_ENV)?,
            core_source_revision: env!("RESTLESS_SOURCE_REVISION").to_owned(),
        })
        .map(Some)
    }

    pub fn from_values(values: HostedPlaneValues) -> Result<Self, HostedRuntimeError> {
        let owner_id = parse_non_nil_uuid(OWNER_ID_ENV, &values.owner_id)?;
        let plane_id = parse_non_nil_uuid(PLANE_ID_ENV, &values.plane_id)?;
        if !valid_hostname(&values.hostname) {
            return Err(HostedRuntimeError::InvalidConfiguration(
                "RESTLESS_ENTRY_HOST must be a bounded lowercase hostname",
            ));
        }
        if !immutable_image(&values.runtime_image) {
            return Err(HostedRuntimeError::InvalidConfiguration(
                "RESTLESS_COMPANY_IMAGE must be an immutable sha256 OCI reference",
            ));
        }
        if !exact_source_revision(&values.core_source_revision) {
            return Err(HostedRuntimeError::InvalidConfiguration(
                "hosted Core must identify an exact 40-character lowercase source revision",
            ));
        }
        if values.bootstrap_token_file.is_empty()
            || values.bootstrap_token_file.contains(['\r', '\n'])
        {
            return Err(HostedRuntimeError::InvalidConfiguration(
                "RESTLESS_RUNTIME_BOOTSTRAP_TOKEN_FILE must name one secret file",
            ));
        }
        Ok(Self {
            owner_id,
            plane_id,
            hostname: values.hostname,
            runtime_image: values.runtime_image,
            core_source_revision: values.core_source_revision,
            bootstrap_token_file: PathBuf::from(values.bootstrap_token_file),
        })
    }

    pub fn bootstrap_secret(&self) -> Result<BootstrapSecret, HostedRuntimeError> {
        BootstrapSecret::read(&self.bootstrap_token_file)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedPlaneValues {
    pub owner_id: String,
    pub plane_id: String,
    pub hostname: String,
    pub runtime_image: String,
    pub core_source_revision: String,
    pub bootstrap_token_file: String,
}

/// Secret used only to authenticate Fleet's bootstrap calls to this plane.
/// It is deliberately neither serializable nor printable.
#[derive(Clone, PartialEq, Eq)]
pub struct BootstrapSecret(Vec<u8>);

impl fmt::Debug for BootstrapSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BootstrapSecret([REDACTED])")
    }
}

impl BootstrapSecret {
    pub fn read(path: &Path) -> Result<Self, HostedRuntimeError> {
        let bytes = read_private_file(path, 43, HostedRuntimeError::SecretUnavailable)?;
        Self::from_bytes(bytes)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, HostedRuntimeError> {
        if bytes.len() != 43
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || URL_SAFE_NO_PAD.decode(&bytes).ok().map(|value| value.len()) != Some(32)
        {
            return Err(HostedRuntimeError::SecretUnavailable);
        }
        Ok(Self(bytes))
    }

    pub fn authorizes(&self, authorization: Option<&str>) -> bool {
        let Some(candidate) = authorization.and_then(|value| value.strip_prefix("Bearer ")) else {
            return false;
        };
        constant_time_equal(&self.0, candidate.as_bytes())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompanyBootstrapRequest {
    pub contract_version: u32,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub model: String,
    pub reasoning_effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompanyBootstrapResponse {
    pub contract_version: u32,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub model: String,
    pub reasoning_effort: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBootstrapRequest {
    pub contract_version: u32,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub runtime_id: String,
    pub runtime_generation: i64,
    pub desired_revision: i64,
    pub runtime_image: String,
    pub volume_name: String,
    pub source_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBootstrapResponse {
    pub contract_version: u32,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub desired_revision: i64,
    pub runtime_generation: i64,
    pub capability: String,
    pub valid_for_seconds: u64,
}

/// Exact identity carried by a Runtime Agent registration capability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HostedRuntimeIdentity {
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub runtime_id: String,
    pub runtime_generation: i64,
    pub runtime_image: String,
    pub volume_name: String,
    pub source_revision: String,
}

impl HostedRuntimeIdentity {
    pub fn core_company_name(&self) -> String {
        format!("c{}", self.company_id.simple())
    }
}

/// Validation result intentionally separate from the `ready` response.
///
/// The HTTP handler must durably initialise the company database, Authority
/// and model configuration before constructing `CompanyBootstrapResponse`.
/// Pure request validation alone is not readiness evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedCompanyBootstrap(CompanyBootstrapRequest);

impl ValidatedCompanyBootstrap {
    pub fn request(&self) -> &CompanyBootstrapRequest {
        &self.0
    }
}

#[derive(Clone)]
pub struct RuntimeBridgeBootstrap {
    plane: HostedPlaneConfig,
    bootstrap_secret: BootstrapSecret,
    capability_key: RuntimeBridgeCapabilityKey,
}

impl RuntimeBridgeBootstrap {
    pub fn new(
        plane: HostedPlaneConfig,
        bootstrap_secret: BootstrapSecret,
        capability_key: RuntimeBridgeCapabilityKey,
    ) -> Self {
        Self {
            plane,
            bootstrap_secret,
            capability_key,
        }
    }

    pub fn validate_company(
        &self,
        authorization: Option<&str>,
        request: CompanyBootstrapRequest,
    ) -> Result<ValidatedCompanyBootstrap, HostedRuntimeError> {
        self.authorize(authorization)?;
        validate_common_scope(
            &self.plane,
            request.contract_version,
            request.owner_id,
            request.plane_id,
            request.company_id,
            request.cell_id,
        )?;
        if !valid_model(&request.model) || !valid_reasoning_effort(&request.reasoning_effort) {
            return Err(HostedRuntimeError::InvalidRequest(
                "company bootstrap model configuration is invalid",
            ));
        }
        Ok(ValidatedCompanyBootstrap(request))
    }

    /// Authenticate and durably initialise the exact Fleet company before
    /// returning `ready`. The provisioner must not manufacture readiness from
    /// the request; it owns the account-plane database and Authority checks.
    pub async fn bootstrap_company(
        &self,
        authorization: Option<&str>,
        request: CompanyBootstrapRequest,
        provisioner: &impl HostedCompanyProvisioner,
    ) -> Result<CompanyBootstrapResponse, HostedRuntimeError> {
        let validated = self.validate_company(authorization, request)?;
        if !provisioner.ensure_company(validated.request()).await? {
            return Err(HostedRuntimeError::CompanyNotReady);
        }
        let request = validated.request();
        Ok(CompanyBootstrapResponse {
            contract_version: RUNTIME_BRIDGE_CONTRACT_VERSION,
            owner_id: request.owner_id,
            plane_id: request.plane_id,
            company_id: request.company_id,
            cell_id: request.cell_id,
            model: request.model.clone(),
            reasoning_effort: request.reasoning_effort.clone(),
            status: "ready".to_owned(),
        })
    }

    /// Validate Fleet authority and exact immutable Runtime identity, then
    /// issue a registration capability only after the caller proves the same
    /// company/cell is durably ready in Core.
    pub async fn issue_runtime_capability(
        &self,
        authorization: Option<&str>,
        request: RuntimeBootstrapRequest,
        authority: &impl HostedRuntimeAdmission,
        now: DateTime<Utc>,
    ) -> Result<RuntimeBootstrapResponse, HostedRuntimeError> {
        self.authorize(authorization)?;
        let desired_revision = request.desired_revision;
        let identity = self.validate_runtime(request)?;
        let scope = HostedCompanyScope::from(&identity);
        if !authority.exact_company_is_ready(&scope).await? {
            return Err(HostedRuntimeError::CompanyNotReady);
        }
        if !authority.admit_runtime(&identity, desired_revision).await? {
            return Err(HostedRuntimeError::IdentityMismatch);
        }
        let capability = self.capability_key.issue(&identity, now)?;
        Ok(RuntimeBootstrapResponse {
            contract_version: RUNTIME_BRIDGE_CONTRACT_VERSION,
            company_id: identity.company_id,
            cell_id: identity.cell_id,
            desired_revision,
            runtime_generation: identity.runtime_generation,
            capability,
            valid_for_seconds: CAPABILITY_TTL_SECONDS as u64,
        })
    }

    fn authorize(&self, authorization: Option<&str>) -> Result<(), HostedRuntimeError> {
        if self.bootstrap_secret.authorizes(authorization) {
            Ok(())
        } else {
            Err(HostedRuntimeError::Unauthorized)
        }
    }

    fn validate_runtime(
        &self,
        request: RuntimeBootstrapRequest,
    ) -> Result<HostedRuntimeIdentity, HostedRuntimeError> {
        validate_common_scope(
            &self.plane,
            request.contract_version,
            request.owner_id,
            request.plane_id,
            request.company_id,
            request.cell_id,
        )?;
        let expected_runtime_id = format!("restless-cell-{}", request.cell_id);
        if request.runtime_generation < 1
            || request.desired_revision < 1
            || request.runtime_id != expected_runtime_id
            || request.volume_name != format!("{expected_runtime_id}-data")
            || request.runtime_image != self.plane.runtime_image
            || request.source_revision != self.plane.core_source_revision
        {
            return Err(HostedRuntimeError::IdentityMismatch);
        }
        Ok(HostedRuntimeIdentity {
            owner_id: request.owner_id,
            plane_id: request.plane_id,
            company_id: request.company_id,
            cell_id: request.cell_id,
            runtime_id: request.runtime_id,
            runtime_generation: request.runtime_generation,
            runtime_image: request.runtime_image,
            volume_name: request.volume_name,
            source_revision: request.source_revision,
        })
    }
}

/// Durable company identity that must exist before Core grants a Runtime
/// Agent access. The implementation belongs to the account plane's database
/// and Authority store, not to Fleet or the Runtime Supervisor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedCompanyScope {
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub company_id: Uuid,
    pub cell_id: Uuid,
}

impl From<&HostedRuntimeIdentity> for HostedCompanyScope {
    fn from(identity: &HostedRuntimeIdentity) -> Self {
        Self {
            owner_id: identity.owner_id,
            plane_id: identity.plane_id,
            company_id: identity.company_id,
            cell_id: identity.cell_id,
        }
    }
}

#[async_trait]
pub trait HostedCompanyProvisioner: Send + Sync {
    /// Return true only after the exact request is durably reflected in Core's
    /// company config, isolated database, Authority identities and model.
    async fn ensure_company(
        &self,
        request: &CompanyBootstrapRequest,
    ) -> Result<bool, HostedRuntimeError>;
}

#[async_trait]
pub trait HostedCompanyReadiness: Send + Sync {
    /// Return true only after company config, isolated database, standing
    /// Authority identities and model selection all durably match this scope.
    async fn exact_company_is_ready(
        &self,
        scope: &HostedCompanyScope,
    ) -> Result<bool, HostedRuntimeError>;
}

/// Durable admission boundary for one exact current or pending Runtime.
///
/// A higher generation is staged as pending and becomes current only when its
/// one-use bridge grant is consumed. This preserves the old current identity
/// if container replacement fails before the new Runtime connects. A hot
/// capacity revision on the same exact generation may advance in place.
#[async_trait]
pub trait HostedRuntimeAdmission: HostedCompanyReadiness {
    async fn admit_runtime(
        &self,
        identity: &HostedRuntimeIdentity,
        desired_revision: i64,
    ) -> Result<bool, HostedRuntimeError>;
}

#[derive(Clone)]
pub struct RuntimeBridgeCapabilityKey([u8; 32]);

impl fmt::Debug for RuntimeBridgeCapabilityKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RuntimeBridgeCapabilityKey([REDACTED])")
    }
}

impl RuntimeBridgeCapabilityKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Derive a domain-separated bridge key from Core's existing private
    /// installation capability key. The caller passes the established key
    /// path; no new Cloud secret or environment variable is introduced.
    pub fn from_installation_key(path: &Path) -> Result<Self, HostedRuntimeError> {
        let material = read_private_file(path, 32, HostedRuntimeError::CapabilityKeyUnavailable)?;
        let mut mac = HmacSha256::new_from_slice(&material)
            .expect("installation capability key has a fixed length");
        mac.update(b"restless.hosted-runtime-bridge.key.v1\0");
        let derived: [u8; 32] = mac.finalize().into_bytes().into();
        Ok(Self(derived))
    }

    pub fn issue(
        &self,
        identity: &HostedRuntimeIdentity,
        now: DateTime<Utc>,
    ) -> Result<String, HostedRuntimeError> {
        validate_identity(identity)?;
        let claims = RuntimeBridgeClaims {
            contract_version: RUNTIME_BRIDGE_CONTRACT_VERSION,
            audience: CAPABILITY_AUDIENCE.to_owned(),
            jti: Uuid::new_v4(),
            issued_at: now,
            expires_at: now + Duration::seconds(CAPABILITY_TTL_SECONDS),
            identity: identity.clone(),
        };
        let payload =
            serde_json::to_vec(&claims).map_err(|_| HostedRuntimeError::CapabilityInvalid)?;
        let encoded = URL_SAFE_NO_PAD.encode(payload);
        let signed = format!("{TOKEN_VERSION}.{encoded}");
        let mut mac =
            HmacSha256::new_from_slice(&self.0).expect("Runtime Bridge key has a fixed length");
        mac.update(signed.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        Ok(format!("{signed}.{signature}"))
    }

    pub fn verify(
        &self,
        capability: &str,
        expected: &HostedRuntimeIdentity,
        now: DateTime<Utc>,
    ) -> Result<RuntimeBridgeGrant, HostedRuntimeError> {
        if capability.len() > 16_384 || capability.contains(char::is_whitespace) {
            return Err(HostedRuntimeError::CapabilityInvalid);
        }
        let mut pieces = capability.split('.');
        let version = pieces.next();
        let payload = pieces.next();
        let signature = pieces.next();
        if version != Some(TOKEN_VERSION)
            || payload.is_none_or(str::is_empty)
            || signature.is_none_or(str::is_empty)
            || pieces.next().is_some()
        {
            return Err(HostedRuntimeError::CapabilityInvalid);
        }
        let payload = payload.expect("validated payload");
        let signature = URL_SAFE_NO_PAD
            .decode(signature.expect("validated signature"))
            .map_err(|_| HostedRuntimeError::CapabilityInvalid)?;
        let signed = format!("{TOKEN_VERSION}.{payload}");
        let mut mac =
            HmacSha256::new_from_slice(&self.0).expect("Runtime Bridge key has a fixed length");
        mac.update(signed.as_bytes());
        mac.verify_slice(&signature)
            .map_err(|_| HostedRuntimeError::CapabilityInvalid)?;
        let claims: RuntimeBridgeClaims = serde_json::from_slice(
            &URL_SAFE_NO_PAD
                .decode(payload)
                .map_err(|_| HostedRuntimeError::CapabilityInvalid)?,
        )
        .map_err(|_| HostedRuntimeError::CapabilityInvalid)?;
        validate_claims(&claims, now)?;
        if &claims.identity != expected {
            return Err(HostedRuntimeError::IdentityMismatch);
        }
        Ok(RuntimeBridgeGrant {
            jti: claims.jti,
            identity: claims.identity,
            expires_at: claims.expires_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBridgeGrant {
    pub jti: Uuid,
    pub identity: HostedRuntimeIdentity,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RuntimeBridgeClaims {
    contract_version: u32,
    audience: String,
    jti: Uuid,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    #[serde(flatten)]
    identity: HostedRuntimeIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostedRuntimeError {
    MissingConfiguration(&'static str),
    InvalidConfiguration(&'static str),
    SecretUnavailable,
    CapabilityKeyUnavailable,
    HostedDockerForbidden,
    Unauthorized,
    InvalidRequest(&'static str),
    IdentityMismatch,
    CompanyNotReady,
    CapabilityInvalid,
    CapabilityExpired,
}

impl fmt::Display for HostedRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingConfiguration(name) => write!(formatter, "{name} is required in network entry mode"),
            Self::InvalidConfiguration(reason) => formatter.write_str(reason),
            Self::SecretUnavailable => formatter.write_str("Runtime bootstrap secret is unavailable or unsafe"),
            Self::CapabilityKeyUnavailable => formatter.write_str("Runtime Bridge signing key is unavailable or unsafe"),
            Self::HostedDockerForbidden => formatter.write_str(
                "hosted account planes cannot use the local Docker Runtime driver; Fleet owns lifecycle and company work must use the authenticated Runtime Bridge",
            ),
            Self::Unauthorized => formatter.write_str("Runtime bootstrap authentication failed"),
            Self::InvalidRequest(reason) => formatter.write_str(reason),
            Self::IdentityMismatch => formatter.write_str("Runtime bootstrap identity does not match this plane"),
            Self::CompanyNotReady => formatter.write_str("the exact hosted company and cell are not durably ready"),
            Self::CapabilityInvalid => formatter.write_str("Runtime Bridge capability is invalid"),
            Self::CapabilityExpired => formatter.write_str("Runtime Bridge capability has expired"),
        }
    }
}

impl std::error::Error for HostedRuntimeError {}

fn env_value(name: &'static str) -> Result<String, HostedRuntimeError> {
    optional_env_value(name)?
        .filter(|value| !value.is_empty())
        .ok_or(HostedRuntimeError::MissingConfiguration(name))
}

fn optional_env_value(name: &'static str) -> Result<Option<String>, HostedRuntimeError> {
    std::env::var_os(name)
        .map(|value| {
            value
                .into_string()
                .map_err(|_| HostedRuntimeError::InvalidConfiguration(name))
        })
        .transpose()
}

fn read_private_file(
    path: &Path,
    expected_len: usize,
    error: HostedRuntimeError,
) -> Result<Vec<u8>, HostedRuntimeError> {
    let file = fs::File::open(path).map_err(|_| error.clone())?;
    let metadata = file.metadata().map_err(|_| error.clone())?;
    if !metadata.is_file() || metadata.len() != expected_len as u64 {
        return Err(error);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(error);
        }
    }
    let mut bytes = Vec::with_capacity(expected_len);
    file.take(expected_len as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error.clone())?;
    if bytes.len() != expected_len {
        return Err(error);
    }
    Ok(bytes)
}

fn parse_non_nil_uuid(label: &'static str, value: &str) -> Result<Uuid, HostedRuntimeError> {
    let parsed =
        Uuid::parse_str(value).map_err(|_| HostedRuntimeError::InvalidConfiguration(label))?;
    if parsed.is_nil() {
        return Err(HostedRuntimeError::InvalidConfiguration(label));
    }
    Ok(parsed)
}

fn validate_common_scope(
    plane: &HostedPlaneConfig,
    contract_version: u32,
    owner_id: Uuid,
    plane_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
) -> Result<(), HostedRuntimeError> {
    if contract_version != RUNTIME_BRIDGE_CONTRACT_VERSION
        || company_id.is_nil()
        || cell_id.is_nil()
    {
        return Err(HostedRuntimeError::InvalidRequest(
            "unsupported Runtime bootstrap contract or nil company identity",
        ));
    }
    if owner_id != plane.owner_id || plane_id != plane.plane_id {
        return Err(HostedRuntimeError::IdentityMismatch);
    }
    Ok(())
}

fn validate_identity(identity: &HostedRuntimeIdentity) -> Result<(), HostedRuntimeError> {
    let expected_runtime_id = format!("restless-cell-{}", identity.cell_id);
    if identity.owner_id.is_nil()
        || identity.plane_id.is_nil()
        || identity.company_id.is_nil()
        || identity.cell_id.is_nil()
        || identity.runtime_generation < 1
        || identity.runtime_id != expected_runtime_id
        || identity.volume_name != format!("{expected_runtime_id}-data")
        || !immutable_image(&identity.runtime_image)
        || !exact_source_revision(&identity.source_revision)
    {
        return Err(HostedRuntimeError::CapabilityInvalid);
    }
    Ok(())
}

fn validate_claims(
    claims: &RuntimeBridgeClaims,
    now: DateTime<Utc>,
) -> Result<(), HostedRuntimeError> {
    validate_identity(&claims.identity)?;
    if claims.contract_version != RUNTIME_BRIDGE_CONTRACT_VERSION
        || claims.audience != CAPABILITY_AUDIENCE
        || claims.jti.is_nil()
        || claims.expires_at - claims.issued_at != Duration::seconds(CAPABILITY_TTL_SECONDS)
        || claims.issued_at > now + Duration::seconds(5)
    {
        return Err(HostedRuntimeError::CapabilityInvalid);
    }
    if claims.expires_at <= now {
        return Err(HostedRuntimeError::CapabilityExpired);
    }
    Ok(())
}

fn immutable_image(reference: &str) -> bool {
    if reference.len() > 1024 {
        return false;
    }
    let Some((repository, digest)) = reference.split_once("@sha256:") else {
        return false;
    };
    !repository.is_empty()
        && !repository.contains(char::is_whitespace)
        && digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn exact_source_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_hostname(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
}

fn valid_model(value: &str) -> bool {
    value.len() <= 160
        && value.split_once('/').is_some_and(|(provider, model)| {
            !provider.is_empty()
                && !model.is_empty()
                && provider
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && model.chars().all(|character| !character.is_control())
        })
}

fn valid_reasoning_effort(value: &str) -> bool {
    matches!(
        value,
        "none" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
    )
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
    use super::*;

    const IMAGE: &str = "ghcr.io/blueprintlabio/restless-company-runtime@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";
    const SECRET: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    fn ids() -> (Uuid, Uuid, Uuid, Uuid) {
        (
            Uuid::parse_str("018f47de-5708-7c87-90f8-f0a9a9fb2f31").unwrap(),
            Uuid::parse_str("018f47df-8fc7-72ea-9675-4363f793bb39").unwrap(),
            Uuid::parse_str("018f47e0-4e21-7a92-bb31-b1f24221c13c").unwrap(),
            Uuid::parse_str("018f47e1-12e3-7d7d-a946-2439690b3e42").unwrap(),
        )
    }

    fn plane() -> HostedPlaneConfig {
        let (owner_id, plane_id, _, _) = ids();
        HostedPlaneConfig::from_values(HostedPlaneValues {
            owner_id: owner_id.to_string(),
            plane_id: plane_id.to_string(),
            hostname: "owner.core.example.test".into(),
            runtime_image: IMAGE.into(),
            core_source_revision: REVISION.into(),
            bootstrap_token_file: "/run/secrets/runtime_bootstrap_token".into(),
        })
        .unwrap()
    }

    fn company_request() -> CompanyBootstrapRequest {
        let (owner_id, plane_id, company_id, cell_id) = ids();
        CompanyBootstrapRequest {
            contract_version: 1,
            owner_id,
            plane_id,
            company_id,
            cell_id,
            model: "openai/gpt-5.4".into(),
            reasoning_effort: "high".into(),
        }
    }

    fn runtime_request() -> RuntimeBootstrapRequest {
        let (owner_id, plane_id, company_id, cell_id) = ids();
        RuntimeBootstrapRequest {
            contract_version: 1,
            owner_id,
            plane_id,
            company_id,
            cell_id,
            runtime_id: format!("restless-cell-{cell_id}"),
            runtime_generation: 1,
            desired_revision: 7,
            runtime_image: IMAGE.into(),
            volume_name: format!("restless-cell-{cell_id}-data"),
            source_revision: REVISION.into(),
        }
    }

    fn boundary() -> RuntimeBridgeBootstrap {
        RuntimeBridgeBootstrap::new(
            plane(),
            BootstrapSecret::from_bytes(SECRET.as_bytes().to_vec()).unwrap(),
            RuntimeBridgeCapabilityKey::from_bytes([9; 32]),
        )
    }

    struct ReadyCompany {
        request: CompanyBootstrapRequest,
    }

    #[async_trait]
    impl HostedCompanyProvisioner for ReadyCompany {
        async fn ensure_company(
            &self,
            request: &CompanyBootstrapRequest,
        ) -> Result<bool, HostedRuntimeError> {
            Ok(&self.request == request)
        }
    }

    #[async_trait]
    impl HostedCompanyReadiness for ReadyCompany {
        async fn exact_company_is_ready(
            &self,
            scope: &HostedCompanyScope,
        ) -> Result<bool, HostedRuntimeError> {
            Ok(self.request.owner_id == scope.owner_id
                && self.request.plane_id == scope.plane_id
                && self.request.company_id == scope.company_id
                && self.request.cell_id == scope.cell_id)
        }
    }

    #[async_trait]
    impl HostedRuntimeAdmission for ReadyCompany {
        async fn admit_runtime(
            &self,
            identity: &HostedRuntimeIdentity,
            desired_revision: i64,
        ) -> Result<bool, HostedRuntimeError> {
            Ok(desired_revision == 7
                && identity.owner_id == self.request.owner_id
                && identity.plane_id == self.request.plane_id
                && identity.company_id == self.request.company_id
                && identity.cell_id == self.request.cell_id)
        }
    }

    fn ready_company() -> ReadyCompany {
        ReadyCompany {
            request: company_request(),
        }
    }

    #[test]
    fn network_entry_can_never_select_the_local_docker_driver() {
        assert_eq!(
            RuntimeBackendKind::from_entry_mode(Some("local")).unwrap(),
            RuntimeBackendKind::LocalDocker
        );
        assert_eq!(
            RuntimeBackendKind::from_entry_mode(Some("network")).unwrap(),
            RuntimeBackendKind::HostedRuntimeBridge
        );
        assert_eq!(
            require_local_docker(Some("network")),
            Err(HostedRuntimeError::HostedDockerForbidden)
        );
        assert!(RuntimeBackendKind::from_entry_mode(Some("cloud-ish")).is_err());
    }

    #[test]
    fn hosted_configuration_uses_only_the_reviewed_plane_inputs() {
        let config = plane();
        assert_eq!(config.owner_id, ids().0);
        assert_eq!(config.plane_id, ids().1);
        assert_eq!(config.runtime_image, IMAGE);
        assert!(HostedPlaneConfig::from_values(HostedPlaneValues {
            runtime_image: "restless-company:latest".into(),
            ..HostedPlaneValues {
                owner_id: ids().0.to_string(),
                plane_id: ids().1.to_string(),
                hostname: "owner.core.example.test".into(),
                runtime_image: IMAGE.into(),
                core_source_revision: REVISION.into(),
                bootstrap_token_file: "/run/secrets/runtime_bootstrap_token".into(),
            }
        })
        .is_err());
    }

    #[test]
    fn bootstrap_auth_is_exact_and_never_debug_prints_the_secret() {
        let secret = BootstrapSecret::from_bytes(SECRET.as_bytes().to_vec()).unwrap();
        assert!(secret.authorizes(Some(&format!("Bearer {SECRET}"))));
        assert!(!secret.authorizes(Some(SECRET)));
        assert!(!secret.authorizes(Some(&format!("bearer {SECRET}"))));
        assert!(!secret.authorizes(Some(&format!("Bearer {SECRET}x"))));
        assert_eq!(format!("{secret:?}"), "BootstrapSecret([REDACTED])");
    }

    #[cfg(unix)]
    #[test]
    fn secret_files_must_be_exact_private_regular_files() {
        use std::os::unix::fs::PermissionsExt as _;

        let root = std::env::temp_dir().join(format!(
            "restless-hosted-runtime-secret-test-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(&root).unwrap();
        let bootstrap = root.join("bootstrap");
        fs::write(&bootstrap, SECRET).unwrap();
        fs::set_permissions(&bootstrap, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(BootstrapSecret::read(&bootstrap).is_ok());
        fs::set_permissions(&bootstrap, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            BootstrapSecret::read(&bootstrap),
            Err(HostedRuntimeError::SecretUnavailable)
        );

        let installation = root.join("runtime-capability.key");
        fs::write(&installation, [7; 32]).unwrap();
        fs::set_permissions(&installation, fs::Permissions::from_mode(0o600)).unwrap();
        let first = RuntimeBridgeCapabilityKey::from_installation_key(&installation).unwrap();
        let second = RuntimeBridgeCapabilityKey::from_installation_key(&installation).unwrap();
        assert_eq!(first.0, second.0);
        assert_ne!(first.0, [7; 32]);
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn company_bootstrap_cannot_claim_ready_before_durable_provisioning() {
        let boundary = boundary();
        let auth = format!("Bearer {SECRET}");
        let expected = company_request();
        let ready = ready_company();
        let response = boundary
            .bootstrap_company(Some(&auth), expected.clone(), &ready)
            .await
            .unwrap();
        assert_eq!(response.status, "ready");
        assert_eq!(response.company_id, expected.company_id);

        let unavailable = ReadyCompany {
            request: CompanyBootstrapRequest {
                company_id: Uuid::new_v4(),
                ..expected.clone()
            },
        };
        assert_eq!(
            boundary
                .bootstrap_company(Some(&auth), expected, &unavailable)
                .await,
            Err(HostedRuntimeError::CompanyNotReady)
        );
    }

    #[test]
    fn fleet_runtime_bootstrap_json_shape_is_exact() {
        let request = runtime_request();
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({
                "contract_version": 1,
                "owner_id": request.owner_id,
                "plane_id": request.plane_id,
                "company_id": request.company_id,
                "cell_id": request.cell_id,
                "runtime_id": request.runtime_id,
                "runtime_generation": 1,
                "desired_revision": 7,
                "runtime_image": IMAGE,
                "volume_name": request.volume_name,
                "source_revision": REVISION,
            })
        );
    }

    #[tokio::test]
    async fn runtime_capability_is_bound_to_every_fleet_identity() {
        let boundary = boundary();
        let auth = format!("Bearer {SECRET}");
        boundary
            .validate_company(Some(&auth), company_request())
            .unwrap();
        let now = DateTime::parse_from_rfc3339("2026-09-04T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let response = boundary
            .issue_runtime_capability(Some(&auth), runtime_request(), &ready_company(), now)
            .await
            .unwrap();
        assert_eq!(response.valid_for_seconds, 900);
        let expected = boundary.validate_runtime(runtime_request()).unwrap();
        let grant = boundary
            .capability_key
            .verify(&response.capability, &expected, now + Duration::seconds(1))
            .unwrap();
        assert_eq!(grant.identity, expected);
        assert_eq!(
            grant.identity.core_company_name(),
            "c018f47e04e217a92bb31b1f24221c13c"
        );

        let mut wrong_generation = expected.clone();
        wrong_generation.runtime_generation = 2;
        assert_eq!(
            boundary.capability_key.verify(
                &response.capability,
                &wrong_generation,
                now + Duration::seconds(1),
            ),
            Err(HostedRuntimeError::IdentityMismatch)
        );
    }

    #[tokio::test]
    async fn bootstrap_refuses_wrong_plane_unknown_company_and_mutable_identity() {
        let boundary = boundary();
        let auth = format!("Bearer {SECRET}");
        boundary
            .validate_company(Some(&auth), company_request())
            .unwrap();
        let company = ready_company();
        let now = Utc::now();

        let mut wrong_plane = runtime_request();
        wrong_plane.plane_id = Uuid::new_v4();
        assert_eq!(
            boundary
                .issue_runtime_capability(Some(&auth), wrong_plane, &company, now)
                .await,
            Err(HostedRuntimeError::IdentityMismatch)
        );

        let mut wrong_company = ready_company();
        wrong_company.request.company_id = Uuid::new_v4();
        assert_eq!(
            boundary
                .issue_runtime_capability(Some(&auth), runtime_request(), &wrong_company, now)
                .await,
            Err(HostedRuntimeError::CompanyNotReady)
        );

        let mut mutable = runtime_request();
        mutable.runtime_image = "restless-company:latest".into();
        assert_eq!(
            boundary
                .issue_runtime_capability(Some(&auth), mutable, &company, now)
                .await,
            Err(HostedRuntimeError::IdentityMismatch)
        );
        assert_eq!(
            boundary
                .issue_runtime_capability(None, runtime_request(), &company, now)
                .await,
            Err(HostedRuntimeError::Unauthorized)
        );
    }

    #[test]
    fn capability_expires_and_tampering_is_rejected() {
        let key = RuntimeBridgeCapabilityKey::from_bytes([3; 32]);
        let identity = boundary().validate_runtime(runtime_request()).unwrap();
        let now = Utc::now();
        let capability = key.issue(&identity, now).unwrap();
        assert_eq!(
            key.verify(
                &capability,
                &identity,
                now + Duration::seconds(CAPABILITY_TTL_SECONDS),
            ),
            Err(HostedRuntimeError::CapabilityExpired)
        );
        assert_eq!(
            key.verify(&format!("{capability}x"), &identity, now),
            Err(HostedRuntimeError::CapabilityInvalid)
        );
    }

    #[test]
    fn strict_json_rejects_fields_outside_the_public_bootstrap_contract() {
        let mut value = serde_json::to_value(runtime_request()).unwrap();
        value["supervisor_token"] = serde_json::json!("must-never-enter-core");
        assert!(serde_json::from_value::<RuntimeBootstrapRequest>(value).is_err());
    }
}
