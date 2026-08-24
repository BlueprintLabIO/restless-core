//! Stable JSON-lines transport types, grouped by the plane that owns each
//! input. The wire stays flat for the existing CLI and company runtime; the
//! daemon no longer grows one universal all-optional request bag.

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

/// The header determines routing and authority; flattened domain inputs keep
/// the existing socket/CLI JSON source-compatible without pretending that
/// every domain field belongs to every command.
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
    use super::Request;

    #[test]
    fn flat_json_decodes_into_the_owner_domain_without_a_universal_payload() {
        let request: Request = serde_json::from_str(
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
    }
}
