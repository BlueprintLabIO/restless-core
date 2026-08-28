//! Artifact references, deterministic gates, and Work graph snapshots.
//!
//! This is a move-only ownership split; Git and Runtime remain the source of files.

use super::*;

impl OrgIntel {
    /// Artifact references point at Runtime truth; digest/commit make the
    /// exact version an Attempt consumed checkable without importing custody.
    pub async fn link_work_artifact(&self, artifact: NewArtifactRef<'_>) -> Result<Uuid> {
        if artifact.kind.trim().is_empty()
            || artifact.uri.trim().is_empty()
            || artifact.label.trim().is_empty()
        {
            return Err(OrgIntelError::InvalidWork(
                "a Work artifact needs kind, locator and label".into(),
            ));
        }
        if artifact.work_id.is_some() != artifact.attempt_id.is_some() {
            return Err(OrgIntelError::InvalidWork(
                "a Work artifact must name both its Work and producer Attempt".into(),
            ));
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        if let (Some(work_id), Some(attempt_id)) = (artifact.work_id, artifact.attempt_id) {
            let matches: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM work_attempts \
                 WHERE id=$1 AND work_id=$2 AND actor_id=$3)",
            )
            .bind(attempt_id)
            .bind(work_id)
            .bind(artifact.created_by)
            .fetch_one(&mut *tx)
            .await?;
            if !matches {
                return Err(OrgIntelError::InvalidWork(
                    "artifact producer, Attempt and Work do not match".into(),
                ));
            }
        }
        sqlx::query(
            "INSERT INTO artifact_refs \
             (id, kind, uri, note, created_by, work_id, attempt_id, digest, source_commit, \
              runtime_generation, label, state) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'available')",
        )
        .bind(id)
        .bind(artifact.kind)
        .bind(artifact.uri)
        .bind(artifact.note)
        .bind(artifact.created_by)
        .bind(artifact.work_id)
        .bind(artifact.attempt_id)
        .bind(artifact.digest)
        .bind(artifact.source_commit)
        .bind(artifact.runtime_generation)
        .bind(artifact.label)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Canonicalise artifact versions against the clean Git commit observed
    /// by the Runtime when their producing Attempt ends. Staff may omit the
    /// redundant flag or provide an abbreviated hash; the Runtime observation
    /// remains the authoritative source of the full commit.
    pub async fn bind_attempt_artifacts_to_observed_commit(
        &self,
        attempt_id: Uuid,
        observed_commit: &str,
    ) -> Result<u64> {
        if observed_commit.len() != 40
            || !observed_commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(OrgIntelError::InvalidWork(
                "an observed artifact commit must be a full Git object id".into(),
            ));
        }
        let result = sqlx::query(
            "UPDATE artifact_refs SET source_commit=$2 \
             WHERE attempt_id=$1 AND state='available' \
               AND (source_commit IS NULL OR $2 LIKE source_commit || '%')",
        )
        .bind(attempt_id)
        .bind(observed_commit)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn list_artifact_refs(&self, work_id: Option<Uuid>) -> Result<Vec<ArtifactRefRow>> {
        Ok(sqlx::query_as(
            "SELECT id, kind, uri, note, created_by, work_id, attempt_id, digest, source_commit, \
                    runtime_generation, label, state, created_at, superseded_at \
             FROM artifact_refs WHERE ($1::uuid IS NULL OR work_id = $1) \
             ORDER BY created_at, id",
        )
        .bind(work_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn add_work_gate(&self, gate: NewWorkGate<'_>) -> Result<Uuid> {
        if gate.command.is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "a Work gate needs an argv command".into(),
            ));
        }
        if !valid_company_cwd(gate.cwd) || gate.command.iter().any(|part| part.contains('\0')) {
            return Err(OrgIntelError::InvalidWork(
                "a Work gate needs NUL-free argv and an absolute cwd under /company".into(),
            ));
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        // Serialise appends for one Work node so two actors cannot claim the
        // same next pipeline position.
        sqlx::query("SELECT id FROM work WHERE id=$1 FOR UPDATE")
            .bind(gate.work_id)
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO work_gates \
             (id, work_id, name, cwd, command, created_by, sequence_no) \
             SELECT $1,$2,$3,$4,$5,$6,COALESCE(MAX(sequence_no), -1) + 1 \
             FROM work_gates WHERE work_id=$2",
        )
        .bind(id)
        .bind(gate.work_id)
        .bind(gate.name)
        .bind(gate.cwd)
        .bind(
            serde_json::to_value(gate.command)
                .map_err(|error| OrgIntelError::Db(sqlx::Error::Protocol(error.to_string())))?,
        )
        .bind(gate.created_by)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    pub async fn list_work_gates(&self, work_id: Uuid) -> Result<Vec<WorkGateRow>> {
        Ok(sqlx::query_as(
            "SELECT id, work_id, name, cwd, command, created_by, sequence_no, created_at, \
                    retired_at, retired_by, retired_reason \
             FROM work_gates WHERE work_id = $1 AND retired_at IS NULL ORDER BY sequence_no",
        )
        .bind(work_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Retire a mistaken gate without rewriting the evidence of earlier runs.
    /// Work remains blocked until its accountable coordinator declares any
    /// replacement gate and explicitly resumes it.
    pub async fn retire_work_gate(
        &self,
        gate_id: Uuid,
        retired_by: &str,
        reason: &str,
    ) -> Result<bool> {
        if retired_by.trim().is_empty() || reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "retiring a Work gate needs an actor and evidence-backed reason".into(),
            ));
        }
        let result = sqlx::query(
            "UPDATE work_gates SET retired_at=now(), retired_by=$2, retired_reason=$3 \
             WHERE id=$1 AND retired_at IS NULL",
        )
        .bind(gate_id)
        .bind(retired_by)
        .bind(reason)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM work_gates WHERE id=$1)")
                    .bind(gate_id)
                    .fetch_one(&self.pool)
                    .await?;
            if !exists {
                return Err(OrgIntelError::InvalidWork(
                    "Work gate does not exist".into(),
                ));
            }
        }
        Ok(result.rows_affected() == 1)
    }

    /// One repeatable-read projection for CLI and SPA. It is deliberately a
    /// read model over ordinary OrgIntel rows, never a second writer.
    pub async fn work_graph_snapshot(&self) -> Result<WorkGraphSnapshot> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await?;
        let work = sqlx::query_as(
            "SELECT id, goal_id, owner_id, title, outcome, status, resolution, priority, \
             expected_artifact, owner_review_required, repo, base_ref, integration_branch, worktree, revision, \
             attempt_limit, created_at, updated_at FROM work ORDER BY priority DESC, created_at",
        )
        .fetch_all(&mut *tx)
        .await?;
        let edges = sqlx::query_as(
            "SELECT from_work_id, to_work_id, kind, created_at FROM work_edges \
             ORDER BY created_at, from_work_id, to_work_id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let attempts = sqlx::query_as(
            "SELECT id, work_id, revision, attempt_no, actor_id, session_id, state, trigger, \
                    input_fingerprint, feedback_cursor, model, started_at, finished_at, summary \
             FROM work_attempts ORDER BY started_at, id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let attempt_inputs = sqlx::query_as(
            "SELECT attempt_id, artifact_ref_id FROM work_attempt_inputs \
             ORDER BY attempt_id, artifact_ref_id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let attempt_feedback = sqlx::query_as(
            "SELECT attempt_id, message_id FROM work_attempt_feedback \
             ORDER BY attempt_id, message_id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let artifacts = sqlx::query_as(
            "SELECT id, kind, uri, note, created_by, work_id, attempt_id, digest, source_commit, \
                    runtime_generation, label, state, created_at, superseded_at \
             FROM artifact_refs WHERE work_id IS NOT NULL ORDER BY created_at, id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let gates = sqlx::query_as(
            "SELECT id, work_id, name, cwd, command, created_by, sequence_no, created_at, \
                    retired_at, retired_by, retired_reason \
             FROM work_gates ORDER BY work_id, sequence_no",
        )
        .fetch_all(&mut *tx)
        .await?;
        let gate_runs = sqlx::query_as(
            "SELECT id, gate_id, attempt_id, exit_code, output_digest, output_excerpt, \
                    passed, ran_at FROM work_gate_runs ORDER BY ran_at, id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let handoffs = sqlx::query_as(
            "SELECT id, work_id, attempt_id, requested_by, category, requested_action, \
                    prepared_state, resume_condition, state, resolution, assigned_to, \
                    escalated_from, escalated_at, owner_brief, briefed_by, briefed_at, \
                    brief_source_fingerprint, delivered_at, created_at, resolved_at \
             FROM owner_handoffs ORDER BY created_at, id",
        )
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(WorkGraphSnapshot {
            work,
            edges,
            attempts,
            attempt_inputs,
            attempt_feedback,
            artifacts,
            gates,
            gate_runs,
            handoffs,
        })
    }

    pub async fn record_gate_run(&self, run: NewGateRun<'_>) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let valid: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work_gates g JOIN work_attempts a \
             ON a.work_id=g.work_id WHERE g.id=$1 AND a.id=$2)",
        )
        .bind(run.gate_id)
        .bind(run.attempt_id)
        .fetch_one(&self.pool)
        .await?;
        if !valid {
            return Err(OrgIntelError::InvalidWork(
                "gate and Attempt must belong to the same Work".into(),
            ));
        }
        sqlx::query(
            "INSERT INTO work_gate_runs \
             (id, gate_id, attempt_id, exit_code, output_digest, output_excerpt, passed) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) \
             ON CONFLICT (gate_id, attempt_id) DO UPDATE SET \
               exit_code=EXCLUDED.exit_code, output_digest=EXCLUDED.output_digest, \
               output_excerpt=EXCLUDED.output_excerpt, passed=EXCLUDED.passed, ran_at=now()",
        )
        .bind(id)
        .bind(run.gate_id)
        .bind(run.attempt_id)
        .bind(run.exit_code)
        .bind(run.output_digest)
        .bind(run.output_excerpt)
        .bind(run.passed)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn gates_passed(&self, work_id: Uuid, attempt_id: Uuid) -> Result<bool> {
        sqlx::query_scalar(
            "SELECT NOT EXISTS (\
               SELECT 1 FROM work_gates g \
               LEFT JOIN work_gate_runs r ON r.gate_id=g.id AND r.attempt_id=$2 \
               WHERE g.work_id=$1 AND g.retired_at IS NULL \
                 AND COALESCE(r.passed, false)=false\
             )",
        )
        .bind(work_id)
        .bind(attempt_id)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }
}
