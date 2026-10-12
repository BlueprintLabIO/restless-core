//! People own teams; teams own goals; agents do the work.
//!
//! A team may have one responsible person: a human member who directs it, answers the
//! judgement its agent lead and Exec pass up, and accepts its outcomes. Nobody responsible
//! means the owner, so a company that never appoints anyone behaves as it always did. The
//! owner appoints, either directly or by telling Exec, who records it citing that message.

use super::*;

/// One appointment or release, as the company record keeps it.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct TeamResponsibilityEvent {
    pub id: Uuid,
    pub team_id: Uuid,
    /// The person now responsible; None when the team went back to the owner.
    pub actor_id: Option<String>,
    pub previous_actor_id: Option<String>,
    pub changed_by: String,
    pub cited_message_id: Option<i64>,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

/// A judgement or review that has waited on a responsible person.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WaitingOnPerson {
    pub handoff_id: Uuid,
    pub work_id: Uuid,
    pub actor_id: String,
    pub requested_action: String,
    pub waiting_since: DateTime<Utc>,
}

impl OrgIntel {
    /// Make `person` responsible for a team, or move the team to them from someone else.
    /// Only the owner decides this: directly (`changed_by = "owner"`), or through Exec citing
    /// the owner's own message to Exec.
    pub async fn appoint_responsible_person(
        &self,
        team_id: Uuid,
        person: &str,
        changed_by: &str,
        cited_message_id: Option<i64>,
        reason: &str,
    ) -> Result<()> {
        self.change_responsible_person(team_id, Some(person), changed_by, cited_message_id, reason)
            .await
    }

    /// Hand a team back to the owner. Same authority as an appointment.
    pub async fn release_responsible_person(
        &self,
        team_id: Uuid,
        changed_by: &str,
        cited_message_id: Option<i64>,
        reason: &str,
    ) -> Result<()> {
        self.change_responsible_person(team_id, None, changed_by, cited_message_id, reason)
            .await
    }

    async fn change_responsible_person(
        &self,
        team_id: Uuid,
        person: Option<&str>,
        changed_by: &str,
        cited_message_id: Option<i64>,
        reason: &str,
    ) -> Result<()> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "changing who owns a team needs a reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        match changed_by {
            "owner" => {}
            "exec" => {
                let Some(message_id) = cited_message_id else {
                    return Err(OrgIntelError::InvalidWork(
                        "only the owner decides who owns a team: Exec must cite the owner's message"
                            .into(),
                    ));
                };
                let from_owner: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM messages WHERE id=$1 \
                     AND from_actor='owner' AND to_actor='exec')",
                )
                .bind(message_id)
                .fetch_one(&mut *tx)
                .await?;
                if !from_owner {
                    return Err(OrgIntelError::InvalidWork(
                        "only the owner decides who owns a team: the cited message is not the \
                         owner's message to Exec"
                            .into(),
                    ));
                }
            }
            _ => {
                return Err(OrgIntelError::InvalidWork(
                    "only the owner decides who owns a team".into(),
                ));
            }
        }
        let previous: Option<Option<String>> = sqlx::query_scalar(
            "SELECT responsible_actor_id FROM teams \
             WHERE id=$1 AND disbanded_at IS NULL FOR UPDATE",
        )
        .bind(team_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(previous) = previous else {
            return Err(OrgIntelError::InvalidWork(format!(
                "team {team_id} is not active"
            )));
        };
        if let Some(person) = person {
            if person == "owner" {
                return Err(OrgIntelError::InvalidWork(
                    "the owner already answers for every team nobody else owns; release the \
                     team instead"
                        .into(),
                ));
            }
            let human: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 \
                 AND actor_class='human' AND retired_at IS NULL)",
            )
            .bind(person)
            .fetch_one(&mut *tx)
            .await?;
            if !human {
                return Err(OrgIntelError::InvalidWork(format!(
                    "{person:?} is not a current person in this company; agents run teams as \
                     leads, people own them"
                )));
            }
        }
        if previous.as_deref() == person {
            return Err(OrgIntelError::InvalidWork(match person {
                Some(_) => "that person already owns this team".into(),
                None => "the owner already holds this team".into(),
            }));
        }
        sqlx::query("UPDATE teams SET responsible_actor_id=$2 WHERE id=$1")
            .bind(team_id)
            .bind(person)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO team_responsibility_events \
             (id,team_id,actor_id,previous_actor_id,changed_by,cited_message_id,reason) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(Uuid::new_v4())
        .bind(team_id)
        .bind(person)
        .bind(previous.as_deref())
        .bind(changed_by)
        .bind(cited_message_id)
        .bind(reason)
        .execute(&mut *tx)
        .await?;
        // Judgement already waiting on the previous person follows the team.
        if let Some(previous) = previous.as_deref() {
            sqlx::query(
                "UPDATE owner_handoffs h SET assigned_to=$2, delivered_at=NULL \
                 FROM work w JOIN actors a ON a.id=w.owner_id \
                 WHERE h.work_id=w.id AND a.team_id=$3 \
                   AND h.state IN ('pending','preparing') AND h.assigned_to=$1",
            )
            .bind(previous)
            .bind(person)
            .bind(team_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// The live teams this person owns.
    pub async fn teams_owned_by(&self, actor_id: &str) -> Result<Vec<Uuid>> {
        Ok(sqlx::query_scalar(
            "SELECT id FROM teams WHERE responsible_actor_id=$1 AND disbanded_at IS NULL \
             ORDER BY created_at, id",
        )
        .bind(actor_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Who owns a team, as the company record tells it, newest first.
    pub async fn team_responsibility_history(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<TeamResponsibilityEvent>> {
        Ok(sqlx::query_as(
            "SELECT id,team_id,actor_id,previous_actor_id,changed_by,cited_message_id,reason,\
                    created_at \
             FROM team_responsibility_events WHERE team_id=$1 ORDER BY created_at DESC, id",
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Put a goal under the team that delivers it, or back with the owner (`None`). The
    /// owner and Exec organise the company; a person may move a goal only into or out of a
    /// team they own.
    pub async fn set_goal_team(
        &self,
        goal_id: Uuid,
        team_id: Option<Uuid>,
        changed_by: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let current: Option<Option<Uuid>> =
            sqlx::query_scalar("SELECT team_id FROM goals WHERE id=$1 FOR UPDATE")
                .bind(goal_id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(current) = current else {
            return Err(OrgIntelError::InvalidWork(format!(
                "goal {goal_id} does not exist"
            )));
        };
        if let Some(team_id) = team_id {
            let live: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM teams WHERE id=$1 AND disbanded_at IS NULL)",
            )
            .bind(team_id)
            .fetch_one(&mut *tx)
            .await?;
            if !live {
                return Err(OrgIntelError::InvalidWork(format!(
                    "team {team_id} is not active"
                )));
            }
        }
        if !matches!(changed_by, "owner" | "exec") {
            let owns = |team: Option<Uuid>| async move {
                match team {
                    None => Ok::<bool, OrgIntelError>(false),
                    Some(team) => Ok(sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM teams WHERE id=$1 \
                         AND responsible_actor_id=$2 AND disbanded_at IS NULL)",
                    )
                    .bind(team)
                    .bind(changed_by)
                    .fetch_one(&self.pool)
                    .await?),
                }
            };
            let from_ok = current.is_none() || owns(current).await?;
            let to_ok = team_id.is_none() || owns(team_id).await?;
            if !(from_ok && to_ok) || (current.is_none() && team_id.is_none()) {
                return Err(OrgIntelError::InvalidWork(
                    "a person moves goals only between teams they own; the owner and Exec \
                     organise the rest"
                        .into(),
                ));
            }
        }
        sqlx::query("UPDATE goals SET team_id=$2 WHERE id=$1")
            .bind(goal_id)
            .bind(team_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Judgement and reviews assigned to a person that have waited at least `older_than`.
    /// Exec reminds them; past the owner's limit the item goes to the owner.
    pub async fn waiting_on_people(
        &self,
        older_than: chrono::Duration,
    ) -> Result<Vec<WaitingOnPerson>> {
        Ok(sqlx::query_as(
            "SELECT h.id AS handoff_id, h.work_id, h.assigned_to AS actor_id, h.requested_action, \
                    COALESCE(h.escalated_at, h.created_at) AS waiting_since \
             FROM owner_handoffs h JOIN actors a ON a.id=h.assigned_to \
             WHERE h.state='pending' AND a.actor_class='human' AND a.id<>'owner' \
               AND COALESCE(h.escalated_at, h.created_at) <= now() - $1 \
             ORDER BY waiting_since, h.id",
        )
        .bind(older_than)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Exec reminds each person once about judgement that has waited on them for
    /// `older_than`, in their direct conversation with Exec. Returns how many were reminded.
    pub async fn remind_waiting_people(&self, older_than: chrono::Duration) -> Result<u64> {
        let due: Vec<(Uuid, String, String)> = sqlx::query_as(
            "SELECT h.id, h.assigned_to, h.requested_action \
             FROM owner_handoffs h JOIN actors a ON a.id=h.assigned_to \
             WHERE h.state='pending' AND a.actor_class='human' AND a.id<>'owner' \
               AND a.retired_at IS NULL \
               AND COALESCE(h.escalated_at, h.created_at) <= now() - $1 \
               AND (h.person_reminded_at IS NULL \
                    OR h.person_reminded_at < COALESCE(h.escalated_at, h.created_at)) \
             ORDER BY h.created_at, h.id",
        )
        .bind(older_than)
        .fetch_all(&self.pool)
        .await?;
        let mut reminded = 0;
        for (handoff, person, action) in due {
            let claimed = sqlx::query(
                "UPDATE owner_handoffs SET person_reminded_at=now() \
                 WHERE id=$1 AND state='pending' AND assigned_to=$2 \
                   AND (person_reminded_at IS NULL \
                        OR person_reminded_at < COALESCE(escalated_at, created_at))",
            )
            .bind(handoff)
            .bind(&person)
            .execute(&self.pool)
            .await?
            .rows_affected();
            if claimed == 0 {
                continue;
            }
            let body = format!(
                "A reminder: your team is waiting on you for \"{}\". It's in your Inbox. If it \
                 isn't yours to decide, pass it to the owner from there.",
                action.trim()
            );
            self.send_message("exec", Some(&person), &body).await?;
            reminded += 1;
        }
        Ok(reminded)
    }

    /// Pass judgement a person has left unanswered for `older_than` up to the owner. The
    /// record keeps who it was waiting on. Returns how many moved.
    pub async fn escalate_unanswered_to_owner(&self, older_than: chrono::Duration) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE owner_handoffs h SET assigned_to=NULL, escalated_from=h.assigned_to, \
                    escalated_at=now(), delivered_at=NULL, \
                    resolution='No answer from ' || a.display || ' in time; passed to the owner.' \
             FROM actors a \
             WHERE a.id=h.assigned_to AND a.actor_class='human' AND a.id<>'owner' \
               AND h.state='pending' \
               AND COALESCE(h.escalated_at, h.created_at) <= now() - $1",
        )
        .bind(older_than)
        .execute(&self.pool)
        .await?
        .rows_affected())
    }
}
