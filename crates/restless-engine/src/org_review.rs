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
//!
//! The owner's friction (docs/specs/owner-interaction.md D7, 20 November 2026) is a second way a
//! review becomes due: over the last week, status checks above 8% of the owner's messages,
//! redirect runs above the owner's own baseline, or any Work reopened. Each needs something new
//! since the last review, so a number that stays high asks once, not on every turn.
//!
//! Exec judges each owner message as it answers it (21 November 2026): whether the company could
//! have spared the owner it, the gap and the repair. Those labels come first: at least 3 avoidable
//! in the owner's last 10 messages, or the same gap twice in the week, makes a review due, and the
//! review hands each repair to the lead who owns it as Work. Once Exec has judged enough of the
//! week, the status-check and redirect-run triggers, which read the rules' classes, stand down.

use chrono::{DateTime, Utc};
use restless_orgintel::{
    classify_owner_message, Gap, GoalMinds, OrgIntel, OrgReviewRow, OwnerFriction, OwnerGap,
    OwnerIntent, OwnerSignalRow, TeamScore,
};
use serde::Serialize;

/// Team numbers always cover this long. Long enough to hold a company's recent shape, short enough
/// that a team reorganised a month ago is not judged on its old numbers. Owner signals are
/// counted from the last review instead (or this far back when there has been none), so a review
/// answers new signal with a full window of results, never the few Attempts since yesterday.
const SCORE_WINDOW_DAYS: i64 = 14;
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
/// Owner friction covers this long: a week holds the owner's working rhythm.
const FRICTION_WINDOW_DAYS: i64 = 7;
/// Status checks were 2% of the owner's prompts with one agent and rose to about 8% (their 20
/// November report). Above that, the company is making them pull more than one agent did.
pub const STATUS_CHECK_BASELINE_PERCENT: i64 = 8;
/// Below this many owner messages in the week a share is anecdote ("progress?" once in five).
const MIN_OWNER_MESSAGES: i64 = 10;
/// Below this many redirects a run rate is anecdote.
const MIN_REDIRECTS: i64 = 3;
/// This many of the owner's last ten messages judged avoidable make a review due.
pub const AVOIDABLE_IN_RECENT: i64 = 3;
/// A gap behind this many avoidable messages in the week is a pattern, not a slip.
pub const REPEATED_GAP: i64 = 2;
/// From this many messages Exec judged in the week, its labels decide: the share-of-status-checks
/// and redirect-run triggers, which read the rules' classes, stand down for the avoidable ones.
const EXEC_LABELS_DECIDE: i64 = 5;

/// What the owner's friction is judged against. `redirect_run_percent` is the run rate stored at
/// the last review when that review had enough redirects to have one; before that it is the
/// owner's own share of messages that are redirects, so a run counts only when redirects cluster
/// more than the owner's base rate (their records: 3.9 times as likely after one redirect).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FrictionBaseline {
    pub status_check_percent: i64,
    pub redirect_run_percent: i64,
    pub from_last_review: bool,
}

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

/// A short owner message that means "I had to come and pull": a status check, by the same rules
/// that classify every owner message (`restless_orgintel::classify_owner_message`), or one of a
/// closed list of what the owner types to push stalled work on (Reno and Frankensurf, October to
/// November: "progress?" 24 times). Matched on the whole message, so a real brief that merely
/// contains "status" is not a nudge.
pub fn is_nudge(body: &str) -> bool {
    if classify_owner_message(body, false) == OwnerIntent::StatusCheck {
        return true;
    }
    let text: String = body
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    const NUDGES: &[&str] = &[
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
                signals
                    .examples
                    .push(format!("\"{}\"", example(&row.detail)));
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
    /// Team and goal numbers cover Attempts from here.
    pub since: DateTime<Utc>,
    /// Owner signals are counted from here: the last review, or the same window.
    pub signals_since: DateTime<Utc>,
    pub teams: Vec<TeamScore>,
    pub goals: Vec<GoalMinds>,
    pub signals: OwnerSignals,
    pub signals_through: DateTime<Utc>,
    /// The owner's friction over the last week, from here.
    pub friction_since: DateTime<Utc>,
    pub friction: OwnerFriction,
    pub friction_baseline: FrictionBaseline,
    /// Why the friction alone makes a review due, one line each; empty when it does not.
    pub friction_due: Vec<String>,
    pub last_review: Option<LastReview>,
    pub due: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LastReview {
    pub at: DateTime<Utc>,
    pub conclusion: String,
    pub teams: Vec<TeamScore>,
    /// Reviews before 20 November 2026 stored no friction.
    pub friction: Option<OwnerFriction>,
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

/// A rate as a fraction, so comparisons never lose to rounding.
fn above(part: i64, whole: i64, base_part: i64, base_whole: i64) -> bool {
    whole > 0 && base_whole > 0 && part * base_whole > base_part * whole
}

/// The redirect-run baseline as a fraction: the last review's run rate when it had one, else the
/// owner's own redirect share this week.
fn run_baseline(friction: &OwnerFriction, last: Option<&OwnerFriction>) -> (i64, i64, bool) {
    match last.filter(|last| last.redirects >= MIN_REDIRECTS) {
        Some(last) => (last.redirect_runs, last.redirects, true),
        None => (friction.redirects, friction.owner_messages, false),
    }
}

pub fn friction_baseline(
    friction: &OwnerFriction,
    last: Option<&OwnerFriction>,
) -> FrictionBaseline {
    let (part, whole, from_last_review) = run_baseline(friction, last);
    FrictionBaseline {
        status_check_percent: STATUS_CHECK_BASELINE_PERCENT,
        redirect_run_percent: percent(part, whole),
        from_last_review,
    }
}

/// Why the owner's friction makes a review due, one line each. Each reason needs something new
/// since the last review (`new_*`), so the same week of numbers asks once.
pub fn friction_due(friction: &OwnerFriction, last: Option<&OwnerFriction>) -> Vec<String> {
    let mut reasons = Vec::new();
    // Exec judged the owner's messages in context: whether each could have been spared them is
    // the signal. The rules' counts below are the fallback for a company without those labels.
    let exec_decides = friction.judged >= EXEC_LABELS_DECIDE;
    if friction.recent_avoidable >= AVOIDABLE_IN_RECENT && friction.new_avoidable > 0 {
        reasons.push(format!(
            "{} of the owner's last {} messages could have been spared them",
            friction.recent_avoidable, friction.recent_messages
        ));
    }
    for gap in friction
        .gaps
        .iter()
        .filter(|gap| gap.times >= REPEATED_GAP && gap.new_times > 0)
    {
        reasons.push(format!(
            "the same gap came back {} times this week: {}",
            gap.times,
            gap_words(gap)
        ));
    }
    if !exec_decides
        && friction.owner_messages >= MIN_OWNER_MESSAGES
        && friction.new_status_checks > 0
        && above(
            friction.status_checks,
            friction.owner_messages,
            STATUS_CHECK_BASELINE_PERCENT,
            100,
        )
    {
        reasons.push(format!(
            "status checks are {}% of the owner's messages this week ({} of {}), above {}%",
            percent(friction.status_checks, friction.owner_messages),
            friction.status_checks,
            friction.owner_messages,
            STATUS_CHECK_BASELINE_PERCENT
        ));
    }
    let (base_part, base_whole, from_last) = run_baseline(friction, last);
    if !exec_decides
        && friction.redirects >= MIN_REDIRECTS
        && friction.new_redirect_runs > 0
        && above(
            friction.redirect_runs,
            friction.redirects,
            base_part,
            base_whole,
        )
    {
        reasons.push(format!(
            "{} of {} redirects were followed by another on the same thread ({}%), above {} ({}%)",
            friction.redirect_runs,
            friction.redirects,
            percent(friction.redirect_runs, friction.redirects),
            if from_last {
                "the rate at the last review"
            } else {
                "the owner's own redirect rate"
            },
            percent(base_part, base_whole)
        ));
    }
    if friction.new_work_reopened > 0 {
        reasons.push(format!(
            "{} sent back after it was done",
            if friction.new_work_reopened == 1 {
                "1 Work was".to_string()
            } else {
                format!("{} Work were", friction.new_work_reopened)
            }
        ));
    }
    reasons
}

pub async fn scorecard(org: &OrgIntel, now: DateTime<Utc>) -> anyhow::Result<Scorecard> {
    let last = org.latest_org_review().await?;
    let since = now - chrono::Duration::days(SCORE_WINDOW_DAYS);
    let friction_since = now - chrono::Duration::days(FRICTION_WINDOW_DAYS);
    let signals_after = last
        .as_ref()
        .map_or(since, |review| review.signals_through.max(since));
    let (teams, goals, rows, friction) = tokio::try_join!(
        org.team_scores(since),
        org.goal_minds(since),
        org.owner_signals_after(signals_after),
        org.owner_friction(friction_since, signals_after),
    )?;
    let signals = classify(&rows);
    let last_review = last.map(last_review);
    let last_friction = last_review
        .as_ref()
        .and_then(|review| review.friction.as_ref());
    let friction_due = friction_due(&friction, last_friction);
    let friction_baseline = friction_baseline(&friction, last_friction);
    let due = due(&signals, &teams) || !friction_due.is_empty();
    Ok(Scorecard {
        since,
        signals_since: signals_after,
        due,
        teams,
        goals,
        signals,
        signals_through: now,
        friction_since,
        friction,
        friction_baseline,
        friction_due,
        last_review,
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
        friction: row
            .scorecard
            .get("friction")
            .cloned()
            .and_then(|friction| serde_json::from_value(friction).ok()),
        conclusion: row.conclusion,
    }
}

/// A gap in plain words; a name this build does not know is shown as stored.
pub fn gap_words(gap: &OwnerGap) -> String {
    Gap::parse(&gap.gap).map_or_else(|| gap.gap.clone(), |known| known.plain().to_string())
}

/// One gap with the repair Exec named for it, for the review to act on.
fn gap_line(gap: &OwnerGap) -> String {
    let mut line = format!(
        "{} ({}): {} time{}",
        gap.gap,
        gap_words(gap),
        gap.times,
        if gap.times == 1 { "" } else { "s" }
    );
    if gap.new_times > 0 && gap.new_times < gap.times {
        line.push_str(&format!(", {} since the last review", gap.new_times));
    }
    if let Some(known) = &gap.could_have_known {
        line.push_str(&format!("; could have known: {known}"));
    }
    match (&gap.repair, &gap.repair_lead) {
        (Some(repair), Some(lead)) => line.push_str(&format!("; repair: {repair} (lead {lead})")),
        (Some(repair), None) => line.push_str(&format!("; repair: {repair} (no lead named)")),
        _ => line.push_str("; no repair named yet"),
    }
    line
}

/// "4.0", or "none landed" when nothing did.
fn per_landed(friction: &OwnerFriction) -> String {
    if friction.work_landed <= 0 {
        return "no Work landed".to_string();
    }
    let tenths = (friction.owner_messages * 10 + friction.work_landed / 2) / friction.work_landed;
    format!(
        "{}.{} owner messages per landed Work ({} over {})",
        tenths / 10,
        tenths % 10,
        friction.owner_messages,
        friction.work_landed
    )
}

/// The owner's friction as one paragraph: what `restless org scorecard` prints.
pub fn friction_lines(card: &Scorecard) -> String {
    let f = &card.friction;
    let mut out = format!(
        "Owner friction since {}: {} {}",
        card.friction_since.format("%-d %b"),
        f.owner_messages,
        if f.owner_messages == 1 {
            "message"
        } else {
            "messages"
        }
    );
    if f.owner_messages == 0 {
        out.push_str(".\n");
        return out;
    }
    if f.judged > 0 {
        out.push_str(&format!(
            "; Exec judged {} and {} could have been spared ({}%, {} of them partly), {} of the \
             last {}",
            f.judged,
            f.avoidable,
            percent(f.avoidable, f.judged),
            f.partly_avoidable,
            f.recent_avoidable,
            f.recent_messages
        ));
    }
    out.push_str(&format!(
        "; status checks {} ({}%, baseline {}%)",
        f.status_checks,
        percent(f.status_checks, f.owner_messages),
        card.friction_baseline.status_check_percent
    ));
    out.push_str(&format!(
        "; redirected after {} of {} outcome-only asks ({}%) and {} of {} instructions ({}%)",
        f.outcome_only_redirected,
        f.outcome_only,
        percent(f.outcome_only_redirected, f.outcome_only),
        f.instructions_redirected,
        f.instructions,
        percent(f.instructions_redirected, f.instructions)
    ));
    out.push_str(&format!(
        "; redirect runs {} of {} redirects ({}%, baseline {}%)",
        f.redirect_runs,
        f.redirects,
        percent(f.redirect_runs, f.redirects),
        card.friction_baseline.redirect_run_percent
    ));
    out.push_str(&format!("; Work reopened {}", f.work_reopened));
    out.push_str(&format!("; {}.\n", per_landed(f)));
    for gap in &f.gaps {
        out.push_str(&format!("  - gap {}\n", gap_line(gap)));
    }
    for reason in &card.friction_due {
        out.push_str(&format!("  - review due: {reason}\n"));
    }
    out
}

fn team_label(team: &TeamScore) -> String {
    match (&team.team_id, &team.lead_actor_id) {
        (Some(_), Some(lead)) => format!("\"{}\" (lead {lead})", team.team_name),
        _ => "no team (Exec and unassigned)".to_string(),
    }
}

/// One team, in a line a reviewer can act on: the rate first, then what stands out.
pub fn team_line(team: &TeamScore, before: Option<&TeamScore>) -> String {
    let mut line = if team.attempts > 0 {
        format!(
            "{}: {} of {} attempts landed ({}%)",
            team_label(team),
            team.landed,
            team.attempts,
            percent(team.landed, team.attempts)
        )
    } else {
        format!("{}: no production attempts", team_label(team))
    };
    if let Some(before) = before.filter(|before| before.attempts > 0 && team.attempts > 0) {
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
        "; {} on {} Work",
        if team.minds == 1 {
            "1 mind".to_string()
        } else {
            format!("{} minds", team.minds)
        },
        team.work_items
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
    let plural =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
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
        "none".to_string()
    } else {
        parts.join(", ")
    }
}

/// The scorecard as text: what `restless org scorecard` prints and what a due review shows.
pub fn render_numbers(card: &Scorecard) -> String {
    let mut out = format!(
        "Owner signal since {}: {}.\n",
        card.signals_since.format("%-d %b"),
        signal_sentence(&card.signals)
    );
    for example in &card.signals.examples {
        out.push_str(&format!("  - {example}\n"));
    }
    out.push_str(&friction_lines(card));
    out.push_str(&format!(
        "Team results since {}:\n",
        card.since.format("%-d %b")
    ));
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
         - Redirect runs and reopened Work mean the goal was lost or a fix did not hold. Change \
         who checks the work or how a request is confirmed, not only the one piece of Work.\n\
         {}\
         Tell the owner each change in one line with the number behind it. Then record it, or this \
         stays owed: `restless org review --conclusion '<what the numbers showed, what you \
         changed, or why nothing needed changing>'`.\n",
        render_numbers(card),
        repairs_owed(card)
    ))
}

/// What a review owes for the gaps behind avoidable messages: each repair handed to the lead who
/// owns it as Work, so the review changes the company instead of reporting on it.
fn repairs_owed(card: &Scorecard) -> String {
    if card.friction.gaps.is_empty() {
        return String::new();
    }
    String::from(
        "- The gap lines above are messages the owner could have been spared. For each gap, hand \
         its repair to the lead who owns it as Work: `restless commission --lead <lead> --title \
         '<the repair>' --outcome '<what stops this gap recurring, with the owner's words and how \
         the lead will show it held>'`, unless that Work is already open. A repair changes a \
         check, a rule or who does what, not one message. With no lead named, give it to the \
         lead closest to the work, or do it yourself if it is your own habit (a lesson, a push \
         you owe). Name each Work in the conclusion.\n",
    )
}

/// What the review stores for the next one to compare against.
pub fn stored(card: &Scorecard) -> serde_json::Value {
    serde_json::json!({
        "since": card.since,
        "teams": card.teams,
        "goals": card.goals,
        "friction": card.friction,
    })
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
            since: Utc::now() - chrono::Duration::days(14),
            signals_since: Utc::now() - chrono::Duration::days(3),
            teams,
            goals: Vec::new(),
            signals,
            signals_through: Utc::now(),
            friction_since: Utc::now() - chrono::Duration::days(7),
            friction: OwnerFriction::default(),
            friction_baseline: friction_baseline(&OwnerFriction::default(), None),
            friction_due: Vec::new(),
            last_review: None,
            due,
        }
    }

    /// A week of 20 owner messages with 4 redirects, one of them a run, nothing new since the
    /// last review.
    fn week() -> OwnerFriction {
        OwnerFriction {
            owner_messages: 20,
            status_checks: 1,
            outcome_only: 3,
            outcome_only_redirected: 1,
            instructions: 10,
            instructions_redirected: 1,
            redirects: 4,
            redirect_runs: 1,
            work_reopened: 0,
            work_landed: 5,
            ..Default::default()
        }
    }

    #[test]
    fn friction_below_its_baselines_asks_nothing() {
        // 1 status check in 20 is 5%. The run rate (25%) is above the 20% redirect share, but no
        // run happened since the last review.
        assert!(friction_due(&week(), None).is_empty());
    }

    #[test]
    fn status_checks_above_eight_percent_make_a_review_due_once() {
        let mut friction = week();
        friction.status_checks = 2; // 10% of 20
        assert!(friction_due(&friction, None).is_empty(), "nothing new");
        friction.new_status_checks = 1;
        let reasons = friction_due(&friction, None);
        assert_eq!(reasons.len(), 1, "{reasons:?}");
        assert!(
            reasons[0].contains("10% of the owner's messages"),
            "{reasons:?}"
        );
        // Too few messages for a share to mean anything.
        let few = OwnerFriction {
            owner_messages: 5,
            status_checks: 2,
            new_status_checks: 2,
            ..Default::default()
        };
        assert!(friction_due(&few, None).is_empty());
        // Exactly 8% is not above it.
        let even = OwnerFriction {
            owner_messages: 25,
            status_checks: 2,
            new_status_checks: 1,
            ..Default::default()
        };
        assert!(friction_due(&even, None).is_empty());
    }

    #[test]
    fn redirect_runs_are_judged_against_the_owners_own_baseline() {
        let mut friction = week();
        friction.new_redirect_runs = 1;
        // 1 run in 4 redirects (25%) against a 20% redirect share: redirects cluster.
        let reasons = friction_due(&friction, None);
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("the owner's own redirect rate (20%)")),
            "{reasons:?}"
        );
        // The last review saw 2 runs in 4: 25% now is better, not a reason.
        let last = OwnerFriction {
            redirects: 4,
            redirect_runs: 2,
            ..Default::default()
        };
        assert!(friction_due(&friction, Some(&last)).is_empty());
        let baseline = friction_baseline(&friction, Some(&last));
        assert_eq!(
            (baseline.redirect_run_percent, baseline.from_last_review),
            (50, true)
        );
        // Two redirects are too few to have a rate.
        friction.redirects = 2;
        assert!(friction_due(&friction, None).is_empty());
    }

    #[test]
    fn any_reopened_work_makes_a_review_due() {
        let mut friction = week();
        friction.work_reopened = 1;
        assert!(friction_due(&friction, None).is_empty(), "already reviewed");
        friction.new_work_reopened = 1;
        assert_eq!(
            friction_due(&friction, None),
            vec!["1 Work was sent back after it was done".to_string()]
        );
    }

    fn told(times: i64, new_times: i64) -> OwnerGap {
        OwnerGap {
            gap: "told".into(),
            times,
            new_times,
            repair: Some("Exec posts when Work lands, with the proof.".into()),
            repair_lead: Some("growth-lead".into()),
            could_have_known: Some("The deploy finished an hour ago.".into()),
        }
    }

    #[test]
    fn exec_labels_make_a_review_due_on_avoidable_messages_and_repeated_gaps() {
        let mut friction = week();
        friction.judged = 10;
        friction.recent_messages = 10;
        friction.recent_avoidable = 2;
        friction.new_avoidable = 2;
        assert!(
            friction_due(&friction, None).is_empty(),
            "2 of 10 is a slip"
        );
        friction.recent_avoidable = 3;
        let reasons = friction_due(&friction, None);
        assert_eq!(
            reasons,
            vec!["3 of the owner's last 10 messages could have been spared them".to_string()]
        );
        friction.new_avoidable = 0;
        assert!(friction_due(&friction, None).is_empty(), "nothing new");

        // The same gap twice, once since the last review.
        friction.gaps = vec![told(2, 1)];
        let reasons = friction_due(&friction, None);
        assert_eq!(reasons.len(), 1, "{reasons:?}");
        assert!(
            reasons[0]
                .contains("came back 2 times this week: you were not told where things stood"),
            "{reasons:?}"
        );
        friction.gaps = vec![told(2, 0)];
        assert!(friction_due(&friction, None).is_empty());
    }

    #[test]
    fn exec_labels_stand_the_rules_triggers_down_and_the_rules_remain_the_fallback() {
        let mut friction = week();
        friction.status_checks = 3; // 15% of 20
        friction.new_status_checks = 1;
        assert_eq!(friction_due(&friction, None).len(), 1, "rules decide");
        friction.judged = EXEC_LABELS_DECIDE;
        assert!(
            friction_due(&friction, None).is_empty(),
            "Exec judged them, and none was avoidable"
        );
        // Reopened Work is a fact, not a label: it still counts.
        friction.new_work_reopened = 1;
        assert_eq!(friction_due(&friction, None).len(), 1);
    }

    #[test]
    fn a_due_review_hands_each_repair_to_its_lead() {
        let mut card = card(OwnerSignals::default(), vec![team("Growth", 10, 8)]);
        card.friction = week();
        card.friction.judged = 10;
        card.friction.avoidable = 4;
        card.friction.partly_avoidable = 1;
        card.friction.recent_messages = 10;
        card.friction.recent_avoidable = 3;
        card.friction.new_avoidable = 1;
        card.friction.gaps = vec![told(3, 1)];
        card.friction_due = friction_due(&card.friction, None);
        card.due = true;
        let block = exec_block(&card).unwrap();
        assert!(
            block.contains("Exec judged 10 and 4 could have been spared (40%, 1 of them partly), 3 of the last 10"),
            "{block}"
        );
        assert!(
            block.contains("gap told (you were not told where things stood): 3 times, 1 since the last review; could have known: The deploy finished an hour ago.; repair: Exec posts when Work lands, with the proof. (lead growth-lead)"),
            "{block}"
        );
        assert!(
            block.contains("restless commission --lead <lead>"),
            "{block}"
        );
        assert!(
            block.contains("Name each Work in the conclusion"),
            "{block}"
        );
        // No gaps, no repair instructions.
        card.friction.gaps.clear();
        assert!(!exec_block(&card).unwrap().contains("restless commission"));
    }

    #[test]
    fn the_scorecard_prints_the_owners_friction() {
        let mut card = card(OwnerSignals::default(), vec![team("Growth", 10, 8)]);
        card.friction = week();
        card.friction_baseline = friction_baseline(&card.friction, None);
        let text = render_numbers(&card);
        assert!(
            text.contains("20 messages; status checks 1 (5%, baseline 8%)"),
            "{text}"
        );
        assert!(
            text.contains(
                "redirected after 1 of 3 outcome-only asks (33%) and 1 of 10 instructions (10%)"
            ),
            "{text}"
        );
        assert!(
            text.contains("redirect runs 1 of 4 redirects (25%, baseline 20%)"),
            "{text}"
        );
        assert!(
            text.contains("4.0 owner messages per landed Work (20 over 5)"),
            "{text}"
        );
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
        assert!(
            block.contains("restless org review --conclusion"),
            "{block}"
        );
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
            friction: None,
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
        assert!(
            text.contains("\"Launch Reno\": 9 minds, 3 of them critics"),
            "{text}"
        );
        assert!(!text.contains("\"Small\""), "{text}");
    }
}
