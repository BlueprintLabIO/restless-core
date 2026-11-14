//! Exec checks in with the owner instead of waiting to be driven (owner decision, 13 November
//! 2026: the company felt like "a dumb chatbot that I have to keep prompting and driving").
//!
//! A check-in is an Exec turn with no owner message: a short update, the questions whose answers
//! would most improve the next hour, and what Exec will do meanwhile. The rhythm follows the
//! owner, never a bare clock:
//!
//! - While the owner engages (writes to Exec, or acts on an Inbox card) the next check-in is due
//!   [`CHECKIN_EVERY`] after the last one, or after Exec last wrote to them if that is later.
//! - A check-in the owner did not answer gets one short nudge [`CHECKIN_NUDGE_AFTER`] later that
//!   restates what is still open. After that nothing is posted until the owner returns; an hour
//!   after they do, the rhythm resumes.
//! - With no open goal, no open Work, nothing finished and nothing waiting on the owner, no
//!   model is woken at all. Nor is one woken for an owner who went quiet before the first
//!   check-in: that one follows their next message or card. A check-in that finds nothing worth saying ends quietly and still
//!   counts as the last one.
//!
//! This replaces the two-hour progress wake (`progress_due`, 12 November), so the two never
//! both fire. A check-in is a daemon message: it is not an owner message, an owner nudge or
//! owner activity anywhere else in the company.

use chrono::{DateTime, Utc};
use sqlx::Row as _;

use crate::{ensure_direct_message_room_in_tx, OrgIntel, Result};

/// How long after a check-in the next one is due, while the owner engages.
pub const CHECKIN_EVERY: chrono::Duration = chrono::Duration::hours(1);
/// How long an unanswered check-in waits before its one nudge.
pub const CHECKIN_NUDGE_AFTER: chrono::Duration = chrono::Duration::hours(3);
/// Unprompted posts into silence: the check-in and its one nudge. Then the owner must return.
pub const CHECKINS_INTO_SILENCE: i64 = 2;
/// The event every check-in leaves; the rhythm is read from these.
pub const CHECKIN_EVENT: &str = "exec_checkin";
/// How each trigger message to Exec starts.
pub const CHECKIN_MESSAGE_PREFIX: &str = "Check-in";

pub(crate) const OWNER_WROTE: &str = "(m.from_actor='owner' OR EXISTS (SELECT 1 FROM human_principal_actor_bindings b \
     WHERE b.actor_id=m.from_actor AND b.membership_role='owner' AND b.membership_status='active'))";
const TO_OWNER: &str = "(m.to_actor IS NULL OR m.to_actor='owner' OR EXISTS (SELECT 1 FROM human_principal_actor_bindings b \
     WHERE b.actor_id=m.to_actor AND b.membership_role='owner' AND b.membership_status='active'))";

/// The two intervals, so a test or a local stack can shorten them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckinRhythm {
    pub every: chrono::Duration,
    pub nudge_after: chrono::Duration,
}

impl Default for CheckinRhythm {
    fn default() -> Self {
        Self {
            every: CHECKIN_EVERY,
            nudge_after: CHECKIN_NUDGE_AFTER,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckinKind {
    /// An update, questions and the next hour.
    Checkin,
    /// One short restatement of what the last check-in left open.
    Nudge,
}

impl CheckinKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Checkin => "checkin",
            Self::Nudge => "nudge",
        }
    }
}

/// Everything the rhythm is decided from. Facts only; [`Self::next`] is pure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckinState {
    /// The owner (any owner of the company) turned check-ins off.
    pub enabled: bool,
    /// When Exec was last asked to check in.
    pub last: Option<DateTime<Utc>>,
    /// Check-ins asked for since the owner last engaged.
    pub unanswered: i64,
    /// Exec wrote to the owner after the last check-in was asked for.
    pub said_since_last: bool,
    /// When Exec last wrote to the owner.
    pub heard_at: Option<DateTime<Utc>>,
    /// When the owner first engaged after the last check-in; with no check-in yet, when they
    /// last engaged.
    pub engaged_at: Option<DateTime<Utc>>,
    pub open_goals: i64,
    /// Work not finished or abandoned, and how much of it is blocked.
    pub open_work: i64,
    pub blocked_work: i64,
    /// Attempts running now.
    pub running: i64,
    /// Attempts that ended with a result, and that ended blocked or failed, since the last
    /// check-in (or since Exec last wrote to the owner, before the first one).
    pub landed: i64,
    pub stuck: i64,
    /// Inbox items waiting on the owner.
    pub waiting_on_owner: i64,
}

impl CheckinState {
    /// Work moved since the last check-in.
    pub fn changes(&self) -> i64 {
        self.landed + self.stuck
    }

    /// Whether the company has anything a check-in could be about. Without it no model is woken.
    pub fn active(&self) -> bool {
        self.open_goals > 0 || self.open_work > 0 || self.changes() > 0 || self.waiting_on_owner > 0
    }

    /// When the next check-in is due and which kind, or `None` while nothing is owed: nothing
    /// active, check-ins off, or the owner has not come back since the nudge.
    pub fn next(&self, rhythm: CheckinRhythm) -> Option<(DateTime<Utc>, CheckinKind)> {
        if !self.enabled {
            return None;
        }
        match (self.unanswered, self.last) {
            // The owner is here (or there has been no check-in yet): the regular rhythm, counted
            // from whichever is latest of the last check-in, Exec's last word and their return.
            (0, _) => {
                if !self.active() {
                    return None;
                }
                let anchor = [self.last, self.heard_at, self.engaged_at]
                    .into_iter()
                    .flatten()
                    .max()?;
                Some((anchor + rhythm.every, CheckinKind::Checkin))
            }
            // One check-in went unanswered. If it said something, nudge once. If it ended
            // quietly there is nothing to restate: check in again only once Work has moved.
            (1, Some(last)) => {
                if self.said_since_last {
                    Some((last + rhythm.nudge_after, CheckinKind::Nudge))
                } else if self.changes() > 0 {
                    Some((last + rhythm.nudge_after, CheckinKind::Checkin))
                } else {
                    None
                }
            }
            // Never keep posting into silence.
            _ => None,
        }
    }

    /// The kind due at `now`, if any.
    ///
    /// Before the first check-in there is no "since the last one" to read engagement from, so
    /// the owner must have engaged recently: within the time a check-in and its nudge would
    /// have waited. A company whose owner went quiet long ago is not woken to post into that
    /// silence; its first check-in follows their next message or card.
    pub fn due(&self, now: DateTime<Utc>, rhythm: CheckinRhythm) -> Option<CheckinKind> {
        if self.last.is_none()
            && !self
                .engaged_at
                .is_some_and(|engaged| now - engaged <= rhythm.every + rhythm.nudge_after)
        {
            return None;
        }
        self.next(rhythm)
            .and_then(|(at, kind)| (at <= now).then_some(kind))
    }

    /// The trigger Exec gets. The instructions live here, not in the fixed prompt, so they cost
    /// tokens only on a check-in turn. Counts only: Exec reads the Work itself.
    pub fn message(&self, kind: CheckinKind, now: DateTime<Utc>) -> String {
        match kind {
            CheckinKind::Checkin => {
                let mut facts = Vec::new();
                if self.landed > 0 {
                    facts.push(format!("{} finished", self.landed));
                }
                if self.stuck > 0 {
                    facts.push(format!("{} blocked or failed", self.stuck));
                }
                facts.push(format!("{} running now", self.running));
                facts.push(format!("{} open", self.open_work));
                if self.blocked_work > 0 {
                    facts.push(format!("{} of them blocked", self.blocked_work));
                }
                if self.waiting_on_owner > 0 {
                    facts.push(format!("{} waiting on the owner", self.waiting_on_owner));
                }
                format!(
                    "{CHECKIN_MESSAGE_PREFIX} due. The owner has not written; this is you coming \
                     to them. Work since your last check-in: {}. Look at the Work, then write \
                     them one short message in plain words, with no ids and no commands:\n\
                     1. Update, two or three lines: what landed, what is running, what is next. \
                     Leave out what they already know.\n\
                     2. Up to three questions whose answers would most improve the next hour: a \
                     choice between options, something only they know, or one of their Inbox \
                     items that has sat too long. Number them, make each answerable in a word or \
                     two, and give your recommendation with each.\n\
                     3. One line on what you will do in the next hour without waiting for them. \
                     Then do it in this turn: start or steer that Work.\n\
                     Do not ask what you can decide yourself. If there is nothing new to tell and \
                     nothing to ask, stay quiet.",
                    facts.join(", ")
                )
            }
            CheckinKind::Nudge => {
                let hours = self.last.map_or(0, |last| (now - last).num_hours());
                let waited = match hours {
                    0 => "a while".to_string(),
                    1 => "an hour".to_string(),
                    hours => format!("{hours} hours"),
                };
                format!(
                    "{CHECKIN_MESSAGE_PREFIX} nudge. The owner has not answered your last \
                     check-in, {waited} ago. If it asked questions that are still open, restate \
                     them in one or two lines, each with your recommendation, and say what you \
                     are doing meanwhile. Plain words, no ids, no commands. This is not a new \
                     report. If it asked nothing, or everything it asked is settled, stay quiet. \
                     You will not be asked again until the owner is back, so keep the work going \
                     on your own recommendation where that is safe."
                )
            }
        }
    }
}

impl OrgIntel {
    /// The facts the check-in rhythm is decided from.
    ///
    /// "Since the last check-in" is told by row order, not by clocks: each check-in records the
    /// newest message and the moment it was written, so an owner message or card decision made
    /// after it is seen as engagement whatever time the scheduler believed it was.
    pub async fn checkin_state(&self) -> Result<CheckinState> {
        let row = sqlx::query(&format!(
            "WITH last AS ( \
               SELECT e.id, e.created_at AS at, \
                      (e.body->>'message_mark')::bigint AS mark, \
                      (e.body->>'through')::timestamptz AS through \
               FROM events e WHERE e.kind='{CHECKIN_EVENT}' ORDER BY e.id DESC LIMIT 1), \
             mark AS (SELECT (SELECT id FROM last) AS id, (SELECT at FROM last) AS at, \
                             (SELECT mark FROM last) AS mark, (SELECT through FROM last) AS through), \
             wrote AS (SELECT max(m.id) AS id, max(m.created_at) AS at FROM messages m \
                       WHERE m.to_actor='exec' AND {OWNER_WROTE}), \
             decided AS (SELECT max(d.created_at) AS at FROM owner_handoff_decisions d \
                         WHERE d.state <> 'canceled'), \
             acted AS (SELECT max(e.id) AS id, max(e.created_at) AS at FROM events e \
                       WHERE e.kind='owner_activity'), \
             heard AS (SELECT max(m.id) AS id, max(m.created_at) AS at FROM messages m \
                       WHERE m.from_actor='exec' AND {TO_OWNER}) \
             SELECT mark.at AS last, \
               NOT EXISTS (SELECT 1 FROM notification_preferences m \
                           WHERE NOT m.exec_checkins AND {OWNER_PREFERENCE}) AS enabled, \
               (SELECT count(*) FROM events e \
                 WHERE e.kind='{CHECKIN_EVENT}' \
                   AND (wrote.id IS NULL OR (e.body->>'message_mark')::bigint >= wrote.id) \
                   AND (decided.at IS NULL OR (e.body->>'through')::timestamptz >= decided.at) \
                   AND (acted.id IS NULL OR e.id > acted.id)) AS unanswered, \
               COALESCE(heard.id > mark.mark, false) AS said_since_last, \
               heard.at AS heard_at, \
               CASE WHEN mark.id IS NULL THEN GREATEST(wrote.at, decided.at, acted.at) \
                    ELSE LEAST( \
                      (SELECT min(m.created_at) FROM messages m \
                        WHERE m.to_actor='exec' AND m.id > mark.mark AND {OWNER_WROTE}), \
                      (SELECT min(d.created_at) FROM owner_handoff_decisions d \
                        WHERE d.state <> 'canceled' AND d.created_at > mark.through), \
                      (SELECT min(e.created_at) FROM events e \
                        WHERE e.kind='owner_activity' AND e.id > mark.id)) END AS engaged_at, \
               (SELECT count(*) FROM goals g WHERE g.closed_at IS NULL) AS open_goals, \
               (SELECT count(*) FROM work w \
                 WHERE w.status IN ('proposed','active','blocked')) AS open_work, \
               (SELECT count(*) FROM work w WHERE w.status='blocked') AS blocked_work, \
               (SELECT count(*) FROM work_attempts a WHERE a.state='running') AS running, \
               (SELECT count(*) FROM work_attempts a \
                 WHERE a.state IN ('produced','changes_requested') \
                   AND a.finished_at > COALESCE(mark.through, heard.at, '-infinity')) AS landed, \
               (SELECT count(*) FROM work_attempts a \
                 WHERE a.state IN ('blocked','failed') \
                   AND a.finished_at > COALESCE(mark.through, heard.at, '-infinity')) AS stuck, \
               (SELECT count(*) FROM owner_handoffs h \
                 WHERE h.state='pending' AND h.assigned_to IS NULL) AS waiting_on_owner \
             FROM mark, wrote, decided, acted, heard",
            OWNER_PREFERENCE = "(m.actor_id='owner' OR EXISTS (SELECT 1 FROM human_principal_actor_bindings b \
                 WHERE b.actor_id=m.actor_id AND b.membership_role='owner' \
                   AND b.membership_status='active'))",
        ))
        .fetch_one(&self.pool)
        .await?;
        Ok(CheckinState {
            enabled: row.get("enabled"),
            last: row.get("last"),
            unanswered: row.get("unanswered"),
            said_since_last: row.get("said_since_last"),
            heard_at: row.get("heard_at"),
            engaged_at: row.get("engaged_at"),
            open_goals: row.get("open_goals"),
            open_work: row.get("open_work"),
            blocked_work: row.get("blocked_work"),
            running: row.get("running"),
            landed: row.get("landed"),
            stuck: row.get("stuck"),
            waiting_on_owner: row.get("waiting_on_owner"),
        })
    }

    /// When the next check-in is due, so the scheduler can sleep until then.
    pub async fn next_checkin_at(&self, rhythm: CheckinRhythm) -> Result<Option<DateTime<Utc>>> {
        Ok(self.checkin_state().await?.next(rhythm).map(|(at, _)| at))
    }

    /// Ask Exec to check in if one is due at `now`, and record it so the rhythm counts from it
    /// whether or not Exec ends up saying anything. Not while a message from the owner is still
    /// unread: Exec is about to answer them anyway. Returns what was asked for.
    pub async fn signal_checkin_at(
        &self,
        now: DateTime<Utc>,
        rhythm: CheckinRhythm,
    ) -> Result<Option<(CheckinKind, CheckinState)>> {
        if self.checkin_state().await?.due(now, rhythm).is_none() {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        // Two planes sweeping the same company: one asks, the other sees the new check-in.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('exec_checkin'))")
            .execute(&mut *tx)
            .await?;
        let owner_waiting: bool = sqlx::query_scalar(&format!(
            "SELECT EXISTS(SELECT 1 FROM messages m WHERE m.to_actor='exec' AND m.read_at IS NULL \
               AND {OWNER_WROTE})"
        ))
        .fetch_one(&mut *tx)
        .await?;
        // Nor in the same scan as a daily review or just after one: that Exec turn settles first,
        // and what it posts moves this rhythm on by itself.
        let just_reviewed: bool = sqlx::query_scalar(&format!(
            "SELECT EXISTS(SELECT 1 FROM events e WHERE e.kind='{}' AND e.created_at > $1)",
            crate::daily_review::DAILY_REVIEW_EVENT
        ))
        .bind(now - crate::daily_review::settle_for(rhythm.every))
        .fetch_one(&mut *tx)
        .await?;
        if owner_waiting || just_reviewed {
            tx.rollback().await?;
            return Ok(None);
        }
        // Read again under the lock: the other plane's check-in has moved the rhythm on.
        let state = self.checkin_state().await?;
        let Some(kind) = state.due(now, rhythm) else {
            tx.rollback().await?;
            return Ok(None);
        };
        let room_id = ensure_direct_message_room_in_tx(&mut tx, "daemon", Some("exec")).await?;
        sqlx::query(
            "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,'daemon','exec',$2)",
        )
        .bind(room_id)
        .bind(state.message(kind, now))
        .execute(&mut *tx)
        .await?;
        sqlx::query(&format!(
            "INSERT INTO events (kind, actor_id, body, created_at) \
             VALUES ('{CHECKIN_EVENT}','daemon', $1 || jsonb_build_object( \
                 'through', clock_timestamp(), \
                 'message_mark', (SELECT COALESCE(max(id),0) FROM messages)), $2)"
        ))
        .bind(serde_json::json!({
            "kind": kind.as_str(),
            "landed": state.landed,
            "stuck": state.stuck,
            "running": state.running,
            "open_work": state.open_work,
            "waiting_on_owner": state.waiting_on_owner,
        }))
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some((kind, state)))
    }

    /// Whether this person has check-ins on; yes until they say no.
    pub async fn exec_checkins(&self, actor_id: &str) -> Result<bool> {
        let on: Option<bool> = sqlx::query_scalar(
            "SELECT exec_checkins FROM notification_preferences WHERE actor_id=$1",
        )
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(on.unwrap_or(true))
    }

    /// Turn this person's check-ins on or off. Only an owner's choice changes the rhythm.
    pub async fn set_exec_checkins(&self, actor_id: &str, on: bool) -> Result<()> {
        sqlx::query(
            "INSERT INTO notification_preferences (actor_id,exec_checkins,updated_at) \
             VALUES ($1,$2,now()) ON CONFLICT (actor_id) DO UPDATE SET \
             exec_checkins=EXCLUDED.exec_checkins, updated_at=now()",
        )
        .bind(actor_id)
        .bind(on)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hours: i64, minutes: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-13T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + chrono::Duration::hours(hours)
            + chrono::Duration::minutes(minutes)
    }

    /// A company with Work open, an owner who wrote at 09:00 and Exec's answer a minute later.
    fn working() -> CheckinState {
        CheckinState {
            enabled: true,
            last: None,
            unanswered: 0,
            said_since_last: false,
            heard_at: Some(at(0, 1)),
            engaged_at: Some(at(0, 0)),
            open_goals: 1,
            open_work: 3,
            blocked_work: 0,
            running: 2,
            landed: 0,
            stuck: 0,
            waiting_on_owner: 0,
        }
    }

    fn rhythm() -> CheckinRhythm {
        CheckinRhythm::default()
    }

    #[test]
    fn an_engaged_owner_gets_a_check_in_every_hour() {
        let first = working();
        assert_eq!(
            first.due(at(1, 0), rhythm()),
            None,
            "an hour after Exec's last word"
        );
        assert_eq!(first.due(at(1, 1), rhythm()), Some(CheckinKind::Checkin));

        // They answered the 10:01 check-in at 10:20 and Exec replied at 10:21.
        let answered = CheckinState {
            last: Some(at(1, 1)),
            said_since_last: true,
            engaged_at: Some(at(1, 20)),
            heard_at: Some(at(1, 21)),
            ..working()
        };
        assert_eq!(answered.due(at(2, 20), rhythm()), None);
        assert_eq!(
            answered.due(at(2, 21), rhythm()),
            Some(CheckinKind::Checkin)
        );

        // They acted on a card and Exec said nothing more: an hour after the check-in.
        let acted = CheckinState {
            heard_at: Some(at(1, 2)),
            engaged_at: Some(at(1, 5)),
            ..answered
        };
        assert_eq!(
            acted.next(rhythm()),
            Some((at(2, 5), CheckinKind::Checkin)),
            "an hour after they came back to it"
        );
    }

    #[test]
    fn silence_gets_one_nudge_after_three_hours_then_nothing() {
        let unanswered = CheckinState {
            last: Some(at(1, 1)),
            unanswered: 1,
            said_since_last: true,
            heard_at: Some(at(1, 2)),
            ..working()
        };
        assert_eq!(
            unanswered.due(at(2, 30), rhythm()),
            None,
            "not hourly into silence"
        );
        assert_eq!(unanswered.due(at(4, 0), rhythm()), None);
        assert_eq!(unanswered.due(at(4, 1), rhythm()), Some(CheckinKind::Nudge));

        let nudged = CheckinState {
            last: Some(at(4, 1)),
            unanswered: CHECKINS_INTO_SILENCE,
            heard_at: Some(at(4, 2)),
            landed: 5,
            ..unanswered
        };
        assert_eq!(
            nudged.next(rhythm()),
            None,
            "whatever happens, however long"
        );
        assert_eq!(nudged.due(at(900, 0), rhythm()), None);
    }

    #[test]
    fn the_hourly_rhythm_resumes_an_hour_after_the_owner_returns() {
        // Nudged at 13:01, silence for two days, then the owner writes and Exec answers.
        let back = CheckinState {
            last: Some(at(4, 1)),
            unanswered: 0,
            said_since_last: true,
            engaged_at: Some(at(50, 0)),
            heard_at: Some(at(50, 2)),
            ..working()
        };
        assert_eq!(
            back.due(at(50, 30), rhythm()),
            None,
            "not the moment they return"
        );
        assert_eq!(back.next(rhythm()), Some((at(51, 2), CheckinKind::Checkin)));
    }

    #[test]
    fn nothing_active_never_wakes_a_model() {
        let idle = CheckinState {
            open_goals: 0,
            open_work: 0,
            running: 0,
            ..working()
        };
        assert!(!idle.active());
        assert_eq!(idle.next(rhythm()), None);
        assert_eq!(idle.due(at(900, 0), rhythm()), None);
        // One thing waiting on the owner is an open question; so is Work that just finished.
        for active in [
            CheckinState {
                waiting_on_owner: 1,
                ..idle
            },
            CheckinState { landed: 1, ..idle },
            CheckinState {
                open_goals: 1,
                ..idle
            },
        ] {
            assert_eq!(active.due(at(1, 1), rhythm()), Some(CheckinKind::Checkin));
        }
        // Turned off by the owner: nothing, however busy.
        let off = CheckinState {
            enabled: false,
            ..working()
        };
        assert_eq!(off.next(rhythm()), None);
    }

    #[test]
    fn an_owner_who_went_quiet_before_any_check_in_is_not_woken_for() {
        // Work is open, but they last engaged yesterday and there has never been a check-in.
        let dormant = working();
        assert_eq!(dormant.due(at(4, 0), rhythm()), Some(CheckinKind::Checkin));
        assert_eq!(dormant.due(at(4, 1), rhythm()), None);
        assert_eq!(dormant.due(at(30, 0), rhythm()), None);
        // Never engaged at all: nothing, even with Work open.
        let never = CheckinState {
            engaged_at: None,
            ..working()
        };
        assert_eq!(never.due(at(2, 0), rhythm()), None);
        // They write again: an hour after Exec's answer, the first check-in.
        let back = CheckinState {
            engaged_at: Some(at(30, 0)),
            heard_at: Some(at(30, 1)),
            ..working()
        };
        assert_eq!(back.due(at(31, 1), rhythm()), Some(CheckinKind::Checkin));
    }

    #[test]
    fn a_check_in_that_ended_quietly_is_not_nudged() {
        let quiet = CheckinState {
            last: Some(at(1, 1)),
            unanswered: 1,
            said_since_last: false,
            ..working()
        };
        assert_eq!(
            quiet.next(rhythm()),
            None,
            "nothing was asked, nothing to restate"
        );
        // Work moved since: one more check-in at the three-hour mark, a full one, then silence.
        let moved = CheckinState { stuck: 1, ..quiet };
        assert_eq!(moved.next(rhythm()), Some((at(4, 1), CheckinKind::Checkin)));
    }

    #[test]
    fn the_intervals_can_be_shortened_together() {
        let fast = CheckinRhythm {
            every: chrono::Duration::seconds(60),
            nudge_after: chrono::Duration::seconds(180),
        };
        assert_eq!(working().next(fast), Some((at(0, 2), CheckinKind::Checkin)));
    }

    #[test]
    fn the_asks_carry_their_own_instructions_and_stay_short() {
        let state = CheckinState {
            last: Some(at(1, 0)),
            landed: 3,
            stuck: 1,
            blocked_work: 1,
            waiting_on_owner: 2,
            ..working()
        };
        let body = state.message(CheckinKind::Checkin, at(2, 0));
        assert!(body.starts_with(CHECKIN_MESSAGE_PREFIX), "{body}");
        assert!(
            body.contains(
                "3 finished, 1 blocked or failed, 2 running now, 3 open, 1 of them blocked, \
                 2 waiting on the owner"
            ),
            "{body}"
        );
        for part in [
            "what landed, what is running, what is next",
            "answerable in a word or two",
            "your recommendation",
            "without waiting for them",
            "no ids and no commands",
            "stay quiet",
        ] {
            assert!(body.contains(part), "missing {part:?} in {body}");
        }
        assert!(body.len() < 1_100, "{}", body.len());

        let nudge = state.message(CheckinKind::Nudge, at(4, 0));
        assert!(nudge.contains("3 hours ago"), "{nudge}");
        assert!(nudge.contains("not a new report"), "{nudge}");
        assert!(nudge.contains("stay quiet"), "{nudge}");
        assert!(nudge.len() < 650, "{}", nudge.len());
    }
}
