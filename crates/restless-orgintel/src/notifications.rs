//! Minimal external-delivery projection over Core-owned durable work.
//!
//! This is deliberately not another notification outbox. Mentions and owner
//! handoffs remain the canonical durable objects and their existing event rows
//! remain the wake/replay substrate. Cloud polls this projection and owns only
//! delivery attempts and receipts for the external email effect.

use super::*;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HumanNotificationRecipient {
    pub identity_issuer: String,
    pub user_id: String,
    pub membership_id: String,
    pub actor_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PendingMentionNotification {
    pub mention_id: Uuid,
    pub room_id: Uuid,
    pub room_title: String,
    pub message_id: i64,
    pub thread_root_message_id: i64,
    pub source_event_id: i64,
    pub source_actor_id: String,
    pub source_actor_display: String,
    pub source_user_id: Option<String>,
    pub recipient: HumanNotificationRecipient,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffNotificationPresentation {
    pub handoff_id: Uuid,
    pub notification_revision: i64,
    pub source_actor_display: String,
    pub source_user_id: Option<String>,
}

impl OrgIntel {
    /// Current unresolved direct mentions owed to hosted human Actors.
    ///
    /// The Message body and structured mention evidence are intentionally not
    /// selected. External delivery needs only attribution and an exact return
    /// coordinate; the authenticated cockpit owns the private content.
    pub async fn pending_human_mention_notifications(
        &self,
    ) -> Result<Vec<PendingMentionNotification>> {
        let rows = sqlx::query(
            "SELECT mention.id AS mention_id,mention.room_id, \
                    mention.notification_room_title AS room_title, \
                    mention.message_id,mention.thread_root_message_id, \
                    mention.created_event_id AS source_event_id,message.from_actor, \
                    mention.notification_source_display AS source_actor_display, \
                    mention.notification_source_user_id AS source_user_id, \
                    recipient_binding.issuer AS recipient_issuer, \
                    recipient_binding.subject AS recipient_user_id, \
                    recipient_binding.membership_id AS recipient_membership_id, \
                    recipient_binding.actor_id AS recipient_actor_id,mention.created_at \
               FROM message_mentions mention \
               JOIN messages message ON message.id=mention.message_id \
               JOIN rooms room ON room.id=mention.room_id AND room.archived_at IS NULL \
               JOIN human_principal_actor_bindings recipient_binding \
                 ON recipient_binding.actor_id=mention.mentioned_actor_id \
                AND recipient_binding.membership_status='active' \
              WHERE mention.kind='direct' AND mention.resolved_at IS NULL \
                AND mention.cancelled_at IS NULL AND message.deleted_at IS NULL \
              ORDER BY mention.created_at,mention.id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| PendingMentionNotification {
                mention_id: row.get("mention_id"),
                room_id: row.get("room_id"),
                room_title: row.get("room_title"),
                message_id: row.get("message_id"),
                thread_root_message_id: row.get("thread_root_message_id"),
                source_event_id: row.get("source_event_id"),
                source_actor_id: row.get("from_actor"),
                source_actor_display: row.get("source_actor_display"),
                source_user_id: row.get("source_user_id"),
                recipient: HumanNotificationRecipient {
                    identity_issuer: row.get("recipient_issuer"),
                    user_id: row.get("recipient_user_id"),
                    membership_id: row.get("recipient_membership_id"),
                    actor_id: row.get("recipient_actor_id"),
                },
                created_at: row.get("created_at"),
            })
            .collect())
    }

    /// Immutable delivery presentation and monotonic epoch for canonical
    /// owner handoffs. This remains metadata on the source row, not a second
    /// notification lifecycle.
    pub async fn owner_handoff_notification_presentations(
        &self,
        handoff_ids: &[Uuid],
    ) -> Result<Vec<HandoffNotificationPresentation>> {
        if handoff_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(sqlx::query(
            "SELECT id,notification_revision,notification_source_display, \
                    notification_source_user_id \
               FROM owner_handoffs WHERE id=ANY($1)",
        )
        .bind(handoff_ids)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| HandoffNotificationPresentation {
            handoff_id: row.get("id"),
            notification_revision: row.get("notification_revision"),
            source_actor_display: row.get("notification_source_display"),
            source_user_id: row.get("notification_source_user_id"),
        })
        .collect())
    }

    pub async fn active_hosted_owners(&self) -> Result<Vec<HumanNotificationRecipient>> {
        Ok(sqlx::query(
            "SELECT issuer,subject,membership_id,actor_id \
               FROM human_principal_actor_bindings \
              WHERE membership_role='owner' AND membership_status='active' \
              ORDER BY membership_id",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| HumanNotificationRecipient {
            identity_issuer: row.get("issuer"),
            user_id: row.get("subject"),
            membership_id: row.get("membership_id"),
            actor_id: row.get("actor_id"),
        })
        .collect())
    }
}
