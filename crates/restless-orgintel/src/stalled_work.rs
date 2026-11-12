//! Work that has stopped with nobody asked (Sprint 109 blockers audit, finding 10).
//!
//! Blocked Work is never claimed again, and its "accountable recovery" used to be one message to
//! its lead (Frankensurf's 0.50.4 release sat blocked from 19 Oct). Work that requires abandoned
//! Work can never start at all (finding 9). This module finds both, so the owner's Inbox carries
//! one card per stalled Work item and its lead is told once per revision.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row as _;
use uuid::Uuid;

use crate::{ensure_direct_message_room_in_tx, OrgIntel, Result};

/// How long Work may sit stopped before the owner gets a card for it.
pub const STALLED_AFTER: chrono::Duration = chrono::Duration::hours(24);

/// Why stalled Work can't move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StallCause {
    /// Blocked, with no pending owner handoff, and nobody resumed it. `reason` is the block's own
    /// words.
    Blocked { reason: String },
    /// It requires Work that was abandoned, so it can never start as it is.
    AbandonedRequirement {
        upstream_id: Uuid,
        upstream_title: String,
    },
}

/// How the platform stopped Work that had nothing wrong with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformStop {
    /// A restart, or a supervisor that ended without recording an outcome.
    Restart,
    /// A watchdog cut the turn; the words are the block's own.
    Interruption(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct StalledWork {
    pub work_id: Uuid,
    pub title: String,
    /// The Work's lead: who it waits on.
    pub lead: String,
    pub revision: i64,
    /// When it stopped: the block, or the requirement's abandonment.
    pub since: DateTime<Utc>,
    pub cause: StallCause,
}

impl StalledWork {
    /// The one sentence that says how to get it moving.
    pub fn way_through(&self) -> String {
        match &self.cause {
            StallCause::Blocked { .. } => "Once the cause is fixed, resume it; if it is no longer \
                wanted, stop it for good. Either is on the Work's page, or ask its lead in chat."
                .into(),
            StallCause::AbandonedRequirement { .. } => "Drop the requirement on the abandoned \
                Work, or stop this Work for good too. Its lead can drop the requirement; you can \
                stop it on the Work's page."
                .into(),
        }
    }

    /// The idempotency key of the lead's notice for this exact stall.
    fn notice_key(&self) -> String {
        match &self.cause {
            StallCause::Blocked { .. } => format!("blocked:{}:{}", self.work_id, self.revision),
            StallCause::AbandonedRequirement { upstream_id, .. } => {
                format!("requires-abandoned:{}:{upstream_id}", self.work_id)
            }
        }
    }

    fn lead_notice(&self) -> String {
        let since = self.since.format("%-d %b %H:%M UTC");
        match &self.cause {
            StallCause::Blocked { reason } => format!(
                "Work {} (\"{}\") has been blocked since {since} and nobody has picked it up: \
                 {}. The owner now has a card for it. Fix the cause and run `restless work resume \
                 --work {} --reason <what changed>`; if only the owner can unblock it, tell Exec \
                 exactly what is needed; if it is no longer wanted, abandon it.",
                self.work_id,
                self.title,
                if reason.trim().is_empty() {
                    "no reason was recorded"
                } else {
                    reason.trim()
                },
                self.work_id,
            ),
            StallCause::AbandonedRequirement {
                upstream_id,
                upstream_title,
            } => format!(
                "Work {} (\"{}\") requires Work {upstream_id} (\"{upstream_title}\"), which was \
                 abandoned, so it can never start as it is. The owner now has a card for it. Either \
                 drop the requirement with `restless work edge --from {upstream_id} --to {} --kind \
                 requires --remove --reason <why>`, or abandon it too.",
                self.work_id, self.title, self.work_id,
            ),
        }
    }
}

impl OrgIntel {
    /// Blocked Work whose Attempt stopped on a model provider's refusal (`[quota] …`: credit,
    /// usage or rate limits; `[credential] …`: a refused sign-in or key), with nothing wrong in
    /// the Work itself. Once no such refusal is active, it can simply go on (S109 blockers audit,
    /// finding 15: "Work resumes within minutes of a top-up" was not true for it).
    pub async fn work_blocked_by_provider_refusal(&self) -> Result<Vec<Uuid>> {
        Ok(sqlx::query_scalar(
            "SELECT w.id FROM work w WHERE w.status = 'blocked' \
               AND (w.resolution LIKE '[quota] %' OR w.resolution LIKE '[credential] %') \
               AND NOT EXISTS (SELECT 1 FROM owner_handoffs h \
                               WHERE h.work_id = w.id AND h.state = 'pending') \
             ORDER BY w.updated_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Blocked Work the platform itself stopped, with nothing wrong in the Work: a restart or a
    /// lost supervisor closed its Attempt as unknown (`restart`), or a watchdog cut its turn
    /// (`interruption`; never one the owner interrupted). The daemon resumes these itself,
    /// within its daily bound, so a restart never leaves Work for the owner to resume by hand
    /// (12 November: the resume was once per revision for good, so a second restart stranded it).
    /// Only Work stopped in the last two days: enough for the daily bound to free up, and Work
    /// that has sat blocked for longer is not started again without a person.
    pub async fn work_stopped_by_platform(&self) -> Result<Vec<(Uuid, PlatformStop)>> {
        let rows = sqlx::query(
            "SELECT w.id, w.resolution FROM work w WHERE w.status = 'blocked' \
               AND w.updated_at > now() - interval '2 days' \
               AND (w.resolution LIKE $1 OR w.resolution LIKE $2 \
                    OR (w.resolution LIKE '[resumable] %' \
                        AND w.resolution NOT LIKE '%the owner interrupted%')) \
               AND NOT EXISTS (SELECT 1 FROM owner_handoffs h \
                               WHERE h.work_id = w.id AND h.state = 'pending') \
             ORDER BY w.updated_at",
        )
        .bind(format!(
            "{}supervisor restarted%",
            crate::UNKNOWN_OUTCOME_PREFIX
        ))
        .bind(format!(
            "{}its supervising process ended without recording an outcome%",
            crate::UNKNOWN_OUTCOME_PREFIX
        ))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let resolution: String = row.get("resolution");
                let stop = if resolution.starts_with("[resumable] ") {
                    PlatformStop::Interruption(resolution)
                } else {
                    PlatformStop::Restart
                };
                (row.get("id"), stop)
            })
            .collect())
    }

    /// Work stopped for longer than `after` with nobody asked:
    /// - blocked, with no pending owner handoff (a handoff has its own card), not paused on
    ///   purpose by a person, and not parked until a time that is still to come (its schedule
    ///   releases it; a card for it told the owner to resume Work that was only waiting);
    /// - open, but requiring Work that was abandoned that long ago.
    pub async fn stalled_work(&self, after: chrono::Duration) -> Result<Vec<StalledWork>> {
        let cutoff = Utc::now() - after;
        let mut stalled = Vec::new();
        let blocked = sqlx::query(
            "SELECT w.id, w.title, w.owner_id, w.revision, w.resolution, w.updated_at FROM work w \
             WHERE w.status = 'blocked' AND w.updated_at < $1 \
               AND w.resolution NOT LIKE 'paused by %' \
               AND NOT EXISTS (SELECT 1 FROM owner_handoffs h \
                               WHERE h.work_id = w.id AND h.state = 'pending') \
               AND NOT EXISTS (SELECT 1 FROM schedules s WHERE s.work_id = w.id \
                               AND s.fired_at IS NULL AND s.cancelled_at IS NULL) \
             ORDER BY w.updated_at",
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;
        for row in blocked {
            stalled.push(StalledWork {
                work_id: row.get("id"),
                title: row.get("title"),
                lead: row.get("owner_id"),
                revision: row.get("revision"),
                since: row.get("updated_at"),
                cause: StallCause::Blocked {
                    reason: row.get("resolution"),
                },
            });
        }
        let stranded = sqlx::query(
            "SELECT DISTINCT ON (w.id) w.id, w.title, w.owner_id, w.revision, \
                    u.id AS upstream_id, u.title AS upstream_title, u.updated_at AS abandoned_at \
             FROM work_edges e JOIN work w ON w.id = e.to_work_id JOIN work u ON u.id = e.from_work_id \
             WHERE e.kind = 'requires' AND u.status = 'abandoned' AND u.updated_at < $1 \
               AND w.status IN ('proposed','active') \
             ORDER BY w.id, u.updated_at",
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;
        for row in stranded {
            stalled.push(StalledWork {
                work_id: row.get("id"),
                title: row.get("title"),
                lead: row.get("owner_id"),
                revision: row.get("revision"),
                since: row.get("abandoned_at"),
                cause: StallCause::AbandonedRequirement {
                    upstream_id: row.get("upstream_id"),
                    upstream_title: row.get("upstream_title"),
                },
            });
        }
        stalled.sort_by_key(|work| work.since);
        Ok(stalled)
    }

    /// Ask whoever briefed a pending owner judgement to brief it again once its Work has changed
    /// since: the Inbox keeps the card, marked as written for an earlier version, and a stale
    /// outcome review can't be accepted until it is re-briefed (S109 blockers audit, finding 18).
    /// Once per handoff and Work revision. Returns how many were asked.
    pub async fn ask_for_stale_owner_briefs(&self) -> Result<usize> {
        let mut asked = 0;
        for handoff in self.list_owner_handoffs().await? {
            if handoff.state != crate::OwnerHandoffState::Pending
                || handoff.category != crate::OwnerHandoffCategory::OwnerJudgement
                || handoff.assigned_to.is_some()
                || handoff.owner_brief.is_none()
            {
                continue;
            }
            let Some(work) = self.get_work(handoff.work_id).await? else {
                continue;
            };
            if handoff.owner_brief_is_current(work.revision) {
                continue;
            }
            let key = format!("stale-brief:{}:{}", handoff.id, work.revision);
            let mut tx = self.pool.begin().await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
                .bind(&key)
                .execute(&mut *tx)
                .await?;
            let already: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM events WHERE kind='work_stalled_notice' \
                   AND body->>'key' = $1)",
            )
            .bind(&key)
            .fetch_one(&mut *tx)
            .await?;
            if already {
                continue;
            }
            let to = handoff
                .briefed_by
                .clone()
                .filter(|actor| !matches!(actor.as_str(), "owner" | "world" | "daemon"))
                .unwrap_or_else(|| "exec".into());
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1)")
                    .bind(&to)
                    .fetch_one(&mut *tx)
                    .await?;
            if exists {
                let body = format!(
                    "The owner's Inbox still holds your brief for handoff {} on Work {} (\"{}\"), but \
                     the Work has changed since you wrote it, so the card is marked as out of date \
                     and an outcome review can't be accepted. Brief it again for the Work as it is \
                     now with `restless work prepare-owner-brief --handoff {}`, or resolve the \
                     handoff if it no longer applies.",
                    handoff.id, work.id, work.title, handoff.id
                );
                let room_id =
                    ensure_direct_message_room_in_tx(&mut tx, "daemon", Some(&to)).await?;
                sqlx::query(
                    "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,'daemon',$2,$3)",
                )
                .bind(room_id)
                .bind(&to)
                .bind(body)
                .execute(&mut *tx)
                .await?;
                asked += 1;
            }
            sqlx::query(
                "INSERT INTO events (kind, actor_id, body) VALUES ('work_stalled_notice','daemon',$1)",
            )
            .bind(serde_json::json!({
                "key": key,
                "work_id": work.id,
                "revision": work.revision,
                "handoff_id": handoff.id,
                "told": exists.then_some(to),
            }))
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        Ok(asked)
    }

    /// Tell each stalled Work's lead, once per stall, what stopped it and the way through. The
    /// message comes from the daemon, never in Exec's name. Returns how many leads were told.
    pub async fn tell_leads_of_stalled_work(&self, after: chrono::Duration) -> Result<usize> {
        let mut told = 0;
        for stalled in self.stalled_work(after).await? {
            let mut tx = self.pool.begin().await?;
            let key = stalled.notice_key();
            // One notice per stall, even with two planes sweeping the same company at once.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
                .bind(&key)
                .execute(&mut *tx)
                .await?;
            let already: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM events WHERE kind='work_stalled_notice' \
                   AND body->>'key' = $1)",
            )
            .bind(&key)
            .fetch_one(&mut *tx)
            .await?;
            if already {
                continue;
            }
            // Owner-led Work goes to Exec, who answers for the company's Work.
            let to = if matches!(stalled.lead.as_str(), "owner" | "world" | "daemon") {
                "exec".to_string()
            } else {
                stalled.lead.clone()
            };
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1)")
                    .bind(&to)
                    .fetch_one(&mut *tx)
                    .await?;
            if exists {
                let room_id =
                    ensure_direct_message_room_in_tx(&mut tx, "daemon", Some(&to)).await?;
                sqlx::query(
                    "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,'daemon',$2,$3)",
                )
                .bind(room_id)
                .bind(&to)
                .bind(stalled.lead_notice())
                .execute(&mut *tx)
                .await?;
                told += 1;
            }
            sqlx::query(
                "INSERT INTO events (kind, actor_id, body) VALUES ('work_stalled_notice','daemon',$1)",
            )
            .bind(serde_json::json!({
                "key": key,
                "work_id": stalled.work_id,
                "revision": stalled.revision,
                "told": exists.then_some(to),
            }))
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        Ok(told)
    }
}
