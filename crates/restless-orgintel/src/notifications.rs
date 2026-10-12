//! Minimal external-delivery projection over Core-owned durable work.
//!
//! This is deliberately not another notification outbox. Mentions and owner
//! handoffs remain the canonical durable objects and their existing event rows
//! remain the wake/replay substrate. Cloud polls this projection and owns only
//! delivery attempts and receipts for the external email effect.
//!
//! Document mentions and replies in threads a person is in reach email through
//! the same projection, read from For you: an item still unread a while after
//! it happened is owed, and opening it anywhere withdraws it.

use super::*;
use serde::Serialize;

/// How long a For you item waits unread before it is emailed: long enough that
/// someone at the cockpit reads it there first.
pub const FOR_YOU_EMAIL_AFTER: chrono::Duration = chrono::Duration::minutes(10);
/// Older unread items are history, not news; they are never emailed, so the
/// first projection after an upgrade does not send a backlog.
pub const FOR_YOU_EMAIL_WITHIN: chrono::Duration = chrono::Duration::days(2);

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

/// A For you item a hosted person has not opened: a mention of them in a
/// document comment, or a reply in a document or Room thread they are in.
#[derive(Debug, Clone)]
pub struct PendingForYouNotification {
    pub item: ForYouItem,
    pub recipient: HumanNotificationRecipient,
}

/// Which For you items are worth an email: document mentions and replies.
/// Room mentions already have their own intent; shares, reviews, comments on
/// followed documents and changes stay in the cockpit.
pub fn for_you_item_is_emailed(item: &ForYouItem, now: DateTime<Utc>) -> bool {
    (item.key.starts_with("doc-mention:") || item.kind == "reply")
        && item.unread
        && item.at <= now - FOR_YOU_EMAIL_AFTER
        && item.at > now - FOR_YOU_EMAIL_WITHIN
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

    /// Document mentions and thread replies still unread by each active hosted
    /// person a while after they happened, each still readable by them.
    pub async fn pending_human_for_you_notifications(
        &self,
        now: DateTime<Utc>,
    ) -> DocumentResult<Vec<PendingForYouNotification>> {
        let recipients = sqlx::query(
            "SELECT issuer,subject,membership_id,actor_id \
               FROM human_principal_actor_bindings \
              WHERE membership_status='active' ORDER BY membership_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut pending = Vec::new();
        for row in recipients {
            let recipient = HumanNotificationRecipient {
                identity_issuer: row.get("issuer"),
                user_id: row.get("subject"),
                membership_id: row.get("membership_id"),
                actor_id: row.get("actor_id"),
            };
            // Marking something unread is a note to oneself, not news; it is never emailed.
            let reminders: Vec<String> = sqlx::query_scalar(
                "SELECT item_key FROM for_you_reads WHERE actor_id=$1 AND marked_unread",
            )
            .bind(&recipient.actor_id)
            .fetch_all(&self.pool)
            .await?;
            for item in self.unread_for_you_items(&recipient.actor_id).await? {
                if for_you_item_is_emailed(&item, now) && !reminders.contains(&item.key) {
                    pending.push(PendingForYouNotification {
                        item,
                        recipient: recipient.clone(),
                    });
                }
            }
        }
        Ok(pending)
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
