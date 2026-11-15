//! A question for the owner travels; the work does not stop (Sprint 118).
//!
//! In the owner's own small runs on 12 November an agent working alone asked a necessary
//! question 24 times out of 24, and 3 times out of 24 under an Exec and a lead: the managers'
//! briefs said "do not guess, stop and report back", so the question stopped with the lead and
//! the work sat unfinished.
//!
//! A question only the owner can answer is recorded here once, by whoever has it, with the
//! answer they are assuming meanwhile. Nobody has to relay it:
//!
//! - It comes from a worker's turn marker (`ownerQuestion`, `assumed`) at the end of an Attempt,
//!   or from a judgement an agent has held past an hour (`health_signals`).
//! - It rides Exec's next check-in as one of that check-in's three questions. No model turn is
//!   added to carry it.
//! - It stays open, and is carried again at the next check-in, until Exec has written to its
//!   lead, naming the Work, after the owner wrote back, or until it has been carried
//!   [`MAX_QUESTION_CARRIES`] times. Work that was stopped for good takes its questions with it.
//! - A held judgement stops being a question for the owner when its holder settles it for
//!   real, or when it reaches the owner as its own Inbox item. Answered `Assumed: …`, it stays
//!   one, and the assumption travels with it.
//!
//! Recording a question never changes Work. The Work carries on on the stated assumption; only
//! a step that cannot be undone, or that acts outside the company, still waits for the owner,
//! and that is the owner handoff, unchanged.
//!
//! The record is an event. No table: a question has one writer and is read back by key.

use chrono::{DateTime, Utc};
use sqlx::Row as _;
use uuid::Uuid;

use crate::checkin::{CHECKIN_EVENT, OWNER_WROTE};
use crate::{OrgIntel, Result};

/// The event a recorded question leaves.
pub const OWNER_QUESTION_EVENT: &str = "owner_question";
/// Check-ins that may carry one question before it is let go.
pub const MAX_QUESTION_CARRIES: i64 = 3;
/// Team questions one check-in carries. A check-in asks at most three; one slot stays Exec's.
pub const QUESTIONS_PER_CHECKIN: usize = 2;
const QUESTION_CHARS: usize = 400;
const ASSUMED_CHARS: usize = 300;

/// A question as it is recorded.
#[derive(Debug, Clone, Copy)]
pub struct NewOwnerQuestion<'a> {
    /// What makes it this question: `attempt:<id>` or `handoff:<id>`. Recorded once per key.
    pub key: &'a str,
    pub work_id: Uuid,
    pub asked_by: &'a str,
    pub question: &'a str,
    /// The answer the work is carrying on with meanwhile; may be empty.
    pub assumed: &'a str,
    /// `attempt` or `held_handoff`.
    pub source: &'a str,
}

/// A question still waiting for the owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerQuestion {
    pub key: String,
    pub work_id: Uuid,
    pub work_title: String,
    pub asked_by: String,
    /// The lead accountable for the Work, who gets the answer.
    pub lead: String,
    /// The lead's name as people see it.
    pub lead_name: String,
    pub question: String,
    pub assumed: String,
    pub asked_at: DateTime<Utc>,
    /// Check-ins that have carried it so far.
    pub carried: i64,
}

fn one_line(text: &str, chars: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut kept: String = text.chars().take(chars).collect();
    if text.chars().count() > chars {
        kept.push('…');
    }
    kept
}

/// The answer in `Assumed: <answer>`, the way a holder is told to settle a held judgement it
/// cannot decide.
fn stated_assumption(resolution: &str) -> Option<String> {
    const PREFIX: &str = "assumed:";
    let resolution = resolution.trim();
    let head = resolution.get(..PREFIX.len())?;
    head.eq_ignore_ascii_case(PREFIX)
        .then(|| one_line(&resolution[PREFIX.len()..], ASSUMED_CHARS))
        .filter(|assumed| !assumed.is_empty())
}

/// What a check-in is told about the team's questions, after its own instructions. Ids and one
/// command appear here because Exec reads it; what Exec writes the owner has neither.
pub fn owner_questions_block(questions: &[OwnerQuestion]) -> String {
    if questions.is_empty() {
        return String::new();
    }
    let mut block = String::from(
        "\nQuestions from the team for the owner. Each takes one of your three question slots, \
         ahead of your own. The work has not stopped: it is carrying on with the answer shown, \
         so say that too.\n",
    );
    for question in questions {
        let meanwhile = if question.assumed.is_empty() {
            "No answer is being assumed meanwhile.".to_string()
        } else {
            // A full stop, so the next line does not run on from it.
            let stop = if question.assumed.ends_with(['.', '!', '?', '…']) {
                ""
            } else {
                "."
            };
            format!("Meanwhile assuming: {}{stop}", question.assumed)
        };
        block.push_str(&format!(
            "- From {} (`{}`) on \"{}\": {} {meanwhile}\n",
            question.lead_name, question.lead, question.work_title, question.question,
        ));
    }
    block.push_str(
        "Put each to the owner in plain words with what they need to answer it in a word or \
         two. When they answer one, tell its lead in that same turn with `restless message --to \
         <the id shown> '<the Work's title>: the owner says …'`, so the work changes course if \
         the assumption was wrong. Start with the title exactly as shown: that is what settles \
         the question. If they wave one off, tell its lead that instead.",
    );
    block
}

impl OrgIntel {
    /// Record a question only the owner can answer, once per key. The Work is not touched and
    /// nobody is woken: the question rides the next check-in. Returns whether it was new. A
    /// blank question, or one about Work that no longer exists, is not recorded.
    pub async fn record_owner_question(&self, question: NewOwnerQuestion<'_>) -> Result<bool> {
        let asked = one_line(question.question, QUESTION_CHARS);
        if asked.is_empty() || question.key.trim().is_empty() {
            return Ok(false);
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
            .bind(format!("owner_question:{}", question.key))
            .execute(&mut *tx)
            .await?;
        let already: bool = sqlx::query_scalar(&format!(
            "SELECT EXISTS(SELECT 1 FROM events WHERE kind='{OWNER_QUESTION_EVENT}' \
               AND body->>'key' = $1)"
        ))
        .bind(question.key)
        .fetch_one(&mut *tx)
        .await?;
        if already {
            tx.rollback().await?;
            return Ok(false);
        }
        // The lead accountable for the Work: its owner's team lead, or its owner.
        let lead: Option<String> = sqlx::query_scalar(
            "SELECT COALESCE((SELECT t.lead_actor_id FROM actors a JOIN teams t ON t.id=a.team_id \
                               WHERE a.id=w.owner_id AND t.disbanded_at IS NULL), w.owner_id) \
             FROM work w WHERE w.id=$1",
        )
        .bind(question.work_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(lead) = lead else {
            tx.rollback().await?;
            return Ok(false);
        };
        let lead = if matches!(lead.as_str(), "owner" | "world" | "daemon") {
            "exec".to_string()
        } else {
            lead
        };
        sqlx::query(&format!(
            "INSERT INTO events (kind, actor_id, body) VALUES ('{OWNER_QUESTION_EVENT}',$1,$2)"
        ))
        .bind(question.asked_by)
        .bind(serde_json::json!({
            "key": question.key,
            "work_id": question.work_id,
            "asked_by": question.asked_by,
            "lead": lead,
            "question": asked,
            "assumed": one_line(question.assumed, ASSUMED_CHARS),
            "source": question.source,
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    /// The questions still waiting for the owner, oldest first.
    ///
    /// A question is settled once Exec has written to its lead (or to whoever asked), naming
    /// the Work by its title, after the owner wrote to Exec after a check-in that carried it:
    /// the answer has gone back down. For Work Exec leads itself, the owner's reply settles it.
    /// Told by row order, as the check-in rhythm is.
    pub async fn open_owner_questions(&self) -> Result<Vec<OwnerQuestion>> {
        let owner_wrote = OWNER_WROTE.replace("m.", "o.");
        let rows = sqlx::query(&format!(
            "SELECT q.body, q.created_at, w.title, \
                    COALESCE(NULLIF(a.display,''), q.body->>'lead') AS lead_name, carries.count AS carried, \
                    (SELECT h.resolution FROM owner_handoffs h \
                      WHERE q.body->>'key' = 'handoff:' || h.id::text AND h.state='resolved') AS answered_with \
             FROM events q \
             JOIN work w ON w.id::text = q.body->>'work_id' \
             LEFT JOIN actors a ON a.id = q.body->>'lead' \
             CROSS JOIN LATERAL ( \
               SELECT count(*) AS count FROM events c \
                WHERE c.kind='{CHECKIN_EVENT}' AND c.id > q.id \
                  AND jsonb_exists(c.body->'questions', q.body->>'key')) carries \
             WHERE q.kind='{OWNER_QUESTION_EVENT}' AND w.status <> 'abandoned' \
               AND carries.count < $1 \
               AND (q.body->>'source' <> 'held_handoff' OR EXISTS ( \
                      SELECT 1 FROM owner_handoffs h \
                       WHERE q.body->>'key' = 'handoff:' || h.id::text \
                         AND ((h.state='pending' AND h.assigned_to IS NOT NULL) \
                           OR (h.state='resolved' AND h.resolution ILIKE 'assumed:%')))) \
               AND NOT EXISTS ( \
                 SELECT 1 FROM events c \
                 CROSS JOIN LATERAL ( \
                   SELECT min(o.id) AS id FROM messages o \
                    WHERE o.to_actor='exec' AND o.id > (c.body->>'message_mark')::bigint \
                      AND {owner_wrote}) answered \
                 WHERE c.kind='{CHECKIN_EVENT}' AND c.id > q.id \
                   AND jsonb_exists(c.body->'questions', q.body->>'key') \
                   AND answered.id IS NOT NULL \
                   AND (q.body->>'lead' = 'exec' OR EXISTS ( \
                          SELECT 1 FROM messages m WHERE m.id > answered.id AND m.from_actor='exec' \
                             AND m.to_actor IN (q.body->>'lead', q.body->>'asked_by') \
                             AND position(lower(w.title) in lower(m.body)) > 0))) \
             ORDER BY q.id"
        ))
        .bind(MAX_QUESTION_CARRIES)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let body: serde_json::Value = row.get("body");
                let text = |field: &str| body[field].as_str().unwrap_or_default().to_string();
                Some(OwnerQuestion {
                    key: text("key"),
                    work_id: Uuid::parse_str(body["work_id"].as_str()?).ok()?,
                    work_title: row.get("title"),
                    asked_by: text("asked_by"),
                    lead: text("lead"),
                    lead_name: row.get("lead_name"),
                    question: text("question"),
                    // The asker's own assumption, or the one its lead answered the held
                    // judgement with.
                    assumed: Some(text("assumed"))
                        .filter(|assumed| !assumed.is_empty())
                        .or_else(|| {
                            row.get::<Option<String>, _>("answered_with")
                                .as_deref()
                                .and_then(stated_assumption)
                        })
                        .unwrap_or_default(),
                    asked_at: row.get("created_at"),
                    carried: row.get("carried"),
                })
            })
            .collect())
    }

    /// Record, as questions for the owner, the judgements agents have held past
    /// `rhythm.held_question_after()`. The handoff stays where it is; this only makes sure the
    /// question reaches the owner whatever its holder does. Returns how many were new.
    pub async fn carry_held_questions_at(
        &self,
        now: DateTime<Utc>,
        held: &[crate::HeldQuestion],
        rhythm: crate::HealthRhythm,
    ) -> Result<usize> {
        let mut recorded = 0;
        for question in held
            .iter()
            .filter(|held| now - held.since >= rhythm.held_question_after())
        {
            if self
                .record_owner_question(NewOwnerQuestion {
                    key: &format!("handoff:{}", question.handoff_id),
                    work_id: question.work_id,
                    asked_by: &question.asked_by,
                    question: &question.question,
                    assumed: "",
                    source: "held_handoff",
                })
                .await?
            {
                recorded += 1;
            }
        }
        Ok(recorded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(lead: &str, assumed: &str) -> OwnerQuestion {
        OwnerQuestion {
            key: "attempt:1".into(),
            work_id: Uuid::nil(),
            work_title: "Price the menu".into(),
            asked_by: "alpha-build".into(),
            lead: lead.into(),
            lead_name: "Aang".into(),
            question: "Should the menu show prices with or without tax?".into(),
            assumed: assumed.into(),
            asked_at: Utc::now(),
            carried: 0,
        }
    }

    #[test]
    fn the_block_gives_exec_the_question_the_assumption_and_the_way_back() {
        assert_eq!(owner_questions_block(&[]), "");
        let block =
            owner_questions_block(&[question("alpha-lead", "with tax, as on the old menu")]);
        for part in [
            "one of your three question slots",
            "The work has not stopped",
            "From Aang (`alpha-lead`) on \"Price the menu\"",
            "with or without tax?",
            "Meanwhile assuming: with tax, as on the old menu.\n",
            "restless message --to",
            "Start with the title exactly as shown",
            "in plain words",
        ] {
            assert!(block.contains(part), "missing {part:?} in {block}");
        }
        assert!(block.len() < 1_000, "{}", block.len());
        let bare = owner_questions_block(&[question("alpha-lead", "")]);
        assert!(
            bare.contains("No answer is being assumed meanwhile."),
            "{bare}"
        );
    }

    #[test]
    fn a_held_judgement_answered_on_an_assumption_carries_it() {
        assert_eq!(
            stated_assumption(
                " Assumed: the printer near the shop,\n because the proof can be checked. "
            ),
            Some("the printer near the shop, because the proof can be checked.".into())
        );
        assert_eq!(
            stated_assumption("ASSUMED: with tax"),
            Some("with tax".into())
        );
        for settled in ["Use the online printer.", "Assumed:", "", "é"] {
            assert_eq!(stated_assumption(settled), None, "{settled:?}");
        }
    }

    #[test]
    fn a_long_question_is_cut_not_dropped() {
        let long = "word ".repeat(200);
        let kept = one_line(&long, QUESTION_CHARS);
        assert_eq!(kept.chars().count(), QUESTION_CHARS + 1);
        assert!(kept.ends_with('…'));
        assert_eq!(one_line("  two\n lines  ", 50), "two lines");
    }
}
