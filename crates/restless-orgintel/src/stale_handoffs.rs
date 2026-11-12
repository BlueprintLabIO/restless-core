//! Items that wait on the owner do not go stale in silence (owner decision, 12 November 2026:
//! one waited 192 hours, three sat open 12 to 20 days, and nothing raised or withdrew them).
//!
//! - A handoff whose Work is finished or abandoned, or whose goal is closed, is withdrawn with
//!   the reason recorded. Nobody is asked.
//! - One still needed after [`STALE_HANDOFF_AFTER`] wakes Exec with one compact message listing
//!   every such item. Exec decides for each: withdraw it, act within its own authority, or raise
//!   it again once with what it now costs. A re-raised card is the same card, back at the top.
//!
//! Counts and times live on the handoff, so the backoff holds across restarts and Exec is never
//! asked more than three times about one item.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::Row as _;
use uuid::Uuid;

use crate::{ensure_direct_message_room_in_tx, OrgIntel, OrgIntelError, Result};

/// How long an item waits on the owner before Exec is asked about it: three days, then six days
/// after that, then twelve days after that (days 3, 9 and 21), then never again.
pub const STALE_HANDOFF_AFTER: [chrono::Duration; 3] = [
    chrono::Duration::days(3),
    chrono::Duration::days(6),
    chrono::Duration::days(12),
];

/// When Exec is next asked about an item, or `None` once it has been asked three times. `since`
/// is when it reached the owner; `last_asked` is the last time Exec was asked or raised it again.
pub fn next_stale_signal(
    since: DateTime<Utc>,
    asked: i32,
    last_asked: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    let after = STALE_HANDOFF_AFTER.get(usize::try_from(asked).ok()?)?;
    Some(last_asked.map_or(since, |last| last.max(since)) + *after)
}

/// An item that has waited on the owner past its threshold.
#[derive(Debug, Clone)]
pub struct StaleHandoff {
    pub id: Uuid,
    pub title: String,
    pub work_id: Uuid,
    pub work_title: String,
    /// Other open Work that requires this Work, and so waits behind the item too.
    pub blocked_behind: i64,
    pub waiting_since: DateTime<Utc>,
    pub asked: i32,
}

/// A handoff withdrawn because nothing needs it any more.
#[derive(Debug, Clone)]
pub struct WithdrawnHandoff {
    pub id: Uuid,
    pub work_id: Uuid,
    pub reason: String,
}

/// A card Exec raised again: how often, when, and what it said the wait now costs.
#[derive(Debug, Clone)]
pub struct HandoffReraise {
    pub count: i32,
    pub at: DateTime<Utc>,
    pub reason: String,
}

fn days_waiting(since: DateTime<Utc>, now: DateTime<Utc>) -> i64 {
    (now - since).num_days().max(1)
}

/// "Still waiting, 5 days": the mark a re-raised card carries.
pub fn still_waiting_mark(since: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let days = days_waiting(since, now);
    format!(
        "Still waiting, {days} day{}",
        if days == 1 { "" } else { "s" }
    )
}

/// The one message Exec gets for every item that is due. Short on purpose: ids, ages, what waits
/// behind each, and the three ways on.
pub fn stale_signal_message(items: &[StaleHandoff], now: DateTime<Utc>) -> String {
    let mut body = String::from("These have waited on the owner a long time:\n");
    for item in items {
        let days = days_waiting(item.waiting_since, now);
        let behind = match item.blocked_behind {
            0 => String::new(),
            more => format!(" and {more} more behind it"),
        };
        body.push_str(&format!(
            "- {} \"{}\": {days} day{}. Blocks Work {} (\"{}\"){behind}.\n",
            item.id,
            item.title,
            if days == 1 { "" } else { "s" },
            item.work_id,
            item.work_title,
        ));
    }
    body.push_str(
        "Decide for each whether it still matters. If not, withdraw it with `restless work \
         resolve-handoff --handoff <id> --state withdrawn --resolution <why> --as exec`, then \
         resume or abandon its Work. If it does, act within your own authority, or raise it again \
         once with a fresh one-line reason of what the wait now costs: `restless work \
         reraise-handoff --handoff <id> --reason <one line> --as exec`. One short note to the \
         owner covers them all.",
    );
    body
}

fn headline(owner_brief: Option<serde_json::Value>, requested_action: &str) -> String {
    let title = owner_brief
        .as_ref()
        .and_then(|brief| brief["headline"].as_str())
        .map(str::trim)
        .filter(|headline| !headline.is_empty())
        .or_else(|| {
            requested_action
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
        })
        .unwrap_or("An item waiting on the owner");
    let mut short: String = title.chars().take(120).collect();
    if title.chars().count() > 120 {
        short.push('…');
    }
    short
}

impl OrgIntel {
    /// Withdraw every outstanding handoff that nothing needs any more: its Work is completed or
    /// abandoned, or its goal is closed. A payment keeps its provider's own state, and a decision
    /// inside its Undo window is left to finish. Each withdrawal is an event with its reason.
    pub async fn withdraw_obsolete_handoffs(&self) -> Result<Vec<WithdrawnHandoff>> {
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query(
            "UPDATE owner_handoffs h SET state='withdrawn', resolved_at=now(), \
                    deferred_review_attempt_id=NULL, deferred_review_by=NULL, \
                    deferred_review_fingerprint=NULL, \
                    resolution=CASE \
                      WHEN w.status='completed' THEN 'Withdrawn: the Work it was for is finished.' \
                      WHEN w.status='abandoned' THEN 'Withdrawn: the Work it was for was stopped for good.' \
                      ELSE 'Withdrawn: the goal it was for is closed.' END \
             FROM work w LEFT JOIN goals g ON g.id=w.goal_id \
             WHERE w.id=h.work_id AND h.state IN ('pending','preparing') \
               AND h.category <> 'payment_confirmation' \
               AND (w.status IN ('completed','abandoned') OR g.closed_at IS NOT NULL) \
               AND NOT EXISTS (SELECT 1 FROM owner_handoff_decisions d \
                               WHERE d.handoff_id=h.id AND d.state='pending') \
             RETURNING h.id, h.work_id, h.resolution",
        )
        .fetch_all(&mut *tx)
        .await?;
        let mut withdrawn = Vec::with_capacity(rows.len());
        for row in rows {
            let item = WithdrawnHandoff {
                id: row.get("id"),
                work_id: row.get("work_id"),
                reason: row.get("resolution"),
            };
            sqlx::query(
                "INSERT INTO events (kind, actor_id, body) VALUES ('owner_handoff_withdrawn','daemon',$1)",
            )
            .bind(serde_json::json!({
                "handoff_id": item.id,
                "work_id": item.work_id,
                "reason": item.reason,
            }))
            .execute(&mut *tx)
            .await?;
            // Work under a closed goal stays as it was (closing a goal does not stop its Work),
            // but it no longer waits on the owner: it is paused, so it raises no "stuck" card and
            // can be resumed if the goal opens again.
            sqlx::query(
                "UPDATE work SET resolution='paused by the daemon: its goal was closed while it waited on a person'                  WHERE id=$1 AND status='blocked'",
            )
            .bind(item.work_id)
            .execute(&mut *tx)
            .await?;
            withdrawn.push(item);
        }
        tx.commit().await?;
        Ok(withdrawn)
    }

    /// Items owed to the owner that Exec is due to be asked about at `now`. One the owner
    /// snoozed or set aside is theirs to bring back, and is not raised over their head.
    pub async fn stale_owner_handoffs_at(&self, now: DateTime<Utc>) -> Result<Vec<StaleHandoff>> {
        let rows = sqlx::query(
            "SELECT h.id, h.owner_brief, h.requested_action, h.work_id, w.title AS work_title, \
                    COALESCE(h.escalated_at, h.created_at) AS waiting_since, \
                    h.stale_signal_count, \
                    GREATEST(h.stale_signalled_at, h.reraised_at) AS last_asked, \
                    (SELECT count(*) FROM work_edges e JOIN work d ON d.id=e.to_work_id \
                      WHERE e.from_work_id=w.id AND e.kind='requires' \
                        AND d.status IN ('proposed','active','blocked')) AS blocked_behind \
             FROM owner_handoffs h JOIN work w ON w.id=h.work_id \
             WHERE h.state='pending' AND h.assigned_to IS NULL \
               AND h.stale_signal_count < $2 \
               AND COALESCE(h.escalated_at, h.created_at) <= $1 - $3::interval \
               AND NOT EXISTS (SELECT 1 FROM inbox_marks m \
                               WHERE m.item_id = 'orgintel:handoff:' || h.id::text \
                                 AND m.snoozed_until > $1) \
               AND NOT EXISTS (SELECT 1 FROM attention_dismissals x \
                               WHERE x.item_id = 'orgintel:handoff:' || h.id::text) \
             ORDER BY waiting_since, h.id",
        )
        .bind(now)
        .bind(STALE_HANDOFF_AFTER.len() as i32)
        .bind(STALE_HANDOFF_AFTER[0])
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let waiting_since: DateTime<Utc> = row.get("waiting_since");
                let asked: i32 = row.get("stale_signal_count");
                let due = next_stale_signal(waiting_since, asked, row.get("last_asked"))?;
                (due <= now).then(|| StaleHandoff {
                    id: row.get("id"),
                    title: headline(
                        row.get("owner_brief"),
                        row.get::<String, _>("requested_action").as_str(),
                    ),
                    work_id: row.get("work_id"),
                    work_title: row.get("work_title"),
                    blocked_behind: row.get("blocked_behind"),
                    waiting_since,
                    asked,
                })
            })
            .collect())
    }

    /// Ask Exec, in one message, about every item that is due at `now`, and count the ask on each
    /// so the backoff holds. Returns the items asked about; empty means nothing was sent.
    pub async fn signal_stale_owner_handoffs_at(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<StaleHandoff>> {
        let due = self.stale_owner_handoffs_at(now).await?;
        if due.is_empty() {
            return Ok(due);
        }
        let mut tx = self.pool.begin().await?;
        let mut claimed = Vec::with_capacity(due.len());
        for item in due {
            // Two planes sweeping the same company: only one counts this ask.
            let won = sqlx::query(
                "UPDATE owner_handoffs SET stale_signal_count=stale_signal_count+1, \
                        stale_signalled_at=$3 \
                 WHERE id=$1 AND state='pending' AND assigned_to IS NULL \
                   AND stale_signal_count=$2",
            )
            .bind(item.id)
            .bind(item.asked)
            .bind(now)
            .execute(&mut *tx)
            .await?
            .rows_affected()
                == 1;
            if won {
                claimed.push(item);
            }
        }
        if claimed.is_empty() {
            tx.rollback().await?;
            return Ok(claimed);
        }
        let room_id = ensure_direct_message_room_in_tx(&mut tx, "daemon", Some("exec")).await?;
        sqlx::query(
            "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,'daemon','exec',$2)",
        )
        .bind(room_id)
        .bind(stale_signal_message(&claimed, now))
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('stale_handoffs_signalled','daemon',$1)",
        )
        .bind(serde_json::json!({
            "handoffs": claimed.iter().map(|item| item.id).collect::<Vec<_>>(),
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(claimed)
    }

    /// Exec raises a stale item again with what the wait now costs. The card keeps its identity
    /// and returns to the top of the owner's Inbox, marked as still waiting. Only Exec, only an
    /// item it was asked about, and once per ask, so the owner is never nagged past the backoff.
    pub async fn reraise_owner_handoff(
        &self,
        id: Uuid,
        by: &str,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Result<HandoffReraise> {
        if by != "exec" {
            return Err(OrgIntelError::InvalidWork(
                "only Exec raises an owner's item again".into(),
            ));
        }
        let reason = reason.split_whitespace().collect::<Vec<_>>().join(" ");
        if reason.is_empty() || reason.chars().count() > 240 {
            return Err(OrgIntelError::InvalidWork(
                "say in one line (under 240 characters) what the wait now costs".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT state::text AS state, assigned_to, stale_signal_count, reraise_count \
             FROM owner_handoffs WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let refusal = match &row {
            None => Some("no handoff with that id"),
            Some(row) if row.get::<String, _>("state") != "pending" => {
                Some("that item is no longer waiting")
            }
            Some(row) if row.get::<Option<String>, _>("assigned_to").is_some() => {
                Some("that item is not waiting on the owner")
            }
            Some(row)
                if row.get::<i32, _>("reraise_count")
                    >= row.get::<i32, _>("stale_signal_count") =>
            {
                Some(
                    "raise an item again only when asked about it, and once per ask; it has \
                     already been raised",
                )
            }
            Some(_) => None,
        };
        if let Some(refusal) = refusal {
            tx.rollback().await?;
            return Err(OrgIntelError::InvalidWork(refusal.into()));
        }
        let count: i32 = sqlx::query_scalar(
            "UPDATE owner_handoffs SET reraise_count=reraise_count+1, reraised_at=$2, \
                    reraise_reason=$3 WHERE id=$1 RETURNING reraise_count",
        )
        .bind(id)
        .bind(now)
        .bind(&reason)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('owner_handoff_reraised',$1,$2)",
        )
        .bind(by)
        .bind(serde_json::json!({ "handoff_id": id, "count": count, "reason": reason }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(HandoffReraise {
            count,
            at: now,
            reason,
        })
    }

    /// Exec withdraws an item it was asked about and judged no longer needed. The Work stays
    /// blocked with the reason, for Exec to resume or abandon; its lead is told. A payment keeps
    /// its provider's state, and an item Exec was never asked about stays the owner's.
    pub async fn withdraw_stale_owner_handoff(
        &self,
        id: Uuid,
        by: &str,
        reason: &str,
    ) -> Result<()> {
        if by != "exec" {
            return Err(OrgIntelError::InvalidWork(
                "only Exec withdraws an item waiting on the owner".into(),
            ));
        }
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "say why the item is no longer needed".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "UPDATE owner_handoffs h SET state='withdrawn', resolved_at=now(), resolution=$2, \
                    deferred_review_attempt_id=NULL, deferred_review_by=NULL, \
                    deferred_review_fingerprint=NULL \
             FROM work w \
             WHERE h.id=$1 AND w.id=h.work_id AND h.state='pending' AND h.assigned_to IS NULL \
               AND h.stale_signal_count > 0 AND h.category <> 'payment_confirmation' \
               AND NOT EXISTS (SELECT 1 FROM owner_handoff_decisions d \
                               WHERE d.handoff_id=h.id AND d.state='pending') \
             RETURNING h.work_id, w.owner_id, w.status::text AS status",
        )
        .bind(id)
        .bind(format!("Withdrawn by Exec: {reason}"))
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            tx.rollback().await?;
            return Err(OrgIntelError::InvalidWork(
                "Exec withdraws only an item waiting on the owner that it was asked about, and \
                 never a payment; anything else is the owner's to answer"
                    .into(),
            ));
        };
        let work_id: Uuid = row.get("work_id");
        let lead: String = row.get("owner_id");
        if row.get::<String, _>("status") == "blocked" {
            sqlx::query("UPDATE work SET resolution=$2 WHERE id=$1")
                .bind(work_id)
                .bind(format!(
                    "its question to the owner was withdrawn by Exec: {reason}"
                ))
                .execute(&mut *tx)
                .await?;
        }
        if !matches!(lead.as_str(), "owner" | "world" | "daemon" | "exec") {
            let room_id = ensure_direct_message_room_in_tx(&mut tx, "exec", Some(&lead)).await?;
            sqlx::query(
                "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,'exec',$2,$3)",
            )
            .bind(room_id)
            .bind(&lead)
            .bind(format!(
                "I withdrew the question to the owner on Work {work_id}: {reason}"
            ))
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('owner_handoff_withdrawn',$1,$2)",
        )
        .bind(by)
        .bind(serde_json::json!({ "handoff_id": id, "work_id": work_id, "reason": reason }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// The pending cards Exec has raised again, for the Inbox to show at the top and mark.
    pub async fn handoff_reraises(&self) -> Result<HashMap<Uuid, HandoffReraise>> {
        let rows = sqlx::query(
            "SELECT id, reraise_count, reraised_at, reraise_reason FROM owner_handoffs \
             WHERE state IN ('pending','preparing') AND reraise_count > 0 \
               AND reraised_at IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    row.get("id"),
                    HandoffReraise {
                        count: row.get("reraise_count"),
                        at: row.get("reraised_at"),
                        reason: row
                            .get::<Option<String>, _>("reraise_reason")
                            .unwrap_or_default(),
                    },
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(day: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + chrono::Duration::days(day)
    }

    #[test]
    fn exec_is_asked_on_days_three_nine_and_twenty_one_then_never() {
        assert_eq!(next_stale_signal(day(0), 0, None), Some(day(3)));
        assert_eq!(next_stale_signal(day(0), 1, Some(day(3))), Some(day(9)));
        assert_eq!(next_stale_signal(day(0), 2, Some(day(9))), Some(day(21)));
        assert_eq!(next_stale_signal(day(0), 3, Some(day(21))), None);
        // A late sweep pushes the next ask out rather than stacking two.
        assert_eq!(next_stale_signal(day(0), 1, Some(day(8))), Some(day(14)));
    }

    #[test]
    fn the_signal_is_one_compact_message_for_every_item() {
        let item = |title: &str, since: i64, behind: i64| StaleHandoff {
            id: Uuid::nil(),
            title: title.into(),
            work_id: Uuid::nil(),
            work_title: "Launch the store".into(),
            blocked_behind: behind,
            waiting_since: day(since),
            asked: 0,
        };
        let body = stale_signal_message(
            &[item("Choose the price", 0, 2), item("Sign the lease", 4, 0)],
            day(8),
        );
        assert!(body.contains("\"Choose the price\": 8 days"), "{body}");
        assert!(body.contains("and 2 more behind it"), "{body}");
        assert!(body.contains("\"Sign the lease\": 4 days"), "{body}");
        assert!(body.contains("--state withdrawn"), "{body}");
        assert!(body.contains("reraise-handoff"), "{body}");
        assert_eq!(body.matches("Decide for each").count(), 1);
        assert_eq!(still_waiting_mark(day(0), day(5)), "Still waiting, 5 days");
        assert_eq!(still_waiting_mark(day(0), day(1)), "Still waiting, 1 day");
    }
}
