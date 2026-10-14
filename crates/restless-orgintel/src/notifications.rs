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

/// A person's daily email time: what is owed waits until the next one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmailDigest {
    pub at: chrono::NaiveTime,
    /// An IANA time zone, such as `Europe/London`.
    pub time_zone: String,
}

impl EmailDigest {
    pub fn zone(&self) -> Option<chrono_tz::Tz> {
        self.time_zone.parse().ok()
    }

    /// The most recent digest time at or before `now`: anything that happened by then is due.
    pub fn last_release(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        use chrono::TimeZone as _;
        let zone = self.zone()?;
        let at = |date: chrono::NaiveDate| {
            let local = date.and_time(self.at);
            // A time skipped by a clock change happens an hour later that day.
            zone.from_local_datetime(&local)
                .earliest()
                .or_else(|| {
                    zone.from_local_datetime(&(local + chrono::Duration::hours(1)))
                        .earliest()
                })
                .map(|time| time.with_timezone(&Utc))
        };
        let today = now.with_timezone(&zone).date_naive();
        match at(today)? {
            release if release <= now => Some(release),
            _ => at(today - chrono::Duration::days(1)),
        }
    }
}

/// One browser that asked to be pushed what needs this person.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PushSubscription {
    pub endpoint: String,
    pub actor_id: String,
    pub p256dh: String,
    pub auth: String,
    pub created_at: DateTime<Utc>,
}

impl OrgIntel {
    /// Whether this person wants email for what needs them; yes until they say no.
    pub async fn email_notifications(&self, actor_id: &str) -> Result<bool> {
        let email: Option<bool> =
            sqlx::query_scalar("SELECT email FROM notification_preferences WHERE actor_id=$1")
                .bind(actor_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(email.unwrap_or(true))
    }

    /// Turn this person's email on or off.
    pub async fn set_email_notifications(&self, actor_id: &str, email: bool) -> Result<()> {
        sqlx::query(
            "INSERT INTO notification_preferences (actor_id,email,updated_at) VALUES ($1,$2,now()) \
             ON CONFLICT (actor_id) DO UPDATE SET email=EXCLUDED.email, updated_at=now()",
        )
        .bind(actor_id)
        .bind(email)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The people who turned email off, so the delivery projection owes them nothing.
    pub async fn email_opted_out(&self) -> Result<std::collections::HashSet<String>> {
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT actor_id FROM notification_preferences WHERE NOT email",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect())
    }

    /// This person's daily email time, if they asked for one batch a day.
    pub async fn email_digest(&self, actor_id: &str) -> Result<Option<EmailDigest>> {
        let row: Option<(Option<chrono::NaiveTime>, Option<String>)> = sqlx::query_as(
            "SELECT digest_at, digest_time_zone FROM notification_preferences WHERE actor_id=$1",
        )
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|(at, zone)| {
            Some(EmailDigest {
                at: at?,
                time_zone: zone?,
            })
        }))
    }

    /// Ask for email once a day at a local time, or (`None`) as things happen.
    pub async fn set_email_digest(
        &self,
        actor_id: &str,
        digest: Option<&EmailDigest>,
    ) -> Result<()> {
        if let Some(digest) = digest.filter(|digest| digest.zone().is_none()) {
            return Err(OrgIntelError::InvalidContext(format!(
                "unknown time zone {:?}",
                digest.time_zone
            )));
        }
        sqlx::query(
            "INSERT INTO notification_preferences (actor_id,digest_at,digest_time_zone,updated_at) \
             VALUES ($1,$2,$3,now()) ON CONFLICT (actor_id) DO UPDATE SET \
             digest_at=EXCLUDED.digest_at, digest_time_zone=EXCLUDED.digest_time_zone, updated_at=now()",
        )
        .bind(actor_id)
        .bind(digest.map(|digest| digest.at))
        .bind(digest.map(|digest| digest.time_zone.as_str()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Everyone who asked for a daily batch, so the delivery projection can hold their email.
    pub async fn email_digests(&self) -> Result<std::collections::HashMap<String, EmailDigest>> {
        let rows: Vec<(String, chrono::NaiveTime, String)> = sqlx::query_as(
            "SELECT actor_id, digest_at, digest_time_zone FROM notification_preferences \
             WHERE email AND digest_at IS NOT NULL AND digest_time_zone IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(actor, at, time_zone)| (actor, EmailDigest { at, time_zone }))
            .collect())
    }

    /// Keep this browser for this person. A browser someone else subscribed becomes theirs and
    /// starts afresh, so it is never told what was owed to the person before.
    pub async fn subscribe_push(
        &self,
        actor_id: &str,
        endpoint: &str,
        p256dh: &str,
        auth: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO push_subscriptions (endpoint,actor_id,p256dh,auth) VALUES ($1,$2,$3,$4) \
             ON CONFLICT (endpoint) DO UPDATE SET actor_id=EXCLUDED.actor_id, \
             p256dh=EXCLUDED.p256dh, auth=EXCLUDED.auth, created_at=CASE \
             WHEN push_subscriptions.actor_id=EXCLUDED.actor_id THEN push_subscriptions.created_at \
             ELSE now() END",
        )
        .bind(endpoint)
        .bind(actor_id)
        .bind(p256dh)
        .bind(auth)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Stop pushing to this person's browser. Someone else's browser is left alone.
    pub async fn unsubscribe_push(&self, actor_id: &str, endpoint: &str) -> Result<()> {
        sqlx::query("DELETE FROM push_subscriptions WHERE endpoint=$1 AND actor_id=$2")
            .bind(endpoint)
            .bind(actor_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// The push service no longer knows this browser.
    pub async fn forget_push_endpoint(&self, endpoint: &str) -> Result<()> {
        sqlx::query("DELETE FROM push_subscriptions WHERE endpoint=$1")
            .bind(endpoint)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn push_subscriptions(&self) -> Result<Vec<PushSubscription>> {
        Ok(sqlx::query_as(
            "SELECT endpoint, actor_id, p256dh, auth, created_at FROM push_subscriptions \
             ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Claim one notice for one browser before sending it; false when it was already claimed.
    pub async fn claim_push(&self, endpoint: &str, intent_id: &str) -> Result<bool> {
        Ok(sqlx::query(
            "INSERT INTO push_deliveries (endpoint,intent_id) VALUES ($1,$2) ON CONFLICT DO NOTHING",
        )
        .bind(endpoint)
        .bind(intent_id)
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1)
    }

    /// A send that failed gives its claim back for the next pass.
    pub async fn release_push(&self, endpoint: &str, intent_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM push_deliveries WHERE endpoint=$1 AND intent_id=$2")
            .bind(endpoint)
            .bind(intent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Claims older than anything still pushed are history.
    pub async fn prune_push_deliveries(&self, before: DateTime<Utc>) -> Result<()> {
        sqlx::query("DELETE FROM push_deliveries WHERE created_at < $1")
            .bind(before)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

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
