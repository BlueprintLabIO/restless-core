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

export type OutcomeStandard = "fast" | "thorough" | "exceptional" | "frontier";

export type OutcomeStandardSource = "company_default" | "owner_override" | "owner_language";

export type ProducingTopology = "coherent_single_worker" | "locally_closing_parallel_unit";

export type IdentityPillar = "truth" | "voice" | "visual" | "culture";

export type IdentityStatementKind = "fact" | "belief" | "guidance" | "observation" | "example" | "exception";

export type IdentityEvidenceStatus = "active" | "disputed" | "corrected";

export type IdentityPolarity = "neutral" | "positive" | "negative";

export type IdentityProposalState = "pending" | "promoted" | "rejected";

export type VoiceEvidenceKind = "approved_passage" | "rejected_passage" | "expression_principle" | "vocabulary" | "named_author" | "channel_observation";

export type VoiceChannel = "newsletter" | "founder_email" | "support" | "transactional_email" | "product_ui" | "blog";

export type VoiceReviewVerdict = "accept" | "revise" | "reject";

export type VoiceLearningKind = "typo" | "fact_correction" | "voice_observation";

export type VisualEvidenceKind = "semantic_token" | "typography_role" | "composition_principle" | "imagery_direction" | "motion_pattern" | "product_representation_rule" | "primitive" | "approved_composition" | "rejected_example";

export type VisualChannel = "landing_page" | "email" | "product" | "social";

export type VisualRepresentation = "exact_product" | "clearly_abstract" | "none";

export type VisualMotionState = "full" | "reduced" | "static";

export type VisualReviewVerdict = "accept" | "revise" | "reject";

export type CultureEvidenceKind = "founding_decision" | "observed_conduct" | "counterexample" | "promoted_norm" | "bounded_exception";

export type CultureCase = "disagreement" | "uncertain_incident" | "customer_recovery" | "quality_tradeoff" | "hiring";

export type CultureConfidence = "tentative" | "corroborated" | "owner_founded";

export type CultureReviewVerdict = "accept" | "revise" | "reject";

export type ConstitutionLearningTrigger = "owner_evidence" | "customer_evidence" | "exercised_outcome";

export type IdentityDriftKind = "truth_stale" | "voice_difference" | "visual_difference" | "culture_difference" | "unknown_dependency";

export type IdentityMigrationDisposition = "retain" | "revise" | "retire";

export type OwnerBriefKind = "outcome_review" | "decision" | "blocker" | "opportunity" | "contradiction" | "human_step";

export type OwnerBrief = { kind: OwnerBriefKind, headline: string, situation: string, impact: string, recommendation: string, no_action: string, uncertainty: string | null, deadline: string | null, };

export type WorkspaceSpec = { repo: string | null, base_ref: string | null, integration_branch: string | null, worktree: string | null, };

export type TeamRow = { id: string, name: string,
/**
 * Why this team exists and what it is accountable for.
 */
brief: string, outcome_standard: OutcomeStandard, outcome_standard_source: OutcomeStandardSource, standard_source_message_id: number | null, lead_actor_id: string, created_by: string, created_at: string, disbanded_at: string | null, };

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
owner_review_required: boolean,
/**
 * Exact producing route selected at commission. Parallel capacity is
 * represented as several disjoint unit Work nodes, never several writers
 * racing inside one node.
 */
producing_topology: ProducingTopology,
/**
 * Actor that made the durable commission. This can be Exec only for the
 * unambiguous sole-worker fast path; accountability still belongs to the
 * worker's team lead.
 */
commissioned_by: string, repo: string | null, base_ref: string | null, integration_branch: string | null, worktree: string | null, revision: number, attempt_limit: number | null, created_at: string, updated_at: string, };

export type WorkEdgeRow = { from_work_id: string, to_work_id: string, kind: WorkEdgeKind, created_at: string, };

export type WorkAttemptRow = { id: string, work_id: string, revision: number, attempt_no: number, actor_id: string, session_id: string, state: WorkAttemptState, trigger: string, input_fingerprint: string, feedback_cursor: number, requested_source_ref: string | null, source_commit: string | null, source_tree: string | null, terminal_source_commit: string | null, terminal_source_tree: string | null, terminal_status_digest: string | null, terminal_dirty_entries: number | null, terminal_observed_at: string | null, gate_set_digest: string, environment_fingerprint: string, materialized_at: string | null, interrupt_requested_at: string | null, interrupt_requested_by: string | null, interrupt_reason: string | null, feedback_checkpoint_cursor: number, model: string | null, started_at: string, finished_at: string | null, summary: string, };

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

export type ScheduleRow = { id: string, actor_id: string, work_id: string | null, reason: string, fire_at: string, fired_at: string | null, cancelled_at: string | null, recurrence: string | null, timezone: string | null, local_time: string | null, last_fired_at: string | null, missed_policy: string, catch_up_grace_seconds: number | null, last_missed_at: string | null, last_considered_at: string | null, machine_requirement: string, created_at: string, };

export type MessageRow = { id: number, from_actor: string, to_actor: string | null, body: string,
/**
 * An explicit owner composer override. Absence means the Exec applies the
 * company default or makes an attributable natural-language judgement.
 */
outcome_standard: OutcomeStandard | null, created_at: string, read_at: string | null, };

export type EventRow = { id: number, kind: string, actor_id: string | null, body: JsonValue, created_at: string, };

export type IdentityEvidenceRow = { id: string, pillar: IdentityPillar, statement_kind: IdentityStatementKind, claim_key: string, statement: string, author_id: string, source: string, authority: string, scope: string, observed_at: string, evidence_locator: string, polarity: IdentityPolarity, status: IdentityEvidenceStatus, channel: string | null, audience: string | null, supersedes_evidence_id: string | null, exception_expires_at: string | null, exception_indefinite: boolean, created_at: string, };

export type IdentityProposalRow = { id: string, created_by: string, rationale: string, expected_predecessor: string | null, state: IdentityProposalState, decided_by: string | null, authority_record_id: string | null, decision_rationale: string, created_at: string, decided_at: string | null, };

export type IdentityReleaseRow = { id: string, predecessor: string | null, effective_from: string, promoted_by: string, authority_record_id: string, change_account: string, created_at: string, };

export type IdentityWorkBindingRow = { work_id: string, release_id: string, bound_at: string, stale_at: string | null, stale_reason: string, };

export type IdentityProposalEvidenceRow = { proposal_id: string, evidence_id: string, };

export type IdentityReleaseEvidenceRow = { release_id: string, evidence_id: string, };

export type VoiceEvidenceDetailRow = { evidence_id: string, kind: VoiceEvidenceKind, judgement_reason: string, named_author: string | null, channel: VoiceChannel | null, audience: string | null, };

export type VoiceWorkContractRow = { work_id: string, release_id: string, channel: VoiceChannel, author: string, bound_by: string, audience: string, reader_situation: string, desired_understanding: string, desired_action: string, proof: string, consequence: string, contract_digest: string, bound_at: string, };

export type VoiceRenderEvidenceRow = { id: string, artifact_ref_id: string, channel: VoiceChannel, renderer: string, renderer_version: string, semantic_checks: JsonValue, captured_by: string, captured_at: string, };

export type VoiceReviewRow = { id: string, render_evidence_id: string, reviewer: string, verdict: VoiceReviewVerdict, factual_findings: string, abstraction_findings: string, repetition_findings: string, channel_findings: string, authorship_findings: string, concepts_removed: string, created_at: string, };

export type VisualEvidenceDetailRow = { evidence_id: string, kind: VisualEvidenceKind, channel: VisualChannel | null, purpose: string, rationale: string, semantic_role: string | null, value: string | null, reduced_motion_replacement: string | null, product_truth_locator: string | null, origin: string | null, licence: string | null, framework: string | null, dependencies: JsonValue, adaptation_status: string | null, accessibility_notes: string, };

export type VisualWorkContractRow = { work_id: string, release_id: string, channel: VisualChannel, bound_by: string, audience: string, outcome: string, information_hierarchy: string, proof: string, density: string, imagery_role: string, motion_role: string, product_representation: VisualRepresentation, product_truth_locator: string | null, requested_departure: string | null, contract_digest: string, bound_at: string, };

export type VisualPrimitiveUseRow = { work_id: string, evidence_id: string, primitive_version: string, purpose: string, };

export type VisualRenderEvidenceRow = { id: string, work_id: string, artifact_ref_id: string, channel: VisualChannel, renderer: string, renderer_version: string, viewport_width: number, viewport_height: number, motion_state: VisualMotionState, native_checks: JsonValue, captured_by: string, captured_at: string, };

export type VisualReviewRow = { id: string, render_evidence_id: string, control_render_evidence_id: string | null, reviewer: string, verdict: VisualReviewVerdict, identity_findings: string, hierarchy_findings: string, density_findings: string, proof_findings: string, product_fidelity_findings: string, motion_findings: string, defect_findings: string, departure_decision: string, created_at: string, };

export type CultureEvidenceDetailRow = { evidence_id: string, kind: CultureEvidenceKind, case_kind: CultureCase | null, situation: string, consequence: string, actors: string, decision_authority: string, conduct: string, observed_outcome: string, confidence: CultureConfidence, counterexample: string, boundary_conditions: string, operational_implication: string, actor_scope: string, };

export type CultureWorkContractRow = { work_id: string, release_id: string, case_kind: CultureCase, actor: string, actor_role: string, team: string, consequence: string, decision_boundary: string, bound_by: string, contract_digest: string, bound_at: string, };

export type CultureCaseRecordRow = { id: string, work_id: string, artifact_ref_id: string, case_kind: CultureCase, decision: string, alternatives: JsonValue, unknowns: string, correction_of: string | null, correction_account: string, customer_action: string, native_checks: JsonValue, recorded_by: string, recorded_at: string, };

export type CultureReviewRow = { id: string, case_record_id: string, reviewer: string, verdict: CultureReviewVerdict, conduct_findings: string, dissent_findings: string, uncertainty_findings: string, correction_findings: string, authority_findings: string, customer_or_hiring_findings: string, slogan_recitation_detected: boolean, created_at: string, };

export type ConstitutionArtifactBindingRow = { artifact_ref_id: string, work_id: string, release_id: string, channel: string, audience: string, named_author: string, producer: string, accountable_lead: string, company_voice: string, native_evidence: JsonValue, constitution_digest: string, bound_at: string, };

export type ConstitutionArtifactEvidenceRow = { artifact_ref_id: string, evidence_id: string, };

export type ConstitutionLearningProposalRow = { proposal_id: string, evidence_id: string, pillar: IdentityPillar, trigger_kind: ConstitutionLearningTrigger, triggering_event: string, before_artifact_ref_id: string, after_artifact_ref_id: string, scope: string, contradiction_check: string, created_at: string, };

export type IdentityDriftFindingRow = { id: string, artifact_ref_id: string, from_release_id: string, to_release_id: string, kind: IdentityDriftKind, old_evidence_id: string | null, new_evidence_id: string | null, dependency: string, consequence: string, created_at: string, };

export type IdentityMigrationDecisionRow = { drift_finding_id: string, disposition: IdentityMigrationDisposition, decided_by: string, rationale: string, authority_record_id: string, decided_at: string, };

export type CompanyIdentitySnapshot = { current_release: IdentityReleaseRow | null, releases: Array<IdentityReleaseRow>, pending_proposals: Array<IdentityProposalRow>, evidence: Array<IdentityEvidenceRow>, proposal_evidence: Array<IdentityProposalEvidenceRow>, release_evidence: Array<IdentityReleaseEvidenceRow>, bindings: Array<IdentityWorkBindingRow>, voice_evidence_details: Array<VoiceEvidenceDetailRow>, voice_work_contracts: Array<VoiceWorkContractRow>, voice_render_evidence: Array<VoiceRenderEvidenceRow>, voice_reviews: Array<VoiceReviewRow>, visual_evidence_details: Array<VisualEvidenceDetailRow>, visual_work_contracts: Array<VisualWorkContractRow>, visual_primitive_uses: Array<VisualPrimitiveUseRow>, visual_render_evidence: Array<VisualRenderEvidenceRow>, visual_reviews: Array<VisualReviewRow>, culture_evidence_details: Array<CultureEvidenceDetailRow>, culture_work_contracts: Array<CultureWorkContractRow>, culture_case_records: Array<CultureCaseRecordRow>, culture_reviews: Array<CultureReviewRow>, constitution_artifact_bindings: Array<ConstitutionArtifactBindingRow>, constitution_artifact_evidence: Array<ConstitutionArtifactEvidenceRow>, constitution_learning_proposals: Array<ConstitutionLearningProposalRow>, identity_drift_findings: Array<IdentityDriftFindingRow>, identity_migration_decisions: Array<IdentityMigrationDecisionRow>, };

export type IdentityBrief = { release_id: string, outcome: string, channel: string, audience: string, author: string, body: string, digest: string, included_evidence_ids: Array<string>, omitted_evidence_ids: Array<string>, bytes: number, };

export type VoiceContractBrief = { contract: VoiceWorkContractRow, body: string, digest: string, included_evidence_ids: Array<string>, omitted_evidence_ids: Array<string>, bytes: number, };

export type VisualDirectionBrief = { contract: VisualWorkContractRow, body: string, digest: string, included_evidence_ids: Array<string>, omitted_evidence_ids: Array<string>, bytes: number, };

export type CulturePostureBrief = { contract: CultureWorkContractRow, body: string, digest: string, included_evidence_ids: Array<string>, omitted_evidence_ids: Array<string>, bytes: number, };

export type ConstitutionPillarAccount = { pillar: IdentityPillar, status: string, digest: string | null, bytes: number, included_evidence_ids: Array<string>, omitted_evidence_ids: Array<string>, };

export type ConstitutionBrief = { work_id: string, release_id: string, body: string, digest: string, pillars: Array<ConstitutionPillarAccount>, bytes: number, };

