// GENERATED — do not edit.
//
// Source: crates/restless-orgintel/src/lib.rs (the single writer).
// Regenerate: RESTLESS_WRITE_BINDINGS=1 cargo test -p restless-orgintel
//
// These are OrgIntel rows as they cross the wire, not the owner-surface
// view model. `$lib/model/view.ts` stays hand-written: it is a contract
// in its own right (what the surfaces need), and these are its inputs.

export type JsonValue = number | string | boolean | Array<JsonValue> | { [key in string]: JsonValue } | null;

export type WorkStatus = "proposed" | "active" | "blocked" | "completed" | "abandoned";

export type WorkEdgeKind = "requires" | "revises";

export type WorkAttemptState = "running" | "produced" | "changes_requested" | "blocked" | "failed" | "abandoned" | "superseded";

export type ArtifactRefState = "available" | "stale" | "missing" | "superseded" | "unknown";

export type OwnerHandoffCategory = "identity" | "captcha" | "mfa" | "legal_attestation" | "payment_confirmation" | "owner_judgement";

export type OwnerHandoffState = "pending" | "resolved" | "declined" | "withdrawn";

export type OwnerBriefKind = "outcome_review" | "decision" | "blocker" | "opportunity" | "contradiction" | "human_step";

export type OwnerBrief = { kind: OwnerBriefKind, headline: string, situation: string, impact: string, recommendation: string, no_action: string, uncertainty: string | null, deadline: string | null, };

export type WorkspaceSpec = { repo: string | null, base_ref: string | null, integration_branch: string | null, worktree: string | null, };

export type TeamRow = { id: string, name: string,
/**
 * Why this team exists and what it is accountable for.
 */
brief: string, lead_actor_id: string, created_by: string, created_at: string, disbanded_at: string | null, };

export type ActorRow = { id: string,
/**
 * Small principal class used for filtering and trust/presentation:
 * `owner`, `exec`, `staff`, or `system`.
 */
kind: string,
/**
 * Durable organisational craft/responsibility, separate from actor class
 * and current team relation.
 */
role: string, display: string,
/**
 * NULL means inherited or not applicable, never "unknown".
 */
model: string | null,
/**
 * The team this actor belongs to, or NULL for unassigned. Unassigned is a
 * normal state that surfaces show as such — never a default team (S06-T4).
 */
team_id: string | null,
/**
 * Retirement preserves historical attribution while removing the actor
 * from future staffing. Active-list reads filter this to NULL.
 */
retired_at: string | null,
/**
 * The owner or Exec who made retirement explicit.
 */
retired_by: string | null,
/**
 * Why the actor stopped being available; never inferred from inactivity.
 */
retirement_reason: string, created_at: string, };

export type GoalRow = { id: string, title: string, body: string, created_by: string, created_at: string, closed_at: string | null, };

export type WorkRow = { id: string, goal_id: string | null, owner_id: string, title: string, outcome: string, status: WorkStatus, resolution: string, priority: number, expected_artifact: string,
/**
 * Explicitly opt a Work into the qualified owner-outcome handoff. This is
 * recoverable coordination state, not an implicit consequence of a
 * generic artifact or completion state.
 */
owner_review_required: boolean, repo: string | null, base_ref: string | null, integration_branch: string | null, worktree: string | null, revision: number, attempt_limit: number | null, created_at: string, updated_at: string, };

export type WorkEdgeRow = { from_work_id: string, to_work_id: string, kind: WorkEdgeKind, created_at: string, };

export type WorkAttemptRow = { id: string, work_id: string, revision: number, attempt_no: number, actor_id: string, session_id: string, state: WorkAttemptState, trigger: string, input_fingerprint: string, feedback_cursor: number, requested_source_ref: string | null, source_commit: string | null, source_tree: string | null, gate_set_digest: string, environment_fingerprint: string, materialized_at: string | null, interrupt_requested_at: string | null, interrupt_requested_by: string | null, interrupt_reason: string | null, feedback_checkpoint_cursor: number, model: string | null, started_at: string, finished_at: string | null, summary: string, };

export type WorkAttemptInputRow = { attempt_id: string, artifact_ref_id: string, };

export type WorkAttemptFeedbackRow = { attempt_id: string, message_id: number, };

export type ArtifactRefRow = { id: string, kind: string, uri: string, note: string, created_by: string, work_id: string | null, attempt_id: string | null, digest: string | null, source_commit: string | null, runtime_generation: string | null, label: string, state: ArtifactRefState, created_at: string, superseded_at: string | null, };

export type WorkGateRow = { id: string, work_id: string, name: string, cwd: string, command: JsonValue, created_by: string, sequence_no: number, stage: string, timeout_seconds: number, resources: JsonValue, created_at: string, retired_at: string | null, retired_by: string | null, retired_reason: string | null, };

export type WorkGateRunRow = { id: string, gate_id: string, attempt_id: string, exit_code: number | null, output_digest: string, output_excerpt: string, passed: boolean, candidate_tree: string, definition_digest: string, toolchain_fingerprint: string, status: string, duration_ms: number | null, cache_source_run_id: string | null, leaked_processes: number, ran_at: string, };

export type OwnerHandoffRow = { id: string, work_id: string, attempt_id: string | null, requested_by: string, category: OwnerHandoffCategory, requested_action: string, prepared_state: string, resume_condition: string, state: OwnerHandoffState, resolution: string,
/**
 * Who owes this judgement. `None` is the owner — which is what every row
 * written before S06-T5 means, and why this is nullable rather than
 * defaulted to an actor.
 */
assigned_to: string | null,
/**
 * The actor that held it before it reached the owner. A fall-through must
 * be visible: a lead that silently swallows escalations is the S05-T7
 * single point of failure one level down, with the evidence removed.
 */
escalated_from: string | null, escalated_at: string | null, owner_brief: OwnerBrief | null, briefed_by: string | null, briefed_at: string | null, brief_source_fingerprint: string | null,
/**
 * When this exact pending judgement was last carried into a turn that
 * actually ran for its assignee. `None` means it is still owed and must
 * keep waking that assignee; it is cleared again by reassignment or by a
 * change to the prepared meaning. This gates the wake trigger only — a
 * delivered handoff that is still pending stays in its assignee's context.
 */
delivered_at: string | null, created_at: string, resolved_at: string | null, };

export type WorkGraphSnapshot = { work: Array<WorkRow>, edges: Array<WorkEdgeRow>, attempts: Array<WorkAttemptRow>, attempt_inputs: Array<WorkAttemptInputRow>, attempt_feedback: Array<WorkAttemptFeedbackRow>, artifacts: Array<ArtifactRefRow>, gates: Array<WorkGateRow>, gate_runs: Array<WorkGateRunRow>, handoffs: Array<OwnerHandoffRow>, };

export type ScheduleRow = { id: string, actor_id: string, work_id: string | null, reason: string, fire_at: string, fired_at: string | null, cancelled_at: string | null, created_at: string, };

export type MessageRow = { id: number, from_actor: string, to_actor: string | null, body: string, created_at: string, read_at: string | null, };

export type EventRow = { id: number, kind: string, actor_id: string | null, body: JsonValue, created_at: string, };

