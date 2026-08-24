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
    /// Rebuild and reconcile the Company Runtime image. Docker remains a
    /// Runtime implementation detail, not part of an owner transcript.
    #[serde(default)]
    pub(crate) reconcile: bool,
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

#[derive(Debug, Deserialize)]
pub(crate) struct InitialWorkGateRequest {
    pub(crate) name: String,
    pub(crate) command: Vec<String>,
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
    pub(crate) requires: Vec<String>,
    #[serde(default)]
    pub(crate) revises: Vec<String>,
    #[serde(default)]
    pub(crate) gates: Vec<InitialWorkGateRequest>,
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
    pub(crate) orgintel: OrgIntelInput,
    #[serde(flatten)]
    pub(crate) owner: OwnerInput,
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
        serde_json::from_value(value).map_err(|error| format!("decode {command:?}: {error}"))
    }
}

/// This is a decoder allowlist, not a second command algebra: the dispatcher
/// remains the only command behaviour and source owner. Each row merely names
/// the already-existing fields that cross its concrete domain boundary.
fn command_fields(command: &str) -> Option<&'static [&'static str]> {
    Some(match command {
        "company-list" | "status" | "doctor" | "company-show" | "credential-check"
        | "legal-show" | "legal-probe" | "finance-show" | "finance-balances" | "finance-probe"
        | "orgintel-init" | "teams" | "spend" | "goals" | "work" | "work-graph"
        | "clear-poison" | "attention" | "browser-status" | "browser-release" | "watch" => &[],
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
        "team-create" => &["name", "to", "body", "actor"],
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
            "goal",
            "priority",
            "expected_artifact",
            "repo",
            "base_ref",
            "integration_branch",
            "worktree",
            "attempt_limit",
            "owner_review",
            "requires",
            "revises",
            "gates",
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
        "work-gate" => &["id", "name", "cwd", "argv", "actor"],
        "work-handoff" => &[
            "id",
            "attempt",
            "category",
            "action",
            "prepared",
            "resume_when",
            "actor",
        ],
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
        "work-resume" | "work-abandon" => &["id", "as_actor", "reason"],
        "work-review" => &["id", "state", "resolution"],
        "inbox" => &["actor", "as_actor"],
        "message" => &["from", "to", "id", "body"],
        "events" => &["limit"],
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
                "gates":[{"name":"review-target-live-probe","command":["test","-s","report.html"]}]
            }"#,
        )
        .expect("decode explicit owner-review Work contract");
        assert!(request.orgintel.owner_review);
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
            "work-gate",
            "work-handoff",
            "work-handoff-refresh",
            "work-handoff-prepare-brief",
            "work-handoff-resolve",
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
        ];
        for command in COMMANDS {
            assert!(
                command_fields(command).is_some(),
                "dispatch command {command:?} has no checked input view"
            );
        }
    }
}
