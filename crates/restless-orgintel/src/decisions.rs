//! Structured decisions with their probabilities, and what the company later did about them.

use super::*;
use serde::Serialize;

/// One structured decision as recorded: the probabilities beside the action, so every decision can
/// be audited and a shadow decision can be compared with what the company actually did.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DecisionRow {
    pub id: i64,
    pub kind: String,
    pub subject: String,
    pub mode: String,
    pub model: String,
    pub answers: serde_json::Value,
    pub choice: String,
    pub latency_ms: i32,
    pub outcome: Option<String>,
    pub outcome_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub struct NewDecision<'a> {
    pub kind: &'a str,
    pub subject: &'a str,
    pub mode: &'a str,
    pub model: &'a str,
    pub answers: &'a serde_json::Value,
    pub choice: &'a str,
    pub latency_ms: i32,
}

impl OrgIntel {
    /// Record a decision once per (kind, subject); a repeat of the same subject keeps the first.
    pub async fn record_decision(&self, decision: NewDecision<'_>) -> Result<bool> {
        Ok(sqlx::query(
            "INSERT INTO decision_log (kind, subject, mode, model, answers, choice, latency_ms) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (kind, subject) DO NOTHING",
        )
        .bind(decision.kind)
        .bind(decision.subject)
        .bind(decision.mode)
        .bind(decision.model)
        .bind(decision.answers)
        .bind(decision.choice)
        .bind(decision.latency_ms)
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1)
    }

    /// Subjects of one kind that already have a decision, among the given candidates.
    pub async fn decided_subjects(&self, kind: &str, subjects: &[String]) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar(
            "SELECT subject FROM decision_log WHERE kind=$1 AND subject = ANY($2)",
        )
        .bind(kind)
        .bind(subjects)
        .fetch_all(&self.pool)
        .await?)
    }

    /// What the company actually did about these subjects, written once.
    pub async fn settle_decision_outcomes(
        &self,
        kind: &str,
        subjects: &[String],
        outcome: &str,
    ) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE decision_log SET outcome=$3, outcome_at=now() \
             WHERE kind=$1 AND subject = ANY($2) AND outcome IS NULL",
        )
        .bind(kind)
        .bind(subjects)
        .bind(outcome)
        .execute(&self.pool)
        .await?
        .rows_affected())
    }

    /// Recent decisions, newest first, optionally of one kind.
    pub async fn recent_decisions(
        &self,
        kind: Option<&str>,
        limit: i64,
    ) -> Result<Vec<DecisionRow>> {
        Ok(sqlx::query_as(
            "SELECT id, kind, subject, mode, model, answers, choice, latency_ms, outcome, outcome_at, created_at \
             FROM decision_log WHERE ($1::text IS NULL OR kind=$1) ORDER BY created_at DESC, id DESC LIMIT $2",
        )
        .bind(kind)
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await?)
    }
}
