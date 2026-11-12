//! What the company's own records say about how its shape is working (migration 0120).
//!
//! These are reads over Attempts, owner decisions and owner messages, plus the one table of
//! completed reviews. Deciding what counts as a signal and when a review is due is the engine's
//! (`restless_engine::org_review`); this module only fetches.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{OrgIntel, OrgIntelError, Result};

/// How one team's Attempts ended in a window. Critic Attempts (Work that `revises` other Work)
/// are counted apart from production, because a critic that passes everything and a producer
/// that lands everything look the same in a single total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamScore {
    /// `None` gathers Attempts by actors on no team (Exec's own included).
    pub team_id: Option<Uuid>,
    pub team_name: String,
    pub lead_actor_id: Option<String>,
    /// Finished production Attempts.
    pub attempts: i64,
    pub landed: i64,
    pub failed: i64,
    pub blocked: i64,
    /// Finished critic Attempts, and how many of them sent the work back.
    pub critic_attempts: i64,
    pub critic_catches: i64,
    /// Distinct actors that ran any Attempt for this team.
    pub minds: i64,
    /// Distinct production Work items those Attempts served.
    pub work_items: i64,
    /// Outcomes the owner sent back, by the producing Work's current owner.
    pub owner_sent_back: i64,
}

/// How many different actors, and how many critics, worked under one goal in a window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct GoalMinds {
    pub goal_id: Uuid,
    pub title: String,
    pub minds: i64,
    pub critics: i64,
    pub work_items: i64,
}

/// One thing the owner did that says how the company is doing. `kind` is `sent_back`,
/// `accepted`, `message` (a short owner message, classified by the engine), or `lesson`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct OwnerSignalRow {
    pub kind: String,
    pub at: DateTime<Utc>,
    /// The agent the signal is about, when the record names one.
    pub actor_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct OrgReviewRow {
    pub id: i64,
    pub reviewed_by: String,
    pub reviewed_at: DateTime<Utc>,
    pub signals_through: DateTime<Utc>,
    pub signal_count: i32,
    pub scorecard: serde_json::Value,
    pub conclusion: String,
}

/// Longest owner message read as a possible nudge. "progress?" and "try now" are short; a real
/// brief is not, and there is no reason to carry its text here.
const SHORT_MESSAGE_CHARS: i32 = 60;
const MAX_SIGNAL_ROWS: i64 = 400;
pub const MAX_CONCLUSION_CHARS: usize = 1200;

impl OrgIntel {
    /// Per-team outcomes of Attempts that started at or after `since`.
    pub async fn team_scores(&self, since: DateTime<Utc>) -> Result<Vec<TeamScore>> {
        Ok(sqlx::query_as(
            "WITH critic_work AS ( \
               SELECT DISTINCT from_work_id AS work_id FROM work_edges WHERE kind='revises' \
             ), scored AS ( \
               SELECT attempt.team_id, attempt.actor_id, attempt.work_id, attempt.state, \
                      (critic.work_id IS NOT NULL) AS critic \
               FROM work_attempts attempt \
               LEFT JOIN critic_work critic ON critic.work_id=attempt.work_id \
               WHERE attempt.started_at >= $1 AND attempt.state NOT IN ('running','superseded') \
             ), sent_back AS ( \
               SELECT member.team_id, COUNT(*) AS sent_back \
               FROM owner_handoffs handoff \
               JOIN work ON work.id=handoff.work_id \
               JOIN actors member ON member.id=work.owner_id \
               WHERE handoff.state='declined' AND handoff.assigned_to IS NULL \
                 AND handoff.resolved_at >= $1 \
               GROUP BY member.team_id \
             ) \
             SELECT scored.team_id, COALESCE(team.name,'') AS team_name, team.lead_actor_id, \
                    COUNT(*) FILTER (WHERE NOT critic) AS attempts, \
                    COUNT(*) FILTER (WHERE NOT critic AND state='produced') AS landed, \
                    COUNT(*) FILTER (WHERE NOT critic AND state='failed') AS failed, \
                    COUNT(*) FILTER (WHERE NOT critic AND state='blocked') AS blocked, \
                    COUNT(*) FILTER (WHERE critic) AS critic_attempts, \
                    COUNT(*) FILTER (WHERE critic AND state='changes_requested') AS critic_catches, \
                    COUNT(DISTINCT scored.actor_id) AS minds, \
                    COUNT(DISTINCT scored.work_id) FILTER (WHERE NOT critic) AS work_items, \
                    COALESCE(MAX(sent_back.sent_back),0) AS owner_sent_back \
             FROM scored \
             LEFT JOIN teams team ON team.id=scored.team_id \
             LEFT JOIN sent_back ON sent_back.team_id IS NOT DISTINCT FROM scored.team_id \
             GROUP BY scored.team_id, team.name, team.lead_actor_id \
             ORDER BY COUNT(*) DESC, team.name",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Open goals by how many actors worked under them since `since`, widest first.
    pub async fn goal_minds(&self, since: DateTime<Utc>) -> Result<Vec<GoalMinds>> {
        Ok(sqlx::query_as(
            "SELECT goal.id AS goal_id, goal.title, \
                    COUNT(DISTINCT attempt.actor_id) AS minds, \
                    COUNT(DISTINCT attempt.actor_id) FILTER (WHERE EXISTS ( \
                        SELECT 1 FROM work_edges edge \
                        WHERE edge.from_work_id=work.id AND edge.kind='revises')) AS critics, \
                    COUNT(DISTINCT work.id) AS work_items \
             FROM goals goal \
             JOIN work ON work.goal_id=goal.id \
             JOIN work_attempts attempt ON attempt.work_id=work.id AND attempt.started_at >= $1 \
             WHERE goal.closed_at IS NULL \
             GROUP BY goal.id, goal.title \
             ORDER BY COUNT(DISTINCT attempt.actor_id) DESC, goal.title \
             LIMIT 12",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?)
    }

    /// What the owner did after `after`, oldest first: outcomes accepted or sent back, short
    /// messages, and lessons recorded from their corrections.
    pub async fn owner_signals_after(&self, after: DateTime<Utc>) -> Result<Vec<OwnerSignalRow>> {
        Ok(sqlx::query_as(
            "SELECT kind, at, actor_id, detail FROM ( \
               SELECT CASE handoff.state WHEN 'declined' THEN 'sent_back' ELSE 'accepted' END AS kind, \
                      handoff.resolved_at AS at, work.owner_id AS actor_id, \
                      left(handoff.resolution, 200) AS detail \
               FROM owner_handoffs handoff JOIN work ON work.id=handoff.work_id \
               WHERE handoff.state IN ('declined','resolved') AND handoff.assigned_to IS NULL \
                 AND handoff.resolved_at > $1 \
               UNION ALL \
               SELECT 'message', message.created_at, NULL, message.body \
               FROM messages message \
               JOIN actors author ON author.id=message.from_actor \
               WHERE (author.kind='owner' OR author.actor_class='human') \
                 AND message.created_at > $1 AND message.deleted_at IS NULL \
                 AND char_length(message.body) <= $2 \
               UNION ALL \
               SELECT 'lesson', event.created_at, event.actor_id, event.kind \
               FROM events event \
               WHERE event.kind IN ('lesson_recorded','lesson_recurred') AND event.created_at > $1 \
             ) signals ORDER BY at LIMIT $3",
        )
        .bind(after)
        .bind(SHORT_MESSAGE_CHARS)
        .bind(MAX_SIGNAL_ROWS)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn latest_org_review(&self) -> Result<Option<OrgReviewRow>> {
        Ok(sqlx::query_as(
            "SELECT id, reviewed_by, reviewed_at, signals_through, signal_count, scorecard, \
                    conclusion \
             FROM org_reviews ORDER BY id DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn list_org_reviews(&self, limit: i64) -> Result<Vec<OrgReviewRow>> {
        Ok(sqlx::query_as(
            "SELECT id, reviewed_by, reviewed_at, signals_through, signal_count, scorecard, \
                    conclusion \
             FROM org_reviews ORDER BY id DESC LIMIT $1",
        )
        .bind(limit.clamp(1, 50))
        .fetch_all(&self.pool)
        .await?)
    }

    /// Record a finished review. Only Exec or the owner reviews the company's shape; the
    /// conclusion is required, because a review that changed nothing must still say why.
    pub async fn record_org_review(
        &self,
        reviewed_by: &str,
        signals_through: DateTime<Utc>,
        signal_count: i32,
        scorecard: serde_json::Value,
        conclusion: &str,
    ) -> Result<OrgReviewRow> {
        let conclusion = conclusion.trim();
        if conclusion.is_empty() || conclusion.chars().count() > MAX_CONCLUSION_CHARS {
            return Err(OrgIntelError::InvalidWork(format!(
                "a review needs a conclusion of 1 to {MAX_CONCLUSION_CHARS} characters: what the \
                 numbers showed and what you changed, or why nothing needed changing"
            )));
        }
        let reviewer: Option<String> =
            sqlx::query_scalar("SELECT kind FROM actors WHERE id=$1 AND retired_at IS NULL")
                .bind(reviewed_by)
                .fetch_optional(&self.pool)
                .await?;
        if !matches!(reviewer.as_deref(), Some("exec" | "owner")) {
            return Err(OrgIntelError::InvalidWork(
                "only Exec or the owner reviews the company's shape".into(),
            ));
        }
        let row: OrgReviewRow = sqlx::query_as(
            "INSERT INTO org_reviews \
               (reviewed_by, signals_through, signal_count, scorecard, conclusion) \
             VALUES ($1,$2,$3,$4,$5) \
             RETURNING id, reviewed_by, reviewed_at, signals_through, signal_count, scorecard, \
                       conclusion",
        )
        .bind(reviewed_by)
        .bind(signals_through)
        .bind(signal_count.max(0))
        .bind(scorecard)
        .bind(conclusion)
        .fetch_one(&self.pool)
        .await?;
        self.emit_event(
            "org_reviewed",
            Some(reviewed_by),
            serde_json::json!({ "review_id": row.id, "signal_count": row.signal_count }),
        )
        .await?;
        Ok(row)
    }
}
