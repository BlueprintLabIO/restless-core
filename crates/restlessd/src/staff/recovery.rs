//! Attempt completion, terminal observations, and recovery capsules.
//!
//! These functions turn a finished Runtime process into OrgIntel's existing
//! Work/Attempt facts. They do not own agent execution or add a workflow.

use anyhow::{bail, Context as _, Result};
use restless_orgintel::{
    ArtifactRefState, NewArtifactRef, NewAttemptRecovery, NewCandidatePromotion,
    NewImmutableReviewTarget, WorkAttemptState, WorkRow,
};
use restless_runtime_bridge_protocol::{ArtifactSelector, CandidateIdentity, RuntimeIdentity};
use sha2::Digest as _;

use crate::exec::Termination;

use super::gates::{run_gates_with_actor_session, run_hosted_gates};
use super::workspace::{
    cleanup_attempt_runtime, hosted_candidate_identity, hosted_workspace_identity,
    observe_hosted_workspace, observe_workspace, promote_integration_commit, WorkspaceObservation,
};

/// Close the exact Attempt that launched this process. Completion is accepted
/// only after its declared artifact and deterministic gates are observed.
pub(super) struct StaffAttemptContext<'a> {
    pub(super) container: &'a str,
    pub(super) company: &'a str,
    pub(super) actor: &'a str,
    pub(super) name: &'a str,
    pub(super) capabilities: &'a crate::capability::CapabilityIssuer,
    pub(super) work_id: uuid::Uuid,
    pub(super) attempt_id: uuid::Uuid,
    pub(super) workdir: &'a str,
    pub(super) start_observation: WorkspaceObservation,
}

struct AttemptCompletionContext<'a> {
    container: &'a str,
    company: &'a str,
    capabilities: &'a crate::capability::CapabilityIssuer,
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
        company,
        actor,
        name,
        capabilities,
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
                        container,
                        company,
                        capabilities,
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
                        company,
                        capabilities,
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
            let cleanup = cleanup_attempt_runtime(container, workdir, attempt_id).await?;
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

/// Settle a hosted coordination or repository Work slice without touching the
/// account plane's Docker daemon. Repository Work is accepted only from a
/// clean, content-free terminal Git observation returned by the exact Runtime.
/// Artifacts, gates, promotion and cleanup remain the next typed boundary.
#[expect(
    clippy::too_many_arguments,
    reason = "hosted settlement keeps Runtime, Work and frozen workspace identity explicit"
)]
pub(super) async fn record_hosted_staff_outcome(
    org: &restless_orgintel::OrgIntel,
    actor: &str,
    name: &str,
    work_id: uuid::Uuid,
    attempt_id: uuid::Uuid,
    workdir: &str,
    start_observation: WorkspaceObservation,
    expected_environment_fingerprint: Option<&str>,
    runtime_bridges: &crate::runtime_bridge::RuntimeBridgeRegistry,
    runtime_identity: Option<&restless_runtime_bridge_protocol::RuntimeIdentity>,
    outcome: Result<(Termination, String)>,
) {
    let record = async {
        let work = org
            .get_work(work_id)
            .await?
            .context("hosted claimed Work disappeared before settlement")?;
        let end_observation = if work.repo.is_some() {
            let identity = runtime_identity
                .context("repository Work lost its exact hosted Runtime identity")?;
            let environment = expected_environment_fingerprint
                .context("repository Work lost its frozen Runtime environment")?;
            match observe_hosted_workspace(
                runtime_bridges,
                identity,
                &work,
                attempt_id,
                environment,
            )
            .await
            {
                Ok(observation) => observation,
                Err(error) => {
                    let reason = format!(
                        "hosted Runtime could not provide terminal repository evidence: {error:#}"
                    );
                    record_unknown_recovery(
                        org,
                        attempt_id,
                        &reason,
                        &start_observation,
                        &start_observation,
                    )
                    .await?;
                    org.release_attempt_resources(
                        attempt_id,
                        "hosted terminal observation became recoverable",
                    )
                    .await?;
                    return anyhow::Ok(());
                }
            }
        } else {
            WorkspaceObservation {
                workdir: workdir.to_string(),
                ..WorkspaceObservation::default()
            }
        };
        org.bind_attempt_terminal_coordinates(
            attempt_id,
            end_observation.source_commit.as_deref(),
            end_observation.source_tree.as_deref(),
            end_observation.status_digest.as_deref(),
            end_observation.dirty_entries,
        )
        .await?;
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
        let terminal = match outcome {
            Ok((Termination::OutcomeMet, summary)) => {
                if work.repo.is_some() {
                    let identity = runtime_identity
                        .context("repository Work lost its exact hosted Runtime identity")?;
                    let environment = expected_environment_fingerprint
                        .context("repository Work lost its frozen Runtime environment")?;
                    match settle_hosted_repository_outcome(
                        org,
                        runtime_bridges,
                        identity,
                        &work,
                        attempt_id,
                        &end_observation,
                        environment,
                        &summary,
                    )
                    .await
                    {
                        Ok(accepted_summary) => {
                            let effective = org
                                .finish_work_attempt(
                                    attempt_id,
                                    WorkAttemptState::Produced,
                                    &accepted_summary,
                                )
                                .await?;
                            if work.owner_review_required && effective != WorkAttemptState::Produced
                            {
                                retire_hosted_review_targets(
                                    org,
                                    &work,
                                    attempt_id,
                                    "withheld because final Attempt settlement did not produce",
                                )
                                .await?;
                            }
                        }
                        Err(error) => {
                            if work.owner_review_required {
                                retire_hosted_review_targets(
                                    org,
                                    &work,
                                    attempt_id,
                                    "withheld because hosted repository acceptance failed",
                                )
                                .await?;
                            }
                            let failure =
                                bounded_failure("hosted repository acceptance failed", &error);
                            org.finish_work_attempt(attempt_id, WorkAttemptState::Failed, &failure)
                                .await?;
                        }
                    }
                } else {
                    org.finish_work_attempt(attempt_id, WorkAttemptState::Produced, &summary)
                        .await?;
                }
                true
            }
            Ok((Termination::ChangesRequested, summary)) => {
                org.finish_work_attempt(attempt_id, WorkAttemptState::ChangesRequested, &summary)
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
                record_unknown_recovery(
                    org,
                    attempt_id,
                    &format!("hosted cognitive session requested continuation: {summary}"),
                    &start_observation,
                    &end_observation,
                )
                .await?;
                false
            }
            Err(error) => {
                let reason = format!(
                    "hosted cognitive process ended before a trustworthy semantic result: {error:#}"
                );
                org.emit_event(
                    "staff_crash",
                    Some(actor),
                    serde_json::json!({ "error": format!("{error:#}"), "transport": "hosted-runtime-bridge" }),
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
        if terminal {
            org.release_attempt_resources(attempt_id, "hosted Attempt reached terminal state")
                .await?;
            if work.repo.is_some() {
                let identity = runtime_identity
                    .context("repository Work lost its exact hosted Runtime identity")?;
                let environment = expected_environment_fingerprint
                    .context("repository Work lost its frozen Runtime environment")?;
                let workspace = hosted_workspace_identity(&work, attempt_id)?;
                match crate::runtime_bridge::cleanup_attempt(
                    runtime_bridges,
                    identity,
                    workspace,
                    environment.to_string(),
                )
                .await
                {
                    Ok(receipt) => {
                        org.emit_event(
                            "attempt_runtime_cleaned",
                            Some("daemon"),
                            serde_json::to_value(receipt)?,
                        )
                        .await?;
                    }
                    Err(error) => {
                        org.emit_event(
                            "attempt_runtime_cleanup_failed",
                            Some("daemon"),
                            serde_json::json!({
                                "work_id": work_id,
                                "attempt_id": attempt_id,
                                "error": bounded_failure("typed hosted cleanup failed", &error),
                            }),
                        )
                        .await?;
                    }
                }
            }
            org.flush_terminal_supervisor_notices(16).await?;
        }
        anyhow::Ok(())
    };
    if let Err(error) = record.await {
        tracing::error!(staff = name, %attempt_id, "failed to record hosted Staff outcome: {error:#}");
    }
}

async fn settle_hosted_repository_outcome(
    org: &restless_orgintel::OrgIntel,
    runtime_bridges: &crate::runtime_bridge::RuntimeBridgeRegistry,
    runtime_identity: &RuntimeIdentity,
    work: &WorkRow,
    attempt_id: uuid::Uuid,
    end_observation: &WorkspaceObservation,
    environment_fingerprint: &str,
    summary: &str,
) -> Result<String> {
    let workspace = hosted_workspace_identity(work, attempt_id)?;
    let candidate = hosted_candidate_identity(end_observation, environment_fingerprint)?;
    require_current_hosted_candidate(org, work, attempt_id, &candidate).await?;

    if !run_hosted_gates(
        org,
        runtime_bridges,
        runtime_identity,
        work,
        attempt_id,
        &candidate,
    )
    .await?
    {
        bail!("one or more deterministic Work gates failed");
    }
    require_current_hosted_candidate(org, work, attempt_id, &candidate).await?;
    if work.owner_review_required {
        let has_live_probe = org
            .list_work_gates(work.id)
            .await?
            .iter()
            .any(|gate| gate.name == restless_orgintel::REVIEW_TARGET_LIVE_PROBE_GATE);
        if !has_live_probe {
            bail!("owner-review Work has no frozen ReviewTarget live-probe gate");
        }
        // An agent may have linked a mutable target before its deterministic
        // probe ran. Once the probe has passed, withdraw every such reference
        // before review-copy preparation or promotion. Only the exact,
        // commit-addressed Runtime copy is exposed after every later boundary
        // succeeds, so a promotion/refusal can never leave a current target.
        retire_hosted_review_targets(
            org,
            work,
            attempt_id,
            "withheld until exact hosted Runtime review preparation and promotion settle",
        )
        .await?;
    }

    let artifact_receipt = crate::runtime_bridge::observe_artifact(
        runtime_bridges,
        runtime_identity,
        workspace.clone(),
        candidate.clone(),
        ArtifactSelector::RepositoryTree,
    )
    .await?;
    require_current_hosted_candidate(org, work, attempt_id, &candidate).await?;

    let needs_review_copy = work.owner_review_required
        || work.integration_branch.is_some()
        || !work.expected_artifact.trim().is_empty();
    let review_receipt = if needs_review_copy {
        let receipt = crate::runtime_bridge::prepare_review_copy(
            runtime_bridges,
            runtime_identity,
            workspace.clone(),
            candidate.clone(),
        )
        .await?;
        require_current_hosted_candidate(org, work, attempt_id, &candidate).await?;
        Some(receipt)
    } else {
        None
    };

    // Promotion is the only externally visible mutable repository action.
    // Prepare and verify all candidate/review bytes first, but publish no
    // ReviewTarget reference until promotion has conclusively succeeded.
    if let Some(branch) = work.integration_branch.as_deref() {
        require_current_hosted_candidate(org, work, attempt_id, &candidate).await?;
        let manifest = serde_json::json!({
            "contract": "hosted-runtime-promotion-v1",
            "runtime": runtime_identity,
            "workspace": workspace,
            "candidate": candidate,
            "artifact_receipt": artifact_receipt,
            "review_receipt": review_receipt,
        });
        let promotion = org
            .begin_candidate_promotion(NewCandidatePromotion {
                work_id: work.id,
                attempt_id,
                repo: work.repo.as_deref().context("repository Work lost repo")?,
                integration_branch: branch,
                source_commit: &candidate.source_commit,
                source_tree: &candidate.source_tree,
                manifest: &manifest,
            })
            .await?;
        match crate::runtime_bridge::promote_candidate(
            runtime_bridges,
            runtime_identity,
            workspace.clone(),
            candidate.clone(),
            branch.to_string(),
        )
        .await
        {
            Ok(receipt) => {
                require_current_hosted_candidate(org, work, attempt_id, &candidate).await?;
                org.finish_candidate_promotion(promotion.id, true, None)
                    .await?;
                org.emit_event(
                    "work_artifact_promoted",
                    Some(&work.owner_id),
                    serde_json::json!({
                        "work_id": work.id,
                        "attempt_id": attempt_id,
                        "receipt": receipt,
                    }),
                )
                .await?;
            }
            Err(error) => {
                let failure = bounded_failure("exact hosted integration promotion failed", &error);
                org.finish_candidate_promotion(promotion.id, false, Some(&failure))
                    .await?;
                return Err(error.context("exact hosted integration promotion failed"));
            }
        }
    }

    require_current_hosted_candidate(org, work, attempt_id, &candidate).await?;
    org.bind_attempt_artifacts_to_observed_commit(attempt_id, &candidate.source_commit)
        .await?;
    let runtime_generation = runtime_identity.runtime_generation.to_string();
    let owner_label = owner_artifact_label(&work.title);
    let artifact_note = serde_json::to_string(&serde_json::json!({
        "contract": "hosted-runtime-artifact-receipt-v1",
        "receipt": artifact_receipt,
        "runtime": runtime_identity,
        "workspace": workspace,
    }))?;
    org.link_work_artifact(NewArtifactRef {
        kind: "repository_tree",
        uri: &artifact_receipt.uri,
        note: &artifact_note,
        created_by: &work.owner_id,
        work_id: Some(work.id),
        attempt_id: Some(attempt_id),
        // Preserve the existing repository-tree contract: this digest is the
        // exact Git tree object. The bounded manifest digest remains in the
        // Runtime receipt above.
        digest: Some(&candidate.source_tree),
        source_commit: Some(&candidate.source_commit),
        runtime_generation: Some(&runtime_generation),
        label: &owner_label,
    })
    .await?;

    if let Some(review) = review_receipt {
        let manifest = serde_json::json!({
            "contract": "hosted-runtime-review-copy-v1",
            "runtime": runtime_identity,
            "workspace": workspace,
            "receipt": review,
        });
        org.record_immutable_review_target(NewImmutableReviewTarget {
            work_id: work.id,
            attempt_id,
            content_digest: &review.content_digest,
            uri: &review.uri,
            alias_uri: Some(&review.alias_uri),
            source_commit: Some(&candidate.source_commit),
            manifest: &manifest,
        })
        .await?;
        let review_note = serde_json::to_string(&manifest)?;
        let review_label = format!("Review copy: {}", work.title);
        org.link_work_artifact(NewArtifactRef {
            kind: "review_copy",
            uri: &review.uri,
            note: &review_note,
            created_by: &work.owner_id,
            work_id: Some(work.id),
            attempt_id: Some(attempt_id),
            digest: Some(&review.content_digest),
            source_commit: Some(&candidate.source_commit),
            runtime_generation: Some(&runtime_generation),
            label: &review_label,
        })
        .await?;
        let publication_label = format!("Publishable source: {}", work.title);
        org.link_work_artifact(NewArtifactRef {
            kind: "published_service_source",
            uri: &artifact_receipt.uri,
            note: &artifact_note,
            created_by: &work.owner_id,
            work_id: Some(work.id),
            attempt_id: Some(attempt_id),
            digest: Some(&artifact_receipt.content_digest),
            source_commit: Some(&candidate.source_commit),
            runtime_generation: Some(&runtime_generation),
            label: &publication_label,
        })
        .await?;

        if work.owner_review_required {
            let target_label = format!("Review target: {}", work.title);
            org.link_work_artifact(NewArtifactRef {
                kind: restless_orgintel::REVIEW_TARGET_ARTIFACT_KIND,
                uri: &review.uri,
                note: &review_note,
                created_by: &work.owner_id,
                work_id: Some(work.id),
                attempt_id: Some(attempt_id),
                digest: Some(&review.content_digest),
                source_commit: Some(&candidate.source_commit),
                runtime_generation: Some(&runtime_generation),
                label: &target_label,
            })
            .await?;
        }
    }

    require_current_hosted_candidate(org, work, attempt_id, &candidate).await?;
    hosted_repository_completion(summary, end_observation).map_err(anyhow::Error::msg)
}

async fn retire_hosted_review_targets(
    org: &restless_orgintel::OrgIntel,
    work: &WorkRow,
    attempt_id: uuid::Uuid,
    reason: &str,
) -> Result<()> {
    for artifact in org.list_artifact_refs(Some(work.id)).await? {
        if artifact.attempt_id == Some(attempt_id)
            && artifact.kind == restless_orgintel::REVIEW_TARGET_ARTIFACT_KIND
            && artifact.state == ArtifactRefState::Available
        {
            org.retire_work_artifact(artifact.id, &work.owner_id, reason)
                .await?;
        }
    }
    Ok(())
}

async fn require_current_hosted_candidate(
    org: &restless_orgintel::OrgIntel,
    frozen_work: &WorkRow,
    attempt_id: uuid::Uuid,
    candidate: &CandidateIdentity,
) -> Result<()> {
    let work = org
        .get_work(frozen_work.id)
        .await?
        .context("hosted Work disappeared during candidate acceptance")?;
    if work.revision != frozen_work.revision
        || work.repo != frozen_work.repo
        || work.integration_branch != frozen_work.integration_branch
        || work.owner_id != frozen_work.owner_id
    {
        bail!("hosted Work changed after its candidate was frozen");
    }
    let attempt = org
        .list_work_attempts(Some(work.id))
        .await?
        .into_iter()
        .find(|attempt| attempt.id == attempt_id)
        .context("hosted Attempt disappeared during candidate acceptance")?;
    if attempt.work_id != work.id
        || attempt.revision != work.revision
        || attempt.actor_id != work.owner_id
        || attempt.state != WorkAttemptState::Running
        || attempt.terminal_source_commit.as_deref() != Some(candidate.source_commit.as_str())
        || attempt.terminal_source_tree.as_deref() != Some(candidate.source_tree.as_str())
        || attempt.terminal_status_digest.as_deref() != Some(candidate.status_digest.as_str())
        || attempt.terminal_dirty_entries != Some(0)
        || attempt.environment_fingerprint != candidate.environment_fingerprint
    {
        bail!("hosted candidate is stale or differs from exact running Attempt evidence");
    }
    Ok(())
}

fn bounded_failure(context: &str, error: &anyhow::Error) -> String {
    let mut message = format!("{context}: {error:#}");
    if message.len() > 4_000 {
        let mut boundary = 4_000;
        while !message.is_char_boundary(boundary) {
            boundary -= 1;
        }
        message.truncate(boundary);
    }
    message
}

fn hosted_repository_completion(
    summary: &str,
    observation: &WorkspaceObservation,
) -> std::result::Result<String, String> {
    let commit = observation
        .source_commit
        .as_deref()
        .ok_or_else(|| "reported complete without an exact terminal Git commit".to_string())?;
    let tree = observation
        .source_tree
        .as_deref()
        .ok_or_else(|| "reported complete without an exact terminal Git tree".to_string())?;
    let status = observation
        .status_digest
        .as_deref()
        .ok_or_else(|| "reported complete without a terminal Git status digest".to_string())?;
    if observation.dirty_entries != 0 {
        return Err(format!(
            "reported complete with {} uncommitted entries at exact HEAD {commit}; the Runtime preserved the workspace",
            observation.dirty_entries,
        ));
    }
    Ok(format!(
        "{summary}\n\nRuntime evidence: clean commit {commit}, tree {tree}, status {status}."
    ))
}

/// Reconcile journals and leases after a scheduler restart. A pending Git
/// promotion is idempotently replayed from its exact commit; no model is
/// asked to rediscover or narrate the mechanical repair.
pub(crate) async fn reconcile_execution_substrate(
    org: &restless_orgintel::OrgIntel,
    container: &str,
) -> Result<()> {
    let released = org.reconcile_runtime_resources().await?;
    if released > 0 {
        tracing::warn!(released, "released stale Runtime resource leases");
    }
    for promotion in org.pending_candidate_promotions().await? {
        match promote_integration_commit(
            container,
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
        container,
        company,
        capabilities,
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
                let digest = tokio::process::Command::new("docker")
                    .args([
                        "exec",
                        "-u",
                        "company",
                        container,
                        "sha256sum",
                        "--",
                        &work.expected_artifact,
                    ])
                    .output()
                    .await
                    .context("observe declared file artifact")?;
                if digest.status.success() {
                    let digest = String::from_utf8_lossy(&digest.stdout)
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_string();
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
                let gates_passed = match run_gates_with_actor_session(
                    org,
                    container,
                    company,
                    capabilities,
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
                    match promote_integration_commit(container, repo, branch, commit).await {
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

    #[test]
    fn hosted_repository_completion_is_clean_and_names_exact_git_evidence() {
        let clean = WorkspaceObservation {
            workdir: "/company/worktrees/work-123-r1".into(),
            source_commit: Some("a".repeat(40)),
            source_tree: Some("b".repeat(40)),
            status_digest: Some("c".repeat(64)),
            dirty_entries: 0,
        };
        let summary = hosted_repository_completion("game implemented", &clean).unwrap();
        assert!(summary.contains(&format!("clean commit {}", "a".repeat(40))));
        assert!(summary.contains(&format!("tree {}", "b".repeat(40))));
        assert!(summary.contains(&format!("status {}", "c".repeat(64))));

        let dirty = WorkspaceObservation {
            dirty_entries: 2,
            ..clean
        };
        let refusal = hosted_repository_completion("claimed complete", &dirty).unwrap_err();
        assert!(refusal.contains("2 uncommitted entries"));
        assert!(refusal.contains(&"a".repeat(40)));
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
        let _work_id = org
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
            None,
            "a supervisor notice remains attributable coordination, not duplicate producer input"
        );
        org.drop_schema().await.expect("drop scratch schema");
    }
}
