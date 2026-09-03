//! Read-only decision telemetry assembled from the Runtime's canonical facts.
//!
//! This module owns no lifecycle and writes no aggregate. OrgIntel remains the
//! source for Work, Attempts, gates, messages and events; the model ledger
//! remains the source for charged requests. An unavailable measurement is
//! represented as `None`, never as an invented zero.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use chrono::Utc;
use restless_model_gateway::SpendRecord;
use serde::Serialize;
use uuid::Uuid;

use crate::spend::SpendLedger;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct SettlementTelemetry {
    request_count: usize,
    uncertain_request_count: usize,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    total_tokens: Option<u64>,
    cached_input_tokens: Option<u64>,
    accounted_cost_micro_usd: u64,
    settled_cost_micro_usd: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct AttemptDecisionTelemetry {
    work_id: Uuid,
    attempt_id: Uuid,
    actor_id: String,
    elapsed_ms: Option<i64>,
    active_model_ms: Option<i64>,
    model: Option<String>,
    settlement: SettlementTelemetry,
    tool_failures: Option<u64>,
    gate_executions: usize,
    gate_cache_hits: usize,
    process_replacements: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct AttributionTelemetry {
    id: String,
    settlement: SettlementTelemetry,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct DuplicateWorkTelemetry {
    duplicate_count: usize,
    groups: Vec<Vec<Uuid>>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct DecisionTelemetry {
    company: String,
    generated_at: chrono::DateTime<Utc>,
    company_settlement: SettlementTelemetry,
    by_actor: Vec<AttributionTelemetry>,
    by_work: Vec<AttributionTelemetry>,
    attempts: Vec<AttemptDecisionTelemetry>,
    supervisor_wakes: usize,
    supervisor_interventions: usize,
    duplicated_work: DuplicateWorkTelemetry,
    process_replacements: usize,
    unknown_measurements: Vec<String>,
}

fn settlement(exact: &[SpendRecord], unknown: &[SpendRecord]) -> SettlementTelemetry {
    let exact_split_known = !exact.is_empty()
        && exact.iter().all(|record| {
            record.input_tokens != 0 || record.output_tokens != 0 || record.total_tokens == 0
        });
    let exact_cached_known = !exact.is_empty()
        && exact
            .iter()
            .all(|record| record.cached_input_tokens.is_some());
    let fully_settled = unknown.is_empty();
    let sum = |value: fn(&SpendRecord) -> u64| exact.iter().map(value).sum::<u64>();
    let accounted_cost_micro_usd = sum(|record| record.cost_micro_usd);
    SettlementTelemetry {
        request_count: exact.len(),
        uncertain_request_count: unknown.len(),
        input_tokens: (fully_settled && exact_split_known)
            .then(|| sum(|record| record.input_tokens)),
        output_tokens: (fully_settled && exact_split_known)
            .then(|| sum(|record| record.output_tokens)),
        total_tokens: (fully_settled && !exact.is_empty())
            .then(|| sum(|record| record.total_tokens)),
        cached_input_tokens: (fully_settled && exact_cached_known)
            .then(|| sum(|record| record.cached_input_tokens.unwrap_or_default())),
        accounted_cost_micro_usd,
        settled_cost_micro_usd: fully_settled.then_some(accounted_cost_micro_usd),
    }
}

fn grouped_settlement(
    exact: &[SpendRecord],
    unknown: &[SpendRecord],
    key: impl Fn(&SpendRecord) -> Option<String>,
) -> Vec<AttributionTelemetry> {
    let mut ids = BTreeSet::new();
    for record in exact.iter().chain(unknown) {
        if let Some(id) = key(record) {
            ids.insert(id);
        }
    }
    ids.into_iter()
        .map(|id| {
            let exact_rows = exact
                .iter()
                .filter(|record| key(record).as_deref() == Some(id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            let unknown_rows = unknown
                .iter()
                .filter(|record| key(record).as_deref() == Some(id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            AttributionTelemetry {
                id,
                settlement: settlement(&exact_rows, &unknown_rows),
            }
        })
        .collect()
}

pub(crate) async fn collect(
    company: &str,
    org: &restless_orgintel::OrgIntel,
    spend: &SpendLedger,
) -> Result<DecisionTelemetry> {
    let graph = org.work_graph_snapshot().await?;
    let events = org.events_after(0).await?;
    let teams = org.list_teams().await?;
    let lead_ids = teams
        .iter()
        .map(|team| team.lead_actor_id.clone())
        .collect::<BTreeSet<_>>();
    let lead_list = lead_ids.iter().cloned().collect::<Vec<_>>();
    let excluded = ["owner", "exec", "world", "daemon"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let supervisor_interventions = org
        .count_internal_messages_from(&lead_list, &excluded)
        .await?
        .try_into()
        .unwrap_or_default();
    let exact = spend.records(company);
    let unknown = spend.unknown_requests(company);

    let replacement_events = events
        .iter()
        .filter(|event| matches!(event.kind.as_str(), "model_failover" | "staff_crash"))
        .collect::<Vec<_>>();
    let process_replacements = replacement_events.len();
    let supervisor_wakes = events
        .iter()
        .filter(|event| {
            event.kind == "actor_wake_end"
                && event
                    .actor_id
                    .as_ref()
                    .is_some_and(|actor| lead_ids.contains(actor))
        })
        .count();

    let mut attempts = Vec::with_capacity(graph.attempts.len());
    for attempt in &graph.attempts {
        let attempt_id = attempt.id.to_string();
        let exact_rows = exact
            .iter()
            .filter(|record| record.attempt_id == Some(attempt.id))
            .cloned()
            .collect::<Vec<_>>();
        let unknown_rows = unknown
            .iter()
            .filter(|record| record.attempt_id == Some(attempt.id))
            .cloned()
            .collect::<Vec<_>>();
        let gate_runs = graph
            .gate_runs
            .iter()
            .filter(|run| run.attempt_id == attempt.id)
            .collect::<Vec<_>>();
        let replacements = replacement_events
            .iter()
            .filter(|event| {
                event
                    .body
                    .pointer("/attempt_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(attempt_id.as_str())
                    || event.actor_id.as_deref() == Some(attempt.actor_id.as_str())
            })
            .count();
        attempts.push(AttemptDecisionTelemetry {
            work_id: attempt.work_id,
            attempt_id: attempt.id,
            actor_id: attempt.actor_id.clone(),
            elapsed_ms: attempt
                .finished_at
                .map(|finished| (finished - attempt.started_at).num_milliseconds()),
            active_model_ms: None,
            model: attempt.model.clone(),
            settlement: settlement(&exact_rows, &unknown_rows),
            tool_failures: None,
            gate_executions: gate_runs.len(),
            gate_cache_hits: gate_runs
                .iter()
                .filter(|run| run.cache_source_run_id.is_some())
                .count(),
            process_replacements: replacements,
        });
    }

    let mut duplicate_groups = BTreeMap::<(String, String, String), Vec<Uuid>>::new();
    for work in &graph.work {
        if work.status == restless_orgintel::WorkStatus::Abandoned {
            continue;
        }
        duplicate_groups
            .entry((
                work.owner_id.trim().to_lowercase(),
                work.title
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase(),
                work.outcome
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase(),
            ))
            .or_default()
            .push(work.id);
    }
    let groups = duplicate_groups
        .into_values()
        .filter(|ids| ids.len() > 1)
        .collect::<Vec<_>>();
    let duplicate_count = groups.iter().map(|ids| ids.len() - 1).sum();

    Ok(DecisionTelemetry {
        company: company.to_string(),
        generated_at: Utc::now(),
        company_settlement: settlement(&exact, &unknown),
        by_actor: grouped_settlement(&exact, &unknown, |record| {
            (!record.actor_id.is_empty()).then(|| record.actor_id.clone())
        }),
        by_work: grouped_settlement(&exact, &unknown, |record| {
            record.work_id.map(|id| id.to_string())
        }),
        attempts,
        supervisor_wakes,
        supervisor_interventions,
        duplicated_work: DuplicateWorkTelemetry {
            duplicate_count,
            groups,
        },
        process_replacements,
        unknown_measurements: vec![
            "active_model_ms (provider/runtime timing is not emitted)".into(),
            "tool_failures (tool-call outcomes are not yet canonical Runtime events)".into(),
            "input/output token split for pi-native totals".into(),
            "cached input tokens when the provider does not report them".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use restless_model_gateway::SpendSettlement;

    fn record(cost: u64) -> SpendRecord {
        SpendRecord {
            request_id: Uuid::new_v4(),
            company_id: "acme".into(),
            model: "litellm/gpt-5.6-sol".into(),
            input_tokens: 100,
            output_tokens: 20,
            total_tokens: 120,
            cached_input_tokens: Some(40),
            cost_micro_usd: cost,
            actor_id: "worker".into(),
            session_id: "session".into(),
            responsibility: "work:test".into(),
            work_id: Some(Uuid::new_v4()),
            attempt_id: Some(Uuid::new_v4()),
            settlement: SpendSettlement::Accounted,
            occurred_at: Utc::now(),
        }
    }

    #[test]
    fn missing_measurement_is_unknown_and_request_uncertainty_does_not_erase_exact_cost() {
        let exact = [record(7), record(11)];
        let mut unknown = record(0);
        unknown.settlement = SpendSettlement::MeteringUnknown;
        let telemetry = settlement(&exact, &[unknown]);
        assert_eq!(telemetry.accounted_cost_micro_usd, 18);
        assert_eq!(telemetry.settled_cost_micro_usd, None);
        assert_eq!(telemetry.total_tokens, None);
        assert_eq!(telemetry.uncertain_request_count, 1);

        let complete = settlement(&exact, &[]);
        assert_eq!(complete.settled_cost_micro_usd, Some(18));
        assert_eq!(complete.total_tokens, Some(240));
        assert_eq!(complete.cached_input_tokens, Some(80));
    }

    #[tokio::test]
    async fn collector_reconciles_runtime_facts_without_inventing_missing_measurements() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping telemetry projection scenario");
            return;
        };
        let company = format!("telemetry{}", Uuid::new_v4().simple());
        let org = restless_orgintel::OrgIntel::ensure(&database_url, &company)
            .await
            .unwrap();
        org.ensure_actor("exec", "exec", "exec", "The Exec")
            .await
            .unwrap();
        org.create_actor(
            "delivery-direction",
            "lead",
            "Morgan Vale",
            None,
            "exec",
            "supervises delivery",
        )
        .await
        .unwrap();
        let team = org
            .create_team(
                "Delivery",
                "Deliver one exact outcome",
                "delivery-direction",
                "exec",
            )
            .await
            .unwrap();
        org.create_actor(
            "delivery-worker",
            "builder",
            "Riley Chen",
            None,
            "delivery-direction",
            "produces bounded delivery Work",
        )
        .await
        .unwrap();
        org.set_actor_team(
            "delivery-worker",
            Some(team),
            "delivery-direction",
            "assigned",
        )
        .await
        .unwrap();
        for message in org.inbox(Some("delivery-direction")).await.unwrap() {
            org.mark_read(message.id).await.unwrap();
        }
        let new_work = || restless_orgintel::NewWork {
            owner_id: "delivery-worker",
            title: "Telemetry candidate",
            outcome: "one exact telemetry candidate",
            goal_id: None,
            priority: 10,
            expected_artifact: "",
            workspace: restless_orgintel::WorkspaceSpec::default(),
            attempt_limit: Some(1),
        };
        let work_id = org.add_work(new_work()).await.unwrap();
        let mut duplicate = new_work();
        duplicate.priority = 9;
        org.add_work(duplicate).await.unwrap();
        let gate_id = org
            .add_work_gate(restless_orgintel::NewWorkGate {
                work_id,
                name: "telemetry-pass",
                cwd: "@attempt",
                command: &["true".into()],
                created_by: "delivery-direction",
            })
            .await
            .unwrap();
        let attempt = org.claim_ready_work("telemetry").await.unwrap().unwrap();
        org.record_gate_run(restless_orgintel::NewGateRun {
            gate_id,
            attempt_id: attempt.attempt_id,
            exit_code: Some(0),
            output_digest: "sha256:telemetry-pass",
            output_excerpt: "pass",
            passed: true,
        })
        .await
        .unwrap();
        org.finish_work_attempt(
            attempt.attempt_id,
            restless_orgintel::WorkAttemptState::Produced,
            "telemetry fixture passed",
        )
        .await
        .unwrap();
        org.emit_event(
            "actor_wake_end",
            Some("delivery-direction"),
            serde_json::json!({"reason":"material fixture settled"}),
        )
        .await
        .unwrap();
        org.send_work_message(
            "delivery-direction",
            "delivery-worker",
            work_id,
            "Revise the exact bounded issue.",
        )
        .await
        .unwrap();

        let root = std::env::temp_dir().join(format!("restless-telemetry-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let ledger = SpendLedger::open(&root).unwrap();
        let mut exact = record(19);
        exact.company_id = company.clone();
        exact.actor_id = "delivery-worker".into();
        exact.work_id = Some(work_id);
        exact.attempt_id = Some(attempt.attempt_id);
        ledger.meter().record_exact(exact);

        let report = collect(&company, &org, &ledger).await.unwrap();
        assert_eq!(report.company_settlement.settled_cost_micro_usd, Some(19));
        assert_eq!(report.supervisor_wakes, 1);
        assert_eq!(report.supervisor_interventions, 1);
        assert_eq!(report.duplicated_work.duplicate_count, 1);
        let attempt_report = report
            .attempts
            .iter()
            .find(|row| row.attempt_id == attempt.attempt_id)
            .unwrap();
        assert!(attempt_report.elapsed_ms.is_some());
        assert_eq!(attempt_report.active_model_ms, None);
        assert_eq!(attempt_report.tool_failures, None);
        assert_eq!(attempt_report.gate_executions, 1);
        assert_eq!(attempt_report.gate_cache_hits, 0);

        org.drop_schema().await.unwrap();
        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }
}
