//! The company moves first (Sprint 116, owner decisions of 14 November 2026). Every other
//! automatic wake comes from something already in motion: an owner message, Work settling, a
//! block, an unanswered card. Nothing read the goals and started work, so a company whose owner
//! said nothing did nothing.
//!
//! A daily review is an Exec turn nobody asked for: once a day, for a company with an open goal
//! that has nothing in motion or is nearly due, Exec is told which goals those are and to start
//! the next step of each (act then tell), then to tell the owner what it started.
//!
//! - Due [`DAILY_REVIEW_EVERY`] after the last one. Nothing reliable says what time of day it is
//!   for the owner (a time zone exists only on a schedule and on a person's optional email
//!   digest), so there is no time-of-day rule, and "due within three days" is counted in UTC days.
//! - With no open goal, or with every open goal in motion and none nearly due, no model is woken.
//! - It stops by itself: after [`QUIET_REVIEWS_BEFORE_STOP`] reviews in a row with no Work started
//!   toward a goal, no owner engagement and no goal added, changed, closed or reopened, nothing
//!   more is asked until one of those happens.
//! - Before the first review the company must have stirred (the owner engaged, or a goal changed)
//!   within the last two intervals, so existing quiet companies are not all woken at rollout.
//! - It never shares a scan with a check-in: it waits [`DAILY_REVIEW_SETTLE`] after one (and
//!   after the owner's own message or goal change), and a check-in waits the same after a review. What Exec posts from a review is its last word to
//!   the owner, which the check-in rhythm already counts from.
//!
//! Like a check-in it is a daemon message: not owner activity and not an org-review signal.
//!
//! What Exec writes the owner from a review is the day's brief (Sprint 117): what moved, what it
//! is starting unless they say no, and what it needs from them. [`OrgIntel::latest_brief`] reads
//! it back for the landing page and the notification, without waking anything.

use chrono::{DateTime, Datelike as _, NaiveDate, Utc};
use sqlx::Row as _;

use crate::checkin::{CHECKIN_EVENT, OWNER_WROTE, TO_OWNER};
use crate::direction_session::DIRECTION_SESSION_EVENT;
use crate::{ensure_direct_message_room_in_tx, OrgIntel, Result};

/// How long after a daily review the next one is due.
pub const DAILY_REVIEW_EVERY: chrono::Duration = chrono::Duration::hours(24);
/// How long a daily review and a check-in keep apart, so one Exec turn settles before the other.
pub const DAILY_REVIEW_SETTLE: chrono::Duration = chrono::Duration::minutes(15);
/// Reviews in a row that started nothing before the company is left alone.
pub const QUIET_REVIEWS_BEFORE_STOP: i64 = 3;
/// A goal due within this many days is listed even with Work in motion.
pub const DUE_SOON_DAYS: i64 = 3;
/// The event every daily review leaves; the rhythm is read from these.
pub const DAILY_REVIEW_EVENT: &str = "exec_daily_review";
/// How the trigger message to Exec starts.
pub const DAILY_REVIEW_MESSAGE_PREFIX: &str = "Daily review";
/// Goals named in one message; the rest are counted.
const LISTED_GOALS: usize = 6;
const TITLE_CHARS: usize = 80;
/// How long after a review Exec's message to the owner still counts as that review's brief.
pub const BRIEF_WITHIN: chrono::Duration = chrono::Duration::hours(1);
/// Things a brief may ask of the owner.
pub const MAX_BRIEF_NEEDS: usize = 3;
/// A due review of how the company is organised rides a daily review at most this often
/// (Sprint 119). Whether one is due is still decided by the owner's signal, not by this clock.
pub const ORG_REVIEW_RIDES_EVERY: chrono::Duration = chrono::Duration::days(7);
/// The words in a daily review that say it carries the team review. Exec's context shows the
/// review's numbers on a turn with no owner message only when the ask holds them.
pub const ORG_REVIEW_RIDER: &str = "A review of how the company is organised is also due";

/// Whether a due team review rides this daily review: only when the engine's evidence test
/// says one is due, and not within [`ORG_REVIEW_RIDES_EVERY`] of the last one a daily review
/// carried. With no new evidence there is none, whatever the date.
pub fn org_review_rides(
    due: bool,
    last_carried: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    due && last_carried.is_none_or(|at| now - at >= ORG_REVIEW_RIDES_EVERY)
}

/// Whether a message to Exec is a daily review that carries the team review.
pub fn carries_org_review(from_actor: &str, body: &str) -> bool {
    from_actor == "daemon"
        && body.starts_with(DAILY_REVIEW_MESSAGE_PREFIX)
        && body.contains(ORG_REVIEW_RIDER)
}

/// The settling time for a given interval: a quarter of it, at most [`DAILY_REVIEW_SETTLE`], so
/// shortened test intervals shorten it too.
pub fn settle_for(every: chrono::Duration) -> chrono::Duration {
    (every / 4).min(DAILY_REVIEW_SETTLE)
}

/// The interval, so a test or a local stack can shorten it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyReviewRhythm {
    pub every: chrono::Duration,
}

impl Default for DailyReviewRhythm {
    fn default() -> Self {
        Self {
            every: DAILY_REVIEW_EVERY,
        }
    }
}

/// What makes a due daily review wait. It is held, never dropped: the next scan asks again.
///
/// The owner being away is deliberately not here (the company keeps working while they are
/// away), and neither is the Check-ins notification choice (that is about messages, this is
/// about work).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DailyReviewHolds {
    /// The company's spend limit is reached: a turn could not run.
    pub spend_limit_reached: bool,
    /// The plane is draining for an update.
    pub plane_draining: bool,
    /// The owner stopped the company computer.
    pub computer_stopped: bool,
    /// A message from the owner to Exec is unread: Exec is about to answer them.
    pub owner_message_unread: bool,
}

impl DailyReviewHolds {
    pub fn held(self) -> bool {
        self.spend_limit_reached
            || self.plane_draining
            || self.computer_stopped
            || self.owner_message_unread
    }
}

/// One open goal as the review sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalStanding {
    pub title: String,
    pub due_on: Option<NaiveDate>,
    /// Open Work (waiting, running or blocked) is linked to it.
    pub in_motion: bool,
}

impl GoalStanding {
    fn due_in_days(&self, today: NaiveDate) -> Option<i64> {
        self.due_on.map(|due| (due - today).num_days())
    }

    /// Whether the review names it: nothing in motion, or overdue or due soon.
    fn needs_a_look(&self, today: NaiveDate) -> bool {
        !self.in_motion
            || self
                .due_in_days(today)
                .is_some_and(|days| days <= DUE_SOON_DAYS)
    }

    pub(crate) fn line(&self, today: NaiveDate) -> String {
        let mut title: String = self.title.chars().take(TITLE_CHARS).collect();
        if self.title.chars().count() > TITLE_CHARS {
            title.push('…');
        }
        let motion = if self.in_motion {
            "work in motion"
        } else {
            "nothing in motion"
        };
        let due = match (self.due_on, self.due_in_days(today)) {
            (Some(due), Some(days)) => {
                let date = format!("{} {}", due.day(), due.format("%B"));
                match days {
                    days if days < 0 => format!(", overdue since {date}"),
                    0 => ", due today".to_string(),
                    1 => format!(", due tomorrow ({date})"),
                    days if days <= DUE_SOON_DAYS => format!(", due in {days} days ({date})"),
                    _ => format!(", due {date}"),
                }
            }
            _ => String::new(),
        };
        format!("- {title}: {motion}{due}")
    }
}

/// Everything the daily review is decided from. Facts only; the rule is pure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyReviewState {
    /// When Exec was last asked for a daily review.
    pub last: Option<DateTime<Utc>>,
    /// Daily reviews since anything last stirred or started: the owner engaged, a goal was
    /// added, changed, closed or reopened, or Work was started toward a goal.
    pub quiet_reviews: i64,
    /// When the owner last engaged or a goal last changed.
    pub stirred_at: Option<DateTime<Utc>>,
    /// When Exec was last asked to check in, or for a direction session.
    pub last_checkin: Option<DateTime<Utc>>,
    /// Attempts that ended with a result, and that ended blocked or failed, since the last
    /// review.
    pub landed: i64,
    pub stuck: i64,
    /// The open goals, soonest due first.
    pub goals: Vec<GoalStanding>,
}

/// The brief Exec wrote the owner from a daily review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MorningBrief {
    /// The review it answers; one brief per review.
    pub review_id: i64,
    pub message_id: i64,
    /// The message as stored, markers included.
    pub body: String,
    pub at: DateTime<Utc>,
    /// The owner has engaged since it was written: they have seen it or moved on.
    pub answered: bool,
}

impl DailyReviewState {
    /// The goals a review at `now` would name.
    pub fn listed(&self, now: DateTime<Utc>) -> Vec<&GoalStanding> {
        let today = now.date_naive();
        self.goals
            .iter()
            .filter(|goal| goal.needs_a_look(today))
            .collect()
    }

    /// When the next daily review could be due, or `None` while none is owed: no open goal, or
    /// the company has gone quiet. Whether a goal needs a look is decided at that time.
    pub fn next(&self, rhythm: DailyReviewRhythm) -> Option<DateTime<Utc>> {
        if self.goals.is_empty() || self.quiet_reviews >= QUIET_REVIEWS_BEFORE_STOP {
            return None;
        }
        let after_last = self.last.or(self.stirred_at)? + rhythm.every;
        // Never on the heels of a check-in, nor of the owner's own message or goal change: the
        // Exec turn that answers them settles first (seen live: a review asked in the same
        // second as Exec's reply to a returning owner).
        let settled = [self.last_checkin, self.stirred_at]
            .into_iter()
            .flatten()
            .max()
            .map(|at| at + settle_for(rhythm.every));
        Some(settled.map_or(after_last, |settled| settled.max(after_last)))
    }

    /// Whether a daily review is due at `now`.
    ///
    /// Before the first one the company must have stirred within the last two intervals: a
    /// company that was already quiet when this shipped waits for its owner or a goal change.
    pub fn due(&self, now: DateTime<Utc>, rhythm: DailyReviewRhythm) -> bool {
        if self.last.is_none()
            && !self
                .stirred_at
                .is_some_and(|stirred| now - stirred <= rhythm.every * 2)
        {
            return false;
        }
        self.next(rhythm).is_some_and(|at| at <= now) && !self.listed(now).is_empty()
    }

    /// The trigger Exec gets. The instructions live here, not in the fixed prompt, so they cost
    /// tokens only on a daily review turn.
    pub fn message(&self, now: DateTime<Utc>) -> String {
        let today = now.date_naive();
        let listed = self.listed(now);
        let mut lines: Vec<String> = listed
            .iter()
            .take(LISTED_GOALS)
            .map(|goal| goal.line(today))
            .collect();
        if listed.len() > LISTED_GOALS {
            lines.push(format!("- and {} more", listed.len() - LISTED_GOALS));
        }
        let moved = match (self.landed, self.stuck) {
            (0, 0) => "nothing finished".to_string(),
            (landed, 0) => format!("{landed} finished"),
            (0, stuck) => format!("{stuck} blocked or failed"),
            (landed, stuck) => format!("{landed} finished, {stuck} blocked or failed"),
        };
        format!(
            "{DAILY_REVIEW_MESSAGE_PREFIX} due. The owner has not asked for this; it is the \
             company moving on the goals they set. Since the last review: {moved}. Goals that \
             need a next step:\n{}\n\
             For each one, start the next step now: commission it to an accountable lead as Work \
             under that goal. Do not ask first; the owner set these goals and this stays inside \
             the spend limit. End the outcome of what you commission with what it rests on: a \
             line `Assumptions:`, up to three lines each starting `- `, and one line `Least sure \
             of: …`. Only when a goal is truly blocked on the owner, ask them one prepared \
             question instead. Then write the owner today's brief: one short message in plain \
             words, with no ids and no commands, in three parts. What moved since the last \
             brief. What you are starting today unless they say no, and why; they can tell you \
             to undo it. What you need from them: at most {MAX_BRIEF_NEEDS} things, each \
             answerable in a word. Lead with the part that matters most and leave out a part \
             that is empty. If nothing moved, nothing needs starting and you need nothing, stay \
             quiet.",
            lines.join("\n")
        )
    }
}

impl OrgIntel {
    /// The facts the daily review is decided from.
    ///
    /// "Since a review" is told by row order, as for check-ins: each review records the newest
    /// message and the moment it was written.
    pub async fn daily_review_state(&self) -> Result<DailyReviewState> {
        let row = sqlx::query(&format!(
            "WITH wrote AS (SELECT max(m.id) AS id, max(m.created_at) AS at FROM messages m \
                       WHERE m.to_actor='exec' AND {OWNER_WROTE}), \
             decided AS (SELECT max(d.created_at) AS at FROM owner_handoff_decisions d \
                         WHERE d.state <> 'canceled'), \
             acted AS (SELECT max(e.id) AS id, max(e.created_at) AS at FROM events e \
                       WHERE e.kind='owner_activity'), \
             changed AS (SELECT max(e.id) AS id, max(e.created_at) AS at FROM events e \
                         WHERE e.kind IN ('goal_created','goal_updated','goal_closed','goal_reopened')), \
             started AS (SELECT max(e.id) AS id FROM events e \
                         WHERE e.kind='work_goal_assigned' \
                            OR (e.kind='work_commissioned' AND EXISTS ( \
                                  SELECT 1 FROM work w WHERE w.id::text=e.body->>'work_id' \
                                    AND w.goal_id IS NOT NULL))) \
             SELECT (SELECT max(e.created_at) FROM events e WHERE e.kind='{DAILY_REVIEW_EVENT}') AS last, \
               (SELECT count(*) FROM events e \
                 WHERE e.kind='{DAILY_REVIEW_EVENT}' \
                   AND (wrote.id IS NULL OR (e.body->>'message_mark')::bigint >= wrote.id) \
                   AND (decided.at IS NULL OR (e.body->>'through')::timestamptz >= decided.at) \
                   AND (acted.id IS NULL OR e.id > acted.id) \
                   AND (changed.id IS NULL OR e.id > changed.id) \
                   AND (started.id IS NULL OR e.id > started.id)) AS quiet_reviews, \
               GREATEST(wrote.at, decided.at, acted.at, changed.at) AS stirred_at, \
               (SELECT max(e.created_at) FROM events e \
                 WHERE e.kind IN ('{CHECKIN_EVENT}','{DIRECTION_SESSION_EVENT}')) AS last_checkin, \
               (SELECT count(*) FROM work_attempts a \
                 WHERE a.state IN ('produced','changes_requested') \
                   AND a.finished_at > COALESCE(reviewed.through, '-infinity')) AS landed, \
               (SELECT count(*) FROM work_attempts a \
                 WHERE a.state IN ('blocked','failed') \
                   AND a.finished_at > COALESCE(reviewed.through, '-infinity')) AS stuck \
             FROM wrote, decided, acted, changed, started, \
               (SELECT (SELECT (e.body->>'through')::timestamptz FROM events e \
                         WHERE e.kind='{DAILY_REVIEW_EVENT}' ORDER BY e.id DESC LIMIT 1) AS through) reviewed"
        ))
        .fetch_one(&self.pool)
        .await?;
        let goals = sqlx::query(
            "SELECT g.title, g.due_on, EXISTS (SELECT 1 FROM work w WHERE w.goal_id=g.id \
                      AND w.status IN ('proposed','active','blocked')) AS in_motion \
             FROM goals g WHERE g.closed_at IS NULL ORDER BY g.due_on NULLS LAST, g.created_at",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|goal| GoalStanding {
            title: goal.get("title"),
            due_on: goal.get("due_on"),
            in_motion: goal.get("in_motion"),
        })
        .collect();
        Ok(DailyReviewState {
            last: row.get("last"),
            quiet_reviews: row.get("quiet_reviews"),
            stirred_at: row.get("stirred_at"),
            last_checkin: row.get("last_checkin"),
            landed: row.get("landed"),
            stuck: row.get("stuck"),
            goals,
        })
    }

    /// The latest brief: the first thing Exec wrote the owner after a daily review, within
    /// `within` of it, before the owner wrote again (after that, Exec is answering them) and
    /// before Exec was asked for a check-in or a direction session (after that, it is answering
    /// those).
    /// A review Exec ended quietly has no brief, so the one before it stays the latest.
    /// A read only: it wakes nothing.
    pub async fn latest_brief(&self, within: chrono::Duration) -> Result<Option<MorningBrief>> {
        let owner_wrote = OWNER_WROTE.replace("m.", "o.");
        let row = sqlx::query(&format!(
            "SELECT r.id AS review_id, brief.id AS message_id, brief.body, brief.created_at, \
               (EXISTS (SELECT 1 FROM messages o WHERE o.to_actor='exec' AND o.id > brief.id \
                          AND {owner_wrote}) \
                OR EXISTS (SELECT 1 FROM owner_handoff_decisions d \
                            WHERE d.state <> 'canceled' AND d.created_at > brief.created_at) \
                OR EXISTS (SELECT 1 FROM events e WHERE e.kind='owner_activity' \
                            AND e.created_at > brief.created_at)) AS answered \
             FROM events r \
             JOIN LATERAL ( \
               SELECT m.id, m.body, m.created_at FROM messages m \
               WHERE m.from_actor='exec' AND {TO_OWNER} AND m.deleted_at IS NULL \
                 AND m.id > (r.body->>'message_mark')::bigint \
                 AND m.created_at <= (r.body->>'through')::timestamptz + $1 \
                 AND NOT EXISTS (SELECT 1 FROM messages o WHERE o.to_actor='exec' \
                                   AND o.id > (r.body->>'message_mark')::bigint AND o.id < m.id \
                                   AND {owner_wrote}) \
                 AND NOT EXISTS (SELECT 1 FROM events w \
                                 WHERE w.kind IN ('{CHECKIN_EVENT}','{DIRECTION_SESSION_EVENT}') \
                                   AND w.id > r.id \
                                   AND (w.body->>'message_mark')::bigint < m.id) \
               ORDER BY m.id LIMIT 1) brief ON true \
             WHERE r.kind='{DAILY_REVIEW_EVENT}' ORDER BY r.id DESC LIMIT 1"
        ))
        .bind(within)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| MorningBrief {
            review_id: row.get("review_id"),
            message_id: row.get("message_id"),
            body: row.get("body"),
            at: row.get("created_at"),
            answered: row.get("answered"),
        }))
    }

    /// When the next daily review could be due, so the scheduler can sleep until then.
    pub async fn next_daily_review_at(
        &self,
        rhythm: DailyReviewRhythm,
    ) -> Result<Option<DateTime<Utc>>> {
        Ok(self.daily_review_state().await?.next(rhythm))
    }

    /// Ask Exec for a daily review if one is due at `now`, and record it so the next counts from
    /// it whether or not Exec starts anything. Not while a message from the owner is unread:
    /// Exec is about to answer them. Returns the facts it was asked with.
    pub async fn signal_daily_review_at(
        &self,
        now: DateTime<Utc>,
        rhythm: DailyReviewRhythm,
    ) -> Result<Option<DailyReviewState>> {
        self.signal_daily_review_with_at(now, rhythm, false).await
    }

    /// When a daily review last carried the team review.
    pub async fn org_review_last_carried(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(sqlx::query_scalar(&format!(
            "SELECT max(created_at) FROM events WHERE kind='{DAILY_REVIEW_EVENT}' \
               AND COALESCE((body->>'org_review_carried')::boolean, false)"
        ))
        .fetch_one(&self.pool)
        .await?)
    }

    /// [`Self::signal_daily_review_at`], and when `org_review_due` (the engine's evidence
    /// test, unchanged) the ask also says a review of how the company is organised is due, at
    /// most once in [`ORG_REVIEW_RIDES_EVERY`]. It adds no wake: with no daily review due,
    /// nothing is asked.
    pub async fn signal_daily_review_with_at(
        &self,
        now: DateTime<Utc>,
        rhythm: DailyReviewRhythm,
        org_review_due: bool,
    ) -> Result<Option<DailyReviewState>> {
        if !self.daily_review_state().await?.due(now, rhythm) {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        // The check-in's lock: two planes, or a check-in and a review, never ask at once.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('exec_checkin'))")
            .execute(&mut *tx)
            .await?;
        let owner_waiting: bool = sqlx::query_scalar(&format!(
            "SELECT EXISTS(SELECT 1 FROM messages m WHERE m.to_actor='exec' AND m.read_at IS NULL \
               AND {OWNER_WROTE})"
        ))
        .fetch_one(&mut *tx)
        .await?;
        let holds = DailyReviewHolds {
            owner_message_unread: owner_waiting,
            ..DailyReviewHolds::default()
        };
        if holds.held() {
            tx.rollback().await?;
            return Ok(None);
        }
        // Read again under the lock: a check-in or the other plane's review may have just been asked.
        let state = self.daily_review_state().await?;
        if !state.due(now, rhythm) {
            tx.rollback().await?;
            return Ok(None);
        }
        let last_carried: Option<DateTime<Utc>> = sqlx::query_scalar(&format!(
            "SELECT max(created_at) FROM events WHERE kind='{DAILY_REVIEW_EVENT}' \
               AND COALESCE((body->>'org_review_carried')::boolean, false)"
        ))
        .fetch_one(&mut *tx)
        .await?;
        let carries = org_review_rides(org_review_due, last_carried, now);
        let mut ask = state.message(now);
        if carries {
            ask.push_str(&format!(
                "\n{ORG_REVIEW_RIDER}: the owner has given enough new signal since the last one. \
                 Its numbers and what to do are under Company state in this turn; do it now and \
                 tell the owner each change in one line of today's brief."
            ));
        }
        let room_id = ensure_direct_message_room_in_tx(&mut tx, "daemon", Some("exec")).await?;
        sqlx::query(
            "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,'daemon','exec',$2)",
        )
        .bind(room_id)
        .bind(ask)
        .execute(&mut *tx)
        .await?;
        sqlx::query(&format!(
            "INSERT INTO events (kind, actor_id, body, created_at) \
             VALUES ('{DAILY_REVIEW_EVENT}','daemon', $1 || jsonb_build_object( \
                 'through', clock_timestamp(), \
                 'message_mark', (SELECT COALESCE(max(id),0) FROM messages)), $2)"
        ))
        .bind(serde_json::json!({
            "open_goals": state.goals.len(),
            "listed": state.listed(now).len(),
            "quiet_reviews": state.quiet_reviews,
            "org_review_carried": carries,
        }))
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hours: i64, minutes: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-14T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + chrono::Duration::hours(hours)
            + chrono::Duration::minutes(minutes)
    }

    fn goal(title: &str, due_in_days: Option<i64>, in_motion: bool) -> GoalStanding {
        GoalStanding {
            title: title.to_string(),
            due_on: due_in_days.map(|days| at(0, 0).date_naive() + chrono::Duration::days(days)),
            in_motion,
        }
    }

    /// One open goal with nothing in motion; the owner set it at 09:00 and has said nothing since.
    fn idle() -> DailyReviewState {
        DailyReviewState {
            last: None,
            quiet_reviews: 0,
            stirred_at: Some(at(0, 0)),
            last_checkin: None,
            landed: 0,
            stuck: 0,
            goals: vec![goal("Three paying clients", None, false)],
        }
    }

    fn rhythm() -> DailyReviewRhythm {
        DailyReviewRhythm::default()
    }

    #[test]
    fn an_open_goal_is_reviewed_once_a_day() {
        let first = idle();
        assert!(!first.due(at(23, 59), rhythm()));
        assert!(first.due(at(24, 0), rhythm()));
        // Asked at 09:00 the next day: not again that day, however often the scan runs.
        let asked = DailyReviewState {
            last: Some(at(24, 0)),
            quiet_reviews: 1,
            ..idle()
        };
        assert!(!asked.due(at(24, 1), rhythm()));
        assert!(!asked.due(at(47, 59), rhythm()));
        assert_eq!(asked.next(rhythm()), Some(at(48, 0)));
        assert!(asked.due(at(48, 0), rhythm()));
    }

    #[test]
    fn no_open_goal_never_wakes_a_model() {
        let none = DailyReviewState {
            goals: Vec::new(),
            ..idle()
        };
        assert_eq!(none.next(rhythm()), None);
        assert!(!none.due(at(24, 0), rhythm()));
        assert!(!none.due(at(9000, 0), rhythm()));
    }

    #[test]
    fn it_names_goals_with_nothing_in_motion_and_goals_nearly_due() {
        let state = DailyReviewState {
            goals: vec![
                goal("Overdue and moving", Some(-2), true),
                goal("Due soon and moving", Some(3), true),
                goal("Idle with no date", None, false),
                goal("Moving, due next month", Some(30), true),
                goal("Moving with no date", None, true),
            ],
            ..idle()
        };
        let listed: Vec<&str> = state
            .listed(at(0, 0))
            .iter()
            .map(|goal| goal.title.as_str())
            .collect();
        assert_eq!(
            listed,
            [
                "Overdue and moving",
                "Due soon and moving",
                "Idle with no date"
            ]
        );
        // Every open goal in motion and none nearly due: nothing to start, no model woken.
        let moving = DailyReviewState {
            goals: vec![
                goal("Moving, due next month", Some(30), true),
                goal("Moving with no date", None, true),
            ],
            ..idle()
        };
        assert!(!moving.due(at(24, 0), rhythm()));
        // The same goal four days out is not listed; a day later it is.
        let nearing = DailyReviewState {
            goals: vec![goal("Launch", Some(4), true)],
            ..idle()
        };
        assert!(nearing.listed(at(0, 0)).is_empty());
        assert_eq!(nearing.listed(at(24, 0)).len(), 1);
        assert!(nearing.due(at(24, 0), rhythm()));
    }

    #[test]
    fn each_hold_makes_a_due_review_wait_and_nothing_else_does() {
        let clear = DailyReviewHolds::default();
        assert!(!clear.held());
        for held in [
            DailyReviewHolds {
                spend_limit_reached: true,
                ..clear
            },
            DailyReviewHolds {
                plane_draining: true,
                ..clear
            },
            DailyReviewHolds {
                computer_stopped: true,
                ..clear
            },
            DailyReviewHolds {
                owner_message_unread: true,
                ..clear
            },
        ] {
            assert!(held.held(), "{held:?}");
        }
        // Held, not dropped: the review is still due when the hold lifts, days later.
        let waiting = DailyReviewState {
            last: Some(at(24, 0)),
            quiet_reviews: 1,
            ..idle()
        };
        assert!(waiting.due(at(48, 0), rhythm()) && waiting.due(at(120, 0), rhythm()));
        // Away and the Check-ins choice are not holds: there is nothing here to set for them.
        let DailyReviewHolds {
            spend_limit_reached: _,
            plane_draining: _,
            computer_stopped: _,
            owner_message_unread: _,
        } = clear;
    }

    #[test]
    fn it_does_not_stack_with_a_check_in() {
        // A check-in was asked at 08:55 the next day: the review waits out the settling time.
        let checked = DailyReviewState {
            last_checkin: Some(at(23, 55)),
            ..idle()
        };
        assert!(!checked.due(at(24, 0), rhythm()));
        assert!(!checked.due(at(24, 9), rhythm()));
        assert!(checked.due(at(24, 10), rhythm()));
        // Asked in the same scan (the check-in goes first): never both.
        let same_scan = DailyReviewState {
            last_checkin: Some(at(24, 0)),
            ..idle()
        };
        assert!(!same_scan.due(at(24, 0), rhythm()));
        assert_eq!(same_scan.next(rhythm()), Some(at(24, 15)));
        // An old check-in changes nothing.
        let long_ago = DailyReviewState {
            last_checkin: Some(at(2, 0)),
            ..idle()
        };
        assert!(long_ago.due(at(24, 0), rhythm()));
    }

    #[test]
    fn three_reviews_that_start_nothing_and_it_goes_quiet_until_something_stirs() {
        let two = DailyReviewState {
            last: Some(at(48, 0)),
            quiet_reviews: 2,
            ..idle()
        };
        assert!(two.due(at(72, 0), rhythm()), "the third");
        let three = DailyReviewState {
            last: Some(at(72, 0)),
            quiet_reviews: QUIET_REVIEWS_BEFORE_STOP,
            ..idle()
        };
        assert_eq!(three.next(rhythm()), None);
        assert!(!three.due(at(96, 0), rhythm()));
        assert!(!three.due(at(9000, 0), rhythm()), "however long");
        // The owner writes, a goal changes or Work is started toward a goal: the count starts
        // again (the database reads it as zero), and the review that was due is asked.
        let stirred = DailyReviewState {
            quiet_reviews: 0,
            stirred_at: Some(at(200, 0)),
            ..three.clone()
        };
        assert!(
            !stirred.due(at(200, 14), rhythm()),
            "not in the same breath as Exec's answer to them"
        );
        assert!(stirred.due(at(200, 15), rhythm()));
        // A review that did start Work does not count toward going quiet.
        let working = DailyReviewState {
            last: Some(at(72, 0)),
            quiet_reviews: 0,
            ..idle()
        };
        assert!(working.due(at(96, 0), rhythm()));
    }

    #[test]
    fn a_company_already_quiet_at_rollout_is_not_woken() {
        // Never reviewed, and the owner and the goals last moved three days ago.
        let dormant = idle();
        assert!(dormant.due(at(48, 0), rhythm()));
        assert!(!dormant.due(at(48, 1), rhythm()));
        assert!(!dormant.due(at(72, 0), rhythm()));
        // Nothing ever stirred (a goal from before events were kept): nothing.
        let never = DailyReviewState {
            stirred_at: None,
            ..idle()
        };
        assert!(!never.due(at(24, 0), rhythm()));
        // The owner comes back or a goal changes: a day later, the first review.
        let back = DailyReviewState {
            stirred_at: Some(at(500, 0)),
            ..idle()
        };
        assert!(!back.due(at(523, 0), rhythm()));
        assert!(back.due(at(524, 0), rhythm()));
        // Once reviewed, the window no longer applies: a review held for days is still owed.
        let held = DailyReviewState {
            last: Some(at(24, 0)),
            quiet_reviews: 1,
            ..idle()
        };
        assert!(held.due(at(240, 0), rhythm()));
    }

    #[test]
    fn the_team_review_rides_only_on_evidence_and_at_most_weekly() {
        let now = at(24 * 30, 0);
        // Due by the owner's signal, never carried: it rides.
        assert!(org_review_rides(true, None, now));
        // Carried yesterday: not again today, though it is still due.
        assert!(!org_review_rides(
            true,
            Some(now - chrono::Duration::days(1)),
            now
        ));
        // A week on, and still due: once more.
        assert!(org_review_rides(
            true,
            Some(now - chrono::Duration::days(7)),
            now
        ));
        // No new evidence: never, whatever the date.
        assert!(!org_review_rides(false, None, now));
        assert!(!org_review_rides(
            false,
            Some(now - chrono::Duration::days(60)),
            now
        ));
        // Exec's context tells a carrying review from an ordinary one by the ask's own words.
        let plain = idle().message(now);
        assert!(!carries_org_review("daemon", &plain));
        let carrying = format!("{plain}\n{ORG_REVIEW_RIDER}: the owner has given enough.");
        assert!(carries_org_review("daemon", &carrying));
        assert!(!carries_org_review("owner", &carrying));
    }

    #[test]
    fn the_interval_can_be_shortened() {
        let fast = DailyReviewRhythm {
            every: chrono::Duration::seconds(120),
        };
        assert_eq!(idle().next(fast), Some(at(0, 2)));
        assert!(idle().due(at(0, 2), fast));
        assert_eq!(settle_for(fast.every), chrono::Duration::seconds(30));
        assert_eq!(settle_for(DAILY_REVIEW_EVERY), DAILY_REVIEW_SETTLE);
    }

    #[test]
    fn the_ask_carries_its_own_instructions_in_plain_words_and_stays_short() {
        let long = "A goal whose title runs on and on ".repeat(6);
        let mut goals = vec![
            goal("Ship the spring catalogue", Some(-2), true),
            goal("Three paying clients", Some(0), false),
            goal("Open the Melbourne shop", Some(1), false),
            goal("Hire a bookkeeper", Some(3), true),
            goal(&long, None, false),
            goal("Rewrite the welcome email", Some(40), false),
        ];
        goals.extend((0..4).map(|n| goal(&format!("Extra goal {n}"), None, false)));
        let state = DailyReviewState {
            goals,
            landed: 2,
            stuck: 1,
            ..idle()
        };
        let body = state.message(at(0, 0));
        assert!(body.starts_with(DAILY_REVIEW_MESSAGE_PREFIX), "{body}");
        for part in [
            "- Ship the spring catalogue: work in motion, overdue since 12 November",
            "- Three paying clients: nothing in motion, due today",
            "- Open the Melbourne shop: nothing in motion, due tomorrow (15 November)",
            "- Hire a bookkeeper: work in motion, due in 3 days (17 November)",
            "…: nothing in motion\n",
            "- Rewrite the welcome email: nothing in motion, due 24 December",
            "- and 4 more",
            "commission it to an accountable lead",
            "Do not ask first",
            "inside the spend limit",
            "one prepared question",
            // The brief (Sprint 117): three parts, and the assumptions the new Work rests on.
            "Since the last review: 2 finished, 1 blocked or failed.",
            "today's brief",
            "What moved since the last brief",
            "What you are starting today unless they say no",
            "What you need from them: at most 3 things",
            "`Assumptions:`",
            "`Least sure of: …`",
            "undo it",
            "no ids and no commands",
            "stay quiet",
        ] {
            assert!(body.contains(part), "missing {part:?} in {body}");
        }
        // Nothing the owner would not recognise: no ids, no command names.
        assert!(
            !body.contains("restless ") && !body.contains("goal_id"),
            "{body}"
        );
        assert!(body.len() < 2_200, "{}", body.len());
        // The usual case, one goal, stays near a kilobyte.
        let usual = idle().message(at(24, 0));
        assert!(usual.contains("Since the last review: nothing finished."));
        assert!(usual.len() < 1_400, "{}", usual.len());
    }
}
