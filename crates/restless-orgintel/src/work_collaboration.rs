//! Small-company Work audiences. This module decides who may observe Work;
//! it never grants Authority, Runtime control or mutation permission.

use super::*;
use sha2::Sha256;

fn scope_digest(input: &SetWorkCollaborationScope<'_>) -> String {
    let mut digest = Sha256::new();
    for part in [
        "restless.work-collaboration-scope.v1".to_string(),
        input.work_id.to_string(),
        input.actor_id.trim().to_string(),
        input.expected_revision.to_string(),
        match input.visibility {
            WorkCollaborationVisibility::Company => "company".to_string(),
            WorkCollaborationVisibility::Room => "room".to_string(),
        },
        input.room_id.map(|id| id.to_string()).unwrap_or_default(),
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

impl OrgIntel {
    pub async fn work_collaboration_scope(
        &self,
        work_id: Uuid,
    ) -> Result<Option<WorkCollaborationScopeRow>> {
        Ok(sqlx::query_as(
            "SELECT id AS work_id, collaboration_visibility AS visibility, \
                    collaboration_room_id AS room_id, \
                    collaboration_revision AS revision \
             FROM work WHERE id=$1",
        )
        .bind(work_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Idempotently change one Work audience. Authentication membership and
    /// organisational authority are checked by the caller; this transaction
    /// proves only that the acting human and target Room remain active.
    pub async fn set_work_collaboration_scope(
        &self,
        input: SetWorkCollaborationScope<'_>,
    ) -> Result<WorkCollaborationScopeResult> {
        let actor_id = input.actor_id.trim();
        if input.command_id.is_nil() || actor_id.is_empty() || input.expected_revision < 1 {
            return Err(OrgIntelError::InvalidWork(
                "a Work audience change needs a command id, active Actor and positive expected collaboration revision"
                    .into(),
            ));
        }
        match (input.visibility, input.room_id) {
            (WorkCollaborationVisibility::Company, None)
            | (WorkCollaborationVisibility::Room, Some(_)) => {}
            _ => {
                return Err(OrgIntelError::InvalidWork(
                    "company Work has no Room; Room-visible Work needs exactly one Room".into(),
                ));
            }
        }

        let payload_sha256 = scope_digest(&input);
        let mut tx = self.pool.begin().await?;
        let current = sqlx::query_as::<_, (WorkCollaborationVisibility, Option<Uuid>, i64)>(
            "SELECT collaboration_visibility,collaboration_room_id,collaboration_revision \
             FROM work WHERE id=$1 FOR UPDATE",
        )
        .bind(input.work_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("Work does not exist".into()))?;

        if let Some((work_id, committed_digest, visibility, room_id, revision)) =
            sqlx::query_as::<_, (Uuid, String, WorkCollaborationVisibility, Option<Uuid>, i64)>(
                "SELECT work_id,payload_sha256,resulting_visibility,resulting_room_id,\
                        resulting_collaboration_revision \
                 FROM work_collaboration_scope_commands \
                 WHERE actor_id=$1 AND command_id=$2",
            )
            .bind(actor_id)
            .bind(input.command_id)
            .fetch_optional(&mut *tx)
            .await?
        {
            if work_id != input.work_id || committed_digest != payload_sha256 {
                return Err(OrgIntelError::RoomCommandConflict(
                    "the Work audience command id already names different intent".into(),
                ));
            }
            tx.commit().await?;
            return Ok(WorkCollaborationScopeResult {
                scope: WorkCollaborationScopeRow {
                    work_id,
                    visibility,
                    room_id,
                    revision,
                },
                created: false,
            });
        }

        let actor_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if !actor_active {
            return Err(OrgIntelError::InvalidWork(
                "a Work audience change needs an active Actor".into(),
            ));
        }
        if current.2 != input.expected_revision {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work audience changed from revision {} to {}",
                input.expected_revision, current.2
            )));
        }

        if let Some(room_id) = input.room_id {
            let target_visible: bool = sqlx::query_scalar(
                "SELECT EXISTS(\
                   SELECT 1 FROM rooms room \
                   JOIN room_participants participant ON participant.room_id=room.id \
                   WHERE room.id=$1 AND room.archived_at IS NULL \
                     AND participant.actor_id=$2 AND participant.left_at IS NULL\
                 )",
            )
            .bind(room_id)
            .bind(actor_id)
            .fetch_one(&mut *tx)
            .await?;
            if !target_visible {
                return Err(OrgIntelError::RoomAccessDenied(
                    "the acting Actor must be an active participant of the Work audience Room"
                        .into(),
                ));
            }
        }

        let changed = current.0 != input.visibility || current.1 != input.room_id;
        let revision = if changed { current.2 + 1 } else { current.2 };
        if changed {
            sqlx::query(
                "UPDATE work SET collaboration_visibility=$2,collaboration_room_id=$3,\
                                 collaboration_revision=$4 \
                 WHERE id=$1",
            )
            .bind(input.work_id)
            .bind(input.visibility)
            .bind(input.room_id)
            .bind(revision)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO events (kind,actor_id,body) \
                 VALUES ('work_collaboration_scope_changed.v1',$1,$2)",
            )
            .bind(actor_id)
            .bind(serde_json::json!({
                "work_id": input.work_id,
                "visibility": input.visibility,
                "room_id": input.room_id,
                "collaboration_revision": revision,
            }))
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "INSERT INTO work_collaboration_scope_commands \
             (actor_id,command_id,work_id,payload_sha256,resulting_visibility,\
              resulting_room_id,resulting_collaboration_revision) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(actor_id)
        .bind(input.command_id)
        .bind(input.work_id)
        .bind(&payload_sha256)
        .bind(input.visibility)
        .bind(input.room_id)
        .bind(revision)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(WorkCollaborationScopeResult {
            scope: WorkCollaborationScopeRow {
                work_id: input.work_id,
                visibility: input.visibility,
                room_id: input.room_id,
                revision,
            },
            created: true,
        })
    }

    pub async fn work_is_visible_to_actor(&self, work_id: Uuid, actor_id: &str) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(\
               SELECT 1 FROM work item \
               JOIN actors actor ON actor.id=$2 AND actor.retired_at IS NULL \
               WHERE item.id=$1 AND (\
                 item.collaboration_visibility='company' \
                 OR EXISTS(\
                   SELECT 1 FROM rooms room \
                   JOIN room_participants participant ON participant.room_id=room.id \
                   WHERE room.id=item.collaboration_room_id \
                     AND room.archived_at IS NULL \
                     AND participant.actor_id=actor.id \
                     AND participant.left_at IS NULL\
                 )\
               )\
             )",
        )
        .bind(work_id)
        .bind(actor_id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// A Room message may reference company-visible Work or Work explicitly
    /// scoped to that same Room, but never a different Room's Work.
    pub async fn work_is_visible_in_room_to_actor(
        &self,
        work_id: Uuid,
        room_id: Uuid,
        actor_id: &str,
    ) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(\
               SELECT 1 FROM work item \
               JOIN actors actor ON actor.id=$3 AND actor.retired_at IS NULL \
               JOIN rooms room ON room.id=$2 AND room.archived_at IS NULL \
               JOIN room_participants participant \
                 ON participant.room_id=room.id \
                AND participant.actor_id=actor.id \
                AND participant.left_at IS NULL \
               WHERE item.id=$1 AND (\
                 item.collaboration_visibility='company' \
                 OR (item.collaboration_visibility='room' \
                     AND item.collaboration_room_id=room.id)\
               )\
             )",
        )
        .bind(work_id)
        .bind(room_id)
        .bind(actor_id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// One repeatable-read graph containing only Work visible to this active
    /// Actor. Every dependent row is filtered in SQL to the same id set.
    pub async fn collaborator_work_graph_snapshot(
        &self,
        actor_id: &str,
    ) -> Result<WorkGraphSnapshot> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await?;
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if !active {
            return Err(OrgIntelError::RoomAccessDenied(
                "a shared Work projection needs an active Actor".into(),
            ));
        }
        let visible: Vec<Uuid> = sqlx::query_scalar(
            "SELECT item.id FROM work item \
             WHERE item.collaboration_visibility='company' \
                OR EXISTS(\
                  SELECT 1 FROM rooms room \
                  JOIN room_participants participant ON participant.room_id=room.id \
                  WHERE room.id=item.collaboration_room_id \
                    AND room.archived_at IS NULL \
                    AND participant.actor_id=$1 \
                    AND participant.left_at IS NULL\
                ) \
             ORDER BY item.id",
        )
        .bind(actor_id)
        .fetch_all(&mut *tx)
        .await?;

        let work = sqlx::query_as(
            "SELECT id,goal_id,owner_id,title,outcome,status,resolution,priority,\
                    expected_artifact,owner_review_required,producing_topology,commissioned_by,\
                    repo,base_ref,integration_branch,worktree,revision,attempt_limit,\
                    created_at,updated_at \
             FROM work WHERE id=ANY($1) ORDER BY priority DESC,created_at",
        )
        .bind(&visible)
        .fetch_all(&mut *tx)
        .await?;
        let edges = sqlx::query_as(
            "SELECT from_work_id,to_work_id,kind,created_at FROM work_edges \
             WHERE from_work_id=ANY($1) AND to_work_id=ANY($1) \
             ORDER BY created_at,from_work_id,to_work_id",
        )
        .bind(&visible)
        .fetch_all(&mut *tx)
        .await?;
        let attempts = sqlx::query_as(
            "SELECT id,work_id,revision,attempt_no,actor_id,session_id,state,trigger,\
                    input_fingerprint,feedback_cursor,requested_source_ref,source_commit,\
                    source_tree,terminal_source_commit,terminal_source_tree,\
                    terminal_status_digest,terminal_dirty_entries,terminal_observed_at,\
                    gate_set_digest,environment_fingerprint,materialized_at,\
                    interrupt_requested_at,interrupt_requested_by,interrupt_reason,\
                    feedback_checkpoint_cursor,model,harness,harness_build,harness_transport,\
                    harness_capabilities,started_at,finished_at,summary \
             FROM work_attempts WHERE work_id=ANY($1) ORDER BY started_at,id",
        )
        .bind(&visible)
        .fetch_all(&mut *tx)
        .await?;
        let attempt_inputs = sqlx::query_as(
            "SELECT input.attempt_id,input.artifact_ref_id FROM work_attempt_inputs input \
             JOIN work_attempts attempt ON attempt.id=input.attempt_id \
             WHERE attempt.work_id=ANY($1) ORDER BY input.attempt_id,input.artifact_ref_id",
        )
        .bind(&visible)
        .fetch_all(&mut *tx)
        .await?;
        let attempt_feedback = sqlx::query_as(
            "SELECT feedback.attempt_id,feedback.message_id FROM work_attempt_feedback feedback \
             JOIN work_attempts attempt ON attempt.id=feedback.attempt_id \
             WHERE attempt.work_id=ANY($1) ORDER BY feedback.attempt_id,feedback.message_id",
        )
        .bind(&visible)
        .fetch_all(&mut *tx)
        .await?;
        let artifacts = sqlx::query_as(
            "SELECT id,kind,uri,note,created_by,work_id,attempt_id,digest,source_commit,\
                    runtime_generation,label,state,created_at,superseded_at \
             FROM artifact_refs WHERE work_id=ANY($1) ORDER BY created_at,id",
        )
        .bind(&visible)
        .fetch_all(&mut *tx)
        .await?;
        let gates = sqlx::query_as(
            "SELECT id,work_id,name,cwd,command,created_by,sequence_no,stage,timeout_seconds,\
                    resources,created_at,retired_at,retired_by,retired_reason \
             FROM work_gates WHERE work_id=ANY($1) ORDER BY work_id,sequence_no",
        )
        .bind(&visible)
        .fetch_all(&mut *tx)
        .await?;
        let gate_runs = sqlx::query_as(
            "SELECT run.id,run.gate_id,run.attempt_id,run.exit_code,run.output_digest,\
                    run.output_excerpt,run.passed,run.candidate_tree,run.definition_digest,\
                    run.toolchain_fingerprint,run.status,run.duration_ms,run.cache_source_run_id,\
                    run.leaked_processes,run.ran_at \
             FROM work_gate_runs run \
             JOIN work_gates gate ON gate.id=run.gate_id \
             WHERE gate.work_id=ANY($1) ORDER BY run.ran_at,run.id",
        )
        .bind(&visible)
        .fetch_all(&mut *tx)
        .await?;
        let handoffs = sqlx::query_as(
            "SELECT id,work_id,attempt_id,requested_by,category,requested_action,prepared_state,\
                    resume_condition,state,resolution,assigned_to,escalated_from,escalated_at,\
                    owner_brief,briefed_by,briefed_at,brief_source_fingerprint,delivered_at,\
                    created_at,resolved_at \
             FROM owner_handoffs WHERE work_id=ANY($1) AND assigned_to=$2 \
             ORDER BY created_at,id",
        )
        .bind(&visible)
        .bind(actor_id)
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
}
