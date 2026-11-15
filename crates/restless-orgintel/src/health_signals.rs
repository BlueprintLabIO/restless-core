//! It notices and repairs itself (Sprint 118). The organisational health signals were computed
//! only inside an Exec wake that something else had caused, so a problem nobody was looking at
//! woke nobody.
//!
//! A problem here is one observation about the company's own Work, read without a model from
//! rows that already exist. Each one owes at most one wake to the lead accountable for the
//! affected Work, or to Exec when no single lead is. The rules:
//!
//! - **Once per problem.** Every problem has a key that names the signal, its subject and what
//!   would make it a different problem (a new revision, a new Attempt, a doubled cost). A key is
//!   raised once, recorded as an event, and never again: no model is woken to find nothing
//!   changed. When the problem changes, its key does, and it may be raised once more.
//! - **Advice, never a gate.** Raising a problem writes one message and one event. It never
//!   changes Work, pauses an Attempt or refuses a commission (`docs/specs/orgintel.md` §7.2: "a
//!   wakeup, not a universal blocker"). The lead may find that nothing is wrong.
//! - **Bounded.** At most [`MAX_HEALTH_WAKES_PER_DAY`] are raised per company in any 24 hours,
//!   so a detector that is wrong cannot spend freely. What is over the bound waits; it is not
//!   dropped.
//! - **Never with Exec's other wakes.** A problem for Exec waits out a check-in, a daily review
//!   or a direction session asked just before, and an unread owner message. Problems for leads
//!   do not wait: repair carries on whoever Exec is talking to.
//!
//! The signals: a review loop, one blocker holding several jobs, cost rising with no result,
//! duplicate work, the same failure twice (replan, not retry), and a lead holding a question for
//! the owner. Stalled Work keeps its own notice (`stalled_work`), so a block is not told twice.

use std::collections::{BTreeSet, HashMap};

use chrono::{DateTime, Utc};
use sqlx::Row as _;
use uuid::Uuid;

use crate::checkin::{CHECKIN_EVENT, OWNER_WROTE};
use crate::daily_review::{DAILY_REVIEW_EVENT, DAILY_REVIEW_SETTLE};
use crate::direction_session::DIRECTION_SESSION_EVENT;
use crate::{ensure_direct_message_room_in_tx, OrgIntel, Result, WorkAttemptState, WorkStatus};

/// How often the company's health is looked at.
pub const HEALTH_SWEEP_EVERY: chrono::Duration = chrono::Duration::minutes(10);
/// The event every raised problem leaves; "once per problem" is read from these.
pub const HEALTH_SIGNAL_EVENT: &str = "health_signal";
/// How every notice starts.
pub const HEALTH_NOTICE_PREFIX: &str = "Health notice";
/// Problems raised per company in any 24 hours. The rest wait.
pub const MAX_HEALTH_WAKES_PER_DAY: i64 = 12;
/// A Work sent back by review past this revision is in a loop.
pub const REVIEW_LOOP_PAST_REVISION: i64 = 3;
/// Open jobs that must require one stopped Work before it is a shared blocker.
pub const SHARED_BLOCKER_DEPENDENTS: usize = 2;
/// Spend on one Work since its last result before it is worth a look. Loose on purpose, like
/// the company-wide figure: a signal that fires on normal work is noise.
pub const COST_WITHOUT_OUTPUT_USD: f64 = 2.0;
/// Attempts on one Work that must end the same way before its lead is asked to replan.
pub const SAME_FAILURES_BEFORE_REPLAN: usize = 2;

const TITLE_CHARS: usize = 80;
const REASON_CHARS: usize = 200;

/// The interval, so a test or a local stack can shorten it. The waiting times that go with it
/// are multiples, so shortening one shortens them all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthRhythm {
    pub every: chrono::Duration,
}

impl Default for HealthRhythm {
    fn default() -> Self {
        Self {
            every: HEALTH_SWEEP_EVERY,
        }
    }
}

impl HealthRhythm {
    /// How long a Work must have been stopped before the jobs behind it make it a problem:
    /// half an hour, so an ordinary block that is resumed at once is never raised.
    pub fn shared_blocker_after(self) -> chrono::Duration {
        self.every * 3
    }

    /// How long a lead may hold a question for the owner before it goes up: an hour.
    pub fn held_question_after(self) -> chrono::Duration {
        self.every * 6
    }

    /// How long a problem for Exec keeps apart from a check-in, a daily review or a direction
    /// session: fifteen minutes, less when the interval is shortened.
    pub fn settle(self) -> chrono::Duration {
        (self.every * 6 / 4).min(DAILY_REVIEW_SETTLE)
    }
}

/// One Attempt as the rules see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthAttempt {
    pub id: Uuid,
    pub state: WorkAttemptState,
    pub summary: String,
    pub finished_at: Option<DateTime<Utc>>,
}

/// One piece of Work as the rules see it: open, or abandoned with open Work still requiring it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthWork {
    pub id: Uuid,
    pub title: String,
    pub outcome: String,
    pub expected_artifact: String,
    pub goal_id: Option<Uuid>,
    pub owner_id: String,
    /// Who answers for it: the lead of its owner's team, or its owner when there is none.
    pub lead: String,
    pub status: WorkStatus,
    pub resolution: String,
    pub revision: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// An item for it is waiting on the owner: they already have a card.
    pub waiting_on_owner: bool,
    /// A schedule will release it: it is waiting, not stuck.
    pub parked: bool,
    /// The latest word from its review is "changes", and it has produced nothing since.
    pub sent_back: bool,
    /// Its Attempts, oldest first.
    pub attempts: Vec<HealthAttempt>,
}

impl HealthWork {
    fn open(&self) -> bool {
        matches!(
            self.status,
            WorkStatus::Proposed | WorkStatus::Active | WorkStatus::Blocked
        )
    }

    /// The last Attempt that produced a result, if any.
    pub fn last_output(&self) -> Option<&HealthAttempt> {
        self.attempts
            .iter()
            .rev()
            .find(|attempt| attempt.state == WorkAttemptState::Produced)
    }

    /// Finished Attempts since the last result.
    fn attempts_since_output(&self) -> usize {
        self.attempts
            .iter()
            .rev()
            .take_while(|attempt| attempt.state != WorkAttemptState::Produced)
            .filter(|attempt| attempt.finished_at.is_some())
            .count()
    }
}

/// A question for the owner that an agent has been holding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldQuestion {
    pub handoff_id: Uuid,
    pub work_id: Uuid,
    pub work_title: String,
    /// Who holds it now.
    pub holder: String,
    /// Who asked.
    pub asked_by: String,
    pub question: String,
    /// What was prepared for the answer, in the asker's words.
    pub prepared: String,
    pub since: DateTime<Utc>,
}

/// Everything the rules are decided from. Facts only; every rule below is pure.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HealthFacts {
    pub work: Vec<HealthWork>,
    /// `requires` edges as (required, requiring).
    pub requires: Vec<(Uuid, Uuid)>,
    /// Every edge of any kind, as an unordered pair: linked Work is never a duplicate.
    pub linked: Vec<(Uuid, Uuid)>,
    pub held: Vec<HeldQuestion>,
}

/// What one Work has cost since its last result. Spend is kept by the daemon's ledger, not by
/// OrgIntel, so the caller supplies it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkSpend {
    pub work_id: Uuid,
    pub usd_since_output: f64,
}

/// One observation that owes one wake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthProblem {
    pub kind: &'static str,
    /// What makes this problem this problem. Raised once per key.
    pub key: String,
    /// The accountable lead, or `exec`.
    pub to: String,
    pub work_id: Option<Uuid>,
    /// The observation and the repairs open to whoever reads it.
    pub detail: String,
}

impl HealthProblem {
    /// The wake message. The instructions ride here, not in any fixed prompt.
    pub fn message(&self) -> String {
        format!(
            "{HEALTH_NOTICE_PREFIX}: {} Nothing has been paused or changed, and nothing waits on \
             this notice. Check it and make the smallest repair inside your own outcome; if \
             nothing is wrong, carry on. You will not be told again unless it changes.",
            self.detail
        )
    }
}

fn short(text: &str, chars: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut kept: String = text.chars().take(chars).collect();
    if text.chars().count() > chars {
        kept.push('…');
    }
    kept
}

/// The actor a problem about `lead`'s Work goes to: Exec answers for Work the owner leads.
fn accountable(lead: &str) -> String {
    if matches!(lead, "" | "owner" | "world" | "daemon") {
        "exec".to_string()
    } else {
        lead.to_string()
    }
}

/// Review keeps sending a Work back: it is past its third revision and the latest word from
/// its review is "changes". One wake per revision, to its lead.
pub fn review_loops(facts: &HealthFacts) -> Vec<HealthProblem> {
    facts
        .work
        .iter()
        .filter(|work| work.open() && work.revision > REVIEW_LOOP_PAST_REVISION && work.sent_back)
        .map(|work| HealthProblem {
            kind: "review-loop",
            key: format!("review-loop:{}:{}", work.id, work.revision),
            to: accountable(&work.lead),
            work_id: Some(work.id),
            detail: format!(
                "Work {} (\"{}\") is on revision {} and review has sent it back again. Past its \
                 third revision a further pass rarely supplies what is missing: an acceptance \
                 test nobody wrote down, a reviewer and a producer reading the brief differently, \
                 or a brief too big for one piece of Work. Find which it is and change that \
                 before the next pass.",
                work.id,
                short(&work.title, TITLE_CHARS),
                work.revision
            ),
        })
        .collect()
}

/// One stopped Work that several open jobs require. To the stopped Work's lead, once per stop:
/// the key holds the Attempt that stopped it (resuming does not change a Work's revision), so
/// Work that is resumed and stops again is told once more. Work that is paused on purpose,
/// parked until a time or already waiting on the owner is not raised: someone already knows.
pub fn shared_blockers(
    facts: &HealthFacts,
    now: DateTime<Utc>,
    rhythm: HealthRhythm,
) -> Vec<HealthProblem> {
    let open: HashMap<Uuid, &HealthWork> = facts
        .work
        .iter()
        .filter(|work| work.open())
        .map(|work| (work.id, work))
        .collect();
    facts
        .work
        .iter()
        .filter(|work| matches!(work.status, WorkStatus::Blocked | WorkStatus::Abandoned))
        .filter(|work| now - work.updated_at >= rhythm.shared_blocker_after())
        .filter(|work| {
            work.status == WorkStatus::Abandoned
                || !(work.waiting_on_owner
                    || work.parked
                    || work.resolution.starts_with("paused by "))
        })
        .filter_map(|work| {
            let behind: Vec<&HealthWork> = facts
                .requires
                .iter()
                .filter(|(required, _)| *required == work.id)
                .filter_map(|(_, requiring)| open.get(requiring).copied())
                .collect();
            if behind.len() < SHARED_BLOCKER_DEPENDENTS {
                return None;
            }
            let state = if work.status == WorkStatus::Abandoned {
                "was stopped for good"
            } else {
                "is blocked"
            };
            let names = behind
                .iter()
                .take(3)
                .map(|item| format!("\"{}\"", short(&item.title, TITLE_CHARS)))
                .collect::<Vec<_>>()
                .join(", ");
            let reason = if work.status == WorkStatus::Blocked && !work.resolution.trim().is_empty()
            {
                format!(
                    " Its block reads: {}.",
                    short(&work.resolution, REASON_CHARS)
                )
            } else {
                String::new()
            };
            let stopped_by = work
                .attempts
                .iter()
                .rev()
                .find(|attempt| attempt.finished_at.is_some())
                .map_or_else(
                    || format!("r{}", work.revision),
                    |attempt| attempt.id.to_string(),
                );
            Some(HealthProblem {
                kind: "shared-blocker",
                key: format!(
                    "shared-blocker:{}:{stopped_by}:{}",
                    work.id,
                    if work.status == WorkStatus::Abandoned {
                        "abandoned"
                    } else {
                        "blocked"
                    }
                ),
                to: accountable(&work.lead),
                work_id: Some(work.id),
                detail: format!(
                    "Work {} (\"{}\") {state} and {} other open jobs require it ({names}), so \
                     none of them can start.{reason} Unblock it, replace it, or drop the \
                     requirement on the jobs that can go on without it.",
                    work.id,
                    short(&work.title, TITLE_CHARS),
                    behind.len(),
                ),
            })
        })
        .collect()
}

/// How far past the threshold a cost is: 1 at the threshold, 2 at twice it, 3 at four times.
/// A problem is raised once per level, so rising cost is told again only when it has doubled.
pub fn cost_level(usd: f64, threshold: f64) -> u32 {
    if usd.is_nan() || threshold.is_nan() || threshold <= 0.0 || usd < threshold {
        return 0;
    }
    1 + (usd / threshold).log2().floor().max(0.0) as u32
}

/// Spend on one open Work, since its last result, past [`COST_WITHOUT_OUTPUT_USD`]. To its
/// lead, once per result and per doubling. Work on a route that is not charged per token has no
/// figure and is never raised here.
pub fn cost_without_output(
    facts: &HealthFacts,
    spend: &[WorkSpend],
    threshold_usd: f64,
) -> Vec<HealthProblem> {
    spend
        .iter()
        .filter_map(|spent| {
            let work = facts
                .work
                .iter()
                .find(|work| work.id == spent.work_id && work.open())?;
            let level = cost_level(spent.usd_since_output, threshold_usd);
            if level == 0 {
                return None;
            }
            let (anchor, since) = match work.last_output() {
                Some(output) => (output.id.to_string(), "since its last result"),
                None => ("none".to_string(), "and has produced no result yet"),
            };
            let attempts = work.attempts_since_output();
            Some(HealthProblem {
                kind: "cost-without-output",
                key: format!("cost-without-output:{}:{anchor}:{level}", work.id),
                to: accountable(&work.lead),
                work_id: Some(work.id),
                detail: format!(
                    "Work {} (\"{}\") has spent ${:.2} {since}, over {attempts} Attempt{}. Check \
                     whether it is too big to finish, going in circles or quietly failing; if \
                     so, cut it down, split it or stop it.",
                    work.id,
                    short(&work.title, TITLE_CHARS),
                    spent.usd_since_output,
                    if attempts == 1 { "" } else { "s" },
                ),
            })
        })
        .collect()
}

const STOPWORDS: [&str; 18] = [
    "the", "and", "for", "with", "that", "this", "from", "into", "are", "was", "will", "our",
    "your", "its", "all", "one", "new", "each",
];

/// The words a text is compared by: lower case, letters and digits only, short and filler
/// words dropped. Numbers are always kept, so "prospects 1 to 10" and "prospects 11 to 20" differ.
fn words(text: &str) -> BTreeSet<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|word| {
            let numeric = word.chars().any(|character| character.is_ascii_digit());
            numeric || (word.chars().count() >= 3 && !STOPWORDS.contains(&word.as_str()))
        })
        .filter(|word| !word.is_empty())
        .collect()
}

/// The outcome without what it rests on: two jobs sharing an assumption are not the same job.
fn outcome_body(outcome: &str) -> &str {
    let lower = outcome.to_ascii_lowercase();
    match lower.find(&crate::assumptions::ASSUMPTIONS_HEADING.to_ascii_lowercase()) {
        Some(at) => &outcome[..at],
        None => outcome,
    }
}

fn overlap(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    a.intersection(b).count() as f64 / a.union(b).count() as f64
}

/// Whether two pieces of Work read like the same job. Deliberately the obvious case only: the
/// same title words, or nearly the same title with mostly the same outcome, or a long outcome
/// that is almost word for word. It needs no model and it misses a duplicate that was reworded.
pub fn reads_like_the_same_job(a: &HealthWork, b: &HealthWork) -> bool {
    let (title_a, title_b) = (words(&a.title), words(&b.title));
    let (outcome_a, outcome_b) = (
        words(outcome_body(&a.outcome)),
        words(outcome_body(&b.outcome)),
    );
    let titles = overlap(&title_a, &title_b);
    let outcomes = overlap(&outcome_a, &outcome_b);
    (title_a.len() >= 2 && title_a == title_b)
        || (titles >= 0.75 && outcomes >= 0.6)
        || (outcome_a.len().min(outcome_b.len()) >= 8 && outcomes >= 0.9)
}

/// Open pieces of Work under one goal (or, with no goal, under one lead) that read like the same
/// job and are not linked to each other. To their lead when they share one, otherwise to Exec.
///
/// Work that matches is gathered into one group and raised once per group, so thirty copies of
/// one job are one notice and not four hundred pairs. Stopping a member makes it a different
/// group, told once more while two or more remain.
pub fn duplicate_work(facts: &HealthFacts) -> Vec<HealthProblem> {
    fn first_of(group: &[usize], mut index: usize) -> usize {
        while group[index] != index {
            index = group[index];
        }
        index
    }
    let linked: BTreeSet<(Uuid, Uuid)> = facts
        .linked
        .iter()
        .map(|(a, b)| (*a.min(b), *a.max(b)))
        .collect();
    let open: Vec<&HealthWork> = facts.work.iter().filter(|work| work.open()).collect();
    let comparable = |a: &HealthWork, b: &HealthWork| {
        (a.goal_id.is_some() && a.goal_id == b.goal_id)
            || (a.goal_id.is_none() && b.goal_id.is_none() && a.lead == b.lead)
    };
    // Each Work points at the earliest Work it reads the same as.
    let mut group: Vec<usize> = (0..open.len()).collect();
    for (index, a) in open.iter().enumerate() {
        for (later, b) in open.iter().enumerate().skip(index + 1) {
            if !comparable(a, b)
                || linked.contains(&(a.id.min(b.id), a.id.max(b.id)))
                || !reads_like_the_same_job(a, b)
            {
                continue;
            }
            let (one, other) = (first_of(&group, index), first_of(&group, later));
            if one != other {
                group[one.max(other)] = one.min(other);
            }
        }
    }
    let mut problems = Vec::new();
    for first in 0..open.len() {
        if first_of(&group, first) != first {
            continue;
        }
        let members: Vec<&HealthWork> = (0..open.len())
            .filter(|index| first_of(&group, *index) == first)
            .map(|index| open[index])
            .collect();
        if members.len() < 2 {
            continue;
        }
        let (a, b) = (members[0], members[1]);
        let mut ids: Vec<String> = members.iter().map(|work| work.id.to_string()).collect();
        ids.sort();
        let key = if ids.len() <= 3 {
            format!("duplicate-work:{}", ids.join(":"))
        } else {
            format!(
                "duplicate-work:{}:{}:and-{}-more",
                ids[0],
                ids[1],
                ids.len() - 2
            )
        };
        let to = if members.iter().all(|work| work.lead == a.lead) {
            accountable(&a.lead)
        } else {
            "exec".to_string()
        };
        let (more, all) = match members.len() - 2 {
            0 => (String::new(), "both"),
            more => (format!(" and {more} more"), "all"),
        };
        problems.push(HealthProblem {
            kind: "duplicate-work",
            key,
            to,
            work_id: Some(a.id),
            detail: format!(
                "Work {} (\"{}\", with {}) and Work {} (\"{}\", with {}){more} are {all} open {} \
                 and read like the same job. If they are, keep the one further along, fold in \
                 what the others have and stop the others for good, and say which you kept; if \
                 they are deliberately separate, carry on.",
                a.id,
                short(&a.title, TITLE_CHARS),
                a.owner_id,
                b.id,
                short(&b.title, TITLE_CHARS),
                b.owner_id,
                if a.goal_id.is_some() {
                    "under the same goal"
                } else {
                    "with the same lead"
                },
            ),
        });
    }
    problems
}

/// Typed stops that say nothing about the Work's approach: the platform, the provider, the
/// spend limit. The daemon resumes or reports those itself.
const NOT_THE_WORKS_FAILURE: [&str; 12] = [
    "quota",
    "credential",
    "model",
    "no-op",
    "transport",
    "budget",
    "container",
    "disk",
    "context",
    "resumable",
    "escalate",
    "completion_protocol",
];

/// How an Attempt failed, for telling two failures apart, or `None` when it did not fail or
/// the failure was not the Work's. Read from what the Attempt already holds: a typed stop
/// (`[kind] …`) is its kind; free text is its first twelve words with numbers and ids removed,
/// so "3 of 9 tests failed" and "4 of 9 tests failed" are the same way.
pub fn failure_signature(state: WorkAttemptState, summary: &str) -> Option<String> {
    if !matches!(state, WorkAttemptState::Failed | WorkAttemptState::Blocked) {
        return None;
    }
    let summary = summary.trim();
    if summary.starts_with(crate::UNKNOWN_OUTCOME_PREFIX)
        || summary.starts_with("paused by ")
        || summary.starts_with("awaiting ")
    {
        return None;
    }
    if let Some((kind, _)) = summary
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
    {
        let kind = kind.trim().to_lowercase();
        return (!NOT_THE_WORKS_FAILURE.contains(&kind.as_str())).then(|| format!("[{kind}]"));
    }
    Some(
        summary
            .split(|character: char| !character.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(|word| {
                if word.chars().any(|character| character.is_ascii_digit()) {
                    "#".to_string()
                } else {
                    word.to_lowercase()
                }
            })
            .take(12)
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Replan, not retry (`ARCHITECTURE.md` §4.6): the last [`SAME_FAILURES_BEFORE_REPLAN`] Attempts
/// of an open Work ended failed or blocked the same way. To its lead, once per latest Attempt:
/// a third failure of the same kind is told again, a result in between clears it.
pub fn repeated_failures(facts: &HealthFacts) -> Vec<HealthProblem> {
    facts
        .work
        .iter()
        .filter(|work| work.open())
        .filter_map(|work| {
            let ended: Vec<&HealthAttempt> = work
                .attempts
                .iter()
                .filter(|attempt| {
                    attempt.finished_at.is_some()
                        && !matches!(
                            attempt.state,
                            WorkAttemptState::Superseded | WorkAttemptState::Abandoned
                        )
                })
                .collect();
            let latest = ended.last()?;
            let way = failure_signature(latest.state, &latest.summary)?;
            let same = ended
                .iter()
                .rev()
                .take_while(|attempt| {
                    failure_signature(attempt.state, &attempt.summary).as_ref() == Some(&way)
                })
                .count();
            if same < SAME_FAILURES_BEFORE_REPLAN {
                return None;
            }
            let how = if latest.summary.trim().is_empty() {
                "no reason was recorded".to_string()
            } else {
                short(&latest.summary, REASON_CHARS)
            };
            Some(HealthProblem {
                kind: "replan",
                key: format!("replan:{}:{}", work.id, latest.id),
                to: accountable(&work.lead),
                work_id: Some(work.id),
                detail: format!(
                    "Work {} (\"{}\") has now stopped {same} times in a row the same way: {how}. \
                     Another try with the same approach is likely to end the same. Before it \
                     runs again, change the approach, split the Work, or drop it and say why; \
                     resume it only with what changed (`restless work resume --work {} --reason \
                     <what changed>`).",
                    work.id,
                    short(&work.title, TITLE_CHARS),
                    work.id,
                ),
            })
        })
        .collect()
}

/// A question for the owner that an agent has held longer than [`HealthRhythm::held_question_after`].
/// To whoever holds it, once per question. The question itself is carried to the owner's next
/// check-in whatever the holder does (`owner_questions`).
pub fn held_questions(
    facts: &HealthFacts,
    now: DateTime<Utc>,
    rhythm: HealthRhythm,
) -> Vec<HealthProblem> {
    facts
        .held
        .iter()
        .filter(|held| now - held.since >= rhythm.held_question_after())
        .map(|held| HealthProblem {
            kind: "held-question",
            key: format!("held-question:{}", held.handoff_id),
            to: accountable(&held.holder),
            work_id: Some(held.work_id),
            detail: format!(
                "You have held a question for the owner on Work {} (\"{}\") since {}: {} It now \
                 goes to the owner at Exec's next check-in, so you do not need to pass it on. Do \
                 not keep the Work waiting for the answer. If you can settle it yourself, do. \
                 Otherwise choose the most likely answer and resolve the handoff with it as a \
                 stated assumption (`restless work resolve-handoff --handoff {} --state resolved \
                 --resolution 'Assumed: <the answer you chose>'`), so the Work carries on \
                 wherever the step can be undone. Exec tells you when the owner answers.",
                held.work_id,
                short(&held.work_title, TITLE_CHARS),
                held.since.format("%-d %b %H:%M UTC"),
                short(&held.question, 300),
                held.handoff_id,
            ),
        })
        .collect()
}

/// Every problem the Work rows show at `now`, in a stable order.
pub fn health_problems(
    facts: &HealthFacts,
    now: DateTime<Utc>,
    rhythm: HealthRhythm,
    spend: &[WorkSpend],
) -> Vec<HealthProblem> {
    let mut problems = held_questions(facts, now, rhythm);
    problems.extend(repeated_failures(facts));
    problems.extend(shared_blockers(facts, now, rhythm));
    problems.extend(review_loops(facts));
    problems.extend(duplicate_work(facts));
    problems.extend(cost_without_output(facts, spend, COST_WITHOUT_OUTPUT_USD));
    problems
}

impl OrgIntel {
    /// The rows the health rules are decided from: open Work with its Attempts, the Work it
    /// requires, and questions for the owner that an agent holds.
    pub async fn health_facts(&self) -> Result<HealthFacts> {
        const OPEN: &str = "('proposed','active','blocked')";
        let mut work: Vec<HealthWork> = sqlx::query(&format!(
            "SELECT w.id, w.title, w.outcome, w.expected_artifact, w.goal_id, w.owner_id, \
                    w.status, w.resolution, w.revision, w.created_at, w.updated_at, \
                    COALESCE((SELECT t.lead_actor_id FROM actors a JOIN teams t ON t.id=a.team_id \
                               WHERE a.id=w.owner_id AND t.disbanded_at IS NULL), w.owner_id) AS lead, \
                    EXISTS (SELECT 1 FROM owner_handoffs h WHERE h.work_id=w.id \
                              AND h.state='pending' AND h.assigned_to IS NULL) AS waiting_on_owner, \
                    EXISTS (SELECT 1 FROM schedules s WHERE s.work_id=w.id \
                              AND s.fired_at IS NULL AND s.cancelled_at IS NULL) AS parked, \
                    (w.resolution LIKE 'changes requested by %' OR EXISTS ( \
                       SELECT 1 FROM work_edges e \
                       JOIN LATERAL (SELECT a.state, a.finished_at FROM work_attempts a \
                                      WHERE a.work_id=e.from_work_id AND a.finished_at IS NOT NULL \
                                      ORDER BY a.finished_at DESC LIMIT 1) review ON true \
                       WHERE e.to_work_id=w.id AND e.kind='revises' \
                         AND review.state='changes_requested' \
                         AND NOT EXISTS (SELECT 1 FROM work_attempts p WHERE p.work_id=w.id \
                                           AND p.state='produced' \
                                           AND p.finished_at > review.finished_at))) AS sent_back \
             FROM work w \
             WHERE w.status IN {OPEN} \
                OR (w.status='abandoned' AND EXISTS ( \
                      SELECT 1 FROM work_edges e JOIN work d ON d.id=e.to_work_id \
                       WHERE e.from_work_id=w.id AND e.kind='requires' AND d.status IN {OPEN})) \
             ORDER BY w.created_at, w.id"
        ))
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| HealthWork {
            id: row.get("id"),
            title: row.get("title"),
            outcome: row.get("outcome"),
            expected_artifact: row.get("expected_artifact"),
            goal_id: row.get("goal_id"),
            owner_id: row.get("owner_id"),
            lead: row.get("lead"),
            status: row.get("status"),
            resolution: row.get("resolution"),
            revision: row.get("revision"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            waiting_on_owner: row.get("waiting_on_owner"),
            parked: row.get("parked"),
            sent_back: row.get("sent_back"),
            attempts: Vec::new(),
        })
        .collect();
        let attempts = sqlx::query(&format!(
            "SELECT a.id, a.work_id, a.state, a.summary, a.finished_at FROM work_attempts a \
             JOIN work w ON w.id=a.work_id WHERE w.status IN {OPEN} \
             ORDER BY a.started_at, a.revision, a.attempt_no"
        ))
        .fetch_all(&self.pool)
        .await?;
        let mut by_work: HashMap<Uuid, Vec<HealthAttempt>> = HashMap::new();
        for row in attempts {
            by_work
                .entry(row.get("work_id"))
                .or_default()
                .push(HealthAttempt {
                    id: row.get("id"),
                    state: row.get("state"),
                    summary: row.get("summary"),
                    finished_at: row.get("finished_at"),
                });
        }
        for item in &mut work {
            item.attempts = by_work.remove(&item.id).unwrap_or_default();
        }
        let edges = sqlx::query(&format!(
            "SELECT e.from_work_id, e.to_work_id, e.kind::text AS kind FROM work_edges e \
             WHERE EXISTS (SELECT 1 FROM work w WHERE w.id IN (e.from_work_id, e.to_work_id) \
                             AND w.status IN {OPEN})"
        ))
        .fetch_all(&self.pool)
        .await?;
        let mut requires = Vec::new();
        let mut linked = Vec::new();
        for row in edges {
            let (from, to): (Uuid, Uuid) = (row.get("from_work_id"), row.get("to_work_id"));
            if row.get::<String, _>("kind") == "requires" {
                requires.push((from, to));
            }
            linked.push((from, to));
        }
        // A judgement an agent holds. An outcome review is a review to do, not a question.
        let held = sqlx::query(
            "SELECT h.id, h.work_id, w.title, h.assigned_to, h.requested_by, h.requested_action, \
                    h.prepared_state, COALESCE(h.escalated_at, h.created_at) AS since \
             FROM owner_handoffs h JOIN work w ON w.id=h.work_id JOIN actors a ON a.id=h.assigned_to \
             WHERE h.state='pending' AND h.category='owner_judgement' \
               AND a.actor_class <> 'human' AND a.retired_at IS NULL \
               AND NOT w.owner_review_required \
               AND COALESCE(h.owner_brief->>'kind','') <> 'outcome_review' \
             ORDER BY since, h.id",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| HeldQuestion {
            handoff_id: row.get("id"),
            work_id: row.get("work_id"),
            work_title: row.get("title"),
            holder: row.get("assigned_to"),
            asked_by: row.get("requested_by"),
            question: row.get("requested_action"),
            prepared: row.get("prepared_state"),
            since: row.get("since"),
        })
        .collect();
        Ok(HealthFacts {
            work,
            requires,
            linked,
            held,
        })
    }

    /// Whether this exact problem has been raised before.
    pub async fn health_problem_raised(&self, key: &str) -> Result<bool> {
        Ok(sqlx::query_scalar(&format!(
            "SELECT EXISTS(SELECT 1 FROM events WHERE kind='{HEALTH_SIGNAL_EVENT}' \
               AND body->>'key' = $1)"
        ))
        .bind(key)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Raise each problem that has not been raised before: one message from the daemon to its
    /// accountable lead (Exec when that actor is gone or is a person) and one event that holds
    /// its key. Nothing else is written; Work is never touched. Returns what was raised.
    ///
    /// A problem for Exec waits (it is not recorded, so a later sweep raises it) while a
    /// message from the owner is unread or a check-in, daily review or direction session was
    /// asked within the settling time. Past the day's bound everything waits.
    pub async fn raise_health_problems_at(
        &self,
        now: DateTime<Utc>,
        problems: &[HealthProblem],
        rhythm: HealthRhythm,
    ) -> Result<Vec<HealthProblem>> {
        let mut raised = Vec::new();
        for problem in problems {
            if self.health_problem_raised(&problem.key).await? {
                continue;
            }
            let mut tx = self.pool.begin().await?;
            // One notice per problem, even with two planes sweeping the same company at once.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
                .bind(&problem.key)
                .execute(&mut *tx)
                .await?;
            let already: bool = sqlx::query_scalar(&format!(
                "SELECT EXISTS(SELECT 1 FROM events WHERE kind='{HEALTH_SIGNAL_EVENT}' \
                   AND body->>'key' = $1)"
            ))
            .bind(&problem.key)
            .fetch_one(&mut *tx)
            .await?;
            if already {
                tx.rollback().await?;
                continue;
            }
            let today: i64 = sqlx::query_scalar(&format!(
                "SELECT count(*) FROM events WHERE kind='{HEALTH_SIGNAL_EVENT}' \
                   AND created_at > $1 AND body->>'told' IS NOT NULL"
            ))
            .bind(now - chrono::Duration::hours(24))
            .fetch_one(&mut *tx)
            .await?;
            if today >= MAX_HEALTH_WAKES_PER_DAY {
                tx.rollback().await?;
                break;
            }
            let agent: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
                   AND actor_class <> 'human')",
            )
            .bind(&problem.to)
            .fetch_one(&mut *tx)
            .await?;
            let to = if agent { problem.to.as_str() } else { "exec" };
            if to == "exec" {
                let busy: bool = sqlx::query_scalar(&format!(
                    "SELECT EXISTS(SELECT 1 FROM messages m WHERE m.to_actor='exec' \
                              AND m.read_at IS NULL AND {OWNER_WROTE}) \
                         OR EXISTS(SELECT 1 FROM events e \
                              WHERE e.kind IN ('{CHECKIN_EVENT}','{DAILY_REVIEW_EVENT}','{DIRECTION_SESSION_EVENT}') \
                                AND e.created_at > $1)"
                ))
                .bind(now - rhythm.settle())
                .fetch_one(&mut *tx)
                .await?;
                if busy {
                    tx.rollback().await?;
                    continue;
                }
            }
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1)")
                    .bind(to)
                    .fetch_one(&mut *tx)
                    .await?;
            if exists {
                let room_id = ensure_direct_message_room_in_tx(&mut tx, "daemon", Some(to)).await?;
                sqlx::query(
                    "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,'daemon',$2,$3)",
                )
                .bind(room_id)
                .bind(to)
                .bind(problem.message())
                .execute(&mut *tx)
                .await?;
            }
            sqlx::query(&format!(
                "INSERT INTO events (kind, actor_id, body, created_at) \
                 VALUES ('{HEALTH_SIGNAL_EVENT}','daemon',$1,$2)"
            ))
            .bind(serde_json::json!({
                "key": problem.key,
                "kind": problem.kind,
                "work_id": problem.work_id,
                "told": exists.then_some(to),
            }))
            .bind(now)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            if exists {
                raised.push(HealthProblem {
                    to: to.to_string(),
                    ..problem.clone()
                });
            }
        }
        Ok(raised)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(minutes: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-16T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
            + chrono::Duration::minutes(minutes)
    }

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn work(n: u128, title: &str) -> HealthWork {
        HealthWork {
            id: id(n),
            title: title.into(),
            outcome: "done".into(),
            expected_artifact: String::new(),
            goal_id: Some(id(900)),
            owner_id: "alpha-build".into(),
            lead: "alpha-lead".into(),
            status: WorkStatus::Active,
            resolution: String::new(),
            revision: 1,
            created_at: at(0),
            updated_at: at(0),
            waiting_on_owner: false,
            parked: false,
            sent_back: false,
            attempts: Vec::new(),
        }
    }

    fn attempt(n: u128, state: WorkAttemptState, summary: &str, minutes: i64) -> HealthAttempt {
        HealthAttempt {
            id: id(n),
            state,
            summary: summary.into(),
            finished_at: Some(at(minutes)),
        }
    }

    fn facts(work: Vec<HealthWork>) -> HealthFacts {
        HealthFacts {
            work,
            ..HealthFacts::default()
        }
    }

    fn rhythm() -> HealthRhythm {
        HealthRhythm::default()
    }

    #[test]
    fn a_review_loop_is_past_the_third_revision_and_still_being_sent_back() {
        let looping = HealthWork {
            revision: 4,
            sent_back: true,
            ..work(1, "Landing page copy")
        };
        let found = review_loops(&facts(vec![looping.clone()]));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, format!("review-loop:{}:4", id(1)));
        assert_eq!(found[0].to, "alpha-lead");
        // Near misses: the third revision itself, a fourth that review has not sent back, and
        // one that is finished.
        for quiet in [
            HealthWork {
                revision: 3,
                ..looping.clone()
            },
            HealthWork {
                sent_back: false,
                ..looping.clone()
            },
            HealthWork {
                status: WorkStatus::Completed,
                ..looping.clone()
            },
        ] {
            assert!(review_loops(&facts(vec![quiet])).is_empty());
        }
        // The next send-back is a new revision, so a new problem.
        let again = HealthWork {
            revision: 5,
            ..looping
        };
        assert_ne!(review_loops(&facts(vec![again]))[0].key, found[0].key);
    }

    #[test]
    fn one_stopped_work_that_two_open_jobs_require_is_a_shared_blocker() {
        let blocker = HealthWork {
            status: WorkStatus::Blocked,
            resolution: "the supplier price list is missing".into(),
            ..work(1, "Get the supplier price list")
        };
        let two_behind = HealthFacts {
            work: vec![
                blocker.clone(),
                work(2, "Price the menu"),
                work(3, "Print the menu"),
            ],
            requires: vec![(id(1), id(2)), (id(1), id(3))],
            ..HealthFacts::default()
        };
        let found = shared_blockers(&two_behind, at(30), rhythm());
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].to, "alpha-lead");
        assert!(
            found[0].detail.contains("2 other open jobs"),
            "{}",
            found[0].detail
        );
        assert!(found[0].detail.contains("supplier price list is missing"));
        // Not yet: an ordinary block resumed within half an hour is never raised.
        assert!(shared_blockers(&two_behind, at(29), rhythm()).is_empty());
        // Near miss: only one job behind it.
        let one_behind = HealthFacts {
            requires: vec![(id(1), id(2))],
            ..two_behind.clone()
        };
        assert!(shared_blockers(&one_behind, at(600), rhythm()).is_empty());
        // Someone already knows: paused on purpose, parked until a time, or with the owner.
        for known in [
            HealthWork {
                resolution: "paused by owner: waiting for the launch".into(),
                ..blocker.clone()
            },
            HealthWork {
                parked: true,
                ..blocker.clone()
            },
            HealthWork {
                waiting_on_owner: true,
                ..blocker.clone()
            },
        ] {
            let facts = HealthFacts {
                work: vec![known, work(2, "Price the menu"), work(3, "Print the menu")],
                ..two_behind.clone()
            };
            assert!(shared_blockers(&facts, at(600), rhythm()).is_empty());
        }
        // Abandoned Work that open jobs still require is one too, and a different problem.
        let abandoned = HealthFacts {
            work: vec![
                HealthWork {
                    status: WorkStatus::Abandoned,
                    ..blocker.clone()
                },
                work(2, "Price the menu"),
                work(3, "Print the menu"),
            ],
            ..two_behind.clone()
        };
        let stranded = shared_blockers(&abandoned, at(600), rhythm());
        assert_eq!(stranded.len(), 1);
        assert_ne!(stranded[0].key, found[0].key);
        // Resumed and blocked again is a new Attempt: told once more.
        let next = HealthFacts {
            work: vec![
                HealthWork {
                    attempts: vec![attempt(
                        20,
                        WorkAttemptState::Blocked,
                        "the supplier price list is missing",
                        5,
                    )],
                    ..blocker
                },
                work(2, "Price the menu"),
                work(3, "Print the menu"),
            ],
            ..two_behind
        };
        assert_ne!(
            shared_blockers(&next, at(600), rhythm())[0].key,
            found[0].key
        );
    }

    #[test]
    fn cost_with_no_result_is_raised_at_the_threshold_and_again_only_when_it_doubles() {
        assert_eq!(cost_level(1.99, 2.0), 0);
        assert_eq!(cost_level(2.0, 2.0), 1);
        assert_eq!(cost_level(3.99, 2.0), 1);
        assert_eq!(cost_level(4.0, 2.0), 2);
        assert_eq!(cost_level(8.5, 2.0), 3);
        assert_eq!(cost_level(f64::NAN, 2.0), 0);

        let running = HealthWork {
            attempts: vec![attempt(10, WorkAttemptState::Failed, "tests failed", 5)],
            ..work(1, "Rebuild the importer")
        };
        let spent = |usd| {
            [WorkSpend {
                work_id: id(1),
                usd_since_output: usd,
            }]
        };
        let state = facts(vec![running.clone()]);
        assert!(cost_without_output(&state, &spent(1.5), 2.0).is_empty());
        let first = cost_without_output(&state, &spent(2.4), 2.0);
        assert_eq!(first.len(), 1);
        assert!(first[0].detail.contains("$2.40"), "{}", first[0].detail);
        assert!(first[0].detail.contains("produced no result yet"));
        assert!(
            first[0].detail.contains("1 Attempt."),
            "{}",
            first[0].detail
        );
        // More of the same is the same problem; doubled is a new one.
        assert_eq!(
            cost_without_output(&state, &spent(3.9), 2.0)[0].key,
            first[0].key
        );
        assert_ne!(
            cost_without_output(&state, &spent(4.1), 2.0)[0].key,
            first[0].key
        );
        // A result clears it: what was spent before the result no longer counts, and the key
        // names the result, so spend after it is a new problem.
        let produced = HealthWork {
            attempts: vec![
                attempt(10, WorkAttemptState::Failed, "tests failed", 5),
                attempt(11, WorkAttemptState::Produced, "done", 9),
            ],
            ..running.clone()
        };
        let after = cost_without_output(&facts(vec![produced]), &spent(2.4), 2.0);
        assert_ne!(after[0].key, first[0].key);
        assert!(after[0].detail.contains("since its last result"));
        // Finished Work is never raised.
        let done = HealthWork {
            status: WorkStatus::Completed,
            ..running
        };
        assert!(cost_without_output(&facts(vec![done]), &spent(40.0), 2.0).is_empty());
    }

    #[test]
    fn only_the_obvious_duplicate_is_called_one() {
        let a = HealthWork {
            outcome: "Twelve local accounting firms listed with a contact each.\nAssumptions:\n- Local firms answer cold email.".into(),
            ..work(1, "List twelve local prospects")
        };
        let same = HealthWork {
            owner_id: "beta-check".into(),
            outcome: "Twelve local accounting firms listed, each with a contact.".into(),
            ..work(2, "List twelve local prospects")
        };
        let found = duplicate_work(&facts(vec![a.clone(), same.clone()]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].key, format!("duplicate-work:{}:{}", id(1), id(2)));
        assert_eq!(found[0].to, "alpha-lead", "one lead answers for both");
        assert!(found[0].detail.contains("under the same goal"));
        // Two leads: nobody below Exec answers for both.
        let other_lead = HealthWork {
            lead: "beta-lead".into(),
            ..same.clone()
        };
        assert_eq!(
            duplicate_work(&facts(vec![a.clone(), other_lead]))[0].to,
            "exec"
        );

        // Near misses. A batch split by number; a different job on the same subject; the same
        // words under another goal; a review of the Work, which is linked to it; finished Work.
        let batch_one = work(3, "Write to prospects 1 to 10");
        let batch_two = work(4, "Write to prospects 11 to 20");
        assert!(duplicate_work(&facts(vec![batch_one, batch_two])).is_empty());
        let related = HealthWork {
            outcome: "Each of the twelve firms written to once.".into(),
            ..work(5, "Write to the twelve local prospects")
        };
        assert!(duplicate_work(&facts(vec![a.clone(), related])).is_empty());
        let elsewhere = HealthWork {
            goal_id: Some(id(901)),
            ..same.clone()
        };
        assert!(duplicate_work(&facts(vec![a.clone(), elsewhere])).is_empty());
        let linked = HealthFacts {
            work: vec![a.clone(), same.clone()],
            linked: vec![(id(2), id(1))],
            ..HealthFacts::default()
        };
        assert!(duplicate_work(&linked).is_empty());
        let finished = HealthWork {
            status: WorkStatus::Completed,
            ..same
        };
        assert!(duplicate_work(&facts(vec![a.clone(), finished])).is_empty());

        // With no goal, only the same lead's Work is compared.
        let loose = |n, lead: &str| HealthWork {
            goal_id: None,
            lead: lead.into(),
            ..work(n, "Tidy the shared drive")
        };
        assert_eq!(
            duplicate_work(&facts(vec![loose(6, "alpha-lead"), loose(7, "alpha-lead")])).len(),
            1
        );
        assert!(
            duplicate_work(&facts(vec![loose(6, "alpha-lead"), loose(7, "beta-lead")])).is_empty()
        );
        // Many copies of one job are one group and one notice. Stopping one makes it a new
        // group, told once more while two or more remain.
        let copies: Vec<HealthWork> = (10..15)
            .map(|n| work(n, "Write the welcome email"))
            .collect();
        let many = duplicate_work(&facts(copies.clone()));
        assert_eq!(many.len(), 1, "{many:?}");
        assert!(
            many[0].detail.contains("and 3 more are all open"),
            "{}",
            many[0].detail
        );
        let fewer = duplicate_work(&facts(copies[1..].to_vec()));
        assert_eq!(fewer.len(), 1);
        assert_ne!(fewer[0].key, many[0].key);
        let pair = duplicate_work(&facts(copies[3..].to_vec()));
        assert_eq!(pair[0].key, format!("duplicate-work:{}:{}", id(13), id(14)));
        assert!(duplicate_work(&facts(copies[4..].to_vec())).is_empty());
        // A reworded duplicate is missed. That is the price of needing no model.
        let reworded = HealthWork {
            outcome: "A dozen nearby bookkeepers we could sell to.".into(),
            ..work(8, "Find a dozen nearby leads")
        };
        assert!(duplicate_work(&facts(vec![a, reworded])).is_empty());
    }

    #[test]
    fn the_same_way_is_read_from_what_the_attempt_holds() {
        use WorkAttemptState::{Blocked, Failed, Produced};
        // Free text: numbers and ids do not make it a different failure.
        assert_eq!(
            failure_signature(Failed, "3 of 9 tests failed in importer.rs"),
            failure_signature(Failed, "4 of 9 tests failed in importer.rs"),
        );
        assert_ne!(
            failure_signature(Failed, "3 of 9 tests failed"),
            failure_signature(Failed, "the build did not compile"),
        );
        // A typed stop is its kind, whatever follows.
        assert_eq!(
            failure_signature(Blocked, "[gate] the lint gate failed: 12 warnings"),
            Some("[gate]".into())
        );
        // Not the Work's failure: the platform, the provider, a pause, a wait on a person.
        for platform in [
            "[quota] provider refused on quota or credit",
            "[transport] the agent session failed",
            "[resumable] the turn was cut",
            "paused by owner: later",
            "awaiting owner handoff 5b2c",
            "cognitive process ended without trustworthy semantic completion; productive outcome unknown: supervisor restarted",
        ] {
            assert_eq!(failure_signature(Blocked, platform), None, "{platform}");
        }
        assert_eq!(failure_signature(Produced, "done"), None);
    }

    #[test]
    fn two_failures_the_same_way_ask_for_a_replan_once_per_latest_attempt() {
        use WorkAttemptState::{Blocked, Failed, Produced};
        let twice = HealthWork {
            status: WorkStatus::Blocked,
            attempts: vec![
                attempt(10, Failed, "3 of 9 tests failed", 5),
                attempt(11, Failed, "4 of 9 tests failed", 9),
            ],
            ..work(1, "Rebuild the importer")
        };
        let found = repeated_failures(&facts(vec![twice.clone()]));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, format!("replan:{}:{}", id(1), id(11)));
        assert_eq!(found[0].to, "alpha-lead");
        assert!(
            found[0].detail.contains("2 times in a row"),
            "{}",
            found[0].detail
        );
        assert!(found[0]
            .detail
            .contains("change the approach, split the Work, or drop it"));
        // Near misses: once; twice in different ways; a result in between; a platform stop.
        for attempts in [
            vec![attempt(10, Failed, "3 of 9 tests failed", 5)],
            vec![
                attempt(10, Failed, "3 of 9 tests failed", 5),
                attempt(11, Blocked, "the staging database is down", 9),
            ],
            vec![
                attempt(10, Failed, "3 of 9 tests failed", 5),
                attempt(11, Produced, "done", 7),
                attempt(12, Failed, "3 of 9 tests failed", 9),
            ],
            vec![
                attempt(10, Blocked, "[quota] provider refused", 5),
                attempt(11, Blocked, "[quota] provider refused", 9),
            ],
        ] {
            let quiet = HealthWork {
                attempts,
                ..twice.clone()
            };
            assert!(repeated_failures(&facts(vec![quiet])).is_empty());
        }
        // A third failure the same way is a new Attempt, so one more ask.
        let thrice = HealthWork {
            attempts: vec![
                attempt(10, Failed, "3 of 9 tests failed", 5),
                attempt(11, Failed, "4 of 9 tests failed", 9),
                attempt(12, Failed, "4 of 9 tests failed", 14),
            ],
            ..twice
        };
        let third = repeated_failures(&facts(vec![thrice]));
        assert_ne!(third[0].key, found[0].key);
        assert!(third[0].detail.contains("3 times in a row"));
    }

    #[test]
    fn a_question_held_for_an_hour_goes_up_once() {
        let held = HeldQuestion {
            handoff_id: id(50),
            work_id: id(1),
            work_title: "Price the menu".into(),
            holder: "alpha-lead".into(),
            asked_by: "alpha-build".into(),
            question: "Should the menu show prices with or without tax?".into(),
            prepared: "Both versions are drafted.".into(),
            since: at(0),
        };
        let state = HealthFacts {
            held: vec![held],
            ..HealthFacts::default()
        };
        assert!(held_questions(&state, at(59), rhythm()).is_empty());
        let found = held_questions(&state, at(60), rhythm());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].to, "alpha-lead");
        assert_eq!(found[0].key, format!("held-question:{}", id(50)));
        assert!(found[0].detail.contains("with or without tax"));
        assert!(found[0].detail.contains("Do not keep the Work waiting"));
        // The same question a day later is the same problem.
        assert_eq!(
            held_questions(&state, at(60 * 24), rhythm())[0].key,
            found[0].key
        );
    }

    #[test]
    fn a_healthy_company_has_no_problems_and_a_notice_never_blocks() {
        let healthy = HealthFacts {
            work: vec![
                work(1, "List twelve local prospects"),
                HealthWork {
                    attempts: vec![attempt(10, WorkAttemptState::Produced, "done", 5)],
                    ..work(2, "Draft the one-line offer")
                },
                HealthWork {
                    status: WorkStatus::Blocked,
                    resolution: "waiting for the supplier".into(),
                    attempts: vec![attempt(
                        11,
                        WorkAttemptState::Blocked,
                        "waiting for the supplier",
                        5,
                    )],
                    ..work(3, "Order the samples")
                },
            ],
            requires: vec![(id(1), id(2))],
            linked: vec![(id(1), id(2))],
            held: Vec::new(),
        };
        assert!(health_problems(&healthy, at(600), rhythm(), &[]).is_empty());

        let problem = HealthProblem {
            kind: "replan",
            key: "replan:x".into(),
            to: "alpha-lead".into(),
            work_id: None,
            detail: "Work x has stopped twice.".into(),
        };
        let message = problem.message();
        assert!(message.starts_with(HEALTH_NOTICE_PREFIX), "{message}");
        assert!(message.contains("Nothing has been paused or changed"));
        assert!(message.contains("if nothing is wrong, carry on"));
        assert!(message.contains("You will not be told again unless it changes"));
    }

    #[test]
    fn the_waits_shorten_with_the_interval() {
        let fast = HealthRhythm {
            every: chrono::Duration::seconds(20),
        };
        assert_eq!(fast.shared_blocker_after(), chrono::Duration::seconds(60));
        assert_eq!(fast.held_question_after(), chrono::Duration::seconds(120));
        assert_eq!(fast.settle(), chrono::Duration::seconds(30));
        assert_eq!(rhythm().held_question_after(), chrono::Duration::hours(1));
        assert_eq!(
            rhythm().shared_blocker_after(),
            chrono::Duration::minutes(30)
        );
        assert_eq!(rhythm().settle(), chrono::Duration::minutes(15));
    }
}
