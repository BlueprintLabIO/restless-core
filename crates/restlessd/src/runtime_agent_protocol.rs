//! Public wire contract between a hosted Core account plane and the
//! cell-local Runtime Agent.
//!
//! The agent opens the connection. Nothing in this protocol grants Fleet or
//! the Runtime Supervisor access to company work, and there is deliberately
//! no Docker, host, mount, arbitrary-network, or credential-store operation.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::hosted_runtime::HostedRuntimeIdentity;
use crate::runtime_transport;

pub const RUNTIME_AGENT_PROTOCOL: &str = "restless-runtime-agent.v1";
pub const RUNTIME_AGENT_MAX_FRAME_BYTES: usize = 512 * 1024;
pub const RUNTIME_AGENT_MAX_CHUNK_BYTES: usize = 192 * 1024;
pub const RUNTIME_AGENT_MAX_UPLOAD_BYTES: u64 = 5 * 1024 * 1024;

/// A registration or subsequent message sent by the cell-local agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    content = "body",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeAgentToPlane {
    Register(RuntimeRegistration),
    RegistrationConfirmed(RuntimeRegistrationConfirmed),
    CapabilityRenewed(RuntimeCapabilityRenewalConfirmed),
    Response(RuntimeResponseEnvelope),
    Event(RuntimeEventEnvelope),
}

/// A registration decision or authenticated request sent by the account plane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    content = "body",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimePlaneToAgent {
    Registered(RuntimeRegistrationAccepted),
    Rejected(RuntimeRegistrationRejected),
    RenewCapability(RuntimeCapabilityRenewal),
    Request(RuntimeRequestEnvelope),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRegistration {
    pub protocol: String,
    #[serde(flatten)]
    pub identity: HostedRuntimeIdentity,
    pub desired_revision: i64,
    pub features: Vec<RuntimeAgentFeature>,
    pub capability: RuntimeBridgeCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRegistrationAccepted {
    pub protocol: String,
    pub connection_id: Uuid,
    pub server_time: DateTime<Utc>,
    pub next_session_sequence: u64,
    /// A connection-bound rotation. The agent persists this before it
    /// acknowledges registration and prefers it after restart.
    pub renewed_capability: Option<RuntimeBridgeCapability>,
    pub renewed_capability_expires_at: Option<DateTime<Utc>>,
}

/// Sent only after any in-band rotation has been atomically persisted. The
/// account plane must not expose this connection to Runtime callers before it
/// receives this exact connection-bound acknowledgement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRegistrationConfirmed {
    pub connection_id: Uuid,
    pub persisted_capability: bool,
}

/// In-band rotation of the unconsumed capability reserved for the next
/// reconnect. It is bound to this authenticated connection and exact Runtime
/// identity by the plane's signed capability payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCapabilityRenewal {
    pub connection_id: Uuid,
    pub renewal_id: Uuid,
    pub renewed_capability: RuntimeBridgeCapability,
    pub expires_at: DateTime<Utc>,
}

/// Emitted only after the renewed reconnect grant is atomically durable in
/// the agent-only control volume. The opaque capability is never echoed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCapabilityRenewalConfirmed {
    pub connection_id: Uuid,
    pub renewal_id: Uuid,
    pub persisted_capability: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRegistrationRejected {
    pub code: RuntimeProtocolErrorCode,
    pub retryable: bool,
}

/// Secret-bearing wire value whose debug representation can never disclose
/// the signed registration grant.
#[derive(Clone, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct RuntimeBridgeCapability(String);

impl RuntimeBridgeCapability {
    pub fn new(value: String) -> Result<Self, &'static str> {
        if !(64..=16_384).contains(&value.len())
            || value.contains(char::is_whitespace)
            || !value.is_ascii()
        {
            return Err("Runtime Bridge capability is malformed");
        }
        Ok(Self(value))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RuntimeBridgeCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RuntimeBridgeCapability([REDACTED])")
    }
}

impl<'de> Deserialize<'de> for RuntimeBridgeCapability {
    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAgentFeature {
    ReleaseReadiness,
    GovernedProcess,
    BoundedFiles,
    CompanyServices,
    Activity,
}

impl RuntimeAgentFeature {
    pub const ALL: [Self; 5] = [
        Self::ReleaseReadiness,
        Self::GovernedProcess,
        Self::BoundedFiles,
        Self::CompanyServices,
        Self::Activity,
    ];
}

/// Every post-registration request is independently addressable, bounded by a
/// wall-clock deadline, and ordered within its authenticated connection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRequestEnvelope {
    pub operation_id: Uuid,
    pub deadline: DateTime<Utc>,
    pub session_sequence: u64,
    pub request: RuntimeAgentRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeResponseEnvelope {
    pub operation_id: Uuid,
    pub session_sequence: u64,
    pub response: RuntimeAgentResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEventEnvelope {
    pub operation_id: Option<Uuid>,
    pub event_sequence: u64,
    pub event: RuntimeAgentEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeAgentRequest {
    Readiness,
    ProcessStart(ProcessStartRequest),
    ProcessStdin(ProcessStdinRequest),
    ProcessSignal(ProcessSignalRequest),
    ProcessObserve(ProcessObserveRequest),
    File(FileRequest),
    ServiceOpen(ServiceOpenRequest),
    ServiceWrite(ServiceWriteRequest),
    ServiceClose(ServiceCloseRequest),
    Activity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeAgentResponse {
    Readiness(RuntimeReadiness),
    ProcessStarted(ProcessStarted),
    ProcessInputAccepted(ProcessInputAccepted),
    ProcessSignalAccepted(ProcessSignalAccepted),
    ProcessObserved(ProcessObserved),
    File(FileResponse),
    ServiceOpened(ServiceOpened),
    ServiceWriteAccepted(ServiceWriteAccepted),
    ServiceClosed(ServiceClosed),
    Activity(RuntimeActivity),
    Error(RuntimeProtocolError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeAgentEvent {
    ProcessOutput(ProcessOutput),
    ProcessExited(ProcessExited),
    ServiceOutput(ServiceOutput),
    ServiceClosed(ServiceClosed),
    ActivityChanged(RuntimeActivity),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeReadiness {
    pub protocol: String,
    pub runtime_image: String,
    pub source_revision: String,
    pub core_version: String,
    pub api_contract_version: String,
    pub assertion_contract_version: String,
    pub schema_version: String,
    pub volume_name: String,
    pub runtime_id: String,
    pub runtime_generation: i64,
    pub desired_revision: i64,
    pub ready: bool,
    pub checks: Vec<RuntimeReadinessCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeReadinessCheck {
    pub component: RuntimeReadinessComponent,
    pub status: RuntimeCheckStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeReadinessComponent {
    RuntimeAgent,
    PersistentVolume,
    SessionScratch,
    ProcessExecution,
    Desktop,
    BrowserBroker,
    ReleaseHealth,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeCheckStatus {
    Ready,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessStartRequest {
    pub process_id: Uuid,
    pub authority: ProcessAuthority,
    pub executable: String,
    pub arguments: Vec<String>,
    pub working_directory: RuntimeWorkingDirectory,
    pub environment: BTreeMap<String, SensitiveString>,
    pub stdin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ProcessAuthority {
    Attempt {
        company: String,
        actor: String,
        responsibility: String,
        work_id: Uuid,
        attempt_id: Uuid,
        session_id: String,
    },
    /// Productive execution rooted in an already-durable OrgIntel event when
    /// there is no claimed Work Attempt (for example an Exec wake or a
    /// conversation turn). InfrastructureProbe is never a substitute.
    AuthorityEvent {
        company: String,
        actor: String,
        responsibility: String,
        event_id: i64,
        session_id: String,
    },
    GovernedEffect {
        company: String,
        actor: String,
        effect_class: String,
        authority_id: i64,
        idempotency_key: String,
        execution_no: i32,
        staging_id: Uuid,
        phase: EffectProcessPhase,
    },
    InfrastructureProbe {
        company: String,
        probe: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectProcessPhase {
    WorkspaceAccess,
    ArtifactStage,
    Execute,
    ArtifactCleanup,
    RecoverCompanyProcess,
    RecoverEffectProcess,
}

/// A process cwd is intentionally broader than the bounded owner file API:
/// Core may execute at `/company` or in product-owned trees such as
/// `/company/reviews`, while file requests remain limited to named roots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeWorkingDirectory {
    pub path: String,
}

/// A secret-capable child input whose value is serializable onto the encrypted
/// channel but redacted from all debug and tracing output.
#[derive(Clone, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SensitiveString(String);

impl SensitiveString {
    pub fn new(value: String) -> Result<Self, &'static str> {
        if value.len() > 32 * 1024 || value.contains('\0') {
            return Err("sensitive process input is malformed");
        }
        Ok(Self(value))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SensitiveString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SensitiveString([REDACTED])")
    }
}

impl<'de> Deserialize<'de> for SensitiveString {
    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessStdinRequest {
    pub process_id: Uuid,
    pub data_base64: String,
    pub eof: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessSignalRequest {
    pub process_id: Uuid,
    pub signal: GovernedSignal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessObserveRequest {
    pub process_id: Uuid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GovernedSignal {
    Interrupt,
    Terminate,
    Kill,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessStarted {
    pub process_id: Uuid,
    pub pid: u32,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessInputAccepted {
    pub process_id: Uuid,
    pub decoded_bytes: u32,
    pub eof: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessSignalAccepted {
    pub process_id: Uuid,
    pub signal: GovernedSignal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessObserved {
    pub process_id: Uuid,
    pub pid: u32,
    pub state: ProcessState,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessState {
    Running,
    Exited,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessOutput {
    pub process_id: Uuid,
    pub stream: ProcessStream,
    pub data_base64: String,
    pub eof: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProcessExited {
    pub process_id: Uuid,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FileRequest {
    Stat {
        path: RuntimePath,
    },
    List {
        path: RuntimePath,
        cursor: Option<String>,
        limit: u16,
    },
    Read {
        path: RuntimePath,
        offset: u64,
        max_bytes: u32,
    },
    AtomicWrite {
        path: RuntimePath,
        data_base64: String,
        expected_sha256: Option<String>,
        mode: u32,
    },
    UploadBegin {
        write_id: Uuid,
        path: RuntimePath,
        exact_size: u64,
        exact_sha256: String,
        expected_sha256: Option<String>,
        mode: u32,
    },
    UploadChunk {
        write_id: Uuid,
        offset: u64,
        data_base64: String,
    },
    UploadCommit {
        write_id: Uuid,
    },
    UploadAbort {
        write_id: Uuid,
    },
    Rename {
        from: RuntimePath,
        to: RuntimePath,
        no_replace: bool,
    },
    Digest {
        path: RuntimePath,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FileResponse {
    Stat(RuntimeFileMetadata),
    List(RuntimeFileList),
    Read(RuntimeFileChunk),
    Written(RuntimeFileMutation),
    Renamed(RuntimeFileMutation),
    Digest(RuntimeFileDigest),
    UploadBegun(RuntimeFileUpload),
    UploadChunkAccepted(RuntimeFileUploadProgress),
    UploadAborted { write_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct RuntimePath {
    pub root: RuntimeFileRoot,
    /// Always a slash-separated relative path. Empty means the selected root.
    pub relative: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeFileRoot {
    Org,
    Projects,
    Knowledge,
    Outputs,
    Repos,
    Home,
    Downloads,
    SessionScratch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeFileKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFileMetadata {
    pub path: RuntimePath,
    pub kind: RuntimeFileKind,
    pub size: u64,
    pub modified_at: DateTime<Utc>,
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFileListEntry {
    pub name: String,
    pub kind: RuntimeFileKind,
    pub size: u64,
    pub modified_at: DateTime<Utc>,
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFileList {
    pub path: RuntimePath,
    pub entries: Vec<RuntimeFileListEntry>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFileChunk {
    pub path: RuntimePath,
    pub offset: u64,
    pub data_base64: String,
    pub eof: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFileMutation {
    pub path: RuntimePath,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFileDigest {
    pub path: RuntimePath,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFileUpload {
    pub write_id: Uuid,
    pub path: RuntimePath,
    pub exact_size: u64,
    pub exact_sha256: String,
    pub mode: u32,
    pub next_offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFileUploadProgress {
    pub write_id: Uuid,
    pub accepted_bytes: u32,
    pub next_offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceOpenRequest {
    pub stream_id: Uuid,
    pub service: RuntimeService,
    pub idle_timeout_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceWriteRequest {
    pub stream_id: Uuid,
    pub data_base64: String,
    pub eof: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceCloseRequest {
    pub stream_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeService {
    Desktop,
    BrowserControl,
    ReleaseHealth,
    /// Accepted only when the exact port is present in the agent's local
    /// immutable allow-list. It is never interpreted as a hostname.
    Published {
        port: u16,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceOpened {
    pub stream_id: Uuid,
    pub service: RuntimeService,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceWriteAccepted {
    pub stream_id: Uuid,
    pub decoded_bytes: u32,
    pub eof: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceOutput {
    pub stream_id: Uuid,
    pub data_base64: String,
    pub eof: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServiceClosed {
    pub stream_id: Uuid,
    pub reason: ServiceCloseReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceCloseReason {
    Requested,
    RemoteClosed,
    IdleTimeout,
    TransportError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeActivity {
    pub observed_at: DateTime<Utc>,
    pub processes: Vec<ActiveProcess>,
    pub service_streams: Vec<ActiveServiceStream>,
    pub accepts_new_sessions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActiveProcess {
    pub process_id: Uuid,
    pub pid: u32,
    pub authority: ProcessAuthority,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActiveServiceStream {
    pub stream_id: Uuid,
    pub service: RuntimeService,
    pub opened_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeProtocolError {
    pub code: RuntimeProtocolErrorCode,
    /// Bounded, non-sensitive diagnostic intended for operators and tests.
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProtocolErrorCode {
    InvalidProtocol,
    InvalidIdentity,
    InvalidCapability,
    Expired,
    SequenceViolation,
    OperationConflict,
    OperationPending,
    InvalidRequest,
    ResourceNotFound,
    ResourceExists,
    PermissionDenied,
    LimitExceeded,
    ProcessUnavailable,
    ServiceUnavailable,
    Internal,
}

impl From<&runtime_transport::RuntimeProcessAuthority> for ProcessAuthority {
    fn from(authority: &runtime_transport::RuntimeProcessAuthority) -> Self {
        match authority {
            runtime_transport::RuntimeProcessAuthority::Attempt {
                company,
                actor,
                responsibility,
                work_id,
                attempt_id,
                session_id,
            } => Self::Attempt {
                company: company.clone(),
                actor: actor.clone(),
                responsibility: responsibility.clone(),
                work_id: *work_id,
                attempt_id: *attempt_id,
                session_id: session_id.clone(),
            },
            runtime_transport::RuntimeProcessAuthority::AuthorityEvent {
                company,
                actor,
                responsibility,
                event_id,
                session_id,
            } => Self::AuthorityEvent {
                company: company.clone(),
                actor: actor.clone(),
                responsibility: responsibility.clone(),
                event_id: *event_id,
                session_id: session_id.clone(),
            },
            runtime_transport::RuntimeProcessAuthority::GovernedEffect {
                company,
                actor,
                effect_class,
                authority_id,
                idempotency_key,
                execution_no,
                staging_id,
                phase,
            } => Self::GovernedEffect {
                company: company.clone(),
                actor: actor.clone(),
                effect_class: effect_class.clone(),
                authority_id: *authority_id,
                idempotency_key: idempotency_key.clone(),
                execution_no: *execution_no,
                staging_id: *staging_id,
                phase: (*phase).into(),
            },
            runtime_transport::RuntimeProcessAuthority::InfrastructureProbe { company, probe } => {
                Self::InfrastructureProbe {
                    company: company.clone(),
                    probe: probe.clone(),
                }
            }
        }
    }
}

impl From<&ProcessAuthority> for runtime_transport::RuntimeProcessAuthority {
    fn from(authority: &ProcessAuthority) -> Self {
        match authority {
            ProcessAuthority::Attempt {
                company,
                actor,
                responsibility,
                work_id,
                attempt_id,
                session_id,
            } => Self::Attempt {
                company: company.clone(),
                actor: actor.clone(),
                responsibility: responsibility.clone(),
                work_id: *work_id,
                attempt_id: *attempt_id,
                session_id: session_id.clone(),
            },
            ProcessAuthority::AuthorityEvent {
                company,
                actor,
                responsibility,
                event_id,
                session_id,
            } => Self::AuthorityEvent {
                company: company.clone(),
                actor: actor.clone(),
                responsibility: responsibility.clone(),
                event_id: *event_id,
                session_id: session_id.clone(),
            },
            ProcessAuthority::GovernedEffect {
                company,
                actor,
                effect_class,
                authority_id,
                idempotency_key,
                execution_no,
                staging_id,
                phase,
            } => Self::GovernedEffect {
                company: company.clone(),
                actor: actor.clone(),
                effect_class: effect_class.clone(),
                authority_id: *authority_id,
                idempotency_key: idempotency_key.clone(),
                execution_no: *execution_no,
                staging_id: *staging_id,
                phase: (*phase).into(),
            },
            ProcessAuthority::InfrastructureProbe { company, probe } => Self::InfrastructureProbe {
                company: company.clone(),
                probe: probe.clone(),
            },
        }
    }
}

impl From<runtime_transport::RuntimeEffectPhase> for EffectProcessPhase {
    fn from(phase: runtime_transport::RuntimeEffectPhase) -> Self {
        match phase {
            runtime_transport::RuntimeEffectPhase::WorkspaceAccess => Self::WorkspaceAccess,
            runtime_transport::RuntimeEffectPhase::ArtifactStage => Self::ArtifactStage,
            runtime_transport::RuntimeEffectPhase::Execute => Self::Execute,
            runtime_transport::RuntimeEffectPhase::ArtifactCleanup => Self::ArtifactCleanup,
            runtime_transport::RuntimeEffectPhase::RecoverCompanyProcess => {
                Self::RecoverCompanyProcess
            }
            runtime_transport::RuntimeEffectPhase::RecoverEffectProcess => {
                Self::RecoverEffectProcess
            }
        }
    }
}

impl From<EffectProcessPhase> for runtime_transport::RuntimeEffectPhase {
    fn from(phase: EffectProcessPhase) -> Self {
        match phase {
            EffectProcessPhase::WorkspaceAccess => Self::WorkspaceAccess,
            EffectProcessPhase::ArtifactStage => Self::ArtifactStage,
            EffectProcessPhase::Execute => Self::Execute,
            EffectProcessPhase::ArtifactCleanup => Self::ArtifactCleanup,
            EffectProcessPhase::RecoverCompanyProcess => Self::RecoverCompanyProcess,
            EffectProcessPhase::RecoverEffectProcess => Self::RecoverEffectProcess,
        }
    }
}

impl TryFrom<&runtime_transport::CompanyPath> for RuntimeWorkingDirectory {
    type Error = runtime_transport::RuntimeTransportError;

    fn try_from(path: &runtime_transport::CompanyPath) -> Result<Self, Self::Error> {
        let value = path.as_str();
        if value != "/company" && !value.starts_with("/company/") {
            return Err(runtime_transport::RuntimeTransportError::InvalidRequest(
                "process working directory must be beneath /company",
            ));
        }
        Ok(Self {
            path: value.to_owned(),
        })
    }
}

impl From<runtime_transport::RuntimeSignal> for GovernedSignal {
    fn from(signal: runtime_transport::RuntimeSignal) -> Self {
        match signal {
            runtime_transport::RuntimeSignal::Interrupt => Self::Interrupt,
            runtime_transport::RuntimeSignal::Terminate => Self::Terminate,
            runtime_transport::RuntimeSignal::Kill => Self::Kill,
        }
    }
}

impl From<GovernedSignal> for runtime_transport::RuntimeSignal {
    fn from(signal: GovernedSignal) -> Self {
        match signal {
            GovernedSignal::Interrupt => Self::Interrupt,
            GovernedSignal::Terminate => Self::Terminate,
            GovernedSignal::Kill => Self::Kill,
        }
    }
}

impl From<runtime_transport::RuntimeService> for RuntimeService {
    fn from(service: runtime_transport::RuntimeService) -> Self {
        match service {
            runtime_transport::RuntimeService::Desktop => Self::Desktop,
            runtime_transport::RuntimeService::BrowserControl => Self::BrowserControl,
            runtime_transport::RuntimeService::ReleaseHealth => Self::ReleaseHealth,
            runtime_transport::RuntimeService::Published(port) => Self::Published { port },
        }
    }
}

impl From<RuntimeService> for runtime_transport::RuntimeService {
    fn from(service: RuntimeService) -> Self {
        match service {
            RuntimeService::Desktop => Self::Desktop,
            RuntimeService::BrowserControl => Self::BrowserControl,
            RuntimeService::ReleaseHealth => Self::ReleaseHealth,
            RuntimeService::Published { port } => Self::Published(port),
        }
    }
}

impl TryFrom<&runtime_transport::CompanyPath> for RuntimePath {
    type Error = runtime_transport::RuntimeTransportError;

    fn try_from(path: &runtime_transport::CompanyPath) -> Result<Self, Self::Error> {
        let value = path.as_str();
        let choices = [
            ("/company/org", RuntimeFileRoot::Org),
            ("/company/projects", RuntimeFileRoot::Projects),
            ("/company/knowledge", RuntimeFileRoot::Knowledge),
            ("/company/outputs", RuntimeFileRoot::Outputs),
            ("/company/repos", RuntimeFileRoot::Repos),
            ("/company/home", RuntimeFileRoot::Home),
            ("/company/downloads", RuntimeFileRoot::Downloads),
            ("/company/run/sessions", RuntimeFileRoot::SessionScratch),
        ];
        for (prefix, root) in choices {
            if value == prefix {
                return Ok(Self {
                    root,
                    relative: String::new(),
                });
            }
            if let Some(relative) = value.strip_prefix(&format!("{prefix}/")) {
                return Ok(Self {
                    root,
                    relative: relative.to_owned(),
                });
            }
        }
        Err(runtime_transport::RuntimeTransportError::InvalidRequest(
            "path is outside the Runtime Agent's approved company roots",
        ))
    }
}

impl TryFrom<&RuntimePath> for runtime_transport::CompanyPath {
    type Error = runtime_transport::RuntimeTransportError;

    fn try_from(path: &RuntimePath) -> Result<Self, Self::Error> {
        let prefix = match path.root {
            RuntimeFileRoot::Org => "/company/org",
            RuntimeFileRoot::Projects => "/company/projects",
            RuntimeFileRoot::Knowledge => "/company/knowledge",
            RuntimeFileRoot::Outputs => "/company/outputs",
            RuntimeFileRoot::Repos => "/company/repos",
            RuntimeFileRoot::Home => "/company/home",
            RuntimeFileRoot::Downloads => "/company/downloads",
            RuntimeFileRoot::SessionScratch => "/company/run/sessions",
        };
        let value = if path.relative.is_empty() {
            prefix.to_owned()
        } else {
            format!("{prefix}/{}", path.relative)
        };
        runtime_transport::CompanyPath::parse(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_redacted_but_round_trip_on_the_wire() {
        let capability = RuntimeBridgeCapability::new("a".repeat(64)).unwrap();
        assert_eq!(
            format!("{capability:?}"),
            "RuntimeBridgeCapability([REDACTED])"
        );
        let json = serde_json::to_string(&capability).unwrap();
        assert_eq!(json, format!("\"{}\"", "a".repeat(64)));
        let decoded: RuntimeBridgeCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, capability);
        assert!(serde_json::from_str::<RuntimeBridgeCapability>(r#""short""#).is_err());

        let sensitive = SensitiveString::new("top-secret".to_owned()).unwrap();
        assert_eq!(format!("{sensitive:?}"), "SensitiveString([REDACTED])");
        let oversized = format!("\"{}\"", "x".repeat(32 * 1024 + 1));
        assert!(serde_json::from_str::<SensitiveString>(&oversized).is_err());
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let json = r#"{"operation_id":"00000000-0000-0000-0000-000000000001","deadline":"2030-01-01T00:00:00Z","session_sequence":1,"request":{"kind":"activity"},"extra":true}"#;
        assert!(serde_json::from_str::<RuntimeRequestEnvelope>(json).is_err());
    }

    #[test]
    fn all_features_are_stable_and_unique() {
        let encoded = RuntimeAgentFeature::ALL
            .iter()
            .map(|feature| serde_json::to_string(feature).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(encoded.len(), 5);
        assert_eq!(
            encoded
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            5
        );
    }
}
