//! Owner handoffs, authored briefs, and outcome-review decisions.

use super::types::{owner_handoff_source_fingerprint, validate_owner_brief};
use super::*;

impl OrgIntel {
    pub async fn request_owner_handoff(&self, handoff: NewOwnerHandoff<'_>) -> Result<Uuid> {
        if handoff.requested_action.trim().is_empty()
            || handoff.prepared_state.trim().is_empty()
            || handoff.resume_condition.trim().is_empty()
        {
            return Err(OrgIntelError::InvalidWork(
                "owner handoff needs an exact action, prepared state and observable resume condition"
                    .into(),
            ));
        }
        let id = Uuid::new_v4();

        // Where this judgement goes (S06-T5). Only `owner_judgement` can be
        // answered by anyone other than the owner: identity, CAPTCHA, MFA, legal
        // attestation and payment confirmation are irreducibly human and stay
        // with the owner no matter how large the company grows. A lead absorbing
        // one of those would be a lead exercising authority it does not have.
        //
        // For judgement, the requester's team lead answers first. A lead or an
        // unassigned specialist asks the Exec. Only an Exec judgement reaches
        // the owner; nobody escalates to themselves.
        let assigned_to = if handoff.category == OwnerHandoffCategory::OwnerJudgement {
            if handoff.requested_by == "exec" {
                // Exec still performs the explicit final admission step. A
                // request cannot appear in owner Attention before its source
                // snapshot has a current authored brief.
                Some("exec".to_string())
            } else {
                self.team_lead_for(handoff.requested_by)
                    .await?
                    .or_else(|| Some("exec".to_string()))
            }
        } else {
            None
        };

        // The blocked Work says who it is waiting on, not just that it waits.
        // "awaiting owner handoff" on work a lead owes is how a queue becomes
        // invisible to the person who could clear it.
        let blocked_reason = match assigned_to.as_deref() {
            Some(lead) => format!("awaiting {lead} judgement, handoff {id}"),
            None => format!("awaiting owner handoff {id}"),
        };

        let mut tx = self.pool.begin().await?;
        // Serialize with scheduler claim. A handoff without an Attempt is
        // valid for a prepared legacy/manual outcome, but it must not detach
        // an already-running actor from the Attempt that still attributes its
        // process and any work it performs while the owner item is pending.
        sqlx::query("SELECT id FROM work WHERE id=$1 FOR UPDATE")
            .bind(handoff.work_id)
            .fetch_one(&mut *tx)
            .await?;
        let running_attempt = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM work_attempts WHERE work_id=$1 AND state='running' LIMIT 1",
        )
        .bind(handoff.work_id)
        .fetch_optional(&mut *tx)
        .await?;
        match (handoff.attempt_id, running_attempt) {
            (None, Some(_)) => {
                return Err(OrgIntelError::InvalidWork(
                    "a Work with a running Attempt must attach that Attempt to its owner handoff"
                        .into(),
                ));
            }
            (Some(attached), Some(running)) if attached == running => {}
            (Some(_), _) => {
                return Err(OrgIntelError::InvalidWork(
                    "handoff Attempt must be the running Attempt of this Work".into(),
                ));
            }
            (None, None) => {}
        }
        sqlx::query(
            "INSERT INTO owner_handoffs \
             (id, work_id, attempt_id, requested_by, category, requested_action, \
              prepared_state, resume_condition, assigned_to) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(handoff.work_id)
        .bind(handoff.attempt_id)
        .bind(handoff.requested_by)
        .bind(handoff.category)
        .bind(handoff.requested_action)
        .bind(handoff.prepared_state)
        .bind(handoff.resume_condition)
        .bind(assigned_to.as_deref())
        .execute(&mut *tx)
        .await?;
        // The Work is blocked, but an attached Attempt stays running until
        // its supervised process actually returns a terminal result. Closing
        // it here would make any preparation the still-live process performs
        // unattributed and would let the scheduler start a second process for
        // the same durable actor. Usually the actor now returns `blocked`; if
        // the owner responds while it is still live, that same Attempt may
        // continue and finish with the response observed.
        sqlx::query("UPDATE work SET status='blocked', resolution=$2 WHERE id=$1")
            .bind(handoff.work_id)
            .bind(&blocked_reason)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Refresh the prepared state of an outstanding handoff when the same Work
    /// has advanced after the owner request was first raised. The handoff stays
    /// pending and keeps its identity; only the exact outcome presented for the
    /// eventual decision changes. This is ordinary, attributable OrgIntel
    /// coordination, not an Authority mutation.
    pub async fn refresh_owner_handoff(
        &self,
        id: Uuid,
        changed_by: &str,
        requested_action: &str,
        prepared_state: &str,
        resume_condition: &str,
    ) -> Result<()> {
        if requested_action.trim().is_empty()
            || prepared_state.trim().is_empty()
            || resume_condition.trim().is_empty()
        {
            return Err(OrgIntelError::InvalidWork(
                "refreshing a handoff needs an exact action, prepared state and observable resume condition"
                    .into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT h.work_id, h.requested_action, h.prepared_state, h.resume_condition, \
                    w.owner_id \
             FROM owner_handoffs h JOIN work w ON w.id=h.work_id \
             WHERE h.id=$1 AND h.state='pending' FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("no outstanding handoff with that id".into()))?;
        let work_id: Uuid = row.get("work_id");
        let work_owner: String = row.get("owner_id");
        if !matches!(changed_by, "owner" | "exec") && changed_by != work_owner {
            return Err(OrgIntelError::InvalidWork(
                "only the Work owner, Exec or owner may refresh its prepared handoff".into(),
            ));
        }
        let previous_action: String = row.get("requested_action");
        let previous_prepared: String = row.get("prepared_state");
        let previous_resume: String = row.get("resume_condition");
        if previous_action == requested_action.trim()
            && previous_prepared == prepared_state.trim()
            && previous_resume == resume_condition.trim()
        {
            return Err(OrgIntelError::InvalidWork(
                "the prepared handoff is unchanged".into(),
            ));
        }

        sqlx::query(
            "UPDATE owner_handoffs SET requested_action=$2, prepared_state=$3, \
                    resume_condition=$4, delivered_at=NULL, \
                    assigned_to=CASE WHEN category='owner_judgement' AND assigned_to IS NULL \
                                     THEN 'exec' ELSE assigned_to END \
             WHERE id=$1",
        )
        .bind(id)
        .bind(requested_action.trim())
        .bind(prepared_state.trim())
        .bind(resume_condition.trim())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('owner_handoff_refreshed',$1,$2)",
        )
        .bind(changed_by)
        .bind(serde_json::json!({
            "handoff_id": id,
            "work_id": work_id,
            "previous_requested_action": previous_action,
            "requested_action": requested_action.trim(),
            "previous_prepared_state": previous_prepared,
            "prepared_state": prepared_state.trim(),
            "previous_resume_condition": previous_resume,
            "resume_condition": resume_condition.trim(),
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Attach or refresh the accountable actor's owner-altitude explanation.
    /// This does not move the handoff into the owner's queue; the Exec still
    /// performs the explicit final admission after checking the need.
    pub async fn prepare_owner_brief(
        &self,
        id: Uuid,
        briefed_by: &str,
        brief: OwnerBrief,
    ) -> Result<()> {
        validate_owner_brief(&brief)?;
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT h.work_id, h.attempt_id, h.category, h.requested_action, \
                    h.prepared_state, h.resume_condition, h.owner_brief, h.briefed_by, \
                    h.brief_source_fingerprint, h.assigned_to, w.owner_id, w.revision, \
                    t.lead_actor_id \
             FROM owner_handoffs h JOIN work w ON w.id=h.work_id \
             LEFT JOIN actors a ON a.id=w.owner_id \
             LEFT JOIN teams t ON t.id=a.team_id AND t.disbanded_at IS NULL \
             WHERE h.id=$1 AND h.state='pending' FOR UPDATE OF h, w",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("no outstanding handoff with that id".into()))?;
        let work_owner: String = row.get("owner_id");
        let team_lead: Option<String> = row.get("lead_actor_id");
        if briefed_by != "exec"
            && briefed_by != work_owner
            && team_lead.as_deref() != Some(briefed_by)
        {
            return Err(OrgIntelError::InvalidWork(
                "only the Work owner, its accountable lead or Exec may prepare its owner brief"
                    .into(),
            ));
        }
        let category: OwnerHandoffCategory = row.get("category");
        let kind_matches_boundary = match category {
            OwnerHandoffCategory::OwnerJudgement => brief.kind != OwnerBriefKind::HumanStep,
            OwnerHandoffCategory::Identity
            | OwnerHandoffCategory::Captcha
            | OwnerHandoffCategory::Mfa
            | OwnerHandoffCategory::LegalAttestation
            | OwnerHandoffCategory::PaymentConfirmation => brief.kind == OwnerBriefKind::HumanStep,
        };
        if !kind_matches_boundary {
            return Err(OrgIntelError::InvalidWork(
                "owner brief kind must preserve the handoff's judgement or irreducible-human boundary"
                    .into(),
            ));
        }
        let fingerprint = owner_handoff_source_fingerprint(
            row.get("work_id"),
            row.get("attempt_id"),
            category,
            row.get::<String, _>("requested_action").as_str(),
            row.get::<String, _>("prepared_state").as_str(),
            row.get::<String, _>("resume_condition").as_str(),
            row.get("revision"),
        );
        let previous_brief: Option<serde_json::Value> = row.get("owner_brief");
        let previous_author: Option<String> = row.get("briefed_by");
        let previous_fingerprint: Option<String> = row.get("brief_source_fingerprint");
        let retract_from_owner = category == OwnerHandoffCategory::OwnerJudgement
            && row.get::<Option<String>, _>("assigned_to").is_none();
        let encoded = serde_json::to_value(&brief)
            .map_err(|error| OrgIntelError::InvalidWork(format!("invalid owner brief: {error}")))?;
        if previous_brief.as_ref() == Some(&encoded)
            && previous_author.as_deref() == Some(briefed_by)
            && previous_fingerprint.as_deref() == Some(fingerprint.as_str())
        {
            return Err(OrgIntelError::InvalidWork(
                "the owner brief and its source snapshot are unchanged".into(),
            ));
        }
        sqlx::query(
            "UPDATE owner_handoffs SET owner_brief=$2, briefed_by=$3, briefed_at=now(), \
                    brief_source_fingerprint=$4, \
                    assigned_to=CASE WHEN $5 THEN 'exec' ELSE assigned_to END, \
                    escalated_from=CASE WHEN $5 THEN $3 ELSE escalated_from END, \
                    escalated_at=CASE WHEN $5 THEN now() ELSE escalated_at END \
             WHERE id=$1",
        )
        .bind(id)
        .bind(&encoded)
        .bind(briefed_by)
        .bind(&fingerprint)
        .bind(retract_from_owner)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('owner_brief_prepared',$1,$2)",
        )
        .bind(briefed_by)
        .bind(serde_json::json!({
            "handoff_id": id,
            "work_id": row.get::<Uuid, _>("work_id"),
            "source_fingerprint": fingerprint,
            "kind": brief.kind,
            "replaced_briefed_by": previous_author,
            "attention_retracted": retract_from_owner,
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn resolve_owner_handoff(
        &self,
        id: Uuid,
        state: OwnerHandoffState,
        resolution: &str,
    ) -> Result<()> {
        self.resolve_handoff(id, "owner", state, resolution, false)
            .await
            .map(|_| ())
    }

    /// Resolve an irreducible-human handoff from a live external observation.
    /// The observer is explicit in the transcript and feedback edge; this is
    /// never available for open-ended owner judgement or outcome review.
    /// Returns false when the exact handoff was already resolved, which makes
    /// repeated provider reconciliation harmless.
    pub async fn resolve_observed_handoff(
        &self,
        id: Uuid,
        observed_by: &str,
        resolution: &str,
    ) -> Result<bool> {
        self.resolve_handoff(
            id,
            observed_by,
            OwnerHandoffState::Resolved,
            resolution,
            true,
        )
        .await
    }

    /// Resolve judgement at the altitude that currently owes it, write the
    /// answer back as exact Work input, then release the blocked node. The
    /// return path is therefore observable by the next Attempt rather than
    /// ending as an isolated management answer.
    pub async fn resolve_handoff_as(
        &self,
        id: Uuid,
        resolved_by: &str,
        state: OwnerHandoffState,
        resolution: &str,
    ) -> Result<()> {
        self.resolve_handoff(id, resolved_by, state, resolution, false)
            .await
            .map(|_| ())
    }

    async fn resolve_handoff(
        &self,
        id: Uuid,
        resolved_by: &str,
        state: OwnerHandoffState,
        resolution: &str,
        external_observation: bool,
    ) -> Result<bool> {
        if state == OwnerHandoffState::Pending {
            return Err(OrgIntelError::InvalidWork(
                "a handoff cannot be resolved back to pending".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        if resolution.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "resolving a handoff needs the answer or observed outcome".into(),
            ));
        }
        let row = sqlx::query(
            "SELECT h.work_id, h.attempt_id, h.category, h.requested_action, \
                    h.prepared_state, h.resume_condition, h.assigned_to, h.owner_brief, \
                    h.brief_source_fingerprint, w.owner_id, w.revision \
             FROM owner_handoffs h JOIN work w ON w.id=h.work_id \
             WHERE h.id=$1 AND h.state='pending' FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            if external_observation {
                return Ok(false);
            }
            return Err(OrgIntelError::InvalidWork(
                "no pending owner handoff with that id".into(),
            ));
        };
        let work_id: Uuid = row.get("work_id");
        let assigned_to: Option<String> = row.get("assigned_to");
        let work_owner: String = row.get("owner_id");
        let category: OwnerHandoffCategory = row.get("category");
        let observable_human_step = external_observation
            && assigned_to.is_none()
            && category != OwnerHandoffCategory::OwnerJudgement;
        let resolver_owns = observable_human_step
            || assigned_to.as_deref() == Some(resolved_by)
            || (assigned_to.is_none() && resolved_by == "owner");
        if !resolver_owns {
            return Err(OrgIntelError::InvalidWork(format!(
                "handoff is not currently owed by {resolved_by:?}"
            )));
        }
        if external_observation && resolved_by != "daemon" {
            return Err(OrgIntelError::InvalidWork(
                "external owner-step observations must be attributed to daemon".into(),
            ));
        }
        if resolved_by == "owner" && category == OwnerHandoffCategory::OwnerJudgement {
            let brief = row
                .get::<Option<serde_json::Value>, _>("owner_brief")
                .and_then(|value| serde_json::from_value::<OwnerBrief>(value).ok())
                .ok_or_else(|| {
                    OrgIntelError::InvalidWork(
                        "owner decision needs a current authored owner brief".into(),
                    )
                })?;
            let current = owner_handoff_source_fingerprint(
                work_id,
                row.get("attempt_id"),
                category,
                row.get::<String, _>("requested_action").as_str(),
                row.get::<String, _>("prepared_state").as_str(),
                row.get::<String, _>("resume_condition").as_str(),
                row.get("revision"),
            );
            let recorded: Option<String> = row.get("brief_source_fingerprint");
            if recorded.as_deref() != Some(current.as_str()) {
                return Err(OrgIntelError::InvalidWork(
                    "owner decision brief is stale against the current handoff source".into(),
                ));
            }
            if brief.kind == OwnerBriefKind::OutcomeReview {
                return Err(OrgIntelError::InvalidWork(
                    "outcome_review must use accept or request-changes semantics".into(),
                ));
            }
        }
        sqlx::query(
            "UPDATE owner_handoffs SET state=$2, resolution=$3, resolved_at=now() WHERE id=$1",
        )
        .bind(id)
        .bind(state)
        .bind(resolution.trim())
        .execute(&mut *tx)
        .await?;
        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor,to_actor,body) VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(resolved_by)
        .bind(&work_owner)
        .bind(resolution.trim())
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO work_feedback (work_id,message_id,linked_by) VALUES ($1,$2,$3)")
            .bind(work_id)
            .bind(message_id)
            .bind(resolved_by)
            .execute(&mut *tx)
            .await?;
        let work_status = if state == OwnerHandoffState::Resolved {
            "active"
        } else {
            "blocked"
        };
        sqlx::query("UPDATE work SET status=$2::work_state, resolution=$3 WHERE id=$1")
            .bind(work_id)
            .bind(work_status)
            .bind(resolution)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }

    /// Resolve an owner-judgement handoff as a review of the exact prepared
    /// outcome. Accepting closes the Work; requesting changes advances the
    /// Work and its hard descendants to a new revision with the owner's exact
    /// feedback as kickoff context.
    pub async fn decide_owner_review(
        &self,
        id: Uuid,
        decision: OwnerReviewDecision,
        feedback: &str,
    ) -> Result<()> {
        let feedback = feedback.trim();
        if decision == OwnerReviewDecision::ChangesRequested && feedback.is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "requesting changes needs owner feedback for the next Attempt".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT h.work_id, h.attempt_id, h.category, h.requested_action, \
                    h.prepared_state, h.resume_condition, h.assigned_to, h.owner_brief, \
                    h.brief_source_fingerprint, w.owner_id, w.revision \
             FROM owner_handoffs h JOIN work w ON w.id=h.work_id \
             WHERE h.id=$1 AND h.state='pending' FOR UPDATE",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let work_id: Uuid = row.get("work_id");
        let category: OwnerHandoffCategory = row.get("category");
        let owner_id: String = row.get("owner_id");
        if category != OwnerHandoffCategory::OwnerJudgement {
            return Err(OrgIntelError::InvalidWork(
                "only an owner_judgement handoff can receive an outcome review".into(),
            ));
        }
        let assigned_to: Option<String> = row.get("assigned_to");
        if assigned_to.is_some() {
            return Err(OrgIntelError::InvalidWork(
                "the owner cannot review judgement that is still assigned below them".into(),
            ));
        }
        let brief: Option<serde_json::Value> = row.get("owner_brief");
        let brief = brief
            .and_then(|value| serde_json::from_value::<OwnerBrief>(value).ok())
            .ok_or_else(|| {
                OrgIntelError::InvalidWork(
                    "outcome review needs a current authored owner brief".into(),
                )
            })?;
        let current = owner_handoff_source_fingerprint(
            work_id,
            row.get("attempt_id"),
            category,
            row.get::<String, _>("requested_action").as_str(),
            row.get::<String, _>("prepared_state").as_str(),
            row.get::<String, _>("resume_condition").as_str(),
            row.get("revision"),
        );
        let recorded: Option<String> = row.get("brief_source_fingerprint");
        if recorded.as_deref() != Some(current.as_str()) {
            return Err(OrgIntelError::InvalidWork(
                "outcome review brief is stale against the current handoff source".into(),
            ));
        }
        if brief.kind != OwnerBriefKind::OutcomeReview {
            return Err(OrgIntelError::InvalidWork(
                "only an outcome_review brief has accept/request-changes semantics".into(),
            ));
        }
        let has_running_attempt: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work_attempts WHERE work_id=$1 AND state='running')",
        )
        .bind(work_id)
        .fetch_one(&mut *tx)
        .await?;
        if has_running_attempt {
            return Err(OrgIntelError::InvalidWork(
                "cannot decide an outcome review while its Work Attempt is still running".into(),
            ));
        }

        sqlx::query(
            "INSERT INTO actors (id, kind, role, display) VALUES ('owner','owner','owner','The Owner') \
             ON CONFLICT (id) DO NOTHING",
        )
        .execute(&mut *tx)
        .await?;
        if !feedback.is_empty() {
            let existing_message: Option<i64> = sqlx::query_scalar(
                "SELECT m.id FROM messages m JOIN work_feedback f ON f.message_id=m.id \
                 WHERE f.work_id=$1 AND m.from_actor='owner' AND m.to_actor=$2 AND m.body=$3 \
                   AND NOT EXISTS (SELECT 1 FROM work_attempt_feedback af WHERE af.message_id=m.id) \
                 ORDER BY m.id DESC LIMIT 1",
            )
            .bind(work_id)
            .bind(&owner_id)
            .bind(feedback)
            .fetch_optional(&mut *tx)
            .await?;
            if existing_message.is_none() {
                let message_id: i64 = sqlx::query_scalar(
                    "INSERT INTO messages (from_actor,to_actor,body) \
                     VALUES ('owner',$1,$2) RETURNING id",
                )
                .bind(&owner_id)
                .bind(feedback)
                .fetch_one(&mut *tx)
                .await?;
                sqlx::query(
                    "INSERT INTO work_feedback (work_id,message_id,linked_by) \
                     VALUES ($1,$2,'owner')",
                )
                .bind(work_id)
                .bind(message_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        let (handoff_state, resolution) = match decision {
            OwnerReviewDecision::Accepted => (
                OwnerHandoffState::Resolved,
                if feedback.is_empty() {
                    "Owner accepted the prepared outcome".to_string()
                } else {
                    format!("Owner accepted the prepared outcome: {feedback}")
                },
            ),
            OwnerReviewDecision::ChangesRequested => (
                OwnerHandoffState::Declined,
                format!("Owner requested changes: {feedback}"),
            ),
        };
        sqlx::query(
            "UPDATE owner_handoffs SET state=$2,resolution=$3,resolved_at=now() WHERE id=$1",
        )
        .bind(id)
        .bind(handoff_state)
        .bind(&resolution)
        .execute(&mut *tx)
        .await?;
        match decision {
            OwnerReviewDecision::Accepted => {
                sqlx::query(
                    "UPDATE work SET status='completed',resolution=$2,updated_at=now() WHERE id=$1",
                )
                .bind(work_id)
                .bind(&resolution)
                .execute(&mut *tx)
                .await?;
            }
            OwnerReviewDecision::ChangesRequested => {
                invalidate_from(&mut tx, work_id, "owner", feedback).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_owner_handoffs(&self) -> Result<Vec<OwnerHandoffRow>> {
        Ok(sqlx::query_as(
            "SELECT id, work_id, attempt_id, requested_by, category, requested_action, \
                    prepared_state, resume_condition, state, resolution, assigned_to, \
                    escalated_from, escalated_at, owner_brief, briefed_by, briefed_at, \
                    brief_source_fingerprint, delivered_at, created_at, resolved_at \
             FROM owner_handoffs ORDER BY created_at, id",
        )
        .fetch_all(&self.pool)
        .await?)
    }
}
