//! Exec manages the owner's Inbox from their chat (owner decision, 3 November 2026): "the inbox
//! is a live thing that's the top priorities for my attention that can change based on my
//! discussion with my assistant."
//!
//! Each row is one thing Exec did on the owner's word. It always names the owner's message to
//! Exec that asked for it, so nothing here happens on Exec's own initiative. It waits out an Undo
//! window owned by the database clock; the owner gateway then applies it through the item's own
//! decision path, with that path's checks. The Inbox shows each one beside its item ("Exec
//! accepted this from your chat · Undo · 0:30 left").
//!
//! The window is the owner's, so it runs only while they can see the change: a change is held,
//! with no deadline, until the reply that carries it has finished streaming
//! ([`OrgIntel::show_exec_inbox_actions`]). From then it stays open until the owner's next message
//! to Exec ([`OrgIntel::owner_moved_on_from_exec_inbox_actions`]), but never less than
//! [`EXEC_UNDO_FLOOR`] and never more than [`EXEC_UNDO_CEILING`]. The owner's own clicks keep their
//! five seconds; those are in their hand already.
use super::*;
use serde::{Deserialize, Serialize};

/// The least time the owner has to undo a change Exec made, counted from when they can see it.
pub const EXEC_UNDO_FLOOR: std::time::Duration = std::time::Duration::from_secs(30);
/// The most: an owner who reads the reply and walks away still gets the change, two minutes on.
pub const EXEC_UNDO_CEILING: std::time::Duration = std::time::Duration::from_secs(120);

/// What Exec did. `add` raises a new item: the row itself is that Inbox item until it is set
/// aside. `move` re-ranks an item in the owner's list (`position` 1 is the top, 0 the bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecInboxVerb {
    Accept,
    Decline,
    Answer,
    Snooze,
    Dismiss,
    Move,
    Add,
}

impl ExecInboxVerb {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Decline => "decline",
            Self::Answer => "answer",
            Self::Snooze => "snooze",
            Self::Dismiss => "dismiss",
            Self::Move => "move",
            Self::Add => "add",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "accept" => Self::Accept,
            "decline" => Self::Decline,
            "answer" => Self::Answer,
            "snooze" => Self::Snooze,
            "dismiss" => Self::Dismiss,
            "move" => Self::Move,
            "add" => Self::Add,
            _ => return None,
        })
    }
}

/// One Exec action as saved: what, on which item, citing which owner message.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ExecInboxAction {
    pub id: Uuid,
    pub item_id: String,
    pub item_title: String,
    pub verb: String,
    pub note: String,
    pub target: serde_json::Value,
    pub snooze_until: Option<DateTime<Utc>>,
    pub position: Option<i32>,
    pub source_message_id: i64,
    pub owner_words: String,
    /// When the Undo window ends; none while the reply carrying the change is still streaming.
    pub undo_until: Option<DateTime<Utc>>,
    /// When the owner could first see it: the reply that carries it had finished.
    pub shown_at: Option<DateTime<Utc>>,
    pub state: String,
    pub problem: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl ExecInboxAction {
    pub fn verb(&self) -> Option<ExecInboxVerb> {
        ExecInboxVerb::parse(&self.verb)
    }

    /// Still waiting for the reply that carries it to finish: the owner has not seen it yet.
    pub fn held(&self) -> bool {
        self.state == "pending" && self.shown_at.is_none()
    }
}

/// What Exec asks to do. The caller (the engine's coordination socket) has already checked the
/// item exists and that this verb applies to its kind; this layer owns the grounding: the cited
/// message must be the company owner's own message to Exec.
#[derive(Debug, Clone)]
pub struct NewExecInboxAction<'a> {
    pub id: Uuid,
    pub item_id: &'a str,
    pub item_title: &'a str,
    pub verb: ExecInboxVerb,
    pub note: &'a str,
    pub target: serde_json::Value,
    pub snooze_until: Option<DateTime<Utc>>,
    pub position: Option<i32>,
    pub source_message_id: i64,
}

const OWNER_WORDS_LIMIT: usize = 400;

impl OrgIntel {
    /// The whole body of the company owner's own message to Exec, or a refusal naming why it
    /// cannot ground an Inbox action or a recorded answer: only the owner's own chat with Exec can
    /// (a member's or an admin's cannot decide for the owner, and an agent's message is never the
    /// owner's word).
    pub async fn owner_message_to_exec(&self, message_id: i64) -> Result<String> {
        let row: Option<(String, Option<String>, String)> =
            sqlx::query_as("SELECT from_actor, to_actor, body FROM messages WHERE id=$1")
                .bind(message_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some((from, to, body)) = row else {
            return Err(OrgIntelError::InvalidWork(format!(
                "message {message_id} does not exist; cite the owner's message that asked for this"
            )));
        };
        if to.as_deref() != Some("exec") {
            return Err(OrgIntelError::InvalidWork(format!(
                "message {message_id} is not the owner's message to Exec; cite their message in your chat"
            )));
        }
        if from != "owner" {
            // A company owner who signed in through a membership has their own human Actor.
            let owner: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM human_principal_actor_bindings binding \
                 JOIN actors actor ON actor.id=binding.actor_id \
                 WHERE binding.actor_id=$1 AND binding.membership_role='owner' \
                   AND binding.membership_status='active' AND actor.actor_class='human' \
                   AND actor.retired_at IS NULL)",
            )
            .bind(&from)
            .fetch_one(&self.pool)
            .await?;
            if !owner {
                return Err(OrgIntelError::InvalidWork(format!(
                    "message {message_id} is not the owner's message ({from} wrote it); only the \
                     owner's own words decide what waits on the owner"
                )));
            }
        }
        // The owner's words only: the page they sent it from rides along for Exec's context
        // (`transcript::CONTEXT_BLOCK` / `CONTEXT_MARKER`) and is not what they said.
        let words = ["\n\n[Owner cockpit context]\n", "\n\n<!--restless-context:"]
            .iter()
            .filter_map(|marker| body.find(marker))
            .min()
            .map_or(body.as_str(), |end| &body[..end]);
        Ok(words.trim().to_string())
    }

    /// [`Self::owner_message_to_exec`], as the short excerpt the Inbox quotes.
    pub async fn owner_words_to_exec(&self, message_id: i64) -> Result<String> {
        let body = self.owner_message_to_exec(message_id).await?;
        let words = body.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut excerpt: String = words.chars().take(OWNER_WORDS_LIMIT).collect();
        if words.chars().count() > OWNER_WORDS_LIMIT {
            excerpt.push('…');
        }
        Ok(excerpt)
    }

    /// Save one Exec action, held until the reply that carries it is shown (no deadline yet).
    /// Nothing happens to the item until its Undo window ends. A replay of
    /// the same id returns the saved row; a second action on an item that already has one waiting
    /// is refused, so the owner never has two pending changes on one card.
    pub async fn queue_exec_inbox_action(
        &self,
        action: &NewExecInboxAction<'_>,
    ) -> Result<ExecInboxAction> {
        if action.item_id.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork("name the Inbox item".into()));
        }
        if action.note.len() > 16 * 1024 {
            return Err(OrgIntelError::InvalidWork(
                "the note must not exceed 16 KiB".into(),
            ));
        }
        match action.verb {
            ExecInboxVerb::Snooze if action.snooze_until.is_none() => {
                return Err(OrgIntelError::InvalidWork(
                    "a snooze needs the time it ends".into(),
                ))
            }
            ExecInboxVerb::Move if action.position.is_none() => {
                return Err(OrgIntelError::InvalidWork(
                    "a move needs the place in the list".into(),
                ))
            }
            ExecInboxVerb::Answer | ExecInboxVerb::Add if action.note.trim().is_empty() => {
                return Err(OrgIntelError::InvalidWork(
                    "say what the owner said: the answer or the item needs text".into(),
                ))
            }
            _ => {}
        }
        let owner_words = self.owner_words_to_exec(action.source_message_id).await?;
        let title: String = action.item_title.trim().chars().take(400).collect();
        let mut tx = self.pool.begin().await?;
        if let Some(saved) =
            sqlx::query_as::<_, ExecInboxAction>("SELECT * FROM exec_inbox_actions WHERE id=$1")
                .bind(action.id)
                .fetch_optional(&mut *tx)
                .await?
        {
            tx.commit().await?;
            if saved.item_id != action.item_id || saved.verb != action.verb.as_str() {
                return Err(OrgIntelError::InvalidWork(
                    "this action id already names a different action".into(),
                ));
            }
            return Ok(saved);
        }
        let saved = sqlx::query_as::<_, ExecInboxAction>(
            "INSERT INTO exec_inbox_actions \
               (id,item_id,item_title,verb,note,target,snooze_until,position,source_message_id,owner_words) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) \
             ON CONFLICT DO NOTHING RETURNING *",
        )
        .bind(action.id)
        .bind(action.item_id)
        .bind(&title)
        .bind(action.verb.as_str())
        .bind(action.note.trim())
        .bind(&action.target)
        .bind(action.snooze_until)
        .bind(action.position)
        .bind(action.source_message_id)
        .bind(&owner_words)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(
                "another change to this item is already waiting out its Undo window".into(),
            )
        })?;
        notify_exec_inbox_changed(&mut tx).await?;
        tx.commit().await?;
        Ok(saved)
    }

    /// Undo before the window ends. The gateway checks the caller is the owner.
    pub async fn cancel_exec_inbox_action(&self, id: Uuid) -> Result<ExecInboxAction> {
        let mut tx = self.pool.begin().await?;
        let saved = sqlx::query_as::<_, ExecInboxAction>(
            "SELECT * FROM exec_inbox_actions WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("This change was not found.".into()))?;
        if saved.state == "canceled" {
            tx.commit().await?;
            return Ok(saved);
        }
        let Some(saved) = sqlx::query_as::<_, ExecInboxAction>(
            "UPDATE exec_inbox_actions SET state='canceled',completed_at=clock_timestamp() \
             WHERE id=$1 AND state='pending' \
               AND (undo_until IS NULL OR undo_until>clock_timestamp()) RETURNING *",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            // Release the row now. A dropped transaction rolls back later, and until then the
            // worker applying this due change skips the row as locked.
            tx.rollback().await?;
            return Err(OrgIntelError::InvalidWork(
                "The Undo window has ended.".into(),
            ));
        };
        notify_exec_inbox_changed(&mut tx).await?;
        tx.commit().await?;
        Ok(saved)
    }

    /// Open the Undo window on every held action: the reply that carries them has finished and
    /// the owner can see them now. Exec makes one turn at a time, so every held action belongs to
    /// the reply that just ended, or to a turn that stopped before its reply did and is shown now.
    /// Returns how many windows opened.
    pub async fn show_exec_inbox_actions(&self) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let opened = sqlx::query(
            "UPDATE exec_inbox_actions SET shown_at=clock_timestamp(), \
               undo_until=clock_timestamp() + make_interval(secs => $1) \
             WHERE state='pending' AND shown_at IS NULL",
        )
        .bind(EXEC_UNDO_CEILING.as_secs_f64())
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if opened > 0 {
            notify_exec_inbox_changed(&mut tx).await?;
        }
        tx.commit().await?;
        Ok(opened)
    }

    /// The owner sent Exec their next message: they have moved on from the changes already shown,
    /// so each window ends now, or once it has been open for [`EXEC_UNDO_FLOOR`], whichever is
    /// later. A change still held (its reply streaming) is untouched. Returns how many changed.
    pub async fn owner_moved_on_from_exec_inbox_actions(&self) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let changed = sqlx::query(
            "UPDATE exec_inbox_actions SET undo_until=GREATEST( \
                 shown_at + make_interval(secs => $1), clock_timestamp()) \
             WHERE state='pending' AND shown_at IS NOT NULL \
               AND undo_until > GREATEST(shown_at + make_interval(secs => $1), clock_timestamp())",
        )
        .bind(EXEC_UNDO_FLOOR.as_secs_f64())
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if changed > 0 {
            notify_exec_inbox_changed(&mut tx).await?;
        }
        tx.commit().await?;
        Ok(changed)
    }

    /// Due actions, oldest deadline first. The worker claims each one before applying it.
    pub async fn due_exec_inbox_actions(&self) -> Result<Vec<ExecInboxAction>> {
        Ok(sqlx::query_as(
            "SELECT * FROM exec_inbox_actions WHERE state='pending' \
             AND undo_until<=clock_timestamp() ORDER BY undo_until,id LIMIT 50",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Hold a due action while it is applied: a second worker skips it, and a late Undo waits,
    /// then finds the window over. Settle it with [`ExecInboxClaim::settle`].
    pub async fn claim_exec_inbox_action(&self, id: Uuid) -> Result<Option<ExecInboxClaim>> {
        let mut tx = self.pool.begin().await?;
        let saved = sqlx::query_as::<_, ExecInboxAction>(
            "SELECT * FROM exec_inbox_actions WHERE id=$1 AND state='pending' \
             AND undo_until<=clock_timestamp() FOR UPDATE SKIP LOCKED",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        match saved {
            Some(saved) => Ok(Some(ExecInboxClaim { tx, saved })),
            None => {
                tx.rollback().await?;
                Ok(None)
            }
        }
    }

    /// Recent actions, newest first: what the Inbox shows beside its items and what Exec reads
    /// back about its own earlier actions.
    pub async fn recent_exec_inbox_actions(&self, limit: i64) -> Result<Vec<ExecInboxAction>> {
        Ok(sqlx::query_as(
            "SELECT * FROM exec_inbox_actions WHERE created_at > clock_timestamp() - INTERVAL '14 days' \
             ORDER BY created_at DESC, id DESC LIMIT $1",
        )
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await?)
    }

    /// Items Exec added. The Inbox projection drops the ones the owner has since set aside, like
    /// any other item, so a set-aside one can be brought back.
    pub async fn open_exec_inbox_items(&self) -> Result<Vec<ExecInboxAction>> {
        Ok(sqlx::query_as(
            "SELECT action.* FROM exec_inbox_actions action \
             WHERE action.verb='add' AND action.state='applied' \
               AND action.completed_at > clock_timestamp() - INTERVAL '90 days' \
             ORDER BY action.created_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// The current list position Exec gave each item: the newest applied move per item wins.
    pub async fn exec_inbox_positions(&self) -> Result<Vec<(String, i32, DateTime<Utc>)>> {
        Ok(sqlx::query_as(
            "SELECT DISTINCT ON (item_id) item_id, position, completed_at FROM exec_inbox_actions \
             WHERE verb='move' AND state='applied' AND position IS NOT NULL \
             ORDER BY item_id, completed_at DESC",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Delay to the next Exec action deadline, by the database clock.
    pub async fn next_exec_inbox_delay(&self) -> Result<Option<std::time::Duration>> {
        let seconds: Option<f64> = sqlx::query_scalar(
            "SELECT GREATEST(0, EXTRACT(EPOCH FROM (MIN(undo_until) - clock_timestamp())))::double precision \
             FROM exec_inbox_actions WHERE state='pending' AND undo_until IS NOT NULL \
             HAVING COUNT(*) > 0",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(seconds.map(std::time::Duration::from_secs_f64))
    }
}

/// One worker's hold on a due Exec action.
pub struct ExecInboxClaim {
    tx: Transaction<'static, Postgres>,
    saved: ExecInboxAction,
}

impl ExecInboxClaim {
    pub fn action(&self) -> &ExecInboxAction {
        &self.saved
    }

    /// Record the outcome: applied, or failed with the plain reason the owner and Exec will read.
    pub async fn settle(mut self, problem: Option<String>) -> Result<&'static str> {
        let state = match problem {
            None => {
                sqlx::query(
                    "UPDATE exec_inbox_actions SET state='applied',completed_at=clock_timestamp() \
                     WHERE id=$1",
                )
                .bind(self.saved.id)
                .execute(&mut *self.tx)
                .await?;
                "applied"
            }
            Some(problem) => {
                let problem: String = problem.chars().take(2000).collect();
                sqlx::query(
                    "UPDATE exec_inbox_actions SET state='failed',problem=$2,completed_at=clock_timestamp() \
                     WHERE id=$1",
                )
                .bind(self.saved.id)
                .bind(problem)
                .execute(&mut *self.tx)
                .await?;
                "failed"
            }
        };
        notify_exec_inbox_changed(&mut self.tx).await?;
        self.tx.commit().await?;
        Ok(state)
    }
}

/// Body-free hint, delivered only when the transaction commits. It reuses the handoff decision
/// kind, so the gateway's decision worker and the cockpit's receipt refresh both wake on it.
async fn notify_exec_inbox_changed(tx: &mut Transaction<'_, Postgres>) -> Result<()> {
    sqlx::query("SELECT pg_notify('restless_orgintel', json_build_object('company',current_schema(),'kind','handoff_decision_changed','body',json_build_object())::text)")
        .execute(&mut **tx)
        .await?;
    Ok(())
}
