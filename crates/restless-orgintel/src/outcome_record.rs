//! The outcome record (Sprint 119): for one piece of Work, what was expected, what happened,
//! what it cost and the verdict. Nothing recorded this before, so nothing could be learned
//! from how Work ended.
//!
//! It is a read, not a table. Every value is derived from rows that already exist:
//!
//! - **Expected**: the Work's outcome text, the artifact it named, and what it rests on (the
//!   `Assumptions:` section Sprint 117 added to the outcome).
//! - **Happened**: how the Work ended (its resolution), how many Attempts it took, how many
//!   outputs it linked and how many times it was sent back.
//! - **Verdict**: accepted by the owner, accepted by its lead, finished and not reviewed, sent
//!   back, failed, stopped, or still in progress; read from the Work's status, its review
//!   handoffs and its latest Attempt.
//! - **Cost**: the spend ledger lives in the daemon, not here, so the caller supplies it
//!   ([`OutcomeRecord::with_cost`]). With no ledger entry for the Work it is "unknown"; it is
//!   never estimated.
//!
//! Nothing is stored because nothing here cannot be derived. Readable without a model.

use super::*;
use crate::assumptions::read_assumptions;
use crate::health_signals::outcome_body;

/// How many finished pieces of Work one listing returns.
pub const OUTCOME_RECORDS_LISTED: i64 = 30;

/// How a piece of Work ended, or where it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeVerdict {
    AcceptedByOwner,
    AcceptedByLead,
    /// Completed, and nobody was asked to review it.
    Finished,
    SentBack,
    Failed,
    /// Stopped for good.
    Abandoned,
    InProgress,
}

impl OutcomeVerdict {
    /// As the owner reads it.
    pub fn label(self) -> &'static str {
        match self {
            Self::AcceptedByOwner => "Accepted by the owner",
            Self::AcceptedByLead => "Accepted by its lead",
            Self::Finished => "Finished, not reviewed",
            Self::SentBack => "Sent back for changes",
            Self::Failed => "Failed",
            Self::Abandoned => "Stopped",
            Self::InProgress => "In progress",
        }
    }
}

/// The rows one record is derived from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeFacts {
    pub work_id: Uuid,
    pub title: String,
    pub outcome: String,
    pub expected_artifact: String,
    pub status: WorkStatus,
    pub resolution: String,
    pub updated_at: DateTime<Utc>,
    /// Attempts that ran, superseded ones aside.
    pub attempts: i64,
    /// How the latest finished Attempt ended.
    pub last_attempt: Option<WorkAttemptState>,
    /// Outputs linked and still current.
    pub outputs: i64,
    /// Times a reviewer, a lead or the owner sent it back.
    pub sent_back: i64,
    /// Who accepted the outcome when it was reviewed: `Some(true)` the owner, `Some(false)` an
    /// agent (its lead).
    pub accepted_by_owner: Option<bool>,
}

/// One Work's outcome record.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct OutcomeRecord {
    pub work_id: Uuid,
    pub title: String,
    /// What it was meant to produce.
    pub expected: String,
    pub expected_artifact: String,
    /// What it was commissioned on, as written then.
    pub assumptions: Vec<String>,
    pub least_sure: Option<String>,
    /// How it ended, in the words recorded then.
    pub happened: String,
    pub attempts: i64,
    pub outputs: i64,
    pub sent_back: i64,
    pub verdict: OutcomeVerdict,
    pub verdict_label: &'static str,
    /// Metered model spend attributed to this Work; `None` when the ledger holds nothing for it.
    pub cost_usd: Option<f64>,
    pub cost_label: String,
    pub at: DateTime<Utc>,
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl OutcomeRecord {
    /// The record with what the spend ledger holds for this Work. `None` stays "unknown".
    pub fn with_cost(mut self, cost_usd: Option<f64>) -> Self {
        self.cost_label = cost_label(cost_usd);
        self.cost_usd = cost_usd;
        self
    }

    /// The four values as labelled lines, for a terminal or a page.
    pub fn lines(&self) -> Vec<(&'static str, String)> {
        let mut expected = self.expected.clone();
        if !self.expected_artifact.trim().is_empty() {
            expected.push_str(&format!(" (output: {})", self.expected_artifact.trim()));
        }
        if !self.assumptions.is_empty() {
            expected.push_str(&format!(" Assumed: {}", self.assumptions.join(" ")));
        }
        let mut happened = if self.happened.is_empty() {
            "Nothing recorded yet.".to_string()
        } else {
            self.happened.clone()
        };
        let count =
            |n: i64, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        let mut facts = vec![count(self.attempts, "attempt", "attempts")];
        if self.outputs > 0 {
            facts.push(count(self.outputs, "output", "outputs"));
        }
        if self.sent_back > 0 {
            facts.push(format!(
                "sent back {}",
                if self.sent_back == 1 {
                    "once".to_string()
                } else {
                    format!("{} times", self.sent_back)
                }
            ));
        }
        happened.push_str(&format!(" ({})", facts.join(", ")));
        vec![
            ("Expected", expected),
            ("What happened", happened),
            ("Cost", self.cost_label.clone()),
            ("Verdict", self.verdict_label.to_string()),
        ]
    }
}

/// "$1.25", or "unknown" when the ledger holds nothing for the Work.
pub fn cost_label(cost_usd: Option<f64>) -> String {
    match cost_usd {
        Some(usd) => format!("${usd:.2}"),
        None => "unknown".to_string(),
    }
}

/// The verdict the rows show.
pub fn outcome_verdict(facts: &OutcomeFacts) -> OutcomeVerdict {
    match facts.status {
        WorkStatus::Abandoned => OutcomeVerdict::Abandoned,
        WorkStatus::Completed => match facts.accepted_by_owner {
            Some(true) => OutcomeVerdict::AcceptedByOwner,
            Some(false) => OutcomeVerdict::AcceptedByLead,
            None => OutcomeVerdict::Finished,
        },
        _ if facts.resolution.starts_with("changes requested by ") => OutcomeVerdict::SentBack,
        _ if facts.last_attempt == Some(WorkAttemptState::Failed) => OutcomeVerdict::Failed,
        _ => OutcomeVerdict::InProgress,
    }
}

/// Derive the record. Cost is "unknown" until the caller supplies it.
pub fn derive_outcome_record(facts: &OutcomeFacts) -> OutcomeRecord {
    let rests_on = read_assumptions(&facts.outcome);
    let verdict = outcome_verdict(facts);
    OutcomeRecord {
        work_id: facts.work_id,
        title: facts.title.clone(),
        expected: one_line(outcome_body(&facts.outcome)),
        expected_artifact: facts.expected_artifact.clone(),
        assumptions: rests_on.assumptions,
        least_sure: rests_on.least_sure,
        happened: one_line(&facts.resolution),
        attempts: facts.attempts,
        outputs: facts.outputs,
        sent_back: facts.sent_back,
        verdict,
        verdict_label: verdict.label(),
        cost_usd: None,
        cost_label: cost_label(None),
        at: facts.updated_at,
    }
}

const OUTCOME_FACTS: &str = "SELECT w.id, w.title, w.outcome, w.expected_artifact, w.status, \
        w.resolution, w.updated_at, \
        (SELECT count(*) FROM work_attempts a WHERE a.work_id=w.id AND a.state <> 'superseded') AS attempts, \
        (SELECT a.state FROM work_attempts a WHERE a.work_id=w.id AND a.finished_at IS NOT NULL \
            AND a.state <> 'superseded' ORDER BY a.finished_at DESC LIMIT 1) AS last_attempt, \
        (SELECT count(*) FROM artifact_refs r WHERE r.work_id=w.id AND r.state='available') AS outputs, \
        (SELECT count(*) FROM owner_handoffs h WHERE h.work_id=w.id AND h.state='declined') \
          + (SELECT count(*) FROM work_edges e JOIN work_attempts a ON a.work_id=e.from_work_id \
              WHERE e.to_work_id=w.id AND e.kind='revises' AND a.state='changes_requested') \
          + (SELECT count(*) FROM events e WHERE e.kind='work_reopened' \
              AND e.body->>'work_id'=w.id::text) AS sent_back, \
        (SELECT h.assigned_to IS NULL FROM owner_handoffs h WHERE h.work_id=w.id \
            AND h.state='resolved' AND COALESCE(h.owner_brief->>'kind','')='outcome_review' \
          ORDER BY h.resolved_at DESC NULLS LAST LIMIT 1) AS accepted_by_owner \
     FROM work w";

fn facts_from(row: sqlx::postgres::PgRow) -> OutcomeFacts {
    OutcomeFacts {
        work_id: row.get("id"),
        title: row.get("title"),
        outcome: row.get("outcome"),
        expected_artifact: row.get("expected_artifact"),
        status: row.get("status"),
        resolution: row.get("resolution"),
        updated_at: row.get("updated_at"),
        attempts: row.get("attempts"),
        last_attempt: row.get("last_attempt"),
        outputs: row.get("outputs"),
        sent_back: row.get("sent_back"),
        accepted_by_owner: row.get("accepted_by_owner"),
    }
}

impl OrgIntel {
    /// One Work's outcome record, whatever state it is in. Cost is "unknown" until the caller
    /// supplies it from the spend ledger.
    pub async fn outcome_record(&self, work_id: Uuid) -> Result<OutcomeRecord> {
        let row = sqlx::query(&format!("{OUTCOME_FACTS} WHERE w.id=$1"))
            .bind(work_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| OrgIntelError::InvalidWork("no Work with that id".into()))?;
        Ok(derive_outcome_record(&facts_from(row)))
    }

    /// The records of the Work that ended most recently (done or stopped), newest first.
    pub async fn outcome_records(&self, limit: i64) -> Result<Vec<OutcomeRecord>> {
        Ok(sqlx::query(&format!(
            "{OUTCOME_FACTS} WHERE w.status IN ('completed','abandoned') \
             ORDER BY w.updated_at DESC, w.id LIMIT $1"
        ))
        .bind(limit.clamp(1, 200))
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| derive_outcome_record(&facts_from(row)))
        .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(status: WorkStatus, resolution: &str) -> OutcomeFacts {
        OutcomeFacts {
            work_id: Uuid::from_u128(7),
            title: "Price the new menu".into(),
            outcome: "Every dish priced,\nwith tax shown.\nAssumptions:\n- Prices include GST.\n- The printer needs A4.\nLeast sure of: whether lunch is priced apart."
                .into(),
            expected_artifact: "menu/prices.md".into(),
            status,
            resolution: resolution.into(),
            updated_at: Utc::now(),
            attempts: 2,
            last_attempt: Some(WorkAttemptState::Produced),
            outputs: 1,
            sent_back: 1,
            accepted_by_owner: None,
        }
    }

    #[test]
    fn the_four_values_are_read_from_the_works_own_rows() {
        let record = derive_outcome_record(&facts(WorkStatus::Completed, "Priced 24 dishes."));
        assert_eq!(record.expected, "Every dish priced, with tax shown.");
        assert_eq!(
            record.assumptions,
            vec!["Prices include GST.", "The printer needs A4."]
        );
        assert_eq!(
            record.least_sure.as_deref(),
            Some("whether lunch is priced apart.")
        );
        assert_eq!(record.verdict, OutcomeVerdict::Finished);
        let lines = record.lines();
        assert_eq!(
            lines.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
            vec!["Expected", "What happened", "Cost", "Verdict"]
        );
        assert_eq!(
            lines[0].1,
            "Every dish priced, with tax shown. (output: menu/prices.md) Assumed: Prices include GST. The printer needs A4."
        );
        assert_eq!(
            lines[1].1,
            "Priced 24 dishes. (2 attempts, 1 output, sent back once)"
        );
        assert_eq!(lines[3].1, "Finished, not reviewed");
    }

    #[test]
    fn cost_is_unknown_until_the_ledger_says_and_is_never_guessed() {
        let record = derive_outcome_record(&facts(WorkStatus::Completed, "Priced."));
        assert_eq!(record.cost_usd, None);
        assert_eq!(record.lines()[2], ("Cost", "unknown".to_string()));
        let costed = record.clone().with_cost(Some(1.254));
        assert_eq!(costed.lines()[2], ("Cost", "$1.25".to_string()));
        // A Work on a route that is not charged per use has entries that add to nothing.
        assert_eq!(record.with_cost(Some(0.0)).cost_label, "$0.00");
    }

    #[test]
    fn the_verdict_follows_the_status_the_review_and_the_last_attempt() {
        let verdict = |facts: OutcomeFacts| outcome_verdict(&facts);
        let mut accepted = facts(
            WorkStatus::Completed,
            "The owner accepted the prepared outcome",
        );
        accepted.accepted_by_owner = Some(true);
        assert_eq!(verdict(accepted.clone()), OutcomeVerdict::AcceptedByOwner);
        accepted.accepted_by_owner = Some(false);
        assert_eq!(verdict(accepted), OutcomeVerdict::AcceptedByLead);
        assert_eq!(
            verdict(facts(WorkStatus::Abandoned, "no longer needed")),
            OutcomeVerdict::Abandoned
        );
        assert_eq!(
            verdict(facts(
                WorkStatus::Blocked,
                "changes requested by menu-check: prices are shown without GST"
            )),
            OutcomeVerdict::SentBack
        );
        let mut failed = facts(WorkStatus::Blocked, "the price list never loaded");
        failed.last_attempt = Some(WorkAttemptState::Failed);
        assert_eq!(verdict(failed), OutcomeVerdict::Failed);
        let mut waiting = facts(WorkStatus::Blocked, "awaiting the supplier");
        waiting.last_attempt = Some(WorkAttemptState::Blocked);
        assert_eq!(verdict(waiting), OutcomeVerdict::InProgress);
        assert_eq!(
            verdict(facts(WorkStatus::Active, "")),
            OutcomeVerdict::InProgress
        );
    }
}
