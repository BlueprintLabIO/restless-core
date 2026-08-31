//! Stored OrgIntel rows and input contracts.
//!
//! These are projections of one Postgres owner, re-exported from the crate
//! root so callers keep the existing public facade.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

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
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub summary: String,
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
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct MessageRow {
    pub id: i64,
    pub from_actor: String,
    pub to_actor: Option<String>,
    pub body: String,
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
