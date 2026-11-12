//! The company reviews its own shape when the owner has given it enough new signal.
//!
//! Owner decision, 12 November 2026: "self evolution should happen as soon as there's enough new
//! usage data in the company ie signal from the human operator", and "rn it sort of just freezes
//! itself into shitty org structures that don't make sense".
//!
//! Before this, every restructuring move existed and none was ever prompted. Exec wakes only on
//! events, the operating rules told it to leave teams alone without the owner's agreement, and
//! nothing measured whether a team, a lead or a critic was earning its place. Reno ended with one
//! lead, nine Staff and three critics judging the same work, found by the owner reading the
//! graph.
//!
//! The trigger is the owner, not a clock: outcomes they sent back, corrections recorded as
//! lessons, and nudges ("progress?", "try now") that mean they had to pull. Each already wakes
//! Exec, so a due review costs one prompt block on a turn that was happening anyway, and no model
//! is ever woken to find nothing changed. Exec finishes with `restless org review`, which moves
//! the watermark; the next review compares against the numbers stored then.

use chrono::{DateTime, Utc};
use restless_orgintel::{GoalMinds, OrgIntel, OrgReviewRow, OwnerSignalRow, TeamScore};
use serde::Serialize;

/// With no earlier review, and for the scorecard `restless org scorecard` prints, look this far
/// back. Long enough to hold a company's recent shape, short enough that a team reorganised a
/// month ago is not judged on its old numbers.
const FIRST_WINDOW_DAYS: i64 = 14;
/// Signals that say something went wrong for the owner: sent back, corrected, had to nudge.
const REVIEW_AFTER_NEGATIVE: usize = 3;
/// Decisions of any kind. A run of clean accepts is also evidence worth one look, far less often.
const REVIEW_AFTER_ANY: usize = 12;
/// Below this many finished Attempts the rates are noise, and a review would act on anecdote.
const MIN_ATTEMPTS: i64 = 6;
/// A team's critics are questioned only after this many runs.
const CRITIC_RUNS_BEFORE_JUDGING: i64 = 4;
/// A goal this wide is named in the review whatever its rates.
const WIDE_GOAL_MINDS: i64 = 5;
const MAX_TEAMS_SHOWN: usize = 8;

/// What the owner's actions since the last review add up to.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OwnerSignals {
    pub sent_back: usize,
    pub accepted: usize,
    /// Short messages asking for status or telling stalled work to try again.
    pub nudges: usize,
    /// Corrections recorded as lessons, and lessons that came back.
    pub lessons: usize,
    /// A few of the owner's own words, newest last, so the review reads evidence, not a count.
    pub examples: Vec<String>,
}

impl OwnerSignals {
    pub fn negative(&self) -> usize {
        self.sent_back + self.nudges + self.lessons
    }
    pub fn total(&self) -> usize {
        self.negative() + self.accepted
    }
}

/// A short owner message that means "I had to come and pull". Deliberately a closed list of what
/// the owner actually types (Reno and Frankensurf, October to November: "progress?" 24 times),
/// matched on the whole message, so a real brief that merely contains "status" is not a nudge.
pub fn is_nudge(body: &str) -> bool {
    let text: String = body
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    const NUDGES: &[&str] = &[
        "progress",
        "any progress",
        "status",
        "status update",
        "any update",
        "any updates",
        "hows it going",
        "how is it going",
        "where are we",
        "where are we at",
        "whats happening",
        "whats going on",
        "try now",
        "try again",
        "try it now",
        "try again now",
        "retry",
        "signed in",
        "logged in",
        "im signed in",
        "im logged in",
        "done try now",
        "continue",
        "keep going",
        "go on",
        "resume",
        "still there",
        "are you there",
    ];
    NUDGES.contains(&text.as_str())
}

pub fn classify(rows: &[OwnerSignalRow]) -> OwnerSignals {
    let mut signals = OwnerSignals::default();
    let example = |text: &str| text.trim().chars().take(100).collect::<String>();
    for row in rows {
        match row.kind.as_str() {
            "sent_back" => {
                signals.sent_back += 1;
                signals.examples.push(example(&row.detail));
            }
            "accepted" => signals.accepted += 1,
            "lesson" => signals.lessons += 1,
            "message" if is_nudge(&row.detail) => {
                signals.nudges += 1;
                signals.examples.push(format!("\"{}\"", example(&row.detail)));
            }
            _ => {}
        }
    }
    let keep = signals.examples.len().saturating_sub(5);
    signals.examples.drain(..keep);
    signals
}

/// The numbers a review reads, and the stored form the next review compares against.
#[derive(Debug, Clone, Serialize)]
pub struct Scorecard {
    pub since: DateTime<Utc>,
    pub teams: Vec<TeamScore>,
    pub goals: Vec<GoalMinds>,
    pub signals: OwnerSignals,
    pub signals_through: DateTime<Utc>,
    pub last_review: Option<LastReview>,
    pub due: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LastReview {
    pub at: DateTime<Utc>,
    pub conclusion: String,
    pub teams: Vec<TeamScore>,
}

fn percent(part: i64, whole: i64) -> i64 {
    if whole <= 0 {
        0
    } else {
        (part * 100 + whole / 2) / whole
    }
}

/// Enough new owner signal, and enough finished work for the rates to mean something.
pub fn due(signals: &OwnerSignals, teams: &[TeamScore]) -> bool {
    let attempts: i64 = teams
        .iter()
        .map(|team| team.attempts + team.critic_attempts)
        .sum();
    attempts >= MIN_ATTEMPTS
        && (signals.negative() >= REVIEW_AFTER_NEGATIVE || signals.total() >= REVIEW_AFTER_ANY)
}

pub async fn scorecard(org: &OrgIntel, now: DateTime<Utc>) -> anyhow::Result<Scorecard> {
    let last = org.latest_org_review().await?;
    let since = last
        .as_ref()
        .map(|review| review.signals_through)
        .unwrap_or(now - chrono::Duration::days(FIRST_WINDOW_DAYS));
    let (teams, goals, rows) = tokio::try_join!(
        org.team_scores(since),
        org.goal_minds(since),
        org.owner_signals_after(since),
    )?;
    let signals = classify(&rows);
    let due = due(&signals, &teams);
    Ok(Scorecard {
        since,
        due,
        teams,
        goals,
        signals,
        signals_through: now,
        last_review: last.map(last_review),
    })
}

fn last_review(row: OrgReviewRow) -> LastReview {
    LastReview {
        at: row.reviewed_at,
        teams: row
            .scorecard
            .get("teams")
            .cloned()
            .and_then(|teams| serde_json::from_value(teams).ok())
            .unwrap_or_default(),
        conclusion: row.conclusion,
    }
}

fn team_label(team: &TeamScore) -> String {
    match (&team.team_id, &team.lead_actor_id) {
        (Some(_), Some(lead)) => format!("\"{}\" (lead {lead})", team.team_name),
        _ => "no team (Exec and unassigned)".to_string(),
    }
}

/// One team, in a line a reviewer can act on: the rate first, then what stands out.
pub fn team_line(team: &TeamScore, before: Option<&TeamScore>) -> String {
    let mut line = format!(
        "{}: {} of {} attempts landed ({}%)",
        team_label(team),
        team.landed,
        team.attempts,
        percent(team.landed, team.attempts)
    );
    if let Some(before) = before.filter(|before| before.attempts > 0) {
        line.push_str(&format!(
            ", was {}% at the last review",
            percent(before.landed, before.attempts)
        ));
    }
    if team.failed > 0 {
        line.push_str(&format!("; {} failed", team.failed));
    }
    if team.blocked > 0 {
        line.push_str(&format!("; {} blocked", team.blocked));
    }
    if team.owner_sent_back > 0 {
        line.push_str(&format!("; owner sent back {}", team.owner_sent_back));
    }
    line.push_str(&format!(
        "; {} minds on {} Work",
        team.minds, team.work_items
    ));
    if team.critic_attempts > 0 {
        line.push_str(&format!(
            "; critics ran {} times and sent work back {}",
            team.critic_attempts, team.critic_catches
        ));
        if team.critic_attempts >= CRITIC_RUNS_BEFORE_JUDGING && team.critic_catches == 0 {
            line.push_str(" (they changed nothing: drop them unless a mistake here is costly)");
        }
    }
    line
}

fn signal_sentence(signals: &OwnerSignals) -> String {
    let mut parts = Vec::new();
    let plural = |n: usize, one: &str, many: &str| {
        format!("{n} {}", if n == 1 { one } else { many })
    };
    if signals.sent_back > 0 {
        parts.push(plural(
            signals.sent_back,
            "outcome sent back",
            "outcomes sent back",
        ));
    }
    if signals.lessons > 0 {
        parts.push(plural(signals.lessons, "correction", "corrections"));
    }
    if signals.nudges > 0 {
        parts.push(format!(
            "{} asking for status or telling stalled work to try again",
            plural(signals.nudges, "message", "messages")
        ));
    }
    if signals.accepted > 0 {
        parts.push(plural(
            signals.accepted,
            "outcome accepted",
            "outcomes accepted",
        ));
    }
    if parts.is_empty() {
        "nothing yet".to_string()
    } else {
        parts.join(", ")
    }
}

/// The scorecard as text: what `restless org scorecard` prints and what a due review shows.
pub fn render_numbers(card: &Scorecard) -> String {
    let mut out = format!(
        "Since {}: the owner gave {}.\n",
        card.since.format("%-d %b"),
        signal_sentence(&card.signals)
    );
    for example in &card.signals.examples {
        out.push_str(&format!("  - {example}\n"));
    }
    if card.teams.is_empty() {
        out.push_str("No Attempt has finished in that time.\n");
    }
    for team in card.teams.iter().take(MAX_TEAMS_SHOWN) {
        let before = card.last_review.as_ref().and_then(|last| {
            last.teams
                .iter()
                .find(|before| before.team_id == team.team_id)
        });
        out.push_str(&format!("- {}\n", team_line(team, before)));
    }
    if card.teams.len() > MAX_TEAMS_SHOWN {
        out.push_str(&format!(
            "- and {} smaller teams (`restless org scorecard`)\n",
            card.teams.len() - MAX_TEAMS_SHOWN
        ));
    }
    for goal in card
        .goals
        .iter()
        .filter(|goal| goal.minds >= WIDE_GOAL_MINDS || goal.critics >= 2)
    {
        out.push_str(&format!(
            "- goal \"{}\": {} minds, {} of them critics, on {} Work. Each mind is another \
             briefing and another place to lose the thread.\n",
            goal.title.chars().take(60).collect::<String>(),
            goal.minds,
            goal.critics,
            goal.work_items
        ));
    }
    if let Some(last) = &card.last_review {
        out.push_str(&format!(
            "Last review ({}): {}\n",
            last.at.format("%-d %b"),
            last.conclusion
        ));
    }
    out
}

/// The block a due review adds to Exec's turn. `None` when no review is due, so an ordinary wake
/// pays nothing for this.
pub fn exec_block(card: &Scorecard) -> Option<String> {
    if !card.due {
        return None;
    }
    Some(format!(
        "\n# Review how the company is organised [owed this turn: the owner has given enough new \
         signal]\n{}\
         Read the numbers against the owner's words, then make the one or two changes most likely \
         to stop them sending work back or asking for status. This is yours to do without asking:\n\
         - Shrink freely. Release a member with no current or next Work (`restless teams assign \
         --actor <id> --team none`), retire a specialist nothing needs (`restless people retire`), \
         hand a team's remaining Work to one lead and disband what is left (`restless teams \
         disband`), and stop commissioning critics that changed nothing.\n\
         - Reshape for outcomes the owner already asked for: give a goal that can be judged on \
         its own its own lead, or move a lead whose team keeps failing the same way. Only a team \
         or spend beyond what the owner asked for needs their yes.\n\
         - If the last review's change made its number worse, undo it.\n\
         - Status and try-again messages mean the owner had to pull. Say what you changed so they \
         need not ask next time.\n\
         Tell the owner each change in one line with the number behind it. Then record it, or this \
         stays owed: `restless org review --conclusion '<what the numbers showed, what you \
         changed, or why nothing needed changing>'`.\n",
        render_numbers(card)
    ))
}

/// What the review stores for the next one to compare against.
pub fn stored(card: &Scorecard) -> serde_json::Value {
    serde_json::json!({ "since": card.since, "teams": card.teams, "goals": card.goals })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(kind: &str, detail: &str) -> OwnerSignalRow {
        OwnerSignalRow {
            kind: kind.into(),
            at: Utc::now(),
            actor_id: None,
            detail: detail.into(),
        }
    }

    fn team(name: &str, attempts: i64, landed: i64) -> TeamScore {
        TeamScore {
            team_id: Some(uuid::Uuid::from_u128(name.len() as u128)),
            team_name: name.into(),
            lead_actor_id: Some("growth-lead".into()),
            attempts,
            landed,
            failed: 0,
            blocked: 0,
            critic_attempts: 0,
            critic_catches: 0,
            minds: 2,
            work_items: 3,
            owner_sent_back: 0,
        }
    }

    fn card(signals: OwnerSignals, teams: Vec<TeamScore>) -> Scorecard {
        let due = due(&signals, &teams);
        Scorecard {
            since: Utc::now() - chrono::Duration::days(3),
            teams,
            goals: Vec::new(),
            signals,
            signals_through: Utc::now(),
            last_review: None,
            due,
        }
    }

    #[test]
    fn the_owners_real_nudges_count_and_a_brief_does_not() {
        for nudge in [
            "progress?",
            "Progress?",
            " try now ",
            "signed in",
            "any update?",
            "status",
            "how's it going?",
            "I'm signed in.",
        ] {
            assert!(is_nudge(nudge), "{nudge}");
        }
        for brief in [
            "send me the status report for Q3",
            "try the new pricing page now",
            "progress on the launch is what matters most",
            "yes",
            "approved",
            "",
        ] {
            assert!(!is_nudge(brief), "{brief}");
        }
    }

    #[test]
    fn signals_are_counted_by_what_the_owner_did() {
        let signals = classify(&[
            row("sent_back", "The owner requested changes: too long"),
            row("accepted", ""),
            row("message", "progress?"),
            row("message", "thanks"),
            row("lesson", "lesson_recorded"),
        ]);
        assert_eq!(
            (
                signals.sent_back,
                signals.accepted,
                signals.nudges,
                signals.lessons
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(signals.negative(), 3);
        assert_eq!(signals.examples.len(), 2);
    }

    #[test]
    fn only_the_newest_examples_are_kept() {
        let rows: Vec<_> = (0..9)
            .map(|n| row("sent_back", &format!("reason {n}")))
            .collect();
        let signals = classify(&rows);
        assert_eq!(signals.examples.len(), 5);
        assert_eq!(signals.examples.last().unwrap(), "reason 8");
    }

    #[test]
    fn a_review_needs_owner_signal_and_enough_finished_work() {
        let unhappy = OwnerSignals {
            sent_back: 2,
            nudges: 1,
            ..Default::default()
        };
        assert!(due(&unhappy, &[team("Growth", 10, 5)]));
        // Three signals over two Attempts is anecdote.
        assert!(!due(&unhappy, &[team("Growth", 2, 1)]));
        // Plenty of work, and the owner said nothing: no review, no cost.
        assert!(!due(&OwnerSignals::default(), &[team("Growth", 40, 20)]));
        // Two complaints are below the line; a long run of accepts earns one look.
        let two = OwnerSignals {
            sent_back: 2,
            ..Default::default()
        };
        assert!(!due(&two, &[team("Growth", 10, 5)]));
        let content = OwnerSignals {
            accepted: 12,
            ..Default::default()
        };
        assert!(due(&content, &[team("Growth", 10, 9)]));
    }

    #[test]
    fn an_ordinary_wake_carries_no_review_block() {
        assert!(exec_block(&card(OwnerSignals::default(), vec![team("Growth", 40, 20)])).is_none());
    }

    #[test]
    fn a_due_review_shows_rates_words_and_how_to_record_it() {
        let mut growth = team("Growth", 10, 4);
        growth.failed = 3;
        growth.owner_sent_back = 2;
        growth.critic_attempts = 5;
        let signals = classify(&[
            row("sent_back", "too long"),
            row("sent_back", "wrong audience"),
            row("message", "progress?"),
        ]);
        let block = exec_block(&card(signals, vec![growth])).unwrap();
        assert!(block.contains("4 of 10 attempts landed (40%)"), "{block}");
        assert!(block.contains("owner sent back 2"), "{block}");
        assert!(block.contains("they changed nothing"), "{block}");
        assert!(block.contains("wrong audience"), "{block}");
        assert!(block.contains("restless org review --conclusion"), "{block}");
        assert!(block.contains("2 outcomes sent back"), "{block}");
    }

    #[test]
    fn the_next_review_sees_what_the_last_one_did() {
        let mut card = card(
            OwnerSignals {
                sent_back: 3,
                ..Default::default()
            },
            vec![team("Growth", 10, 8)],
        );
        card.last_review = Some(LastReview {
            at: Utc::now() - chrono::Duration::days(3),
            conclusion: "Released two idle members from Growth.".into(),
            teams: vec![team("Growth", 10, 4)],
        });
        let text = render_numbers(&card);
        assert!(text.contains("(80%), was 40% at the last review"), "{text}");
        assert!(text.contains("Released two idle members"), "{text}");
    }

    #[test]
    fn critics_that_catch_things_are_left_alone() {
        let mut growth = team("Growth", 10, 8);
        growth.critic_attempts = 6;
        growth.critic_catches = 2;
        assert!(!team_line(&growth, None).contains("changed nothing"));
        // Too few runs to judge.
        growth.critic_attempts = 2;
        growth.critic_catches = 0;
        assert!(!team_line(&growth, None).contains("changed nothing"));
    }

    #[test]
    fn a_wide_goal_is_named() {
        let mut card = card(OwnerSignals::default(), vec![team("Growth", 10, 8)]);
        card.goals = vec![
            GoalMinds {
                goal_id: uuid::Uuid::nil(),
                title: "Launch Reno".into(),
                minds: 9,
                critics: 3,
                work_items: 14,
            },
            GoalMinds {
                goal_id: uuid::Uuid::nil(),
                title: "Small".into(),
                minds: 2,
                critics: 0,
                work_items: 2,
            },
        ];
        let text = render_numbers(&card);
        assert!(text.contains("\"Launch Reno\": 9 minds, 3 of them critics"), "{text}");
        assert!(!text.contains("\"Small\""), "{text}");
    }
}
