//! Stored OrgIntel rows and input contracts.
//!
//! These are projections of one Postgres owner, re-exported from the crate
//! root so callers keep the existing public facade.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

/// The owner-visible ambition promise for one commissioned outcome. It changes
/// accountable judgement posture, never authority, safety floors, or a fixed
/// amount of Runtime machinery.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "outcome_standard", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OutcomeStandard {
    Fast,
    Thorough,
    #[default]
    Exceptional,
    Frontier,
}

impl OutcomeStandard {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Thorough => "thorough",
            Self::Exceptional => "exceptional",
            Self::Frontier => "frontier",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "fast" => Some(Self::Fast),
            "thorough" => Some(Self::Thorough),
            "exceptional" => Some(Self::Exceptional),
            "frontier" => Some(Self::Frontier),
            _ => None,
        }
    }
}

impl std::fmt::Display for OutcomeStandard {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "outcome_standard_source", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OutcomeStandardSource {
    #[default]
    CompanyDefault,
    OwnerOverride,
    OwnerLanguage,
}

impl OutcomeStandardSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CompanyDefault => "company_default",
            Self::OwnerOverride => "owner_override",
            Self::OwnerLanguage => "owner_language",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "company_default" => Some(Self::CompanyDefault),
            "owner_override" => Some(Self::OwnerOverride),
            "owner_language" => Some(Self::OwnerLanguage),
            _ => None,
        }
    }
}

/// The smallest producing shape selected when Work is commissioned. This is
/// a routing fact, not a workflow: one Work still has one accountable producer
/// and one Attempt at a time.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "producing_topology", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProducingTopology {
    #[default]
    CoherentSingleWorker,
    LocallyClosingParallelUnit,
}

impl ProducingTopology {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CoherentSingleWorker => "coherent_single_worker",
            Self::LocallyClosingParallelUnit => "locally_closing_parallel_unit",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "coherent_single_worker" => Some(Self::CoherentSingleWorker),
            "locally_closing_parallel_unit" => Some(Self::LocallyClosingParallelUnit),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProducingTopology {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The Work lifecycle. Migration 0006 renames the former primitive in place;
/// there is no second task or workflow truth beneath it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "work_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WorkStatus {
    Proposed,
    Active,
    Blocked,
    Completed,
    Abandoned,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "work_edge_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WorkEdgeKind {
    Requires,
    Revises,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ts_rs::TS)]
#[sqlx(type_name = "work_attempt_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WorkAttemptState {
    Running,
    Produced,
    ChangesRequested,
    Blocked,
    Failed,
    Abandoned,
    Superseded,
}

impl WorkAttemptState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Produced => "produced",
            Self::ChangesRequested => "changes_requested",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Abandoned => "abandoned",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ts_rs::TS)]
#[sqlx(type_name = "artifact_ref_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRefState {
    Available,
    Stale,
    Missing,
    Superseded,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ts_rs::TS)]
#[sqlx(type_name = "owner_handoff_category", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OwnerHandoffCategory {
    Identity,
    Captcha,
    Mfa,
    LegalAttestation,
    PaymentConfirmation,
    OwnerJudgement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ts_rs::TS)]
#[sqlx(type_name = "owner_handoff_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OwnerHandoffState {
    Pending,
    Resolved,
    Declined,
    Withdrawn,
}

/// The meaning of an organisational owner handoff. This controls presentation
/// and which existing source operation is truthful; it is not a Work or
/// Authority lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum OwnerBriefKind {
    OutcomeReview,
    Decision,
    Blocker,
    Opportunity,
    Contradiction,
    HumanStep,
}

/// A stable executive explanation of one exact handoff source snapshot.
/// Evidence remains in `prepared_state`, artifacts, gates and their owning
/// planes; this is the accountable actor's authored meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct OwnerBrief {
    pub kind: OwnerBriefKind,
    pub headline: String,
    pub situation: String,
    pub impact: String,
    pub recommendation: String,
    pub no_action: String,
    pub uncertainty: Option<String>,
    pub deadline: Option<String>,
}

/// An explicit owner decision on a prepared outcome. Ordinary Work-linked
/// conversation never implies either variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerReviewDecision {
    Accepted,
    ChangesRequested,
}

/// A ReviewTarget is an ordinary artifact reference chosen by the accountable
/// producer. These fixed labels only qualify the one automatic outcome-review
/// path; they are not a new artifact lifecycle or renderer catalogue.
pub const REVIEW_TARGET_ARTIFACT_KIND: &str = "review_target";
pub const REVIEW_TARGET_LIVE_PROBE_GATE: &str = "review-target-live-probe";

/// The exact runtime workspace inherited by every Attempt of a Work node.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
pub struct WorkspaceSpec {
    pub repo: Option<String>,
    pub base_ref: Option<String>,
    pub integration_branch: Option<String>,
    pub worktree: Option<String>,
}

/// Input for one Work node. The node is deliberately a stable outcome
/// contract around flexible model/runtime execution.
pub struct NewWork<'a> {
    pub owner_id: &'a str,
    pub title: &'a str,
    pub outcome: &'a str,
    pub goal_id: Option<Uuid>,
    pub priority: i16,
    pub expected_artifact: &'a str,
    pub workspace: WorkspaceSpec,
    pub attempt_limit: Option<i32>,
}

/// One deterministic check declared in the same transaction as its Work.
/// It runs from the current Attempt workspace, so a revision cannot silently
/// keep checking the prior revision's generated worktree.
pub struct InitialWorkGate<'a> {
    pub name: &'a str,
    pub command: &'a [String],
    pub stage: &'a str,
    pub timeout_seconds: i32,
    pub resources: &'a [String],
}

#[derive(Debug, thiserror::Error)]
pub enum OrgIntelError {
    #[error("invalid company schema name {0:?}")]
    BadSchemaName(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("invalid Work graph: {0}")]
    InvalidWork(String),
}

pub type Result<T> = std::result::Result<T, OrgIntelError>;

#[derive(Debug, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ActorRow {
    pub id: String,
    /// Small principal class used for filtering and trust/presentation:
    /// `owner`, `exec`, `staff`, or `system`.
    pub kind: String,
    /// Durable organisational craft/responsibility, separate from actor class
    /// and current team relation.
    pub role: String,
    pub display: String,
    /// NULL means inherited or not applicable, never "unknown".
    pub model: Option<String>,
    /// The team this actor belongs to, or NULL for unassigned. Unassigned is a
    /// normal state that surfaces show as such — never a default team (S06-T4).
    pub team_id: Option<Uuid>,
    /// Retirement preserves historical attribution while removing the actor
    /// from future staffing. Active-list reads filter this to NULL.
    pub retired_at: Option<DateTime<Utc>>,
    /// The owner or Exec who made retirement explicit.
    pub retired_by: Option<String>,
    /// Why the actor stopped being available; never inferred from inactivity.
    pub retirement_reason: String,
    pub created_at: DateTime<Utc>,
}

/// A group of actors with one accountable lead (S06-T4).
///
/// Coordination state, not kernel truth: recoverable, overridable, repairable.
/// A team grants no effect permission, no budget, no credential scope and no
/// approval right — a lead cannot approve what its members could not, and the
/// owner's approval boundary is unchanged by any team.
#[derive(Debug, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct TeamRow {
    pub id: Uuid,
    pub name: String,
    /// Why this team exists and what it is accountable for.
    pub brief: String,
    pub outcome_standard: OutcomeStandard,
    pub outcome_standard_source: OutcomeStandardSource,
    pub standard_source_message_id: Option<i64>,
    pub lead_actor_id: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub disbanded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct GoalRow {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkRow {
    pub id: Uuid,
    pub goal_id: Option<Uuid>,
    pub owner_id: String,
    pub title: String,
    pub outcome: String,
    pub status: WorkStatus,
    pub resolution: String,
    pub priority: i16,
    pub expected_artifact: String,
    /// Explicitly opt a Work into the qualified owner-outcome handoff. This is
    /// recoverable coordination state, not an implicit consequence of a
    /// generic artifact or completion state.
    pub owner_review_required: bool,
    /// Exact producing route selected at commission. Parallel capacity is
    /// represented as several disjoint unit Work nodes, never several writers
    /// racing inside one node.
    pub producing_topology: ProducingTopology,
    /// Actor that made the durable commission. This can be Exec only for the
    /// unambiguous sole-worker fast path; accountability still belongs to the
    /// worker's team lead.
    pub commissioned_by: String,
    pub repo: Option<String>,
    pub base_ref: Option<String>,
    pub integration_branch: Option<String>,
    pub worktree: Option<String>,
    pub revision: i64,
    pub attempt_limit: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkEdgeRow {
    pub from_work_id: Uuid,
    pub to_work_id: Uuid,
    pub kind: WorkEdgeKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkAttemptRow {
    pub id: Uuid,
    pub work_id: Uuid,
    pub revision: i64,
    pub attempt_no: i32,
    pub actor_id: String,
    pub session_id: String,
    pub state: WorkAttemptState,
    pub trigger: String,
    pub input_fingerprint: String,
    pub feedback_cursor: i64,
    pub requested_source_ref: Option<String>,
    pub source_commit: Option<String>,
    pub source_tree: Option<String>,
    pub terminal_source_commit: Option<String>,
    pub terminal_source_tree: Option<String>,
    pub terminal_status_digest: Option<String>,
    pub terminal_dirty_entries: Option<i32>,
    pub terminal_observed_at: Option<DateTime<Utc>>,
    pub gate_set_digest: String,
    pub environment_fingerprint: String,
    pub materialized_at: Option<DateTime<Utc>>,
    pub interrupt_requested_at: Option<DateTime<Utc>>,
    pub interrupt_requested_by: Option<String>,
    pub interrupt_reason: Option<String>,
    pub feedback_checkpoint_cursor: i64,
    pub model: Option<String>,
    pub harness: Option<String>,
    pub harness_build: Option<String>,
    pub harness_transport: Option<String>,
    #[ts(type = "unknown")]
    pub harness_capabilities: Option<serde_json::Value>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub summary: String,
}

pub struct NewAgentSession<'a> {
    pub launch_id: &'a str,
    pub actor_id: &'a str,
    pub responsibility: &'a str,
    pub work_id: Option<Uuid>,
    pub attempt_id: Option<Uuid>,
    pub harness: &'a str,
    pub harness_build: &'a str,
    pub transport: &'a str,
    pub model: &'a str,
    pub configured_effort: &'a str,
    pub provider_session_id: &'a str,
    pub capabilities: &'a serde_json::Value,
    pub resumed: bool,
    pub reconstructed: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct AgentSessionRow {
    pub launch_id: String,
    pub actor_id: String,
    pub responsibility: String,
    pub work_id: Option<Uuid>,
    pub attempt_id: Option<Uuid>,
    pub harness: String,
    pub harness_build: String,
    pub transport: String,
    pub model: String,
    pub configured_effort: String,
    pub provider_session_id: String,
    #[ts(type = "unknown")]
    pub capabilities: serde_json::Value,
    pub resumed: bool,
    pub reconstructed: bool,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ArtifactRefRow {
    pub id: Uuid,
    pub kind: String,
    pub uri: String,
    pub note: String,
    pub created_by: String,
    pub work_id: Option<Uuid>,
    pub attempt_id: Option<Uuid>,
    pub digest: Option<String>,
    pub source_commit: Option<String>,
    pub runtime_generation: Option<String>,
    pub label: String,
    pub state: ArtifactRefState,
    pub created_at: DateTime<Utc>,
    pub superseded_at: Option<DateTime<Utc>>,
}

pub struct NewArtifactRef<'a> {
    pub kind: &'a str,
    pub uri: &'a str,
    pub note: &'a str,
    pub created_by: &'a str,
    pub work_id: Option<Uuid>,
    pub attempt_id: Option<Uuid>,
    pub digest: Option<&'a str>,
    pub source_commit: Option<&'a str>,
    pub runtime_generation: Option<&'a str>,
    pub label: &'a str,
}

/// Bounded Runtime facts captured when an ACP process ends before it produces
/// a semantic result. These remain observations; the lead judges whether the
/// referenced work is useful.
pub struct NewAttemptRecovery<'a> {
    pub observed_by: &'a str,
    pub reason: &'a str,
    pub workspace: &'a str,
    pub start_observation: &'a serde_json::Value,
    pub end_observation: &'a serde_json::Value,
    pub start_summary: &'a str,
    pub end_summary: &'a str,
    pub changed_since_start: bool,
    pub observation_digest: Option<&'a str>,
    pub end_commit: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttemptRecoveryNotice {
    pub work_id: Uuid,
    pub attempt_id: Uuid,
    pub actor_id: String,
    pub coordinator_id: String,
    pub message_id: i64,
    pub artifact_ref_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct ClaimedWork {
    pub work: WorkRow,
    /// Producer Work this node is explicitly responsible for reviewing and
    /// may return feedback to. An empty set means this is production Work.
    /// Keeping the distinction in the immutable Attempt membrane lets the
    /// Runtime give creators and critics different cognitive jobs without
    /// guessing from actor names or prose.
    pub review_targets: Vec<Uuid>,
    /// The immutable Git starting point for this Attempt. A moving Work ref
    /// such as `main` resolves to the sole exact commit produced by a
    /// completed same-repository prerequisite when one exists.
    pub effective_base_ref: Option<String>,
    pub attempt_id: Uuid,
    pub attempt_no: i32,
    pub session_id: String,
    pub input_fingerprint: String,
    pub inputs: Vec<ArtifactRefRow>,
    pub feedback: Vec<MessageRow>,
    /// The newest prior Attempt in this Work revision, when one exists. The
    /// launch membrane uses this bounded terminal fact to prevent a successor
    /// from repeating context-heavy capture work after provider rejection.
    pub previous_attempt_state: Option<WorkAttemptState>,
    pub previous_attempt_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkGateRow {
    pub id: Uuid,
    pub work_id: Uuid,
    pub name: String,
    pub cwd: String,
    pub command: serde_json::Value,
    pub created_by: String,
    pub sequence_no: i32,
    pub stage: String,
    pub timeout_seconds: i32,
    pub resources: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub retired_at: Option<DateTime<Utc>>,
    pub retired_by: Option<String>,
    pub retired_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkGateRunRow {
    pub id: Uuid,
    pub gate_id: Uuid,
    pub attempt_id: Uuid,
    pub exit_code: Option<i32>,
    pub output_digest: String,
    pub output_excerpt: String,
    pub passed: bool,
    pub candidate_tree: String,
    pub definition_digest: String,
    pub toolchain_fingerprint: String,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub cache_source_run_id: Option<Uuid>,
    pub leaked_processes: i32,
    pub ran_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct RuntimeResourceLeaseRow {
    pub id: Uuid,
    pub attempt_id: Uuid,
    pub gate_id: Option<Uuid>,
    pub kind: String,
    pub value: String,
    pub holder_token: String,
    pub acquired_at: DateTime<Utc>,
    pub released_at: Option<DateTime<Utc>>,
    pub release_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct CandidatePromotionRow {
    pub id: Uuid,
    pub work_id: Uuid,
    pub attempt_id: Uuid,
    pub repo: String,
    pub integration_branch: String,
    pub source_commit: String,
    pub source_tree: String,
    pub manifest: serde_json::Value,
    pub state: String,
    pub failure: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub struct NewCandidatePromotion<'a> {
    pub work_id: Uuid,
    pub attempt_id: Uuid,
    pub repo: &'a str,
    pub integration_branch: &'a str,
    pub source_commit: &'a str,
    pub source_tree: &'a str,
    pub manifest: &'a serde_json::Value,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ImmutableReviewTargetRow {
    pub id: Uuid,
    pub work_id: Uuid,
    pub attempt_id: Uuid,
    pub content_digest: String,
    pub uri: String,
    pub alias_uri: Option<String>,
    pub source_commit: Option<String>,
    pub manifest: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

pub struct NewImmutableReviewTarget<'a> {
    pub work_id: Uuid,
    pub attempt_id: Uuid,
    pub content_digest: &'a str,
    pub uri: &'a str,
    pub alias_uri: Option<&'a str>,
    pub source_commit: Option<&'a str>,
    pub manifest: &'a serde_json::Value,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkAttemptInputRow {
    pub attempt_id: Uuid,
    pub artifact_ref_id: Uuid,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkAttemptFeedbackRow {
    pub attempt_id: Uuid,
    pub message_id: i64,
}

pub struct NewWorkGate<'a> {
    pub work_id: Uuid,
    pub name: &'a str,
    pub cwd: &'a str,
    pub command: &'a [String],
    pub created_by: &'a str,
}

pub struct NewGateRun<'a> {
    pub gate_id: Uuid,
    pub attempt_id: Uuid,
    pub exit_code: Option<i32>,
    pub output_digest: &'a str,
    pub output_excerpt: &'a str,
    pub passed: bool,
}

/// Complete Runtime evidence for a governed gate execution. The older
/// `NewGateRun` remains only for importing historical/manual evidence; new
/// Runtime executions must use this exact-keyed form.
pub struct NewGateRunEvidence<'a> {
    pub gate_id: Uuid,
    pub attempt_id: Uuid,
    pub exit_code: Option<i32>,
    pub output_digest: &'a str,
    pub output_excerpt: &'a str,
    pub passed: bool,
    pub candidate_tree: &'a str,
    pub definition_digest: &'a str,
    pub toolchain_fingerprint: &'a str,
    pub status: &'a str,
    pub duration_ms: Option<i64>,
    pub cache_source_run_id: Option<Uuid>,
    pub leaked_processes: i32,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct OwnerHandoffRow {
    pub id: Uuid,
    pub work_id: Uuid,
    pub attempt_id: Option<Uuid>,
    pub requested_by: String,
    pub category: OwnerHandoffCategory,
    pub requested_action: String,
    pub prepared_state: String,
    pub resume_condition: String,
    pub state: OwnerHandoffState,
    pub resolution: String,
    /// Who owes this judgement. `None` is the owner — which is what every row
    /// written before S06-T5 means, and why this is nullable rather than
    /// defaulted to an actor.
    pub assigned_to: Option<String>,
    /// The actor that held it before it reached the owner. A fall-through must
    /// be visible: a lead that silently swallows escalations is the S05-T7
    /// single point of failure one level down, with the evidence removed.
    pub escalated_from: Option<String>,
    pub escalated_at: Option<DateTime<Utc>>,
    #[sqlx(json(nullable))]
    pub owner_brief: Option<OwnerBrief>,
    pub briefed_by: Option<String>,
    pub briefed_at: Option<DateTime<Utc>>,
    pub brief_source_fingerprint: Option<String>,
    /// When this exact pending judgement was last carried into a turn that
    /// actually ran for its assignee. `None` means it is still owed and must
    /// keep waking that assignee; it is cleared again by reassignment or by a
    /// change to the prepared meaning. This gates the wake trigger only — a
    /// delivered handoff that is still pending stays in its assignee's context.
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl OwnerHandoffRow {
    /// Whether the authored meaning still names this exact mutable source
    /// snapshot. Callers supply the current Work revision they already read.
    pub fn owner_brief_is_current(&self, work_revision: i64) -> bool {
        let current = owner_handoff_source_fingerprint(
            self.work_id,
            self.attempt_id,
            self.category,
            &self.requested_action,
            &self.prepared_state,
            &self.resume_condition,
            work_revision,
        );
        self.owner_brief.is_some()
            && self.brief_source_fingerprint.as_deref() == Some(current.as_str())
    }
}

pub(super) fn validate_owner_brief(brief: &OwnerBrief) -> Result<()> {
    for (name, value) in [
        ("headline", brief.headline.as_str()),
        ("situation", brief.situation.as_str()),
        ("impact", brief.impact.as_str()),
        ("recommendation", brief.recommendation.as_str()),
        ("no-action consequence", brief.no_action.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(format!(
                "owner brief needs a non-empty {name}"
            )));
        }
    }
    if brief
        .uncertainty
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
        || brief
            .deadline
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
    {
        return Err(OrgIntelError::InvalidWork(
            "optional owner brief fields must be omitted rather than blank".into(),
        ));
    }

    if brief.headline.contains(['\n', '\r']) {
        return Err(OrgIntelError::InvalidWork(
            "owner brief headline must be one readable line".into(),
        ));
    }

    let roles = [
        ("situation", brief.situation.trim()),
        ("impact", brief.impact.trim()),
        ("recommendation", brief.recommendation.trim()),
        ("no-action consequence", brief.no_action.trim()),
    ];
    for (index, (left_name, left)) in roles.iter().enumerate() {
        for (right_name, right) in roles.iter().skip(index + 1) {
            if left == right {
                return Err(OrgIntelError::InvalidWork(format!(
                    "owner brief {left_name} and {right_name} repeat the same text; give each field one job"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod owner_brief_validation_tests {
    use super::{validate_owner_brief, OwnerBrief, OwnerBriefKind};

    fn brief() -> OwnerBrief {
        OwnerBrief {
            kind: OwnerBriefKind::Decision,
            headline: "Send the four reviewed emails".into(),
            situation: "The lead has checked the recipients and drafts.".into(),
            impact: "Sending them starts the first customer conversations.".into(),
            recommendation: "Approve this one campaign.".into(),
            no_action: "Nothing is sent; unrelated work continues.".into(),
            uncertainty: None,
            deadline: None,
        }
    }

    #[test]
    fn accepts_distinct_plain_language_roles() {
        validate_owner_brief(&brief()).expect("plain, distinct brief should pass");
    }

    #[test]
    fn rejects_a_multiline_headline() {
        let mut value = brief();
        value.headline = "Send the emails\nafter review".into();
        let error = validate_owner_brief(&value).expect_err("multiline headline should fail");
        assert!(error.to_string().contains("one readable line"));
    }

    #[test]
    fn rejects_exact_repetition_across_semantic_roles() {
        let mut value = brief();
        value.impact = value.situation.clone();
        let error = validate_owner_brief(&value).expect_err("repeated roles should fail");
        assert!(error.to_string().contains("give each field one job"));
    }
}

pub(super) fn owner_handoff_source_fingerprint(
    work_id: Uuid,
    attempt_id: Option<Uuid>,
    category: OwnerHandoffCategory,
    requested_action: &str,
    prepared_state: &str,
    resume_condition: &str,
    work_revision: i64,
) -> String {
    let mut digest = Sha256::new();
    for part in [
        work_id.to_string(),
        attempt_id.map(|id| id.to_string()).unwrap_or_default(),
        format!("{category:?}"),
        requested_action.trim().to_string(),
        prepared_state.trim().to_string(),
        resume_condition.trim().to_string(),
        work_revision.to_string(),
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct WorkGraphSnapshot {
    pub work: Vec<WorkRow>,
    pub edges: Vec<WorkEdgeRow>,
    pub attempts: Vec<WorkAttemptRow>,
    pub attempt_inputs: Vec<WorkAttemptInputRow>,
    pub attempt_feedback: Vec<WorkAttemptFeedbackRow>,
    pub artifacts: Vec<ArtifactRefRow>,
    pub gates: Vec<WorkGateRow>,
    pub gate_runs: Vec<WorkGateRunRow>,
    pub handoffs: Vec<OwnerHandoffRow>,
}

pub struct NewOwnerHandoff<'a> {
    pub work_id: Uuid,
    pub attempt_id: Option<Uuid>,
    pub requested_by: &'a str,
    pub category: OwnerHandoffCategory,
    pub requested_action: &'a str,
    pub prepared_state: &'a str,
    pub resume_condition: &'a str,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ScheduleRow {
    pub id: Uuid,
    pub actor_id: String,
    pub work_id: Option<Uuid>,
    pub reason: String,
    pub fire_at: DateTime<Utc>,
    pub fired_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub recurrence: Option<String>,
    pub timezone: Option<String>,
    pub local_time: Option<chrono::NaiveTime>,
    pub last_fired_at: Option<DateTime<Utc>>,
    pub missed_policy: String,
    pub catch_up_grace_seconds: Option<i64>,
    pub last_missed_at: Option<DateTime<Utc>>,
    pub last_considered_at: Option<DateTime<Utc>>,
    pub machine_requirement: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ScheduleOccurrenceRow {
    pub schedule_id: Uuid,
    pub scheduled_for: DateTime<Utc>,
    pub fired_at: DateTime<Utc>,
    pub disposition: String,
    pub detail: Option<String>,
    pub supersedes_through: Option<DateTime<Utc>>,
    pub superseded_count: i64,
    pub recovered_at: Option<DateTime<Utc>>,
    pub recovery_message_id: Option<i64>,
    pub recovered_by: Option<String>,
    pub recovery_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ScheduleRecoveryRow {
    pub schedule_id: Uuid,
    pub scheduled_for: DateTime<Utc>,
    pub actor_id: String,
    pub message_id: i64,
    pub recovered_at: DateTime<Utc>,
    pub recovered_by: String,
    pub reason: String,
    pub created: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ScheduleRecoveryRetryRow {
    pub schedule_id: Uuid,
    pub scheduled_for: DateTime<Utc>,
    pub actor_id: String,
    pub retry_key: String,
    pub prior_message_id: i64,
    pub message_id: i64,
    pub retried_at: DateTime<Utc>,
    pub retried_by: String,
    pub reason: String,
    pub created: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct MessageRow {
    pub id: i64,
    pub from_actor: String,
    pub to_actor: Option<String>,
    pub body: String,
    /// An explicit owner composer override. Absence means the Exec applies the
    /// company default or makes an attributable natural-language judgement.
    pub outcome_standard: Option<OutcomeStandard>,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

/// One bounded external fact linked to Work through its ordinary message
/// input. The external provider remains authoritative; this row only gives
/// owner and agent projections the exact source reference and bounded context
/// already admitted into OrgIntel.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ExternalMessageSourceRow {
    pub source_ref: String,
    pub message_id: i64,
    pub from_actor: String,
    pub body: String,
    pub provider: String,
    pub provider_event_id: String,
    pub provider_email_id: Option<String>,
    pub provider_message_id: Option<String>,
    pub provider_thread_id: Option<String>,
    pub source_url: Option<String>,
    pub metadata: serde_json::Value,
    pub projected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ConversationFocusRow {
    pub after_message_id: i64,
    pub started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct EventRow {
    pub id: i64,
    pub kind: String,
    pub actor_id: Option<String>,
    pub body: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ---- Company expression identity (S31) ----

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_identity_pillar", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum IdentityPillar {
    Truth,
    Voice,
    Visual,
    Culture,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(
    type_name = "company_identity_statement_kind",
    rename_all = "snake_case"
)]
#[serde(rename_all = "snake_case")]
pub enum IdentityStatementKind {
    Fact,
    Belief,
    Guidance,
    Observation,
    Example,
    Exception,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(
    type_name = "company_identity_evidence_status",
    rename_all = "snake_case"
)]
#[serde(rename_all = "snake_case")]
pub enum IdentityEvidenceStatus {
    Active,
    Disputed,
    Corrected,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_identity_polarity", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum IdentityPolarity {
    Neutral,
    Positive,
    Negative,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(
    type_name = "company_identity_proposal_state",
    rename_all = "snake_case"
)]
#[serde(rename_all = "snake_case")]
pub enum IdentityProposalState {
    Pending,
    Promoted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct IdentityEvidenceRow {
    pub id: Uuid,
    pub pillar: IdentityPillar,
    pub statement_kind: IdentityStatementKind,
    pub claim_key: String,
    pub statement: String,
    pub author_id: String,
    pub source: String,
    pub authority: String,
    pub scope: String,
    pub observed_at: DateTime<Utc>,
    pub evidence_locator: String,
    pub polarity: IdentityPolarity,
    pub status: IdentityEvidenceStatus,
    pub channel: Option<String>,
    pub audience: Option<String>,
    pub supersedes_evidence_id: Option<Uuid>,
    pub exception_expires_at: Option<DateTime<Utc>>,
    pub exception_indefinite: bool,
    pub created_at: DateTime<Utc>,
}

pub struct NewIdentityEvidence<'a> {
    pub pillar: IdentityPillar,
    pub statement_kind: IdentityStatementKind,
    pub claim_key: &'a str,
    pub statement: &'a str,
    pub author_id: &'a str,
    pub source: &'a str,
    pub authority: &'a str,
    pub scope: &'a str,
    pub observed_at: DateTime<Utc>,
    pub evidence_locator: &'a str,
    pub polarity: IdentityPolarity,
    pub status: IdentityEvidenceStatus,
    pub channel: Option<&'a str>,
    pub audience: Option<&'a str>,
    pub supersedes_evidence_id: Option<Uuid>,
    pub exception_expires_at: Option<DateTime<Utc>>,
    pub exception_indefinite: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct IdentityProposalRow {
    pub id: Uuid,
    pub created_by: String,
    pub rationale: String,
    pub expected_predecessor: Option<Uuid>,
    pub state: IdentityProposalState,
    pub decided_by: Option<String>,
    pub authority_record_id: Option<String>,
    pub decision_rationale: String,
    pub created_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct IdentityReleaseRow {
    pub id: Uuid,
    pub predecessor: Option<Uuid>,
    pub effective_from: DateTime<Utc>,
    pub promoted_by: String,
    pub authority_record_id: String,
    pub change_account: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct IdentityWorkBindingRow {
    pub work_id: Uuid,
    pub release_id: Uuid,
    pub bound_at: DateTime<Utc>,
    pub stale_at: Option<DateTime<Utc>>,
    pub stale_reason: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct IdentityProposalEvidenceRow {
    pub proposal_id: Uuid,
    pub evidence_id: Uuid,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct IdentityReleaseEvidenceRow {
    pub release_id: Uuid,
    pub evidence_id: Uuid,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct CompanyIdentitySnapshot {
    pub current_release: Option<IdentityReleaseRow>,
    pub releases: Vec<IdentityReleaseRow>,
    pub pending_proposals: Vec<IdentityProposalRow>,
    pub evidence: Vec<IdentityEvidenceRow>,
    pub proposal_evidence: Vec<IdentityProposalEvidenceRow>,
    pub release_evidence: Vec<IdentityReleaseEvidenceRow>,
    pub bindings: Vec<IdentityWorkBindingRow>,
    pub voice_evidence_details: Vec<VoiceEvidenceDetailRow>,
    pub voice_work_contracts: Vec<VoiceWorkContractRow>,
    pub voice_render_evidence: Vec<VoiceRenderEvidenceRow>,
    pub voice_reviews: Vec<VoiceReviewRow>,
    pub visual_evidence_details: Vec<VisualEvidenceDetailRow>,
    pub visual_work_contracts: Vec<VisualWorkContractRow>,
    pub visual_primitive_uses: Vec<VisualPrimitiveUseRow>,
    pub visual_render_evidence: Vec<VisualRenderEvidenceRow>,
    pub visual_reviews: Vec<VisualReviewRow>,
    pub culture_evidence_details: Vec<CultureEvidenceDetailRow>,
    pub culture_work_contracts: Vec<CultureWorkContractRow>,
    pub culture_case_records: Vec<CultureCaseRecordRow>,
    pub culture_reviews: Vec<CultureReviewRow>,
    pub constitution_artifact_bindings: Vec<ConstitutionArtifactBindingRow>,
    pub constitution_artifact_evidence: Vec<ConstitutionArtifactEvidenceRow>,
    pub constitution_learning_proposals: Vec<ConstitutionLearningProposalRow>,
    pub identity_drift_findings: Vec<IdentityDriftFindingRow>,
    pub identity_migration_decisions: Vec<IdentityMigrationDecisionRow>,
}

pub struct IdentityBriefRequest<'a> {
    pub release_id: Uuid,
    pub outcome: &'a str,
    pub channel: &'a str,
    pub audience: &'a str,
    pub author: &'a str,
    pub max_bytes: usize,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct IdentityBrief {
    pub release_id: Uuid,
    pub outcome: String,
    pub channel: String,
    pub audience: String,
    pub author: String,
    pub body: String,
    pub digest: String,
    pub included_evidence_ids: Vec<Uuid>,
    pub omitted_evidence_ids: Vec<Uuid>,
    pub bytes: usize,
}

// ---- Human company voice (S32) ----

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_voice_evidence_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VoiceEvidenceKind {
    ApprovedPassage,
    RejectedPassage,
    ExpressionPrinciple,
    Vocabulary,
    NamedAuthor,
    ChannelObservation,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_voice_channel", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VoiceChannel {
    Newsletter,
    FounderEmail,
    Support,
    TransactionalEmail,
    ProductUi,
    Blog,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_voice_review_verdict", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VoiceReviewVerdict {
    Accept,
    Revise,
    Reject,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_voice_learning_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VoiceLearningKind {
    Typo,
    FactCorrection,
    VoiceObservation,
}

pub struct NewVoiceEvidence<'a> {
    pub kind: VoiceEvidenceKind,
    pub claim_key: &'a str,
    pub passage_or_principle: &'a str,
    pub author_id: &'a str,
    pub named_author: Option<&'a str>,
    pub source: &'a str,
    pub authority: &'a str,
    pub scope: &'a str,
    pub observed_at: DateTime<Utc>,
    pub evidence_locator: &'a str,
    pub judgement_reason: &'a str,
    pub polarity: IdentityPolarity,
    pub channel: Option<VoiceChannel>,
    pub audience: Option<&'a str>,
    pub supersedes_evidence_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct VoiceEvidenceDetailRow {
    pub evidence_id: Uuid,
    pub kind: VoiceEvidenceKind,
    pub judgement_reason: String,
    pub named_author: Option<String>,
    pub channel: Option<VoiceChannel>,
    pub audience: Option<String>,
}

pub struct NewVoiceWorkContract<'a> {
    pub work_id: Uuid,
    pub channel: VoiceChannel,
    pub author: &'a str,
    pub bound_by: &'a str,
    pub audience: &'a str,
    pub reader_situation: &'a str,
    pub desired_understanding: &'a str,
    pub desired_action: &'a str,
    pub proof: &'a str,
    pub consequence: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct VoiceWorkContractRow {
    pub work_id: Uuid,
    pub release_id: Uuid,
    pub channel: VoiceChannel,
    pub author: String,
    pub bound_by: String,
    pub audience: String,
    pub reader_situation: String,
    pub desired_understanding: String,
    pub desired_action: String,
    pub proof: String,
    pub consequence: String,
    pub contract_digest: String,
    pub bound_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct VoiceContractBrief {
    pub contract: VoiceWorkContractRow,
    pub body: String,
    pub digest: String,
    pub included_evidence_ids: Vec<Uuid>,
    pub omitted_evidence_ids: Vec<Uuid>,
    pub bytes: usize,
}

pub struct NewVoiceRenderEvidence<'a> {
    pub artifact_ref_id: Uuid,
    pub channel: VoiceChannel,
    pub renderer: &'a str,
    pub renderer_version: &'a str,
    pub semantic_checks: &'a serde_json::Value,
    pub captured_by: &'a str,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct VoiceRenderEvidenceRow {
    pub id: Uuid,
    pub artifact_ref_id: Uuid,
    pub channel: VoiceChannel,
    pub renderer: String,
    pub renderer_version: String,
    pub semantic_checks: serde_json::Value,
    pub captured_by: String,
    pub captured_at: DateTime<Utc>,
}

pub struct NewVoiceReview<'a> {
    pub render_evidence_id: Uuid,
    pub reviewer: &'a str,
    pub verdict: VoiceReviewVerdict,
    pub factual_findings: &'a str,
    pub abstraction_findings: &'a str,
    pub repetition_findings: &'a str,
    pub channel_findings: &'a str,
    pub authorship_findings: &'a str,
    pub concepts_removed: &'a str,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct VoiceReviewRow {
    pub id: Uuid,
    pub render_evidence_id: Uuid,
    pub reviewer: String,
    pub verdict: VoiceReviewVerdict,
    pub factual_findings: String,
    pub abstraction_findings: String,
    pub repetition_findings: String,
    pub channel_findings: String,
    pub authorship_findings: String,
    pub concepts_removed: String,
    pub created_at: DateTime<Utc>,
}

pub struct NewVoiceLearningProposal<'a> {
    pub created_by: &'a str,
    pub before_artifact_ref_id: Uuid,
    pub after_artifact_ref_id: Uuid,
    pub change_kind: VoiceLearningKind,
    pub claim_key: &'a str,
    pub observation: &'a str,
    pub motivating_decision: &'a str,
    pub scope: &'a str,
    pub source: &'a str,
    pub evidence_locator: &'a str,
    pub channel: Option<VoiceChannel>,
    pub named_author: Option<&'a str>,
    pub audience: Option<&'a str>,
    pub observed_at: DateTime<Utc>,
}

// ---- Durable visual language (S33) ----

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_visual_evidence_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VisualEvidenceKind {
    SemanticToken,
    TypographyRole,
    CompositionPrinciple,
    ImageryDirection,
    MotionPattern,
    ProductRepresentationRule,
    Primitive,
    ApprovedComposition,
    RejectedExample,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_visual_channel", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VisualChannel {
    LandingPage,
    Email,
    Product,
    Social,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_visual_representation", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VisualRepresentation {
    ExactProduct,
    ClearlyAbstract,
    None,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_visual_motion_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VisualMotionState {
    Full,
    Reduced,
    Static,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_visual_review_verdict", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum VisualReviewVerdict {
    Accept,
    Revise,
    Reject,
}

pub struct NewVisualEvidence<'a> {
    pub kind: VisualEvidenceKind,
    pub claim_key: &'a str,
    pub statement: &'a str,
    pub author_id: &'a str,
    pub source: &'a str,
    pub authority: &'a str,
    pub scope: &'a str,
    pub observed_at: DateTime<Utc>,
    pub evidence_locator: &'a str,
    pub rationale: &'a str,
    pub purpose: &'a str,
    pub polarity: IdentityPolarity,
    pub channel: Option<VisualChannel>,
    pub semantic_role: Option<&'a str>,
    pub value: Option<&'a str>,
    pub reduced_motion_replacement: Option<&'a str>,
    pub product_truth_locator: Option<&'a str>,
    pub origin: Option<&'a str>,
    pub licence: Option<&'a str>,
    pub framework: Option<&'a str>,
    pub dependencies: &'a serde_json::Value,
    pub adaptation_status: Option<&'a str>,
    pub accessibility_notes: &'a str,
    pub supersedes_evidence_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct VisualEvidenceDetailRow {
    pub evidence_id: Uuid,
    pub kind: VisualEvidenceKind,
    pub channel: Option<VisualChannel>,
    pub purpose: String,
    pub rationale: String,
    pub semantic_role: Option<String>,
    pub value: Option<String>,
    pub reduced_motion_replacement: Option<String>,
    pub product_truth_locator: Option<String>,
    pub origin: Option<String>,
    pub licence: Option<String>,
    pub framework: Option<String>,
    pub dependencies: serde_json::Value,
    pub adaptation_status: Option<String>,
    pub accessibility_notes: String,
}

pub struct NewVisualWorkContract<'a> {
    pub work_id: Uuid,
    pub channel: VisualChannel,
    pub bound_by: &'a str,
    pub audience: &'a str,
    pub outcome: &'a str,
    pub information_hierarchy: &'a str,
    pub proof: &'a str,
    pub density: &'a str,
    pub imagery_role: &'a str,
    pub motion_role: &'a str,
    pub product_representation: VisualRepresentation,
    pub product_truth_locator: Option<&'a str>,
    pub requested_departure: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct VisualWorkContractRow {
    pub work_id: Uuid,
    pub release_id: Uuid,
    pub channel: VisualChannel,
    pub bound_by: String,
    pub audience: String,
    pub outcome: String,
    pub information_hierarchy: String,
    pub proof: String,
    pub density: String,
    pub imagery_role: String,
    pub motion_role: String,
    pub product_representation: VisualRepresentation,
    pub product_truth_locator: Option<String>,
    pub requested_departure: Option<String>,
    pub contract_digest: String,
    pub bound_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct VisualDirectionBrief {
    pub contract: VisualWorkContractRow,
    pub body: String,
    pub digest: String,
    pub included_evidence_ids: Vec<Uuid>,
    pub omitted_evidence_ids: Vec<Uuid>,
    pub bytes: usize,
}

pub struct NewVisualPrimitiveUse<'a> {
    pub work_id: Uuid,
    pub evidence_id: Uuid,
    pub primitive_version: &'a str,
    pub purpose: &'a str,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct VisualPrimitiveUseRow {
    pub work_id: Uuid,
    pub evidence_id: Uuid,
    pub primitive_version: String,
    pub purpose: String,
}

pub struct NewVisualRenderEvidence<'a> {
    pub work_id: Uuid,
    pub artifact_ref_id: Uuid,
    pub channel: VisualChannel,
    pub renderer: &'a str,
    pub renderer_version: &'a str,
    pub viewport_width: i32,
    pub viewport_height: i32,
    pub motion_state: VisualMotionState,
    pub native_checks: &'a serde_json::Value,
    pub captured_by: &'a str,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct VisualRenderEvidenceRow {
    pub id: Uuid,
    pub work_id: Uuid,
    pub artifact_ref_id: Uuid,
    pub channel: VisualChannel,
    pub renderer: String,
    pub renderer_version: String,
    pub viewport_width: i32,
    pub viewport_height: i32,
    pub motion_state: VisualMotionState,
    pub native_checks: serde_json::Value,
    pub captured_by: String,
    pub captured_at: DateTime<Utc>,
}

pub struct NewVisualReview<'a> {
    pub render_evidence_id: Uuid,
    pub control_render_evidence_id: Option<Uuid>,
    pub reviewer: &'a str,
    pub verdict: VisualReviewVerdict,
    pub identity_findings: &'a str,
    pub hierarchy_findings: &'a str,
    pub density_findings: &'a str,
    pub proof_findings: &'a str,
    pub product_fidelity_findings: &'a str,
    pub motion_findings: &'a str,
    pub defect_findings: &'a str,
    pub departure_decision: &'a str,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct VisualReviewRow {
    pub id: Uuid,
    pub render_evidence_id: Uuid,
    pub control_render_evidence_id: Option<Uuid>,
    pub reviewer: String,
    pub verdict: VisualReviewVerdict,
    pub identity_findings: String,
    pub hierarchy_findings: String,
    pub density_findings: String,
    pub proof_findings: String,
    pub product_fidelity_findings: String,
    pub motion_findings: String,
    pub defect_findings: String,
    pub departure_decision: String,
    pub created_at: DateTime<Utc>,
}

// ---- Observable operating culture (S34) ----
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_culture_evidence_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CultureEvidenceKind {
    FoundingDecision,
    ObservedConduct,
    Counterexample,
    PromotedNorm,
    BoundedException,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_culture_case", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CultureCase {
    Disagreement,
    UncertainIncident,
    CustomerRecovery,
    QualityTradeoff,
    Hiring,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_culture_confidence", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CultureConfidence {
    Tentative,
    Corroborated,
    OwnerFounded,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(
    type_name = "company_culture_review_verdict",
    rename_all = "snake_case"
)]
#[serde(rename_all = "snake_case")]
pub enum CultureReviewVerdict {
    Accept,
    Revise,
    Reject,
}

pub struct NewCultureEvidence<'a> {
    pub kind: CultureEvidenceKind,
    pub case_kind: Option<CultureCase>,
    pub claim_key: &'a str,
    pub statement: &'a str,
    pub author_id: &'a str,
    pub source: &'a str,
    pub authority: &'a str,
    pub scope: &'a str,
    pub observed_at: DateTime<Utc>,
    pub evidence_locator: &'a str,
    pub polarity: IdentityPolarity,
    pub situation: &'a str,
    pub consequence: &'a str,
    pub actors: &'a str,
    pub decision_authority: &'a str,
    pub conduct: &'a str,
    pub observed_outcome: &'a str,
    pub confidence: CultureConfidence,
    pub counterexample: &'a str,
    pub boundary_conditions: &'a str,
    pub operational_implication: &'a str,
    pub actor_scope: &'a str,
    pub supersedes_evidence_id: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct CultureEvidenceDetailRow {
    pub evidence_id: Uuid,
    pub kind: CultureEvidenceKind,
    pub case_kind: Option<CultureCase>,
    pub situation: String,
    pub consequence: String,
    pub actors: String,
    pub decision_authority: String,
    pub conduct: String,
    pub observed_outcome: String,
    pub confidence: CultureConfidence,
    pub counterexample: String,
    pub boundary_conditions: String,
    pub operational_implication: String,
    pub actor_scope: String,
}
pub struct NewCultureWorkContract<'a> {
    pub work_id: Uuid,
    pub case_kind: CultureCase,
    pub actor: &'a str,
    pub actor_role: &'a str,
    pub team: &'a str,
    pub consequence: &'a str,
    pub decision_boundary: &'a str,
    pub bound_by: &'a str,
}

/// Optional Company Constitution context selected by the accountable lead and
/// committed in the same transaction as new Work. These values describe the
/// communication situation; released evidence remains the source of company
/// policy. Keeping them on Work creation prevents Staff from racing ahead of
/// its Voice, Visual, or Culture context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialVoiceContract {
    pub channel: VoiceChannel,
    pub author: String,
    pub audience: String,
    pub reader_situation: String,
    pub desired_understanding: String,
    pub desired_action: String,
    pub proof: String,
    pub consequence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialVisualContract {
    pub channel: VisualChannel,
    pub audience: String,
    pub outcome: String,
    pub information_hierarchy: String,
    pub proof: String,
    pub density: String,
    pub imagery_role: String,
    pub motion_role: String,
    pub product_representation: VisualRepresentation,
    #[serde(default)]
    pub product_truth_locator: Option<String>,
    #[serde(default)]
    pub requested_departure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialCultureContract {
    pub case_kind: CultureCase,
    pub actor: String,
    pub actor_role: String,
    pub team: String,
    pub consequence: String,
    pub decision_boundary: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialConstitutionContracts {
    #[serde(default)]
    pub voice: Option<InitialVoiceContract>,
    #[serde(default)]
    pub visual: Option<InitialVisualContract>,
    #[serde(default)]
    pub culture: Option<InitialCultureContract>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct CultureWorkContractRow {
    pub work_id: Uuid,
    pub release_id: Uuid,
    pub case_kind: CultureCase,
    pub actor: String,
    pub actor_role: String,
    pub team: String,
    pub consequence: String,
    pub decision_boundary: String,
    pub bound_by: String,
    pub contract_digest: String,
    pub bound_at: DateTime<Utc>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct CulturePostureBrief {
    pub contract: CultureWorkContractRow,
    pub body: String,
    pub digest: String,
    pub included_evidence_ids: Vec<Uuid>,
    pub omitted_evidence_ids: Vec<Uuid>,
    pub bytes: usize,
}
pub struct NewCultureCaseRecord<'a> {
    pub work_id: Uuid,
    pub artifact_ref_id: Uuid,
    pub case_kind: CultureCase,
    pub decision: &'a str,
    pub alternatives: &'a serde_json::Value,
    pub unknowns: &'a str,
    pub correction_of: Option<Uuid>,
    pub correction_account: &'a str,
    pub customer_action: &'a str,
    pub native_checks: &'a serde_json::Value,
    pub recorded_by: &'a str,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct CultureCaseRecordRow {
    pub id: Uuid,
    pub work_id: Uuid,
    pub artifact_ref_id: Uuid,
    pub case_kind: CultureCase,
    pub decision: String,
    pub alternatives: serde_json::Value,
    pub unknowns: String,
    pub correction_of: Option<Uuid>,
    pub correction_account: String,
    pub customer_action: String,
    pub native_checks: serde_json::Value,
    pub recorded_by: String,
    pub recorded_at: DateTime<Utc>,
}
pub struct NewCultureReview<'a> {
    pub case_record_id: Uuid,
    pub reviewer: &'a str,
    pub verdict: CultureReviewVerdict,
    pub conduct_findings: &'a str,
    pub dissent_findings: &'a str,
    pub uncertainty_findings: &'a str,
    pub correction_findings: &'a str,
    pub authority_findings: &'a str,
    pub customer_or_hiring_findings: &'a str,
    pub slogan_recitation_detected: bool,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct CultureReviewRow {
    pub id: Uuid,
    pub case_record_id: Uuid,
    pub reviewer: String,
    pub verdict: CultureReviewVerdict,
    pub conduct_findings: String,
    pub dissent_findings: String,
    pub uncertainty_findings: String,
    pub correction_findings: String,
    pub authority_findings: String,
    pub customer_or_hiring_findings: String,
    pub slogan_recitation_detected: bool,
    pub created_at: DateTime<Utc>,
}

// ---- Executable Company Constitution (S35) ----
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(
    type_name = "company_constitution_learning_trigger",
    rename_all = "snake_case"
)]
#[serde(rename_all = "snake_case")]
pub enum ConstitutionLearningTrigger {
    OwnerEvidence,
    CustomerEvidence,
    ExercisedOutcome,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "company_identity_drift_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum IdentityDriftKind {
    TruthStale,
    VoiceDifference,
    VisualDifference,
    CultureDifference,
    UnknownDependency,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(
    type_name = "company_identity_migration_disposition",
    rename_all = "snake_case"
)]
#[serde(rename_all = "snake_case")]
pub enum IdentityMigrationDisposition {
    Retain,
    Revise,
    Retire,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct ConstitutionPillarAccount {
    pub pillar: IdentityPillar,
    pub status: String,
    pub digest: Option<String>,
    pub bytes: usize,
    pub included_evidence_ids: Vec<Uuid>,
    pub omitted_evidence_ids: Vec<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ts_rs::TS)]
pub struct ConstitutionBrief {
    pub work_id: Uuid,
    pub release_id: Uuid,
    pub body: String,
    pub digest: String,
    pub pillars: Vec<ConstitutionPillarAccount>,
    pub bytes: usize,
}
pub struct NewConstitutionArtifactBinding<'a> {
    pub artifact_ref_id: Uuid,
    pub work_id: Uuid,
    pub channel: &'a str,
    pub audience: &'a str,
    pub named_author: &'a str,
    pub producer: &'a str,
    pub accountable_lead: &'a str,
    pub company_voice: &'a str,
    pub native_evidence: &'a serde_json::Value,
    pub constitution_digest: &'a str,
    pub evidence_ids: &'a [Uuid],
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ConstitutionArtifactBindingRow {
    pub artifact_ref_id: Uuid,
    pub work_id: Uuid,
    pub release_id: Uuid,
    pub channel: String,
    pub audience: String,
    pub named_author: String,
    pub producer: String,
    pub accountable_lead: String,
    pub company_voice: String,
    pub native_evidence: serde_json::Value,
    pub constitution_digest: String,
    pub bound_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ConstitutionArtifactEvidenceRow {
    pub artifact_ref_id: Uuid,
    pub evidence_id: Uuid,
}
pub struct NewConstitutionLearningProposal<'a> {
    pub created_by: &'a str,
    pub evidence_id: Uuid,
    pub pillar: IdentityPillar,
    pub trigger_kind: ConstitutionLearningTrigger,
    pub triggering_event: &'a str,
    pub before_artifact_ref_id: Uuid,
    pub after_artifact_ref_id: Uuid,
    pub scope: &'a str,
    pub contradiction_check: &'a str,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ConstitutionLearningProposalRow {
    pub proposal_id: Uuid,
    pub evidence_id: Uuid,
    pub pillar: IdentityPillar,
    pub trigger_kind: ConstitutionLearningTrigger,
    pub triggering_event: String,
    pub before_artifact_ref_id: Uuid,
    pub after_artifact_ref_id: Uuid,
    pub scope: String,
    pub contradiction_check: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct IdentityDriftFindingRow {
    pub id: Uuid,
    pub artifact_ref_id: Uuid,
    pub from_release_id: Uuid,
    pub to_release_id: Uuid,
    pub kind: IdentityDriftKind,
    pub old_evidence_id: Option<Uuid>,
    pub new_evidence_id: Option<Uuid>,
    pub dependency: String,
    pub consequence: String,
    pub created_at: DateTime<Utc>,
}
pub struct NewIdentityMigrationDecision<'a> {
    pub drift_finding_id: Uuid,
    pub disposition: IdentityMigrationDisposition,
    pub decided_by: &'a str,
    pub rationale: &'a str,
    pub authority_record_id: &'a str,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct IdentityMigrationDecisionRow {
    pub drift_finding_id: Uuid,
    pub disposition: IdentityMigrationDisposition,
    pub decided_by: String,
    pub rationale: String,
    pub authority_record_id: String,
    pub decided_at: DateTime<Utc>,
}
