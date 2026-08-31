//! Exact Attempt execution, resource leases and recoverable custody.
//!
//! These are mechanical facts around existing Work/Attempt/Git concepts. They
//! intentionally do not create another workflow or artifact store.

use super::*;

fn exact_oid(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl OrgIntel {
    /// Freeze the Runtime's terminal workspace observation before the Attempt
    /// changes state. A later explicit `resume` may use a clean committed
    /// terminal candidate, but never a dirty or merely mutable worktree.
    pub async fn bind_attempt_terminal_coordinates(
        &self,
        attempt_id: Uuid,
        source_commit: Option<&str>,
        source_tree: Option<&str>,
        status_digest: Option<&str>,
        dirty_entries: usize,
    ) -> Result<()> {
        if source_commit.is_some_and(|value| !exact_oid(value))
            || source_tree.is_some_and(|value| !exact_oid(value))
            || source_commit.is_some() != source_tree.is_some()
        {
            return Err(OrgIntelError::InvalidWork(
                "terminal Attempt coordinates need paired full Git object ids".into(),
            ));
        }
        let dirty_entries = i32::try_from(dirty_entries).map_err(|_| {
            OrgIntelError::InvalidWork("terminal dirty-entry count exceeds i32".into())
        })?;
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT state, terminal_source_commit, terminal_source_tree, \
                    terminal_status_digest, terminal_dirty_entries, terminal_observed_at \
             FROM work_attempts WHERE id=$1 FOR UPDATE",
        )
        .bind(attempt_id)
        .fetch_one(&mut *tx)
        .await?;
        if row
            .get::<Option<DateTime<Utc>>, _>("terminal_observed_at")
            .is_some()
        {
            let matches = row
                .get::<Option<String>, _>("terminal_source_commit")
                .as_deref()
                == source_commit
                && row
                    .get::<Option<String>, _>("terminal_source_tree")
                    .as_deref()
                    == source_tree
                && row
                    .get::<Option<String>, _>("terminal_status_digest")
                    .as_deref()
                    == status_digest
                && row.get::<Option<i32>, _>("terminal_dirty_entries") == Some(dirty_entries);
            if !matches {
                return Err(OrgIntelError::InvalidWork(format!(
                    "Attempt {attempt_id} terminal coordinates are already frozen"
                )));
            }
            tx.commit().await?;
            return Ok(());
        }
        let state: WorkAttemptState = row.get("state");
        if state != WorkAttemptState::Running {
            return Err(OrgIntelError::InvalidWork(format!(
                "Attempt {attempt_id} is not running"
            )));
        }
        sqlx::query(
            "UPDATE work_attempts SET terminal_source_commit=$2, terminal_source_tree=$3, \
                    terminal_status_digest=$4, terminal_dirty_entries=$5, \
                    terminal_observed_at=now() WHERE id=$1",
        )
        .bind(attempt_id)
        .bind(source_commit)
        .bind(source_tree)
        .bind(status_digest)
        .bind(dirty_entries)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Bind the workspace the Runtime actually materialised. Set-once
    /// semantics make a branch movement or stale worktree a typed pre-model
    /// failure instead of a prompt ambiguity.
    pub async fn bind_attempt_execution_coordinates(
        &self,
        attempt_id: Uuid,
        requested_source_ref: Option<&str>,
        source_commit: Option<&str>,
        source_tree: Option<&str>,
        environment_fingerprint: &str,
    ) -> Result<()> {
        if source_commit.is_some_and(|value| !exact_oid(value))
            || source_tree.is_some_and(|value| !exact_oid(value))
            || environment_fingerprint.trim().is_empty()
        {
            return Err(OrgIntelError::InvalidWork(
                "Attempt coordinates need full Git object ids and an environment fingerprint"
                    .into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT state, requested_source_ref, source_commit, source_tree, environment_fingerprint, materialized_at \
             FROM work_attempts WHERE id=$1 FOR UPDATE",
        )
        .bind(attempt_id)
        .fetch_one(&mut *tx)
        .await?;
        let state: WorkAttemptState = row.get("state");
        if state != WorkAttemptState::Running {
            return Err(OrgIntelError::InvalidWork(format!(
                "Attempt {attempt_id} is not running"
            )));
        }
        let materialized: Option<DateTime<Utc>> = row.get("materialized_at");
        if materialized.is_some() {
            let matches = row
                .get::<Option<String>, _>("requested_source_ref")
                .as_deref()
                == requested_source_ref
                && row.get::<Option<String>, _>("source_commit").as_deref() == source_commit
                && row.get::<Option<String>, _>("source_tree").as_deref() == source_tree
                && row.get::<String, _>("environment_fingerprint") == environment_fingerprint;
            if !matches {
                return Err(OrgIntelError::InvalidWork(format!(
                    "Attempt {attempt_id} materialisation differs from its frozen execution coordinates"
                )));
            }
            tx.commit().await?;
            return Ok(());
        }
        sqlx::query(
            "UPDATE work_attempts SET requested_source_ref=$2, source_commit=$3, source_tree=$4, \
                    environment_fingerprint=$5, materialized_at=now() WHERE id=$1",
        )
        .bind(attempt_id)
        .bind(requested_source_ref)
        .bind(source_commit)
        .bind(source_tree)
        .bind(environment_fingerprint)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn request_attempt_interrupt(
        &self,
        work_id: Uuid,
        requested_by: &str,
        reason: &str,
    ) -> Result<Uuid> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "an urgent interrupt needs a concrete reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT attempt.id, work.owner_id, team.lead_actor_id \
             FROM work JOIN work_attempts attempt ON attempt.work_id=work.id AND attempt.state='running' \
             JOIN actors owner ON owner.id=work.owner_id \
             LEFT JOIN teams team ON team.id=owner.team_id AND team.disbanded_at IS NULL \
             WHERE work.id=$1 FOR UPDATE OF attempt",
        )
        .bind(work_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("Work has no running Attempt".into()))?;
        let owner_id: String = row.get("owner_id");
        let lead_id: Option<String> = row.get("lead_actor_id");
        if requested_by != "owner"
            && requested_by != "exec"
            && requested_by != owner_id
            && lead_id.as_deref() != Some(requested_by)
            && requested_by != "daemon"
        {
            return Err(OrgIntelError::InvalidWork(format!(
                "{requested_by:?} cannot interrupt Work {work_id}"
            )));
        }
        let attempt_id: Uuid = row.get("id");
        sqlx::query(
            "UPDATE work_attempts SET interrupt_requested_at=now(), interrupt_requested_by=$2, \
                    interrupt_reason=$3 WHERE id=$1 AND interrupt_requested_at IS NULL",
        )
        .bind(attempt_id)
        .bind(requested_by)
        .bind(reason.trim())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('attempt_interrupt_requested',$1,$2)",
        )
        .bind(requested_by)
        .bind(serde_json::json!({"work_id": work_id, "attempt_id": attempt_id, "reason": reason.trim()}))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(attempt_id)
    }

    /// Consume direct Work feedback at a safe model-turn boundary. Messages
    /// are attached once and remain durable even if the process dies after
    /// delivery.
    pub async fn checkpoint_attempt_feedback(&self, attempt_id: Uuid) -> Result<Vec<MessageRow>> {
        let mut tx = self.pool.begin().await?;
        let attempt = sqlx::query(
            "SELECT work_id, actor_id, state, feedback_checkpoint_cursor \
             FROM work_attempts WHERE id=$1 FOR UPDATE",
        )
        .bind(attempt_id)
        .fetch_one(&mut *tx)
        .await?;
        if attempt.get::<WorkAttemptState, _>("state") != WorkAttemptState::Running {
            tx.commit().await?;
            return Ok(Vec::new());
        }
        let work_id: Uuid = attempt.get("work_id");
        let actor_id: String = attempt.get("actor_id");
        let cursor: i64 = attempt.get("feedback_checkpoint_cursor");
        let messages = sqlx::query_as::<_, MessageRow>(
            "SELECT message.id, message.from_actor, message.to_actor, message.body, \
                    message.created_at, message.read_at \
             FROM work_feedback feedback JOIN messages message ON message.id=feedback.message_id \
             WHERE feedback.work_id=$1 AND message.to_actor=$2 AND message.id>$3 \
               AND NOT EXISTS (SELECT 1 FROM work_attempt_feedback delivered \
                               WHERE delivered.attempt_id=$4 AND delivered.message_id=message.id) \
             ORDER BY message.id",
        )
        .bind(work_id)
        .bind(&actor_id)
        .bind(cursor)
        .bind(attempt_id)
        .fetch_all(&mut *tx)
        .await?;
        for message in &messages {
            sqlx::query(
                "INSERT INTO work_attempt_feedback (attempt_id,message_id) VALUES ($1,$2) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(attempt_id)
            .bind(message.id)
            .execute(&mut *tx)
            .await?;
            sqlx::query("UPDATE messages SET read_at=COALESCE(read_at,now()) WHERE id=$1")
                .bind(message.id)
                .execute(&mut *tx)
                .await?;
        }
        if let Some(last) = messages.last() {
            sqlx::query(
                "UPDATE work_attempts SET feedback_checkpoint_cursor=GREATEST(feedback_checkpoint_cursor,$2) WHERE id=$1",
            )
            .bind(attempt_id)
            .bind(last.id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(messages)
    }

    pub async fn acquire_runtime_resource(
        &self,
        attempt_id: Uuid,
        gate_id: Option<Uuid>,
        kind: &str,
        value: &str,
        holder_token: &str,
    ) -> Result<Option<RuntimeResourceLeaseRow>> {
        if !matches!(kind, "port" | "display" | "tempdir" | "process_group")
            || value.trim().is_empty()
            || holder_token.trim().is_empty()
        {
            return Err(OrgIntelError::InvalidWork(
                "invalid Runtime resource lease".into(),
            ));
        }
        let id = Uuid::new_v4();
        let inserted = sqlx::query_as::<_, RuntimeResourceLeaseRow>(
            "INSERT INTO runtime_resource_leases \
             (id,attempt_id,gate_id,kind,value,holder_token) VALUES ($1,$2,$3,$4,$5,$6) \
             ON CONFLICT (kind,value) WHERE released_at IS NULL DO NOTHING \
             RETURNING id,attempt_id,gate_id,kind,value,holder_token,acquired_at,released_at,release_reason",
        )
        .bind(id)
        .bind(attempt_id)
        .bind(gate_id)
        .bind(kind)
        .bind(value)
        .bind(holder_token)
        .fetch_optional(&self.pool)
        .await?;
        Ok(inserted)
    }

    pub async fn release_runtime_resource(
        &self,
        lease_id: Uuid,
        holder_token: &str,
        reason: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE runtime_resource_leases SET released_at=now(), release_reason=$3 \
             WHERE id=$1 AND holder_token=$2 AND released_at IS NULL",
        )
        .bind(lease_id)
        .bind(holder_token)
        .bind(reason)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn release_attempt_resources(&self, attempt_id: Uuid, reason: &str) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE runtime_resource_leases SET released_at=now(), release_reason=$2 \
             WHERE attempt_id=$1 AND released_at IS NULL",
        )
        .bind(attempt_id)
        .bind(reason)
        .execute(&self.pool)
        .await?
        .rows_affected())
    }

    pub async fn list_live_runtime_resources(&self) -> Result<Vec<RuntimeResourceLeaseRow>> {
        Ok(sqlx::query_as(
            "SELECT id,attempt_id,gate_id,kind,value,holder_token,acquired_at,released_at,release_reason \
             FROM runtime_resource_leases WHERE released_at IS NULL ORDER BY acquired_at,id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Release leases whose Attempt can no longer own a live Runtime process.
    /// This is safe to repeat after every scheduler restart.
    pub async fn reconcile_runtime_resources(&self) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE runtime_resource_leases lease SET released_at=now(), \
                    release_reason='scheduler reconciliation: Attempt is not running' \
             FROM work_attempts attempt WHERE lease.attempt_id=attempt.id \
               AND lease.released_at IS NULL AND attempt.state <> 'running'",
        )
        .execute(&self.pool)
        .await?
        .rows_affected())
    }

    pub async fn begin_candidate_promotion(
        &self,
        promotion: NewCandidatePromotion<'_>,
    ) -> Result<CandidatePromotionRow> {
        let NewCandidatePromotion {
            work_id,
            attempt_id,
            repo,
            integration_branch: branch,
            source_commit,
            source_tree,
            manifest,
        } = promotion;
        if !exact_oid(source_commit) || !exact_oid(source_tree) {
            return Err(OrgIntelError::InvalidWork(
                "promotion needs exact Git objects".into(),
            ));
        }
        let id = Uuid::new_v4();
        let row: CandidatePromotionRow = sqlx::query_as(
            "INSERT INTO candidate_promotions \
             (id,work_id,attempt_id,repo,integration_branch,source_commit,source_tree,manifest) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
             ON CONFLICT (attempt_id) DO UPDATE SET attempt_id=EXCLUDED.attempt_id \
             RETURNING id,work_id,attempt_id,repo,integration_branch,source_commit,source_tree, \
                       manifest,state,failure,created_at,completed_at",
        )
        .bind(id)
        .bind(work_id)
        .bind(attempt_id)
        .bind(repo)
        .bind(branch)
        .bind(source_commit)
        .bind(source_tree)
        .bind(manifest)
        .fetch_one(&self.pool)
        .await?;
        if row.work_id != work_id
            || row.repo != repo
            || row.integration_branch != branch
            || row.source_commit != source_commit
            || row.source_tree != source_tree
            || row.manifest != *manifest
        {
            return Err(OrgIntelError::InvalidWork(
                "promotion journal is immutable and already names different evidence".into(),
            ));
        }
        Ok(row)
    }

    pub async fn finish_candidate_promotion(
        &self,
        promotion_id: Uuid,
        success: bool,
        failure: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE candidate_promotions SET state=CASE WHEN $2 THEN 'completed' ELSE 'failed' END, \
                    failure=$3, completed_at=now() WHERE id=$1 AND state='pending'",
        )
        .bind(promotion_id)
        .bind(success)
        .bind(failure)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn pending_candidate_promotions(&self) -> Result<Vec<CandidatePromotionRow>> {
        Ok(sqlx::query_as(
            "SELECT id,work_id,attempt_id,repo,integration_branch,source_commit,source_tree, \
                    manifest,state,failure,created_at,completed_at \
             FROM candidate_promotions WHERE state='pending' ORDER BY created_at,id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn record_immutable_review_target(
        &self,
        target: NewImmutableReviewTarget<'_>,
    ) -> Result<ImmutableReviewTargetRow> {
        let NewImmutableReviewTarget {
            work_id,
            attempt_id,
            content_digest: digest,
            uri,
            alias_uri,
            source_commit,
            manifest,
        } = target;
        if digest.trim().is_empty() || uri.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "immutable review target needs digest and URI".into(),
            ));
        }
        let id = Uuid::new_v4();
        let row: ImmutableReviewTargetRow = sqlx::query_as(
            "INSERT INTO immutable_review_targets \
             (id,work_id,attempt_id,content_digest,uri,alias_uri,source_commit,manifest) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
             ON CONFLICT (attempt_id) DO UPDATE SET attempt_id=EXCLUDED.attempt_id \
             RETURNING id,work_id,attempt_id,content_digest,uri,alias_uri,source_commit,manifest,created_at",
        )
        .bind(id)
        .bind(work_id)
        .bind(attempt_id)
        .bind(digest)
        .bind(uri)
        .bind(alias_uri)
        .bind(source_commit)
        .bind(manifest)
        .fetch_one(&self.pool)
        .await?;
        if row.work_id != work_id
            || row.content_digest != digest
            || row.uri != uri
            || row.alias_uri.as_deref() != alias_uri
            || row.source_commit.as_deref() != source_commit
            || row.manifest != *manifest
        {
            return Err(OrgIntelError::InvalidWork(
                "review target is write-once and already names different content".into(),
            ));
        }
        Ok(row)
    }
}
