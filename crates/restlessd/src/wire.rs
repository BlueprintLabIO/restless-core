//! Stable JSON-lines transport types, grouped by the plane that owns each
//! input. The wire remains flat for the existing CLI and company runtime, but
//! the decoder rejects fields outside each command's domain view.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
pub(crate) struct CommonInput {
    #[serde(default)]
    pub(crate) reason: Option<String>,
    #[serde(default)]
    pub(crate) body: Option<String>,
    #[serde(default)]
    pub(crate) from: Option<String>,
    #[serde(default)]
    pub(crate) to: Option<String>,
    #[serde(default)]
    pub(crate) as_actor: Option<String>,
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(default)]
    pub(crate) state: Option<String>,
    #[serde(default)]
    pub(crate) resolution: Option<String>,
    #[serde(default)]
    pub(crate) limit: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct LifecycleInput {
    /// `up --from <live>` clones a company into a throwaway; `down --destroy`
    /// removes the environment and its recoverable coordination state.
    #[serde(default)]
    pub(crate) from_company: Option<String>,
    #[serde(default)]
    pub(crate) destroy: bool,
    /// Fetch and reconcile the configured Company Runtime image. Building and
    /// publishing the artifact is a release/Fleet concern.
    #[serde(default)]
    pub(crate) reconcile: bool,
    /// Host transport that delivered a wake-only schedule hint. It carries no
    /// company or task payload and is accepted only on the local-owner socket.
    #[serde(default)]
    pub(crate) adapter: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct AuthorityInput {
    /// Secret material is forwarded once to the configured credential backend
    /// and never persisted in company config or logs.
    #[serde(default)]
    pub(crate) secret_value: Option<String>,
    #[serde(default)]
    pub(crate) correction_id: Option<String>,
    #[serde(default)]
    pub(crate) request_ids: Vec<String>,
    #[serde(default)]
    pub(crate) delta_micro_usd: Option<i64>,
    #[serde(default)]
    pub(crate) apply: bool,
    #[serde(default)]
    pub(crate) capability: Option<String>,
    #[serde(default)]
    pub(crate) effect_class: Option<String>,
    #[serde(default)]
    pub(crate) purpose: Option<String>,
    #[serde(default)]
    pub(crate) artifacts: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) secret_bindings: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub(crate) key: Option<String>,
    #[serde(default)]
    pub(crate) execution_no: Option<i32>,
    #[serde(default)]
    pub(crate) party: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ConnectedToolInput {
    #[serde(default)]
    pub(crate) tool_name: Option<String>,
    #[serde(default)]
    pub(crate) endpoint: Option<String>,
    #[serde(default)]
    pub(crate) assigned_actor: Option<String>,
    #[serde(default)]
    pub(crate) work_id: Option<String>,
    #[serde(default)]
    pub(crate) attempt_id: Option<String>,
    #[serde(default)]
    pub(crate) requested_scopes: Vec<String>,
    #[serde(default)]
    pub(crate) workspace_reference: Option<String>,
    #[serde(default)]
    pub(crate) observed_tools: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct InitialWorkGateRequest {
    pub(crate) name: String,
    pub(crate) command: Vec<String>,
    #[serde(default = "default_gate_stage")]
    pub(crate) stage: String,
    #[serde(default = "default_gate_timeout")]
    pub(crate) timeout_seconds: i32,
    #[serde(default)]
    pub(crate) resources: Vec<String>,
}

fn default_gate_stage() -> String {
    "cumulative".into()
}

fn default_gate_timeout() -> i32 {
    900
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct OrgIntelInput {
    #[serde(default)]
    pub(crate) include_retired: bool,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) new_name: Option<String>,
    #[serde(default)]
    pub(crate) repo: Option<String>,
    #[serde(default)]
    pub(crate) actor: Option<String>,
    #[serde(default)]
    pub(crate) role: Option<String>,
    #[serde(default)]
    pub(crate) model: Option<String>,
    #[serde(default)]
    pub(crate) producing_topology: Option<String>,
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) priority: Option<i16>,
    #[serde(default)]
    pub(crate) expected_artifact: Option<String>,
    #[serde(default)]
    pub(crate) base_ref: Option<String>,
    #[serde(default)]
    pub(crate) integration_branch: Option<String>,
    #[serde(default)]
    pub(crate) worktree: Option<String>,
    #[serde(default)]
    pub(crate) attempt_limit: Option<i32>,
    /// Opt this Work into the qualified owner-outcome path. Completion then
    /// requires one ReviewTarget artifact and its named live-probe gate.
    #[serde(default)]
    pub(crate) owner_review: bool,
    #[serde(default)]
    pub(crate) goal: Option<String>,
    #[serde(default)]
    pub(crate) source_message_id: Option<i64>,
    #[serde(default)]
    pub(crate) outcome_standard: Option<String>,
    #[serde(default)]
    pub(crate) outcome_standard_source: Option<String>,
    #[serde(default)]
    pub(crate) requires: Vec<String>,
    #[serde(default)]
    pub(crate) revises: Vec<String>,
    #[serde(default)]
    pub(crate) gates: Vec<InitialWorkGateRequest>,
    #[serde(default)]
    pub(crate) constitution_contracts: Option<restless_orgintel::InitialConstitutionContracts>,
    #[serde(default)]
    pub(crate) kind: Option<String>,
    #[serde(default)]
    pub(crate) attempt: Option<String>,
    #[serde(default)]
    pub(crate) uri: Option<String>,
    #[serde(default)]
    pub(crate) digest: Option<String>,
    #[serde(default)]
    pub(crate) source_commit: Option<String>,
    #[serde(default)]
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) cwd: Option<String>,
    #[serde(default)]
    pub(crate) argv: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) stage: Option<String>,
    #[serde(default)]
    pub(crate) timeout_seconds: Option<i32>,
    #[serde(default)]
    pub(crate) resources: Vec<String>,
    #[serde(default)]
    pub(crate) fire_at: Option<String>,
    #[serde(default)]
    pub(crate) recurrence: Option<String>,
    #[serde(default)]
    pub(crate) local_time: Option<String>,
    #[serde(default)]
    pub(crate) timezone: Option<String>,
    #[serde(default)]
    pub(crate) missed_policy: Option<String>,
    #[serde(default)]
    pub(crate) catch_up_grace_seconds: Option<i64>,
    #[serde(default)]
    pub(crate) execution_requirement: Option<String>,
    #[serde(default)]
    pub(crate) retry_key: Option<String>,
    #[serde(default)]
    pub(crate) prior_message_id: Option<i64>,
    #[serde(default)]
    pub(crate) include_fired: bool,
    #[serde(default)]
    pub(crate) identity_pillar: Option<String>,
    #[serde(default)]
    pub(crate) identity_kind: Option<String>,
    #[serde(default)]
    pub(crate) claim_key: Option<String>,
    #[serde(default)]
    pub(crate) statement: Option<String>,
    #[serde(default)]
    pub(crate) source: Option<String>,
    #[serde(default)]
    pub(crate) identity_authority: Option<String>,
    #[serde(default)]
    pub(crate) scope: Option<String>,
    #[serde(default)]
    pub(crate) evidence_locator: Option<String>,
    #[serde(default)]
    pub(crate) polarity: Option<String>,
    #[serde(default)]
    pub(crate) evidence_status: Option<String>,
    #[serde(default)]
    pub(crate) channel: Option<String>,
    #[serde(default)]
    pub(crate) audience: Option<String>,
    #[serde(default)]
    pub(crate) supersedes: Option<String>,
    #[serde(default)]
    pub(crate) exception_expires_at: Option<String>,
    #[serde(default)]
    pub(crate) exception_indefinite: bool,
    #[serde(default)]
    pub(crate) evidence_ids: Vec<String>,
    #[serde(default)]
    pub(crate) release_id: Option<String>,
    #[serde(default)]
    pub(crate) max_bytes: Option<usize>,
    #[serde(default)]
    pub(crate) voice_kind: Option<String>,
    #[serde(default)]
    pub(crate) named_author: Option<String>,
    #[serde(default)]
    pub(crate) voice_author: Option<String>,
    #[serde(default)]
    pub(crate) judgement_reason: Option<String>,
    #[serde(default)]
    pub(crate) voice_work_id: Option<String>,
    #[serde(default)]
    pub(crate) reader_situation: Option<String>,
    #[serde(default)]
    pub(crate) desired_understanding: Option<String>,
    #[serde(default)]
    pub(crate) desired_action: Option<String>,
    #[serde(default)]
    pub(crate) proof: Option<String>,
    #[serde(default)]
    pub(crate) consequence: Option<String>,
    #[serde(default)]
    pub(crate) artifact_ref_id: Option<String>,
    #[serde(default)]
    pub(crate) renderer: Option<String>,
    #[serde(default)]
    pub(crate) renderer_version: Option<String>,
    #[serde(default)]
    pub(crate) semantic_checks: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) render_evidence_id: Option<String>,
    #[serde(default)]
    pub(crate) review_verdict: Option<String>,
    #[serde(default)]
    pub(crate) factual_findings: Option<String>,
    #[serde(default)]
    pub(crate) abstraction_findings: Option<String>,
    #[serde(default)]
    pub(crate) repetition_findings: Option<String>,
    #[serde(default)]
    pub(crate) channel_findings: Option<String>,
    #[serde(default)]
    pub(crate) authorship_findings: Option<String>,
    #[serde(default)]
    pub(crate) concepts_removed: Option<String>,
    #[serde(default)]
    pub(crate) before_artifact_ref_id: Option<String>,
    #[serde(default)]
    pub(crate) after_artifact_ref_id: Option<String>,
    #[serde(default)]
    pub(crate) learning_kind: Option<String>,
    #[serde(default)]
    pub(crate) observation: Option<String>,
    #[serde(default)]
    pub(crate) motivating_decision: Option<String>,
    #[serde(default)]
    pub(crate) visual_kind: Option<String>,
    #[serde(default)]
    pub(crate) visual_work_id: Option<String>,
    #[serde(default)]
    pub(crate) visual_purpose: Option<String>,
    #[serde(default)]
    pub(crate) visual_rationale: Option<String>,
    #[serde(default)]
    pub(crate) accessibility_notes: Option<String>,
    #[serde(default)]
    pub(crate) reduced_motion_replacement: Option<String>,
    #[serde(default)]
    pub(crate) product_truth_locator: Option<String>,
    #[serde(default)]
    pub(crate) primitive_origin: Option<String>,
    #[serde(default)]
    pub(crate) primitive_licence: Option<String>,
    #[serde(default)]
    pub(crate) primitive_framework: Option<String>,
    #[serde(default)]
    pub(crate) primitive_dependencies: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) adaptation_status: Option<String>,
    #[serde(default)]
    pub(crate) semantic_role: Option<String>,
    #[serde(default)]
    pub(crate) visual_value: Option<String>,
    #[serde(default)]
    pub(crate) information_hierarchy: Option<String>,
    #[serde(default)]
    pub(crate) visual_density: Option<String>,
    #[serde(default)]
    pub(crate) imagery_role: Option<String>,
    #[serde(default)]
    pub(crate) motion_role: Option<String>,
    #[serde(default)]
    pub(crate) product_representation: Option<String>,
    #[serde(default)]
    pub(crate) requested_departure: Option<String>,
    #[serde(default)]
    pub(crate) visual_evidence_id: Option<String>,
    #[serde(default)]
    pub(crate) primitive_version: Option<String>,
    #[serde(default)]
    pub(crate) viewport_width: Option<i32>,
    #[serde(default)]
    pub(crate) viewport_height: Option<i32>,
    #[serde(default)]
    pub(crate) motion_state: Option<String>,
    #[serde(default)]
    pub(crate) control_render_evidence_id: Option<String>,
    #[serde(default)]
    pub(crate) visual_identity_findings: Option<String>,
    #[serde(default)]
    pub(crate) hierarchy_findings: Option<String>,
    #[serde(default)]
    pub(crate) density_findings: Option<String>,
    #[serde(default)]
    pub(crate) proof_findings: Option<String>,
    #[serde(default)]
    pub(crate) product_fidelity_findings: Option<String>,
    #[serde(default)]
    pub(crate) motion_findings: Option<String>,
    #[serde(default)]
    pub(crate) defect_findings: Option<String>,
    #[serde(default)]
    pub(crate) departure_decision: Option<String>,
    #[serde(default)]
    pub(crate) culture_kind: Option<String>,
    #[serde(default)]
    pub(crate) culture_case_kind: Option<String>,
    #[serde(default)]
    pub(crate) culture_situation: Option<String>,
    #[serde(default)]
    pub(crate) culture_actors: Option<String>,
    #[serde(default)]
    pub(crate) decision_authority: Option<String>,
    #[serde(default)]
    pub(crate) observed_conduct: Option<String>,
    #[serde(default)]
    pub(crate) observed_outcome: Option<String>,
    #[serde(default)]
    pub(crate) culture_confidence: Option<String>,
    #[serde(default)]
    pub(crate) counterexample: Option<String>,
    #[serde(default)]
    pub(crate) boundary_conditions: Option<String>,
    #[serde(default)]
    pub(crate) operational_implication: Option<String>,
    #[serde(default)]
    pub(crate) actor_scope: Option<String>,
    #[serde(default)]
    pub(crate) culture_work_id: Option<String>,
    #[serde(default)]
    pub(crate) culture_actor: Option<String>,
    #[serde(default)]
    pub(crate) actor_role: Option<String>,
    #[serde(default)]
    pub(crate) team_name: Option<String>,
    #[serde(default)]
    pub(crate) decision_boundary: Option<String>,
    #[serde(default)]
    pub(crate) culture_decision: Option<String>,
    #[serde(default)]
    pub(crate) culture_alternatives: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) culture_unknowns: Option<String>,
    #[serde(default)]
    pub(crate) correction_of: Option<String>,
    #[serde(default)]
    pub(crate) correction_account: Option<String>,
    #[serde(default)]
    pub(crate) customer_action: Option<String>,
    #[serde(default)]
    pub(crate) culture_case_record_id: Option<String>,
    #[serde(default)]
    pub(crate) conduct_findings: Option<String>,
    #[serde(default)]
    pub(crate) dissent_findings: Option<String>,
    #[serde(default)]
    pub(crate) uncertainty_findings: Option<String>,
    #[serde(default)]
    pub(crate) correction_findings: Option<String>,
    #[serde(default)]
    pub(crate) authority_findings: Option<String>,
    #[serde(default)]
    pub(crate) customer_or_hiring_findings: Option<String>,
    #[serde(default)]
    pub(crate) slogan_recitation_detected: bool,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct OwnerInput {
    #[serde(default)]
    pub(crate) category: Option<String>,
    #[serde(default)]
    pub(crate) action: Option<String>,
    #[serde(default)]
    pub(crate) prepared: Option<String>,
    #[serde(default)]
    pub(crate) resume_when: Option<String>,
    #[serde(default)]
    pub(crate) owner_kind: Option<String>,
    #[serde(default)]
    pub(crate) headline: Option<String>,
    #[serde(default)]
    pub(crate) situation: Option<String>,
    #[serde(default)]
    pub(crate) impact: Option<String>,
    #[serde(default)]
    pub(crate) recommendation: Option<String>,
    #[serde(default)]
    pub(crate) no_action: Option<String>,
    #[serde(default)]
    pub(crate) uncertainty: Option<String>,
    #[serde(default)]
    pub(crate) deadline: Option<String>,
}

/// Inputs for the bounded published-service contract. These are intentionally
/// not generic route, command, tunnel, environment, or provider fields.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct PublicationInput {
    #[serde(default, rename = "publication_actor")]
    pub(crate) actor: Option<String>,
    #[serde(default)]
    pub(crate) source_artifact_ref_id: Option<String>,
    #[serde(default)]
    pub(crate) candidate_artifact_ref_id: Option<String>,
    #[serde(default)]
    pub(crate) service_manifest: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) publication_id: Option<String>,
    #[serde(default)]
    pub(crate) publication_audience: Option<String>,
    #[serde(default)]
    pub(crate) publication_expires_at: Option<String>,
    #[serde(default)]
    pub(crate) publication_start_deadline: Option<String>,
    #[serde(default)]
    pub(crate) cpu_millis: Option<u32>,
    #[serde(default)]
    pub(crate) memory_mib: Option<u32>,
    #[serde(default)]
    pub(crate) ephemeral_storage_mib: Option<u32>,
    #[serde(default)]
    pub(crate) max_connections: Option<u32>,
    #[serde(default)]
    pub(crate) idempotency_key: Option<String>,
    #[serde(default)]
    pub(crate) invitation_id: Option<String>,
    #[serde(default)]
    pub(crate) invitee: Option<String>,
    #[serde(default)]
    pub(crate) stop_reason: Option<String>,
}

/// The in-memory dispatch view after Request::decode has selected and checked
/// the command's domain fields. The JSON line remains source-compatible with
/// the CLI, but no command may smuggle an optional field from another domain
/// through this aggregate view.
#[derive(Debug, Deserialize)]
pub(crate) struct Request {
    pub(crate) cmd: String,
    pub(crate) company: Option<String>,
    /// Legacy caller spelling retained for source-compatible JSON. The daemon
    /// derives the actual principal from the listener and signed capability;
    /// TCP can never turn this field into owner authority.
    #[serde(default)]
    pub(crate) principal: Option<String>,
    /// Signed Runtime/session grant. It is an authenticated envelope field,
    /// deliberately separate from the Authority-domain capability input.
    #[serde(default)]
    pub(crate) session_capability: Option<String>,
    #[serde(flatten)]
    pub(crate) common: CommonInput,
    #[serde(flatten)]
    pub(crate) lifecycle: LifecycleInput,
    #[serde(flatten)]
    pub(crate) authority: AuthorityInput,
    #[serde(flatten)]
    pub(crate) connected_tool: ConnectedToolInput,
    #[serde(flatten)]
    pub(crate) orgintel: OrgIntelInput,
    #[serde(flatten)]
    pub(crate) owner: OwnerInput,
    #[serde(flatten)]
    pub(crate) publication: PublicationInput,
}

const ENVELOPE_FIELDS: &[&str] = &["cmd", "company", "principal", "session_capability"];

/// Decode exactly the fields that belong to a command. The dispatcher still
/// receives a compact aggregate so existing domain handlers can be moved
/// independently, but this boundary refuses the old universal optional-field
/// bag before any handler sees it.
impl Request {
    pub(crate) fn decode(line: &str) -> std::result::Result<Self, String> {
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
        if matches!(command.as_str(), "publish-candidate" | "publish-request")
            && value
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
        "appliance-drain" | "appliance-resume" | "company-list" | "status" | "doctor"
        | "company-show" | "credential-check" | "legal-show" | "legal-probe" | "finance-show"
        | "finance-balances" | "finance-probe" | "orgintel-init" | "teams" | "spend"
        | "telemetry" | "goals" | "work" | "work-graph" | "clear-poison" | "attention"
        | "browser-status" | "browser-release" | "watch" | "connected-tools" | "identity-show"
        | "publish-list" => &[],
        "schedule-wake" => &["adapter"],
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
        "goal-add" => &["title", "body", "actor"],
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
            "requires",
            "revises",
            "gates",
            "constitution_contracts",
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
            "actor",
        ],
        "work-artifact-retire" => &["id", "reason", "actor"],
        "work-handoff-refresh" => &["id", "as_actor", "action", "prepared", "resume_when"],
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
        "events" => &["limit"],
        "schedule-list" => &["as_actor", "include_fired"],
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
            "local_time",
            "timezone",
            "missed_policy",
            "catch_up_grace_seconds",
            "execution_requirement",
            "reason",
            "id",
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
        "effect-reconcile" => &["key", "execution_no", "state", "id", "actor"],
        "connected-tool-install" | "connected-tool-reconnect" => &[
            "tool_name",
            "endpoint",
            "purpose",
            "assigned_actor",
            "work_id",
            "attempt_id",
            "requested_scopes",
            "actor",
        ],
        "connected-tool-observe" => &[
            "tool_name",
            "workspace_reference",
            "observed_tools",
            "actor",
        ],
        "connected-tool-disable" => &["tool_name", "actor"],
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
pub(crate) enum Principal {
    Owner,
    CompanyExec,
}

impl Principal {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::CompanyExec => "company/exec",
        }
    }
}

/// Commands that widen authority, change company lifecycle, or resolve the
/// owner's review boundary. This is a finite V0 list, not a policy DSL.
pub(crate) const OWNER_ONLY: &[&str] = &[
    "approve",
    "decline",
    "revoke",
    "up",
    "down",
    "clear-poison",
    "spend-correct",
    "company-create",
    "company-set",
    "company-unset",
    "credential-set",
    "credential-promote",
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
];

pub(crate) fn authorize(principal: Principal, cmd: &str) -> std::result::Result<Principal, String> {
    if principal != Principal::Owner && OWNER_ONLY.contains(&cmd) {
        return Err(format!(
            "{cmd} is an act of owner authority; principal {} may not perform it",
            principal.as_str()
        ));
    }
    Ok(principal)
}

impl Principal {
    /// Old images still send company/exec. It is compatibility syntax only:
    /// the listener capability, not this field, supplies real authority.
    pub(crate) fn legacy_runtime_claim(raw: Option<&str>) -> std::result::Result<(), String> {
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
pub(crate) struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorBody>,
}

impl Response {
    pub(crate) fn ok(data: impl Into<serde_json::Value>) -> Self {
        Self {
            ok: true,
            data: Some(data.into()),
            error: None,
        }
    }

    pub(crate) fn ok_serialized(data: impl serde::Serialize) -> Self {
        match serde_json::to_value(data) {
            Ok(data) => Self::ok(data),
            Err(error) => Self::err(format!("encode response: {error}")),
        }
    }

    pub(crate) fn err(message: impl Into<String>) -> Self {
        Self::err_kind("error", message)
    }

    pub(crate) fn err_kind(kind: impl Into<String>, message: impl Into<String>) -> Self {
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
    use super::{command_fields, Request};

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
    fn every_dispatch_command_has_a_checked_domain_view() {
        // Kept next to the boundary rather than parsed from Rust source: the
        // dispatcher legitimately contains nested string matches such as
        // company config keys, which are not transport commands.
        const COMMANDS: &[&str] = &[
            "up",
            "down",
            "status",
            "doctor",
            "company-list",
            "company-create",
            "company-show",
            "company-set",
            "company-unset",
            "credential-set",
            "credential-promote",
            "credential-check",
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
            "work-goal",
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
            "browser-request",
            "browser-release",
            "effect",
            "effect-reconcile",
            "watch",
            "connected-tools",
            "connected-tool-install",
            "connected-tool-reconnect",
            "connected-tool-observe",
            "connected-tool-disable",
        ];
        for command in COMMANDS {
            assert!(
                command_fields(command).is_some(),
                "dispatch command {command:?} has no checked input view"
            );
        }
    }

    #[test]
    fn connected_tool_install_decodes_one_authority_owned_purpose() {
        let request = Request::decode(
            r#"{"cmd":"connected-tool-install","company":"exp12_attio_test","tool_name":"attio","endpoint":"https://mcp.attio.com/mcp","purpose":"Operate the tutoring-centre pipeline","assigned_actor":"crm-operations","work_id":"ebc5691f-f865-402c-8b31-d8389b5a9ea7","attempt_id":"59b6fd81-438d-48cd-992c-ccd4b0c7eb3f","requested_scopes":["openid","offline_access","mcp"],"actor":"crm-operations"}"#,
        )
        .expect("decode connected-tool install");

        assert_eq!(
            request.authority.purpose.as_deref(),
            Some("Operate the tutoring-centre pipeline")
        );
        assert_eq!(request.connected_tool.tool_name.as_deref(), Some("attio"));
        assert_eq!(
            request.connected_tool.assigned_actor.as_deref(),
            Some("crm-operations")
        );
        assert_eq!(request.orgintel.actor.as_deref(), Some("crm-operations"));
        assert_eq!(
            request.connected_tool.requested_scopes,
            ["openid", "offline_access", "mcp"]
        );
    }
}
