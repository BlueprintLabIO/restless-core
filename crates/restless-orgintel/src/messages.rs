//! Direct mail, Work feedback, and bounded owner conversation reads.

use super::*;

const MAX_CONVERSATION_MESSAGE_BYTES: usize = 64 * 1024;

fn validate_conversation_body(body: &str) -> Result<()> {
    if body.trim().is_empty() || body.len() > MAX_CONVERSATION_MESSAGE_BYTES {
        return Err(OrgIntelError::InvalidWork(format!(
            "a conversation message must contain 1 to {MAX_CONVERSATION_MESSAGE_BYTES} UTF-8 bytes"
        )));
    }
    Ok(())
}

impl OrgIntel {
    // ---- messages ----

    /// Send a directed message; `to_actor: None` addresses the owner inbox.
    pub async fn send_message(&self, from: &str, to: Option<&str>, body: &str) -> Result<i64> {
        validate_conversation_body(body)?;
        let mut tx = self.pool.begin().await?;
        if to.is_none() {
            reject_unfenced_owner_output_in_tx(&mut tx, from).await?;
        }
        let row = sqlx::query(
            "INSERT INTO messages (from_actor, to_actor, body) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(from)
        .bind(to)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        // Directed-mail NOTIFY comes from the database trigger (0002).
        Ok(row.get(0))
    }

    /// Persist a conversation process's final ordinary Message only while its
    /// exact Actor-wide lease is still live. The Actor row lock is shared with
    /// lease invalidation and Work claim, closing the check/write race across
    /// rolling daemon replicas.
    pub async fn send_message_with_cognitive_lease(
        &self,
        lease: &ActorCognitiveLease,
        to: Option<&str>,
        body: &str,
    ) -> Result<i64> {
        validate_conversation_body(body)?;
        let mut tx = self.pool.begin().await?;
        validate_cognitive_lease_in_tx(&mut tx, lease).await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor,to_actor,body) VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(&lease.actor_id)
        .bind(to)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Atomically finish one leased conversation: optionally persist its
    /// single owner-facing answer, link it to Work, and consume only the exact
    /// Messages/handoffs that entered this turn. A stable lease-derived command
    /// key makes a lost commit receipt replay return the same Message rather
    /// than answering the owner twice.
    pub async fn finalize_cognitive_conversation(
        &self,
        lease: &ActorCognitiveLease,
        owner_reply: Option<&str>,
        reply_work_id: Option<Uuid>,
        message_ids: &[i64],
        handoff_inputs: &[OwnerHandoffInput],
    ) -> Result<Option<i64>> {
        if owner_reply.is_none() && reply_work_id.is_some() {
            return Err(OrgIntelError::InvalidWork(
                "a Work-linked conversation finalization needs an owner reply".into(),
            ));
        }
        if owner_reply.is_some_and(|body| body.trim().is_empty()) {
            return Err(OrgIntelError::InvalidWork(
                "a conversation owner reply cannot be empty".into(),
            ));
        }
        if let Some(body) = owner_reply {
            validate_conversation_body(body)?;
        }
        let mut sorted_messages = message_ids.to_vec();
        sorted_messages.sort_unstable();
        sorted_messages.dedup();
        let mut sorted_handoffs = handoff_inputs.to_vec();
        sorted_handoffs.sort_by_key(|input| input.id);
        if sorted_handoffs
            .windows(2)
            .any(|pair| pair[0].id == pair[1].id)
        {
            return Err(OrgIntelError::InvalidWork(
                "a cognitive turn cannot consume the same handoff twice".into(),
            ));
        }
        let payload = serde_json::to_vec(&serde_json::json!({
            "domain": "restless.cognitive-conversation-final.v1",
            "actor_id": &lease.actor_id,
            "reply": owner_reply,
            "reply_work_id": reply_work_id,
            "message_ids": &sorted_messages,
            "handoffs": &sorted_handoffs,
        }))
        .expect("cognitive finalization payload contains only serializable primitives");
        let payload_sha256 = format!("{:x}", Sha256::digest(payload));
        let command_id = format!("runtime-cognitive-final:{}", lease.token);

        let mut tx = self.pool.begin().await?;
        // A lost commit receipt is safe even after the process was fenced: the
        // exact durable reply proves the whole transaction (including input
        // consumption) already committed. Semantic drift still conflicts.
        if owner_reply.is_some() {
            if let Some((message_id, prior_digest)) = sqlx::query_as::<_, (i64, String)>(
                "SELECT id,client_payload_sha256 FROM messages \
                 WHERE to_actor IS NULL AND from_actor=$1 AND client_command_id=$2",
            )
            .bind(&lease.actor_id)
            .bind(&command_id)
            .fetch_optional(&mut *tx)
            .await?
            {
                if prior_digest != payload_sha256 {
                    return Err(OrgIntelError::RoomCommandConflict(
                        "the cognitive-session finalization key was reused with different semantics"
                            .into(),
                    ));
                }
                tx.commit().await?;
                return Ok(Some(message_id));
            }
        }
        let reply_work = if let Some(work_id) = reply_work_id {
            Some((
                work_id,
                crate::actors::lock_work_accountability_in_tx(&mut tx, work_id).await?,
            ))
        } else {
            None
        };
        // Work-linked replies take the canonical Work -> Team -> Actor locks
        // before upgrading the exact replying Actor to the lease fence. This
        // matches reassignment/lifecycle order and keeps responsibility live
        // through the atomic reply + consumption commit.
        validate_cognitive_lease_in_tx(&mut tx, lease).await?;
        if let Some((work_id, accountability)) = reply_work.as_ref() {
            if accountability.owner_id != lease.actor_id
                && accountability.lead_actor_id.as_deref() != Some(lease.actor_id.as_str())
            {
                return Err(OrgIntelError::InvalidWork(format!(
                    "Work {work_id} belongs to {:?}; its accountable lead is {:?}, not replying actor {:?}",
                    accountability.owner_id, accountability.lead_actor_id, lease.actor_id
                )));
            }
        }
        let lease_claimed_at: DateTime<Utc> = sqlx::query_scalar(
            "SELECT claimed_at FROM actor_cognitive_leases \
             WHERE actor_id=$1 AND lease_token=$2",
        )
        .bind(&lease.actor_id)
        .bind(lease.token)
        .fetch_one(&mut *tx)
        .await?;
        if !sorted_messages.is_empty() {
            let still_owed: Vec<i64> = sqlx::query_scalar(
                "SELECT message.id FROM messages message \
                 WHERE message.id=ANY($1) AND message.read_at IS NULL \
                   AND (\
                     EXISTS (SELECT 1 FROM work_feedback feedback \
                             WHERE feedback.message_id=message.id \
                               AND feedback.routed_to_actor=$2) \
                     OR (message.to_actor=$2 AND NOT EXISTS (\
                       SELECT 1 FROM work_feedback feedback \
                       WHERE feedback.message_id=message.id \
                         AND feedback.routed_to_actor IS NOT NULL\
                     ))\
                   ) \
                 ORDER BY message.id FOR UPDATE",
            )
            .bind(&sorted_messages)
            .bind(&lease.actor_id)
            .fetch_all(&mut *tx)
            .await?;
            if still_owed != sorted_messages {
                return Err(OrgIntelError::RoomCommandConflict(
                    "the cognitive-session inputs are no longer all owed to this Actor".into(),
                ));
            }
        }
        if !sorted_handoffs.is_empty() {
            let handoff_ids = sorted_handoffs
                .iter()
                .map(|input| input.id)
                .collect::<Vec<_>>();
            let handoffs: Vec<OwnerHandoffRow> = sqlx::query_as(
                "SELECT id,work_id,attempt_id,requested_by,category,requested_action, \
                        prepared_state,resume_condition,state,resolution,assigned_to, \
                        escalated_from,escalated_at,owner_brief,briefed_by,briefed_at, \
                        brief_source_fingerprint,delivered_at,created_at,resolved_at \
                 FROM owner_handoffs WHERE id=ANY($1) ORDER BY id FOR UPDATE",
            )
            .bind(&handoff_ids)
            .fetch_all(&mut *tx)
            .await?;
            let exact_ids = handoffs.iter().map(|row| row.id).collect::<Vec<_>>();
            let all_handled_by_this_turn = exact_ids == handoff_ids
                && handoffs
                    .iter()
                    .zip(&sorted_handoffs)
                    .all(|(handoff, captured)| {
                        handoff.conversation_input() == *captured
                            && match handoff.state {
                                OwnerHandoffState::Pending => {
                                    (handoff.assigned_to.as_deref()
                                        == Some(lease.actor_id.as_str())
                                        && handoff.delivered_at.is_none())
                                        || (handoff.escalated_from.as_deref()
                                            == Some(lease.actor_id.as_str())
                                            && handoff
                                                .escalated_at
                                                .is_some_and(|at| at >= lease_claimed_at))
                                }
                                OwnerHandoffState::Resolved
                                | OwnerHandoffState::Declined
                                | OwnerHandoffState::Withdrawn => {
                                    handoff.assigned_to.as_deref() == Some(lease.actor_id.as_str())
                                        && handoff
                                            .resolved_at
                                            .is_some_and(|at| at >= lease_claimed_at)
                                }
                            }
                    });
            if !all_handled_by_this_turn {
                return Err(OrgIntelError::RoomCommandConflict(
                    "the cognitive-session handoffs were not all handled by this exact Actor turn"
                        .into(),
                ));
            }
        }
        let reply_message_id = if let Some(body) = owner_reply {
            let inserted: Option<i64> = sqlx::query_scalar(
                "INSERT INTO messages \
                 (from_actor,to_actor,body,client_command_id,client_payload_sha256) \
                 VALUES ($1,NULL,$2,$3,$4) \
                 ON CONFLICT DO NOTHING \
                 RETURNING id",
            )
            .bind(&lease.actor_id)
            .bind(body)
            .bind(&command_id)
            .bind(&payload_sha256)
            .fetch_optional(&mut *tx)
            .await?;
            let message_id = if let Some(message_id) = inserted {
                message_id
            } else {
                let (message_id, prior_digest): (i64, String) = sqlx::query_as(
                    "SELECT id,client_payload_sha256 FROM messages \
                     WHERE to_actor IS NULL AND from_actor=$1 AND client_command_id=$2",
                )
                .bind(&lease.actor_id)
                .bind(&command_id)
                .fetch_one(&mut *tx)
                .await?;
                if prior_digest != payload_sha256 {
                    return Err(OrgIntelError::RoomCommandConflict(
                        "the cognitive-session finalization key was reused with different semantics"
                            .into(),
                    ));
                }
                message_id
            };
            if let Some(work_id) = reply_work_id {
                sqlx::query(
                    "INSERT INTO work_feedback (work_id,message_id,linked_by) \
                     VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
                )
                .bind(work_id)
                .bind(message_id)
                .bind(&lease.actor_id)
                .execute(&mut *tx)
                .await?;
            }
            Some(message_id)
        } else {
            None
        };

        if !sorted_messages.is_empty() {
            sqlx::query("UPDATE messages SET read_at=COALESCE(read_at,now()) WHERE id=ANY($1)")
                .bind(&sorted_messages)
                .execute(&mut *tx)
                .await?;
        }
        if !sorted_handoffs.is_empty() {
            let handoff_ids = sorted_handoffs
                .iter()
                .map(|input| input.id)
                .collect::<Vec<_>>();
            sqlx::query(
                "UPDATE owner_handoffs SET delivered_at=COALESCE(delivered_at,now()) \
                 WHERE id=ANY($1) AND assigned_to=$2 AND state='pending'",
            )
            .bind(&handoff_ids)
            .bind(&lease.actor_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(reply_message_id)
    }

    /// Count durable lead-to-Staff messages for the decision telemetry
    /// projection. This is an observation of persisted coordination, not a
    /// second intervention lifecycle.
    pub async fn count_internal_messages_from(
        &self,
        actors: &[String],
        excluded_recipients: &[String],
    ) -> Result<i64> {
        if actors.is_empty() {
            return Ok(0);
        }
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages \
             WHERE from_actor=ANY($1) AND to_actor IS NOT NULL \
               AND NOT (to_actor=ANY($2))",
        )
        .bind(actors)
        .bind(excluded_recipients)
        .fetch_one(&self.pool)
        .await?)
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
        accountable_team_id: Option<Uuid>,
    ) -> Result<(i64, bool, String)> {
        if source_ref.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "external projection needs bounded context and a stable source reference".into(),
            ));
        }
        validate_conversation_body(body)?;
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
            let existing: Option<(i64, String)> = sqlx::query_as(
                "SELECT source.message_id,message.to_actor \
                 FROM external_message_sources source \
                 JOIN messages message ON message.id=source.message_id \
                 WHERE source.source_ref=$1",
            )
            .bind(source_ref)
            .fetch_optional(&mut *tx)
            .await?;
            let (message_id, routed_to) = existing.ok_or_else(|| {
                OrgIntelError::InvalidWork(
                    "external source projection exists without its message".into(),
                )
            })?;
            tx.commit().await?;
            return Ok((message_id, false, routed_to));
        }
        let mut routed_to = to.to_string();
        let mut effective_work_id = work_id;
        if let Some(work_id) = work_id {
            let accountability =
                crate::actors::lock_work_accountability_in_tx(&mut tx, work_id).await?;
            let pending_handoff: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM owner_handoffs \
                 WHERE work_id=$1 AND state='pending')",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
            let running_attempt: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM work_attempts \
                 WHERE work_id=$1 AND state='running')",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
            routed_to = match accountability.status {
                WorkStatus::Proposed | WorkStatus::Active
                    if !pending_handoff && !running_attempt =>
                {
                    accountability.owner_id
                }
                WorkStatus::Blocked | WorkStatus::Proposed | WorkStatus::Active => accountability
                    .lead_actor_id
                    .unwrap_or_else(|| "exec".to_string()),
                WorkStatus::Completed | WorkStatus::Abandoned => {
                    effective_work_id = None;
                    accountability
                        .lead_actor_id
                        .unwrap_or_else(|| "exec".to_string())
                }
            };
        } else if let Some(team_id) = accountable_team_id {
            // Department addresses name a durable team, not whichever lead an
            // earlier HTTP snapshot happened to observe. Serialize against
            // replacement/disband, then validate the exact current agent lead
            // before inserting the Message. A lifecycle-first race falls
            // through to the Exec rather than stranding mail on a former lead.
            let live_lead: Option<String> = sqlx::query_scalar(
                "SELECT lead_actor_id FROM teams \
                 WHERE id=$1 AND disbanded_at IS NULL FOR SHARE",
            )
            .bind(team_id)
            .fetch_optional(&mut *tx)
            .await?;
            routed_to = if let Some(lead) = live_lead {
                let addressable: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM actors \
                     WHERE id=$1 AND retired_at IS NULL AND actor_class='agent' FOR SHARE)",
                )
                .bind(&lead)
                .fetch_one(&mut *tx)
                .await?;
                if addressable {
                    lead
                } else {
                    "exec".to_string()
                }
            } else {
                "exec".to_string()
            };
        }
        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor,to_actor,body) VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(from)
        .bind(&routed_to)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;
        if let Some(work_id) = effective_work_id {
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
        Ok((message_id, true, routed_to))
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

    /// Send one human-attributed direct conversation message. The caller must
    /// pass the Actor id derived from the verified access context; raw IdP
    /// subject and membership identifiers never become Message attribution.
    ///
    /// This compatibility API intentionally has no caller-controlled retry
    /// key. New HTTP commands should use `send_room_message_with_standard` so
    /// mobile/network retries are idempotent at the command boundary.
    pub async fn send_human_conversation_message(
        &self,
        from_actor: &str,
        target_actor: &str,
        body: &str,
        new_focus: bool,
    ) -> Result<(i64, ConversationFocusRow)> {
        self.send_human_conversation_message_with_standard(
            from_actor,
            target_actor,
            body,
            new_focus,
            None,
        )
        .await
    }

    /// Retry-safe network conversation command. The caller supplies a digest
    /// of the stable request semantics (before attempt-local attachment paths
    /// are allocated). The Message remains the command receipt; focus moves in
    /// the same transaction only for the winning insert. An exact replay is
    /// returned before current target-role admission, so a lost response stays
    /// recoverable after a later lead replacement without creating new work.
    #[expect(
        clippy::too_many_arguments,
        reason = "the network command keeps target, retry, focus and standard semantics explicit"
    )]
    pub async fn send_human_runtime_conversation_message_idempotent_with_standard(
        &self,
        from_actor: &str,
        target_actor: &str,
        body: &str,
        new_focus: bool,
        outcome_standard: Option<OutcomeStandard>,
        attachment_ids: &[Uuid],
        client_command_id: &str,
        client_payload_sha256: &str,
    ) -> Result<(i64, ConversationFocusRow, bool)> {
        let from_actor = from_actor.trim();
        let target_actor = target_actor.trim();
        let client_command_id = client_command_id.trim();
        if from_actor.is_empty() || target_actor.is_empty() || from_actor == target_actor {
            return Err(OrgIntelError::InvalidRoom(
                "a human direct conversation needs two distinct stable Actors".into(),
            ));
        }
        if body.trim().is_empty() || body.len() > 64 * 1024 {
            return Err(OrgIntelError::InvalidRoom(
                "a human conversation message needs 1 to 65536 bytes".into(),
            ));
        }
        if client_command_id.is_empty() || client_command_id.len() > 128 {
            return Err(OrgIntelError::InvalidRoom(
                "a retryable conversation needs a 1 to 128 byte client command id".into(),
            ));
        }
        if client_payload_sha256.len() != 64
            || !client_payload_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(OrgIntelError::InvalidRoom(
                "a retryable conversation needs its lowercase SHA-256 semantic digest".into(),
            ));
        }

        let canonical_key = crate::rooms::direct_room_canonical_key(from_actor, target_actor);
        let mut tx = self.pool.begin().await?;
        if let Some((message_id, prior_digest, focus_after, focus_started)) =
            sqlx::query_as::<_, (i64, String, Option<i64>, Option<DateTime<Utc>>)>(
                "SELECT message.id,message.client_payload_sha256, \
                        message.conversation_focus_receipt_after_message_id, \
                        message.conversation_focus_receipt_started_at \
                 FROM messages message JOIN rooms room ON room.id=message.room_id \
                 WHERE room.canonical_key=$1 AND message.from_actor=$2 \
                   AND message.client_command_id=$3",
            )
            .bind(&canonical_key)
            .bind(from_actor)
            .bind(client_command_id)
            .fetch_optional(&mut *tx)
            .await?
        {
            if prior_digest != client_payload_sha256 {
                return Err(OrgIntelError::RoomCommandConflict(format!(
                    "conversation command {client_command_id:?} was already used with different semantics"
                )));
            }
            let focus_after = focus_after.ok_or_else(|| {
                OrgIntelError::RoomCommandConflict(
                    "conversation command receipt is missing its focus result".into(),
                )
            })?;
            tx.commit().await?;
            return Ok((
                message_id,
                ConversationFocusRow {
                    after_message_id: focus_after,
                    started_at: focus_started,
                },
                false,
            ));
        }

        crate::rooms::lock_runtime_addressable_actors_in_tx(
            &mut tx,
            &[target_actor.to_string()],
            false,
            &[from_actor.to_string()],
        )
        .await?;
        let sender_is_human: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors \
             WHERE id=$1 AND actor_class='human' AND retired_at IS NULL FOR SHARE)",
        )
        .bind(from_actor)
        .fetch_one(&mut *tx)
        .await?;
        if !sender_is_human {
            return Err(OrgIntelError::RoomAccessDenied(
                "the authenticated sender is not an active human Actor".into(),
            ));
        }
        lock_owner_attachments_for_command_in_tx(
            &mut tx,
            from_actor,
            target_actor,
            client_command_id,
            client_payload_sha256,
            attachment_ids,
            body,
        )
        .await?;

        let inserted: Option<(i64, Uuid)> = sqlx::query_as(
            "INSERT INTO messages \
             (from_actor,to_actor,body,client_command_id,client_payload_sha256,outcome_standard) \
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING RETURNING id,room_id",
        )
        .bind(from_actor)
        .bind(target_actor)
        .bind(body)
        .bind(client_command_id)
        .bind(client_payload_sha256)
        .bind(outcome_standard)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((message_id, room_id)) = inserted else {
            let (message_id, prior_digest, focus_after, focus_started): (
                i64,
                String,
                Option<i64>,
                Option<DateTime<Utc>>,
            ) = sqlx::query_as(
                "SELECT message.id,message.client_payload_sha256, \
                        message.conversation_focus_receipt_after_message_id, \
                        message.conversation_focus_receipt_started_at \
                 FROM messages message JOIN rooms room ON room.id=message.room_id \
                 WHERE room.canonical_key=$1 AND message.from_actor=$2 \
                   AND message.client_command_id=$3",
            )
            .bind(&canonical_key)
            .bind(from_actor)
            .bind(client_command_id)
            .fetch_one(&mut *tx)
            .await?;
            if prior_digest != client_payload_sha256 {
                return Err(OrgIntelError::RoomCommandConflict(format!(
                    "conversation command {client_command_id:?} was concurrently used with different semantics"
                )));
            }
            let focus_after = focus_after.ok_or_else(|| {
                OrgIntelError::RoomCommandConflict(
                    "conversation command receipt is missing its focus result".into(),
                )
            })?;
            tx.commit().await?;
            return Ok((
                message_id,
                ConversationFocusRow {
                    after_message_id: focus_after,
                    started_at: focus_started,
                },
                false,
            ));
        };
        link_owner_attachments_in_tx(&mut tx, attachment_ids, message_id).await?;

        let prior_message_id: Option<i64> = if new_focus {
            sqlx::query_scalar("SELECT MAX(id) FROM messages WHERE room_id=$1 AND id<>$2")
                .bind(room_id)
                .bind(message_id)
                .fetch_one(&mut *tx)
                .await?
        } else {
            None
        };
        if new_focus {
            sqlx::query(
                "INSERT INTO room_conversation_focus \
                 (room_id,target_actor_id,after_message_id,started_at) \
                 VALUES ($1,$2,$3,now()) \
                 ON CONFLICT (room_id,target_actor_id) DO UPDATE \
                   SET after_message_id=EXCLUDED.after_message_id,started_at=now()",
            )
            .bind(room_id)
            .bind(target_actor)
            .bind(prior_message_id)
            .execute(&mut *tx)
            .await?;
            if from_actor == "owner" {
                sqlx::query(
                    "UPDATE actors SET conversation_focus_after_message_id=COALESCE($2,0), \
                            conversation_focus_started_at=now() \
                     WHERE id=$1 AND retired_at IS NULL",
                )
                .bind(target_actor)
                .bind(prior_message_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        let focus: ConversationFocusRow = sqlx::query_as(
            "SELECT COALESCE(focus.after_message_id, \
                     CASE WHEN $3='owner' THEN target.conversation_focus_after_message_id ELSE 0 END) \
                       AS after_message_id, \
                    COALESCE(focus.started_at, \
                     CASE WHEN $3='owner' THEN target.conversation_focus_started_at END) \
                       AS started_at \
             FROM actors target \
             LEFT JOIN room_conversation_focus focus \
               ON focus.room_id=$1 AND focus.target_actor_id=target.id \
             WHERE target.id=$2 AND target.retired_at IS NULL",
        )
        .bind(room_id)
        .bind(target_actor)
        .bind(from_actor)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE messages SET conversation_focus_receipt_after_message_id=$2, \
                    conversation_focus_receipt_started_at=$3 WHERE id=$1",
        )
        .bind(message_id)
        .bind(focus.after_message_id)
        .bind(focus.started_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((message_id, focus, true))
    }

    /// Read an exact committed network conversation command before consulting
    /// mutable target responsibility or Attention projections. This is a
    /// receipt lookup only: the HTTP boundary still authenticates the current
    /// human sender, and any semantic drift is a conflict.
    pub async fn conversation_command_receipt(
        &self,
        from_actor: &str,
        target_actor: &str,
        client_command_id: &str,
        client_payload_sha256: &str,
    ) -> Result<Option<(i64, Option<ConversationFocusRow>, String)>> {
        let canonical_key = crate::rooms::direct_room_canonical_key(from_actor, target_actor);
        let existing =
            sqlx::query_as::<_, (i64, String, Option<i64>, Option<DateTime<Utc>>, String)>(
                "SELECT message.id,message.client_payload_sha256, \
                    message.conversation_focus_receipt_after_message_id, \
                    message.conversation_focus_receipt_started_at,message.body \
             FROM messages message JOIN rooms room ON room.id=message.room_id \
             WHERE room.canonical_key=$1 AND message.from_actor=$2 \
               AND message.client_command_id=$3",
            )
            .bind(&canonical_key)
            .bind(from_actor)
            .bind(client_command_id.trim())
            .fetch_optional(&self.pool)
            .await?;
        let Some((message_id, prior_digest, focus_after, focus_started, body)) = existing else {
            return Ok(None);
        };
        if prior_digest != client_payload_sha256 {
            return Err(OrgIntelError::RoomCommandConflict(format!(
                "conversation command {:?} was already used with different semantics",
                client_command_id.trim()
            )));
        }
        Ok(Some((
            message_id,
            focus_after.map(|after_message_id| ConversationFocusRow {
                after_message_id,
                started_at: focus_started,
            }),
            body,
        )))
    }

    /// Return canonical attachment metadata only while the requesting Actor is
    /// a current participant in the authoritative Message Room. UUID secrecy
    /// is never treated as an authorization boundary.
    pub async fn owner_attachment_for_actor(
        &self,
        attachment_id: Uuid,
        requesting_actor: &str,
    ) -> Result<Option<OwnerAttachmentRecord>> {
        Ok(sqlx::query_as(
            "SELECT attachment.attachment_id,attachment.canonical_name, \
                    attachment.canonical_media_type,attachment.size_bytes, \
                    attachment.content_sha256,attachment.message_id \
             FROM owner_attachments attachment \
             JOIN messages message ON message.id=attachment.message_id \
             JOIN room_participants participant ON participant.room_id=message.room_id \
             JOIN actors actor ON actor.id=participant.actor_id \
             WHERE attachment.attachment_id=$1 AND participant.actor_id=$2 \
               AND attachment.purge_requested_at IS NULL AND attachment.purged_at IS NULL \
               AND participant.left_at IS NULL AND actor.retired_at IS NULL \
               AND actor.actor_class='human'",
        )
        .bind(attachment_id)
        .bind(requesting_actor)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Register daemon-generated attachment identities before any company
    /// filesystem bytes are written. The table lock makes the per-company
    /// file/byte caps exact under concurrent browser requests; it is staging
    /// metadata only, never a second Message or artifact ledger.
    #[allow(clippy::too_many_arguments)]
    pub async fn register_owner_attachments(
        &self,
        sender_actor_id: &str,
        target_actor_id: &str,
        client_command_id: &str,
        client_payload_sha256: &str,
        attachments: &[OwnerAttachmentRegistration],
        max_staged_files: i64,
        max_staged_bytes: i64,
        max_retained_files: i64,
        max_retained_bytes: i64,
        max_principal_retained_files: i64,
        max_principal_retained_bytes: i64,
    ) -> Result<()> {
        if attachments.is_empty() {
            return Ok(());
        }
        if max_staged_files <= 0
            || max_staged_bytes <= 0
            || max_retained_files <= 0
            || max_retained_bytes <= 0
            || max_principal_retained_files <= 0
            || max_principal_retained_bytes <= 0
        {
            return Err(OrgIntelError::InvalidRoom(
                "attachment staging limits must be positive".into(),
            ));
        }
        let mut ids = attachments
            .iter()
            .map(|attachment| attachment.attachment_id)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        if ids.len() != attachments.len()
            || attachments.iter().any(|attachment| {
                attachment.size_bytes < 0
                    || attachment.canonical_name.is_empty()
                    || attachment.canonical_name.len() > 1024
                    || attachment.canonical_media_type.is_empty()
                    || attachment.canonical_media_type.len() > 255
                    || attachment.content_sha256.len() != 64
                    || !attachment
                        .content_sha256
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            })
            || client_command_id.trim().is_empty()
            || client_command_id.trim().len() > 128
            || client_payload_sha256.len() != 64
            || !client_payload_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(OrgIntelError::InvalidRoom(
                "invalid owner attachment staging command".into(),
            ));
        }
        let incoming_bytes = attachments
            .iter()
            .try_fold(0i64, |total, attachment| {
                total.checked_add(attachment.size_bytes)
            })
            .ok_or_else(|| OrgIntelError::InvalidRoom("attachment byte count overflow".into()))?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("LOCK TABLE owner_attachments IN SHARE ROW EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await?;
        let (
            retained_files,
            retained_bytes,
            staged_files,
            staged_bytes,
            principal_files,
            principal_bytes,
        ): (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
            "SELECT COUNT(*),COALESCE(SUM(size_bytes),0)::bigint, \
                        COUNT(*) FILTER (WHERE message_id IS NULL OR staging_finished_at IS NULL), \
                        COALESCE(SUM(size_bytes) FILTER \
                          (WHERE message_id IS NULL OR staging_finished_at IS NULL),0)::bigint \
                        ,COUNT(*) FILTER (WHERE sender_actor_id=$1), \
                        COALESCE(SUM(size_bytes) FILTER (WHERE sender_actor_id=$1),0)::bigint \
                 FROM owner_attachments WHERE purged_at IS NULL",
        )
        .bind(sender_actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if retained_files.saturating_add(attachments.len() as i64) > max_retained_files
            || retained_bytes.saturating_add(incoming_bytes) > max_retained_bytes
        {
            return Err(OrgIntelError::InvalidRoom(
                "retained attachment storage is full; remove retained company evidence before uploading more"
                    .into(),
            ));
        }
        if staged_files.saturating_add(attachments.len() as i64) > max_staged_files
            || staged_bytes.saturating_add(incoming_bytes) > max_staged_bytes
        {
            return Err(OrgIntelError::InvalidRoom(
                "attachment staging is full; retry the same command after prior delivery receipts settle"
                    .into(),
            ));
        }
        if principal_files.saturating_add(attachments.len() as i64) > max_principal_retained_files
            || principal_bytes.saturating_add(incoming_bytes) > max_principal_retained_bytes
        {
            return Err(OrgIntelError::InvalidRoom(
                "this principal's retained attachment quota is full".into(),
            ));
        }
        for attachment in attachments {
            sqlx::query(
                "INSERT INTO owner_attachments \
                 (attachment_id,sender_actor_id,target_actor_id,client_command_id, \
                  client_payload_sha256,canonical_name,canonical_media_type,size_bytes,content_sha256) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            )
            .bind(attachment.attachment_id)
            .bind(sender_actor_id)
            .bind(target_actor_id)
            .bind(client_command_id.trim())
            .bind(client_payload_sha256)
            .bind(&attachment.canonical_name)
            .bind(&attachment.canonical_media_type)
            .bind(attachment.size_bytes)
            .bind(&attachment.content_sha256)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Remove only attempt-local stages that were never linked by a Message
    /// commit. Callers remove the corresponding filesystem paths first, so a
    /// crash leaves a durable row for bounded recovery rather than an
    /// untracked orphan.
    pub async fn discard_unlinked_owner_attachments(&self, attachment_ids: &[Uuid]) -> Result<u64> {
        if attachment_ids.is_empty() {
            return Ok(0);
        }
        Ok(sqlx::query(
            "DELETE FROM owner_attachments \
             WHERE attachment_id=ANY($1) AND message_id IS NULL \
               AND gc_claim_token IS NULL",
        )
        .bind(attachment_ids)
        .execute(&self.pool)
        .await?
        .rows_affected())
    }

    /// Clear acknowledged stages only after the filesystem staging receipt is
    /// removed. A row linked to a Message is the durable proof that content
    /// must be retained.
    pub async fn finish_linked_owner_attachments(&self, attachment_ids: &[Uuid]) -> Result<u64> {
        if attachment_ids.is_empty() {
            return Ok(0);
        }
        Ok(sqlx::query(
            "UPDATE owner_attachments SET staging_finished_at=COALESCE(staging_finished_at,now()) \
             WHERE attachment_id=ANY($1) AND message_id IS NOT NULL \
               AND gc_claim_token IS NULL",
        )
        .bind(attachment_ids)
        .execute(&self.pool)
        .await?
        .rows_affected())
    }

    /// Record an owner-authorized retention decision before touching bytes.
    /// The permanent row is an audit tombstone after collection; a crash
    /// before or after filesystem deletion is recovered by the tokenized GC
    /// claim rather than turning deletion into an unrecorded side effect.
    pub async fn request_owner_attachment_purge(
        &self,
        attachment_id: Uuid,
        requested_by: &str,
        reason: &str,
    ) -> Result<bool> {
        let reason = reason.trim();
        if reason.is_empty() || reason.len() > 2_000 {
            return Err(OrgIntelError::InvalidRoom(
                "attachment purge needs a 1 to 2000 byte reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "SELECT id FROM actors WHERE id=$1 AND actor_class='human' \
             AND retired_at IS NULL FOR SHARE",
        )
        .bind(requested_by)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::RoomAccessDenied(
                "attachment retention may only be changed by an active human principal".into(),
            )
        })?;
        let state: Option<(bool, bool)> = sqlx::query_as(
            "SELECT purge_requested_at IS NOT NULL,purged_at IS NOT NULL \
             FROM owner_attachments \
             WHERE attachment_id=$1 AND message_id IS NOT NULL FOR UPDATE",
        )
        .bind(attachment_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((purge_requested, _)) = state else {
            return Err(OrgIntelError::InvalidRoom(
                "the retained attachment does not exist".into(),
            ));
        };
        if purge_requested {
            tx.commit().await?;
            return Ok(false);
        }
        sqlx::query(
            "UPDATE owner_attachments \
             SET purge_requested_at=now(),purge_requested_by=$2,purge_reason=$3, \
                 gc_claimed_at=NULL,gc_claim_token=NULL \
             WHERE attachment_id=$1",
        )
        .bind(attachment_id)
        .bind(requested_by)
        .bind(reason)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO events (kind,actor_id,body) VALUES ($1,$2,$3)")
            .bind("owner.attachment.purge_requested.v1")
            .bind(requested_by)
            .bind(serde_json::json!({
                "attachment_id": attachment_id,
                "reason": reason,
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }

    /// Claim at most one bounded batch of stages for restart recovery. Linked
    /// rows retain content; old unlinked rows are atomically fenced from every
    /// future Message transaction before filesystem quarantine begins.
    pub async fn claim_owner_attachment_gc(
        &self,
        limit: i64,
        stale_after_seconds: i64,
        claim_for_seconds: i64,
    ) -> Result<Vec<(Uuid, bool, bool, Uuid)>> {
        let limit = limit.clamp(1, 64);
        if stale_after_seconds <= 0 || claim_for_seconds <= 0 {
            return Err(OrgIntelError::InvalidRoom(
                "attachment staging retention and claim lease must be positive".into(),
            ));
        }
        let claim_token = Uuid::new_v4();
        Ok(sqlx::query_as(
            "WITH candidates AS ( \
               SELECT attachment_id FROM owner_attachments \
               WHERE ((purge_requested_at IS NOT NULL AND purged_at IS NULL) \
                  OR (message_id IS NOT NULL AND staging_finished_at IS NULL) \
                  OR (message_id IS NULL \
                      AND created_at<=now()-make_interval(secs=>$2::double precision))) \
                 AND (gc_claimed_at IS NULL \
                  OR gc_claimed_at<=now()-make_interval(secs=>$3::double precision)) \
               ORDER BY (message_id IS NOT NULL) DESC,created_at,attachment_id \
               FOR UPDATE SKIP LOCKED LIMIT $1 \
             ) \
             UPDATE owner_attachments stage \
             SET gc_claimed_at=now(),gc_claim_token=$4 \
             FROM candidates WHERE stage.attachment_id=candidates.attachment_id \
             RETURNING stage.attachment_id,(stage.message_id IS NOT NULL), \
                       (stage.purge_requested_at IS NOT NULL),stage.gc_claim_token",
        )
        .bind(limit)
        .bind(stale_after_seconds)
        .bind(claim_for_seconds)
        .bind(claim_token)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn complete_owner_attachment_gc(
        &self,
        attachment_id: Uuid,
        claim_token: Uuid,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let state: Option<(bool, bool)> = sqlx::query_as(
            "SELECT message_id IS NOT NULL,purge_requested_at IS NOT NULL \
             FROM owner_attachments \
             WHERE attachment_id=$1 AND gc_claim_token=$2 FOR UPDATE",
        )
        .bind(attachment_id)
        .bind(claim_token)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((linked, purge_requested)) = state else {
            tx.commit().await?;
            return Ok(false);
        };
        if purge_requested {
            sqlx::query(
                "UPDATE owner_attachments \
                 SET purged_at=COALESCE(purged_at,now()), \
                     staging_finished_at=COALESCE(staging_finished_at,now()), \
                     gc_claimed_at=NULL,gc_claim_token=NULL \
                 WHERE attachment_id=$1 AND gc_claim_token=$2 \
                   AND purge_requested_at IS NOT NULL",
            )
            .bind(attachment_id)
            .bind(claim_token)
            .execute(&mut *tx)
            .await?;
        } else if linked {
            sqlx::query(
                "UPDATE owner_attachments \
                 SET staging_finished_at=COALESCE(staging_finished_at,now()), \
                     gc_claimed_at=NULL,gc_claim_token=NULL \
                 WHERE attachment_id=$1 AND gc_claim_token=$2",
            )
            .bind(attachment_id)
            .bind(claim_token)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "DELETE FROM owner_attachments \
                 WHERE attachment_id=$1 AND gc_claim_token=$2 AND message_id IS NULL",
            )
            .bind(attachment_id)
            .bind(claim_token)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(true)
    }

    /// Human direct conversation with a composer-level outcome standard. A
    /// focus boundary is scoped to this direct Room, so one human cannot erase
    /// another person's context window for the same persistent agent.
    pub async fn send_human_conversation_message_with_standard(
        &self,
        from_actor: &str,
        target_actor: &str,
        body: &str,
        new_focus: bool,
        outcome_standard: Option<OutcomeStandard>,
    ) -> Result<(i64, ConversationFocusRow)> {
        self.send_human_conversation_message_internal(
            from_actor,
            target_actor,
            body,
            new_focus,
            outcome_standard,
            false,
        )
        .await
    }

    /// Network owner/agent conversation boundary. Unlike the arbitrary-human
    /// compatibility facade, this admits only the singleton Exec or a current
    /// accountable lead and locks that role in the same transaction as the
    /// Message insert. Lead replacement/disband therefore linearizes with the
    /// send instead of stranding a post-lifecycle message.
    pub async fn send_human_runtime_conversation_message_with_standard(
        &self,
        from_actor: &str,
        target_actor: &str,
        body: &str,
        new_focus: bool,
        outcome_standard: Option<OutcomeStandard>,
    ) -> Result<(i64, ConversationFocusRow)> {
        self.send_human_conversation_message_internal(
            from_actor,
            target_actor,
            body,
            new_focus,
            outcome_standard,
            true,
        )
        .await
    }

    async fn send_human_conversation_message_internal(
        &self,
        from_actor: &str,
        target_actor: &str,
        body: &str,
        new_focus: bool,
        outcome_standard: Option<OutcomeStandard>,
        require_runtime_target: bool,
    ) -> Result<(i64, ConversationFocusRow)> {
        let from_actor = from_actor.trim();
        let target_actor = target_actor.trim();
        if from_actor.is_empty() || target_actor.is_empty() || from_actor == target_actor {
            return Err(OrgIntelError::InvalidRoom(
                "a human direct conversation needs two distinct stable Actors".into(),
            ));
        }
        if body.trim().is_empty() || body.len() > 64 * 1024 {
            return Err(OrgIntelError::InvalidRoom(
                "a human conversation message needs 1 to 65536 bytes".into(),
            ));
        }
        let human_is_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors \
             WHERE id=$1 AND actor_class='human' AND retired_at IS NULL)",
        )
        .bind(from_actor)
        .fetch_one(&self.pool)
        .await?;
        if !human_is_active {
            return Err(OrgIntelError::RoomAccessDenied(
                "the authenticated sender is not an active human Actor".into(),
            ));
        }

        let direct_room_command = format!(
            "direct-room-{:x}",
            Sha256::digest(crate::rooms::direct_room_canonical_key(
                from_actor,
                target_actor
            ))
        );
        let room = self
            .create_room(
                from_actor,
                RoomKind::Direct,
                "Direct conversation",
                &[target_actor],
                &direct_room_command,
            )
            .await?;
        let mut tx = self.pool.begin().await?;
        if require_runtime_target {
            crate::rooms::lock_runtime_addressable_actors_in_tx(
                &mut tx,
                &[target_actor.to_string()],
                false,
                &[from_actor.to_string()],
            )
            .await?;
        }
        // Lock the audience row through the Message commit so removal/archive
        // cannot race a send that was authorized against stale membership.
        crate::rooms::active_room_kind(&mut tx, room.id, from_actor).await?;

        if new_focus {
            let after_message_id: Option<i64> =
                sqlx::query_scalar("SELECT MAX(id) FROM messages WHERE room_id=$1")
                    .bind(room.id)
                    .fetch_one(&mut *tx)
                    .await?;
            sqlx::query(
                "INSERT INTO room_conversation_focus \
                 (room_id,target_actor_id,after_message_id,started_at) \
                 VALUES ($1,$2,$3,now()) \
                 ON CONFLICT (room_id,target_actor_id) DO UPDATE \
                   SET after_message_id=EXCLUDED.after_message_id,started_at=now()",
            )
            .bind(room.id)
            .bind(target_actor)
            .bind(after_message_id)
            .execute(&mut *tx)
            .await?;

            // Preserve the established owner/agent wake projection until all
            // model-context consumers move to Room-scoped focus.
            if from_actor == "owner" {
                sqlx::query(
                    "UPDATE actors SET conversation_focus_after_message_id=COALESCE($2,0), \
                            conversation_focus_started_at=now() \
                     WHERE id=$1 AND retired_at IS NULL",
                )
                .bind(target_actor)
                .bind(after_message_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (room_id,from_actor,to_actor,body,outcome_standard) \
             VALUES ($1,$2,$3,$4,$5) RETURNING id",
        )
        .bind(room.id)
        .bind(from_actor)
        .bind((target_actor != "owner").then_some(target_actor))
        .bind(body)
        .bind(outcome_standard)
        .fetch_one(&mut *tx)
        .await?;
        let focus = sqlx::query_as(
            "SELECT COALESCE(focus.after_message_id, \
                     CASE WHEN $3='owner' THEN target.conversation_focus_after_message_id ELSE 0 END) \
                       AS after_message_id, \
                    COALESCE(focus.started_at, \
                     CASE WHEN $3='owner' THEN target.conversation_focus_started_at END) \
                       AS started_at \
             FROM actors target \
             LEFT JOIN room_conversation_focus focus \
               ON focus.room_id=$1 AND focus.target_actor_id=target.id \
             WHERE target.id=$2 AND target.retired_at IS NULL",
        )
        .bind(room.id)
        .bind(target_actor)
        .bind(from_actor)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidRoom(format!(
                "active conversation target {target_actor:?} does not exist"
            ))
        })?;
        tx.commit().await?;
        Ok((message_id, focus))
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
        self.send_owner_conversation_message_with_standard(actor, body, new_focus, None)
            .await
    }

    /// Send owner conversation with an explicit composer-level outcome
    /// standard. Absence remains meaningful: the company default or the
    /// accountable Exec's natural-language judgement applies at commission.
    pub async fn send_owner_conversation_message_with_standard(
        &self,
        actor: &str,
        body: &str,
        new_focus: bool,
        outcome_standard: Option<OutcomeStandard>,
    ) -> Result<(i64, ConversationFocusRow)> {
        self.send_human_runtime_conversation_message_with_standard(
            "owner",
            actor,
            body,
            new_focus,
            outcome_standard,
        )
        .await
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
        self.send_work_message_internal(from, to, work_id, body, None, &[], None)
            .await
            .map(|(message_id, _)| message_id)
    }

    /// Retry-safe Work-linked form used by the network conversation endpoint.
    /// The Work feedback edge and any accountable-lead correction notice are
    /// committed only by the winning Message command.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_work_message_idempotent(
        &self,
        from: &str,
        to: &str,
        work_id: Uuid,
        body: &str,
        expected_handoff: Option<&OwnerHandoffInput>,
        attachment_ids: &[Uuid],
        client_command_id: &str,
        client_payload_sha256: &str,
    ) -> Result<(i64, bool)> {
        self.send_work_message_internal(
            from,
            to,
            work_id,
            body,
            expected_handoff,
            attachment_ids,
            Some((client_command_id, client_payload_sha256)),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_work_message_internal(
        &self,
        from: &str,
        to: &str,
        work_id: Uuid,
        body: &str,
        expected_handoff: Option<&OwnerHandoffInput>,
        attachment_ids: &[Uuid],
        idempotency: Option<(&str, &str)>,
    ) -> Result<(i64, bool)> {
        validate_conversation_body(body)?;
        if let Some((client_command_id, client_payload_sha256)) = idempotency {
            if client_command_id.trim().is_empty()
                || client_command_id.trim().len() > 128
                || client_payload_sha256.len() != 64
                || !client_payload_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(OrgIntelError::InvalidWork(
                    "retryable Work feedback needs a bounded command id and lowercase SHA-256 digest"
                        .into(),
                ));
            }
        }
        let mut tx = self.pool.begin().await?;
        let canonical_key = crate::rooms::direct_room_canonical_key(from, to);
        if let Some((client_command_id, client_payload_sha256)) = idempotency {
            if let Some((message_id, prior_digest)) = sqlx::query_as::<_, (i64, String)>(
                "SELECT message.id,message.client_payload_sha256 \
                 FROM messages message JOIN rooms room ON room.id=message.room_id \
                 WHERE room.canonical_key=$1 AND message.from_actor=$2 \
                   AND message.client_command_id=$3",
            )
            .bind(&canonical_key)
            .bind(from)
            .bind(client_command_id.trim())
            .fetch_optional(&mut *tx)
            .await?
            {
                if prior_digest != client_payload_sha256 {
                    return Err(OrgIntelError::RoomCommandConflict(format!(
                        "conversation command {:?} was already used with different semantics",
                        client_command_id.trim()
                    )));
                }
                tx.commit().await?;
                return Ok((message_id, false));
            }
        }
        let work = sqlx::query("SELECT owner_id,status FROM work WHERE id=$1 FOR SHARE")
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
        let owner: String = work.get("owner_id");
        let status: WorkStatus = work.get("status");
        if matches!(status, WorkStatus::Completed | WorkStatus::Abandoned) {
            return Err(OrgIntelError::InvalidWork(
                "settled Work cannot accept new linked conversation input".into(),
            ));
        }
        // Work is locked first, then its accountable Team, then Actors. This
        // is the same responsibility order used by Work/lead lifecycle and
        // makes the selected producer/lead authoritative through the insert.
        let owner_team_id: Option<Uuid> =
            sqlx::query_scalar("SELECT team_id FROM actors WHERE id=$1")
                .bind(&owner)
                .fetch_optional(&mut *tx)
                .await?
                .flatten();
        let accountable_lead: Option<String> = sqlx::query_scalar(
            "SELECT lead_actor_id FROM teams \
             WHERE id=$1 AND disbanded_at IS NULL FOR SHARE",
        )
        .bind(owner_team_id)
        .fetch_optional(&mut *tx)
        .await?;
        let mut required_actors = vec![from.to_string(), owner.clone(), to.to_string()];
        required_actors.sort();
        required_actors.dedup();
        let active_actors = sqlx::query_as::<_, (String, Option<Uuid>)>(
            "SELECT id,team_id FROM actors WHERE id=ANY($1) AND retired_at IS NULL \
             ORDER BY id FOR SHARE",
        )
        .bind(&required_actors)
        .fetch_all(&mut *tx)
        .await?;
        if active_actors
            .iter()
            .map(|(actor_id, _)| actor_id)
            .ne(required_actors.iter())
        {
            return Err(OrgIntelError::InvalidWork(
                "Work conversation target is no longer an active Actor".into(),
            ));
        }
        let locked_owner_team = active_actors
            .iter()
            .find(|(actor_id, _)| actor_id == &owner)
            .and_then(|(_, team_id)| *team_id);
        if locked_owner_team != owner_team_id {
            return Err(OrgIntelError::InvalidWork(
                "Work owner responsibility changed while conversation admission was being checked"
                    .into(),
            ));
        }
        if owner != to && accountable_lead.as_deref() != Some(to) {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work {work_id} belongs to {owner:?}; its accountable lead is {:?}, not message recipient {to:?}",
                accountable_lead
            )));
        }
        if let Some(expected_handoff) = expected_handoff {
            let current: Option<OwnerHandoffRow> = sqlx::query_as(
                "SELECT id,work_id,attempt_id,requested_by,category,requested_action, \
                        prepared_state,resume_condition,state,resolution,assigned_to, \
                        escalated_from,escalated_at,owner_brief,briefed_by,briefed_at, \
                        brief_source_fingerprint,delivered_at,created_at,resolved_at \
                 FROM owner_handoffs \
                 WHERE id=$1 AND work_id=$2 AND state='pending' AND assigned_to=$3 \
                 FOR SHARE",
            )
            .bind(expected_handoff.id)
            .bind(work_id)
            .bind(to)
            .fetch_optional(&mut *tx)
            .await?;
            if current
                .as_ref()
                .map(OwnerHandoffRow::conversation_input)
                .as_ref()
                != Some(expected_handoff)
            {
                return Err(OrgIntelError::InvalidWork(
                    "the expected Attention handoff changed, was resolved, or is no longer pending for this Actor"
                        .into(),
                ));
            }
        }
        if let Some((client_command_id, client_payload_sha256)) = idempotency {
            lock_owner_attachments_for_command_in_tx(
                &mut tx,
                from,
                to,
                client_command_id,
                client_payload_sha256,
                attachment_ids,
                body,
            )
            .await?;
        } else if !attachment_ids.is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "owner attachment staging requires an idempotent conversation command".into(),
            ));
        }
        let inserted: Option<i64> = if let Some((client_command_id, client_payload_sha256)) =
            idempotency
        {
            sqlx::query_scalar(
                "INSERT INTO messages \
                 (from_actor,to_actor,body,client_command_id,client_payload_sha256) \
                 VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING RETURNING id",
            )
            .bind(from)
            .bind(to)
            .bind(body)
            .bind(client_command_id.trim())
            .bind(client_payload_sha256)
            .fetch_optional(&mut *tx)
            .await?
        } else {
            Some(
                sqlx::query_scalar(
                    "INSERT INTO messages (from_actor,to_actor,body) VALUES ($1,$2,$3) RETURNING id",
                )
                .bind(from)
                .bind(to)
                .bind(body)
                .fetch_one(&mut *tx)
                .await?,
            )
        };
        let Some(id) = inserted else {
            let (message_id, prior_digest): (i64, String) = sqlx::query_as(
                "SELECT message.id,message.client_payload_sha256 \
                 FROM messages message JOIN rooms room ON room.id=message.room_id \
                 WHERE room.canonical_key=$1 AND message.from_actor=$2 \
                   AND message.client_command_id=$3",
            )
            .bind(&canonical_key)
            .bind(from)
            .bind(
                idempotency
                    .expect("only an idempotent insert can conflict")
                    .0
                    .trim(),
            )
            .fetch_one(&mut *tx)
            .await?;
            if prior_digest != idempotency.expect("idempotency exists").1 {
                return Err(OrgIntelError::RoomCommandConflict(
                    "conversation command was concurrently used with different semantics".into(),
                ));
            }
            tx.commit().await?;
            return Ok((message_id, false));
        };
        link_owner_attachments_in_tx(&mut tx, attachment_ids, id).await?;
        sqlx::query("INSERT INTO work_feedback (work_id, message_id, linked_by) VALUES ($1,$2,$3)")
            .bind(work_id)
            .bind(id)
            .bind(from)
            .execute(&mut *tx)
            .await?;
        // Owner feedback addressed to a live producer remains exact Attempt
        // input, but it also changes the accountable lead's mission. Route one
        // non-producing control notice in the same transaction rather than
        // relying on the worker to relay it or on a polling lead to discover it.
        if from == "owner" && to == owner {
            if let Some(lead) = accountable_lead.as_deref().filter(|lead| *lead != to) {
                let notice_body = format!(
                    "Material owner correction for Work {work_id}; owner message {id} is addressed to producer {owner}: {body}"
                );
                let notice_id: i64 = sqlx::query_scalar(
                    "INSERT INTO messages (from_actor,to_actor,body) \
                     VALUES ('daemon',$1,$2) RETURNING id",
                )
                .bind(lead)
                .bind(&notice_body)
                .fetch_one(&mut *tx)
                .await?;
                // This is a body-bearing coordination notice for the lead,
                // not a second producing input. The owner's source Message is
                // the sole `work_feedback` row and therefore enters the next
                // Attempt context exactly once even when owner and lead later
                // converge on the same Actor.
                sqlx::query(
                    "INSERT INTO events (kind,actor_id,body) \
                     VALUES ('owner_correction_routed','daemon',$1)",
                )
                .bind(serde_json::json!({
                    "work_id": work_id,
                    "source_message_id": id,
                    "notice_message_id": notice_id,
                    "producer_actor_id": owner,
                    "accountable_lead_id": lead,
                }))
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok((id, true))
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
                 AND COALESCE(feedback.routed_to_actor,message.to_actor)=work.owner_id \
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
        self.send_work_message_to_owner_internal(from, work_id, body, None)
            .await
    }

    /// Lease-fenced form used by a live accountable-lead conversation.
    pub async fn send_work_message_to_owner_with_cognitive_lease(
        &self,
        lease: &ActorCognitiveLease,
        work_id: Uuid,
        body: &str,
    ) -> Result<i64> {
        self.send_work_message_to_owner_internal(&lease.actor_id, work_id, body, Some(lease))
            .await
    }

    async fn send_work_message_to_owner_internal(
        &self,
        from: &str,
        work_id: Uuid,
        body: &str,
        lease: Option<&ActorCognitiveLease>,
    ) -> Result<i64> {
        if body.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "Work feedback message cannot be empty".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        if let Some(lease) = lease {
            validate_cognitive_lease_in_tx(&mut tx, lease).await?;
        } else {
            reject_unfenced_owner_output_in_tx(&mut tx, from).await?;
        }
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

    /// One verified human/producer direct-Room conversation linked to Work.
    /// The Work owner remains authoritative; the human Actor is attribution,
    /// not an implicit ownership or Authority grant.
    pub async fn human_work_conversation(
        &self,
        human_actor: &str,
        target_actor: &str,
        work_id: Uuid,
        limit: i64,
    ) -> Result<Vec<MessageRow>> {
        let owner: String = sqlx::query_scalar("SELECT owner_id FROM work WHERE id=$1")
            .bind(work_id)
            .fetch_one(&self.pool)
            .await?;
        if owner != target_actor {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work {work_id} belongs to {owner:?}, not conversation actor {target_actor:?}"
            )));
        }
        let Some(room_id) = self
            .active_human_direct_room_id(human_actor, target_actor)
            .await?
        else {
            return Ok(Vec::new());
        };
        Ok(sqlx::query_as(
            "SELECT id,from_actor,to_actor,body,outcome_standard,created_at,read_at FROM (\
               SELECT m.id,m.from_actor,m.to_actor,m.body,m.outcome_standard,m.created_at,m.read_at \
               FROM messages m JOIN work_feedback f ON f.message_id=m.id \
               WHERE f.work_id=$1 AND m.room_id=$2 \
               ORDER BY m.created_at DESC,m.id DESC LIMIT $3\
             ) recent ORDER BY created_at,id",
        )
        .bind(work_id)
        .bind(room_id)
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?)
    }

    /// Owner-specific compatibility facade for the established cockpit and
    /// agent context builders.
    pub async fn owner_work_conversation(
        &self,
        actor: &str,
        work_id: Uuid,
        limit: i64,
    ) -> Result<Vec<MessageRow>> {
        self.human_work_conversation("owner", actor, work_id, limit)
            .await
    }

    /// An actor's unread inbox (`None` = the owner's), oldest first.
    pub async fn inbox(&self, actor: Option<&str>) -> Result<Vec<MessageRow>> {
        Ok(sqlx::query_as(
            "SELECT id,from_actor,to_actor,body,outcome_standard,created_at,read_at FROM messages \
             WHERE read_at IS NULL \
               AND NOT EXISTS (SELECT 1 FROM message_mentions WHERE message_id=messages.id) \
               AND (\
               ($1::text IS NOT NULL AND (\
                 EXISTS (SELECT 1 FROM work_feedback feedback \
                         WHERE feedback.message_id=messages.id \
                           AND feedback.routed_to_actor=$1) \
                 OR (to_actor=$1 AND NOT EXISTS (\
                   SELECT 1 FROM work_feedback feedback \
                   WHERE feedback.message_id=messages.id \
                     AND feedback.routed_to_actor IS NOT NULL\
                 ))\
               )) OR \
               ($1::text IS NULL AND to_actor IS NULL AND (\
                 room_id IS NULL OR EXISTS (\
                   SELECT 1 FROM rooms WHERE rooms.id=messages.room_id AND rooms.kind='direct'\
                 )\
               ))\
             ) ORDER BY id",
        )
        .bind(actor)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Deterministic oldest-first input for one sovereign conversation turn.
    /// Count and aggregate UTF-8 byte bounds prevent individually valid
    /// messages from forming an un-runnable provider prompt. Work routing and
    /// classification stay inside this one query rather than an N+1 snapshot.
    pub async fn conversation_inbox_batch(
        &self,
        actor: &str,
        max_count: i64,
        max_bytes: i64,
    ) -> Result<Vec<MessageRow>> {
        let max_count = max_count.clamp(1, 128);
        let max_bytes = max_bytes.clamp(1, 1024 * 1024);
        Ok(sqlx::query_as(
            "WITH eligible AS (\
               SELECT message.id,message.from_actor,message.to_actor,message.body,\
                      message.outcome_standard,message.created_at,message.read_at,\
                      octet_length(message.body) AS body_bytes \
               FROM messages message \
               WHERE message.read_at IS NULL AND message.from_actor<>$1 \
                 AND NOT EXISTS (SELECT 1 FROM message_mentions mention \
                                 WHERE mention.message_id=message.id) \
                 AND (\
                   EXISTS (SELECT 1 FROM work_feedback feedback \
                           WHERE feedback.message_id=message.id \
                             AND feedback.routed_to_actor=$1) \
                   OR (message.to_actor=$1 AND NOT EXISTS (\
                     SELECT 1 FROM work_feedback feedback \
                     WHERE feedback.message_id=message.id \
                       AND feedback.routed_to_actor IS NOT NULL\
                   ))\
                 ) \
                 AND NOT EXISTS (\
                   SELECT 1 FROM work_feedback feedback JOIN work ON work.id=feedback.work_id \
                   WHERE feedback.message_id=message.id \
                     AND COALESCE(feedback.routed_to_actor,message.to_actor)=work.owner_id \
                     AND work.status IN ('proposed','active') \
                     AND NOT EXISTS (SELECT 1 FROM owner_handoffs handoff \
                                     WHERE handoff.work_id=work.id AND handoff.state='pending')\
                 ) \
               ORDER BY message.id LIMIT $2\
             ), candidates AS (\
               SELECT eligible.*,sum(body_bytes) OVER (ORDER BY id) AS cumulative_bytes \
               FROM eligible\
             ) \
             SELECT id,from_actor,to_actor,body,outcome_standard,created_at,read_at \
             FROM candidates WHERE cumulative_bytes<=$3 ORDER BY id",
        )
        .bind(actor)
        .bind(max_count)
        .bind(max_bytes)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Runtime's fixed prompt-admission envelope. Individual network messages
    /// are capped below this aggregate, so the oldest owed item always fits;
    /// additional input remains durable for the next successful turn.
    pub async fn conversation_inbox(&self, actor: &str) -> Result<Vec<MessageRow>> {
        self.conversation_inbox_batch(actor, 32, 128 * 1024).await
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
             WHERE message.read_at IS NULL AND message.from_actor<>$1 \
               AND NOT EXISTS (SELECT 1 FROM message_mentions WHERE message_id=message.id) \
               AND (\
                 EXISTS (SELECT 1 FROM work_feedback feedback \
                         WHERE feedback.message_id=message.id \
                           AND feedback.routed_to_actor=$1) \
                 OR (message.to_actor=$1 AND NOT EXISTS (\
                   SELECT 1 FROM work_feedback feedback \
                   WHERE feedback.message_id=message.id \
                     AND feedback.routed_to_actor IS NOT NULL\
                 ))\
               ) \
               AND NOT EXISTS (\
                 SELECT 1 FROM work_feedback feedback JOIN work ON work.id=feedback.work_id \
                 WHERE feedback.message_id=message.id \
                   AND COALESCE(feedback.routed_to_actor,message.to_actor)=work.owner_id \
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
            "SELECT id,from_actor,to_actor,body,outcome_standard,created_at,read_at FROM messages \
             WHERE read_at IS NULL \
               AND (\
                 EXISTS (SELECT 1 FROM work_feedback feedback \
                         WHERE feedback.message_id=messages.id \
                           AND feedback.routed_to_actor=$1) \
                 OR (to_actor=$1 AND NOT EXISTS (\
                   SELECT 1 FROM work_feedback feedback \
                   WHERE feedback.message_id=messages.id \
                     AND feedback.routed_to_actor IS NOT NULL\
                 ))\
               ) \
               AND NOT EXISTS (SELECT 1 FROM message_mentions WHERE message_id=messages.id) \
             ORDER BY id FOR UPDATE",
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

    async fn active_human_direct_room_id(
        &self,
        human_actor: &str,
        target_actor: &str,
    ) -> Result<Option<Uuid>> {
        let canonical_key = crate::rooms::direct_room_canonical_key(human_actor, target_actor);
        Ok(sqlx::query_scalar(
            "SELECT room.id FROM rooms room \
             JOIN room_participants human \
               ON human.room_id=room.id AND human.actor_id=$2 AND human.left_at IS NULL \
             JOIN actors human_actor ON human_actor.id=human.actor_id \
               AND human_actor.actor_class='human' AND human_actor.retired_at IS NULL \
             JOIN room_participants target \
               ON target.room_id=room.id AND target.actor_id=$3 AND target.left_at IS NULL \
             JOIN actors target_actor ON target_actor.id=target.actor_id \
               AND target_actor.retired_at IS NULL \
             WHERE room.canonical_key=$1 AND room.kind='direct' \
               AND room.archived_at IS NULL",
        )
        .bind(canonical_key)
        .bind(human_actor)
        .bind(target_actor)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Ordinary direct conversation between one authenticated human Actor and
    /// one company Actor, oldest first. Attribution and audience come from the
    /// canonical Room; the historical nullable-owner recipient is only a
    /// compatibility projection on those same Message rows.
    pub async fn human_conversation(
        &self,
        human_actor: &str,
        target_actor: &str,
        limit: i64,
    ) -> Result<Vec<MessageRow>> {
        let Some(room_id) = self
            .active_human_direct_room_id(human_actor, target_actor)
            .await?
        else {
            return Ok(Vec::new());
        };
        let limit = limit.clamp(1, 200);
        Ok(sqlx::query_as(
            "SELECT id,from_actor,to_actor,body,outcome_standard,created_at,read_at FROM (\
               SELECT id,from_actor,to_actor,body,outcome_standard,created_at,read_at \
               FROM messages WHERE room_id=$1 ORDER BY id DESC LIMIT $2\
             ) recent ORDER BY id",
        )
        .bind(room_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Room-scoped working-context boundary for a human/Actor direct
    /// conversation. Missing focus is the original uninterrupted context.
    pub async fn human_conversation_focus(
        &self,
        human_actor: &str,
        target_actor: &str,
    ) -> Result<ConversationFocusRow> {
        let Some(room_id) = self
            .active_human_direct_room_id(human_actor, target_actor)
            .await?
        else {
            return Ok(ConversationFocusRow {
                after_message_id: 0,
                started_at: None,
            });
        };
        Ok(sqlx::query_as(
            "SELECT COALESCE(after_message_id,0) AS after_message_id,started_at \
             FROM room_conversation_focus WHERE room_id=$1 AND target_actor_id=$2",
        )
        .bind(room_id)
        .bind(target_actor)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(ConversationFocusRow {
            after_message_id: 0,
            started_at: None,
        }))
    }

    /// Bounded direct-Room transcript newer than one per-human focus cursor.
    pub async fn human_conversation_since(
        &self,
        human_actor: &str,
        target_actor: &str,
        after_message_id: i64,
        limit: i64,
    ) -> Result<Vec<MessageRow>> {
        let Some(room_id) = self
            .active_human_direct_room_id(human_actor, target_actor)
            .await?
        else {
            return Ok(Vec::new());
        };
        let limit = limit.clamp(1, 200);
        Ok(sqlx::query_as(
            "SELECT id,from_actor,to_actor,body,outcome_standard,created_at,read_at FROM (\
               SELECT id,from_actor,to_actor,body,outcome_standard,created_at,read_at \
               FROM messages WHERE room_id=$1 AND id>$2 \
               ORDER BY id DESC LIMIT $3\
             ) recent ORDER BY id",
        )
        .bind(room_id)
        .bind(after_message_id.max(0))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Ordinary conversation between the owner and one actor, oldest first.
    /// This remains as the owner-specific compatibility facade.
    pub async fn owner_conversation(&self, actor: &str, limit: i64) -> Result<Vec<MessageRow>> {
        self.human_conversation("owner", actor, limit).await
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
        sqlx::query("SELECT id FROM actors WHERE id=$1 AND retired_at IS NULL FOR UPDATE")
            .bind(actor)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                OrgIntelError::InvalidWork(format!(
                    "active conversation actor {actor:?} does not exist"
                ))
            })?;
        sqlx::query(
            "UPDATE actor_cognitive_leases \
             SET revoked_at=COALESCE(revoked_at,now()),revoked_by='owner', \
                 revocation_reason='the owner interrupted the active conversation input' \
             WHERE actor_id=$1 AND claimed_until>now()",
        )
        .bind(actor)
        .execute(&mut *tx)
        .await?;
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
        self.human_conversation_since("owner", actor, after_message_id, limit)
            .await
    }

    pub async fn mark_read(&self, message_id: i64) -> Result<()> {
        sqlx::query("UPDATE messages SET read_at = now() WHERE id = $1")
            .bind(message_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

async fn validate_cognitive_lease_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    lease: &ActorCognitiveLease,
) -> Result<()> {
    let actor_available: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM actors \
         WHERE id=$1 AND retired_at IS NULL AND actor_class='agent' FOR UPDATE)",
    )
    .bind(&lease.actor_id)
    .fetch_one(&mut **tx)
    .await?;
    let lease_live: bool = actor_available
        && sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actor_cognitive_leases \
             WHERE actor_id=$1 AND lease_token=$2 AND claimed_until>now() \
               AND revoked_at IS NULL)",
        )
        .bind(&lease.actor_id)
        .bind(lease.token)
        .fetch_one(&mut **tx)
        .await?;
    if !lease_live {
        return Err(OrgIntelError::RoomCommandConflict(
            "the Actor cognitive-session lease is no longer current".into(),
        ));
    }
    Ok(())
}

async fn lock_owner_attachments_for_command_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    sender_actor_id: &str,
    target_actor_id: &str,
    client_command_id: &str,
    client_payload_sha256: &str,
    attachment_ids: &[Uuid],
    body: &str,
) -> Result<()> {
    if attachment_ids.is_empty() {
        return Ok(());
    }
    let mut expected = attachment_ids.to_vec();
    expected.sort_unstable();
    expected.dedup();
    if expected.len() != attachment_ids.len()
        || expected.iter().any(|attachment_id| {
            !body.contains(&format!(
                "/var/lib/restless-owner-attachments/{attachment_id}/content"
            ))
        })
    {
        return Err(OrgIntelError::InvalidRoom(
            "attachment identities do not match the recorded Message body".into(),
        ));
    }
    let locked: Vec<Uuid> = sqlx::query_scalar(
        "SELECT attachment_id FROM owner_attachments \
         WHERE attachment_id=ANY($1) AND sender_actor_id=$2 AND target_actor_id=$3 \
           AND client_command_id=$4 AND client_payload_sha256=$5 \
           AND message_id IS NULL AND gc_claimed_at IS NULL \
           AND purge_requested_at IS NULL AND purged_at IS NULL \
           AND gc_claim_token IS NULL \
         ORDER BY attachment_id FOR UPDATE",
    )
    .bind(&expected)
    .bind(sender_actor_id)
    .bind(target_actor_id)
    .bind(client_command_id.trim())
    .bind(client_payload_sha256)
    .fetch_all(&mut **tx)
    .await?;
    if locked != expected {
        return Err(OrgIntelError::RoomCommandConflict(
            "attachment staging was reclaimed or does not belong to this exact command".into(),
        ));
    }
    Ok(())
}

async fn link_owner_attachments_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    attachment_ids: &[Uuid],
    message_id: i64,
) -> Result<()> {
    if attachment_ids.is_empty() {
        return Ok(());
    }
    let linked = sqlx::query(
        "UPDATE owner_attachments SET message_id=$2,linked_at=now() \
         WHERE attachment_id=ANY($1) AND message_id IS NULL AND gc_claimed_at IS NULL \
           AND purge_requested_at IS NULL AND purged_at IS NULL",
    )
    .bind(attachment_ids)
    .bind(message_id)
    .execute(&mut **tx)
    .await?;
    if linked.rows_affected() != attachment_ids.len() as u64 {
        return Err(OrgIntelError::RoomCommandConflict(
            "attachment staging changed before the Message commit".into(),
        ));
    }
    Ok(())
}

/// Automatic owner-facing conversation output must cross the exact durable
/// lease fence. Productive Work and human/service messages have no cognitive
/// lease and retain the legacy path; a live agent conversation cannot bypass
/// its token through the ordinary CLI/message helper.
async fn reject_unfenced_owner_output_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: &str,
) -> Result<()> {
    sqlx::query("SELECT id FROM actors WHERE id=$1 FOR UPDATE")
        .bind(actor_id)
        .fetch_optional(&mut **tx)
        .await?;
    let fenced: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM actor_cognitive_leases WHERE actor_id=$1)")
            .bind(actor_id)
            .fetch_one(&mut **tx)
            .await?;
    if fenced {
        return Err(OrgIntelError::RoomCommandConflict(
            "a live Actor conversation may write to the owner only through its exact cognitive-session lease"
                .into(),
        ));
    }
    Ok(())
}

/// Move only unread Work-linked conversation inputs when accountable team
/// responsibility itself moves. Ordinary conversation and structured mentions
/// have their own explicit terminal cancellation policy; this narrow reroute
/// follows the Work and records the exact attribution change.
pub(crate) async fn reroute_unread_work_conversation_inputs_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    team_id: Uuid,
    previous_lead: &str,
    next_actor: &str,
    changed_by: &str,
    reason: &str,
) -> Result<u64> {
    let inputs = sqlx::query_as::<_, (i64, String, Uuid)>(
        "SELECT message.id,message.from_actor,feedback.work_id \
         FROM messages message \
         JOIN work_feedback feedback ON feedback.message_id=message.id \
         JOIN work ON work.id=feedback.work_id \
         JOIN actors owner_actor ON owner_actor.id=work.owner_id \
         WHERE COALESCE(feedback.routed_to_actor,message.to_actor)=$1 \
           AND message.read_at IS NULL \
           AND owner_actor.team_id=$2 \
           AND work.owner_id<>$1 \
           AND work.status IN ('proposed','active','blocked') \
         ORDER BY message.id,feedback.work_id FOR UPDATE OF message,feedback",
    )
    .bind(previous_lead)
    .bind(team_id)
    .fetch_all(&mut **tx)
    .await?;
    if inputs.is_empty() {
        return Ok(0);
    }
    for (message_id, _, work_id) in &inputs {
        sqlx::query(
            "UPDATE work_feedback \
             SET routed_to_actor=$3,routed_at=now(),routed_by=$4,route_reason=$5 \
             WHERE work_id=$1 AND message_id=$2",
        )
        .bind(work_id)
        .bind(message_id)
        .bind(next_actor)
        .bind(changed_by)
        .bind(reason)
        .execute(&mut **tx)
        .await?;
    }
    let message_ids = inputs
        .iter()
        .map(|(message_id, _, _)| *message_id)
        .collect::<Vec<_>>();
    sqlx::query("INSERT INTO events (kind,actor_id,body) VALUES ($1,$2,$3)")
        .bind("work.conversation.rerouted.v1")
        .bind(changed_by)
        .bind(serde_json::json!({
            "team_id": team_id,
            "from_actor_id": previous_lead,
            "to_actor_id": next_actor,
            "message_ids": message_ids,
            "inputs": inputs.into_iter().map(|(message_id, author_actor_id, work_id)| {
                serde_json::json!({
                    "message_id": message_id,
                    "author_actor_id": author_actor_id,
                    "work_id": work_id,
                })
            }).collect::<Vec<_>>(),
            "reason": reason,
        }))
        .execute(&mut **tx)
        .await?;
    Ok(message_ids.len() as u64)
}

/// Reassign still-unread, actor-directed inputs with their Work. The original
/// attributed Message is terminally consumed at its old audience and a daemon
/// routing notice carries its exact prose to the new owner, all with one
/// auditable event in the same Work transaction.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn reroute_unread_work_inputs_for_reassignment_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    work_id: Uuid,
    previous_owner: &str,
    next_owner: &str,
    previous_lead: Option<&str>,
    next_lead: Option<&str>,
    changed_by: &str,
    reason: &str,
) -> Result<u64> {
    let inputs = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT message.id,message.from_actor,\
                COALESCE(feedback.routed_to_actor,message.to_actor) AS effective_target \
         FROM messages message JOIN work_feedback feedback ON feedback.message_id=message.id \
         WHERE feedback.work_id=$1 AND message.read_at IS NULL \
           AND message.to_actor IS NOT NULL \
         ORDER BY message.id FOR UPDATE OF message,feedback",
    )
    .bind(work_id)
    .fetch_all(&mut **tx)
    .await?;
    if inputs.is_empty() {
        return Ok(0);
    }
    let fallback_lead = next_lead.unwrap_or("exec");
    let mut routed = Vec::new();
    for (message_id, author_actor_id, prior_target_actor_id) in &inputs {
        let next_target = if prior_target_actor_id == previous_owner {
            Some(next_owner)
        } else if previous_lead == Some(prior_target_actor_id.as_str())
            && previous_lead != next_lead
        {
            Some(fallback_lead)
        } else {
            None
        };
        let Some(next_target) = next_target.filter(|target| *target != prior_target_actor_id)
        else {
            continue;
        };
        sqlx::query(
            "UPDATE work_feedback \
             SET routed_to_actor=$3,routed_at=now(),routed_by=$4,route_reason=$5 \
             WHERE work_id=$1 AND message_id=$2",
        )
        .bind(work_id)
        .bind(message_id)
        .bind(next_target)
        .bind(changed_by)
        .bind(reason)
        .execute(&mut **tx)
        .await?;
        routed.push((
            *message_id,
            author_actor_id.clone(),
            prior_target_actor_id.clone(),
            next_target.to_string(),
        ));
    }
    if routed.is_empty() {
        return Ok(0);
    }
    let message_ids = routed
        .iter()
        .map(|(message_id, _, _, _)| *message_id)
        .collect::<Vec<_>>();
    sqlx::query("INSERT INTO events (kind,actor_id,body) VALUES ($1,$2,$3)")
        .bind("work.conversation.reassigned.v1")
        .bind(changed_by)
        .bind(serde_json::json!({
            "work_id": work_id,
            "from_owner_actor_id": previous_owner,
            "to_owner_actor_id": next_owner,
            "message_ids": message_ids,
            "inputs": routed.into_iter().map(|(message_id, author_actor_id, prior_target_actor_id, next_target_actor_id)| {
                serde_json::json!({
                    "message_id": message_id,
                    "author_actor_id": author_actor_id,
                    "prior_target_actor_id": prior_target_actor_id,
                    "next_target_actor_id": next_target_actor_id,
                })
            }).collect::<Vec<_>>(),
            "previous_accountable_lead_id": previous_lead,
            "next_accountable_lead_id": next_lead,
            "no_lead_fallback_actor_id": (next_lead.is_none()).then_some("exec"),
            "reason": reason,
        }))
        .execute(&mut **tx)
        .await?;
    Ok(message_ids.len() as u64)
}
