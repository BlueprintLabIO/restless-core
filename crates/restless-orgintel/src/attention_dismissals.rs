//! Inbox items the owner set aside as not needed.

use super::*;
use std::collections::HashMap;

/// One set-aside item, as the owner saw it.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AttentionDismissal {
    pub item_id: String,
    pub title: String,
    pub responsible_actor: Option<String>,
    pub dismissed_at: DateTime<Utc>,
}

impl OrgIntel {
    /// Set an item aside now; dismissing it again moves the moment forward.
    pub async fn dismiss_attention(
        &self,
        item_id: &str,
        actor: &str,
        title: &str,
        responsible_actor: Option<&str>,
    ) -> Result<()> {
        let title: String = title.chars().take(400).collect();
        sqlx::query(
            "INSERT INTO attention_dismissals (item_id, dismissed_by, title, responsible_actor) \
             VALUES ($1,$2,$3,$4) ON CONFLICT (item_id) DO UPDATE SET dismissed_by=EXCLUDED.dismissed_by, \
             title=EXCLUDED.title, responsible_actor=EXCLUDED.responsible_actor, dismissed_at=now()",
        )
        .bind(item_id)
        .bind(actor)
        .bind(title)
        .bind(responsible_actor)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Bring a set-aside item back to the Inbox.
    pub async fn restore_attention(&self, item_id: &str) -> Result<bool> {
        Ok(sqlx::query("DELETE FROM attention_dismissals WHERE item_id=$1")
            .bind(item_id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            == 1)
    }

    /// When each dismissed item was set aside.
    pub async fn attention_dismissals(&self) -> Result<HashMap<String, DateTime<Utc>>> {
        Ok(
            sqlx::query_as::<_, (String, DateTime<Utc>)>(
                "SELECT item_id, dismissed_at FROM attention_dismissals",
            )
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .collect(),
        )
    }

    /// The newest set-aside items, for the Inbox archive and agents' context.
    pub async fn recent_attention_dismissals(&self, limit: i64) -> Result<Vec<AttentionDismissal>> {
        Ok(sqlx::query_as::<_, AttentionDismissal>(
            "SELECT item_id, title, responsible_actor, dismissed_at FROM attention_dismissals \
             ORDER BY dismissed_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }
}
