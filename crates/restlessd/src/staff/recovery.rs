//! Attempt completion, terminal observations, and recovery capsules.
//!
//! These functions turn a finished Runtime process into OrgIntel's existing
//! Work/Attempt facts. They do not own agent execution or add a workflow.

use anyhow::{Context as _, Result};
use restless_orgintel::{NewAttemptRecovery, WorkAttemptState};
use restlessd::runtime_transport::{CompanyPath, RuntimeProcessAuthority, RuntimeTransport};
use sha2::Digest as _;

use crate::exec::Termination;

use super::gates::run_gates_via_transport;
use super::workspace::{
    cleanup_attempt_runtime_via_transport, observe_workspace_via_transport,
    promote_integration_commit_via_transport, WorkspaceObservation,
};

/// Close the exact Attempt that launched this process. Completion is accepted
/// only after its declared artifact and deterministic gates are observed.
pub(super) struct StaffAttemptContext<'a> {
    pub(super) runtime_transport: std::sync::Arc<dyn RuntimeTransport>,
    pub(super) runtime_authority: &'a RuntimeProcessAuthority,
    pub(super) actor: &'a str,
    pub(super) name: &'a str,
    pub(super) work_id: uuid::Uuid,
    pub(super) attempt_id: uuid::Uuid,
    pub(super) workdir: &'a str,
    pub(super) start_observation: WorkspaceObservation,
}

struct AttemptCompletionContext<'a> {
    runtime_transport: &'a std::sync::Arc<dyn RuntimeTransport>,
    runtime_authority: &'a RuntimeProcessAuthority,
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
        runtime_transport,
        runtime_authority,
        actor,
        name,
        work_id,
        attempt_id,
        workdir,
        start_observation,
    } = context;
    let end_observation = observe_workspace_via_transport(
        std::sync::Arc::clone(&runtime_transport),
        runtime_authority.clone(),
        workdir,
    )
    .await;
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
        org.bind_attempt_terminal_coordinates(
            attempt_id,
            end_observation.source_commit.as_deref(),
            end_observation.source_tree.as_deref(),
            end_observation.status_digest.as_deref(),
            end_observation.dirty_entries,
        )
        .await?;
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
                        runtime_transport: &runtime_transport,
                        runtime_authority,
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
                        runtime_transport: &runtime_transport,
                        runtime_authority,
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
            org.release_attempt_resources(attempt_id, "Attempt reached terminal state")
                .await?;
            let cleanup = cleanup_attempt_runtime_via_transport(
                runtime_transport.as_ref(),
                runtime_authority,
                workdir,
                attempt_id,
            )
            .await?;
            org.emit_event(
                "attempt_runtime_cleaned",
                Some("daemon"),
                serde_json::to_value(&cleanup)?,
            )
            .await?;
            org.flush_terminal_supervisor_notices(16).await?;
        }
        anyhow::Ok(())
    };
    if let Err(error) = record.await {
        tracing::error!(staff = name, "failed to record staff outcome: {error:#}");
    }
}

/// Reconcile journals and leases after a scheduler restart. A pending Git
/// promotion is idempotently replayed from its exact commit; no model is
/// asked to rediscover or narrate the mechanical repair.
pub(crate) async fn reconcile_execution_substrate(
    org: &restless_orgintel::OrgIntel,
    runtime_transport: &std::sync::Arc<dyn RuntimeTransport>,
    company: &str,
) -> Result<()> {
    let released = org.reconcile_runtime_resources().await?;
    if released > 0 {
        tracing::warn!(released, "released stale Runtime resource leases");
    }
    for promotion in org.pending_candidate_promotions().await? {
        let work = org
            .get_work(promotion.work_id)
            .await?
            .context("pending candidate promotion lost its Work row")?;
        let runtime_authority = RuntimeProcessAuthority::Attempt {
            company: company.to_owned(),
            actor: work.owner_id,
            responsibility: format!("work:{}", promotion.work_id),
            work_id: promotion.work_id,
            attempt_id: promotion.attempt_id,
            session_id: format!("attempt-recovery-{}", promotion.attempt_id.simple()),
        };
        match promote_integration_commit_via_transport(
            std::sync::Arc::clone(runtime_transport),
            &runtime_authority,
            &promotion.repo,
            &promotion.integration_branch,
            &promotion.source_commit,
        )
        .await
        {
            Ok(_) => {
                org.finish_candidate_promotion(promotion.id, true, None)
                    .await?;
                if org
                    .list_work_attempts(Some(promotion.work_id))
                    .await?
                    .iter()
                    .any(|attempt| {
                        attempt.id == promotion.attempt_id
                            && attempt.state == WorkAttemptState::Running
                    })
                {
                    org.finish_work_attempt(
                        promotion.attempt_id,
                        WorkAttemptState::Produced,
                        "exact candidate promotion recovered after Runtime restart",
                    )
                    .await?;
                }
            }
            Err(error) => {
                let failure = format!("restart promotion reconciliation failed: {error:#}");
                org.finish_candidate_promotion(promotion.id, false, Some(&failure))
                    .await?;
                if org
                    .list_work_attempts(Some(promotion.work_id))
                    .await?
                    .iter()
                    .any(|attempt| {
                        attempt.id == promotion.attempt_id
                            && attempt.state == WorkAttemptState::Running
                    })
                {
                    org.finish_work_attempt(
                        promotion.attempt_id,
                        WorkAttemptState::Failed,
                        &failure,
                    )
                    .await?;
                }
            }
        }
    }
    Ok(())
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
        runtime_transport,
        runtime_authority,
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
            let mut artifacts = org.list_artifact_refs(Some(work_id)).await?;
            if !work.expected_artifact.trim().is_empty()
                && work.repo.is_none()
                && work.expected_artifact.starts_with("/company/")
                && !artifacts.iter().any(|artifact| {
                    artifact.attempt_id == Some(attempt_id)
                        && artifact.state == restless_orgintel::ArtifactRefState::Available
                })
            {
                let path = CompanyPath::parse(work.expected_artifact.clone())
                    .map_err(|error| anyhow::anyhow!(error))?;
                if let Ok(digest) = runtime_transport
                    .digest(runtime_authority.company(), &path)
                    .await
                {
                    let digest = digest
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>();
                    if digest.len() == 64 {
                        let owner_label = owner_artifact_label(&work.title);
                        org.link_work_artifact(restless_orgintel::NewArtifactRef {
                            kind: "file",
                            uri: &work.expected_artifact,
                            note: "The exact file produced by this work and observed in the company runtime.",
                            created_by: &work.owner_id,
                            work_id: Some(work_id),
                            attempt_id: Some(attempt_id),
                            digest: Some(&digest),
                            source_commit: None,
                            runtime_generation: None,
                            label: &owner_label,
                        })
                        .await?;
                        artifacts = org.list_artifact_refs(Some(work_id)).await?;
                    }
                }
            }
            if !work.expected_artifact.trim().is_empty()
                && work.repo.is_some()
                && end_observation.dirty_entries == 0
                && !artifacts.iter().any(|artifact| {
                    artifact.attempt_id == Some(attempt_id)
                        && artifact.state == restless_orgintel::ArtifactRefState::Available
                })
            {
                let commit = end_observation
                    .source_commit
                    .as_deref()
                    .context("repository outcome has no exact commit")?;
                let tree = end_observation
                    .source_tree
                    .as_deref()
                    .context("repository outcome has no exact tree")?;
                let uri = format!(
                    "git:/company/repos/{}#{commit}",
                    work.repo.as_deref().unwrap_or_default()
                );
                let owner_label = owner_artifact_label(&work.title);
                org.link_work_artifact(restless_orgintel::NewArtifactRef {
                    kind: "repository_tree",
                    uri: &uri,
                    note: "The saved result produced by this work; Restless observed it with no uncommitted changes.",
                    created_by: &work.owner_id,
                    work_id: Some(work_id),
                    attempt_id: Some(attempt_id),
                    digest: Some(tree),
                    source_commit: Some(commit),
                    runtime_generation: None,
                    label: &owner_label,
                })
                .await?;
                artifacts = org.list_artifact_refs(Some(work_id)).await?;
            }
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
            } else {
                let candidate_identity = end_observation
                    .source_tree
                    .clone()
                    .or_else(|| {
                        let mut digests = artifacts
                            .iter()
                            .filter(|artifact| artifact.attempt_id == Some(attempt_id))
                            .filter_map(|artifact| artifact.digest.clone())
                            .collect::<Vec<_>>();
                        digests.sort();
                        (!digests.is_empty()).then(|| {
                            format!("{:x}", sha2::Sha256::digest(digests.join("\n").as_bytes()))
                        })
                    })
                    .unwrap_or_else(|| format!("attempt:{attempt_id}"));
                let gates_passed = match run_gates_via_transport(
                    org,
                    runtime_transport.as_ref(),
                    runtime_authority,
                    work_id,
                    attempt_id,
                    workdir,
                    &candidate_identity,
                )
                .await
                {
                    Ok(passed) => passed,
                    Err(error) => {
                        org.finish_work_attempt(
                            attempt_id,
                            WorkAttemptState::Failed,
                            &format!("governed gate infrastructure failed: {error:#}"),
                        )
                        .await?;
                        return Ok(());
                    }
                };
                if !gates_passed {
                    org.finish_work_attempt(
                        attempt_id,
                        WorkAttemptState::Failed,
                        "one or more deterministic Work gates failed",
                    )
                    .await?;
                    return Ok(());
                }
                if let (Some(repo), Some(branch), Some(commit)) = (
                    work.repo.as_deref(),
                    work.integration_branch.as_deref(),
                    end_observation.source_commit.as_deref(),
                ) {
                    let tree = end_observation
                        .source_tree
                        .as_deref()
                        .context("promotable candidate has no exact tree")?;
                    let manifest = serde_json::json!({
                        "work_id": work_id,
                        "attempt_id": attempt_id,
                        "source_commit": commit,
                        "source_tree": tree,
                        "artifacts": artifacts.iter().filter(|artifact| {
                            artifact.attempt_id == Some(attempt_id)
                                && artifact.state == restless_orgintel::ArtifactRefState::Available
                        }).map(|artifact| serde_json::json!({
                            "id": artifact.id,
                            "kind": artifact.kind,
                            "uri": artifact.uri,
                            "digest": artifact.digest,
                        })).collect::<Vec<_>>(),
                    });
                    let promotion = org
                        .begin_candidate_promotion(restless_orgintel::NewCandidatePromotion {
                            work_id,
                            attempt_id,
                            repo,
                            integration_branch: branch,
                            source_commit: commit,
                            source_tree: tree,
                            manifest: &manifest,
                        })
                        .await?;
                    match promote_integration_commit_via_transport(
                        std::sync::Arc::clone(runtime_transport),
                        runtime_authority,
                        repo,
                        branch,
                        commit,
                    )
                    .await
                    {
                        Ok(promoted) => {
                            org.finish_candidate_promotion(promotion.id, true, None)
                                .await?;
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
                            org.finish_candidate_promotion(
                                promotion.id,
                                false,
                                Some(&format!("{error:#}")),
                            )
                            .await?;
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

pub(super) fn gate_cwd<'a>(declared: &'a str, attempt_workdir: &'a str) -> &'a str {
    if declared == "@attempt" {
        attempt_workdir
    } else {
        declared
    }
}

fn owner_artifact_label(work_title: &str) -> String {
    format!("Output from: {work_title}")
}

#[cfg(test)]
mod tests {
    use restless_orgintel::{NewWork, OrgIntel, WorkspaceSpec};

    use super::*;

    #[test]
    fn automatic_artifact_labels_name_the_work_not_its_execution_contract() {
        let label = owner_artifact_label("Prepare the customer interview report");
        assert_eq!(label, "Output from: Prepare the customer interview report");
        assert!(!label.contains("commit"));
        assert!(!label.contains("gate"));
    }

    #[tokio::test]
    async fn material_terminal_work_fact_remains_owed_to_the_accountable_lead() {
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
        for message in org.inbox(Some("opportunity-direction")).await.unwrap() {
            org.mark_read(message.id).await.unwrap();
        }
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
            WorkAttemptState::Blocked,
            "source contradiction needs accountable judgement",
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
        assert!(inbox[0]
            .body
            .contains("Attempt blocked, Work blocked, revision 1"));
        assert_eq!(
            org.message_work_id(inbox[0].id).await.unwrap(),
            Some(work_id)
        );
        org.drop_schema().await.expect("drop scratch schema");
    }
}
