//! Read-only record of structured decisions: each with its probabilities and, where the company
//! later showed the answer, the outcome. The summary answers the questions a decision must pass
//! before it may act: how often the Exec wakes for nothing, and whether the gate would ever have
//! skipped a wake that turned into an owner update.

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
                format!("{error:#}"),
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
                format!("{error:#}"),
            );
        }
    };
    Json(serde_json::json!({ "summary": exec_gate_summary(&decisions), "decisions": decisions }))
        .into_response()
}

/// Exec gate, over decisions whose wake has finished without an owner message forcing a reply.
fn exec_gate_summary(decisions: &[restless_orgintel::DecisionRow]) -> serde_json::Value {
    let settled: Vec<_> = decisions
        .iter()
        .filter(|row| {
            row.kind == "exec_gate" && matches!(row.outcome.as_deref(), Some("quiet" | "update"))
        })
        .collect();
    let quiet = settled
        .iter()
        .filter(|row| row.outcome.as_deref() == Some("quiet"))
        .count();
    let would_save = settled
        .iter()
        .filter(|row| row.choice == "skip_wake" && row.outcome.as_deref() == Some("quiet"))
        .count();
    let would_miss = settled
        .iter()
        .filter(|row| row.choice == "skip_wake" && row.outcome.as_deref() == Some("update"))
        .count();
    serde_json::json!({
        "exec_gate": {
            "settled_wakes": settled.len(),
            "quiet_wakes": quiet,
            "would_save": would_save,
            "would_miss_update": would_miss,
            "pending": decisions.iter().filter(|row| row.kind == "exec_gate" && row.outcome.is_none()).count(),
        }
    })
}
