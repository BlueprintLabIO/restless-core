//! Direct mail, Work feedback, and bounded owner conversation reads.

use super::*;

impl OrgIntel {
    // ---- messages ----

    /// Send a directed message; `to_actor: None` addresses the owner inbox.
    pub async fn send_message(&self, from: &str, to: Option<&str>, body: &str) -> Result<i64> {
        let row = sqlx::query(
            "INSERT INTO messages (from_actor, to_actor, body) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(from)
        .bind(to)
        .bind(body)
        .fetch_one(&self.pool)
        .await?;
        // Directed-mail NOTIFY comes from the database trigger (0002).
        Ok(row.get(0))
    }

    /// Project one Authority-owned external fact into one organisational
    /// message. `source_ref` is the idempotency boundary and remains an
    /// ordinary reference, not a second mailbox or delivery lifecycle.
    #[allow(clippy::too_many_arguments)]
    pub async fn project_external_message_once(
        &self,
        from: &str,
        to: &str,
        body: &str,
        source_ref: &str,
        provider: &str,
        provider_event_id: &str,
        provider_email_id: Option<&str>,
        provider_message_id: Option<&str>,
        provider_thread_id: Option<&str>,
        source_url: Option<&str>,
        metadata: &serde_json::Value,
        work_id: Option<Uuid>,
    ) -> Result<(i64, bool)> {
        if body.trim().is_empty() || source_ref.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "external projection needs bounded context and a stable source reference".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let claimed: Option<String> = sqlx::query_scalar(
            "INSERT INTO external_message_sources \
             (source_ref,provider,provider_event_id,provider_email_id,provider_message_id,provider_thread_id,source_url,metadata) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING RETURNING source_ref",
        )
        .bind(source_ref)
        .bind(provider)
        .bind(provider_event_id)
        .bind(provider_email_id)
        .bind(provider_message_id)
        .bind(provider_thread_id)
        .bind(source_url)
        .bind(metadata)
        .fetch_optional(&mut *tx)
        .await?;
        if claimed.is_none() {
            let existing: Option<i64> = sqlx::query_scalar(
                "SELECT message_id FROM external_message_sources WHERE source_ref=$1",
            )
            .bind(source_ref)
            .fetch_one(&mut *tx)
            .await?;
            let message_id = existing.ok_or_else(|| {
                OrgIntelError::InvalidWork(
                    "external source projection exists without its message".into(),
                )
            })?;
            tx.commit().await?;
            return Ok((message_id, false));
        }
        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor,to_actor,body) VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(from)
        .bind(to)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;
        if let Some(work_id) = work_id {
            sqlx::query(
                "INSERT INTO work_feedback (work_id,message_id,linked_by) VALUES ($1,$2,$3)",
            )
            .bind(work_id)
            .bind(message_id)
            .bind(from)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "UPDATE external_message_sources SET message_id=$2,projected_at=now() WHERE source_ref=$1",
        )
        .bind(source_ref)
        .bind(message_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((message_id, true))
    }

    /// Resolve an authenticated provider thread through factual projections
    /// already attached to Work. Active production routes to its worker;
    /// blocked or review-stage Work routes to the accountable lead. Settled
    /// Work still identifies the lead, but the new message remains free to
    /// commission a new Work unit.
    pub async fn external_thread_route(
        &self,
        provider: &str,
        provider_references: &[String],
    ) -> Result<Option<(String, Option<Uuid>)>> {
        if provider.trim().is_empty() || provider_references.is_empty() {
            return Ok(None);
        }
        let Some(row) = sqlx::query(
            "SELECT work.id AS work_id,work.owner_id,work.status,team.lead_actor_id, \
                    EXISTS(SELECT 1 FROM owner_handoffs handoff \
                           WHERE handoff.work_id=work.id AND handoff.state='pending') AS pending_handoff \
                    ,EXISTS(SELECT 1 FROM work_attempts attempt \
                            WHERE attempt.work_id=work.id AND attempt.state='running') AS running_attempt \
             FROM external_message_sources source \
             JOIN work_feedback feedback ON feedback.message_id=source.message_id \
             JOIN work ON work.id=feedback.work_id \
             JOIN actors actor ON actor.id=work.owner_id AND actor.retired_at IS NULL \
             JOIN teams team ON team.id=actor.team_id AND team.disbanded_at IS NULL \
             WHERE source.provider=$1 \
               AND (source.provider_message_id=ANY($2) OR source.provider_thread_id=ANY($2)) \
             ORDER BY source.projected_at DESC,source.message_id DESC LIMIT 1",
        )
        .bind(provider)
        .bind(provider_references)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };
        let work_id: Uuid = row.get("work_id");
        let owner: String = row.get("owner_id");
        let status: WorkStatus = row.get("status");
        let lead: String = row.get("lead_actor_id");
        let pending_handoff: bool = row.get("pending_handoff");
        let running_attempt: bool = row.get("running_attempt");
        Ok(Some(match status {
            WorkStatus::Proposed | WorkStatus::Active if !pending_handoff && !running_attempt => {
                (owner, Some(work_id))
            }
            WorkStatus::Blocked | WorkStatus::Proposed | WorkStatus::Active => {
                (lead, Some(work_id))
            }
            WorkStatus::Completed | WorkStatus::Abandoned => (lead, None),
        }))
    }

    /// Send ordinary owner conversation, optionally moving the actor's one
    /// working-context cursor to the end of the existing transcript first.
    /// The cursor changes what a future model wake carries, never what the
    /// owner can read; no message or actor history is deleted.
    pub async fn send_owner_conversation_message(
        &self,
        actor: &str,
        body: &str,
        new_focus: bool,
    ) -> Result<(i64, ConversationFocusRow)> {
        let mut tx = self.pool.begin().await?;
        if new_focus {
            let after_message_id: i64 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(id), 0) FROM messages \
                 WHERE (from_actor='owner' AND to_actor=$1) \
                    OR (from_actor=$1 AND to_actor IS NULL)",
            )
            .bind(actor)
            .fetch_one(&mut *tx)
            .await?;
            let changed = sqlx::query(
                "UPDATE actors SET conversation_focus_after_message_id=$2, \
                         conversation_focus_started_at=now() \
                 WHERE id=$1 AND retired_at IS NULL",
            )
            .bind(actor)
            .bind(after_message_id)
            .execute(&mut *tx)
            .await?;
            if changed.rows_affected() != 1 {
                return Err(OrgIntelError::InvalidWork(format!(
                    "active conversation actor {actor:?} does not exist"
                )));
            }
        }
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor, to_actor, body) \
             VALUES ('owner',$1,$2) RETURNING id",
        )
        .bind(actor)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;
        let focus = sqlx::query_as(
            "SELECT conversation_focus_after_message_id AS after_message_id, \
                    conversation_focus_started_at AS started_at \
             FROM actors WHERE id=$1 AND retired_at IS NULL",
        )
        .bind(actor)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!(
                "active conversation actor {actor:?} does not exist"
            ))
        })?;
        tx.commit().await?;
        Ok((id, focus))
    }

    /// Send ordinary free-form conversation and link it to the Work it changes.
    /// The link is kickoff context, not a conversation lifecycle.
    pub async fn send_work_message(
        &self,
        from: &str,
        to: &str,
        work_id: Uuid,
        body: &str,
    ) -> Result<i64> {
        if body.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "Work feedback message cannot be empty".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let owner: String = sqlx::query_scalar("SELECT owner_id FROM work WHERE id=$1")
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
        let accountable_lead: Option<String> = sqlx::query_scalar(
            "SELECT t.lead_actor_id FROM actors a JOIN teams t ON t.id=a.team_id \
             WHERE a.id=$1 AND t.disbanded_at IS NULL",
        )
        .bind(&owner)
        .fetch_optional(&mut *tx)
        .await?;
        if owner != to && accountable_lead.as_deref() != Some(to) {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work {work_id} belongs to {owner:?}; its accountable lead is {:?}, not message recipient {to:?}",
                accountable_lead
            )));
        }
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor, to_actor, body) VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(from)
        .bind(to)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO work_feedback (work_id, message_id, linked_by) VALUES ($1,$2,$3)")
            .bind(work_id)
            .bind(id)
            .bind(from)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Whether a directed message is already deterministic input to an active
    /// Work revision. The scheduler uses this after the transaction commits:
    /// such a message must start (or await) the Work Attempt, not race it with
    /// a second free-form actor session.
    pub async fn message_is_work_attempt_input(&self, message_id: i64) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(\
               SELECT 1 FROM work_feedback feedback \
               JOIN work ON work.id=feedback.work_id \
               JOIN messages message ON message.id=feedback.message_id \
               WHERE feedback.message_id=$1 \
                 AND message.to_actor=work.owner_id \
                 AND work.status IN ('proposed','active') \
                 AND NOT EXISTS (\
                   SELECT 1 FROM owner_handoffs handoff \
                   WHERE handoff.work_id=work.id AND handoff.state='pending'\
                 )\
             )",
        )
        .bind(message_id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Work context attached to a message, when there is one. Conversation
    /// streaming uses this only to persist the final reply beside the owner's
    /// triggering message; `work_feedback` remains the one canonical link.
    pub async fn message_work_id(&self, message_id: i64) -> Result<Option<Uuid>> {
        Ok(
            sqlx::query_scalar("SELECT work_id FROM work_feedback WHERE message_id=$1")
                .bind(message_id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    /// Bounded external source records already linked to one Work. This is a
    /// read projection over `external_message_sources -> messages ->
    /// work_feedback`; it creates no review/source lifecycle and copies no
    /// provider payload into a second store.
    pub async fn work_external_message_sources(
        &self,
        work_id: Uuid,
    ) -> Result<Vec<ExternalMessageSourceRow>> {
        Ok(sqlx::query_as(
            "SELECT source.source_ref, source.message_id, message.from_actor, message.body, \
                    source.provider, source.provider_event_id, source.provider_email_id, \
                    source.provider_message_id, source.provider_thread_id, source.source_url, \
                    source.metadata, source.projected_at \
             FROM external_message_sources source \
             JOIN messages message ON message.id=source.message_id \
             JOIN work_feedback feedback ON feedback.message_id=message.id \
             WHERE feedback.work_id=$1 \
             ORDER BY source.projected_at, source.message_id",
        )
        .bind(work_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Reply from the producing actor or its accountable lead to the human
    /// owner, preserving the same Work-scoped conversation. The lead speaks
    /// for the complete team outcome while Staff retains production
    /// attribution. The owner inbox remains the existing `to_actor = NULL`
    /// convention; no thread entity is introduced.
    pub async fn send_work_message_to_owner(
        &self,
        from: &str,
        work_id: Uuid,
        body: &str,
    ) -> Result<i64> {
        if body.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "Work feedback message cannot be empty".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let owner: String = sqlx::query_scalar("SELECT owner_id FROM work WHERE id=$1")
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
        let accountable_lead: Option<String> = sqlx::query_scalar(
            "SELECT team.lead_actor_id FROM actors actor \
             JOIN teams team ON team.id=actor.team_id AND team.disbanded_at IS NULL \
             WHERE actor.id=$1 AND actor.retired_at IS NULL",
        )
        .bind(&owner)
        .fetch_optional(&mut *tx)
        .await?;
        if owner != from && accountable_lead.as_deref() != Some(from) {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work {work_id} belongs to {owner:?}; its accountable lead is {:?}, not replying actor {from:?}",
                accountable_lead
            )));
        }
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor, to_actor, body) VALUES ($1,NULL,$2) RETURNING id",
        )
        .bind(from)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO work_feedback (work_id, message_id, linked_by) VALUES ($1,$2,$3)")
            .bind(work_id)
            .bind(id)
            .bind(from)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// The owner/actor messages linked to one Work item. This keeps the review
    /// conversation focused without inventing a thread entity.
    pub async fn owner_work_conversation(
        &self,
        actor: &str,
        work_id: Uuid,
        limit: i64,
    ) -> Result<Vec<MessageRow>> {
        let owner: String = sqlx::query_scalar("SELECT owner_id FROM work WHERE id=$1")
            .bind(work_id)
            .fetch_one(&self.pool)
            .await?;
        if owner != actor {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work {work_id} belongs to {owner:?}, not conversation actor {actor:?}"
            )));
        }
        Ok(sqlx::query_as(
            "SELECT id,from_actor,to_actor,body,created_at,read_at FROM (\
               SELECT m.id,m.from_actor,m.to_actor,m.body,m.created_at,m.read_at \
               FROM messages m JOIN work_feedback f ON f.message_id=m.id \
               WHERE f.work_id=$1 AND (\
                 (m.from_actor='owner' AND m.to_actor=$2) OR \
                 (m.from_actor=$2 AND m.to_actor IS NULL)\
               ) ORDER BY m.created_at DESC,m.id DESC LIMIT $3\
             ) recent ORDER BY created_at,id",
        )
        .bind(work_id)
        .bind(actor)
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?)
    }

    /// An actor's unread inbox (`None` = the owner's), oldest first.
    pub async fn inbox(&self, actor: Option<&str>) -> Result<Vec<MessageRow>> {
        Ok(sqlx::query_as(
            "SELECT id, from_actor, to_actor, body, created_at, read_at FROM messages \
             WHERE read_at IS NULL AND to_actor IS NOT DISTINCT FROM $1 ORDER BY id",
        )
        .bind(actor)
        .fetch_all(&self.pool)
        .await?)
    }

    /// How much unread conversation this actor genuinely owes a turn: mail
    /// addressed to it, excluding its own notes to itself and any message that
    /// is already deterministic input to an active Work revision.
    ///
    /// `read_at` is the durable delivery record — it is written only when a
    /// turn that carried the message actually completed — so this is safe to
    /// re-derive on every scan. It replaces comparing the newest message with
    /// the newest wake event, which silenced any message that arrived while an
    /// earlier wake was still running and treated a health-gated wake that
    /// assembled no context at all as an observation (S19-T1).
    pub async fn owed_conversation_count(&self, actor: &str) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT count(*) FROM messages message \
             WHERE message.read_at IS NULL AND message.to_actor=$1 AND message.from_actor<>$1 \
               AND NOT EXISTS (\
                 SELECT 1 FROM work_feedback feedback JOIN work ON work.id=feedback.work_id \
                 WHERE feedback.message_id=message.id \
                   AND message.to_actor=work.owner_id \
                   AND work.status IN ('proposed','active') \
                   AND NOT EXISTS (\
                     SELECT 1 FROM owner_handoffs handoff \
                     WHERE handoff.work_id=work.id AND handoff.state='pending'\
                   )\
               )",
        )
        .bind(actor)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Read one actor's own inbox. If that actor has one live Work Attempt,
    /// any Work-linked message addressed to it is recorded as feedback that
    /// exact Attempt received. The initial input snapshot remains fixed at
    /// claim time; this is the later live-observation path permitted by the
    /// one-process rule, not a second kickoff or message lifecycle.
    pub async fn consume_inbox_for_actor(&self, actor: &str) -> Result<Vec<MessageRow>> {
        let mut tx = self.pool.begin().await?;
        let live_attempt = sqlx::query(
            "SELECT id, work_id FROM work_attempts \
             WHERE actor_id=$1 AND state='running' \
             ORDER BY started_at, id LIMIT 1 FOR UPDATE",
        )
        .bind(actor)
        .fetch_optional(&mut *tx)
        .await?;
        let messages = sqlx::query_as::<_, MessageRow>(
            "SELECT id, from_actor, to_actor, body, created_at, read_at FROM messages \
             WHERE read_at IS NULL AND to_actor=$1 ORDER BY id FOR UPDATE",
        )
        .bind(actor)
        .fetch_all(&mut *tx)
        .await?;
        if let Some(attempt) = live_attempt {
            let attempt_id: Uuid = attempt.get("id");
            let work_id: Uuid = attempt.get("work_id");
            for message in &messages {
                sqlx::query(
                    "INSERT INTO work_attempt_feedback (attempt_id, message_id) \
                     SELECT $1,$2 WHERE EXISTS (\
                       SELECT 1 FROM work_feedback WHERE work_id=$3 AND message_id=$2\
                     ) ON CONFLICT DO NOTHING",
                )
                .bind(attempt_id)
                .bind(message.id)
                .bind(work_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        for message in &messages {
            sqlx::query("UPDATE messages SET read_at=now() WHERE id=$1")
                .bind(message.id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(messages)
    }

    /// Ordinary conversation between the owner and one actor, oldest first.
    /// This is a read over the existing message rows, not a handover/thread
    /// entity. `to_actor = NULL` is the established owner-inbox convention.
    pub async fn owner_conversation(&self, actor: &str, limit: i64) -> Result<Vec<MessageRow>> {
        let limit = limit.clamp(1, 200);
        Ok(sqlx::query_as(
            "SELECT id, from_actor, to_actor, body, created_at, read_at FROM (\
               SELECT id, from_actor, to_actor, body, created_at, read_at FROM messages \
               WHERE (from_actor = 'owner' AND to_actor = $1) \
                  OR (from_actor = $1 AND to_actor IS NULL) \
               ORDER BY id DESC LIMIT $2\
             ) recent ORDER BY id",
        )
        .bind(actor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Current working-context boundary for the owner's conversation with one
    /// actor. A zero cursor with no timestamp is the original uninterrupted
    /// conversation; it is not an unknown state.
    pub async fn owner_conversation_focus(&self, actor: &str) -> Result<ConversationFocusRow> {
        sqlx::query_as(
            "SELECT conversation_focus_after_message_id AS after_message_id, \
                    conversation_focus_started_at AS started_at \
             FROM actors WHERE id=$1 AND retired_at IS NULL",
        )
        .bind(actor)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!(
                "active conversation actor {actor:?} does not exist"
            ))
        })
    }

    /// Consume one unread owner conversation message because the owner
    /// explicitly interrupted it before an answer was recorded. This is a
    /// durable delivery decision, not a second conversation message: the
    /// original directive remains visible in the transcript and the event
    /// records why it will not be retried after a daemon restart.
    pub async fn interrupt_owner_conversation_message(
        &self,
        actor: &str,
        message_id: i64,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let consumed: Option<i64> = sqlx::query_scalar(
            "UPDATE messages SET read_at=now() \
             WHERE id=$1 AND from_actor='owner' AND to_actor=$2 AND read_at IS NULL \
               AND NOT EXISTS (SELECT 1 FROM work_feedback WHERE message_id=messages.id) \
             RETURNING id",
        )
        .bind(message_id)
        .bind(actor)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(consumed) = consumed else {
            return Ok(false);
        };
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ($1,$2,$3)")
            .bind("owner_conversation_interrupted")
            .bind("owner")
            .bind(serde_json::json!({
                "message_id": consumed,
                "actor": actor,
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }

    /// The bounded owner/actor transcript newer than a known focus cursor.
    /// This is input material for a wake, not a second conversation record.
    pub async fn owner_conversation_since(
        &self,
        actor: &str,
        after_message_id: i64,
        limit: i64,
    ) -> Result<Vec<MessageRow>> {
        let limit = limit.clamp(1, 200);
        Ok(sqlx::query_as(
            "SELECT id,from_actor,to_actor,body,created_at,read_at FROM (\
               SELECT id,from_actor,to_actor,body,created_at,read_at FROM messages \
               WHERE id>$2 AND ((from_actor='owner' AND to_actor=$1) \
                            OR (from_actor=$1 AND to_actor IS NULL)) \
               ORDER BY id DESC LIMIT $3\
             ) recent ORDER BY id",
        )
        .bind(actor)
        .bind(after_message_id.max(0))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn mark_read(&self, message_id: i64) -> Result<()> {
        sqlx::query("UPDATE messages SET read_at = now() WHERE id = $1")
            .bind(message_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
