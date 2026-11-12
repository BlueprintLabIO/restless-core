//! Blocked work that waits on something outside the company is looked at again without the owner
//! saying "try now" (owner decision, 12 November 2026).
//!
//! Two kinds of wait are typed today, so no text is guessed at:
//! - a sign-in, code or CAPTCHA handoff owed to the owner (an identity, MFA or CAPTCHA step);
//! - Work whose own Attempt ended `blocked`, with no handoff and nobody pausing it.
//!
//! Each gets a short daemon message to the agent that is waiting, on a bounded backoff, and once
//! more shortly after the owner did something that plausibly changed it (a chat message, closing
//! the computer's viewer, saving an account). The agent checks the real thing and either goes on
//! or says in one line that nothing changed. Every recheck is an event, which is also the count
//! that bounds them. A wait an app or account answers is resolved by observation in the engine and
//! never costs a model turn.

use chrono::{DateTime, Utc};
use sqlx::Row as _;
use uuid::Uuid;

use crate::{ensure_direct_message_room_in_tx, OrgIntel, Result};

/// A waiting agent is asked to look again this long after the wait began, then stops.
pub const RECHECK_AFTER: [chrono::Duration; 3] = [
    chrono::Duration::hours(1),
    chrono::Duration::hours(6),
    chrono::Duration::hours(24),
];
/// How long after the owner did something before the agent looks: long enough for the owner to
/// finish, and for Exec to settle it first when the owner said so in chat.
pub const OWNER_ACTIVITY_SETTLE: chrono::Duration = chrono::Duration::minutes(2);
/// Rechecks of one wait are at least this far apart, whatever triggers them.
pub const RECHECK_GAP: chrono::Duration = chrono::Duration::minutes(15);
/// At most this many rechecks of one wait come from the owner being active.
pub const OWNER_ACTIVITY_RECHECKS: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecheckTrigger {
    Timed,
    OwnerActivity,
}

impl RecheckTrigger {
    fn as_str(self) -> &'static str {
        match self {
            Self::Timed => "timed",
            Self::OwnerActivity => "owner_activity",
        }
    }
}

/// What has been asked of one wait so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecheckHistory {
    pub timed: i64,
    pub owner_activity: i64,
    pub last_at: Option<DateTime<Utc>>,
}

/// When this wait is next looked at, and why; `None` once its rechecks are used up and the owner
/// has done nothing new. Pure, so the backoff is tested with any clock.
pub fn next_recheck(
    since: DateTime<Utc>,
    history: RecheckHistory,
    owner_active_at: Option<DateTime<Utc>>,
) -> Option<(DateTime<Utc>, RecheckTrigger)> {
    let timed = usize::try_from(history.timed)
        .ok()
        .and_then(|done| RECHECK_AFTER.get(done))
        .map(|after| (since + *after, RecheckTrigger::Timed));
    let seen_until = history.last_at.map_or(since, |last| last.max(since));
    let active = owner_active_at
        .filter(|at| *at > seen_until && history.owner_activity < OWNER_ACTIVITY_RECHECKS)
        .map(|at| (at + OWNER_ACTIVITY_SETTLE, RecheckTrigger::OwnerActivity));
    let (at, trigger) = match (timed, active) {
        (Some(timed), Some(active)) if active.0 < timed.0 => active,
        (Some(timed), _) => timed,
        (None, Some(active)) => active,
        (None, None) => return None,
    };
    let earliest = history
        .last_at
        .map_or(at, |last| at.max(last + RECHECK_GAP));
    Some((earliest, trigger))
}

/// One blocked thing waiting on something outside the company.
#[derive(Debug, Clone)]
pub struct ExternalWait {
    /// The key its recheck events carry: `handoff:<id>` or `work:<id>:<revision>`.
    pub subject: String,
    pub handoff_id: Option<Uuid>,
    pub work_id: Uuid,
    pub work_title: String,
    /// The agent that is waiting and is asked to look again.
    pub agent: String,
    /// What it waits on, in the block's own words.
    pub waiting_on: String,
    pub since: DateTime<Utc>,
    pub history: RecheckHistory,
}

fn first_line(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let mut short: String = line.chars().take(240).collect();
    if line.chars().count() > 240 {
        short.push('…');
    }
    short
}

fn age_words(age: chrono::Duration) -> String {
    let hours = age.num_hours();
    if hours >= 48 {
        format!("{} days", hours / 24)
    } else if hours >= 1 {
        format!("{hours} hour{}", if hours == 1 { "" } else { "s" })
    } else {
        format!("{} minutes", age.num_minutes().max(1))
    }
}

impl ExternalWait {
    /// The short trigger the waiting agent gets.
    pub fn recheck_message(&self, trigger: RecheckTrigger, now: DateTime<Utc>) -> String {
        let age = age_words(now - self.since);
        let why = match trigger {
            RecheckTrigger::Timed => "",
            RecheckTrigger::OwnerActivity => " The owner has just been active.",
        };
        match self.handoff_id {
            Some(handoff) => format!(
                "Recheck: Work {} (\"{}\") has waited {age} on the owner for: {}.{why} Check \
                 whether what you were waiting for has changed (try the real thing, for example \
                 the sign-in). If it is done, record what you saw with `restless work \
                 resolve-handoff --handoff {handoff} --state resolved --resolution <what you \
                 checked> --as {}` and the Work continues. Otherwise say so in one line and stay \
                 blocked.",
                self.work_id,
                self.work_title,
                first_line(&self.waiting_on),
                self.agent,
            ),
            None => format!(
                "Recheck: Work {} (\"{}\") has been blocked for {age}: {}.{why} Check whether \
                 what you were waiting for has changed (try the real thing). If it has, continue \
                 with `restless work resume --work {} --reason <what changed>`. Otherwise say so \
                 in one line and stay blocked.",
                self.work_id,
                self.work_title,
                first_line(&self.waiting_on),
                self.work_id,
            ),
        }
    }
}

const HISTORY_SQL: &str = "LEFT JOIN LATERAL ( \
    SELECT count(*) FILTER (WHERE e.body->>'trigger'='timed') AS timed, \
           count(*) FILTER (WHERE e.body->>'trigger'='owner_activity') AS owner_activity, \
           max(e.created_at) AS last_at \
    FROM events e WHERE e.kind='block_recheck' AND e.body->>'subject' = s.subject) r ON true";

impl OrgIntel {
    /// Everything blocked on something outside the company that an agent could look at again.
    /// Work with a running Attempt is left alone (that process is still on it), as is anything an
    /// app or account answers, a legal attestation, a payment and a judgement.
    pub async fn external_waits(&self) -> Result<Vec<ExternalWait>> {
        let rows = sqlx::query(&format!(
            "SELECT s.subject, s.handoff_id, s.work_id, s.title, s.agent, s.waiting_on, s.since, \
                    COALESCE(r.timed,0) AS timed, COALESCE(r.owner_activity,0) AS owner_activity, \
                    r.last_at \
             FROM ( \
               SELECT 'handoff:' || h.id::text AS subject, h.id AS handoff_id, w.id AS work_id, \
                      w.title, w.owner_id AS agent, h.requested_action AS waiting_on, \
                      h.created_at AS since \
               FROM owner_handoffs h JOIN work w ON w.id=h.work_id \
               WHERE h.state='pending' AND h.assigned_to IS NULL AND h.app IS NULL \
                 AND h.category IN ('identity','mfa','captcha') AND w.status='blocked' \
                 AND NOT EXISTS (SELECT 1 FROM work_attempts a \
                                 WHERE a.work_id=w.id AND a.state='running') \
               UNION ALL \
               SELECT 'work:' || w.id::text || ':' || w.revision::text, NULL, w.id, w.title, \
                      w.owner_id, w.resolution, w.updated_at \
               FROM work w \
               WHERE w.status='blocked' \
                 AND w.resolution NOT LIKE 'paused by %' \
                 AND w.resolution NOT LIKE '[quota] %' AND w.resolution NOT LIKE '[credential] %' \
                 AND w.resolution NOT LIKE 'awaiting %' \
                 AND NOT EXISTS (SELECT 1 FROM owner_handoffs h WHERE h.work_id=w.id \
                                 AND h.state IN ('pending','preparing')) \
                 AND NOT EXISTS (SELECT 1 FROM native_document_work_review_dependencies d \
                                 WHERE d.work_id=w.id AND d.status='pending') \
                 AND (SELECT a.state FROM work_attempts a \
                      WHERE a.work_id=w.id AND a.revision=w.revision \
                      ORDER BY a.attempt_no DESC LIMIT 1) = 'blocked' \
             ) s {HISTORY_SQL} \
             ORDER BY s.since, s.subject"
        ))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let agent: String = row.get("agent");
                ExternalWait {
                    subject: row.get("subject"),
                    handoff_id: row.get("handoff_id"),
                    work_id: row.get("work_id"),
                    work_title: row.get("title"),
                    // Owner-led Work goes to Exec, who answers for the company's Work.
                    agent: if matches!(agent.as_str(), "owner" | "world" | "daemon") {
                        "exec".into()
                    } else {
                        agent
                    },
                    waiting_on: row.get("waiting_on"),
                    since: row.get("since"),
                    history: RecheckHistory {
                        timed: row.get("timed"),
                        owner_activity: row.get("owner_activity"),
                        last_at: row.get("last_at"),
                    },
                }
            })
            .collect())
    }

    /// When the owner last did something that could have changed a wait: a message of theirs to
    /// Exec, or an act the owner API noted with [`Self::note_owner_activity`].
    pub async fn owner_last_active_at(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(sqlx::query_scalar(
            "SELECT GREATEST( \
               (SELECT max(m.created_at) FROM messages m \
                 WHERE m.to_actor='exec' AND (m.from_actor='owner' OR EXISTS ( \
                   SELECT 1 FROM human_principal_actor_bindings b \
                   WHERE b.actor_id=m.from_actor AND b.membership_role='owner' \
                     AND b.membership_status='active'))), \
               (SELECT max(e.created_at) FROM events e WHERE e.kind='owner_activity'))",
        )
        .fetch_one(&self.pool)
        .await?)
    }

    /// The owner did something that may have settled a wait (`what`: `viewer_closed`,
    /// `account_saved`). Written only while something waits, and at most once a minute, so an
    /// idle company's event stream is untouched. Returns whether it was noted.
    pub async fn note_owner_activity(&self, what: &str) -> Result<bool> {
        if self.external_waits().await?.is_empty() {
            return Ok(false);
        }
        Ok(sqlx::query(
            "INSERT INTO events (kind, actor_id, body) \
             SELECT 'owner_activity','owner',$1 \
             WHERE NOT EXISTS (SELECT 1 FROM events WHERE kind='owner_activity' \
                               AND created_at > now() - interval '1 minute')",
        )
        .bind(serde_json::json!({ "what": what }))
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1)
    }

    /// The earliest moment any wait is due another look, so the scheduler sleeps until then.
    pub async fn next_block_recheck_at(&self) -> Result<Option<DateTime<Utc>>> {
        let waits = self.external_waits().await?;
        if waits.is_empty() {
            return Ok(None);
        }
        let active = self.owner_last_active_at().await?;
        Ok(waits
            .iter()
            .filter_map(|wait| next_recheck(wait.since, wait.history, active))
            .map(|(at, _)| at)
            .min())
    }

    /// Ask every waiting agent whose recheck is due at `now` to look again. One message and one
    /// event per recheck; the event is what bounds them. Returns how many agents were asked.
    pub async fn recheck_external_waits_at(&self, now: DateTime<Utc>) -> Result<usize> {
        let waits = self.external_waits().await?;
        if waits.is_empty() {
            return Ok(0);
        }
        let active = self.owner_last_active_at().await?;
        let mut asked = 0;
        for wait in waits {
            let Some((due, _)) = next_recheck(wait.since, wait.history, active) else {
                continue;
            };
            if due > now {
                continue;
            }
            let mut tx = self.pool.begin().await?;
            // One recheck per due moment, even with two planes sweeping the same company.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
                .bind(&wait.subject)
                .execute(&mut *tx)
                .await?;
            let row = sqlx::query(
                "SELECT count(*) FILTER (WHERE body->>'trigger'='timed') AS timed, \
                        count(*) FILTER (WHERE body->>'trigger'='owner_activity') AS owner_activity, \
                        max(created_at) AS last_at \
                 FROM events WHERE kind='block_recheck' AND body->>'subject'=$1",
            )
            .bind(&wait.subject)
            .fetch_one(&mut *tx)
            .await?;
            let history = RecheckHistory {
                timed: row.get("timed"),
                owner_activity: row.get("owner_activity"),
                last_at: row.get("last_at"),
            };
            let trigger = match next_recheck(wait.since, history, active) {
                Some((due, trigger)) if due <= now => trigger,
                _ => {
                    tx.rollback().await?;
                    continue;
                }
            };
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1)")
                    .bind(&wait.agent)
                    .fetch_one(&mut *tx)
                    .await?;
            if exists {
                let room_id =
                    ensure_direct_message_room_in_tx(&mut tx, "daemon", Some(&wait.agent)).await?;
                sqlx::query(
                    "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,'daemon',$2,$3)",
                )
                .bind(room_id)
                .bind(&wait.agent)
                .bind(wait.recheck_message(trigger, now))
                .execute(&mut *tx)
                .await?;
                asked += 1;
            }
            sqlx::query(
                "INSERT INTO events (kind, actor_id, body) VALUES ('block_recheck','daemon',$1)",
            )
            .bind(serde_json::json!({
                "subject": wait.subject,
                "trigger": trigger.as_str(),
                "work_id": wait.work_id,
                "handoff_id": wait.handoff_id,
                "asked": exists.then_some(&wait.agent),
            }))
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        Ok(asked)
    }

    /// Whether `actor` may record that the sign-in, code or CAPTCHA step it is waiting on is done:
    /// it leads the Work, the step is still owed to the owner, and the daemon has asked it to
    /// look again. A legal attestation, a payment and a judgement stay the owner's.
    pub async fn agent_may_confirm_human_step(&self, handoff: Uuid, actor: &str) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM owner_handoffs h JOIN work w ON w.id=h.work_id \
               WHERE h.id=$1 AND h.state='pending' AND h.assigned_to IS NULL \
                 AND h.category IN ('identity','mfa','captcha') \
                 AND (w.owner_id=$2 OR h.requested_by=$2) \
                 AND EXISTS (SELECT 1 FROM events e WHERE e.kind='block_recheck' \
                             AND e.body->>'subject' = 'handoff:' || h.id::text))",
        )
        .bind(handoff)
        .bind(actor)
        .fetch_one(&self.pool)
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hours: i64, minutes: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + chrono::Duration::hours(hours)
            + chrono::Duration::minutes(minutes)
    }

    fn history(timed: i64, owner_activity: i64, last_at: Option<DateTime<Utc>>) -> RecheckHistory {
        RecheckHistory {
            timed,
            owner_activity,
            last_at,
        }
    }

    #[test]
    fn a_wait_is_rechecked_after_one_six_and_twenty_four_hours_then_never_again() {
        let since = at(0, 0);
        assert_eq!(
            next_recheck(since, history(0, 0, None), None),
            Some((at(1, 0), RecheckTrigger::Timed))
        );
        assert_eq!(
            next_recheck(since, history(1, 0, Some(at(1, 0))), None),
            Some((at(6, 0), RecheckTrigger::Timed))
        );
        assert_eq!(
            next_recheck(since, history(2, 0, Some(at(6, 0))), None),
            Some((at(24, 0), RecheckTrigger::Timed))
        );
        assert_eq!(
            next_recheck(since, history(3, 0, Some(at(24, 0))), None),
            None,
            "three timed rechecks, then it stops"
        );
    }

    #[test]
    fn the_owner_being_active_brings_one_recheck_forward_after_it_settles() {
        let since = at(0, 0);
        // Ten minutes in, the owner writes: the agent looks two minutes later, not at the hour.
        assert_eq!(
            next_recheck(since, history(0, 0, None), Some(at(0, 10))),
            Some((at(0, 12), RecheckTrigger::OwnerActivity))
        );
        // Activity from before the wait began, or before the last recheck, changes nothing.
        assert_eq!(
            next_recheck(since, history(0, 0, None), Some(at(-1, 0))),
            Some((at(1, 0), RecheckTrigger::Timed))
        );
        assert_eq!(
            next_recheck(since, history(0, 1, Some(at(0, 12))), Some(at(0, 10))),
            Some((at(1, 0), RecheckTrigger::Timed))
        );
        // After the timed rechecks are spent, the owner being active still gets one look.
        assert_eq!(
            next_recheck(since, history(3, 0, Some(at(24, 0))), Some(at(30, 0))),
            Some((at(30, 2), RecheckTrigger::OwnerActivity))
        );
    }

    #[test]
    fn rechecks_are_never_closer_than_the_gap_and_owner_activity_is_capped() {
        let since = at(0, 0);
        // The owner keeps chatting: the next look waits for the gap after the last one.
        assert_eq!(
            next_recheck(since, history(0, 1, Some(at(0, 12))), Some(at(0, 13))),
            Some((at(0, 27), RecheckTrigger::OwnerActivity))
        );
        // A daemon that was down past two thresholds does not fire them back to back.
        assert_eq!(
            next_recheck(since, history(1, 0, Some(at(7, 0))), None),
            Some((at(7, 15), RecheckTrigger::Timed))
        );
        // Five looks from owner activity, then only the timed ones remain.
        assert_eq!(
            next_recheck(
                since,
                history(3, OWNER_ACTIVITY_RECHECKS, Some(at(40, 0))),
                Some(at(50, 0))
            ),
            None
        );
    }

    #[test]
    fn the_recheck_names_the_way_on_and_stays_short() {
        let mut wait = ExternalWait {
            subject: "work:x:1".into(),
            handoff_id: None,
            work_id: Uuid::nil(),
            work_title: "Publish the release".into(),
            agent: "alpha-build".into(),
            waiting_on: "the registry token is missing\nsecond line".into(),
            since: at(0, 0),
            history: RecheckHistory::default(),
        };
        let work = wait.recheck_message(RecheckTrigger::Timed, at(6, 0));
        assert!(work.contains("blocked for 6 hours"), "{work}");
        assert!(work.contains("restless work resume --work"), "{work}");
        assert!(!work.contains("second line"), "{work}");
        wait.handoff_id = Some(Uuid::nil());
        let step = wait.recheck_message(RecheckTrigger::OwnerActivity, at(0, 12));
        assert!(step.contains("resolve-handoff --handoff"), "{step}");
        assert!(step.contains("--as alpha-build"), "{step}");
        assert!(step.contains("The owner has just been active."), "{step}");
        assert!(step.len() < 700, "{}", step.len());
    }
}
