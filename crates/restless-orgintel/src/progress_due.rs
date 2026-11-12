//! Progress comes to the owner without "progress?" (owner decision, 12 November 2026: it was
//! their most common message).
//!
//! When Work moved (Attempts finished, failed or blocked; a goal closed) since Exec last wrote to
//! the owner, and the owner has had no update for [`PROGRESS_UPDATE_AFTER`], Exec is woken once
//! with one short message to post one update covering everything. The mark moves with every ask,
//! so nothing wakes Exec again until something new has changed and the threshold has passed
//! again. Nothing changed means no wake, ever: this is a fact check, not a timer on a model.

use chrono::{DateTime, Utc};
use sqlx::Row as _;

use crate::{ensure_direct_message_room_in_tx, OrgIntel, Result};

/// How long the owner goes without hearing from Exec, while Work moves, before an update is due.
pub const PROGRESS_UPDATE_AFTER: chrono::Duration = chrono::Duration::hours(2);

/// What moved since the owner last heard from Exec (or since Exec was last asked for an update).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressDue {
    /// When the owner last heard, or when Exec was last asked; `None` if neither ever happened.
    pub last_update: Option<DateTime<Utc>>,
    /// The first change since then.
    pub first_change: Option<DateTime<Utc>>,
    /// Attempts that finished with a result.
    pub landed: i64,
    /// Attempts that ended blocked or failed.
    pub stuck: i64,
    pub goals_closed: i64,
    /// Attempts running now.
    pub running: i64,
}

impl ProgressDue {
    pub fn changes(&self) -> i64 {
        self.landed + self.stuck + self.goals_closed
    }

    /// Whether an update is due at `now`: something changed, and the owner has gone the whole
    /// threshold without one. With no update ever sent, the wait counts from the first change.
    pub fn due(&self, now: DateTime<Utc>) -> bool {
        if self.changes() == 0 {
            return false;
        }
        match self.last_update.or(self.first_change) {
            Some(since) => now - since >= PROGRESS_UPDATE_AFTER,
            None => false,
        }
    }

    /// The short trigger Exec gets. Counts only: Exec reads the Work itself.
    pub fn message(&self, now: DateTime<Utc>) -> String {
        let quiet = self
            .last_update
            .or(self.first_change)
            .map_or(0, |since| (now - since).num_hours());
        let mut moved = Vec::new();
        if self.landed > 0 {
            moved.push(format!("{} finished", self.landed));
        }
        if self.stuck > 0 {
            moved.push(format!("{} blocked or failed", self.stuck));
        }
        if self.goals_closed > 0 {
            moved.push(format!("{} goal(s) closed", self.goals_closed));
        }
        moved.push(format!("{} running now", self.running));
        format!(
            "Progress update due: the owner has had no update for {quiet} hours while work moved \
             ({}). Post one short update in their chat: what landed, what is running, what is \
             stuck and what you are doing about it. A few lines at most, one update for \
             everything. If they already know all of it, stay quiet.",
            moved.join(", ")
        )
    }
}

impl OrgIntel {
    /// What moved since the owner last heard from Exec, as of `now`.
    pub async fn progress_since_last_update_at(&self, now: DateTime<Utc>) -> Result<ProgressDue> {
        // `heard` is when the owner last heard (or Exec was last asked); `seen` is how far the
        // changes were already counted. They are the same moment except under a test's injected
        // clock, where the ask's time is injected and its changes are real.
        let row = sqlx::query(
            "WITH told AS (SELECT max(m.created_at) AS at FROM messages m \
                   WHERE m.from_actor='exec' AND (m.to_actor IS NULL OR m.to_actor='owner' \
                     OR EXISTS (SELECT 1 FROM human_principal_actor_bindings b \
                                WHERE b.actor_id=m.to_actor AND b.membership_role='owner' \
                                  AND b.membership_status='active'))), \
             asked AS (SELECT max(e.created_at) AS at, \
                              max((e.body->>'through')::timestamptz) AS through \
                       FROM events e WHERE e.kind='progress_due'), \
             mark AS (SELECT GREATEST(told.at, asked.at) AS heard, \
                             GREATEST(told.at, asked.through) AS seen FROM told, asked) \
             SELECT mark.heard AS last_update, \
               (SELECT min(a.finished_at) FROM work_attempts a \
                 WHERE a.finished_at <= $1 AND (mark.seen IS NULL OR a.finished_at > mark.seen) \
                   AND a.state IN ('produced','changes_requested','blocked','failed')) AS first_change, \
               (SELECT count(*) FROM work_attempts a \
                 WHERE a.finished_at <= $1 AND (mark.seen IS NULL OR a.finished_at > mark.seen) \
                   AND a.state IN ('produced','changes_requested')) AS landed, \
               (SELECT count(*) FROM work_attempts a \
                 WHERE a.finished_at <= $1 AND (mark.seen IS NULL OR a.finished_at > mark.seen) \
                   AND a.state IN ('blocked','failed')) AS stuck, \
               (SELECT count(*) FROM events e WHERE e.kind='goal_closed' AND e.created_at <= $1 \
                   AND (mark.seen IS NULL OR e.created_at > mark.seen)) AS goals_closed, \
               (SELECT count(*) FROM work_attempts a WHERE a.state='running') AS running \
             FROM mark",
        )
        .bind(now)
        .fetch_one(&self.pool)
        .await?;
        Ok(ProgressDue {
            last_update: row.get("last_update"),
            first_change: row.get("first_change"),
            landed: row.get("landed"),
            stuck: row.get("stuck"),
            goals_closed: row.get("goals_closed"),
            running: row.get("running"),
        })
    }

    /// Wake Exec for one progress update if one is due at `now`, and move the mark so the same
    /// changes never ask twice. Not while a message from the owner is still unread: Exec is about
    /// to answer them anyway. Returns what moved when Exec was asked.
    pub async fn signal_progress_due_at(&self, now: DateTime<Utc>) -> Result<Option<ProgressDue>> {
        if !self.progress_since_last_update_at(now).await?.due(now) {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        // Two planes sweeping the same company: one asks, the other sees the moved mark.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('progress_due'))")
            .execute(&mut *tx)
            .await?;
        let owner_waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM messages m WHERE m.to_actor='exec' AND m.read_at IS NULL \
               AND (m.from_actor='owner' OR EXISTS (SELECT 1 FROM human_principal_actor_bindings b \
                    WHERE b.actor_id=m.from_actor AND b.membership_role='owner' \
                      AND b.membership_status='active')))",
        )
        .fetch_one(&mut *tx)
        .await?;
        if owner_waiting {
            tx.rollback().await?;
            return Ok(None);
        }
        // Read again under the lock: the other plane's ask has moved the mark.
        let due = self.progress_since_last_update_at(now).await?;
        if !due.due(now) {
            tx.rollback().await?;
            return Ok(None);
        }
        let room_id = ensure_direct_message_room_in_tx(&mut tx, "daemon", Some("exec")).await?;
        sqlx::query(
            "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,'daemon','exec',$2)",
        )
        .bind(room_id)
        .bind(due.message(now))
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body, created_at) \
             VALUES ('progress_due','daemon',$1 || jsonb_build_object('through', now()),$2)",
        )
        .bind(serde_json::json!({
            "landed": due.landed,
            "stuck": due.stuck,
            "goals_closed": due.goals_closed,
            "running": due.running,
        }))
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(due))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hours: i64, minutes: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-01T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + chrono::Duration::hours(hours)
            + chrono::Duration::minutes(minutes)
    }

    fn moved(last_update: Option<DateTime<Utc>>, landed: i64, stuck: i64) -> ProgressDue {
        ProgressDue {
            last_update,
            first_change: Some(at(0, 30)),
            landed,
            stuck,
            goals_closed: 0,
            running: 2,
        }
    }

    #[test]
    fn nothing_changed_never_asks_however_long_the_owner_has_waited() {
        let quiet = ProgressDue {
            first_change: None,
            ..moved(Some(at(0, 0)), 0, 0)
        };
        assert!(!quiet.due(at(2, 0)));
        assert!(!quiet.due(at(500, 0)));
    }

    #[test]
    fn an_update_is_due_two_hours_after_the_owner_last_heard_once_work_moved() {
        let work = moved(Some(at(0, 0)), 1, 0);
        assert!(!work.due(at(1, 59)), "inside the threshold");
        assert!(work.due(at(2, 0)));
        // Only something stuck is still worth an update.
        assert!(moved(Some(at(0, 0)), 0, 1).due(at(2, 0)));
        // Never heard from Exec at all: the wait counts from the first change.
        let first = moved(None, 3, 0);
        assert!(!first.due(at(2, 0)));
        assert!(first.due(at(2, 30)));
    }

    #[test]
    fn the_ask_is_one_short_message_with_counts_only() {
        let body = moved(Some(at(0, 0)), 3, 1).message(at(3, 0));
        assert!(body.contains("no update for 3 hours"), "{body}");
        assert!(
            body.contains("3 finished, 1 blocked or failed, 2 running now"),
            "{body}"
        );
        assert!(body.contains("one update for everything"), "{body}");
        assert!(body.len() < 420, "{}", body.len());
    }
}
