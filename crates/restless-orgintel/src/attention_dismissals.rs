//! Inbox items the owner set aside as not needed.

use super::*;
use std::collections::HashMap;

impl OrgIntel {
    /// Set an item aside now; dismissing it again moves the moment forward.
    pub async fn dismiss_attention(&self, item_id: &str, actor: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO attention_dismissals (item_id, dismissed_by) VALUES ($1,$2) \
             ON CONFLICT (item_id) DO UPDATE SET dismissed_by=EXCLUDED.dismissed_by, dismissed_at=now()",
        )
        .bind(item_id)
        .bind(actor)
        .execute(&self.pool)
        .await?;
        Ok(())
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
}
