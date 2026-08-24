//! Atomic Work claims, Attempt lifecycle, and bounded recovery capsules.
//!
//! This is a move-only ownership split; Git and Runtime remain the source of files.

use super::*;

struct QualifiedOutcomeReview {
    target_uri: String,
    title: String,
    outcome: String,
}

impl OrgIntel {
    /// Atomically lease the highest-priority ready Work node. Readiness is a
    /// database fact: all hard requirements completed, no pending owner
    /// handoff, no live Attempt, and any explicit attempt limit still has room.
    pub async fn claim_ready_work(&self, trigger: &str) -> Result<Option<ClaimedWork>> {
        self.claim_ready_work_excluding(trigger, &[]).await
    }

    /// The scheduler's claim path. A supervised conversation already owns its
    /// actor outside the Work graph, so exclude those actor ids before taking a
    /// database lease. Skipping is not failure: no Attempt row is created and
    /// the Work remains ready for the next scan.
    pub async fn claim_ready_work_excluding(
        &self,
        trigger: &str,
        excluded_actor_ids: &[String],
    ) -> Result<Option<ClaimedWork>> {
        let mut tx = self.pool.begin().await?;
        let work = sqlx::query_as::<_, WorkRow>(
            "SELECT w.id, w.goal_id, w.owner_id, w.title, w.outcome, w.status, \
                    w.resolution, w.priority, w.expected_artifact, w.owner_review_required, \
                    w.repo, w.base_ref, \
                    w.integration_branch, w.worktree, w.revision, w.attempt_limit, \
                    w.created_at, w.updated_at \
             FROM work w \
             WHERE w.status IN ('proposed','active') \
               AND w.owner_id <> ALL($1::text[]) \
               AND NOT EXISTS (SELECT 1 FROM work_attempts a \
                               WHERE a.work_id = w.id AND a.state = 'running') \
               AND NOT EXISTS (SELECT 1 FROM work_attempts a \
                               WHERE a.actor_id = w.owner_id AND a.state = 'running') \
               AND NOT EXISTS (SELECT 1 FROM owner_handoffs h \
                               WHERE h.work_id = w.id AND h.state = 'pending') \
               AND NOT EXISTS (\
                 SELECT 1 FROM work_edges e JOIN work upstream ON upstream.id = e.from_work_id \
                 WHERE e.to_work_id = w.id AND e.kind = 'requires' \
                   AND upstream.status <> 'completed'\
               ) \
               AND (w.attempt_limit IS NULL OR (\
                    SELECT count(*) FROM work_attempts a \
                    WHERE a.work_id = w.id AND a.revision = w.revision\
               ) < w.attempt_limit) \
             ORDER BY w.priority DESC, w.created_at \
             FOR UPDATE OF w SKIP LOCKED LIMIT 1",
        )
        .bind(excluded_actor_ids)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(work) = work else {
            tx.commit().await?;
            return Ok(None);
        };

        let inputs = sqlx::query_as::<_, ArtifactRefRow>(
            "SELECT a.id, a.kind, a.uri, a.note, a.created_by, a.work_id, a.attempt_id, \
                    a.digest, a.source_commit, a.runtime_generation, a.label, a.state, \
                    a.created_at, a.superseded_at \
             FROM work_edges e JOIN artifact_refs a ON a.work_id = e.from_work_id \
             WHERE e.to_work_id = $1 AND e.kind = 'requires' AND a.state = 'available' \
             ORDER BY e.from_work_id, a.created_at, a.id",
        )
        .bind(work.id)
        .fetch_all(&mut *tx)
        .await?;
        let mut fingerprint_source = inputs
            .iter()
            .map(|artifact| {
                format!(
                    "{}:{}:{}",
                    artifact.id,
                    artifact.digest.as_deref().unwrap_or("unknown"),
                    artifact.source_commit.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let feedback = sqlx::query_as::<_, MessageRow>(
            "SELECT id, from_actor, to_actor, body, created_at, read_at FROM (\
               SELECT m.id, m.from_actor, m.to_actor, m.body, m.created_at, m.read_at \
               FROM work_feedback f JOIN messages m ON m.id=f.message_id \
               WHERE f.work_id=$1 ORDER BY m.id DESC LIMIT 100\
             ) recent ORDER BY id",
        )
        .bind(work.id)
        .fetch_all(&mut *tx)
        .await?;
        let feedback_cursor = feedback.last().map(|message| message.id).unwrap_or(0);
        for message in &feedback {
            fingerprint_source.push_str(&format!(
                "\nfeedback:{}:{:x}",
                message.id,
                Sha256::digest(message.body.as_bytes())
            ));
        }
        let input_fingerprint = format!("{:x}", Sha256::digest(fingerprint_source.as_bytes()));
        let attempt_no: i64 = sqlx::query_scalar(
            "SELECT count(*) + 1 FROM work_attempts WHERE work_id = $1 AND revision = $2",
        )
        .bind(work.id)
        .bind(work.revision)
        .fetch_one(&mut *tx)
        .await?;
        let attempt_id = Uuid::new_v4();
        let session_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO work_attempts \
             (id, work_id, revision, attempt_no, actor_id, session_id, trigger, input_fingerprint, feedback_cursor) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(attempt_id)
        .bind(work.id)
        .bind(work.revision)
        .bind(i32::try_from(attempt_no).unwrap_or(i32::MAX))
        .bind(&work.owner_id)
        .bind(&session_id)
        .bind(trigger)
        .bind(&input_fingerprint)
        .bind(feedback_cursor)
        .execute(&mut *tx)
        .await?;
        for artifact in &inputs {
            sqlx::query(
                "INSERT INTO work_attempt_inputs (attempt_id, artifact_ref_id) VALUES ($1,$2)",
            )
            .bind(attempt_id)
            .bind(artifact.id)
            .execute(&mut *tx)
            .await?;
        }
        for message in &feedback {
            sqlx::query(
                "INSERT INTO work_attempt_feedback (attempt_id, message_id) VALUES ($1,$2)",
            )
            .bind(attempt_id)
            .bind(message.id)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("UPDATE work SET status = 'active', resolution = '' WHERE id = $1")
            .bind(work.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Some(ClaimedWork {
            work,
            attempt_id,
            attempt_no: i32::try_from(attempt_no).unwrap_or(i32::MAX),
            session_id,
            input_fingerprint,
            inputs,
            feedback,
        }))
    }

    pub async fn set_attempt_model(&self, attempt_id: Uuid, model: &str) -> Result<()> {
        sqlx::query("UPDATE work_attempts SET model = $2 WHERE id = $1 AND state = 'running'")
            .bind(attempt_id)
            .bind(model)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_work_attempts(&self, work_id: Option<Uuid>) -> Result<Vec<WorkAttemptRow>> {
        Ok(sqlx::query_as(
            "SELECT id, work_id, revision, attempt_no, actor_id, session_id, state, trigger, \
                    input_fingerprint, feedback_cursor, model, started_at, finished_at, summary \
             FROM work_attempts WHERE ($1::uuid IS NULL OR work_id = $1) \
             ORDER BY started_at, id",
        )
        .bind(work_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Close an execution mechanism whose semantic completion is missing and
    /// preserve one bounded recovery capsule for the accountable lead. The
    /// Runtime still owns the worktree and Git facts; OrgIntel owns only the
    /// Attempt, message, and references that make the next judgement possible.
    ///
    /// This is intentionally one transaction. A daemon can replay
    /// reconciliation after a crash, but it must not manufacture a second
    /// recovery message, observation reference, or Attempt.
    pub async fn record_unknown_attempt_recovery(
        &self,
        attempt_id: Uuid,
        recovery: NewAttemptRecovery<'_>,
    ) -> Result<Option<AttemptRecoveryNotice>> {
        if recovery.observed_by.trim().is_empty()
            || recovery.reason.trim().is_empty()
            || recovery.workspace.trim().is_empty()
        {
            return Err(OrgIntelError::InvalidWork(
                "an unknown Attempt recovery needs observer, reason, and workspace".into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT a.work_id, a.actor_id, a.state, a.recovery_message_id, \
                    COALESCE(t.lead_actor_id, 'exec') AS coordinator_id \
             FROM work_attempts a \
             JOIN work w ON w.id=a.work_id \
             JOIN actors actor ON actor.id=a.actor_id \
             LEFT JOIN teams t ON t.id=actor.team_id AND t.disbanded_at IS NULL \
             WHERE a.id=$1 FOR UPDATE OF a, w",
        )
        .bind(attempt_id)
        .fetch_one(&mut *tx)
        .await?;
        let work_id: Uuid = row.get("work_id");
        let actor_id: String = row.get("actor_id");
        let state: WorkAttemptState = row.get("state");
        let existing_notice: Option<i64> = row.get("recovery_message_id");
        let coordinator_id: String = row.get("coordinator_id");
        if existing_notice.is_some() {
            tx.commit().await?;
            return Ok(None);
        }
        if state != WorkAttemptState::Running {
            return Err(OrgIntelError::InvalidWork(format!(
                "Attempt {attempt_id} is {state:?}, not an unknown execution awaiting recovery"
            )));
        }

        let reason = recovery.reason.trim().chars().take(320).collect::<String>();
        let summary = format!(
            "cognitive process ended without trustworthy semantic completion; productive outcome unknown: {reason}"
        );
        sqlx::query(
            "UPDATE work_attempts \
             SET state='failed', summary=$2, finished_at=now() WHERE id=$1",
        )
        .bind(attempt_id)
        .bind(&summary)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE work SET status='blocked', resolution=$2 WHERE id=$1")
            .bind(work_id)
            .bind(&summary)
            .execute(&mut *tx)
            .await?;

        if recovery.changed_since_start {
            let existing: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM artifact_refs \
                 WHERE attempt_id=$1 AND kind='git_worktree_observation' AND uri=$2 \
                 ORDER BY created_at, id LIMIT 1",
            )
            .bind(attempt_id)
            .bind(recovery.workspace)
            .fetch_optional(&mut *tx)
            .await?;
            if existing.is_none() {
                sqlx::query(
                    "INSERT INTO artifact_refs \
                     (id, kind, uri, note, created_by, work_id, attempt_id, digest, source_commit, \
                      runtime_generation, label, state) \
                     VALUES ($1,'git_worktree_observation',$2,$3,$4,$5,$6,$7,$8,NULL, \
                             'Runtime-observed preserved worktree','available')",
                )
                .bind(Uuid::new_v4())
                .bind(recovery.workspace)
                .bind("Runtime observed changed Git state after the cognitive process ended without a trustworthy semantic result. This locator is recovery evidence, not outcome acceptance.")
                // The daemon observed this locator. Attribute it to the
                // Runtime observer, not to the Staff actor whose unfinished
                // contribution the lead still has to judge.
                .bind(recovery.observed_by)
                .bind(work_id)
                .bind(attempt_id)
                .bind(recovery.observation_digest)
                .bind(recovery.end_commit)
                .execute(&mut *tx)
                .await?;
            }
        }

        let artifacts = sqlx::query_as::<_, ArtifactRefRow>(
            "SELECT id, kind, uri, note, created_by, work_id, attempt_id, digest, source_commit, \
                    runtime_generation, label, state, created_at, superseded_at \
             FROM artifact_refs \
             WHERE attempt_id=$1 AND state='available' \
             ORDER BY created_at, id LIMIT 32",
        )
        .bind(attempt_id)
        .fetch_all(&mut *tx)
        .await?;
        let artifact_lines = if artifacts.is_empty() {
            "- no linked artifact; inspect the preserved workspace directly".to_string()
        } else {
            artifacts
                .iter()
                .map(|artifact| {
                    let commit =
                        artifact
                            .source_commit
                            .as_deref()
                            .map_or_else(String::new, |value| {
                                format!(" at commit {}", value.chars().take(12).collect::<String>())
                            });
                    let digest = artifact
                        .digest
                        .as_deref()
                        .map_or_else(String::new, |value| {
                            format!(" digest {}", value.chars().take(12).collect::<String>())
                        });
                    format!(
                        "- {} [{}] {}{}{}",
                        artifact.label, artifact.kind, artifact.uri, commit, digest
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let body = format!(
            "Recovery evidence for Work {work_id}, Attempt {attempt_id}, actor {actor_id}. The cognitive process ended without trustworthy semantic completion; productive outcome is UNKNOWN.\nPreserved workspace: {}\nRuntime observations: start {}; end {}.\nLinked artifacts:\n{}\nInspect this same candidate before choosing to revise, resume, reassign, or abandon. Do not infer artifact quality from process exit. Process observation: {}",
            recovery.workspace,
            recovery.start_summary.trim(),
            recovery.end_summary.trim(),
            artifact_lines,
            reason,
        );
        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor, to_actor, body) VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(recovery.observed_by)
        .bind(&coordinator_id)
        .bind(&body)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO work_feedback (work_id,message_id,linked_by) VALUES ($1,$2,$3)")
            .bind(work_id)
            .bind(message_id)
            .bind(recovery.observed_by)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE work_attempts SET recovery_message_id=$2 WHERE id=$1")
            .bind(attempt_id)
            .bind(message_id)
            .execute(&mut *tx)
            .await?;

        let artifact_ref_ids = artifacts
            .iter()
            .map(|artifact| artifact.id)
            .collect::<Vec<_>>();
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('attempt_process_ended',$1,$2)",
        )
        .bind(recovery.observed_by)
        .bind(serde_json::json!({
            "work_id": work_id,
            "attempt_id": attempt_id,
            "semantic_result": "unreported",
            "workspace": {
                "start": recovery.start_observation,
                "end": recovery.end_observation,
            },
        }))
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('attempt_recovery_capsule',$1,$2)",
        )
        .bind(recovery.observed_by)
        .bind(serde_json::json!({
            "work_id": work_id,
            "attempt_id": attempt_id,
            "actor_id": actor_id,
            "coordinator_id": coordinator_id,
            "message_id": message_id,
            "artifact_ref_ids": artifact_ref_ids,
            "workspace_changed": recovery.changed_since_start,
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(Some(AttemptRecoveryNotice {
            work_id,
            attempt_id,
            actor_id,
            coordinator_id,
            message_id,
            artifact_ref_ids,
        }))
    }

    /// Close one running Attempt and deterministically advance the graph.
    /// A result from an old revision is retained as history but can never
    /// complete the current Work revision.
    pub async fn finish_work_attempt(
        &self,
        attempt_id: Uuid,
        result: WorkAttemptState,
        summary: &str,
    ) -> Result<WorkAttemptState> {
        if result == WorkAttemptState::Running {
            return Err(OrgIntelError::InvalidWork(
                "finishing an Attempt requires a terminal result".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT a.work_id, a.actor_id, a.revision AS attempt_revision, a.feedback_cursor, a.state, \
                    w.revision AS work_revision \
             FROM work_attempts a JOIN work w ON w.id = a.work_id \
             WHERE a.id = $1 FOR UPDATE OF a, w",
        )
        .bind(attempt_id)
        .fetch_one(&mut *tx)
        .await?;
        let work_id: Uuid = row.get("work_id");
        let attempt_actor: String = row.get("actor_id");
        let attempt_revision: i64 = row.get("attempt_revision");
        let feedback_cursor: i64 = row.get("feedback_cursor");
        let work_revision: i64 = row.get("work_revision");
        let current_state: WorkAttemptState = row.get("state");
        if current_state != WorkAttemptState::Running {
            tx.commit().await?;
            return Ok(current_state);
        }
        let mut effective = if attempt_revision == work_revision {
            result
        } else {
            WorkAttemptState::Superseded
        };
        let mut effective_summary = summary.to_string();
        let mut qualified_outcome_review = None;
        if effective != WorkAttemptState::Superseded {
            // Work-linked mail sent directly to this Attempt's owner after
            // its frozen input cursor is a changed assignment fact. The live
            // process may have ended before it could observe that fact; never
            // let its stale terminal report close the Work. Superseding keeps
            // the original Attempt truthful and lets the ordinary scheduler
            // create one sequential successor with the exact message bound as
            // initial input. Messages to the accountable lead are deliberately
            // excluded: they wake judgement, not the member's producer.
            let late_direct_feedback = sqlx::query_scalar::<_, i64>(
                "SELECT m.id FROM work_feedback f \
                 JOIN messages m ON m.id=f.message_id \
                 WHERE f.work_id=$1 AND m.to_actor=$2 AND m.id>$3 \
                   AND NOT EXISTS (SELECT 1 FROM work_attempt_feedback af \
                                   WHERE af.attempt_id=$4 AND af.message_id=m.id) \
                 ORDER BY m.id",
            )
            .bind(work_id)
            .bind(&attempt_actor)
            .bind(feedback_cursor)
            .bind(attempt_id)
            .fetch_all(&mut *tx)
            .await?;
            if !late_direct_feedback.is_empty() {
                effective = WorkAttemptState::Superseded;
                effective_summary = format!(
                    "direct Work feedback {} arrived after this Attempt's frozen input snapshot; its terminal report is stale and the Work remains active for a successor Attempt",
                    late_direct_feedback
                        .iter()
                        .map(i64::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
        if effective == WorkAttemptState::ChangesRequested {
            let has_revision_target: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM work_edges WHERE from_work_id=$1 AND kind='revises')",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
            if !has_revision_target {
                // `changes_requested` has graph power only when this node was
                // explicitly modelled as a reviewer. Evidence and research
                // nodes often contain critical findings for a downstream
                // critic; without a revises edge, those findings are their
                // produced result rather than an invalidation instruction.
                // Convert before validation so expected artifacts and gates
                // remain exactly as strict as any other Produced result.
                effective = WorkAttemptState::Produced;
            }
        }
        if effective == WorkAttemptState::Produced {
            let work = sqlx::query(
                "SELECT expected_artifact, owner_review_required, title, outcome \
                 FROM work WHERE id=$1",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
            let expected_artifact: String = work.get("expected_artifact");
            let owner_review_required: bool = work.get("owner_review_required");
            let artifact_present: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM artifact_refs \
                 WHERE work_id=$1 AND attempt_id=$2 AND state='available')",
            )
            .bind(work_id)
            .bind(attempt_id)
            .fetch_one(&mut *tx)
            .await?;
            let gates_passed: bool = sqlx::query_scalar(
                "SELECT NOT EXISTS (\
                   SELECT 1 FROM work_gates g \
                   LEFT JOIN work_gate_runs r ON r.gate_id=g.id AND r.attempt_id=$2 \
                   WHERE g.work_id=$1 AND COALESCE(r.passed, false)=false\
                 )",
            )
            .bind(work_id)
            .bind(attempt_id)
            .fetch_one(&mut *tx)
            .await?;
            if owner_review_required {
                let review_targets = sqlx::query_scalar::<_, String>(
                    "SELECT uri FROM artifact_refs \
                     WHERE work_id=$1 AND attempt_id=$2 AND kind=$3 AND state='available' \
                     ORDER BY created_at, id LIMIT 2",
                )
                .bind(work_id)
                .bind(attempt_id)
                .bind(REVIEW_TARGET_ARTIFACT_KIND)
                .fetch_all(&mut *tx)
                .await?;
                let probe_passed: bool = sqlx::query_scalar(
                    "SELECT EXISTS(\
                       SELECT 1 FROM work_gates g JOIN work_gate_runs r ON r.gate_id=g.id \
                       WHERE g.work_id=$1 AND g.name=$2 AND r.attempt_id=$3 AND r.passed\
                     )",
                )
                .bind(work_id)
                .bind(REVIEW_TARGET_LIVE_PROBE_GATE)
                .bind(attempt_id)
                .fetch_one(&mut *tx)
                .await?;
                match review_targets.as_slice() {
                    [] => {
                        effective = WorkAttemptState::Failed;
                        effective_summary = "declared owner review without linking one available ReviewTarget artifact".into();
                    }
                    [_] if !gates_passed => {
                        effective = WorkAttemptState::Failed;
                        effective_summary =
                            "one or more deterministic Work gates did not pass".into();
                    }
                    [target] if !probe_passed => {
                        effective = WorkAttemptState::Failed;
                        effective_summary = format!(
                            "declared owner review but {REVIEW_TARGET_LIVE_PROBE_GATE:?} did not pass for ReviewTarget {target:?}"
                        );
                    }
                    [target] => {
                        qualified_outcome_review = Some(QualifiedOutcomeReview {
                            target_uri: target.clone(),
                            title: work.get("title"),
                            outcome: work.get("outcome"),
                        });
                    }
                    _ => {
                        effective = WorkAttemptState::Failed;
                        effective_summary = "declared owner review with multiple available ReviewTarget artifacts; select one prepared native outcome".into();
                    }
                }
            } else if !expected_artifact.trim().is_empty() && !artifact_present {
                effective = WorkAttemptState::Failed;
                effective_summary = format!(
                    "declared complete without linking expected artifact: {expected_artifact}"
                );
            } else if !gates_passed {
                effective = WorkAttemptState::Failed;
                effective_summary = "one or more deterministic Work gates did not pass".into();
            }
        }
        sqlx::query(
            "UPDATE work_attempts SET state = $2, summary = $3, finished_at = now() WHERE id = $1",
        )
        .bind(attempt_id)
        .bind(effective)
        .bind(&effective_summary)
        .execute(&mut *tx)
        .await?;

        match effective {
            WorkAttemptState::Produced => {
                if let Some(review) = qualified_outcome_review {
                    let handoff_id = create_qualified_outcome_review(
                        &mut tx,
                        work_id,
                        attempt_id,
                        &attempt_actor,
                        work_revision,
                        review,
                    )
                    .await?;
                    sqlx::query("UPDATE work SET status='blocked', resolution=$2 WHERE id=$1")
                        .bind(work_id)
                        .bind(format!("awaiting owner outcome review {handoff_id}"))
                        .execute(&mut *tx)
                        .await?;
                } else {
                    sqlx::query(
                        "UPDATE work SET status = 'completed', resolution = $2 WHERE id = $1",
                    )
                    .bind(work_id)
                    .bind(&effective_summary)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            WorkAttemptState::ChangesRequested => {
                // The reviewer completed its assignment, then each revises
                // edge invalidates the target and its hard descendants.
                sqlx::query("UPDATE work SET status = 'completed', resolution = $2 WHERE id = $1")
                    .bind(work_id)
                    .bind(&effective_summary)
                    .execute(&mut *tx)
                    .await?;
                let targets = sqlx::query_scalar::<_, Uuid>(
                    "SELECT to_work_id FROM work_edges \
                     WHERE from_work_id = $1 AND kind = 'revises' ORDER BY to_work_id",
                )
                .bind(work_id)
                .fetch_all(&mut *tx)
                .await?;
                for target in targets {
                    invalidate_from(&mut tx, target, &attempt_actor, &effective_summary).await?;
                }
            }
            WorkAttemptState::Blocked => {
                sqlx::query("UPDATE work SET status = 'blocked', resolution = $2 WHERE id = $1")
                    .bind(work_id)
                    .bind(&effective_summary)
                    .execute(&mut *tx)
                    .await?;
            }
            WorkAttemptState::Failed => {
                // A blind automatic retry repeats the same mechanism before a
                // lead can repair it. Preserve the workspace and stop this node
                // until its owner/lead/Exec explicitly resumes it with a reason.
                sqlx::query("UPDATE work SET status='blocked', resolution=$2 WHERE id=$1")
                    .bind(work_id)
                    .bind(&effective_summary)
                    .execute(&mut *tx)
                    .await?;
            }
            WorkAttemptState::Abandoned => {
                sqlx::query("UPDATE work SET status = 'abandoned', resolution = $2 WHERE id = $1")
                    .bind(work_id)
                    .bind(&effective_summary)
                    .execute(&mut *tx)
                    .await?;
            }
            WorkAttemptState::Superseded => {}
            WorkAttemptState::Running => unreachable!(),
        }
        tx.commit().await?;
        Ok(effective)
    }

    /// Resume a repaired Work node. The reason is operational evidence of
    /// what changed; retrying the same vague instruction is deliberately not
    /// an automatic scheduler behaviour.
    pub async fn resume_work(&self, work_id: Uuid, by: &str, reason: &str) -> Result<()> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "resuming Work needs the concrete repair or changed mechanism".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT owner_id, status, attempt_limit, revision FROM work WHERE id=$1 FOR UPDATE",
        )
        .bind(work_id)
        .fetch_one(&mut *tx)
        .await?;
        let owner_id: String = row.get("owner_id");
        let status: WorkStatus = row.get("status");
        if status != WorkStatus::Blocked {
            return Err(OrgIntelError::InvalidWork(
                "only blocked Work can be resumed after repair".into(),
            ));
        }
        let allowed = by == "owner"
            || by == "exec"
            || by == owner_id
            || self.team_lead_for(&owner_id).await?.as_deref() == Some(by);
        if !allowed {
            return Err(OrgIntelError::InvalidWork(format!(
                "{by:?} is not the Work owner, its lead, the Exec, or the owner"
            )));
        }
        let exhausted: bool = sqlx::query_scalar(
            "SELECT attempt_limit IS NOT NULL AND (SELECT count(*) FROM work_attempts \
             WHERE work_id=$1 AND revision=work.revision) >= attempt_limit FROM work WHERE id=$1",
        )
        .bind(work_id)
        .fetch_one(&mut *tx)
        .await?;
        if exhausted {
            return Err(OrgIntelError::InvalidWork(
                "attempt limit reached; revise or replace the Work instead of resuming it".into(),
            ));
        }
        sqlx::query("UPDATE work SET status='active', resolution=$2 WHERE id=$1")
            .bind(work_id)
            .bind(format!("resumed by {by}: {}", reason.trim()))
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('work_repaired',$1,$2)")
            .bind(by)
            .bind(serde_json::json!({ "work_id": work_id, "reason": reason.trim() }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Retire superseded coordination state without deleting Work, Attempts,
    /// artifacts, or attribution. A live Attempt must be stopped and observed
    /// first so abandoning Work never becomes an implicit process kill.
    pub async fn abandon_work(&self, work_id: Uuid, by: &str, reason: &str) -> Result<()> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "abandoning Work needs an observed reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT owner_id, status FROM work WHERE id=$1 FOR UPDATE")
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
        let owner_id: String = row.get("owner_id");
        let status: WorkStatus = row.get("status");
        if matches!(status, WorkStatus::Completed | WorkStatus::Abandoned) {
            return Err(OrgIntelError::InvalidWork(
                "completed or already-abandoned Work keeps its recorded outcome".into(),
            ));
        }
        let allowed = by == "owner"
            || by == "exec"
            || by == owner_id
            || self.team_lead_for(&owner_id).await?.as_deref() == Some(by);
        if !allowed {
            return Err(OrgIntelError::InvalidWork(format!(
                "{by:?} is not the Work owner, its lead, the Exec, or the owner"
            )));
        }
        let running: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work_attempts WHERE work_id=$1 AND state='running')",
        )
        .bind(work_id)
        .fetch_one(&mut *tx)
        .await?;
        if running {
            return Err(OrgIntelError::InvalidWork(
                "Work has a running Attempt; stop and observe that process before abandoning it"
                    .into(),
            ));
        }
        sqlx::query(
            "UPDATE work SET status='abandoned', resolution=$2, updated_at=now() WHERE id=$1",
        )
        .bind(work_id)
        .bind(format!("abandoned by {by}: {}", reason.trim()))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('work_abandoned',$1,$2)")
            .bind(by)
            .bind(serde_json::json!({ "work_id": work_id, "reason": reason.trim() }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

/// Materialise the owner-facing side of a qualified produced outcome in the
/// same transaction as Attempt completion. The accountable producer chose and
/// live-probed the target; this deterministic transition only brings that
/// exact candidate to the owner and never accepts it or interprets it.
async fn create_qualified_outcome_review(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    work_id: Uuid,
    attempt_id: Uuid,
    requested_by: &str,
    work_revision: i64,
    review: QualifiedOutcomeReview,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let requested_action = format!(
        "Open the prepared outcome at {}. Accept it if it meets the declared outcome, or request concrete changes.",
        review.target_uri
    );
    let prepared_state = format!(
        "ReviewTarget: {}\nDeclared outcome: {}\n{} passed for this Attempt.",
        review.target_uri, review.outcome, REVIEW_TARGET_LIVE_PROBE_GATE
    );
    let resume_condition =
        "The owner accepts the prepared outcome or records concrete requested changes.";
    let brief = OwnerBrief {
        kind: OwnerBriefKind::OutcomeReview,
        headline: format!("Review: {}", review.title),
        situation: format!(
            "The accountable producer completed the declared outcome and prepared this live-probed ReviewTarget: {}.",
            review.target_uri
        ),
        impact: review.outcome,
        recommendation: "Open the prepared outcome, compare it with the declared outcome, then accept it or request concrete changes.".into(),
        no_action: "The Work remains blocked with its prepared outcome intact until this review is decided.".into(),
        uncertainty: None,
        deadline: None,
    };
    validate_owner_brief(&brief)?;
    let brief = serde_json::to_value(&brief).map_err(|error| {
        OrgIntelError::InvalidWork(format!("invalid generated outcome review brief: {error}"))
    })?;
    let fingerprint = owner_handoff_source_fingerprint(
        work_id,
        Some(attempt_id),
        OwnerHandoffCategory::OwnerJudgement,
        &requested_action,
        &prepared_state,
        resume_condition,
        work_revision,
    );
    sqlx::query(
        "INSERT INTO owner_handoffs \
         (id, work_id, attempt_id, requested_by, category, requested_action, prepared_state, \
          resume_condition, assigned_to, escalated_from, escalated_at, owner_brief, briefed_by, \
          briefed_at, brief_source_fingerprint) \
         VALUES ($1,$2,$3,$4,'owner_judgement',$5,$6,$7,NULL,$4,now(),$8,$4,now(),$9)",
    )
    .bind(id)
    .bind(work_id)
    .bind(attempt_id)
    .bind(requested_by)
    .bind(&requested_action)
    .bind(&prepared_state)
    .bind(resume_condition)
    .bind(&brief)
    .bind(&fingerprint)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO events (kind, actor_id, body) VALUES ('outcome_review_prepared',$1,$2)",
    )
    .bind(requested_by)
    .bind(serde_json::json!({
        "handoff_id": id,
        "work_id": work_id,
        "attempt_id": attempt_id,
        "review_target": review.target_uri,
        "work_revision": work_revision,
    }))
    .execute(&mut **tx)
    .await?;
    Ok(id)
}
