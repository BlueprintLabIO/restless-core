//! Attempt completion, terminal observations, and recovery capsules.
//!
//! These functions turn a finished Runtime process into OrgIntel's existing
//! Work/Attempt facts. They do not own agent execution or add a workflow.

use anyhow::{Context as _, Result};
use restless_orgintel::{NewAttemptRecovery, WorkAttemptState};
use sha2::{Digest as _, Sha256};

use crate::exec::Termination;

use super::workspace::{observe_workspace, promote_integration_commit, WorkspaceObservation};

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

struct AttemptCompletionContext<'a> {
    container: &'a str,
    work_id: uuid::Uuid,
    attempt_id: uuid::Uuid,
    workdir: &'a str,
    end_observation: &'a WorkspaceObservation,
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
        if matches!(&outcome, Ok((Termination::OutcomeMet, _)))
            && end_observation.dirty_entries == 0
        {
            if let Some(commit) = end_observation.source_commit.as_deref() {
                org.bind_attempt_artifacts_to_observed_commit(attempt_id, commit)
                    .await?;
            }
        }
        let terminal_fact_recorded = match outcome {
            Ok((Termination::OutcomeMet, summary)) => {
                finish_claimed_attempt(
                    org,
                    AttemptCompletionContext {
                        container,
                        work_id,
                        attempt_id,
                        workdir,
                        end_observation: &end_observation,
                    },
                    Termination::OutcomeMet,
                    &summary,
                )
                .await?;
                true
            }
            Ok((Termination::ChangesRequested, summary)) => {
                finish_claimed_attempt(
                    org,
                    AttemptCompletionContext {
                        container,
                        work_id,
                        attempt_id,
                        workdir,
                        end_observation: &end_observation,
                    },
                    Termination::ChangesRequested,
                    &summary,
                )
                .await?;
                true
            }
            Ok((Termination::Blocked, summary)) => {
                org.finish_work_attempt(attempt_id, WorkAttemptState::Blocked, &summary)
                    .await?;
                true
            }
            Ok((Termination::Abandon, summary)) => {
                org.finish_work_attempt(attempt_id, WorkAttemptState::Abandoned, &summary)
                    .await?;
                true
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
                false
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
                false
            }
        };
        if terminal_fact_recorded {
            org.flush_terminal_supervisor_notices(16).await?;
        }
        anyhow::Ok(())
    };
    if let Err(error) = record.await {
        tracing::error!(staff = name, "failed to record staff outcome: {error:#}");
    }
}

/// Apply one Staff actor's structured result to its claimed Attempt. Leads
/// and Exec never reach this productive path.
async fn finish_claimed_attempt(
    org: &restless_orgintel::OrgIntel,
    context: AttemptCompletionContext<'_>,
    termination: Termination,
    summary: &str,
) -> Result<()> {
    let AttemptCompletionContext {
        container,
        work_id,
        attempt_id,
        workdir,
        end_observation,
    } = context;
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
            let exact_repository_artifact = work.repo.is_none()
                || work.expected_artifact.trim().is_empty()
                || (end_observation.dirty_entries == 0
                    && end_observation
                        .source_commit
                        .as_deref()
                        .is_some_and(|commit| {
                            artifacts.iter().any(|artifact| {
                                artifact.attempt_id == Some(attempt_id)
                                    && artifact.state
                                        == restless_orgintel::ArtifactRefState::Available
                                    && artifact.source_commit.as_deref() == Some(commit)
                            })
                        }));
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
            } else if !exact_repository_artifact {
                org.finish_work_attempt(
                    attempt_id,
                    WorkAttemptState::Failed,
                    "repository Work completed without an artifact bound to its clean terminal commit",
                )
                .await?;
            } else if run_gates(org, container, work_id, attempt_id, workdir).await? {
                if let (Some(repo), Some(branch), Some(commit)) = (
                    work.repo.as_deref(),
                    work.integration_branch.as_deref(),
                    end_observation.source_commit.as_deref(),
                ) {
                    match promote_integration_commit(container, repo, branch, commit).await {
                        Ok(promoted) => {
                            org.emit_event(
                                "work_artifact_promoted",
                                Some(&work.owner_id),
                                serde_json::json!({
                                    "work_id": work_id,
                                    "attempt_id": attempt_id,
                                    "repo": repo,
                                    "branch": branch,
                                    "source_commit": commit,
                                    "workspace": promoted,
                                }),
                            )
                            .await?;
                        }
                        Err(error) => {
                            org.finish_work_attempt(
                                attempt_id,
                                WorkAttemptState::Failed,
                                &format!("exact integration promotion failed: {error:#}"),
                            )
                            .await?;
                            return Ok(());
                        }
                    }
                }
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

#[cfg(test)]
mod tests {
    use restless_orgintel::{NewWork, OrgIntel, WorkspaceSpec};

    use super::*;

    #[tokio::test]
    async fn terminal_work_fact_remains_owed_to_the_accountable_lead() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping terminal supervisor wake scenario"
            );
            return;
        };
        let company = format!("terminalwake{}", uuid::Uuid::new_v4().simple());
        let org = OrgIntel::ensure(&url, &company)
            .await
            .expect("ensure scratch company schema");
        org.ensure_actor("exec", "exec", "exec", "The Exec")
            .await
            .unwrap();
        org.create_actor(
            "opportunity-direction",
            "lead",
            "Opportunity lead",
            Some("openai-codex/gpt-5.6-sol"),
            "exec",
            "owns the outcome",
        )
        .await
        .unwrap();
        let team = org
            .create_team(
                "Opportunity",
                "Maintain one qualified opportunity",
                "opportunity-direction",
                "exec",
            )
            .await
            .unwrap();
        org.create_actor(
            "opportunity-research",
            "staff",
            "Opportunity producer",
            Some("openai-codex/gpt-5.6-sol"),
            "opportunity-direction",
            "owns production",
        )
        .await
        .unwrap();
        org.set_actor_team(
            "opportunity-research",
            Some(team),
            "opportunity-direction",
            "owns the dossier",
        )
        .await
        .unwrap();
        let work_id = org
            .add_work(NewWork {
                owner_id: "opportunity-research",
                title: "Prepare the dossier",
                outcome: "produce one grounded dossier",
                goal_id: None,
                priority: 0,
                expected_artifact: "",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            })
            .await
            .unwrap();
        let claimed = org.claim_ready_work("test runtime").await.unwrap().unwrap();
        org.finish_work_attempt(
            claimed.attempt_id,
            WorkAttemptState::Produced,
            "dossier accepted",
        )
        .await
        .unwrap();

        assert!(org
            .inbox(Some("opportunity-direction"))
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            org.flush_terminal_supervisor_notices(16)
                .await
                .unwrap()
                .len(),
            1
        );
        assert!(org
            .flush_terminal_supervisor_notices(16)
            .await
            .unwrap()
            .is_empty());

        let inbox = org.inbox(Some("opportunity-direction")).await.unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].from_actor, "daemon");
        assert!(inbox[0].body.contains("status completed, revision 1"));
        assert_eq!(
            org.message_work_id(inbox[0].id).await.unwrap(),
            Some(work_id)
        );
        org.drop_schema().await.expect("drop scratch schema");
    }
}
