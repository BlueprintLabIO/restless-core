//! It comes to you with the right questions (Sprint 117, owner decision of 14 November 2026):
//! on direction the company brings questions and its assumptions, and the owner decides.
//!
//! A direction session is an Exec turn nobody asked for, at most once a week, for a company
//! with an open goal and something happening. Exec is handed what the company's own records
//! show (the open goals, the assumptions written into Work, results, and what the daily reviews
//! started) and asked for at most three questions about direction. Nothing here is a model's
//! reading: every line Exec is given is computed from rows.
//!
//! - Due [`DIRECTION_SESSION_EVERY`] after the last one; the first, that long after the oldest
//!   open goal was set.
//! - Only with an open goal, and only if something happened within the last interval (the owner
//!   engaged, a goal changed, Work was commissioned or an attempt finished). A company that has
//!   gone still is not asked about its direction.
//! - One unanswered session, then nothing: no further session is asked until the owner engages
//!   again (writes to Exec, acts on an Inbox card, or does something the owner API noted).
//! - It never stacks: it waits the settling time after a check-in, a daily review, the owner's
//!   own message or a goal change, and those two wait the same after a session.
//! - It is a question to the owner, so like a check-in it waits while they are away and is not
//!   asked at all with check-ins switched off.
//!
//! Like a check-in it is a daemon message: not owner activity and not an org-review signal.

use chrono::{DateTime, Utc};
use sqlx::Row as _;

use crate::assumptions::{read_assumptions, WorkAssumptions};
use crate::checkin::{CHECKIN_EVENT, OWNER_WROTE};
use crate::daily_review::{
    GoalStanding, DAILY_REVIEW_EVENT, DAILY_REVIEW_MESSAGE_PREFIX, DAILY_REVIEW_SETTLE,
};
use crate::{ensure_direct_message_room_in_tx, OrgIntel, Result};

/// How long after a direction session the next one is due.
pub const DIRECTION_SESSION_EVERY: chrono::Duration = chrono::Duration::days(7);
/// Sessions the owner may leave unanswered before the company stops asking.
pub const DIRECTION_SESSIONS_INTO_SILENCE: i64 = 1;
/// Questions one session may hold.
pub const MAX_DIRECTION_QUESTIONS: usize = 3;
/// The event every direction session leaves; the rhythm is read from these.
pub const DIRECTION_SESSION_EVENT: &str = "exec_direction_session";
/// How the trigger message to Exec starts.
pub const DIRECTION_SESSION_MESSAGE_PREFIX: &str = "Weekly direction review";

const LISTED_GOALS: usize = 6;
const LISTED_WORK: usize = 4;
const LISTED_TITLES: usize = 5;
const TITLE_CHARS: usize = 80;
const STATEMENT_CHARS: usize = 120;
const LISTED_CHANGES: usize = 6;
/// The part of the weekly message that says what the company changed about how it works
/// (Sprint 119). It is there only when something changed.
pub const WHAT_CHANGED_HEADING: &str = "What changed about how we work";

/// The interval and the settling time, so a test or a local stack can shorten them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectionSessionRhythm {
    pub every: chrono::Duration,
    /// How long a session keeps apart from a check-in, a daily review and the owner's own turn.
    pub settle: chrono::Duration,
}

impl Default for DirectionSessionRhythm {
    fn default() -> Self {
        Self {
            every: DIRECTION_SESSION_EVERY,
            settle: DAILY_REVIEW_SETTLE,
        }
    }
}

/// What makes a due direction session wait. It is held, never dropped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirectionSessionHolds {
    pub spend_limit_reached: bool,
    pub plane_draining: bool,
    pub computer_stopped: bool,
    /// A message from the owner to Exec is unread: Exec is about to answer them.
    pub owner_message_unread: bool,
    /// The owner said they are away. A daily review carries on without them; a question for
    /// them waits.
    pub owner_away: bool,
}

impl DirectionSessionHolds {
    pub fn held(self) -> bool {
        self.spend_limit_reached
            || self.plane_draining
            || self.computer_stopped
            || self.owner_message_unread
            || self.owner_away
    }
}

/// What one Work toward an open goal was commissioned on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedAssumptions {
    pub work: String,
    pub goal: String,
    pub rests_on: WorkAssumptions,
}

/// Everything a direction session is decided from and told with. Facts only; the rule is pure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectionSessionState {
    /// No owner has switched check-ins off.
    pub enabled: bool,
    /// When Exec was last asked for a direction session.
    pub last: Option<DateTime<Utc>>,
    /// Sessions asked since the owner last engaged.
    pub unanswered: i64,
    /// When the owner last engaged or a goal last changed.
    pub stirred_at: Option<DateTime<Utc>>,
    /// The last thing that happened at all: the owner engaged, a goal changed, Work was
    /// commissioned or an attempt finished.
    pub active_at: Option<DateTime<Utc>>,
    /// When the oldest open goal was set.
    pub first_goal_at: Option<DateTime<Utc>>,
    /// When Exec was last asked for a check-in or a daily review.
    pub last_wake: Option<DateTime<Utc>>,
    /// The open goals, soonest due first.
    pub goals: Vec<GoalStanding>,
    /// Assumptions written into Work toward open goals since the last session, newest first.
    pub assumptions: Vec<RecordedAssumptions>,
    /// Work whose latest attempt since the last session produced a result.
    pub finished: Vec<String>,
    /// Work whose latest attempt since then ended blocked, failed or sent back for changes.
    pub stuck: Vec<String>,
    /// Outcomes the owner accepted, and sent back, since the last session.
    pub accepted: Vec<String>,
    pub sent_back: Vec<String>,
    /// Work a daily review started toward a goal since the last session.
    pub started_by_reviews: Vec<String>,
    /// What the company changed about how it works since the last session, one plain line
    /// each: rules learned, given, dropped or brought back, and team changes.
    pub changed: Vec<String>,
}

fn short(text: &str, chars: usize) -> String {
    let mut kept: String = text.chars().take(chars).collect();
    if text.chars().count() > chars {
        kept.push('…');
    }
    kept
}

/// "A; B; and 3 more".
fn titles(list: &[String]) -> String {
    let mut shown: Vec<String> = list
        .iter()
        .take(LISTED_TITLES)
        .map(|title| short(title, TITLE_CHARS))
        .collect();
    if list.len() > LISTED_TITLES {
        shown.push(format!("and {} more", list.len() - LISTED_TITLES));
    }
    shown.join("; ")
}

impl DirectionSessionState {
    /// When the next direction session could be due, or `None` while none is owed: check-ins
    /// off, no open goal, or the last session went unanswered.
    pub fn next(&self, rhythm: DirectionSessionRhythm) -> Option<DateTime<Utc>> {
        if !self.enabled
            || self.goals.is_empty()
            || self.unanswered >= DIRECTION_SESSIONS_INTO_SILENCE
        {
            return None;
        }
        let after_last = self.last.or(self.first_goal_at)? + rhythm.every;
        // Never on the heels of a check-in or a daily review, nor of the owner's own message
        // or goal change: that Exec turn settles first.
        let settled = [self.last_wake, self.stirred_at]
            .into_iter()
            .flatten()
            .max()
            .map(|at| at + rhythm.settle);
        Some(settled.map_or(after_last, |settled| settled.max(after_last)))
    }

    /// Whether a direction session is due at `now`: the time has come and something happened
    /// within the last interval.
    pub fn due(&self, now: DateTime<Utc>, rhythm: DirectionSessionRhythm) -> bool {
        self.next(rhythm).is_some_and(|at| at <= now)
            && self
                .active_at
                .is_some_and(|active| now - active <= rhythm.every)
    }

    /// The trigger Exec gets. The instructions live here, not in the fixed prompt, so they cost
    /// tokens only on a direction session turn. Every fact is read from rows, not from a model.
    pub fn message(&self, now: DateTime<Utc>) -> String {
        let today = now.date_naive();
        let mut facts = vec!["Open goals:".to_string()];
        facts.extend(
            self.goals
                .iter()
                .take(LISTED_GOALS)
                .map(|goal| goal.line(today)),
        );
        if self.goals.len() > LISTED_GOALS {
            facts.push(format!("- and {} more", self.goals.len() - LISTED_GOALS));
        }
        if self.assumptions.is_empty() {
            facts.push(
                "No assumptions were written into Work toward these goals since the last one."
                    .to_string(),
            );
        } else {
            facts.push("What the work rests on, as written when it was commissioned:".to_string());
            for recorded in self.assumptions.iter().take(LISTED_WORK) {
                let mut parts: Vec<String> = recorded
                    .rests_on
                    .assumptions
                    .iter()
                    .map(|assumption| format!("assumes {}", short(assumption, STATEMENT_CHARS)))
                    .collect();
                if let Some(unsure) = &recorded.rests_on.least_sure {
                    parts.push(format!("least sure of {}", short(unsure, STATEMENT_CHARS)));
                }
                facts.push(format!(
                    "- {} (toward {}): {}",
                    short(&recorded.work, TITLE_CHARS),
                    short(&recorded.goal, TITLE_CHARS),
                    parts.join("; ")
                ));
            }
            if self.assumptions.len() > LISTED_WORK {
                facts.push(format!(
                    "- and {} more pieces of work",
                    self.assumptions.len() - LISTED_WORK
                ));
            }
        }
        for (label, list) in [
            ("Finished", &self.finished),
            ("Blocked, failed or sent back for changes", &self.stuck),
            ("Accepted by the owner", &self.accepted),
            ("Sent back by the owner", &self.sent_back),
            ("Started by daily reviews", &self.started_by_reviews),
        ] {
            if !list.is_empty() {
                facts.push(format!("{label}: {}", titles(list)));
            }
        }
        let changed = if self.changed.is_empty() {
            String::new()
        } else {
            let mut lines: Vec<String> = self
                .changed
                .iter()
                .take(LISTED_CHANGES)
                .map(|line| format!("- {line}"))
                .collect();
            if self.changed.len() > LISTED_CHANGES {
                lines.push(format!(
                    "- and {} more",
                    self.changed.len() - LISTED_CHANGES
                ));
            }
            format!(
                "{WHAT_CHANGED_HEADING} since the last one, from the records:\n{}\n\
                 End your message to the owner with one short part headed \"{WHAT_CHANGED_HEADING}\": \
                 each of those lines in a few plain words with its reason, so they can say if \
                 one is wrong. Send that part even if you have no direction question.\n",
                lines.join("\n")
            )
        };
        format!(
            "{DIRECTION_SESSION_MESSAGE_PREFIX} due. The owner has not asked for this. Once a \
             week the company brings them the questions about direction; the owner decides \
             direction, not you. From the company's own records since the last one:\n{}\n{changed}\
             Write the owner one short message in plain words, with no ids and no commands, \
             holding at most {MAX_DIRECTION_QUESTIONS} questions about direction: which goal \
             matters most, whether to continue, pivot or stop a line of work, what the company \
             is for, who it serves, what to give up. For each question give three short lines: \
             what the company knows, what it is assuming, and what would change with each \
             answer. Offer two or three answers to each and do not choose for them; you may add \
             which way you lean in a few words. Never ask for status, never ask what you could \
             find out yourself, and never repeat a question they have answered. Number the \
             questions so each can be answered in a word. Make the one that matters most \
             answerable by a tap: in your reply's intent marker put that question in \
             \"ownerNeed\" and its answers in \"ownerReplies\", a list of two or three short \
             strings. When the owner answers, act on it in that turn, record a changed goal, \
             and if another of these questions is still open, offer it the same way. Fewer \
             than {MAX_DIRECTION_QUESTIONS} is fine. If there is no real direction question and \
             nothing above changed about how we work, stay quiet. You will not be asked again until the owner has engaged, so keep the \
             work moving on the goals as they stand.",
            facts.join("\n")
        )
    }
}

impl OrgIntel {
    /// The facts a direction session is decided from and told with.
    ///
    /// "Since the last session" is told by row order and by the moment the last session was
    /// written, as for check-ins, never by the scheduler's clock.
    pub async fn direction_session_state(&self) -> Result<DirectionSessionState> {
        let row = sqlx::query(&format!(
            "WITH last AS ( \
               SELECT e.id, e.created_at AS at, (e.body->>'through')::timestamptz AS through \
               FROM events e WHERE e.kind='{DIRECTION_SESSION_EVENT}' ORDER BY e.id DESC LIMIT 1), \
             mark AS (SELECT (SELECT id FROM last) AS id, (SELECT at FROM last) AS at, \
                             (SELECT through FROM last) AS through), \
             wrote AS (SELECT max(m.id) AS id, max(m.created_at) AS at FROM messages m \
                       WHERE m.to_actor='exec' AND {OWNER_WROTE}), \
             decided AS (SELECT max(d.created_at) AS at FROM owner_handoff_decisions d \
                         WHERE d.state <> 'canceled'), \
             acted AS (SELECT max(e.id) AS id, max(e.created_at) AS at FROM events e \
                       WHERE e.kind='owner_activity'), \
             changed AS (SELECT max(e.created_at) AS at FROM events e \
                         WHERE e.kind IN ('goal_created','goal_updated','goal_closed','goal_reopened')), \
             moved AS (SELECT GREATEST( \
                         (SELECT max(e.created_at) FROM events e \
                           WHERE e.kind IN ('work_commissioned','work_goal_assigned')), \
                         (SELECT max(a.finished_at) FROM work_attempts a)) AS at) \
             SELECT mark.id AS last_id, mark.at AS last, mark.through AS through, \
               NOT EXISTS (SELECT 1 FROM notification_preferences p \
                           WHERE NOT p.exec_checkins AND (p.actor_id='owner' OR EXISTS ( \
                             SELECT 1 FROM human_principal_actor_bindings b \
                             WHERE b.actor_id=p.actor_id AND b.membership_role='owner' \
                               AND b.membership_status='active'))) AS enabled, \
               (SELECT count(*) FROM events e \
                 WHERE e.kind='{DIRECTION_SESSION_EVENT}' \
                   AND (wrote.id IS NULL OR (e.body->>'message_mark')::bigint >= wrote.id) \
                   AND (decided.at IS NULL OR (e.body->>'through')::timestamptz >= decided.at) \
                   AND (acted.id IS NULL OR e.id > acted.id)) AS unanswered, \
               GREATEST(wrote.at, decided.at, acted.at, changed.at) AS stirred_at, \
               GREATEST(wrote.at, decided.at, acted.at, changed.at, moved.at) AS active_at, \
               (SELECT min(g.created_at) FROM goals g WHERE g.closed_at IS NULL) AS first_goal_at, \
               (SELECT max(e.created_at) FROM events e \
                 WHERE e.kind IN ('{CHECKIN_EVENT}','{DAILY_REVIEW_EVENT}')) AS last_wake \
             FROM mark, wrote, decided, acted, changed, moved"
        ))
        .fetch_one(&self.pool)
        .await?;
        let last_id: Option<i64> = row.get("last_id");
        let through: Option<DateTime<Utc>> = row.get("through");

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

        // The carrier is the Work's own outcome text; nothing else stores an assumption.
        let assumptions = sqlx::query(
            "SELECT w.title, w.outcome, g.title AS goal FROM work w \
             JOIN goals g ON g.id=w.goal_id AND g.closed_at IS NULL \
             WHERE w.status <> 'abandoned' AND ($1::timestamptz IS NULL OR w.created_at > $1) \
             ORDER BY w.created_at DESC LIMIT 40",
        )
        .bind(through)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .filter_map(|work| {
            let rests_on = read_assumptions(work.get::<String, _>("outcome").as_str());
            (!rests_on.is_empty()).then(|| RecordedAssumptions {
                work: work.get("title"),
                goal: work.get("goal"),
                rests_on,
            })
        })
        .collect();

        // Each Work once, by how its latest attempt since the last session ended.
        let mut finished = Vec::new();
        let mut stuck = Vec::new();
        for attempt in sqlx::query(
            "SELECT title, state FROM ( \
               SELECT DISTINCT ON (w.id) w.title, a.state::text AS state, a.finished_at \
               FROM work_attempts a JOIN work w ON w.id=a.work_id \
               WHERE a.state IN ('produced','changes_requested','blocked','failed') \
                 AND a.finished_at IS NOT NULL \
                 AND ($1::timestamptz IS NULL OR a.finished_at > $1) \
               ORDER BY w.id, a.finished_at DESC) latest \
             ORDER BY finished_at DESC LIMIT 40",
        )
        .bind(through)
        .fetch_all(&self.pool)
        .await?
        {
            let title: String = attempt.get("title");
            if attempt.get::<String, _>("state") == "produced" {
                finished.push(title);
            } else {
                stuck.push(title);
            }
        }

        // The owner's own verdicts, as the org review reads them.
        let mut accepted = Vec::new();
        let mut sent_back = Vec::new();
        for verdict in sqlx::query(
            "SELECT w.title, h.state::text AS state FROM owner_handoffs h \
             JOIN work w ON w.id=h.work_id \
             WHERE h.state IN ('declined','resolved') AND h.assigned_to IS NULL \
               AND h.resolved_at IS NOT NULL \
               AND ($1::timestamptz IS NULL OR h.resolved_at > $1) \
             ORDER BY h.resolved_at DESC LIMIT 40",
        )
        .bind(through)
        .fetch_all(&self.pool)
        .await?
        {
            let title: String = verdict.get("title");
            if verdict.get::<String, _>("state") == "declined" {
                sent_back.push(title);
            } else {
                accepted.push(title);
            }
        }

        // Work toward a goal whose commissioning followed a daily review: the last thing Exec
        // had been sent before it was that review.
        let started_by_reviews = sqlx::query_scalar(&format!(
            "SELECT w.title FROM events e JOIN work w ON w.id::text=e.body->>'work_id' \
             WHERE e.kind='work_commissioned' AND w.goal_id IS NOT NULL \
               AND ($1::bigint IS NULL OR e.id > $1) \
               AND (SELECT m.from_actor='daemon' AND m.body LIKE '{DAILY_REVIEW_MESSAGE_PREFIX}%' \
                    FROM messages m WHERE m.to_actor='exec' AND m.created_at <= e.created_at \
                    ORDER BY m.id DESC LIMIT 1) \
             ORDER BY e.id DESC LIMIT 40"
        ))
        .bind(last_id)
        .fetch_all(&self.pool)
        .await?;

        // What the company changed about how it works: since the last session, or over the
        // last interval before the first one.
        let changed = self
            .working_changes_since(Some(
                through.unwrap_or_else(|| Utc::now() - DIRECTION_SESSION_EVERY),
            ))
            .await?;

        Ok(DirectionSessionState {
            changed,
            enabled: row.get("enabled"),
            last: row.get("last"),
            unanswered: row.get("unanswered"),
            stirred_at: row.get("stirred_at"),
            active_at: row.get("active_at"),
            first_goal_at: row.get("first_goal_at"),
            last_wake: row.get("last_wake"),
            goals,
            assumptions,
            finished,
            stuck,
            accepted,
            sent_back,
            started_by_reviews,
        })
    }

    /// When the next direction session could be due, so the scheduler can sleep until then.
    pub async fn next_direction_session_at(
        &self,
        rhythm: DirectionSessionRhythm,
    ) -> Result<Option<DateTime<Utc>>> {
        Ok(self.direction_session_state().await?.next(rhythm))
    }

    /// Ask Exec for a direction session if one is due at `now`, and record it so the next
    /// counts from it whether or not Exec asks anything. Not while a message from the owner is
    /// unread. Returns the facts it was asked with.
    pub async fn signal_direction_session_at(
        &self,
        now: DateTime<Utc>,
        rhythm: DirectionSessionRhythm,
    ) -> Result<Option<DirectionSessionState>> {
        if !self.direction_session_state().await?.due(now, rhythm) {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        // The check-in's lock: a check-in, a daily review and a session never ask at once.
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
        // Read again under the lock: a check-in, a review or the other plane may have just asked.
        let state = self.direction_session_state().await?;
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
             VALUES ('{DIRECTION_SESSION_EVENT}','daemon', $1 || jsonb_build_object( \
                 'through', clock_timestamp(), \
                 'message_mark', (SELECT COALESCE(max(id),0) FROM messages)), $2)"
        ))
        .bind(serde_json::json!({
            "open_goals": state.goals.len(),
            "work_with_assumptions": state.assumptions.len(),
            "finished": state.finished.len(),
            "started_by_reviews": state.started_by_reviews.len(),
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

    fn at(days: i64, minutes: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-14T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + chrono::Duration::days(days)
            + chrono::Duration::minutes(minutes)
    }

    fn goal(title: &str, in_motion: bool) -> GoalStanding {
        GoalStanding {
            title: title.to_string(),
            due_on: None,
            in_motion,
        }
    }

    /// One open goal set on day 0; the owner last wrote on day 5 and Work finished on day 6.
    fn busy() -> DirectionSessionState {
        DirectionSessionState {
            enabled: true,
            last: None,
            unanswered: 0,
            stirred_at: Some(at(5, 0)),
            active_at: Some(at(6, 0)),
            first_goal_at: Some(at(0, 0)),
            last_wake: None,
            goals: vec![goal("Three paying clients", true)],
            assumptions: Vec::new(),
            finished: Vec::new(),
            stuck: Vec::new(),
            accepted: Vec::new(),
            sent_back: Vec::new(),
            started_by_reviews: Vec::new(),
            changed: Vec::new(),
        }
    }

    fn rhythm() -> DirectionSessionRhythm {
        DirectionSessionRhythm::default()
    }

    #[test]
    fn an_open_goal_with_recent_activity_gets_one_session_a_week() {
        let first = busy();
        assert!(!first.due(at(6, 23 * 60 + 59), rhythm()));
        assert!(first.due(at(7, 0), rhythm()));
        // Asked on day 7 and answered on day 8: not again until day 14, however often scanned.
        let asked = DirectionSessionState {
            last: Some(at(7, 0)),
            stirred_at: Some(at(8, 0)),
            active_at: Some(at(13, 0)),
            ..busy()
        };
        assert!(!asked.due(at(7, 1), rhythm()));
        assert!(!asked.due(at(13, 23 * 60 + 59), rhythm()));
        assert_eq!(asked.next(rhythm()), Some(at(14, 0)));
        assert!(asked.due(at(14, 0), rhythm()));
    }

    #[test]
    fn never_without_an_open_goal_or_without_recent_activity() {
        let no_goal = DirectionSessionState {
            goals: Vec::new(),
            first_goal_at: None,
            ..busy()
        };
        assert_eq!(no_goal.next(rhythm()), None);
        assert!(!no_goal.due(at(7, 0), rhythm()) && !no_goal.due(at(900, 0), rhythm()));
        // An open goal, but nothing has happened for more than a week: not asked, and never
        // asked later for as long as nothing happens.
        let still = DirectionSessionState {
            stirred_at: Some(at(0, 0)),
            active_at: Some(at(0, 0)),
            first_goal_at: Some(at(0, 0)),
            ..busy()
        };
        assert!(still.due(at(7, 0), rhythm()), "the goal's own week");
        assert!(!still.due(at(7, 1), rhythm()));
        assert!(!still.due(at(900, 0), rhythm()));
        let nothing_ever = DirectionSessionState {
            stirred_at: None,
            active_at: None,
            ..busy()
        };
        assert!(!nothing_ever.due(at(7, 0), rhythm()));
        // Check-ins switched off: Exec does not come to the owner with questions either.
        let off = DirectionSessionState {
            enabled: false,
            ..busy()
        };
        assert_eq!(off.next(rhythm()), None);
    }

    #[test]
    fn each_hold_makes_a_due_session_wait() {
        let clear = DirectionSessionHolds::default();
        assert!(!clear.held());
        for held in [
            DirectionSessionHolds {
                spend_limit_reached: true,
                ..clear
            },
            DirectionSessionHolds {
                plane_draining: true,
                ..clear
            },
            DirectionSessionHolds {
                computer_stopped: true,
                ..clear
            },
            DirectionSessionHolds {
                owner_message_unread: true,
                ..clear
            },
            DirectionSessionHolds {
                owner_away: true,
                ..clear
            },
        ] {
            assert!(held.held(), "{held:?}");
        }
        // Held, not dropped: still due while the company stays active.
        let waiting = DirectionSessionState {
            active_at: Some(at(9, 0)),
            ..busy()
        };
        assert!(waiting.due(at(7, 0), rhythm()) && waiting.due(at(10, 0), rhythm()));
    }

    #[test]
    fn it_does_not_stack_with_a_check_in_a_daily_review_or_the_owners_own_turn() {
        // A check-in or daily review was asked five minutes before it fell due.
        let woken = DirectionSessionState {
            last_wake: Some(at(6, 24 * 60 - 5)),
            ..busy()
        };
        assert!(!woken.due(at(7, 0), rhythm()));
        assert!(!woken.due(at(7, 9), rhythm()));
        assert!(woken.due(at(7, 10), rhythm()));
        // Asked in the same scan (they go first): never both.
        let same_scan = DirectionSessionState {
            last_wake: Some(at(7, 0)),
            ..busy()
        };
        assert!(!same_scan.due(at(7, 0), rhythm()));
        assert_eq!(same_scan.next(rhythm()), Some(at(7, 15)));
        // The owner wrote a minute before: Exec's answer settles first.
        let answering = DirectionSessionState {
            stirred_at: Some(at(7, -1)),
            active_at: Some(at(7, -1)),
            ..busy()
        };
        assert!(!answering.due(at(7, 0), rhythm()));
        assert!(answering.due(at(7, 14), rhythm()));
    }

    #[test]
    fn one_unanswered_session_then_quiet_until_the_owner_is_back() {
        let unanswered = DirectionSessionState {
            last: Some(at(7, 0)),
            unanswered: 1,
            active_at: Some(at(13, 0)),
            ..busy()
        };
        assert_eq!(unanswered.next(rhythm()), None);
        assert!(!unanswered.due(at(14, 0), rhythm()));
        assert!(
            !unanswered.due(at(900, 0), rhythm()),
            "however long, however busy"
        );
        // The owner comes back three weeks later: the count is read as zero again, and the
        // session that was due is asked once Exec's answer to them has settled.
        let back = DirectionSessionState {
            unanswered: 0,
            stirred_at: Some(at(28, 0)),
            active_at: Some(at(28, 0)),
            ..unanswered.clone()
        };
        assert!(!back.due(at(28, 14), rhythm()));
        assert!(back.due(at(28, 15), rhythm()));
    }

    #[test]
    fn the_weekly_message_says_what_changed_about_how_we_work_only_when_something_did() {
        let mut state = busy();
        let quiet = state.message(at(8, 0));
        assert!(!quiet.contains(WHAT_CHANGED_HEADING), "{quiet}");

        state.changed = vec![
            "Learned, for the Menu team, after the same thing went wrong twice: Show every price with GST included.".into(),
            "Dropped a rule (Never post on Sundays) because: The owner changed their mind.".into(),
            "Closed the Web team because: one lead now carries both".into(),
        ];
        let message = state.message(at(8, 0));
        assert!(
            message.contains(&format!(
                "{WHAT_CHANGED_HEADING} since the last one, from the records:\n- Learned, for the Menu team"
            )),
            "{message}"
        );
        for line in &state.changed {
            assert!(message.contains(&format!("- {line}")), "{line}");
        }
        // Exec is told to say it to the owner, under that heading, even with no question.
        assert!(message.contains(&format!("one short part headed \"{WHAT_CHANGED_HEADING}\"")));
        assert!(message.contains("Send that part even if you have no direction question."));
        // The records come before the instructions for the questions, and nothing in them is an id.
        assert!(
            message.find(WHAT_CHANGED_HEADING) < message.find("Write the owner one short message")
        );

        // A long week is cut to a few lines, with the rest counted.
        state.changed = (0..9)
            .map(|n| format!("New rule from the owner, for the whole company: rule {n}"))
            .collect();
        let message = state.message(at(8, 0));
        assert!(
            message.contains("rule 5") && !message.contains("rule 6"),
            "{message}"
        );
        assert!(message.contains("- and 3 more"), "{message}");
    }

    #[test]
    fn the_interval_can_be_shortened() {
        let fast = DirectionSessionRhythm {
            every: chrono::Duration::seconds(300),
            settle: chrono::Duration::seconds(30),
        };
        let state = DirectionSessionState {
            stirred_at: Some(at(0, 1)),
            active_at: Some(at(0, 4)),
            ..busy()
        };
        assert_eq!(state.next(fast), Some(at(0, 5)));
        assert!(!state.due(at(0, 4), fast));
        assert!(state.due(at(0, 5), fast));
        assert!(
            !state.due(at(0, 10), fast),
            "nothing happened for an interval"
        );
    }

    #[test]
    fn the_ask_gives_exec_the_records_and_asks_for_questions_not_a_recommendation() {
        let state = DirectionSessionState {
            goals: vec![
                goal("Three paying clients", false),
                goal("Open the Melbourne shop", true),
            ],
            assumptions: vec![RecordedAssumptions {
                work: "List twelve local prospects".into(),
                goal: "Three paying clients".into(),
                rests_on: WorkAssumptions {
                    assumptions: vec![
                        "Local firms answer cold email.".into(),
                        "Twelve prospects is enough to learn from.".into(),
                    ],
                    least_sure: Some("whether price or timing decides it.".into()),
                },
            }],
            finished: vec!["List twelve local prospects".into()],
            stuck: vec!["Book the shopfront inspection".into()],
            accepted: vec!["Price list".into()],
            sent_back: vec!["Welcome email".into()],
            started_by_reviews: vec!["List twelve local prospects".into()],
            changed: Vec::new(),
            ..busy()
        };
        let body = state.message(at(7, 0));
        assert!(body.starts_with(DIRECTION_SESSION_MESSAGE_PREFIX), "{body}");
        for part in [
            "the owner decides direction, not you",
            "- Three paying clients: nothing in motion",
            "- Open the Melbourne shop: work in motion",
            "- List twelve local prospects (toward Three paying clients): assumes Local firms \
             answer cold email.; assumes Twelve prospects is enough to learn from.; least sure \
             of whether price or timing decides it.",
            "Finished: List twelve local prospects",
            "Blocked, failed or sent back for changes: Book the shopfront inspection",
            "Accepted by the owner: Price list",
            "Sent back by the owner: Welcome email",
            "Started by daily reviews: List twelve local prospects",
            "at most 3 questions about direction",
            "what the company knows, what it is assuming, and what would change with each answer",
            "do not choose for them",
            "Never ask for status",
            "never ask what you could find out yourself",
            "\"ownerNeed\"",
            "\"ownerReplies\"",
            "stay quiet",
            "not be asked again until the owner has engaged",
        ] {
            assert!(body.contains(part), "missing {part:?} in {body}");
        }
        // Nothing the owner would not recognise is handed over as fact: no ids, no commands.
        assert!(
            !body.contains("restless ") && !body.contains("goal_id"),
            "{body}"
        );
        assert!(body.len() < 3_500, "{}", body.len());

        // With nothing written down it says so, and the full lists stay bounded.
        let bare = busy().message(at(7, 0));
        assert!(bare.contains("No assumptions were written into Work"));
        assert!(!bare.contains("Finished:") && !bare.contains("Started by daily reviews"));
        assert!(bare.len() < 2_500, "{}", bare.len());
        let long = "A title that runs on and on and on ".repeat(8);
        let crowded = DirectionSessionState {
            goals: (0..12).map(|_| goal(&long, false)).collect(),
            assumptions: (0..9)
                .map(|_| RecordedAssumptions {
                    work: long.clone(),
                    goal: long.clone(),
                    rests_on: WorkAssumptions {
                        assumptions: vec![long.clone(); 3],
                        least_sure: Some(long.clone()),
                    },
                })
                .collect(),
            finished: vec![long.clone(); 30],
            stuck: vec![long.clone(); 30],
            accepted: vec![long.clone(); 30],
            sent_back: vec![long.clone(); 30],
            started_by_reviews: vec![long.clone(); 30],
            changed: Vec::new(),
            ..busy()
        };
        let body = crowded.message(at(7, 0));
        assert!(body.contains("- and 6 more") && body.contains("- and 5 more pieces of work"));
        assert!(body.len() < 8_500, "{}", body.len());
    }
}
