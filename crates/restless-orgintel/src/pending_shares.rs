//! Sharing with someone invited to the company who has not joined yet (S66). The share waits under
//! their email, as the invitation holds it, and is given when they first enter: by whoever shared
//! it, through the same path as any other share, so it reaches them in For you as one.

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::documents::may_manage_participants;
use crate::sheets::may_share;
use crate::{
    DocumentAccess, DocumentError, DocumentResult, OrgIntel, SetDocumentParticipant, SheetError,
};

/// What a pending share is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareTarget {
    Document(Uuid),
    Sheet(Uuid),
}

impl ShareTarget {
    fn ids(self) -> (Option<Uuid>, Option<Uuid>) {
        match self {
            Self::Document(id) => (Some(id), None),
            Self::Sheet(id) => (None, Some(id)),
        }
    }
}

/// Someone a document or sheet waits for, with the access they will be given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct PendingShare {
    pub email: String,
    pub access: String,
    pub invited_by_actor_id: String,
    pub invited_at: DateTime<Utc>,
}

/// An address as invitations hold it, trimmed and lower-cased; `None` when it cannot be one.
pub fn invitee_email(email: &str) -> Option<String> {
    let email = email.trim().to_lowercase();
    let (local, domain) = email.split_once('@')?;
    (!local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && email.len() <= 320
        && !email.chars().any(|c| c.is_whitespace() || c.is_control()))
    .then_some(email)
}

fn sheet_error(error: SheetError) -> DocumentError {
    match error {
        SheetError::Unavailable => DocumentError::Unavailable,
        SheetError::Database(error) => DocumentError::Database(error),
        SheetError::Conflict(reason) => DocumentError::Conflict(reason),
        SheetError::Invalid(reason) => DocumentError::Invalid(reason),
    }
}

impl OrgIntel {
    /// Whoever may share the document or sheet with a colleague may also share it with an invitee.
    async fn may_manage_share(&self, target: ShareTarget, actor: &str) -> DocumentResult<bool> {
        let mut tx = self.pool.begin().await?;
        let allowed = match target {
            ShareTarget::Document(id) => {
                let owner: Option<String> = sqlx::query_scalar(
                    "SELECT owner_actor_id FROM native_documents WHERE id=$1 AND status<>'archived'",
                )
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
                match owner {
                    Some(owner) => may_manage_participants(&mut tx, id, &owner, actor).await?,
                    None => false,
                }
            }
            ShareTarget::Sheet(id) => match may_share(&mut tx, id, actor).await {
                Ok(allowed) => allowed,
                Err(SheetError::Unavailable) => false,
                Err(error) => return Err(sheet_error(error)),
            },
        };
        tx.rollback().await?;
        Ok(allowed)
    }

    /// Who the document or sheet waits for, oldest invitation first. Empty for anyone who may not
    /// share it: what they cannot change they need not see.
    pub async fn pending_shares(
        &self,
        target: ShareTarget,
        actor: &str,
    ) -> DocumentResult<Vec<PendingShare>> {
        if !self.may_manage_share(target, actor).await? {
            return Ok(Vec::new());
        }
        let (document, sheet) = target.ids();
        Ok(sqlx::query_as(
            "SELECT email,access,invited_by_actor_id,invited_at FROM pending_shares \
             WHERE document_id IS NOT DISTINCT FROM $1 AND sheet_id IS NOT DISTINCT FROM $2 \
             ORDER BY invited_at,email",
        )
        .bind(document)
        .bind(sheet)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Keeps a share for someone invited but not yet in the company. Sharing again with the same
    /// address changes the access it will give.
    pub async fn share_with_invitee(
        &self,
        target: ShareTarget,
        actor: &str,
        email: &str,
        access: &str,
    ) -> DocumentResult<()> {
        let email = invitee_email(email)
            .ok_or_else(|| DocumentError::Invalid("that is not an email address".into()))?;
        let known = match target {
            ShareTarget::Document(_) => matches!(access, "read" | "comment" | "edit"),
            ShareTarget::Sheet(_) => matches!(access, "read" | "comment" | "edit"),
        };
        if !known {
            return Err(DocumentError::Invalid(format!("unknown access {access:?}")));
        }
        if !self.may_manage_share(target, actor).await? {
            return Err(DocumentError::Unavailable);
        }
        let (document, sheet) = target.ids();
        let conflict = if document.is_some() {
            "(document_id,email) WHERE document_id IS NOT NULL"
        } else {
            "(sheet_id,email) WHERE sheet_id IS NOT NULL"
        };
        sqlx::query(&format!(
            "INSERT INTO pending_shares (id,email,document_id,sheet_id,access,invited_by_actor_id) \
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT {conflict} DO UPDATE SET \
               access=EXCLUDED.access,invited_by_actor_id=EXCLUDED.invited_by_actor_id, \
               invited_at=now()"
        ))
        .bind(Uuid::new_v4())
        .bind(&email)
        .bind(document)
        .bind(sheet)
        .bind(access)
        .bind(actor)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Stops waiting for someone. Answers whether there was anything to stop.
    pub async fn cancel_pending_share(
        &self,
        target: ShareTarget,
        actor: &str,
        email: &str,
    ) -> DocumentResult<bool> {
        if !self.may_manage_share(target, actor).await? {
            return Err(DocumentError::Unavailable);
        }
        let Some(email) = invitee_email(email) else {
            return Ok(false);
        };
        let (document, sheet) = target.ids();
        Ok(sqlx::query(
            "DELETE FROM pending_shares WHERE email=$1 \
             AND document_id IS NOT DISTINCT FROM $2 AND sheet_id IS NOT DISTINCT FROM $3",
        )
        .bind(&email)
        .bind(document)
        .bind(sheet)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0)
    }

    /// Gives someone who just entered what was shared with their email before they joined, as
    /// whoever shared it. A share its sharer may no longer give, or for something gone, is dropped.
    /// Answers how many were given.
    pub async fn apply_pending_shares(&self, email: &str, actor_id: &str) -> DocumentResult<usize> {
        let Some(email) = invitee_email(email) else {
            return Ok(0);
        };
        let waiting: Vec<(Uuid, Option<Uuid>, Option<Uuid>, String, String)> = sqlx::query_as(
            "SELECT id,document_id,sheet_id,access,invited_by_actor_id FROM pending_shares \
             WHERE email=$1 ORDER BY invited_at",
        )
        .bind(&email)
        .fetch_all(&self.pool)
        .await?;
        let mut given = 0;
        for (id, document, sheet, access, sharer) in waiting {
            let outcome = match (document, sheet) {
                (Some(document_id), _) => {
                    self.give_document(document_id, &sharer, actor_id, &access)
                        .await
                }
                (None, Some(sheet_id)) => self
                    .share_sheet(sheet_id, &sharer, actor_id, Some(&access))
                    .await
                    .map_err(sheet_error),
                (None, None) => Ok(()),
            };
            match outcome {
                Ok(()) => given += 1,
                Err(DocumentError::Database(error)) => return Err(DocumentError::Database(error)),
                // Refused: the sharer lost the right to share it, or it is gone.
                Err(_) => {}
            }
            sqlx::query("DELETE FROM pending_shares WHERE id=$1")
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        Ok(given)
    }

    async fn give_document(
        &self,
        document_id: Uuid,
        sharer: &str,
        person: &str,
        access: &str,
    ) -> DocumentResult<()> {
        let access = match access {
            "read" => DocumentAccess::Read,
            "comment" => DocumentAccess::Comment,
            _ => DocumentAccess::Edit,
        };
        // A colleague editing the document at that moment can move its version on; one retry
        // reads the new one.
        for _ in 0..2 {
            let version: Option<i64> =
                sqlx::query_scalar("SELECT version FROM native_documents WHERE id=$1")
                    .bind(document_id)
                    .fetch_optional(&self.pool)
                    .await?;
            let Some(version) = version else {
                return Err(DocumentError::Unavailable);
            };
            match self
                .set_document_participant(SetDocumentParticipant {
                    command_id: Uuid::new_v4(),
                    document_id,
                    actor_id: sharer,
                    expected_document_version: version,
                    participant_actor_id: person,
                    access,
                })
                .await
            {
                Err(DocumentError::Conflict(_)) => continue,
                outcome => return outcome.map(|_| ()),
            }
        }
        Err(DocumentError::Conflict("the document kept changing".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::invitee_email;

    #[test]
    fn invitee_addresses_are_read_as_invitations_hold_them() {
        assert_eq!(
            invitee_email(" Ana@Example.COM ").as_deref(),
            Some("ana@example.com")
        );
        for bad in [
            "",
            "ana",
            "@example.com",
            "ana@example",
            "ana@.com",
            "a b@example.com",
        ] {
            assert_eq!(invitee_email(bad), None, "{bad:?}");
        }
    }
}
