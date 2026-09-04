//! Atomic Work claims, Attempt lifecycle, and bounded recovery capsules.
//!
//! This is a move-only ownership split; Git and Runtime remain the source of files.

use super::*;

struct QualifiedOutcomeReview {
    target_uri: String,
    title: String,
    outcome: String,
}

type SupervisorNoticeFact = (
    Uuid,
    Uuid,
    String,
    WorkAttemptState,
    WorkStatus,
    i64,
    String,
);

fn looks_like_exact_git_commit(value: &str) -> bool {
    (7..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn resolve_attempt_base(
    declared: Option<&str>,
    prior_revision_commit: Option<&str>,
    retained_blocked_commit: Option<&str>,
    upstream_commits: &[String],
) -> Option<String> {
    if retained_blocked_commit.is_some_and(looks_like_exact_git_commit) {
        return retained_blocked_commit.map(str::to_string);
    }
    if prior_revision_commit.is_some_and(looks_like_exact_git_commit) {
        return prior_revision_commit.map(str::to_string);
    }
    if declared.is_some_and(looks_like_exact_git_commit) {
        return declared.map(str::to_string);
    }
    match upstream_commits {
        [commit] if looks_like_exact_git_commit(commit) => Some(commit.clone()),
        _ => declared.map(str::to_string),
    }
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
        // Supersession means the frozen input snapshot changed. It is not an
        // attributable execution failure and must leave room for the promised
        // successor Attempt even when the Work allows only one real attempt.
        let work = sqlx::query_as::<_, WorkRow>(
            "SELECT w.id, w.goal_id, w.owner_id, w.title, w.outcome, w.status, \
                    w.resolution, w.priority, w.expected_artifact, w.owner_review_required, \
                    w.producing_topology, w.commissioned_by, w.repo, w.base_ref, \
                    w.integration_branch, w.worktree, w.revision, w.attempt_limit, \
                    w.created_at, w.updated_at \
             FROM work w \
             WHERE w.status IN ('proposed','active') \
               AND w.owner_id <> ALL($1::text[]) \
               AND NOT EXISTS (SELECT 1 FROM teams t \
                               WHERE t.lead_actor_id = w.owner_id AND t.disbanded_at IS NULL) \
               AND w.owner_id NOT IN ('owner','exec','world','daemon') \
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
                    WHERE a.work_id = w.id AND a.revision = w.revision \
                      AND a.state <> 'superseded'\
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
             FROM artifact_refs a \
             WHERE (a.state = 'available' AND EXISTS(SELECT 1 FROM work_edges e \
                      WHERE e.to_work_id=$1 AND e.kind='requires' \
                        AND e.from_work_id=a.work_id)) \
               OR (a.work_id=$1 AND a.state IN ('available','superseded') \
                   AND a.kind IN ('output','review_target','repository_tree') AND EXISTS(\
                     SELECT 1 FROM work_attempts prior \
                     WHERE prior.id=a.attempt_id AND prior.revision <= $2 \
                       AND prior.state IN ('produced','superseded')\
                  )) \
             ORDER BY a.work_id, a.created_at, a.id",
        )
        .bind(work.id)
        .bind(work.revision)
        .fetch_all(&mut *tx)
        .await?;
        let upstream_source_commits = if let Some(repo) = work.repo.as_deref() {
            sqlx::query_scalar::<_, String>(
                "SELECT DISTINCT a.source_commit \
                 FROM work_edges e \
                 JOIN work upstream ON upstream.id=e.from_work_id \
                 JOIN artifact_refs a ON a.work_id=upstream.id \
                 WHERE e.to_work_id=$1 AND e.kind='requires' \
                   AND upstream.repo=$2 AND a.state='available' \
                   AND a.source_commit IS NOT NULL \
                 ORDER BY a.source_commit",
            )
            .bind(work.id)
            .bind(repo)
            .fetch_all(&mut *tx)
            .await?
        } else {
            Vec::new()
        };
        let prior_work_commit = if work.repo.is_some() {
            sqlx::query_scalar::<_, String>(
                "SELECT a.source_commit \
                 FROM artifact_refs a \
                 JOIN work_attempts attempt ON attempt.id=a.attempt_id \
                 WHERE a.work_id=$1 AND attempt.revision <= $2 \
                   AND a.source_commit IS NOT NULL AND a.state IN ('available','superseded') \
                   AND (attempt.state IN ('produced','superseded') OR (\
                     attempt.state='failed' \
                     AND a.kind='repository_tree' \
                     AND attempt.terminal_dirty_entries=0 \
                     AND attempt.terminal_source_commit=a.source_commit \
                     AND attempt.terminal_source_tree=a.digest\
                   )) \
                 ORDER BY attempt.revision DESC, attempt.attempt_no DESC, \
                          a.created_at DESC, a.id DESC \
                 LIMIT 1",
            )
            .bind(work.id)
            .bind(work.revision)
            .fetch_optional(&mut *tx)
            .await?
        } else {
            None
        };
        // `resume_work` is an explicit coordinator judgement that the blocker
        // was repaired. Continue from the newest clean committed candidate of
        // the blocked Attempt, never from uncommitted bytes or the mutable
        // worktree path itself.
        let retained_blocked_commit = if work.repo.is_some() {
            sqlx::query_scalar::<_, String>(
                "SELECT terminal_source_commit FROM work_attempts \
                 WHERE work_id=$1 AND revision=$2 AND state='blocked' \
                   AND terminal_dirty_entries=0 \
                   AND terminal_source_commit IS NOT NULL \
                   AND terminal_source_tree IS NOT NULL \
                 ORDER BY attempt_no DESC LIMIT 1",
            )
            .bind(work.id)
            .bind(work.revision)
            .fetch_optional(&mut *tx)
            .await?
        } else {
            None
        };
        let effective_base_ref = resolve_attempt_base(
            work.base_ref.as_deref(),
            prior_work_commit.as_deref(),
            retained_blocked_commit.as_deref(),
            upstream_source_commits.as_slice(),
        );
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
        fingerprint_source.push_str(&format!(
            "\nworkspace_base:{}",
            effective_base_ref.as_deref().unwrap_or("")
        ));
        let feedback = sqlx::query_as::<_, MessageRow>(
            "SELECT id,from_actor,to_actor,body,outcome_standard,created_at,read_at FROM (\
               SELECT m.id,m.from_actor,m.to_actor,m.body,m.outcome_standard,m.created_at,m.read_at \
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
        let gate_rows = sqlx::query(
            "SELECT name,cwd,command,sequence_no,stage,timeout_seconds,resources \
             FROM work_gates WHERE work_id=$1 AND retired_at IS NULL ORDER BY sequence_no",
        )
        .bind(work.id)
        .fetch_all(&mut *tx)
        .await?;
        let mut gate_source = String::new();
        for gate in gate_rows {
            gate_source.push_str(&format!(
                "{}\0{}\0{}\0{}\0{}\0{}\0{}\n",
                gate.get::<String, _>("name"),
                gate.get::<String, _>("cwd"),
                gate.get::<serde_json::Value, _>("command"),
                gate.get::<i32, _>("sequence_no"),
                gate.get::<String, _>("stage"),
                gate.get::<i32, _>("timeout_seconds"),
                gate.get::<serde_json::Value, _>("resources"),
            ));
        }
        let gate_set_digest = format!("{:x}", Sha256::digest(gate_source.as_bytes()));
        let attempt_no: i64 = sqlx::query_scalar(
            "SELECT count(*) + 1 FROM work_attempts WHERE work_id = $1 AND revision = $2",
        )
        .bind(work.id)
        .bind(work.revision)
        .fetch_one(&mut *tx)
        .await?;
        let previous_attempt = sqlx::query_as::<_, (WorkAttemptState, String)>(
            "SELECT state, summary FROM work_attempts \
             WHERE work_id=$1 AND revision=$2 \
             ORDER BY attempt_no DESC LIMIT 1",
        )
        .bind(work.id)
        .bind(work.revision)
        .fetch_optional(&mut *tx)
        .await?;
        let review_targets = sqlx::query_scalar::<_, Uuid>(
            "SELECT to_work_id FROM work_edges \
             WHERE from_work_id=$1 AND kind='revises' ORDER BY to_work_id",
        )
        .bind(work.id)
        .fetch_all(&mut *tx)
        .await?;
        let attempt_id = Uuid::new_v4();
        let session_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO work_attempts \
             (id, work_id, revision, attempt_no, actor_id, session_id, trigger, input_fingerprint, \
              feedback_cursor, feedback_checkpoint_cursor, requested_source_ref, gate_set_digest) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$9,$10,$11)",
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
        .bind(effective_base_ref.as_deref())
        .bind(&gate_set_digest)
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
            review_targets,
            effective_base_ref,
            attempt_id,
            attempt_no: i32::try_from(attempt_no).unwrap_or(i32::MAX),
            session_id,
            input_fingerprint,
            inputs,
            feedback,
            previous_attempt_state: previous_attempt.as_ref().map(|(state, _)| *state),
            previous_attempt_summary: previous_attempt.map(|(_, summary)| summary),
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

    pub async fn record_agent_session(&self, session: NewAgentSession<'_>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO agent_sessions \
             (launch_id,actor_id,responsibility,work_id,attempt_id,harness,harness_build,transport,model,configured_effort,provider_session_id,capabilities,resumed,reconstructed) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) \
             ON CONFLICT (launch_id) DO NOTHING",
        )
        .bind(session.launch_id)
        .bind(session.actor_id)
        .bind(session.responsibility)
        .bind(session.work_id)
        .bind(session.attempt_id)
        .bind(session.harness)
        .bind(session.harness_build)
        .bind(session.transport)
        .bind(session.model)
        .bind(session.configured_effort)
        .bind(session.provider_session_id)
        .bind(session.capabilities)
        .bind(session.resumed)
        .bind(session.reconstructed)
        .execute(&mut *tx)
        .await?;
        if let Some(attempt_id) = session.attempt_id {
            sqlx::query(
                "UPDATE work_attempts SET harness=$2,harness_build=$3,harness_transport=$4,harness_capabilities=$5 \
                 WHERE id=$1 AND state='running'",
            )
            .bind(attempt_id)
            .bind(session.harness)
            .bind(session.harness_build)
            .bind(session.transport)
            .bind(session.capabilities)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Read the durable launch observations used to audit which certified
    /// harness actually ran. Configuration expresses intent; this history is
    /// the evidence for both coordination and productive sessions.
    pub async fn list_agent_sessions(
        &self,
        work_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<AgentSessionRow>> {
        Ok(sqlx::query_as(
            "SELECT launch_id,actor_id,responsibility,work_id,attempt_id,harness,harness_build,transport,model,configured_effort,provider_session_id,capabilities,resumed,reconstructed,started_at \
             FROM agent_sessions WHERE ($1::uuid IS NULL OR work_id=$1) \
             ORDER BY started_at DESC,launch_id DESC LIMIT $2",
        )
        .bind(work_id)
        .bind(limit.clamp(1, 1_000))
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn list_work_attempts(&self, work_id: Option<Uuid>) -> Result<Vec<WorkAttemptRow>> {
        Ok(sqlx::query_as(
            "SELECT id, work_id, revision, attempt_no, actor_id, session_id, state, trigger, \
                    input_fingerprint, feedback_cursor, requested_source_ref, source_commit, \
                    source_tree, terminal_source_commit, terminal_source_tree, \
                    terminal_status_digest, terminal_dirty_entries, terminal_observed_at, \
                    gate_set_digest, environment_fingerprint, materialized_at, \
                    interrupt_requested_at, interrupt_requested_by, interrupt_reason, \
                    feedback_checkpoint_cursor, model, harness, harness_build, harness_transport, \
                    harness_capabilities, started_at, finished_at, summary \
             FROM work_attempts WHERE ($1::uuid IS NULL OR work_id = $1) \
             ORDER BY started_at, id",
        )
        .bind(work_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Recovery needs only live executions. Keeping this query separate from
    /// the owner-facing history projection makes daemon restart cost scale
    /// with current work rather than every Attempt the company has completed.
    pub async fn list_running_work_attempts(&self) -> Result<Vec<WorkAttemptRow>> {
        Ok(sqlx::query_as(
            "SELECT id, work_id, revision, attempt_no, actor_id, session_id, state, trigger, \
                    input_fingerprint, feedback_cursor, requested_source_ref, source_commit, \
                    source_tree, terminal_source_commit, terminal_source_tree, \
                    terminal_status_digest, terminal_dirty_entries, terminal_observed_at, \
                    gate_set_digest, environment_fingerprint, materialized_at, \
                    interrupt_requested_at, interrupt_requested_by, interrupt_reason, \
                    feedback_checkpoint_cursor, model, harness, harness_build, harness_transport, \
                    harness_capabilities, started_at, finished_at, summary \
             FROM work_attempts WHERE state='running' ORDER BY started_at, id",
        )
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
            "SELECT a.work_id, a.actor_id, a.revision AS attempt_revision, a.state, \
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
        // Ordinary feedback is queued information, never an implicit
        // interrupt. Feedback delivered by a safe checkpoint is already in
        // `work_attempt_feedback`. A final race after the last checkpoint
        // preserves this Attempt's useful result and schedules one successor
        // revision instead of relabelling the work as superseded.
        let late_direct_feedback = if effective != WorkAttemptState::Superseded {
            sqlx::query_scalar::<_, i64>(
                "SELECT m.id FROM work_feedback f JOIN messages m ON m.id=f.message_id \
                 WHERE f.work_id=$1 AND m.to_actor=$2 \
                   AND NOT EXISTS (SELECT 1 FROM work_attempt_feedback af \
                                   WHERE af.attempt_id=$3 AND af.message_id=m.id) \
                 ORDER BY m.id",
            )
            .bind(work_id)
            .bind(&attempt_actor)
            .bind(attempt_id)
            .fetch_all(&mut *tx)
            .await?
        } else {
            Vec::new()
        };
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
                "SELECT EXISTS(SELECT 1 FROM artifact_refs a \
                 WHERE a.work_id=$1 AND a.state='available' AND (\
                   a.attempt_id=$2 OR EXISTS(\
                     SELECT 1 FROM work_attempt_inputs input \
                     WHERE input.attempt_id=$2 AND input.artifact_ref_id=a.id\
                   )\
                 ))",
            )
            .bind(work_id)
            .bind(attempt_id)
            .fetch_one(&mut *tx)
            .await?;
            let gates_passed: bool = sqlx::query_scalar(
                "SELECT NOT EXISTS (\
                   SELECT 1 FROM work_gates g \
                   LEFT JOIN work_gate_runs r ON r.gate_id=g.id AND r.attempt_id=$2 \
                   WHERE g.work_id=$1 AND g.retired_at IS NULL \
                     AND COALESCE(r.passed, false)=false\
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
                       WHERE g.work_id=$1 AND g.name=$2 AND g.retired_at IS NULL \
                         AND r.attempt_id=$3 AND r.passed\
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
        let followup_revision = effective == WorkAttemptState::Produced
            && !late_direct_feedback.is_empty()
            && qualified_outcome_review.is_none();
        // A clean passing terminal result is observable state, not a reason to
        // spend a lead turn. Qualified owner review already creates its own
        // exact judgement obligation. Only an unresolved terminal exception
        // enters the supervisor outbox here.
        let material_supervisor_notice = matches!(
            effective,
            WorkAttemptState::Blocked | WorkAttemptState::Failed | WorkAttemptState::Abandoned
        );
        sqlx::query(
            "UPDATE work_attempts SET state=$2, summary=$3, finished_at=now(), \
                    supervisor_notice_owed=$4, supervisor_notice_message_id=NULL \
             WHERE id=$1",
        )
        .bind(attempt_id)
        .bind(effective)
        .bind(&effective_summary)
        .bind(material_supervisor_notice && !followup_revision)
        .execute(&mut *tx)
        .await?;

        match effective {
            WorkAttemptState::Produced => {
                if followup_revision {
                    sqlx::query(
                        "UPDATE work SET status='active', revision=revision+1, resolution=$2 WHERE id=$1",
                    )
                    .bind(work_id)
                    .bind(format!(
                        "Attempt {attempt_id} produced useful output; direct feedback {} arrived after its final checkpoint and is queued for the next revision",
                        late_direct_feedback
                            .iter()
                            .map(i64::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                    .execute(&mut *tx)
                    .await?;
                } else if let Some(review) = qualified_outcome_review {
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
                        .bind(format!(
                            "awaiting accountable-lead outcome review {handoff_id}"
                        ))
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

    /// Flush material terminal exceptions to their accountable leads. Clean
    /// completion never enters this outbox. Exceptions from the same Work are
    /// causally coalesced; unrelated Work remains separately reviewable.
    /// Attempt completion and the outbox bit commit together, while message
    /// creation and clearing the bit also commit together, so crash replay is
    /// safe and exactly once.
    pub async fn flush_terminal_supervisor_notices(&self, limit: i64) -> Result<Vec<i64>> {
        if limit <= 0 {
            return Ok(Vec::new());
        }
        self.ensure_actor("daemon", "system", "system-sender", "The daemon")
            .await?;
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query(
            "SELECT attempt.id AS attempt_id, attempt.actor_id, attempt.state AS attempt_state, \
                    work.id AS work_id, work.status, work.revision, work.resolution, team.lead_actor_id \
             FROM work_attempts attempt \
             JOIN work ON work.id=attempt.work_id \
             JOIN actors work_owner ON work_owner.id=work.owner_id \
             JOIN teams team ON team.id=work_owner.team_id AND team.disbanded_at IS NULL \
             WHERE attempt.supervisor_notice_owed AND attempt.finished_at IS NOT NULL \
             ORDER BY attempt.finished_at, attempt.id \
             LIMIT $1 FOR UPDATE OF attempt SKIP LOCKED",
        )
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;
        let mut by_cause: std::collections::BTreeMap<(String, Uuid), Vec<SupervisorNoticeFact>> =
            std::collections::BTreeMap::new();
        for row in rows {
            let work_id = row.get("work_id");
            by_cause
                .entry((row.get("lead_actor_id"), work_id))
                .or_default()
                .push((
                    row.get("attempt_id"),
                    work_id,
                    row.get("actor_id"),
                    row.get("attempt_state"),
                    row.get("status"),
                    row.get("revision"),
                    row.get("resolution"),
                ));
        }
        let mut message_ids = Vec::with_capacity(by_cause.len());
        for ((lead_actor_id, cause_work_id), notices) in by_cause {
            let body = notices
                .iter()
                .map(
                    |(
                        attempt_id,
                        work_id,
                        actor_id,
                        attempt_state,
                        status,
                        revision,
                        resolution,
                    )| {
                        let status = match status {
                            WorkStatus::Proposed => "proposed",
                            WorkStatus::Active => "active",
                            WorkStatus::Blocked => "blocked",
                            WorkStatus::Completed => "completed",
                            WorkStatus::Abandoned => "abandoned",
                        };
                        let attempt_state = attempt_state.as_str();
                        format!(
                            "Work {work_id}, Attempt {attempt_id}, producer {actor_id}: Attempt {attempt_state}, Work {status}, revision {revision}. Resolution: {resolution}"
                        )
                    },
                )
                .collect::<Vec<_>>()
                .join("\n");
            let body = format!(
                "Material Runtime supervisor events for Work {cause_work_id} ({} exception{}):\n{body}",
                notices.len(),
                if notices.len() == 1 { "" } else { "s" },
            );
            let message_id: i64 = sqlx::query_scalar(
                "INSERT INTO messages (from_actor, to_actor, body) \
                 VALUES ('daemon',$1,$2) RETURNING id",
            )
            .bind(&lead_actor_id)
            .bind(&body)
            .fetch_one(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO work_feedback (work_id, message_id, linked_by) \
                 VALUES ($1,$2,'daemon')",
            )
            .bind(cause_work_id)
            .bind(message_id)
            .execute(&mut *tx)
            .await?;
            for (attempt_id, ..) in &notices {
                sqlx::query(
                    "UPDATE work_attempts SET supervisor_notice_owed=false, \
                            supervisor_notice_message_id=$2 WHERE id=$1",
                )
                .bind(attempt_id)
                .bind(message_id)
                .execute(&mut *tx)
                .await?;
            }
            message_ids.push(message_id);
        }
        tx.commit().await?;
        Ok(message_ids)
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
             WHERE work_id=$1 AND revision=work.revision AND state <> 'superseded') \
             >= attempt_limit FROM work WHERE id=$1",
        )
        .bind(work_id)
        .fetch_one(&mut *tx)
        .await?;
        // `resume` is the accountable coordinator's explicit decision that a
        // concrete repair changed the mechanism. If the old bounded allowance
        // is exhausted, grant exactly one successor Attempt rather than
        // forcing a fake replacement Work node or silently retrying. Each
        // additional grant therefore requires another terminal observation
        // and another attributable repair decision.
        sqlx::query(
            "UPDATE work SET status='active', resolution=$2, \
                    attempt_limit=CASE WHEN $3 AND attempt_limit IS NOT NULL \
                                             AND attempt_limit < 2147483647 \
                                       THEN attempt_limit + 1 ELSE attempt_limit END \
             WHERE id=$1",
        )
        .bind(work_id)
        .bind(format!("resumed by {by}: {}", reason.trim()))
        .bind(exhausted)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('work_repaired',$1,$2)")
            .bind(by)
            .bind(serde_json::json!({
                "work_id": work_id,
                "reason": reason.trim(),
                "attempt_limit_extended": exhausted,
            }))
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
        // A pending handoff is a projection of this Work's current blocker,
        // not an independent owner obligation. Retiring the Work must retire
        // that prepared action in the same OrgIntel transaction; otherwise
        // Attention keeps asking the owner to authorize a path the company
        // has already declared obsolete.
        sqlx::query(
            "UPDATE owner_handoffs SET state='withdrawn', \
                    resolution=$2, resolved_at=now() \
             WHERE work_id=$1 AND state='pending'",
        )
        .bind(work_id)
        .bind(format!(
            "Withdrawn because Work was abandoned by {by}: {}",
            reason.trim()
        ))
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

/// Materialise the supervisory review side of a qualified produced outcome in
/// the same transaction as Attempt completion. The accountable producer chose
/// and live-probed the target; its lead must inspect and judge that exact
/// candidate before any owner-facing admission. This preserves a non-producing
/// supervisor without turning the deterministic transition into acceptance.
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
        "Inspect the prepared outcome at {}. Redirect the worker with concrete Work feedback if it misses the declared outcome; otherwise prepare and escalate only the exact irreducible owner judgement.",
        review.target_uri
    );
    let prepared_state = format!(
        "ReviewTarget: {}\nDeclared outcome: {}\n{} passed for this Attempt.",
        review.target_uri, review.outcome, REVIEW_TARGET_LIVE_PROBE_GATE
    );
    let resume_condition =
        "The accountable lead either redirects attributable revision Work or admits the exact prepared outcome through the remaining Exec/owner judgement boundary.";
    let assigned_to = sqlx::query_scalar::<_, String>(
        "SELECT t.lead_actor_id FROM actors a JOIN teams t ON t.id=a.team_id \
         WHERE a.id=$1 AND a.retired_at IS NULL AND t.disbanded_at IS NULL \
           AND t.lead_actor_id<>a.id",
    )
    .bind(requested_by)
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or_else(|| "exec".to_string());
    sqlx::query(
        "INSERT INTO owner_handoffs \
         (id, work_id, attempt_id, requested_by, category, requested_action, prepared_state, \
          resume_condition, assigned_to) \
         VALUES ($1,$2,$3,$4,'owner_judgement',$5,$6,$7,$8)",
    )
    .bind(id)
    .bind(work_id)
    .bind(attempt_id)
    .bind(requested_by)
    .bind(&requested_action)
    .bind(&prepared_state)
    .bind(resume_condition)
    .bind(&assigned_to)
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
        "assigned_to": assigned_to,
        "title": review.title,
        "outcome": review.outcome,
    }))
    .execute(&mut **tx)
    .await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::resolve_attempt_base;

    #[test]
    fn clean_retained_candidate_precedes_the_original_base() {
        let original = "1111111111111111111111111111111111111111";
        let retained = "2222222222222222222222222222222222222222";
        assert_eq!(
            resolve_attempt_base(Some(original), None, Some(retained), &[]).as_deref(),
            Some(retained)
        );
    }

    #[test]
    fn absent_retained_candidate_preserves_existing_resolution() {
        let original = "1111111111111111111111111111111111111111";
        assert_eq!(
            resolve_attempt_base(Some(original), None, None, &[]).as_deref(),
            Some(original)
        );
    }
}
