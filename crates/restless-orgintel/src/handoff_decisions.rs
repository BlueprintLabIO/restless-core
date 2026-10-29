//! The Undo window belongs to the handoff's company, not a browser timer.
//! No effect is recorded until the original deadline, and the existing handoff
//! transaction writes both the decision and its receipt.
use super::*;
use crate::human_decisions::check_decision_principal_in_tx;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HandoffDecision {
    CompleteHumanStep,
    Skip,
    AcceptReview { feedback: String },
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingHandoffDecision {
    pub id: Uuid,
    pub handoff_id: Uuid,
    #[sqlx(json)]
    pub principal: HumanDecisionPrincipal,
    #[sqlx(json)]
    pub expected_version: OwnerReviewVersion,
    #[sqlx(json)]
    pub decision: HandoffDecision,
    pub undo_until: DateTime<Utc>,
    pub state: String,
    pub problem: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub seen_at: Option<DateTime<Utc>>,
}

pub(super) async fn handoff_version_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    row: &OwnerHandoffRow,
    work_revision: i64,
) -> Result<OwnerReviewVersion> {
    let artifact_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM artifact_refs WHERE work_id=$1 AND state='available' \
         AND kind='review_target' AND ($2::uuid IS NULL OR attempt_id=$2) \
         ORDER BY created_at DESC,id DESC LIMIT 1",
    )
    .bind(row.work_id)
    .bind(row.attempt_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(OwnerReviewVersion {
        handoff: row.conversation_input(),
        work_revision,
        artifact_id,
    })
}

fn same_request(
    saved: &PendingHandoffDecision,
    principal: &HumanDecisionPrincipal,
    expected: &OwnerReviewVersion,
    decision: &HandoffDecision,
) -> bool {
    saved.principal == *principal
        && saved.expected_version == *expected
        && same_decision(&saved.decision, decision)
}

async fn remember_request(
    tx: &mut Transaction<'_, Postgres>,
    request: Uuid,
    decision: Uuid,
) -> Result<()> {
    sqlx::query("INSERT INTO owner_handoff_decision_requests(id,decision_id) VALUES($1,$2) ON CONFLICT(id) DO NOTHING")
        .bind(request).bind(decision).execute(&mut **tx).await?;
    let saved: Uuid =
        sqlx::query_scalar("SELECT decision_id FROM owner_handoff_decision_requests WHERE id=$1")
            .bind(request)
            .fetch_one(&mut **tx)
            .await?;
    if saved != decision {
        return Err(OrgIntelError::InvalidWork(
            "This decision ID already names a different request.".into(),
        ));
    }
    Ok(())
}

impl OrgIntel {
    /// Save one exact human decision before the client leaves its card. Replays
    /// return its original deadline; a second client cannot extend the window.
    pub async fn queue_handoff_decision(
        &self,
        id: Uuid,
        principal: &HumanDecisionPrincipal,
        expected: &OwnerReviewVersion,
        decision: &HandoffDecision,
    ) -> Result<PendingHandoffDecision> {
        if let HandoffDecision::AcceptReview { feedback } = decision {
            if feedback.len() > 64 * 1024 {
                return Err(OrgIntelError::InvalidWork(
                    "Review feedback must not exceed 64 KiB.".into(),
                ));
            }
        }
        let mut tx = self.pool.begin().await?;
        if let Some(saved) = sqlx::query_as::<_, PendingHandoffDecision>(
            "SELECT * FROM owner_handoff_decisions WHERE id=COALESCE((SELECT decision_id FROM owner_handoff_decision_requests WHERE id=$1),$1)",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        {
            if !same_request(&saved, principal, expected, decision) {
                return Err(OrgIntelError::InvalidWork(
                    "This decision ID already names a different request.".into(),
                ));
            }
            check_decision_principal_in_tx(&mut tx, principal).await?;
            remember_request(&mut tx, id, saved.id).await?;
            tx.commit().await?;
            return Ok(saved);
        }
        let handoff_id = expected.handoff.id;
        let work_id: Uuid = sqlx::query_scalar("SELECT work_id FROM owner_handoffs WHERE id=$1")
            .bind(handoff_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                OrgIntelError::InvalidWork("This request is no longer waiting.".into())
            })?;
        let accountability =
            crate::actors::lock_work_accountability_for_update_in_tx(&mut tx, work_id).await?;
        let row = sqlx::query_as::<_, OwnerHandoffRow>(
            "SELECT * FROM owner_handoffs WHERE id=$1 AND state='pending' FOR UPDATE",
        )
        .bind(handoff_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("This request is no longer waiting.".into()))?;
        check_decision_principal_in_tx(&mut tx, principal).await?;
        let owns = row
            .assigned_to
            .as_deref()
            .map_or(principal.resolver() == "owner", |assigned| {
                assigned == principal.resolver()
                    && (!matches!(decision, HandoffDecision::AcceptReview { .. })
                        || accountability.responsible_actor_id.as_deref() == Some(assigned))
            });
        if !owns {
            return Err(OrgIntelError::InvalidWork(
                "This request is waiting on someone else.".into(),
            ));
        }
        if handoff_version_in_tx(&mut tx, &row, accountability.revision).await? != *expected {
            return Err(OrgIntelError::ReviewChanged);
        }
        match decision {
            HandoffDecision::CompleteHumanStep
                if row.category == OwnerHandoffCategory::OwnerJudgement =>
            {
                return Err(OrgIntelError::InvalidWork(
                    "Use this review's decision rather than Mark done.".into(),
                ));
            }
            HandoffDecision::Skip
                if row
                    .owner_brief
                    .as_ref()
                    .is_some_and(|brief| brief.kind == OwnerBriefKind::OutcomeReview) =>
            {
                return Err(OrgIntelError::InvalidWork(
                    "Use the review actions for this prepared outcome.".into(),
                ));
            }
            HandoffDecision::AcceptReview { .. } => {
                let running: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM work_attempts WHERE work_id=$1 AND state='running')")
                    .bind(work_id).fetch_one(&mut *tx).await?;
                if running {
                    return Err(OrgIntelError::InvalidWork(
                        "The outcome is still being prepared.".into(),
                    ));
                }
                if row.category != OwnerHandoffCategory::OwnerJudgement
                    || !row.owner_brief_is_current(accountability.revision)
                    || !row
                        .owner_brief
                        .as_ref()
                        .is_some_and(|brief| brief.kind == OwnerBriefKind::OutcomeReview)
                    || expected.artifact_id.is_none()
                {
                    return Err(OrgIntelError::InvalidWork(
                        "Open a current review with its prepared preview before accepting.".into(),
                    ));
                }
            }
            _ => {}
        }
        // The Work/handoff lock also serializes a same-ID replay that started
        // before the first insert and a different client's equivalent click.
        if let Some(saved) = sqlx::query_as::<_, PendingHandoffDecision>(
            "SELECT * FROM owner_handoff_decisions WHERE id=COALESCE((SELECT decision_id FROM owner_handoff_decision_requests WHERE id=$1),$1) OR (handoff_id=$2 AND state='pending') ORDER BY created_at LIMIT 1",
        ).bind(id).bind(handoff_id).fetch_optional(&mut *tx).await? {
            if !same_request(&saved, principal, expected, decision) {
                return Err(OrgIntelError::InvalidWork("Another decision is already waiting for Undo.".into()));
            }
            remember_request(&mut tx, id, saved.id).await?;
            tx.commit().await?;
            return Ok(saved);
        }
        let saved = sqlx::query_as::<_, PendingHandoffDecision>(
            "INSERT INTO owner_handoff_decisions(id,handoff_id,principal,expected_version,decision) \
             VALUES($1,$2,$3,$4,$5) RETURNING *",
        )
        .bind(id).bind(handoff_id)
        .bind(serde_json::to_value(principal).expect("serializable decision principal"))
        .bind(serde_json::to_value(expected).expect("serializable displayed version"))
        .bind(serde_json::to_value(decision).expect("serializable handoff decision"))
        .fetch_one(&mut *tx).await?;
        remember_request(&mut tx, id, saved.id).await?;
        notify_decision_changed(&mut tx).await?;
        tx.commit().await?;
        Ok(saved)
    }

    pub async fn cancel_handoff_decision(
        &self,
        id: Uuid,
        principal: &HumanDecisionPrincipal,
    ) -> Result<PendingHandoffDecision> {
        let mut tx = self.pool.begin().await?;
        let saved = sqlx::query_as::<_, PendingHandoffDecision>(
            "SELECT * FROM owner_handoff_decisions WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("This decision was not found.".into()))?;
        if saved.principal != *principal {
            return Err(OrgIntelError::InvalidWork(
                "This decision belongs to someone else or an older membership.".into(),
            ));
        }
        check_decision_principal_in_tx(&mut tx, principal).await?;
        if saved.state == "canceled" {
            tx.commit().await?;
            return Ok(saved);
        }
        let saved = sqlx::query_as::<_, PendingHandoffDecision>(
            "UPDATE owner_handoff_decisions SET state='canceled',completed_at=clock_timestamp() \
             WHERE id=$1 AND state='pending' AND undo_until>clock_timestamp() RETURNING *",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("The Undo window has ended.".into()))?;
        notify_decision_changed(&mut tx).await?;
        tx.commit().await?;
        Ok(saved)
    }

    pub async fn due_handoff_decisions(&self) -> Result<Vec<PendingHandoffDecision>> {
        Ok(sqlx::query_as(
            "SELECT * FROM owner_handoff_decisions WHERE state='pending' AND undo_until<=clock_timestamp() \
             ORDER BY undo_until,id LIMIT 50",
        ).fetch_all(&self.pool).await?)
    }

    pub async fn handoff_decisions_for_actor(
        &self,
        actor_id: &str,
    ) -> Result<Vec<PendingHandoffDecision>> {
        Ok(sqlx::query_as(
            "SELECT * FROM owner_handoff_decisions WHERE principal->>'actor_id'=$1 \
             AND (state='pending' OR seen_at IS NULL) \
             ORDER BY created_at DESC LIMIT 100",
        )
        .bind(actor_id)
        .fetch_all(&self.pool)
        .await?)
    }
}

pub(super) async fn admit_pending_handoff_decision_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    handoff: &OwnerHandoffRow,
    work_revision: i64,
    actor: &str,
    decision: &HandoffDecision,
) -> Result<()> {
    let saved = sqlx::query_as::<_, PendingHandoffDecision>(
        "SELECT * FROM owner_handoff_decisions WHERE id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OrgIntelError::InvalidWork("This pending decision was not found.".into()))?;
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    if saved.state != "pending" || saved.undo_until > now {
        return Err(OrgIntelError::InvalidWork(
            "This decision was canceled, settled, or is still in its Undo window.".into(),
        ));
    }
    if saved.handoff_id != handoff.id
        || saved.principal.resolver() != actor
        || !same_decision(&saved.decision, decision)
    {
        return Err(OrgIntelError::InvalidWork(
            "This pending decision names a different action.".into(),
        ));
    }
    check_decision_principal_in_tx(tx, &saved.principal).await?;
    if handoff_version_in_tx(tx, handoff, work_revision).await? != saved.expected_version {
        return Err(OrgIntelError::ReviewChanged);
    }
    Ok(())
}

pub(super) async fn complete_pending_handoff_decision_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<()> {
    let changed = sqlx::query(
        "UPDATE owner_handoff_decisions SET state='applied',completed_at=clock_timestamp() \
         WHERE id=$1 AND state='pending'",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(OrgIntelError::InvalidWork(
            "This decision is no longer pending.".into(),
        ));
    }
    notify_decision_changed(tx).await?;
    Ok(())
}

fn same_decision(a: &HandoffDecision, b: &HandoffDecision) -> bool {
    match (a, b) {
        (
            HandoffDecision::AcceptReview { feedback: a },
            HandoffDecision::AcceptReview { feedback: b },
        ) => a.trim() == b.trim(),
        _ => a == b,
    }
}

impl OrgIntel {
    /// Multiple workers may discover the same due row. The canonical Work /
    /// handoff transaction locks it, rechecks access/version/deadline, and
    /// records the receipt with the mutation. A crash cannot split the two.
    pub async fn apply_handoff_decision(&self, id: Uuid) -> Result<bool> {
        let Some(saved) = sqlx::query_as::<_, PendingHandoffDecision>(
            "SELECT * FROM owner_handoff_decisions WHERE id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Err(OrgIntelError::InvalidWork(
                "This pending decision was not found.".into(),
            ));
        };
        if saved.state != "pending" {
            return Ok(false);
        }
        let result = match &saved.decision {
            HandoffDecision::CompleteHumanStep => self.resolve_handoff(
                saved.handoff_id, saved.principal.resolver(),
                OwnerHandoffState::Resolved, "The person completed their prepared step.",
                false, true, false, Some(id),
            ).await.map(|_| ()),
            HandoffDecision::Skip => self.resolve_handoff(
                saved.handoff_id, saved.principal.resolver(),
                OwnerHandoffState::Declined,
                "Skipped: not needed. Carry on without it. Ask me only if skipping changes what you can deliver.",
                false, false, false, Some(id),
            ).await.map(|_| ()),
            HandoffDecision::AcceptReview { feedback } => self.decide_review(
                saved.handoff_id, saved.principal.resolver(),
                OwnerReviewDecision::Accepted, feedback.as_str(),
                false, Some(&saved.expected_version), Some(id),
            ).await,
        };
        if let Err(error) = result {
            // A concurrent worker/Undo may have won while we waited for the
            // canonical Work lock. Its receipt settles this replay too.
            let state: String =
                sqlx::query_scalar("SELECT state FROM owner_handoff_decisions WHERE id=$1")
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await?;
            if state == "applied" || state == "canceled" {
                return Ok(false);
            }
            return Err(error);
        }
        Ok(true)
    }

    /// A refused commit restores the still-current request to the person's
    /// Inbox. Never overwrite another worker's receipt or a successful Undo.
    pub async fn fail_handoff_decision(&self, id: Uuid, problem: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let problem: String = problem.chars().take(2000).collect();
        sqlx::query(
            "UPDATE owner_handoff_decisions SET state='failed',problem=$2,completed_at=clock_timestamp() \
             WHERE id=$1 AND state='pending'",
        ).bind(id).bind(problem).execute(&mut *tx).await?;
        notify_decision_changed(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }
}

impl OrgIntel {
    /// Receipt acknowledgement changes no Work or permission. A current session
    /// may read/acknowledge its Actor's older membership receipts.
    pub async fn acknowledge_handoff_decision(
        &self,
        id: Uuid,
        principal: &HumanDecisionPrincipal,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let saved = sqlx::query_as::<_, PendingHandoffDecision>(
            "SELECT * FROM owner_handoff_decisions WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("This decision was not found.".into()))?;
        check_decision_principal_in_tx(&mut tx, principal).await?;
        if !saved.principal.same_person(principal) || saved.state == "pending" {
            return Err(OrgIntelError::InvalidWork(
                "This result is not yours to acknowledge.".into(),
            ));
        }
        sqlx::query("UPDATE owner_handoff_decisions SET seen_at=COALESCE(seen_at,clock_timestamp()) WHERE id=$1")
            .bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}

/// Body-free invalidation, delivered only when the decision transaction commits.
async fn notify_decision_changed(tx: &mut Transaction<'_, Postgres>) -> Result<()> {
    sqlx::query("SELECT pg_notify('restless_orgintel', json_build_object('company',current_schema(),'kind','handoff_decision_changed','body',json_build_object())::text)")
        .execute(&mut **tx).await?;
    Ok(())
}
