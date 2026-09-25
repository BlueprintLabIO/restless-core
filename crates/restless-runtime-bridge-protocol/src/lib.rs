//! Bounded protocol for the outbound hosted Company Runtime bridge.
//!
//! It intentionally contains no generic command or shell frame. Core may
//! preflight the immutable Runtime, supervise one declared agent session,
//! or proxy one existing coordination operation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u32 = 5;
pub const MAX_FRAME_BYTES: usize = 256 * 1024;
pub const MAX_TEXT_BYTES: usize = 128 * 1024;
pub const MAX_JSON_BYTES: usize = 128 * 1024;
pub const MAX_OPERATION_MILLIS: u64 = 30 * 60 * 1_000;
pub const MAX_WORKSPACE_OPERATION_MILLIS: u64 = 60_000;
pub const MAX_GATE_TIMEOUT_SECONDS: u32 = 7_200;
pub const MAX_GATE_ARGC: usize = 128;
pub const MAX_GATE_ARG_BYTES: usize = 8 * 1024;
pub const MAX_GATE_ARGV_BYTES: usize = 64 * 1024;
pub const MAX_GATE_OUTPUT_EXCERPT_BYTES: usize = 8 * 1024;
pub const MAX_EVIDENCE_OPERATION_MILLIS: u64 = 5 * 60 * 1_000;

/// Stable coordinates for one repository-bound Work Attempt. Runtime
/// identity (including generation) is carried by the enclosing `Frame`; this
/// tuple prevents a valid frame from being replayed for another Work revision
/// or checkout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceIdentity {
    pub attempt_id: Uuid,
    pub work_id: Uuid,
    pub work_revision: i64,
    pub repo: String,
    pub worktree: String,
}

/// Content-free Git evidence. Status bytes never cross the bridge: only their
/// digest and entry count do.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceObservation {
    pub repo_path: String,
    pub workdir: String,
    pub source_commit: String,
    pub source_tree: String,
    pub status_digest: String,
    pub dirty_entries: u32,
    pub environment_fingerprint: String,
    pub observed_at_ms: i64,
}

/// Exact, clean terminal Git candidate accepted by Core. Every operation that
/// could turn model output into durable evidence repeats this tuple. The
/// Runtime re-observes it before and after the operation instead of trusting
/// a path or a prior response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct CandidateIdentity {
    pub source_commit: String,
    pub source_tree: String,
    pub status_digest: String,
    pub environment_fingerprint: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum GateResource {
    Port,
    Display,
}

/// A frozen OrgIntel gate definition. This is deliberately argv, never an
/// untyped remote-shell payload. `sh -lc` remains possible only when those are
/// the explicitly governed argv bytes stored on the Work before its Attempt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct GateDefinition {
    pub gate_id: Uuid,
    pub name: String,
    pub sequence_no: i32,
    pub stage: String,
    pub cwd: String,
    pub argv: Vec<String>,
    pub timeout_seconds: u32,
    pub resources: Vec<GateResource>,
}

/// Exact resource coordinates allocated and durably leased by Core/OrgIntel.
/// Runtime consumes these values; it never invents a competing allocator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct GateResources {
    pub holder_token: String,
    pub tempdir: String,
    pub process_group_marker: String,
    pub port: Option<u16>,
    pub display: Option<u16>,
}

pub fn gate_definition_digest(definition: &GateDefinition) -> String {
    use sha2::Digest as _;
    format!(
        "{:x}",
        sha2::Sha256::digest(
            serde_json::to_vec(definition).expect("GateDefinition serialization is infallible")
        )
    )
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateCompletionStatus {
    Conclusive,
    Timeout,
    InfrastructureError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GateReceipt {
    pub candidate: CandidateIdentity,
    pub gate_id: Uuid,
    pub definition_digest: String,
    pub resources: GateResources,
    pub output_digest: String,
    pub output_excerpt: String,
    pub exit_code: Option<i32>,
    pub status: GateCompletionStatus,
    pub duration_ms: u64,
    pub leaked_processes: u32,
    pub passed: bool,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactSelector {
    RepositoryTree,
    WorkspacePath { relative_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReceipt {
    pub candidate: CandidateIdentity,
    pub selector: ArtifactSelector,
    pub uri: String,
    pub content_digest: String,
    pub file_count: u32,
    pub total_bytes: u64,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PromotionReceipt {
    pub candidate: CandidateIdentity,
    pub integration_branch: String,
    pub previous_commit: String,
    pub previous_tree: String,
    pub promoted_commit: String,
    pub promoted_tree: String,
    pub status_digest: String,
    pub reused: bool,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewCopyReceipt {
    pub candidate: CandidateIdentity,
    pub uri: String,
    pub alias_uri: String,
    pub content_digest: String,
    pub file_count: u32,
    pub access_probed: bool,
    pub reused: bool,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AttemptCleanupReceipt {
    pub attempt_id: Uuid,
    pub removed_paths: Vec<String>,
    pub residue_count: u32,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct RuntimeIdentity {
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub company: String,
    pub runtime_id: String,
    pub runtime_generation: i64,
    pub runtime_image: String,
    pub volume_name: String,
    pub source_revision: String,
}

/// Receipt for the one infrastructure operation that creates an initial
/// durable repository. It carries no Work identity because repository custody
/// must exist before repository-bound Work can be commissioned.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RepositoryBootstrapReceipt {
    pub command_id: Uuid,
    pub repo: String,
    pub initial_branch: String,
    pub source_commit: String,
    pub source_tree: String,
    pub status_digest: String,
    pub reused: bool,
    pub observed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseIdentity {
    pub core_version: String,
    pub source_revision: String,
    pub api_contract_version: u32,
    pub assertion_contract_version: u32,
    pub schema_version: i64,
    pub harnesses: std::collections::BTreeMap<String, String>,
    pub harness_agents: std::collections::BTreeMap<String, String>,
    pub harness_dependencies: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Message {
    Register {
        connection_nonce: Uuid,
        release: ReleaseIdentity,
    },
    RegisterAck {
        connection_nonce: Uuid,
        accepted_at_ms: i64,
        heartbeat_interval_ms: u64,
    },
    Heartbeat {
        connection_nonce: Uuid,
        sent_at_ms: i64,
    },
    PreflightRequest {
        operation_id: Uuid,
        deadline_ms: i64,
    },
    PreflightResult {
        operation_id: Uuid,
        status: PreflightStatus,
        release: ReleaseIdentity,
        disk_available_bytes: u64,
        observed_at_ms: i64,
        error: Option<String>,
    },
    CreateRepository {
        operation_id: Uuid,
        command_id: Uuid,
        repo: String,
        initial_branch: String,
        deadline_ms: i64,
    },
    RepositoryCreated {
        operation_id: Uuid,
        receipt: RepositoryBootstrapReceipt,
    },
    PrepareWorkspace {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        requested_source_ref: Option<String>,
        /// Present only when OrgIntel has already frozen this Attempt. Both
        /// object ids must be present together or absent together.
        expected_source_commit: Option<String>,
        expected_source_tree: Option<String>,
        deadline_ms: i64,
    },
    WorkspacePrepared {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        requested_source_ref: Option<String>,
        observation: WorkspaceObservation,
        reused: bool,
    },
    ObserveWorkspace {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        expected_environment_fingerprint: String,
        deadline_ms: i64,
    },
    WorkspaceObserved {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        observation: WorkspaceObservation,
    },
    RunGate {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        candidate: CandidateIdentity,
        definition: GateDefinition,
        definition_digest: String,
        resources: GateResources,
        deadline_ms: i64,
    },
    GateCompleted {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        receipt: GateReceipt,
    },
    ObserveArtifact {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        candidate: CandidateIdentity,
        selector: ArtifactSelector,
        deadline_ms: i64,
    },
    ArtifactObserved {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        receipt: ArtifactReceipt,
    },
    PromoteCandidate {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        candidate: CandidateIdentity,
        integration_branch: String,
        deadline_ms: i64,
    },
    CandidatePromoted {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        receipt: PromotionReceipt,
    },
    PrepareReviewCopy {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        candidate: CandidateIdentity,
        deadline_ms: i64,
    },
    ReviewCopyPrepared {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        receipt: ReviewCopyReceipt,
    },
    CleanupAttempt {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        expected_environment_fingerprint: String,
        deadline_ms: i64,
    },
    AttemptCleaned {
        operation_id: Uuid,
        workspace: WorkspaceIdentity,
        receipt: AttemptCleanupReceipt,
    },
    LaunchAgent {
        operation_id: Uuid,
        session_id: String,
        actor: String,
        responsibility: String,
        harness: String,
        model: String,
        reasoning_effort: String,
        workdir: String,
        prompt: String,
        coordination_capability: String,
        model_capability: String,
        model_url: String,
        /// Ephemeral, exact MCP child environment. Only the first-party Codex
        /// runner needs this; ACP carries MCP environments in its session request.
        mcp_environment: BTreeMap<String, String>,
        deadline_ms: i64,
    },
    AcpStdin {
        operation_id: Uuid,
        data_base64: String,
    },
    AcpStdout {
        operation_id: Uuid,
        data_base64: String,
    },
    Cancel {
        operation_id: Uuid,
        reason: String,
    },
    Exit {
        operation_id: Uuid,
        code: Option<i32>,
        signal: Option<i32>,
        error: Option<String>,
    },
    CoordinationRequest {
        operation_id: Uuid,
        method: CoordinationMethod,
        path: String,
        body: Option<serde_json::Value>,
        deadline_ms: i64,
    },
    CoordinationResponse {
        operation_id: Uuid,
        status_code: u16,
        body: Option<serde_json::Value>,
    },
    Error {
        operation_id: Option<Uuid>,
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreflightStatus {
    Ready,
    Refused,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CoordinationMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub protocol_version: u32,
    pub sequence: u64,
    pub identity: RuntimeIdentity,
    pub message: Message,
}

impl Frame {
    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| ProtocolError::Invalid)?;
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::TooLarge);
        }
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.is_empty() || bytes.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::TooLarge);
        }
        let frame: Self = serde_json::from_slice(bytes).map_err(|_| ProtocolError::Invalid)?;
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.protocol_version != PROTOCOL_VERSION
            || self.sequence == 0
            || self.identity.owner_id.is_nil()
            || self.identity.plane_id.is_nil()
            || self.identity.company_id.is_nil()
            || self.identity.cell_id.is_nil()
            || self.identity.runtime_generation < 1
            || !identifier(&self.identity.company)
            || !identifier(&self.identity.runtime_id)
            || !identifier(&self.identity.volume_name)
            || !bounded(&self.identity.runtime_image, 512)
            || !bounded(&self.identity.source_revision, 64)
        {
            return Err(ProtocolError::Invalid);
        }
        validate_message(&self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    Invalid,
    TooLarge,
    DuplicateOrReordered,
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "invalid Runtime bridge frame",
            Self::TooLarge => "Runtime bridge frame exceeds its bound",
            Self::DuplicateOrReordered => "Runtime bridge frame is duplicate or reordered",
        })
    }
}

impl std::error::Error for ProtocolError {}

#[derive(Debug, Default)]
pub struct SequenceTracker(u64);

impl SequenceTracker {
    pub fn observe(&mut self, sequence: u64) -> Result<(), ProtocolError> {
        if sequence == 0 || sequence <= self.0 {
            return Err(ProtocolError::DuplicateOrReordered);
        }
        self.0 = sequence;
        Ok(())
    }
}

fn validate_message(message: &Message) -> Result<(), ProtocolError> {
    let okay = match message {
        Message::Register {
            connection_nonce,
            release,
        } => !connection_nonce.is_nil() && valid_release(release),
        Message::RegisterAck {
            connection_nonce,
            heartbeat_interval_ms,
            ..
        } => !connection_nonce.is_nil() && (1_000..=60_000).contains(heartbeat_interval_ms),
        Message::Heartbeat {
            connection_nonce, ..
        } => !connection_nonce.is_nil(),
        Message::PreflightRequest {
            operation_id,
            deadline_ms,
        } => !operation_id.is_nil() && valid_deadline(*deadline_ms),
        Message::PreflightResult {
            operation_id,
            release,
            error,
            ..
        } => !operation_id.is_nil() && valid_release(release) && optional_bounded(error, 4_096),
        Message::CreateRepository {
            operation_id,
            command_id,
            repo,
            initial_branch,
            deadline_ms,
        } => {
            !operation_id.is_nil()
                && !command_id.is_nil()
                && operation_id == command_id
                && workspace_slug(repo)
                && valid_git_ref(initial_branch)
                && valid_workspace_deadline(*deadline_ms)
        }
        Message::RepositoryCreated {
            operation_id,
            receipt,
        } => {
            !operation_id.is_nil()
                && *operation_id == receipt.command_id
                && valid_repository_bootstrap_receipt(receipt)
        }
        Message::PrepareWorkspace {
            operation_id,
            workspace,
            requested_source_ref,
            expected_source_commit,
            expected_source_tree,
            deadline_ms,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && requested_source_ref
                    .as_ref()
                    .is_none_or(|value| valid_git_ref(value))
                && match (expected_source_commit, expected_source_tree) {
                    (Some(commit), Some(tree)) => exact_oid(commit) && exact_oid(tree),
                    (None, None) => true,
                    _ => false,
                }
                && valid_workspace_deadline(*deadline_ms)
        }
        Message::WorkspacePrepared {
            operation_id,
            workspace,
            requested_source_ref,
            observation,
            ..
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && requested_source_ref
                    .as_ref()
                    .is_none_or(|value| valid_git_ref(value))
                && valid_workspace_observation(workspace, observation)
        }
        Message::ObserveWorkspace {
            operation_id,
            workspace,
            expected_environment_fingerprint,
            deadline_ms,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && digest(expected_environment_fingerprint)
                && valid_workspace_deadline(*deadline_ms)
        }
        Message::WorkspaceObserved {
            operation_id,
            workspace,
            observation,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && valid_workspace_observation(workspace, observation)
        }
        Message::RunGate {
            operation_id,
            workspace,
            candidate,
            definition,
            definition_digest,
            resources,
            deadline_ms,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && valid_candidate(candidate)
                && valid_gate_definition(definition)
                && *definition_digest == gate_definition_digest(definition)
                && valid_gate_resources(*operation_id, workspace, definition, resources)
                && valid_deadline_with(
                    *deadline_ms,
                    u64::from(definition.timeout_seconds)
                        .saturating_mul(1_000)
                        .saturating_add(30_000),
                )
        }
        Message::GateCompleted {
            operation_id,
            workspace,
            receipt,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && valid_gate_receipt(*operation_id, workspace, receipt)
        }
        Message::ObserveArtifact {
            operation_id,
            workspace,
            candidate,
            selector,
            deadline_ms,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && valid_candidate(candidate)
                && valid_artifact_selector(selector)
                && valid_evidence_deadline(*deadline_ms)
        }
        Message::ArtifactObserved {
            operation_id,
            workspace,
            receipt,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && valid_artifact_receipt(workspace, receipt)
        }
        Message::PromoteCandidate {
            operation_id,
            workspace,
            candidate,
            integration_branch,
            deadline_ms,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && valid_candidate(candidate)
                && valid_git_ref(integration_branch)
                && valid_evidence_deadline(*deadline_ms)
        }
        Message::CandidatePromoted {
            operation_id,
            workspace,
            receipt,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && valid_promotion_receipt(receipt)
        }
        Message::PrepareReviewCopy {
            operation_id,
            workspace,
            candidate,
            deadline_ms,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && valid_candidate(candidate)
                && valid_evidence_deadline(*deadline_ms)
        }
        Message::ReviewCopyPrepared {
            operation_id,
            workspace,
            receipt,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && valid_review_receipt(workspace, receipt)
        }
        Message::CleanupAttempt {
            operation_id,
            workspace,
            expected_environment_fingerprint,
            deadline_ms,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && digest(expected_environment_fingerprint)
                && valid_workspace_deadline(*deadline_ms)
        }
        Message::AttemptCleaned {
            operation_id,
            workspace,
            receipt,
        } => {
            !operation_id.is_nil()
                && valid_workspace_identity(workspace)
                && valid_cleanup_receipt(workspace, receipt)
        }
        Message::LaunchAgent {
            operation_id,
            session_id,
            actor,
            responsibility,
            harness,
            model,
            reasoning_effort,
            workdir,
            prompt,
            coordination_capability,
            model_capability,
            model_url,
            mcp_environment,
            deadline_ms,
        } => {
            !operation_id.is_nil()
                && uuid::Uuid::parse_str(session_id).is_ok_and(|value| !value.is_nil())
                && identifier(actor)
                && bounded(responsibility, 300)
                && identifier(harness)
                && bounded(model, 300)
                && identifier(reasoning_effort)
                && bounded(workdir, 1_024)
                && bounded(prompt, MAX_TEXT_BYTES)
                && bounded(coordination_capability, 16_384)
                && bounded(model_capability, 16_384)
                && bounded(model_url, 2_048)
                && valid_mcp_environment(mcp_environment)
                && valid_deadline(*deadline_ms)
        }
        Message::AcpStdin {
            operation_id,
            data_base64,
        }
        | Message::AcpStdout {
            operation_id,
            data_base64,
        } => !operation_id.is_nil() && bounded(data_base64, MAX_TEXT_BYTES),
        Message::Cancel {
            operation_id,
            reason,
        } => !operation_id.is_nil() && bounded(reason, 4_096),
        Message::Exit {
            operation_id,
            error,
            ..
        } => !operation_id.is_nil() && optional_bounded(error, 4_096),
        Message::CoordinationRequest {
            operation_id,
            path,
            body,
            deadline_ms,
            ..
        } => {
            !operation_id.is_nil()
                && path.starts_with('/')
                && bounded(path, 2_048)
                && body_size(body) <= MAX_JSON_BYTES
                && valid_deadline(*deadline_ms)
        }
        Message::CoordinationResponse {
            operation_id,
            status_code,
            body,
        } => {
            !operation_id.is_nil()
                && (100..=599).contains(status_code)
                && body_size(body) <= MAX_JSON_BYTES
        }
        Message::Error {
            operation_id,
            code,
            message,
        } => {
            operation_id.is_none_or(|id| !id.is_nil())
                && identifier(code)
                && bounded(message, 4_096)
        }
    };
    okay.then_some(()).ok_or(ProtocolError::Invalid)
}

fn valid_release(release: &ReleaseIdentity) -> bool {
    bounded(&release.core_version, 64)
        && bounded(&release.source_revision, 64)
        && release.api_contract_version > 0
        && release.assertion_contract_version > 0
        && release.schema_version > 0
        && release.harnesses.len() <= 64
        && release.harness_agents.len() <= 256
        && release.harness_dependencies.len() <= 256
        && release
            .harnesses
            .iter()
            .chain(&release.harness_agents)
            .chain(&release.harness_dependencies)
            .all(|(key, value)| identifier(key) && bounded(value, 300))
}

fn valid_deadline(deadline_ms: i64) -> bool {
    valid_deadline_with(deadline_ms, MAX_OPERATION_MILLIS)
}

fn valid_deadline_with(deadline_ms: i64, maximum_millis: u64) -> bool {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok());
    now_ms.is_some_and(|now| {
        deadline_ms > now
            && deadline_ms.saturating_sub(now)
                <= i64::try_from(maximum_millis).expect("bounded operation limit")
    })
}

fn valid_workspace_deadline(deadline_ms: i64) -> bool {
    valid_deadline_with(deadline_ms, MAX_WORKSPACE_OPERATION_MILLIS)
}

fn valid_evidence_deadline(deadline_ms: i64) -> bool {
    valid_deadline_with(deadline_ms, MAX_EVIDENCE_OPERATION_MILLIS)
}

fn valid_workspace_identity(workspace: &WorkspaceIdentity) -> bool {
    !workspace.attempt_id.is_nil()
        && !workspace.work_id.is_nil()
        && workspace.work_revision > 0
        && workspace_slug(&workspace.repo)
        && workspace_slug(&workspace.worktree)
}

fn valid_repository_bootstrap_receipt(receipt: &RepositoryBootstrapReceipt) -> bool {
    !receipt.command_id.is_nil()
        && workspace_slug(&receipt.repo)
        && valid_git_ref(&receipt.initial_branch)
        && exact_oid(&receipt.source_commit)
        && exact_oid(&receipt.source_tree)
        && digest(&receipt.status_digest)
        && receipt.observed_at_ms > 0
}

fn valid_workspace_observation(
    workspace: &WorkspaceIdentity,
    observation: &WorkspaceObservation,
) -> bool {
    observation.repo_path == format!("/company/repos/{}", workspace.repo)
        && observation.workdir == format!("/company/worktrees/{}", workspace.worktree)
        && exact_oid(&observation.source_commit)
        && exact_oid(&observation.source_tree)
        && digest(&observation.status_digest)
        && digest(&observation.environment_fingerprint)
        && observation.observed_at_ms > 0
}

fn valid_candidate(candidate: &CandidateIdentity) -> bool {
    exact_oid(&candidate.source_commit)
        && exact_oid(&candidate.source_tree)
        && digest(&candidate.status_digest)
        && digest(&candidate.environment_fingerprint)
}

fn valid_gate_definition(definition: &GateDefinition) -> bool {
    !definition.gate_id.is_nil()
        && bounded(&definition.name, 160)
        && (0..=4_096).contains(&definition.sequence_no)
        && matches!(
            definition.stage.as_str(),
            "focused" | "blind" | "cumulative"
        )
        && valid_company_cwd(&definition.cwd)
        && !definition.argv.is_empty()
        && definition.argv.len() <= MAX_GATE_ARGC
        && !definition.argv[0].is_empty()
        && definition.argv.iter().all(|arg| valid_argv_part(arg))
        && definition.argv.iter().map(String::len).sum::<usize>() <= MAX_GATE_ARGV_BYTES
        && (1..=MAX_GATE_TIMEOUT_SECONDS).contains(&definition.timeout_seconds)
        && definition.resources.len() <= 2
        && {
            let mut resources = definition.resources.clone();
            resources.sort_unstable_by_key(|resource| match resource {
                GateResource::Port => 0,
                GateResource::Display => 1,
            });
            resources.dedup();
            resources.len() == definition.resources.len()
        }
}

fn valid_gate_receipt(
    operation_id: Uuid,
    workspace: &WorkspaceIdentity,
    receipt: &GateReceipt,
) -> bool {
    let root = format!(
        "/company/run/gates/{}/{}",
        workspace.attempt_id.simple(),
        operation_id.simple()
    );
    valid_candidate(&receipt.candidate)
        && !receipt.gate_id.is_nil()
        && digest(&receipt.definition_digest)
        && valid_gate_resources_shape(&receipt.resources)
        && receipt.resources.holder_token == operation_id.simple().to_string()
        && receipt.resources.tempdir == format!("{root}/tmp")
        && receipt.resources.process_group_marker == format!("{root}/process-group.pid")
        && digest(&receipt.output_digest)
        && optional_text(&receipt.output_excerpt, MAX_GATE_OUTPUT_EXCERPT_BYTES)
        && receipt.duration_ms <= u64::from(MAX_GATE_TIMEOUT_SECONDS).saturating_mul(1_000) + 30_000
        && receipt.observed_at_ms > 0
        && match receipt.status {
            GateCompletionStatus::Conclusive => {
                receipt.exit_code.is_some()
                    && (!receipt.passed
                        || (receipt.exit_code == Some(0) && receipt.leaked_processes == 0))
            }
            GateCompletionStatus::Timeout | GateCompletionStatus::InfrastructureError => {
                receipt.exit_code.is_none() && !receipt.passed
            }
        }
}

fn valid_gate_resources(
    operation_id: Uuid,
    workspace: &WorkspaceIdentity,
    definition: &GateDefinition,
    resources: &GateResources,
) -> bool {
    let root = format!(
        "/company/run/gates/{}/{}",
        workspace.attempt_id.simple(),
        operation_id.simple()
    );
    let wants_port = definition.resources.contains(&GateResource::Port);
    let wants_display = definition.resources.contains(&GateResource::Display);
    valid_gate_resources_shape(resources)
        && resources.holder_token == operation_id.simple().to_string()
        && resources.tempdir == format!("{root}/tmp")
        && resources.process_group_marker == format!("{root}/process-group.pid")
        && resources.port.is_some() == wants_port
        && resources.display.is_some() == wants_display
}

fn valid_gate_resources_shape(resources: &GateResources) -> bool {
    identifier(&resources.holder_token)
        && resources.tempdir.starts_with("/company/run/gates/")
        && resources.tempdir.ends_with("/tmp")
        && resources
            .process_group_marker
            .starts_with("/company/run/gates/")
        && resources
            .process_group_marker
            .ends_with("/process-group.pid")
        && resources
            .port
            .is_none_or(|port| (24_000..=49_000).contains(&port))
        && resources
            .display
            .is_none_or(|display| (100..=999).contains(&display))
}

fn valid_artifact_selector(selector: &ArtifactSelector) -> bool {
    match selector {
        ArtifactSelector::RepositoryTree => true,
        ArtifactSelector::WorkspacePath { relative_path } => valid_relative_path(relative_path),
    }
}

fn valid_artifact_receipt(workspace: &WorkspaceIdentity, receipt: &ArtifactReceipt) -> bool {
    let expected_uri = match &receipt.selector {
        ArtifactSelector::RepositoryTree => format!(
            "git:/company/repos/{}#{}",
            workspace.repo, receipt.candidate.source_commit
        ),
        ArtifactSelector::WorkspacePath { relative_path } => format!(
            "/company/worktrees/{}/{}",
            workspace.worktree, relative_path
        ),
    };
    valid_candidate(&receipt.candidate)
        && valid_artifact_selector(&receipt.selector)
        && receipt.uri == expected_uri
        && digest(&receipt.content_digest)
        && receipt.file_count <= 100_000
        && receipt.total_bytes <= 4 * 1024 * 1024 * 1024
        && receipt.observed_at_ms > 0
}

fn valid_promotion_receipt(receipt: &PromotionReceipt) -> bool {
    valid_candidate(&receipt.candidate)
        && valid_git_ref(&receipt.integration_branch)
        && exact_oid(&receipt.previous_commit)
        && exact_oid(&receipt.previous_tree)
        && receipt.promoted_commit == receipt.candidate.source_commit
        && receipt.promoted_tree == receipt.candidate.source_tree
        && digest(&receipt.status_digest)
        && receipt.observed_at_ms > 0
}

fn valid_review_receipt(workspace: &WorkspaceIdentity, receipt: &ReviewCopyReceipt) -> bool {
    valid_candidate(&receipt.candidate)
        && receipt.uri == format!("/company/reviews/git/{}", receipt.candidate.source_commit)
        && receipt.alias_uri
            == format!(
                "/company/reviews/by-attempt/{}",
                workspace.attempt_id.simple()
            )
        && digest(&receipt.content_digest)
        && receipt.file_count <= 100_000
        && receipt.access_probed
        && receipt.observed_at_ms > 0
}

fn valid_cleanup_receipt(workspace: &WorkspaceIdentity, receipt: &AttemptCleanupReceipt) -> bool {
    let attempt_id = workspace.attempt_id.to_string();
    receipt.attempt_id == workspace.attempt_id
        && receipt.removed_paths
            == vec![
                format!("/company/run/attempts/{attempt_id}"),
                format!("/company/run/gates/{attempt_id}"),
            ]
        && receipt.residue_count == 0
        && receipt.observed_at_ms > 0
}

fn valid_company_cwd(value: &str) -> bool {
    value == "@attempt"
        || (value.starts_with("/company")
            && value.len() <= 1_024
            && value.trim() == value
            && !value.chars().any(char::is_control)
            && path_components_are_safe(value.trim_start_matches('/')))
}

fn valid_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1_024
        && value.trim() == value
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.chars().any(char::is_control)
        && path_components_are_safe(value)
}

fn path_components_are_safe(value: &str) -> bool {
    value
        .split('/')
        .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn valid_argv_part(value: &str) -> bool {
    value.len() <= MAX_GATE_ARG_BYTES && !value.as_bytes().contains(&0)
}

fn optional_text(value: &str, max: usize) -> bool {
    value.len() <= max && !value.as_bytes().contains(&0)
}

fn workspace_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn exact_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_git_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.starts_with('-')
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.ends_with('.')
        && !value.ends_with(".lock")
        && !value.contains("..")
        && !value.contains("@{")
        && !value.contains("//")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_mcp_environment(values: &BTreeMap<String, String>) -> bool {
    const RESERVED: &[&str] = &[
        "HOME", "PATH", "CODEX_HOME", "OPENAI_API_KEY", "OPENAI_BASE_URL",
        "RESTLESS_ACTOR", "RESTLESS_COORDINATOR", "RESTLESS_SESSION_CAPABILITY",
        "RESTLESS_MODEL_CAPABILITY", "CLAUDE_CONFIG_DIR", "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL", "CLAUDE_CODE_OAUTH_TOKEN",
        "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY",
        "SSL_CERT_FILE", "NODE_EXTRA_CA_CERTS",
    ];
    values.len() <= 32
        && values.iter().map(|(name, value)| name.len() + value.len()).sum::<usize>() <= 64 * 1024
        && values.iter().all(|(name, value)| {
            !name.is_empty()
                && name.len() <= 128
                && name.bytes().enumerate().all(|(index, byte)| {
                    if index == 0 { byte.is_ascii_alphabetic() || byte == b'_' }
                    else { byte.is_ascii_alphanumeric() || byte == b'_' }
                })
                && !RESERVED.iter().any(|reserved| name.eq_ignore_ascii_case(reserved))
                && !value.is_empty()
                && value.len() <= 8 * 1024
                && !value.contains('\0')
        })
}

fn bounded(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn optional_bounded(value: &Option<String>, max: usize) -> bool {
    value.as_ref().is_none_or(|value| bounded(value, max))
}

fn body_size(value: &Option<serde_json::Value>) -> usize {
    value
        .as_ref()
        .and_then(|value| serde_json::to_vec(value).ok())
        .map_or(0, |bytes| bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_and_oversized_frames_are_refused() {
        let mut tracker = SequenceTracker::default();
        assert!(tracker.observe(1).is_ok());
        assert_eq!(tracker.observe(1), Err(ProtocolError::DuplicateOrReordered));
        assert_eq!(
            Frame::decode(&vec![b'x'; MAX_FRAME_BYTES + 1]),
            Err(ProtocolError::TooLarge)
        );
    }

    fn identity() -> RuntimeIdentity {
        RuntimeIdentity {
            owner_id: Uuid::new_v4(),
            plane_id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            cell_id: Uuid::new_v4(),
            company: "acme".into(),
            runtime_id: "runtime-1".into(),
            runtime_generation: 3,
            runtime_image: format!("runtime@sha256:{}", "a".repeat(64)),
            volume_name: "company-acme".into(),
            source_revision: "b".repeat(40),
        }
    }

    fn workspace() -> WorkspaceIdentity {
        WorkspaceIdentity {
            attempt_id: Uuid::new_v4(),
            work_id: Uuid::new_v4(),
            work_revision: 2,
            repo: "game".into(),
            worktree: "work-123-r2".into(),
        }
    }

    #[test]
    fn repository_bootstrap_is_exact_and_rejects_path_or_operation_drift() {
        let command_id = Uuid::new_v4();
        let frame = Frame {
            protocol_version: PROTOCOL_VERSION,
            sequence: 1,
            identity: identity(),
            message: Message::CreateRepository {
                operation_id: command_id,
                command_id,
                repo: "friend-game".into(),
                initial_branch: "main".into(),
                deadline_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
                    + 30_000,
            },
        };
        assert_eq!(Frame::decode(&frame.encode().unwrap()).unwrap(), frame);

        let mut traversal = frame.clone();
        if let Message::CreateRepository { repo, .. } = &mut traversal.message {
            *repo = "../friend-game".into();
        }
        assert_eq!(traversal.validate(), Err(ProtocolError::Invalid));

        let mut operation_drift = frame;
        if let Message::CreateRepository { operation_id, .. } = &mut operation_drift.message {
            *operation_id = Uuid::new_v4();
        }
        assert_eq!(operation_drift.validate(), Err(ProtocolError::Invalid));
    }

    #[test]
    fn repository_operations_are_exact_and_content_free() {
        let workspace = workspace();
        let frame = Frame {
            protocol_version: PROTOCOL_VERSION,
            sequence: 1,
            identity: identity(),
            message: Message::PrepareWorkspace {
                operation_id: Uuid::new_v4(),
                workspace: workspace.clone(),
                requested_source_ref: Some("refs/heads/main".into()),
                expected_source_commit: Some("c".repeat(40)),
                expected_source_tree: Some("d".repeat(40)),
                deadline_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
                    + 30_000,
            },
        };
        assert_eq!(Frame::decode(&frame.encode().unwrap()).unwrap(), frame);

        let mut invalid = frame.clone();
        if let Message::PrepareWorkspace {
            requested_source_ref,
            ..
        } = &mut invalid.message
        {
            *requested_source_ref = Some("--upload-pack=evil".into());
        }
        assert_eq!(invalid.validate(), Err(ProtocolError::Invalid));

        let observation = WorkspaceObservation {
            repo_path: "/company/repos/game".into(),
            workdir: "/company/worktrees/work-123-r2".into(),
            source_commit: "c".repeat(40),
            source_tree: "d".repeat(40),
            status_digest: "e".repeat(64),
            dirty_entries: 0,
            environment_fingerprint: "f".repeat(64),
            observed_at_ms: 1,
        };
        let response = Frame {
            protocol_version: PROTOCOL_VERSION,
            sequence: 2,
            identity: frame.identity.clone(),
            message: Message::WorkspacePrepared {
                operation_id: Uuid::new_v4(),
                workspace,
                requested_source_ref: Some("refs/heads/main".into()),
                observation,
                reused: false,
            },
        };
        assert!(response.validate().is_ok());
        let encoded = String::from_utf8(response.encode().unwrap()).unwrap();
        assert!(!encoded.contains("status_bytes"));
    }

    #[test]
    fn workspace_result_cannot_escape_company_paths() {
        let workspace = workspace();
        let response = Frame {
            protocol_version: PROTOCOL_VERSION,
            sequence: 1,
            identity: identity(),
            message: Message::WorkspaceObserved {
                operation_id: Uuid::new_v4(),
                workspace: workspace.clone(),
                observation: WorkspaceObservation {
                    repo_path: "/company/repos/game".into(),
                    workdir: "/tmp/escape".into(),
                    source_commit: "a".repeat(40),
                    source_tree: "b".repeat(40),
                    status_digest: "c".repeat(64),
                    dirty_entries: 0,
                    environment_fingerprint: "d".repeat(64),
                    observed_at_ms: 1,
                },
            },
        };
        assert_eq!(response.validate(), Err(ProtocolError::Invalid));
    }

    #[test]
    fn hosted_gate_resources_and_artifact_paths_are_exact() {
        let workspace = workspace();
        let operation_id = Uuid::new_v4();
        let candidate = CandidateIdentity {
            source_commit: "a".repeat(40),
            source_tree: "b".repeat(40),
            status_digest: "c".repeat(64),
            environment_fingerprint: "d".repeat(64),
        };
        let definition = GateDefinition {
            gate_id: Uuid::new_v4(),
            name: "browser-smoke".into(),
            sequence_no: 0,
            stage: "focused".into(),
            cwd: "@attempt".into(),
            argv: vec!["npm".into(), "test".into()],
            timeout_seconds: 120,
            resources: vec![GateResource::Port],
        };
        let root = format!(
            "/company/run/gates/{}/{}",
            workspace.attempt_id.simple(),
            operation_id.simple()
        );
        let resources = GateResources {
            holder_token: operation_id.simple().to_string(),
            tempdir: format!("{root}/tmp"),
            process_group_marker: format!("{root}/process-group.pid"),
            port: Some(31_337),
            display: None,
        };
        let frame = Frame {
            protocol_version: PROTOCOL_VERSION,
            sequence: 1,
            identity: identity(),
            message: Message::RunGate {
                operation_id,
                workspace: workspace.clone(),
                candidate: candidate.clone(),
                definition: definition.clone(),
                definition_digest: gate_definition_digest(&definition),
                resources: resources.clone(),
                deadline_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
                    + 150_000,
            },
        };
        assert!(frame.validate().is_ok());

        let mut wrong_resource = frame.clone();
        if let Message::RunGate { resources, .. } = &mut wrong_resource.message {
            resources.tempdir = "/company/run/gates/another-operation/tmp".into();
        }
        assert_eq!(wrong_resource.validate(), Err(ProtocolError::Invalid));

        let mut wrong_definition = frame;
        if let Message::RunGate { definition, .. } = &mut wrong_definition.message {
            definition.argv.push("--changed-after-freeze".into());
        }
        assert_eq!(wrong_definition.validate(), Err(ProtocolError::Invalid));

        let artifact = Frame {
            protocol_version: PROTOCOL_VERSION,
            sequence: 2,
            identity: identity(),
            message: Message::ObserveArtifact {
                operation_id: Uuid::new_v4(),
                workspace,
                candidate,
                selector: ArtifactSelector::WorkspacePath {
                    relative_path: "dist/index.html".into(),
                },
                deadline_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
                    + 120_000,
            },
        };
        assert!(artifact.validate().is_ok());
        let mut traversal = artifact;
        if let Message::ObserveArtifact { selector, .. } = &mut traversal.message {
            *selector = ArtifactSelector::WorkspacePath {
                relative_path: "dist/../../run/secrets".into(),
            };
        }
        assert_eq!(traversal.validate(), Err(ProtocolError::Invalid));
    }
}
