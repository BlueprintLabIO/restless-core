//! Stable JSON-lines transport types, grouped by the plane that owns each
//! input. The wire remains flat for the existing CLI and company runtime, but
//! the decoder rejects fields outside each command's domain view.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
pub struct CommonInput {
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub as_actor: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub responsibility_id: Option<String>,
    #[serde(default)]
    pub owner_epoch: Option<i64>,
    #[serde(default)]
    pub relation: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<serde_json::Value>,
    #[serde(default)]
    pub area_evidence: Vec<serde_json::Value>,
    #[serde(default)]
    pub version: Option<i32>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub policy: Option<serde_json::Value>,
    #[serde(default)]
    pub work_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct LifecycleInput {
    /// `up --from <live>` clones a company into a throwaway; `down --destroy`
    /// removes the environment and its recoverable coordination state.
    #[serde(default)]
    pub from_company: Option<String>,
    #[serde(default)]
    pub destroy: bool,
    /// Fetch and reconcile the configured Company Runtime image. Building and
    /// publishing the artifact is a release/Fleet concern.
    #[serde(default)]
    pub reconcile: bool,
    /// Host transport that delivered a wake-only schedule hint. It carries no
    /// company or task payload and is accepted only on the local-owner socket.
    #[serde(default)]
    pub adapter: Option<String>,
    /// Bounded wait for a disposable schedule lifecycle probe.
    #[serde(default)]
    pub schedule_test_timeout_seconds: Option<u64>,
    /// Opt in to a synthetic, isolated actor run instead of admission only.
    #[serde(default)]
    pub schedule_test_actor: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct AuthorityInput {
    /// Secret material is forwarded once to the configured credential backend
    /// and never persisted in company config or logs.
    #[serde(default)]
    pub secret_value: Option<String>,
    #[serde(default)]
    pub correction_id: Option<String>,
    #[serde(default)]
    pub request_ids: Vec<String>,
    #[serde(default)]
    pub delta_micro_usd: Option<i64>,
    #[serde(default)]
    pub apply: bool,
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub effect_class: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub artifacts: Option<Vec<String>>,
    #[serde(default)]
    pub secret_bindings: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub party: Option<String>,
    #[serde(default)]
    pub mandate_id: Option<String>,
    #[serde(default)]
    pub mandate_proposal: Option<serde_json::Value>,
    #[serde(default)]
    pub email_request: Option<serde_json::Value>,
    #[serde(default)]
    pub email_observe_list: Option<String>,
    #[serde(default)]
    pub email_observe_after: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InitialWorkGateRequest {
    pub name: String,
    pub command: Vec<String>,
    #[serde(default = "default_gate_stage")]
    pub stage: String,
    #[serde(default = "default_gate_timeout")]
    pub timeout_seconds: i32,
    #[serde(default)]
    pub resources: Vec<String>,
}

fn default_gate_stage() -> String {
    "cumulative".into()
}

fn default_gate_timeout() -> i32 {
    900
}

/// Company skills (Sprint 55). Skill packages stay Runtime files; these fields
/// carry only observations, decisions and activations about them.
#[derive(Debug, Default, Deserialize)]
pub struct SkillInput {
    #[serde(default)]
    pub skill: Option<String>,
    #[serde(default)]
    pub skill_digest: Option<String>,
    #[serde(default)]
    pub observed_skills: Vec<restless_orgintel::ObservedSkill>,
    #[serde(default)]
    pub observed_skill: Option<restless_orgintel::ObservedSkill>,
    #[serde(default)]
    pub origin_url: Option<String>,
    #[serde(default)]
    pub origin_ref: Option<String>,
    #[serde(default)]
    pub disposition: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub scope_id: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub skill_work_id: Option<String>,
    #[serde(default)]
    pub skill_attempt_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OrgIntelInput {
    #[serde(default)]
    pub include_retired: bool,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub new_name: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub producing_topology: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub priority: Option<i16>,
    #[serde(default)]
    pub expected_artifact: Option<String>,
    #[serde(default)]
    pub base_ref: Option<String>,
    #[serde(default)]
    pub integration_branch: Option<String>,
    #[serde(default)]
    pub worktree: Option<String>,
    #[serde(default)]
    pub attempt_limit: Option<i32>,
    /// Opt this Work into the qualified owner-outcome path. Completion then
    /// requires one ReviewTarget artifact and its named live-probe gate.
    #[serde(default)]
    pub owner_review: bool,
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub source_message_id: Option<i64>,
    /// Exact recurring Opportunity provenance for atomic Work commissioning.
    #[serde(default)]
    pub opportunity_id: Option<String>,
    #[serde(default)]
    pub schedule_id: Option<String>,
    #[serde(default)]
    pub outcome_standard: Option<String>,
    #[serde(default)]
    pub outcome_standard_source: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub revises: Vec<String>,
    #[serde(default)]
    pub gates: Vec<InitialWorkGateRequest>,
    #[serde(default)]
    pub constitution_contracts: Option<restless_orgintel::InitialConstitutionContracts>,
    /// Company skills selected for new Work; pinned atomically with it.
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub attempt: Option<String>,
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub source_commit: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub argv: Option<Vec<String>>,
    #[serde(default)]
    pub stage: Option<String>,
    #[serde(default)]
    pub timeout_seconds: Option<i32>,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub fire_at: Option<String>,
    #[serde(default)]
    pub recurrence: Option<String>,
    /// `/loop`: the bounded interval for an `interval` recurrence.
    #[serde(default)]
    pub interval_seconds: Option<i32>,
    #[serde(default)]
    pub local_time: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub missed_policy: Option<String>,
    #[serde(default)]
    pub catch_up_grace_seconds: Option<i64>,
    #[serde(default)]
    pub execution_requirement: Option<String>,
    #[serde(default)]
    pub retry_key: Option<String>,
    #[serde(default)]
    pub prior_message_id: Option<i64>,
    #[serde(default)]
    pub include_fired: bool,
    #[serde(default)]
    pub identity_pillar: Option<String>,
    #[serde(default)]
    pub identity_kind: Option<String>,
    #[serde(default)]
    pub claim_key: Option<String>,
    #[serde(default)]
    pub statement: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub identity_authority: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub evidence_locator: Option<String>,
    #[serde(default)]
    pub polarity: Option<String>,
    #[serde(default)]
    pub evidence_status: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub audience: Option<String>,
    #[serde(default)]
    pub supersedes: Option<String>,
    #[serde(default)]
    pub exception_expires_at: Option<String>,
    #[serde(default)]
    pub exception_indefinite: bool,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub release_id: Option<String>,
    #[serde(default)]
    pub max_bytes: Option<usize>,
    #[serde(default)]
    pub voice_kind: Option<String>,
    #[serde(default)]
    pub named_author: Option<String>,
    #[serde(default)]
    pub voice_author: Option<String>,
    #[serde(default)]
    pub judgement_reason: Option<String>,
    #[serde(default)]
    pub voice_work_id: Option<String>,
    #[serde(default)]
    pub reader_situation: Option<String>,
    #[serde(default)]
    pub desired_understanding: Option<String>,
    #[serde(default)]
    pub desired_action: Option<String>,
    #[serde(default)]
    pub proof: Option<String>,
    #[serde(default)]
    pub consequence: Option<String>,
    #[serde(default)]
    pub artifact_ref_id: Option<String>,
    #[serde(default)]
    pub renderer: Option<String>,
    #[serde(default)]
    pub renderer_version: Option<String>,
    #[serde(default)]
    pub semantic_checks: Option<serde_json::Value>,
    #[serde(default)]
    pub render_evidence_id: Option<String>,
    #[serde(default)]
    pub review_verdict: Option<String>,
    #[serde(default)]
    pub factual_findings: Option<String>,
    #[serde(default)]
    pub abstraction_findings: Option<String>,
    #[serde(default)]
    pub repetition_findings: Option<String>,
    #[serde(default)]
    pub channel_findings: Option<String>,
    #[serde(default)]
    pub authorship_findings: Option<String>,
    #[serde(default)]
    pub concepts_removed: Option<String>,
    #[serde(default)]
    pub before_artifact_ref_id: Option<String>,
    #[serde(default)]
    pub after_artifact_ref_id: Option<String>,
    #[serde(default)]
    pub learning_kind: Option<String>,
    #[serde(default)]
    pub observation: Option<String>,
    #[serde(default)]
    pub motivating_decision: Option<String>,
    #[serde(default)]
    pub visual_kind: Option<String>,
    #[serde(default)]
    pub visual_work_id: Option<String>,
    #[serde(default)]
    pub visual_purpose: Option<String>,
    #[serde(default)]
    pub visual_rationale: Option<String>,
    #[serde(default)]
    pub accessibility_notes: Option<String>,
    #[serde(default)]
    pub reduced_motion_replacement: Option<String>,
    #[serde(default)]
    pub product_truth_locator: Option<String>,
    #[serde(default)]
    pub primitive_origin: Option<String>,
    #[serde(default)]
    pub primitive_licence: Option<String>,
    #[serde(default)]
    pub primitive_framework: Option<String>,
    #[serde(default)]
    pub primitive_dependencies: Option<serde_json::Value>,
    #[serde(default)]
    pub adaptation_status: Option<String>,
    #[serde(default)]
    pub semantic_role: Option<String>,
    #[serde(default)]
    pub visual_value: Option<String>,
    #[serde(default)]
    pub information_hierarchy: Option<String>,
    #[serde(default)]
    pub visual_density: Option<String>,
    #[serde(default)]
    pub imagery_role: Option<String>,
    #[serde(default)]
    pub motion_role: Option<String>,
    #[serde(default)]
    pub product_representation: Option<String>,
    #[serde(default)]
    pub requested_departure: Option<String>,
    #[serde(default)]
    pub visual_evidence_id: Option<String>,
    #[serde(default)]
    pub primitive_version: Option<String>,
    #[serde(default)]
    pub viewport_width: Option<i32>,
    #[serde(default)]
    pub viewport_height: Option<i32>,
    #[serde(default)]
    pub motion_state: Option<String>,
    #[serde(default)]
    pub control_render_evidence_id: Option<String>,
    #[serde(default)]
    pub visual_identity_findings: Option<String>,
    #[serde(default)]
    pub hierarchy_findings: Option<String>,
    #[serde(default)]
    pub density_findings: Option<String>,
    #[serde(default)]
    pub proof_findings: Option<String>,
    #[serde(default)]
    pub product_fidelity_findings: Option<String>,
    #[serde(default)]
    pub motion_findings: Option<String>,
    #[serde(default)]
    pub defect_findings: Option<String>,
    #[serde(default)]
    pub departure_decision: Option<String>,
    #[serde(default)]
    pub culture_kind: Option<String>,
    #[serde(default)]
    pub culture_case_kind: Option<String>,
    #[serde(default)]
    pub culture_situation: Option<String>,
    #[serde(default)]
    pub culture_actors: Option<String>,
    #[serde(default)]
    pub decision_authority: Option<String>,
    #[serde(default)]
    pub observed_conduct: Option<String>,
    #[serde(default)]
    pub observed_outcome: Option<String>,
    #[serde(default)]
    pub culture_confidence: Option<String>,
    #[serde(default)]
    pub counterexample: Option<String>,
    #[serde(default)]
    pub boundary_conditions: Option<String>,
    #[serde(default)]
    pub operational_implication: Option<String>,
    #[serde(default)]
    pub actor_scope: Option<String>,
    #[serde(default)]
    pub culture_work_id: Option<String>,
    #[serde(default)]
    pub culture_actor: Option<String>,
    #[serde(default)]
    pub actor_role: Option<String>,
    #[serde(default)]
    pub team_name: Option<String>,
    #[serde(default)]
    pub decision_boundary: Option<String>,
    #[serde(default)]
    pub culture_decision: Option<String>,
    #[serde(default)]
    pub culture_alternatives: Option<serde_json::Value>,
    #[serde(default)]
    pub culture_unknowns: Option<String>,
    #[serde(default)]
    pub correction_of: Option<String>,
    #[serde(default)]
    pub correction_account: Option<String>,
    #[serde(default)]
    pub customer_action: Option<String>,
    #[serde(default)]
    pub culture_case_record_id: Option<String>,
    #[serde(default)]
    pub conduct_findings: Option<String>,
    #[serde(default)]
    pub dissent_findings: Option<String>,
    #[serde(default)]
    pub uncertainty_findings: Option<String>,
    #[serde(default)]
    pub correction_findings: Option<String>,
    #[serde(default)]
    pub authority_findings: Option<String>,
    #[serde(default)]
    pub customer_or_hiring_findings: Option<String>,
    #[serde(default)]
    pub slogan_recitation_detected: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct OwnerInput {
    #[serde(default)]
    pub preparing: bool,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub prepared: Option<String>,
    #[serde(default)]
    pub resume_when: Option<String>,
    /// The app (catalogue key or address) whose addition resolves a sign-in
    /// handoff.
    #[serde(default)]
    pub app: Option<String>,
    #[serde(default)]
    pub owner_kind: Option<String>,
    #[serde(default)]
    pub headline: Option<String>,
    #[serde(default)]
    pub situation: Option<String>,
    #[serde(default)]
    pub impact: Option<String>,
    #[serde(default)]
    pub recommendation: Option<String>,
    #[serde(default)]
    pub no_action: Option<String>,
    #[serde(default)]
    pub uncertainty: Option<String>,
    #[serde(default)]
    pub deadline: Option<String>,
}

/// Inputs for the bounded published-service contract. These are intentionally
/// not generic route, command, tunnel, environment, or provider fields.
#[derive(Debug, Default, Deserialize)]
pub struct PublicationInput {
    #[serde(default, rename = "publication_actor")]
    pub actor: Option<String>,
    #[serde(default)]
    pub source_artifact_ref_id: Option<String>,
    #[serde(default)]
    pub build_context: Option<String>,
    #[serde(default)]
    pub dockerfile: Option<String>,
    #[serde(default)]
    pub build_deadline: Option<String>,
    #[serde(default)]
    pub candidate_artifact_ref_id: Option<String>,
    #[serde(default)]
    pub service_manifest: Option<serde_json::Value>,
    #[serde(default)]
    pub publication_id: Option<String>,
    #[serde(default)]
    pub publication_audience: Option<String>,
    #[serde(default)]
    pub publication_expires_at: Option<String>,
    #[serde(default)]
    pub publication_start_deadline: Option<String>,
    #[serde(default)]
    pub cpu_millis: Option<u32>,
    #[serde(default)]
    pub memory_mib: Option<u32>,
    #[serde(default)]
    pub ephemeral_storage_mib: Option<u32>,
    #[serde(default)]
    pub max_connections: Option<u32>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub invitation_id: Option<String>,
    #[serde(default)]
    pub invitee: Option<String>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

/// Inputs for the Work-bound native Document review operation. Work and
/// Attempt names deliberately do not reuse generic OrgIntel fields: the
/// authenticated Runtime boundary compares this exact tuple with the signed
/// actor-session scope before the dispatcher may touch company state.
#[derive(Debug, Default, Deserialize)]
pub struct DocumentInput {
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub expected_document_version: Option<i64>,
    #[serde(default)]
    pub named_version_id: Option<String>,
    #[serde(default)]
    pub document_work_id: Option<String>,
    #[serde(default)]
    pub document_attempt_id: Option<String>,
    #[serde(default)]
    pub expected_work_revision: Option<i64>,
    #[serde(default)]
    pub reviewer_actor_id: Option<String>,
    #[serde(default)]
    pub review_summary: Option<String>,
    #[serde(default)]
    pub document_command_id: Option<String>,
}

/// The in-memory dispatch view after Request::decode has selected and checked
/// the command's domain fields. The JSON line remains source-compatible with
/// the CLI, but no command may smuggle an optional field from another domain
/// through this aggregate view.
#[derive(Debug, Deserialize)]
pub struct Request {
    pub cmd: String,
    pub company: Option<String>,
    /// Legacy caller spelling retained for source-compatible JSON. The daemon
    /// derives the actual principal from the listener and signed capability;
    /// TCP can never turn this field into owner authority.
    #[serde(default)]
    pub principal: Option<String>,
    /// Signed Runtime/session grant. It is an authenticated envelope field,
    /// deliberately separate from the Authority-domain capability input.
    #[serde(default)]
    pub session_capability: Option<String>,
    /// Populated only after the listener verifies `session_capability`.
    /// Never deserialized from a caller-controlled JSON field.
    #[serde(skip)]
    pub verified_coordination: Option<crate::capability::CoordinationGrant>,
    #[serde(flatten)]
    pub common: CommonInput,
    #[serde(flatten)]
    pub lifecycle: LifecycleInput,
    #[serde(flatten)]
    pub authority: AuthorityInput,
    #[serde(flatten)]
    pub orgintel: OrgIntelInput,
    #[serde(flatten)]
    pub owner: OwnerInput,
    #[serde(flatten)]
    pub publication: PublicationInput,
    #[serde(flatten)]
    pub document: DocumentInput,
    #[serde(flatten)]
    pub skills: SkillInput,
    #[serde(default)]
    pub room_operation: Option<crate::room_commands::RoomOperation>,
    #[serde(default)]
    pub document_operation: Option<crate::document_commands::DocumentOperation>,
    #[serde(default)]
    pub sheet_operation: Option<crate::sheet_commands::SheetOperation>,
    /// A typed decision a company actor asks for (`restless decide`): `state` and `questions`.
    #[serde(default)]
    pub decision: Option<serde_json::Value>,
    /// The app a `restless tools` command is about.
    #[serde(default)]
    pub tool: Option<String>,
    /// A Goal's observable finish line (`restless goal add|update --done-when`).
    #[serde(default)]
    pub done_when: Option<String>,
    /// A Goal's due date, YYYY-MM-DD; empty clears it.
    #[serde(default)]
    pub due: Option<String>,
}

const ENVELOPE_FIELDS: &[&str] = &["cmd", "company", "principal", "session_capability"];

/// Decode exactly the fields that belong to a command. The dispatcher still
/// receives a compact aggregate so existing domain handlers can be moved
/// independently, but this boundary refuses the old universal optional-field
/// bag before any handler sees it.
impl Request {
    pub fn decode(line: &str) -> std::result::Result<Self, String> {
        let mut value = serde_json::from_str::<serde_json::Value>(line)
            .map_err(|error| format!("parse JSON: {error}"))?;
        let object = value
            .as_object()
            .ok_or_else(|| "request must be a JSON object".to_string())?;
        let command = object
            .get("cmd")
            .and_then(serde_json::Value::as_str)
            .filter(|command| !command.is_empty())
            .map(str::to_string)
            .ok_or_else(|| "request needs a string cmd".to_string())?;
        let fields =
            command_fields(&command).ok_or_else(|| format!("unknown command {command:?}"))?;
        for field in object.keys() {
            if !ENVELOPE_FIELDS.contains(&field.as_str()) && !fields.contains(&field.as_str()) {
                return Err(format!(
                    "command {command:?} does not accept field {field:?}"
                ));
            }
        }
        // CommonInput already owns the historic from spelling for messages.
        // Normalise the only lifecycle use before flattened domain decoding so
        // the clone source cannot be silently dropped by that field collision.
        if command == "up" && object.contains_key("from") {
            let object = value
                .as_object_mut()
                .expect("the checked request remains an object");
            if object.contains_key("from_company") {
                return Err("command \"up\" may name only one clone source".to_string());
            }
            if let Some(from) = object.remove("from") {
                object.insert("from_company".to_string(), from);
            }
        }
        // `actor` is intentionally shared by several legacy domain inputs.
        // Flattened serde structs cannot decide which domain owns that key,
        // so normalize the two publication commands after the command
        // allowlist has accepted the public spelling. Without this boundary,
        // a valid CLI `--actor` is silently consumed by OrgIntelInput and the
        // publication handler sees no accountable producer.
        if matches!(
            command.as_str(),
            "publish-build" | "publish-candidate" | "publish-request"
        ) && value
            .as_object()
            .is_some_and(|object| object.contains_key("actor"))
        {
            let object = value
                .as_object_mut()
                .expect("the checked request remains an object");
            if let Some(actor) = object.remove("actor") {
                object.insert("publication_actor".to_string(), actor);
            }
        }
        serde_json::from_value(value).map_err(|error| format!("decode {command:?}: {error}"))
    }
}

/// This is a decoder allowlist, not a second command algebra: the dispatcher
/// remains the only command behaviour and source owner. Each row merely names
/// the already-existing fields that cross its concrete domain boundary.
fn command_fields(command: &str) -> Option<&'static [&'static str]> {
    Some(match command {
        "appliance-drain"
        | "appliance-resume"
        | "company-list"
        | "status"
        | "sleep"
        | "doctor"
        | "doctor-collaboration"
        | "company-show"
        | "credential-check"
        | "legal-show"
        | "legal-probe"
        | "finance-show"
        | "finance-balances"
        | "finance-probe"
        | "orgintel-init"
        | "teams"
        | "spend"
        | "telemetry"
        | "goals"
        | "work"
        | "work-graph"
        | "clear-poison"
        | "attention"
        | "browser-status"
        | "browser-session-register"
        | "browser-session-release"
        | "browser-release"
        | "watch"
        | "identity-show"
        | "publish-list" => &[],
        "schedule-wake" => &["adapter"],
        "publish-build" => &[
            "actor",
            "source_artifact_ref_id",
            "build_context",
            "dockerfile",
            "build_deadline",
            "idempotency_key",
        ],
        "publish-candidate" => &["actor", "source_artifact_ref_id", "service_manifest"],
        "publish-request" => &[
            "actor",
            "candidate_artifact_ref_id",
            "publication_audience",
            "publication_expires_at",
            "publication_start_deadline",
            "cpu_millis",
            "memory_mib",
            "ephemeral_storage_mib",
            "max_connections",
            "idempotency_key",
        ],
        "publish-authorize" | "publish-observe" | "publish-reconcile" => &["publication_id"],
        "publish-invite" => &[
            "publication_id",
            "invitation_id",
            "invitee",
            "publication_expires_at",
        ],
        "publish-revoke" => &["invitation_id"],
        "publish-stop" => &["publication_id", "stop_reason"],
        "publish-show" => &["publication_id"],
        "room-operation" => &["actor", "room_operation"],
        "document-operation" => &["actor", "document_operation"],
        "sheet-operation" => &["actor", "sheet_operation"],
        "decision-ask" => &["actor", "decision"],
        "tools-list" => &["actor", "tool"],
        "tools-check" => &["actor", "tool"],
        "document-review-request" => &[
            "document_id",
            "expected_document_version",
            "named_version_id",
            "document_work_id",
            "document_attempt_id",
            "expected_work_revision",
            "reviewer_actor_id",
            "review_summary",
            "document_command_id",
        ],
        "up" => &["from", "from_company", "reconcile"],
        "down" => &["destroy"],
        "company-create"
        | "legal-set"
        | "finance-envelope-set"
        | "finance-connect-airwallex"
        | "tell" => &["body"],
        "company-set" => &["state", "body"],
        "company-unset" => &["state"],
        "credential-set" => &["capability", "body", "secret_value"],
        "credential-promote" => &["capability", "body"],
        "credential-verify-model" => &["model", "apply"],
        "finance-freeze" => &["state", "apply"],
        "finance-reserve" => &["body"],
        "finance-submit" | "finance-reconcile" => &["key"],
        "wake" => &["reason"],
        "people" => &["include_retired"],
        "actor-create" => &["as_actor", "role", "name", "actor", "reason", "model"],
        "actor-model" => &["as_actor", "model", "actor", "reason"],
        "actor-retire" => &["as_actor", "actor", "reason"],
        "team-create" => &[
            "name",
            "to",
            "body",
            "actor",
            "outcome_standard",
            "outcome_standard_source",
            "source_message_id",
        ],
        "team-update" => &["name", "new_name", "body", "actor", "reason"],
        "team-assign" => &["as_actor", "name", "actor", "reason"],
        "team-lead" => &["name", "to", "actor", "reason"],
        "team-disband" => &["name", "actor", "reason"],
        "judgement" => &["as_actor"],
        "work-handoff-escalate" => &["id", "as_actor", "reason"],
        "receipts" => &["capability", "limit"],
        "spend-correct" => &[
            "correction_id",
            "request_ids",
            "delta_micro_usd",
            "reason",
            "apply",
        ],
        "goal-add" => &["title", "body", "actor", "done_when", "due"],
        "goal-update" => &["goal", "title", "actor", "done_when", "due"],
        "goal-close" => &["goal", "actor"],
        "goal-standard" => &["goal", "outcome_standard", "actor"],
        "work-standard" => &["id", "outcome_standard", "actor"],
        "skill-list" => &["as_actor", "include_retired"],
        "skill-observe" => &["as_actor", "observed_skills"],
        "skill-activate" => &[
            "as_actor",
            "skill",
            "skill_digest",
            "skill_work_id",
            "skill_attempt_id",
        ],
        "skill-candidate-add" => &["as_actor", "observed_skill", "origin_url", "origin_ref"],
        "skill-disposition" => &["as_actor", "skill", "disposition"],
        "skill-assign" => &["as_actor", "skill", "scope", "scope_id", "enabled"],
        "work-skill" => &["id", "as_actor", "skills"],
        "work-goal" => &["id", "goal", "actor"],
        "work-attempts" => &["id"],
        "work-assign" => &["id", "to", "actor", "reason"],
        "work-add" => &[
            "actor",
            "role",
            "title",
            "body",
            "model",
            "producing_topology",
            "goal",
            "priority",
            "expected_artifact",
            "repo",
            "base_ref",
            "integration_branch",
            "worktree",
            "attempt_limit",
            "owner_review",
            "source_message_id",
            "opportunity_id",
            "owner_epoch",
            "schedule_id",
            "requires",
            "revises",
            "gates",
            "constitution_contracts",
            "skills",
            "as_actor",
        ],
        "work-edge" => &["from", "to", "kind", "action", "as_actor", "reason"],
        "work-artifact" => &[
            "id",
            "attempt",
            "kind",
            "uri",
            "body",
            "actor",
            "digest",
            "source_commit",
            "label",
        ],
        "work-gate" => &[
            "id",
            "name",
            "cwd",
            "argv",
            "actor",
            "stage",
            "timeout_seconds",
            "resources",
        ],
        "work-gate-retire" => &["id", "reason", "as_actor"],
        "work-handoff" => &[
            "id",
            "attempt",
            "category",
            "action",
            "prepared",
            "resume_when",
            "app",
            "actor",
        ],
        "work-artifact-retire" => &["id", "reason", "actor"],
        "work-handoff-refresh" => &[
            "id",
            "as_actor",
            "action",
            "prepared",
            "resume_when",
            "preparing",
        ],
        "work-handoff-prepare-brief" => &[
            "id",
            "as_actor",
            "owner_kind",
            "headline",
            "situation",
            "impact",
            "recommendation",
            "no_action",
            "uncertainty",
            "deadline",
        ],
        "work-handoff-resolve" => &["id", "state", "resolution", "as_actor"],
        "work-interrupt" | "work-resume" | "work-abandon" => &["id", "as_actor", "reason"],
        "work-review" => &["id", "state", "resolution"],
        "inbox" => &["actor", "as_actor"],
        "message" => &["from", "to", "id", "body"],
        "message-react" | "message-unreact" => &["id", "body", "actor"],
        "events" => &["limit"],
        "schedule-list" => &["as_actor", "include_fired"],
        "schedule-test" => &["id", "schedule_test_timeout_seconds", "schedule_test_actor"],
        "schedule-opportunities" => &["responsibility_id", "limit"],
        "schedule-link-work" => &["id", "work_id", "owner_epoch", "relation", "as_actor"],
        "schedule-outcome" => &[
            "id",
            "owner_epoch",
            "state",
            "reason",
            "evidence_refs",
            "area_evidence",
            "as_actor",
        ],
        "schedule-responsibility-put" => &["id", "version", "objective", "policy"],
        "schedule-responsibility-bind" => &["id", "responsibility_id", "version"],
        "schedule-responsibility-create" => &[
            "as_actor",
            "fire_at",
            "recurrence",
            "interval_seconds",
            "local_time",
            "timezone",
            "missed_policy",
            "catch_up_grace_seconds",
            "execution_requirement",
            "reason",
            "id",
            "version",
            "objective",
            "policy",
        ],
        "schedule-history" => &["id", "limit"],
        "schedule-recover" => &["id", "fire_at", "as_actor", "from", "reason"],
        "schedule-retry-recovery" => &[
            "id",
            "fire_at",
            "as_actor",
            "from",
            "reason",
            "retry_key",
            "prior_message_id",
        ],
        "schedule-add" => &[
            "as_actor",
            "fire_at",
            "recurrence",
            "interval_seconds",
            "local_time",
            "timezone",
            "missed_policy",
            "catch_up_grace_seconds",
            "execution_requirement",
            "reason",
            "id",
            "version",
            "objective",
            "policy",
        ],
        "schedule-policy" => &["id", "as_actor", "missed_policy", "catch_up_grace_seconds"],
        "schedule-cancel" => &["id", "as_actor", "reason"],
        "approve" | "revoke" | "decline" => &["party"],
        "browser-request" => &["id"],
        "effect" => &[
            "effect_class",
            "purpose",
            "key",
            "cwd",
            "argv",
            "actor",
            "party",
            "artifacts",
            "secret_bindings",
        ],
        "effect-reconcile" => &["key"],
        "mandate-list" => &[],
        "mandate-permit" => &["mandate_id", "mandate_proposal", "actor"],
        "email-preview" | "email-send" => &["email_request", "actor"],
        "email-observe" => &["actor", "email_observe_list", "email_observe_after"],
        "identity-evidence-add" => &[
            "identity_pillar",
            "identity_kind",
            "claim_key",
            "statement",
            "actor",
            "source",
            "identity_authority",
            "scope",
            "evidence_locator",
            "polarity",
            "evidence_status",
            "channel",
            "audience",
            "supersedes",
            "exception_expires_at",
            "exception_indefinite",
        ],
        "identity-propose" => &["actor", "reason", "evidence_ids"],
        "identity-brief" => &[
            "actor",
            "release_id",
            "body",
            "channel",
            "audience",
            "max_bytes",
        ],
        "voice-evidence-add" => &[
            "voice_kind",
            "claim_key",
            "statement",
            "actor",
            "named_author",
            "source",
            "identity_authority",
            "scope",
            "evidence_locator",
            "judgement_reason",
            "polarity",
            "channel",
            "audience",
            "supersedes",
        ],
        "voice-bind" => &[
            "voice_work_id",
            "channel",
            "actor",
            "voice_author",
            "audience",
            "reader_situation",
            "desired_understanding",
            "desired_action",
            "proof",
            "consequence",
        ],
        "voice-brief" => &["voice_work_id", "max_bytes", "actor"],
        "voice-render" => &[
            "artifact_ref_id",
            "channel",
            "renderer",
            "renderer_version",
            "semantic_checks",
            "actor",
        ],
        "voice-review" => &[
            "render_evidence_id",
            "review_verdict",
            "factual_findings",
            "abstraction_findings",
            "repetition_findings",
            "channel_findings",
            "authorship_findings",
            "concepts_removed",
            "actor",
        ],
        "voice-learn" => &[
            "before_artifact_ref_id",
            "after_artifact_ref_id",
            "learning_kind",
            "claim_key",
            "observation",
            "motivating_decision",
            "scope",
            "source",
            "evidence_locator",
            "named_author",
            "channel",
            "audience",
            "actor",
        ],
        "visual-evidence-add" => &[
            "visual_kind",
            "claim_key",
            "statement",
            "actor",
            "source",
            "identity_authority",
            "scope",
            "evidence_locator",
            "visual_purpose",
            "visual_rationale",
            "accessibility_notes",
            "channel",
            "reduced_motion_replacement",
            "product_truth_locator",
            "primitive_origin",
            "primitive_licence",
            "primitive_framework",
            "primitive_dependencies",
            "adaptation_status",
            "semantic_role",
            "visual_value",
            "polarity",
        ],
        "visual-bind" => &[
            "visual_work_id",
            "channel",
            "actor",
            "audience",
            "body",
            "information_hierarchy",
            "proof",
            "visual_density",
            "imagery_role",
            "motion_role",
            "product_representation",
            "product_truth_locator",
            "requested_departure",
        ],
        "visual-brief" => &["visual_work_id", "max_bytes", "actor"],
        "visual-use" => &[
            "visual_work_id",
            "visual_evidence_id",
            "primitive_version",
            "visual_purpose",
            "actor",
        ],
        "visual-render" => &[
            "visual_work_id",
            "artifact_ref_id",
            "channel",
            "renderer",
            "renderer_version",
            "viewport_width",
            "viewport_height",
            "motion_state",
            "semantic_checks",
            "actor",
        ],
        "visual-review" => &[
            "render_evidence_id",
            "control_render_evidence_id",
            "review_verdict",
            "visual_identity_findings",
            "hierarchy_findings",
            "density_findings",
            "proof_findings",
            "product_fidelity_findings",
            "motion_findings",
            "defect_findings",
            "departure_decision",
            "actor",
        ],
        "culture-evidence-add" => &[
            "culture_kind",
            "culture_case_kind",
            "claim_key",
            "statement",
            "actor",
            "source",
            "identity_authority",
            "scope",
            "evidence_locator",
            "culture_situation",
            "consequence",
            "culture_actors",
            "decision_authority",
            "observed_conduct",
            "observed_outcome",
            "culture_confidence",
            "counterexample",
            "boundary_conditions",
            "operational_implication",
            "actor_scope",
        ],
        "culture-bind" => &[
            "culture_work_id",
            "culture_case_kind",
            "culture_actor",
            "actor_role",
            "team_name",
            "consequence",
            "decision_boundary",
            "actor",
        ],
        "culture-brief" => &["culture_work_id", "max_bytes", "actor"],
        "culture-case" => &[
            "culture_work_id",
            "artifact_ref_id",
            "culture_case_kind",
            "culture_decision",
            "culture_alternatives",
            "culture_unknowns",
            "correction_of",
            "correction_account",
            "customer_action",
            "semantic_checks",
            "actor",
        ],
        "culture-review" => &[
            "culture_case_record_id",
            "review_verdict",
            "conduct_findings",
            "dissent_findings",
            "uncertainty_findings",
            "correction_findings",
            "authority_findings",
            "customer_or_hiring_findings",
            "slogan_recitation_detected",
            "actor",
        ],
        _ => return None,
    })
}

/// The V0 principal set (`authority-plane §4.1`). Two, because two is what
/// exists: the human on the host and the company in a Runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Principal {
    Owner,
    CompanyExec,
}

impl Principal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::CompanyExec => "company/exec",
        }
    }
}

/// Commands that widen authority, change company lifecycle, or resolve the
/// owner's review boundary. This is a finite V0 list, not a policy DSL.
pub const OWNER_ONLY: &[&str] = &[
    "doctor-collaboration",
    "schedule-test",
    "schedule-responsibility-put",
    "schedule-responsibility-bind",
    "schedule-responsibility-create",
    "approve",
    "decline",
    "revoke",
    "up",
    "down",
    "sleep",
    "clear-poison",
    "spend-correct",
    "company-create",
    "company-set",
    "company-unset",
    "credential-set",
    "credential-promote",
    "credential-verify-model",
    "legal-set",
    "finance-envelope-set",
    "finance-freeze",
    "finance-connect-airwallex",
    "work-review",
    "voice-learn",
    "publish-authorize",
    "publish-invite",
    "publish-revoke",
    "publish-stop",
    "publish-reconcile",
    "schedule-wake",
    "appliance-drain",
    "appliance-resume",
    // Accepting an imported skill (possibly with scripts) and deciding who
    // may use it are owner trust decisions for this sprint.
    "skill-disposition",
    "skill-assign",
];

/// Actor-owned Opportunity mutations. The owner has a separate, future
/// administrative path and cannot impersonate Exec through these commands.
const COMPANY_EXEC_ONLY: &[&str] = &[
    "mandate-permit",
    "email-send",
    "email-observe",
    "schedule-link-work",
    "schedule-outcome",
    "browser-session-register",
    "browser-session-release",
];

pub fn authorize(principal: Principal, cmd: &str) -> std::result::Result<Principal, String> {
    if principal != Principal::Owner && OWNER_ONLY.contains(&cmd) {
        return Err(format!(
            "{cmd} is an act of owner authority; principal {} may not perform it",
            principal.as_str()
        ));
    }
    if principal != Principal::CompanyExec && COMPANY_EXEC_ONLY.contains(&cmd) {
        return Err(format!(
            "{cmd} is an actor-owned Opportunity mutation; principal {} may not perform it",
            principal.as_str()
        ));
    }
    Ok(principal)
}

impl Principal {
    /// Old images still send company/exec. It is compatibility syntax only:
    /// the listener capability, not this field, supplies real authority.
    pub fn legacy_runtime_claim(raw: Option<&str>) -> std::result::Result<(), String> {
        match raw.map(str::trim).filter(|value| !value.is_empty()) {
            None | Some("company/exec") => Ok(()),
            Some("owner") => Err("TCP Runtime traffic may not claim owner authority".into()),
            Some(other) => Err(format!("unknown TCP principal claim {other:?}")),
        }
    }
}

/// Typed protocol refusal; clients can distinguish authority denial from an
/// unreachable daemon without parsing prose.
#[derive(Debug, Serialize)]
struct ErrorBody {
    kind: String,
    message: String,
}

#[derive(Debug, Serialize)]
pub struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorBody>,
}

impl Response {
    pub fn ok(data: impl Into<serde_json::Value>) -> Self {
        Self {
            ok: true,
            data: Some(data.into()),
            error: None,
        }
    }

    pub fn ok_serialized(data: impl serde::Serialize) -> Self {
        match serde_json::to_value(data) {
            Ok(data) => Self::ok(data),
            Err(error) => Self::err(format!("encode response: {error}")),
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self::err_kind("error", message)
    }

    pub fn err_kind(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(ErrorBody {
                kind: kind.into(),
                message: message.into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{authorize, command_fields, Principal, Request};

    #[test]
    fn skill_trust_decisions_are_owner_acts_and_skill_fields_stay_in_their_commands() {
        assert!(authorize(Principal::CompanyExec, "skill-disposition").is_err());
        assert!(authorize(Principal::CompanyExec, "skill-assign").is_err());
        assert!(authorize(Principal::CompanyExec, "skill-activate").is_ok());
        let request = Request::decode(
            r#"{"cmd":"skill-activate","company":"acme","as_actor":"writer","skill":"frontend-design","skill_digest":"sha256:ab"}"#,
        )
        .unwrap();
        assert_eq!(request.skills.skill.as_deref(), Some("frontend-design"));
        let work = Request::decode(
            r#"{"cmd":"work-add","company":"acme","skills":["gauntlet"],"as_actor":"lead"}"#,
        )
        .unwrap();
        assert_eq!(work.orgintel.skills, vec!["gauntlet".to_string()]);
        assert!(Request::decode(r#"{"cmd":"work-list","company":"acme","skills":["x"]}"#).is_err());
        let interval = Request::decode(
            r#"{"cmd":"schedule-add","company":"acme","as_actor":"exec","recurrence":"interval","interval_seconds":1800,"reason":"check leads"}"#,
        )
        .unwrap();
        assert_eq!(interval.orgintel.interval_seconds, Some(1800));
    }

    #[test]
    fn collaboration_doctor_requires_owner_and_no_arbitrary_probe_target() {
        assert!(authorize(Principal::CompanyExec, "doctor-collaboration").is_err());
        assert!(authorize(Principal::Owner, "doctor-collaboration").is_ok());
        assert!(Request::decode(r#"{"cmd":"doctor-collaboration","company":"acme"}"#).is_ok());
        assert!(Request::decode(
            r#"{"cmd":"doctor-collaboration","company":"acme","body":"untrusted probe"}"#
        )
        .is_err());
    }

    #[test]
    fn command_decoder_selects_the_owner_domain_and_refuses_foreign_fields() {
        let request = Request::decode(
            r#"{
                "cmd":"work-handoff-prepare-brief",
                "company":"review_test",
                "principal":"owner",
                "id":"00000000-0000-0000-0000-000000000001",
                "as_actor":"delivery-lead",
                "owner_kind":"outcome_review",
                "headline":"Inspect the exact candidate",
                "situation":"A native review target is ready.",
                "impact":"Acceptance completes the Work.",
                "recommendation":"Open it.",
                "no_action":"It remains paused."
            }"#,
        )
        .expect("decode owner payload");
        assert_eq!(
            request.common.id.as_deref(),
            Some("00000000-0000-0000-0000-000000000001")
        );
        assert_eq!(request.owner.owner_kind.as_deref(), Some("outcome_review"));
        assert!(request.orgintel.gates.is_empty());
        assert!(!request.lifecycle.reconcile);
        assert!(
            Request::decode(
                r#"{
                    "cmd":"message",
                    "company":"review_test",
                    "from":"exec",
                    "body":"hello",
                    "model":"moonshot/kimi-k3"
                }"#
            )
            .is_err(),
            "a message may not carry a Work/model field"
        );
    }

    #[test]
    fn work_add_keeps_owner_review_an_explicit_narrow_contract() {
        let request = Request::decode(
            r#"{
                "cmd":"work-add",
                "company":"review_test",
                "actor":"research-analyst",
                "role":"research",
                "title":"Review the prepared dossier",
                "body":"a current evidence-linked outcome",
                "expected_artifact":"native ReviewTarget",
                "owner_review":true,
                "producing_topology":"coherent-single-worker",
                "constitution_contracts":{
                    "voice":{
                        "channel":"blog",
                        "author":"Founder",
                        "audience":"operators",
                        "reader_situation":"judging an unfamiliar system",
                        "desired_understanding":"what changes in their work",
                        "desired_action":"review the product proof",
                        "proof":"one exact operating example",
                        "consequence":"less manual coordination"
                    }
                },
                "gates":[{"name":"review-target-live-probe","command":["test","-s","report.html"]}]
            }"#,
        )
        .expect("decode explicit owner-review Work contract");
        assert!(request.orgintel.owner_review);
        assert_eq!(
            request.orgintel.producing_topology.as_deref(),
            Some("coherent-single-worker")
        );
        assert_eq!(
            request
                .orgintel
                .constitution_contracts
                .as_ref()
                .and_then(|contracts| contracts.voice.as_ref())
                .map(|contract| contract.channel),
            Some(restless_orgintel::VoiceChannel::Blog)
        );
        assert!(
            Request::decode(
                r#"{
                    "cmd":"work-add",
                    "company":"review_test",
                    "actor":"research-analyst",
                    "role":"research",
                    "title":"Mistyped contract",
                    "body":"must fail before Work exists",
                    "constitution_contracts":{"voice":{"channe":"blog"}}
                }"#
            )
            .is_err(),
            "unknown constitution fields must not silently erase lead intent"
        );
        assert!(
            Request::decode(
                r#"{
                    "cmd":"work-artifact",
                    "company":"review_test",
                    "id":"00000000-0000-0000-0000-000000000001",
                    "attempt":"00000000-0000-0000-0000-000000000002",
                    "kind":"review_target",
                    "uri":"/company/reports/current.html",
                    "owner_review":true
                }"#
            )
            .is_err(),
            "the flag belongs only to Work creation, not arbitrary artifact writes"
        );
    }

    #[test]
    fn lifecycle_decoder_uses_the_cli_from_spelling() {
        let request = Request::decode(
            r#"{"cmd":"up","company":"clone_test","from":"source_test","reconcile":true}"#,
        )
        .expect("decode clone lifecycle request");
        assert_eq!(
            request.lifecycle.from_company.as_deref(),
            Some("source_test")
        );
        assert!(request.lifecycle.reconcile);
    }

    #[test]
    fn lifecycle_decoder_keeps_the_current_from_company_spelling() {
        let request =
            Request::decode(r#"{"cmd":"up","company":"clone_test","from_company":"source_test"}"#)
                .expect("decode current clone lifecycle request");
        assert_eq!(
            request.lifecycle.from_company.as_deref(),
            Some("source_test")
        );
    }

    #[test]
    fn publication_decoder_preserves_the_accountable_actor() {
        let request = Request::decode(
            r#"{
                "cmd":"publish-candidate",
                "company":"swift_arrival_test",
                "actor":"release-auditor",
                "source_artifact_ref_id":"00000000-0000-0000-0000-000000000001",
                "service_manifest":{}
            }"#,
        )
        .expect("decode publication actor");
        assert_eq!(
            request.publication.actor.as_deref(),
            Some("release-auditor")
        );
        assert_eq!(request.orgintel.actor, None);
    }

    #[test]
    fn document_review_request_has_one_exact_domain_view() {
        let request = Request::decode(
            r#"{
                "cmd":"document-review-request",
                "company":"acme_test",
                "document_id":"00000000-0000-0000-0000-000000000001",
                "expected_document_version":3,
                "named_version_id":"00000000-0000-0000-0000-000000000002",
                "document_work_id":"00000000-0000-0000-0000-000000000003",
                "document_attempt_id":"00000000-0000-0000-0000-000000000004",
                "expected_work_revision":2,
                "reviewer_actor_id":"alex",
                "review_summary":"Review this exact named version",
                "document_command_id":"00000000-0000-0000-0000-000000000005"
            }"#,
        )
        .expect("decode exact document review operation");
        assert_eq!(request.document.expected_work_revision, Some(2));
        assert_eq!(request.document.reviewer_actor_id.as_deref(), Some("alex"));
        assert!(request.orgintel.actor.is_none());
        assert!(Request::decode(
            r#"{
                "cmd":"document-review-request",
                "company":"acme_test",
                "document_id":"00000000-0000-0000-0000-000000000001",
                "actor":"owner"
            }"#,
        )
        .is_err());
        assert!(Request::decode(
            r#"{
                "cmd":"document-review-request",
                "company":"acme_test",
                "document_id":"00000000-0000-0000-0000-000000000001",
                "work_id":"00000000-0000-0000-0000-000000000003"
            }"#,
        )
        .is_err());
    }

    #[test]
    fn every_dispatch_command_has_a_checked_domain_view() {
        // Kept next to the boundary rather than parsed from Rust source: the
        // dispatcher legitimately contains nested string matches such as
        // company config keys, which are not transport commands.
        const COMMANDS: &[&str] = &[
            "up",
            "down",
            "sleep",
            "status",
            "doctor",
            "doctor-collaboration",
            "company-list",
            "company-create",
            "company-show",
            "company-set",
            "company-unset",
            "credential-set",
            "credential-promote",
            "credential-check",
            "credential-verify-model",
            "legal-show",
            "legal-probe",
            "legal-set",
            "finance-show",
            "finance-envelope-set",
            "finance-freeze",
            "finance-connect-airwallex",
            "finance-balances",
            "finance-probe",
            "finance-reserve",
            "finance-submit",
            "finance-reconcile",
            "orgintel-init",
            "wake",
            "tell",
            "people",
            "actor-create",
            "actor-model",
            "actor-retire",
            "teams",
            "team-create",
            "team-update",
            "team-assign",
            "team-lead",
            "team-disband",
            "judgement",
            "work-handoff-escalate",
            "receipts",
            "spend",
            "telemetry",
            "spend-correct",
            "goals",
            "goal-add",
            "goal-update",
            "message-react",
            "message-unreact",
            "goal-standard",
            "work-goal",
            "work-standard",
            "work",
            "work-graph",
            "work-attempts",
            "work-assign",
            "work-add",
            "work-edge",
            "work-artifact",
            "work-artifact-retire",
            "work-gate",
            "work-gate-retire",
            "work-handoff",
            "work-handoff-refresh",
            "work-handoff-prepare-brief",
            "work-handoff-resolve",
            "work-interrupt",
            "work-resume",
            "work-abandon",
            "work-review",
            "inbox",
            "message",
            "events",
            "clear-poison",
            "approve",
            "revoke",
            "decline",
            "attention",
            "browser-status",
            "browser-session-register",
            "browser-session-release",
            "browser-request",
            "browser-release",
            "effect",
            "effect-reconcile",
            "watch",
            "document-review-request",
        ];
        for command in COMMANDS {
            assert!(
                command_fields(command).is_some(),
                "dispatch command {command:?} has no checked input view"
            );
        }
    }
}
