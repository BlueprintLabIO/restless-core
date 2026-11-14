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
//! - It never shares a scan with a check-in: it waits [`DAILY_REVIEW_SETTLE`] after one, and a
//!   check-in waits the same after a review. What Exec posts from a review is its last word to
//!   the owner, which the check-in rhythm already counts from.
//!
//! Like a check-in it is a daemon message: not owner activity and not an org-review signal.

use chrono::{DateTime, Datelike as _, NaiveDate, Utc};
use sqlx::Row as _;

use crate::checkin::{CHECKIN_EVENT, OWNER_WROTE};
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

    fn line(&self, today: NaiveDate) -> String {
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
    /// When Exec was last asked to check in.
    pub last_checkin: Option<DateTime<Utc>>,
    /// The open goals, soonest due first.
    pub goals: Vec<GoalStanding>,
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
        let after_checkin = self
            .last_checkin
            .map(|checkin| checkin + settle_for(rhythm.every));
        Some(after_checkin.map_or(after_last, |settled| settled.max(after_last)))
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
        format!(
            "{DAILY_REVIEW_MESSAGE_PREFIX} due. The owner has not asked for this; it is the \
             company moving on the goals they set. Goals that need a next step:\n{}\n\
             For each one, start the next step now: commission it to an accountable lead as Work \
             under that goal. Do not ask first; the owner set these goals and this stays inside \
             the spend limit. Only when a goal is truly blocked on the owner, ask them one \
             prepared question instead. Then write the owner one short message in plain words, \
             with no ids and no commands: what you started and why, and that they can tell you \
             to undo it. If nothing needs starting, stay quiet.",
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
               (SELECT max(e.created_at) FROM events e WHERE e.kind='{CHECKIN_EVENT}') AS last_checkin \
             FROM wrote, decided, acted, changed, started"
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
            goals,
        })
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
        if owner_waiting {
            tx.rollback().await?;
            return Ok(None);
        }
        // Read again under the lock: a check-in or the other plane's review may have just been asked.
        let state = self.daily_review_state().await?;
        if !state.due(now, rhythm) {
            tx.rollback().await?;
            return Ok(None);
        }
        let room_id = ensure_direct_message_room_in_tx(&mut tx, "daemon", Some("exec")).await?;
        sqlx::query(
            "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,'daemon','exec',$2)",
        )
        .bind(room_id)
        .bind(state.message(now))
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
        assert!(stirred.due(at(200, 1), rhythm()));
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
        let state = DailyReviewState { goals, ..idle() };
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
            "what you started and why",
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
        assert!(body.len() < 1_500, "{}", body.len());
        // The usual case, one goal, is well under a kilobyte.
        assert!(idle().message(at(24, 0)).len() < 800);
    }
}
