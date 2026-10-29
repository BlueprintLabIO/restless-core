//! A person's bounded Undo on Native Document decisions. The document owns the
//! version/access checks and applies its change with this receipt in one transaction.
use super::*;
use crate::documents::{
    self, DocumentAccess, DocumentCollaborationRequest, DocumentError, DocumentResult,
    DocumentReviewRow, DocumentReviewStatus,
};
use crate::human_decisions::check_decision_principal_in_tx;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DocumentDecision {
    AcceptReview {
        expected_document_version: i64,
        expected_review_version: i64,
        accepted_version_name: String,
        feedback: String,
    },
    CompleteCollaboration {
        expected_document_version: i64,
        expected_summary: String,
    },
}
impl DocumentDecision {
    fn document_version(&self) -> i64 {
        match self {
            Self::AcceptReview {
                expected_document_version,
                ..
            }
            | Self::CompleteCollaboration {
                expected_document_version,
                ..
            } => *expected_document_version,
        }
    }
}
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingDocumentDecision {
    pub id: Uuid,
    pub document_id: Uuid,
    pub request_id: Uuid,
    #[sqlx(json)]
    pub principal: HumanDecisionPrincipal,
    #[sqlx(json)]
    pub decision: DocumentDecision,
    pub request_snapshot: serde_json::Value,
    pub comment_command_id: Uuid,
    pub undo_until: DateTime<Utc>,
    pub state: String,
    pub problem: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub seen_at: Option<DateTime<Utc>>,
}
async fn principal_current(
    tx: &mut Transaction<'_, Postgres>,
    principal: &HumanDecisionPrincipal,
) -> DocumentResult<()> {
    check_decision_principal_in_tx(tx, principal)
        .await
        .map_err(|error| DocumentError::Conflict(error.plain_message()))
}
fn changed() -> DocumentError {
    DocumentError::Conflict("This request changed. Open it again before deciding.".into())
}
async fn request_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    document: Uuid,
    request: Uuid,
    principal: &HumanDecisionPrincipal,
    decision: &DocumentDecision,
) -> DocumentResult<serde_json::Value> {
    let version: Option<i64> = sqlx::query_scalar(
        "SELECT version FROM native_documents WHERE id=$1 AND status<>'archived' FOR UPDATE",
    )
    .bind(document)
    .fetch_optional(&mut **tx)
    .await?;
    if version != Some(decision.document_version()) {
        return Err(changed());
    }
    let dependency = if matches!(decision, DocumentDecision::AcceptReview { .. }) {
        documents::load_work_review_dependency(tx, document, request, true).await?
    } else {
        None
    };
    if let Some(dependency) = &dependency {
        if dependency.reviewer_actor_id != principal.actor_id
            || dependency.status != crate::DocumentWorkReviewStatus::Pending
        {
            return Err(DocumentError::Unavailable);
        }
        // Keep Work/Team/Actor locks ahead of the human membership lock.
        documents::require_pending_document_work_ready(tx, dependency).await?;
    }
    principal_current(tx, principal).await?;
    match decision {
        DocumentDecision::AcceptReview {
            expected_review_version,
            accepted_version_name,
            feedback,
            ..
        } => {
            documents::clean_bounded("accepted version name", accepted_version_name, 500)?;
            if !feedback.trim().is_empty() {
                documents::clean_bounded("review feedback", feedback, 4000)?;
            }
            documents::require_document_judgement_actor(tx, &principal.actor_id).await?;
            documents::require_access(tx, document, &principal.actor_id, DocumentAccess::Edit)
                .await?;
            let row = sqlx::query_as::<_, DocumentReviewRow>(&format!(
                "{} WHERE document_id=$1 AND id=$2 FOR UPDATE",
                documents::review_select()
            ))
            .bind(document)
            .bind(request)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(DocumentError::Unavailable)?;
            if row.status != DocumentReviewStatus::Requested
                || row.version != *expected_review_version
                || row
                    .reviewer_actor_id
                    .as_ref()
                    .is_some_and(|actor| actor != &principal.actor_id)
            {
                return Err(changed());
            }
            let current: Uuid = sqlx::query_scalar(
                "SELECT current_named_version_id FROM native_documents WHERE id=$1",
            )
            .bind(document)
            .fetch_one(&mut **tx)
            .await?;
            if current != row.requested_version_id {
                return Err(changed());
            }
            if dependency.as_ref().is_some_and(|dependency| {
                dependency.reviewer_actor_id != principal.actor_id
                    || dependency.status != crate::DocumentWorkReviewStatus::Pending
            }) {
                return Err(changed());
            }
            Ok(serde_json::json!({"review":row,"work_dependency":dependency}))
        }
        DocumentDecision::CompleteCollaboration {
            expected_summary, ..
        } => {
            documents::require_access(tx, document, &principal.actor_id, DocumentAccess::Read)
                .await?;
            let row=sqlx::query_as::<_,DocumentCollaborationRequest>("SELECT * FROM native_document_collaboration_requests WHERE document_id=$1 AND id=$2 FOR UPDATE").bind(document).bind(request).fetch_optional(&mut **tx).await?.ok_or(DocumentError::Unavailable)?;
            if row.resolved_at.is_some()
                || row.requested_owner_actor_id != principal.actor_id
                || row.summary != *expected_summary
            {
                return Err(changed());
            }
            Ok(serde_json::json!(row))
        }
    }
}
async fn notify(tx: &mut Transaction<'_, Postgres>) -> DocumentResult<()> {
    sqlx::query("SELECT pg_notify('restless_orgintel',json_build_object('company',current_schema(),'kind','document_decision_changed','body',json_build_object())::text)").execute(&mut **tx).await?;
    Ok(())
}
fn same(
    saved: &PendingDocumentDecision,
    document: Uuid,
    request: Uuid,
    principal: &HumanDecisionPrincipal,
    decision: &DocumentDecision,
) -> bool {
    saved.document_id == document
        && saved.request_id == request
        && saved.principal == *principal
        && saved.decision == *decision
}
async fn remember_request(
    tx: &mut Transaction<'_, Postgres>,
    request: Uuid,
    decision: Uuid,
) -> DocumentResult<()> {
    sqlx::query("INSERT INTO owner_document_decision_requests(id,decision_id) VALUES($1,$2) ON CONFLICT(id) DO NOTHING")
        .bind(request).bind(decision).execute(&mut **tx).await?;
    let saved: Uuid =
        sqlx::query_scalar("SELECT decision_id FROM owner_document_decision_requests WHERE id=$1")
            .bind(request)
            .fetch_one(&mut **tx)
            .await?;
    if saved != decision {
        return Err(changed());
    }
    Ok(())
}

impl OrgIntel {
    pub async fn queue_document_decision(
        &self,
        id: Uuid,
        document: Uuid,
        request: Uuid,
        principal: &HumanDecisionPrincipal,
        decision: &DocumentDecision,
    ) -> DocumentResult<PendingDocumentDecision> {
        let mut tx = self.pool.begin().await?;
        if let Some(saved) = sqlx::query_as::<_, PendingDocumentDecision>(
            "SELECT * FROM owner_document_decisions WHERE id=COALESCE((SELECT decision_id FROM owner_document_decision_requests WHERE id=$1),$1)",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        {
            if !same(&saved, document, request, principal, decision) {
                return Err(changed());
            }
            principal_current(&mut tx, principal).await?;
            remember_request(&mut tx, id, saved.id).await?;
            tx.commit().await?;
            return Ok(saved);
        }
        let snapshot = request_snapshot(&mut tx, document, request, principal, decision).await?;
        if let Some(saved)=sqlx::query_as::<_,PendingDocumentDecision>("SELECT * FROM owner_document_decisions WHERE id=COALESCE((SELECT decision_id FROM owner_document_decision_requests WHERE id=$1),$1) OR(document_id=$2 AND request_id=$3 AND state='pending' AND decision->>'kind'=$4) ORDER BY created_at LIMIT 1")
            .bind(id).bind(document).bind(request).bind(match decision{DocumentDecision::AcceptReview{..}=>"accept_review",DocumentDecision::CompleteCollaboration{..}=>"complete_collaboration"}).fetch_optional(&mut *tx).await? {
            if !same(&saved,document,request,principal,decision){return Err(changed());}
            remember_request(&mut tx,id,saved.id).await?;tx.commit().await?;return Ok(saved);
        }
        let saved=sqlx::query_as::<_,PendingDocumentDecision>("INSERT INTO owner_document_decisions(id,document_id,request_id,principal,decision,request_snapshot,comment_command_id) VALUES($1,$2,$3,$4,$5,$6,$7) RETURNING *")
            .bind(id).bind(document).bind(request).bind(sqlx::types::Json(principal)).bind(sqlx::types::Json(decision)).bind(snapshot).bind(Uuid::new_v4()).fetch_one(&mut *tx).await?;
        remember_request(&mut tx, id, saved.id).await?;
        notify(&mut tx).await?;
        tx.commit().await?;
        Ok(saved)
    }
    pub async fn cancel_document_decision(
        &self,
        id: Uuid,
        principal: &HumanDecisionPrincipal,
    ) -> DocumentResult<PendingDocumentDecision> {
        let mut tx = self.pool.begin().await?;
        let saved = sqlx::query_as::<_, PendingDocumentDecision>(
            "SELECT * FROM owner_document_decisions WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        if saved.principal != *principal {
            return Err(DocumentError::Unavailable);
        }
        principal_current(&mut tx, principal).await?;
        if saved.state == "canceled" {
            tx.commit().await?;
            return Ok(saved);
        }
        let saved=sqlx::query_as::<_,PendingDocumentDecision>("UPDATE owner_document_decisions SET state='canceled',completed_at=clock_timestamp() WHERE id=$1 AND state='pending' AND undo_until>clock_timestamp() RETURNING *")
            .bind(id).fetch_optional(&mut *tx).await?.ok_or_else(||DocumentError::Conflict("The Undo window has ended.".into()))?;
        notify(&mut tx).await?;
        tx.commit().await?;
        Ok(saved)
    }
    pub async fn document_decisions_for_actor(
        &self,
        actor: &str,
    ) -> DocumentResult<Vec<PendingDocumentDecision>> {
        Ok(sqlx::query_as("SELECT * FROM owner_document_decisions WHERE principal->>'actor_id'=$1 AND(state='pending' OR seen_at IS NULL) ORDER BY created_at DESC LIMIT 100").bind(actor).fetch_all(&self.pool).await?)
    }
    /// Read the person's own last input so Undo or a failed commit can restore its form after reload.
    pub async fn latest_document_decision_draft(
        &self,
        document: Uuid,
        request: Uuid,
        principal: &HumanDecisionPrincipal,
    ) -> DocumentResult<Option<PendingDocumentDecision>> {
        self.get_document_for_actor(document, &principal.actor_id)
            .await?;
        let person = serde_json::json!({"actor_id":principal.actor_id,"owner_account":principal.owner_account});
        let saved=sqlx::query_as::<_,PendingDocumentDecision>("SELECT * FROM owner_document_decisions WHERE document_id=$1 AND request_id=$2 AND principal @> $3::jsonb AND decision->>'kind'='accept_review' ORDER BY created_at DESC LIMIT 1")
            .bind(document).bind(request).bind(person).fetch_optional(&self.pool).await?;
        Ok(
            saved
                .filter(|saved| saved.state != "applied" && saved.principal.same_person(principal)),
        )
    }
    pub async fn due_document_decisions(&self) -> DocumentResult<Vec<PendingDocumentDecision>> {
        Ok(sqlx::query_as("SELECT * FROM owner_document_decisions WHERE state='pending' AND undo_until<=clock_timestamp() ORDER BY undo_until,id LIMIT 50").fetch_all(&self.pool).await?)
    }
    pub async fn fail_document_decision(&self, id: Uuid, problem: &str) -> DocumentResult<()> {
        let mut tx = self.pool.begin().await?;
        let problem: String = problem.chars().take(2000).collect();
        sqlx::query("UPDATE owner_document_decisions SET state='failed',problem=$2,completed_at=clock_timestamp() WHERE id=$1 AND state='pending'").bind(id).bind(problem).execute(&mut *tx).await?;
        notify(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn acknowledge_document_decision(
        &self,
        id: Uuid,
        principal: &HumanDecisionPrincipal,
    ) -> DocumentResult<()> {
        let mut tx = self.pool.begin().await?;
        let saved = sqlx::query_as::<_, PendingDocumentDecision>(
            "SELECT * FROM owner_document_decisions WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        principal_current(&mut tx, principal).await?;
        if !saved.principal.same_person(principal) || saved.state == "pending" {
            return Err(DocumentError::Unavailable);
        }
        sqlx::query("UPDATE owner_document_decisions SET seen_at=COALESCE(seen_at,clock_timestamp()) WHERE id=$1").bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn apply_document_decision(&self, id: Uuid) -> DocumentResult<bool> {
        let saved = sqlx::query_as::<_, PendingDocumentDecision>(
            "SELECT * FROM owner_document_decisions WHERE id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        if saved.state != "pending" {
            return Ok(false);
        }
        let result = match &saved.decision {
            DocumentDecision::AcceptReview {
                expected_document_version,
                expected_review_version,
                accepted_version_name,
                feedback,
            } => self
                .accept_document_review_with_pending(
                    crate::AcceptDocumentReview {
                        document_id: saved.document_id,
                        review_id: saved.request_id,
                        actor_id: &saved.principal.actor_id,
                        command_id: saved.id,
                        expected_document_version: *expected_document_version,
                        expected_review_version: *expected_review_version,
                        accepted_version_name,
                        feedback,
                    },
                    Some(saved.id),
                )
                .await
                .map(|_| ()),
            DocumentDecision::CompleteCollaboration { .. } => self
                .resolve_document_collaboration_with_pending(
                    saved.document_id,
                    &saved.principal.actor_id,
                    saved.request_id,
                    Some(saved.id),
                )
                .await
                .map(|_| ()),
        };
        if let Err(error) = result {
            let state: Option<String> =
                sqlx::query_scalar("SELECT state FROM owner_document_decisions WHERE id=$1")
                    .bind(id)
                    .fetch_optional(&self.pool)
                    .await?;
            if matches!(state.as_deref(), Some("applied" | "canceled")) {
                return Ok(false);
            }
            return Err(error);
        }
        Ok(true)
    }
}
/// Called after the document's own lock, before any comment, acceptance or Work resumption.
pub(super) async fn admit_pending_document_decision(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    document: Uuid,
    request: Uuid,
    actor: &str,
) -> DocumentResult<PendingDocumentDecision> {
    let saved = sqlx::query_as::<_, PendingDocumentDecision>(
        "SELECT * FROM owner_document_decisions WHERE id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(DocumentError::Unavailable)?;
    let due: bool = sqlx::query_scalar(
        "SELECT undo_until<=clock_timestamp() FROM owner_document_decisions WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    if saved.state != "pending"
        || !due
        || saved.document_id != document
        || saved.request_id != request
        || saved.principal.actor_id != actor
    {
        return Err(changed());
    }
    let snapshot =
        request_snapshot(tx, document, request, &saved.principal, &saved.decision).await?;
    if snapshot != saved.request_snapshot {
        return Err(changed());
    }
    Ok(saved)
}
pub(super) async fn complete_pending_document_decision(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> DocumentResult<()> {
    let result=sqlx::query("UPDATE owner_document_decisions SET state='applied',completed_at=clock_timestamp() WHERE id=$1 AND state='pending'").bind(id).execute(&mut **tx).await?;
    if result.rows_affected() != 1 {
        return Err(changed());
    }
    notify(tx).await?;
    Ok(())
}
