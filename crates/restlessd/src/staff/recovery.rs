//! Attempt completion, terminal observations, and recovery capsules.
//!
//! These functions turn a finished Runtime process into OrgIntel's existing
//! Work/Attempt facts. They do not own agent execution or add a workflow.

use anyhow::{Context as _, Result};
use restless_orgintel::{NewAttemptRecovery, WorkAttemptState, WorkStatus};
use sha2::{Digest as _, Sha256};

use crate::exec::Termination;

use super::workspace::{observe_workspace, WorkspaceObservation};

/// Close the exact Attempt that launched this process. Completion is accepted
/// only after its declared artifact and deterministic gates are observed.
pub(super) struct StaffAttemptContext<'a> {
    pub(super) container: &'a str,
    pub(super) actor: &'a str,
    pub(super) name: &'a str,
    pub(super) work_id: uuid::Uuid,
    pub(super) attempt_id: uuid::Uuid,
    pub(super) workdir: &'a str,
    pub(super) start_observation: WorkspaceObservation,
}

pub(super) async fn record_unknown_recovery(
    org: &restless_orgintel::OrgIntel,
    attempt_id: uuid::Uuid,
    reason: &str,
    start: &WorkspaceObservation,
    end: &WorkspaceObservation,
) -> Result<()> {
    org.ensure_actor("daemon", "system", "system-sender", "The daemon")
        .await?;
    let start_value = serde_json::to_value(start)?;
    let end_value = serde_json::to_value(end)?;
    let start_summary = start.compact();
    let end_summary = end.compact();
    let fingerprint = end.fingerprint();
    org.record_unknown_attempt_recovery(
        attempt_id,
        NewAttemptRecovery {
            observed_by: "daemon",
            reason,
            workspace: &end.workdir,
            start_observation: &start_value,
            end_observation: &end_value,
            start_summary: &start_summary,
            end_summary: &end_summary,
            changed_since_start: end.changed_since(start),
            observation_digest: fingerprint.as_deref(),
            end_commit: end.source_commit.as_deref(),
        },
    )
    .await?;
    Ok(())
}

async fn record_terminal_observation(
    org: &restless_orgintel::OrgIntel,
    actor: &str,
    work_id: uuid::Uuid,
    attempt_id: uuid::Uuid,
    semantic_result: &str,
    start: &WorkspaceObservation,
    end: &WorkspaceObservation,
) {
    if let Err(error) = org
        .emit_event(
            "attempt_process_ended",
            Some(actor),
            serde_json::json!({
                "work_id": work_id,
                "attempt_id": attempt_id,
                "semantic_result": semantic_result,
                "workspace": { "start": start, "end": end },
            }),
        )
        .await
    {
        tracing::warn!(%error, %attempt_id, "failed to record terminal process observation");
    }
}

pub(super) async fn record_staff_outcome(
    org: &restless_orgintel::OrgIntel,
    context: StaffAttemptContext<'_>,
    outcome: Result<(Termination, String)>,
) {
    let StaffAttemptContext {
        container,
        actor,
        name,
        work_id,
        attempt_id,
        workdir,
        start_observation,
    } = context;
    let end_observation = observe_workspace(container, workdir).await;
    let semantic_result = match &outcome {
        Ok((Termination::OutcomeMet, _)) => Some("reported_outcome_met"),
        Ok((Termination::ChangesRequested, _)) => Some("changes_requested"),
        Ok((Termination::Blocked, _)) => Some("blocked"),
        Ok((Termination::Abandon, _)) => Some("abandoned"),
        Ok((Termination::Continue, _)) | Err(_) => None,
    };
    if let Some(semantic_result) = semantic_result {
        record_terminal_observation(
            org,
            actor,
            work_id,
            attempt_id,
            semantic_result,
            &start_observation,
            &end_observation,
        )
        .await;
    }
    let record = async {
        match outcome {
            Ok((Termination::OutcomeMet, summary)) => {
                finish_claimed_attempt(
                    org,
                    container,
                    work_id,
                    attempt_id,
                    workdir,
                    Termination::OutcomeMet,
                    &summary,
                )
                .await?;
                if let Some(work) = org.get_work(work_id).await? {
                    if work.status == WorkStatus::Blocked {
                        let coordinator = org
                            .team_lead_for(&work.owner_id)
                            .await?
                            .unwrap_or_else(|| "exec".to_string());
                        org.send_message(
                            actor,
                            Some(&coordinator),
                            &format!(
                                "{name} could not pass completion gates for Work {work_id}: {}",
                                work.resolution
                            ),
                        )
                        .await?;
                    }
                }
            }
            Ok((Termination::ChangesRequested, summary)) => {
                finish_claimed_attempt(
                    org,
                    container,
                    work_id,
                    attempt_id,
                    workdir,
                    Termination::ChangesRequested,
                    &summary,
                )
                .await?
            }
            Ok((Termination::Blocked, summary)) => {
                let effective = org
                    .finish_work_attempt(attempt_id, WorkAttemptState::Blocked, &summary)
                    .await?;
                if effective == WorkAttemptState::Blocked {
                    let coordinator = org
                        .team_lead_for(actor)
                        .await?
                        .unwrap_or_else(|| "exec".to_string());
                    org.send_message(
                        actor,
                        Some(&coordinator),
                        &format!("{name} blocked on Work {work_id}: {summary}"),
                    )
                    .await?;
                }
            }
            Ok((Termination::Abandon, summary)) => {
                org.finish_work_attempt(attempt_id, WorkAttemptState::Abandoned, &summary)
                    .await?;
            }
            Ok((Termination::Continue, summary)) => {
                let reason =
                    format!("cognitive session ended after requesting continuation: {summary}");
                record_unknown_recovery(
                    org,
                    attempt_id,
                    &reason,
                    &start_observation,
                    &end_observation,
                )
                .await?;
            }
            Err(error) => {
                let reason = format!(
                    "cognitive process ended before a trustworthy semantic result: {error:#}"
                );
                org.emit_event(
                    "staff_crash",
                    Some(actor),
                    serde_json::json!({ "error": format!("{error:#}"), "worktree": workdir }),
                )
                .await?;
                record_unknown_recovery(
                    org,
                    attempt_id,
                    &reason,
                    &start_observation,
                    &end_observation,
                )
                .await?;
            }
        }
        anyhow::Ok(())
    };
    if let Err(error) = record.await {
        tracing::error!(staff = name, "failed to record staff outcome: {error:#}");
    }
}

/// Apply one Staff actor's structured result to its claimed Attempt. Leads
/// and Exec never reach this productive path.
pub(super) async fn finish_claimed_attempt(
    org: &restless_orgintel::OrgIntel,
    container: &str,
    work_id: uuid::Uuid,
    attempt_id: uuid::Uuid,
    workdir: &str,
    termination: Termination,
    summary: &str,
) -> Result<()> {
    match termination {
        Termination::OutcomeMet => {
            let work = org
                .get_work(work_id)
                .await?
                .context("claimed Work disappeared")?;
            let artifacts = org.list_artifact_refs(Some(work_id)).await?;
            let observed = work.expected_artifact.trim().is_empty()
                || artifacts.iter().any(|artifact| {
                    artifact.attempt_id == Some(attempt_id)
                        && artifact.state == restless_orgintel::ArtifactRefState::Available
                });
            if !observed {
                org.finish_work_attempt(
                    attempt_id,
                    WorkAttemptState::Failed,
                    &format!(
                        "declared complete without linking expected artifact: {}",
                        work.expected_artifact
                    ),
                )
                .await?;
            } else if run_gates(org, container, work_id, attempt_id, workdir).await? {
                org.finish_work_attempt(attempt_id, WorkAttemptState::Produced, summary)
                    .await?;
            } else {
                org.finish_work_attempt(
                    attempt_id,
                    WorkAttemptState::Failed,
                    "one or more deterministic Work gates failed",
                )
                .await?;
            }
        }
        Termination::ChangesRequested => {
            org.finish_work_attempt(attempt_id, WorkAttemptState::ChangesRequested, summary)
                .await?;
        }
        Termination::Blocked => {
            org.finish_work_attempt(attempt_id, WorkAttemptState::Blocked, summary)
                .await?;
        }
        Termination::Abandon => {
            org.finish_work_attempt(attempt_id, WorkAttemptState::Abandoned, summary)
                .await?;
        }
        Termination::Continue => {
            org.finish_work_attempt(attempt_id, WorkAttemptState::Failed, summary)
                .await?;
        }
    }
    Ok(())
}

async fn run_gates(
    org: &restless_orgintel::OrgIntel,
    container: &str,
    work_id: uuid::Uuid,
    attempt_id: uuid::Uuid,
    workdir: &str,
) -> Result<bool> {
    let gates = org.list_work_gates(work_id).await?;
    for gate in gates {
        let argv: Vec<String> = serde_json::from_value(gate.command.clone())
            .with_context(|| format!("gate {} has invalid argv", gate.name))?;
        let (program, args) = argv.split_first().context("gate command is empty")?;
        let cwd = gate_cwd(&gate.cwd, workdir);
        let output = tokio::process::Command::new("docker")
            .args(["exec", "-u", "company", "-w", cwd, container, program])
            .args(args)
            .output()
            .await
            .with_context(|| format!("run gate {}", gate.name))?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let digest = format!("{:x}", Sha256::digest(combined.as_bytes()));
        org.record_gate_run(restless_orgintel::NewGateRun {
            gate_id: gate.id,
            attempt_id,
            exit_code: output.status.code(),
            output_digest: &digest,
            output_excerpt: &combined.chars().take(2_000).collect::<String>(),
            passed: output.status.success(),
        })
        .await?;
    }
    Ok(org.gates_passed(work_id, attempt_id).await?)
}

pub(super) fn gate_cwd<'a>(declared: &'a str, attempt_workdir: &'a str) -> &'a str {
    if declared == "@attempt" {
        attempt_workdir
    } else {
        declared
    }
}
