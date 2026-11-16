//! The attention report (Sprint 120): for a stretch of time, the results the owner accepted and
//! every time the owner had to step in, each put under a kind the records themselves support.
//!
//! The product's stated target is useful output per unit of owner attention
//! (`owner-cockpit` §16.2: "owner interventions per accepted outcome"), and it had never been
//! counted. This counts it. It is a read, not a table: every line comes from rows that already
//! exist (owner messages, settled Inbox items, goal and Work events, lessons), and no model is
//! asked anything.
//!
//! `evaluation-dogfood` §6.3 names eight kinds of intervention. The records cannot tell all
//! eight apart, so this does not pretend to. What they can tell:
//!
//! | Kind here | From | §6.3 kind it stands for |
//! | --- | --- | --- |
//! | accepted a result | an outcome review the owner accepted | product/taste judgment |
//! | sent work back or corrected it | a review declined, done Work sent back, a message that became a lesson or an observation, a rule the owner wrote | product/taste judgment, or correcting a false claim: not told apart |
//! | answered the company | a reply quoting a question, the reply after a question, the first reply after a check-in, daily brief or direction session, an Inbox decision | mandate/strategy judgment, mostly; not told apart from taste |
//! | approved or refused an outside action | an Authority approval, a payment confirmation | capital or authority decision |
//! | did a step only a person can do | identity, human verification, sign-in code, legal confirmation | external relationship/human-only action |
//! | gave new direction | a goal the owner set, changed, closed or reopened | mandate/strategy judgment |
//! | stepped into the work | Work the owner paused, stopped, restarted or reassigned | ordinary task decomposition, or repairing a harness failure: not told apart |
//! | chased | a short message such as "progress?" or "try again" that answers nothing | agent chasing/status request |
//! | not told by the records | any other message the owner wrote unprompted | could be direction, task decomposition, a repair or a correction |
//!
//! "Chased" is shown apart from the total because the spec calls it a product failure. A
//! message is never guessed into a kind from its words, with that one exception: the closed
//! list of nudges the caller supplies (`restless_engine::org_review::is_nudge`).
//!
//! An intervention is one thing the owner did. A decision the owner gave in chat is one
//! intervention, not two: the Inbox item it settled names the message, and the message is then
//! not counted again. Accepting a result is itself counted as stepping in, so the number for
//! each accepted result is never below one.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row as _;
use uuid::Uuid;

use crate::checkin::OWNER_WROTE;
use crate::{OrgIntel, Result};

/// What kind of stepping in the records show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionKind {
    AcceptedResult,
    SentBack,
    Answered,
    Approved,
    HumanStep,
    Direction,
    Steered,
    Chased,
    Unclassified,
}

impl AttentionKind {
    pub const ALL: [Self; 9] = [
        Self::AcceptedResult,
        Self::SentBack,
        Self::Answered,
        Self::Approved,
        Self::HumanStep,
        Self::Direction,
        Self::Steered,
        Self::Chased,
        Self::Unclassified,
    ];

    /// As a person reads it.
    pub fn label(self) -> &'static str {
        match self {
            Self::AcceptedResult => "Accepted a result",
            Self::SentBack => "Sent work back or corrected it",
            Self::Answered => "Answered the company",
            Self::Approved => "Approved or refused an outside action",
            Self::HumanStep => "Did a step only a person can do",
            Self::Direction => "Gave new direction",
            Self::Steered => "Stepped into the work",
            Self::Chased => "Chased for status",
            Self::Unclassified => "Not told by the records",
        }
    }
}

/// One thing the owner did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Intervention {
    pub at: DateTime<Utc>,
    pub kind: AttentionKind,
    pub kind_label: &'static str,
    /// What it was, in a few words: the owner's own, or the title of what they settled.
    pub what: String,
    /// Where the record is: `message`, `inbox`, `approval`, `goal`, `work` or `rule`.
    pub source: &'static str,
}

impl Intervention {
    pub fn new(at: DateTime<Utc>, kind: AttentionKind, what: &str, source: &'static str) -> Self {
        Self {
            at,
            kind,
            kind_label: kind.label(),
            what: excerpt(what),
            source,
        }
    }
}

/// A result the owner accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AcceptedResult {
    pub work_id: Uuid,
    pub title: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KindCount {
    pub kind: AttentionKind,
    pub label: &'static str,
    pub count: usize,
}

/// The report for one stretch of time, `from` included and `to` not.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AttentionReport {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub accepted_results: Vec<AcceptedResult>,
    pub interventions: Vec<Intervention>,
    /// Every kind, in a fixed order, zero included.
    pub by_kind: Vec<KindCount>,
    pub total: usize,
    pub chased: usize,
    pub unclassified: usize,
    /// Times the owner stepped in for each result they accepted. `None` when none was accepted:
    /// nothing is divided by zero and nothing is estimated.
    pub per_accepted_result: Option<f64>,
    pub chased_per_accepted_result: Option<f64>,
}

/// What the rows say about one message the owner wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnerMessageFacts {
    /// It quotes the question it answers ("> Exec: …"), as a tapped or typed answer does.
    pub quotes_question: bool,
    /// The message before it in the conversation was an agent's and asked the owner something.
    pub after_question: bool,
    /// It is the owner's first message after a check-in, daily brief or direction session.
    pub after_wake: bool,
    /// A lesson or an observation names it as where it came from.
    pub became_correction: bool,
    /// Exec changed an Inbox item on these words.
    pub settled_inbox_item: bool,
    /// It is on the closed list of nudges ("progress?", "try again").
    pub nudge: bool,
}

/// The kind one owner message is, from its facts alone. An answer is never a chase, whatever
/// its words: "continue" after "shall I continue?" is an answer.
pub fn classify_message(facts: &OwnerMessageFacts) -> AttentionKind {
    if facts.became_correction {
        AttentionKind::SentBack
    } else if facts.settled_inbox_item
        || facts.quotes_question
        || facts.after_question
        || facts.after_wake
    {
        AttentionKind::Answered
    } else if facts.nudge {
        AttentionKind::Chased
    } else {
        AttentionKind::Unclassified
    }
}

/// The kind a settled Inbox item is. `review` is an outcome review; `accepted` is false when the
/// owner declined or asked for changes.
pub fn classify_handoff(category: &str, review: bool, accepted: bool) -> AttentionKind {
    match category {
        "owner_judgement" if review && accepted => AttentionKind::AcceptedResult,
        "owner_judgement" if review => AttentionKind::SentBack,
        "owner_judgement" => AttentionKind::Answered,
        "payment_confirmation" => AttentionKind::Approved,
        _ => AttentionKind::HumanStep,
    }
}

/// What a person sees of a message: no hidden markers, no quoted question, and not the note
/// the cockpit adds after their words about the page they were on. On one line.
fn excerpt(text: &str) -> String {
    let typed = ["\n\n[Owner cockpit context]\n", "\n\n[Restless "]
        .iter()
        .filter_map(|block| text.find(block))
        .min()
        .map_or(text, |end| &text[..end]);
    let mut visible = String::new();
    let mut rest = typed;
    while let Some(start) = rest.find("<!--") {
        visible.push_str(&rest[..start]);
        rest = rest[start..]
            .find("-->")
            .map_or("", |end| &rest[start + end + 3..]);
    }
    visible.push_str(rest);
    let line = visible
        .lines()
        .filter(|line| !line.trim_start().starts_with('>'))
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ");
    let mut kept: String = line.chars().take(120).collect();
    if line.chars().count() > 120 {
        kept.push('…');
    }
    kept
}

fn ratio(part: usize, whole: usize) -> Option<f64> {
    (whole > 0).then(|| (part as f64 / whole as f64 * 100.0).round() / 100.0)
}

impl AttentionReport {
    /// Count what was found. Interventions are put in time order.
    pub fn build(
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        accepted_results: Vec<AcceptedResult>,
        mut interventions: Vec<Intervention>,
    ) -> Self {
        interventions.sort_by_key(|intervention| intervention.at);
        let count = |kind| {
            interventions
                .iter()
                .filter(|intervention| intervention.kind == kind)
                .count()
        };
        let by_kind = AttentionKind::ALL
            .into_iter()
            .map(|kind| KindCount {
                kind,
                label: kind.label(),
                count: count(kind),
            })
            .collect();
        let (total, chased) = (interventions.len(), count(AttentionKind::Chased));
        Self {
            from,
            to,
            per_accepted_result: ratio(total, accepted_results.len()),
            chased_per_accepted_result: ratio(chased, accepted_results.len()),
            unclassified: count(AttentionKind::Unclassified),
            total,
            chased,
            by_kind,
            accepted_results,
            interventions,
        }
    }

    /// The report as plain lines, for a terminal or a file.
    pub fn summary(&self) -> String {
        let when = |at: &DateTime<Utc>| at.format("%-d %b %H:%M").to_string();
        let times = |n: usize| {
            if n == 1 {
                "1 time".to_string()
            } else {
                format!("{n} times")
            }
        };
        let mut out = format!(
            "Owner attention, {} to {} (UTC)\n\nResults the owner accepted: {}\n",
            when(&self.from),
            when(&self.to),
            self.accepted_results.len()
        );
        out.push_str(&format!("The owner stepped in: {}\n", times(self.total)));
        match self.per_accepted_result {
            Some(per) => out.push_str(&format!(
                "For each accepted result the owner stepped in: {per:.2} times\n"
            )),
            None => out.push_str(
                "No result was accepted in this time, so there is no number for each result.\n",
            ),
        }
        out.push_str(&format!(
            "Of those, chasing for status: {}{}. The company should never need this.\n",
            times(self.chased),
            self.chased_per_accepted_result
                .map(|per| format!(" ({per:.2} for each accepted result)"))
                .unwrap_or_default()
        ));
        out.push_str("\nBy kind:\n");
        for row in &self.by_kind {
            out.push_str(&format!("  {}: {}\n", row.label, row.count));
        }
        if !self.accepted_results.is_empty() {
            out.push_str("\nAccepted:\n");
            for result in &self.accepted_results {
                out.push_str(&format!("  {}  {}\n", when(&result.at), result.title));
            }
        }
        if !self.interventions.is_empty() {
            out.push_str("\nEach time, oldest first:\n");
            for step in &self.interventions {
                out.push_str(&format!(
                    "  {}  {}: {}\n",
                    when(&step.at),
                    step.kind_label,
                    if step.what.is_empty() {
                        "(no words)"
                    } else {
                        &step.what
                    }
                ));
            }
        }
        out
    }
}

/// The message a chat answer names: "The owner, in conversation (message 12): …".
fn chat_message_id(resolution: &str) -> Option<i64> {
    const MARK: &str = "in conversation (message ";
    let rest = &resolution[resolution.find(MARK)? + MARK.len()..];
    rest[..rest.find(')')?].trim().parse().ok()
}

impl OrgIntel {
    /// The attention report for `from..to`.
    ///
    /// `is_nudge` is the closed list of chasing messages. `other_planes` are decisions the owner
    /// made that this company's own database does not hold (Authority approvals); they are
    /// counted as given, when they fall inside the range.
    pub async fn attention_report(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        is_nudge: fn(&str) -> bool,
        other_planes: Vec<Intervention>,
    ) -> Result<AttentionReport> {
        let mut interventions: Vec<Intervention> = other_planes
            .into_iter()
            .filter(|step| step.at >= from && step.at < to)
            .collect();
        let mut accepted = Vec::new();
        // Messages a settled Inbox item already stands for.
        let mut counted_messages = std::collections::HashSet::new();

        // Inbox items the owner settled. One the owner answered in chat names the message.
        let handoffs = sqlx::query(
            "SELECT h.work_id, w.title AS work_title, h.category::text AS category, \
                    (COALESCE(h.owner_brief->>'kind','')='outcome_review' \
                       OR (w.owner_review_required AND EXISTS (SELECT 1 FROM work_attempts a \
                             WHERE a.id=h.attempt_id AND a.state='produced'))) AS review, \
                    COALESCE(NULLIF(h.owner_brief->>'headline',''), h.requested_action) AS title, \
                    (h.state='resolved') AS accepted, h.resolution, h.resolved_at \
             FROM owner_handoffs h JOIN work w ON w.id=h.work_id \
             WHERE h.state IN ('resolved','declined') AND h.resolved_at >= $1 AND h.resolved_at < $2 \
               AND (h.assigned_to IS NULL OR h.resolution LIKE '%The owner, in conversation (message %') \
             ORDER BY h.resolved_at, h.id",
        )
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        for row in handoffs {
            let resolution: String = row.get("resolution");
            if let Some(message_id) = chat_message_id(&resolution) {
                counted_messages.insert(message_id);
            }
            let (review, was_accepted): (bool, bool) = (row.get("review"), row.get("accepted"));
            let kind = classify_handoff(
                row.get::<String, _>("category").as_str(),
                review,
                was_accepted,
            );
            let at: DateTime<Utc> = row.get("resolved_at");
            let work_title: String = row.get("work_title");
            if kind == AttentionKind::AcceptedResult {
                accepted.push(AcceptedResult {
                    work_id: row.get("work_id"),
                    title: work_title.clone(),
                    at,
                });
            }
            let title: String = row.get("title");
            let what = if review { work_title } else { title };
            interventions.push(Intervention::new(at, kind, &what, "inbox"));
        }

        // Messages the owner wrote. A review or a send-back also leaves messages in the owner's
        // name (the feedback handed to the Work's owner and its lead); those are the decision
        // already counted, not something more the owner typed, so they are left out. The wake
        // test is the one the team review uses: the owner's first message after Exec spoke
        // following a check-in, daily brief or direction session.
        let messages = sqlx::query(&format!(
            "SELECT m.id, m.created_at, m.body, \
                    (prev.from_actor IS DISTINCT FROM m.from_actor \
                       AND position('\"ownerNeed\"' in COALESCE(prev.body,'')) > 0) AS after_question, \
                    EXISTS (SELECT 1 FROM events wake \
                       JOIN messages said ON said.from_actor='exec' \
                         AND (said.to_actor IS NULL OR said.to_actor=m.from_actor) \
                         AND said.id > (wake.body->>'message_mark')::bigint AND said.id < m.id \
                       WHERE wake.kind IN ('exec_checkin','exec_daily_review','exec_direction_session') \
                         AND NOT EXISTS (SELECT 1 FROM messages earlier \
                           WHERE earlier.from_actor=m.from_actor AND earlier.id < m.id \
                             AND earlier.id > (wake.body->>'message_mark')::bigint)) AS after_wake, \
                    (EXISTS (SELECT 1 FROM company_lessons l \
                        WHERE l.source_message_id=m.id OR l.last_recurrence_message_id=m.id) \
                      OR EXISTS (SELECT 1 FROM events o WHERE o.kind='work_observation' \
                        AND o.body->>'source_message_id'=m.id::text)) AS became_correction, \
                    EXISTS (SELECT 1 FROM exec_inbox_actions a WHERE a.source_message_id=m.id \
                        AND a.state IN ('pending','applied')) AS settled_inbox_item \
             FROM messages m \
             LEFT JOIN LATERAL (SELECT p.from_actor, p.body FROM messages p \
                 WHERE p.room_id=m.room_id AND p.id < m.id AND p.deleted_at IS NULL \
                 ORDER BY p.id DESC LIMIT 1) prev ON true \
             WHERE {OWNER_WROTE} AND m.deleted_at IS NULL \
               AND m.created_at >= $1 AND m.created_at < $2 \
               AND NOT EXISTS (SELECT 1 FROM work_feedback f WHERE f.message_id=m.id) \
               AND m.body NOT LIKE 'Review rejected Work %' \
             ORDER BY m.id"
        ))
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        for row in messages {
            if counted_messages.contains(&row.get::<i64, _>("id")) {
                continue;
            }
            let body: String = row.get("body");
            let said = excerpt(&body);
            let facts = OwnerMessageFacts {
                quotes_question: body.trim_start().starts_with("> "),
                after_question: row.get("after_question"),
                after_wake: row.get("after_wake"),
                became_correction: row.get("became_correction"),
                settled_inbox_item: row.get("settled_inbox_item"),
                nudge: is_nudge(&said),
            };
            interventions.push(Intervention::new(
                row.get("created_at"),
                classify_message(&facts),
                &said,
                "message",
            ));
        }

        // What the owner did to goals and Work directly, and rules the owner wrote themselves.
        let owner_did = OWNER_WROTE.replace("m.from_actor", "e.actor_id");
        let events = sqlx::query(&format!(
            "SELECT e.kind, e.created_at, \
                    COALESCE(e.body->>'title', e.body->>'rule', \
                      (SELECT g.title FROM goals g WHERE g.id::text=e.body->>'goal_id'), \
                      (SELECT w.title FROM work w WHERE w.id::text=e.body->>'work_id'), '') AS title \
             FROM events e \
             WHERE e.kind IN ('goal_created','goal_updated','goal_closed','goal_reopened', \
                              'work_reopened','work_paused','work_abandoned','work_restored', \
                              'work_reassigned','lesson_recorded') \
               AND {owner_did} AND e.created_at >= $1 AND e.created_at < $2 \
             ORDER BY e.id"
        ))
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        for row in events {
            let kind: String = row.get("kind");
            let title: String = row.get("title");
            let (kind, what, source) = match kind.as_str() {
                "goal_created" => (
                    AttentionKind::Direction,
                    format!("Set a goal: {title}"),
                    "goal",
                ),
                "goal_updated" => (
                    AttentionKind::Direction,
                    format!("Changed a goal: {title}"),
                    "goal",
                ),
                "goal_closed" => (
                    AttentionKind::Direction,
                    format!("Closed a goal: {title}"),
                    "goal",
                ),
                "goal_reopened" => (
                    AttentionKind::Direction,
                    format!("Reopened a goal: {title}"),
                    "goal",
                ),
                "work_reopened" => (
                    AttentionKind::SentBack,
                    format!("Sent back: {title}"),
                    "work",
                ),
                "lesson_recorded" => (
                    AttentionKind::SentBack,
                    format!("Wrote a rule: {title}"),
                    "rule",
                ),
                "work_paused" => (AttentionKind::Steered, format!("Paused: {title}"), "work"),
                "work_abandoned" => (AttentionKind::Steered, format!("Stopped: {title}"), "work"),
                "work_restored" => (
                    AttentionKind::Steered,
                    format!("Restarted: {title}"),
                    "work",
                ),
                _ => (
                    AttentionKind::Steered,
                    format!("Reassigned: {title}"),
                    "work",
                ),
            };
            interventions.push(Intervention::new(
                row.get("created_at"),
                kind,
                &what,
                source,
            ));
        }

        Ok(AttentionReport::build(from, to, accepted, interventions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn step(minute: i64, kind: AttentionKind) -> Intervention {
        Intervention::new(
            DateTime::<Utc>::UNIX_EPOCH + Duration::minutes(minute),
            kind,
            "x",
            "message",
        )
    }

    fn result(minute: i64) -> AcceptedResult {
        AcceptedResult {
            work_id: Uuid::from_u128(minute as u128),
            title: "Menu".into(),
            at: DateTime::<Utc>::UNIX_EPOCH + Duration::minutes(minute),
        }
    }

    #[test]
    fn a_message_is_a_kind_only_when_a_record_says_so() {
        let kind = |facts: OwnerMessageFacts| classify_message(&facts);
        let none = OwnerMessageFacts::default;
        assert_eq!(kind(none()), AttentionKind::Unclassified);
        assert_eq!(
            kind(OwnerMessageFacts {
                nudge: true,
                ..none()
            }),
            AttentionKind::Chased
        );
        // "continue" is on the nudge list, but said in answer it is an answer.
        for answered in [
            OwnerMessageFacts {
                nudge: true,
                quotes_question: true,
                ..none()
            },
            OwnerMessageFacts {
                nudge: true,
                after_question: true,
                ..none()
            },
            OwnerMessageFacts {
                nudge: true,
                after_wake: true,
                ..none()
            },
            OwnerMessageFacts {
                settled_inbox_item: true,
                ..none()
            },
        ] {
            assert_eq!(kind(answered), AttentionKind::Answered);
        }
        // A reply that became a lesson was a correction, even though it answered a question.
        assert_eq!(
            kind(OwnerMessageFacts {
                became_correction: true,
                after_question: true,
                ..none()
            }),
            AttentionKind::SentBack
        );
    }

    #[test]
    fn an_inbox_item_is_classified_by_what_it_asked_for() {
        assert_eq!(
            classify_handoff("owner_judgement", true, true),
            AttentionKind::AcceptedResult
        );
        assert_eq!(
            classify_handoff("owner_judgement", true, false),
            AttentionKind::SentBack
        );
        assert_eq!(
            classify_handoff("owner_judgement", false, true),
            AttentionKind::Answered
        );
        assert_eq!(
            classify_handoff("owner_judgement", false, false),
            AttentionKind::Answered
        );
        assert_eq!(
            classify_handoff("payment_confirmation", false, true),
            AttentionKind::Approved
        );
        for human in ["identity", "captcha", "mfa", "legal_attestation"] {
            assert_eq!(
                classify_handoff(human, false, true),
                AttentionKind::HumanStep
            );
        }
    }

    #[test]
    fn the_number_is_interventions_for_each_accepted_result_with_chasing_apart() {
        let (from, to) = (
            DateTime::<Utc>::UNIX_EPOCH,
            DateTime::<Utc>::UNIX_EPOCH + Duration::days(3),
        );
        let report = AttentionReport::build(
            from,
            to,
            vec![result(50), result(90)],
            vec![
                step(90, AttentionKind::AcceptedResult),
                step(10, AttentionKind::Chased),
                step(50, AttentionKind::AcceptedResult),
                step(20, AttentionKind::Unclassified),
                step(30, AttentionKind::Chased),
                step(40, AttentionKind::Answered),
                step(60, AttentionKind::Chased),
            ],
        );
        assert_eq!(
            (report.total, report.chased, report.unclassified),
            (7, 3, 1)
        );
        assert_eq!(report.per_accepted_result, Some(3.5));
        assert_eq!(report.chased_per_accepted_result, Some(1.5));
        // Oldest first, and every kind is listed, zero included.
        assert_eq!(report.interventions[0].kind, AttentionKind::Chased);
        assert_eq!(report.interventions[6].kind, AttentionKind::AcceptedResult);
        assert_eq!(report.by_kind.len(), AttentionKind::ALL.len());
        assert_eq!(report.by_kind.iter().map(|row| row.count).sum::<usize>(), 7);
        let summary = report.summary();
        for part in [
            "Results the owner accepted: 2",
            "The owner stepped in: 7 times",
            "For each accepted result the owner stepped in: 3.50 times",
            "chasing for status: 3 times (1.50 for each accepted result)",
            "Not told by the records: 1",
            "Approved or refused an outside action: 0",
        ] {
            assert!(summary.contains(part), "missing {part:?} in\n{summary}");
        }
    }

    #[test]
    fn with_no_accepted_result_there_is_no_number_and_nothing_is_divided() {
        let from = DateTime::<Utc>::UNIX_EPOCH;
        let report = AttentionReport::build(
            from,
            from + Duration::days(1),
            Vec::new(),
            vec![step(1, AttentionKind::Chased)],
        );
        assert_eq!(report.per_accepted_result, None);
        assert_eq!(report.chased_per_accepted_result, None);
        let summary = report.summary();
        assert!(
            summary.contains("No result was accepted in this time"),
            "{summary}"
        );
        assert!(summary.contains("chasing for status: 1 time."), "{summary}");
        let quiet = AttentionReport::build(from, from + Duration::days(1), Vec::new(), Vec::new());
        assert_eq!((quiet.total, quiet.per_accepted_result), (0, None));
    }

    #[test]
    fn what_is_shown_of_a_message_has_no_markers_and_no_quoted_question() {
        assert_eq!(
            excerpt(
                "> Exec: Local firms only?\n\nLocal only\n<!--restless-intent:{\"kind\":\"x\"}-->"
            ),
            "Local only"
        );
        assert_eq!(excerpt("  progress?  "), "progress?");
        // As the cockpit sends it: the words, then the page the owner was on.
        assert_eq!(
            excerpt("progress?\n\n[Owner cockpit context]\n/cafe/work\n\n<!--restless-context:{\"path\":\"/cafe/work\"}-->"),
            "progress?"
        );
        assert_eq!(excerpt(&"word ".repeat(60)).chars().count(), 121);
        assert_eq!(
            chat_message_id("Fine.\n\nThe owner, in conversation (message 1513): go"),
            Some(1513)
        );
        assert_eq!(chat_message_id("Owner accepted the prepared outcome"), None);
    }
}
