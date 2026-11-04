//! Read-only record of structured decisions: each with its probabilities and, where the company
//! later showed the answer, the outcome. The summary is what Company › Intelligence shows beside
//! the decision model (S107): when one last ran, the Exec wake-ups it saved, the ones it would
//! save in shadow, and whether it would ever have skipped a wake that became an owner update.

use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct DecisionsQuery {
    kind: Option<String>,
    limit: Option<i64>,
}

pub(super) async fn list(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<DecisionsQuery>,
) -> Response<Body> {
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                logged(error, "Decisions could not be read. Try again."),
            );
        }
    };
    let decisions = match org
        .recent_decisions(query.kind.as_deref(), query.limit.unwrap_or(100))
        .await
    {
        Ok(decisions) => decisions,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                logged(error, "Decisions could not be read. Try again."),
            );
        }
    };
    Json(serde_json::json!({ "summary": summary(&decisions), "decisions": decisions }))
        .into_response()
}

/// Over the given decisions: the Exec gate's record and how Work was tiered.
pub(super) fn summary(decisions: &[restless_orgintel::DecisionRow]) -> serde_json::Value {
    let gate = |row: &&restless_orgintel::DecisionRow| row.kind == "exec_gate";
    // Settled wakes the owner's own message did not force.
    let settled: Vec<_> = decisions
        .iter()
        .filter(gate)
        .filter(|row| matches!(row.outcome.as_deref(), Some("quiet" | "update")))
        .collect();
    let count = |rows: &[&restless_orgintel::DecisionRow], choice: &str, outcome: &str| {
        rows.iter()
            .filter(|row| row.choice == choice && row.outcome.as_deref() == Some(outcome))
            .count()
    };
    let tier = |choice: &str| {
        decisions
            .iter()
            .filter(|row| row.kind == "work_tier" && row.choice == choice)
            .count()
    };
    // Team leads' turns as routed (owner, 4 Nov): by rule or by the decision model.
    let lead = |choice: &str| {
        decisions
            .iter()
            .filter(|row| row.kind == "lead_tier" && row.choice == choice)
            .count()
    };
    serde_json::json!({
        "last_ran_at": decisions
            .iter()
            // The decision model's own runs: a lead turn routed by rule asked no model.
            .filter(|row| row.kind != "staff" && row.kind != "lead_escalation" && row.model != "rule")
            .map(|row| row.created_at)
            .max(),
        "exec_gate": {
            "decided": decisions.iter().filter(gate).count(),
            // Consumed without an Exec wake.
            "saved_wakes": decisions
                .iter()
                .filter(gate)
                .filter(|row| row.mode == "active" && row.choice == "skip_wake")
                .count(),
            "settled_wakes": settled.len(),
            "quiet_wakes": settled.iter().filter(|row| row.outcome.as_deref() == Some("quiet")).count(),
            "would_save": count(&settled, "skip_wake", "quiet"),
            "would_miss_update": count(&settled, "skip_wake", "update"),
            "pending": decisions.iter().filter(gate).filter(|row| row.outcome.is_none() && row.mode == "shadow").count(),
        },
        "work_tier": { "everyday": tier("everyday"), "primary": tier("primary") },
        "lead_tier": {
            "everyday": lead("everyday"),
            "main": lead("main"),
            "escalated": decisions
                .iter()
                .filter(|row| row.kind == "lead_tier" && row.outcome.as_deref() == Some("escalated"))
                .count(),
        },
    })
}
