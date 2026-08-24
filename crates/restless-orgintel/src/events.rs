//! Compactable operational events and ordinary decisions.

use super::*;

impl OrgIntel {
    // ---- decisions ----

    pub async fn add_decision(&self, title: &str, body: &str, decided_by: &str) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO decisions (id, title, body, decided_by) VALUES ($1, $2, $3, $4)")
            .bind(id)
            .bind(title)
            .bind(body)
            .bind(decided_by)
            .execute(&self.pool)
            .await?;
        Ok(id)
    }

    // ---- events: operational stream, compactable, not a ledger (§4.4) ----

    pub async fn emit_event(
        &self,
        kind: &str,
        actor: Option<&str>,
        body: serde_json::Value,
    ) -> Result<i64> {
        let row = sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(kind)
        .bind(actor)
        .bind(body)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get(0))
    }

    pub async fn list_events(&self, limit: i64) -> Result<Vec<EventRow>> {
        Ok(sqlx::query_as(
            "SELECT id, kind, actor_id, body, created_at FROM events \
             ORDER BY id DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Every event of one kind, oldest first. Reconciliation reads the whole
    /// effect history: a partial view would understate what the company
    /// actually did, which is the opposite of the point.
    pub async fn events_of_kind(&self, kind: &str) -> Result<Vec<EventRow>> {
        Ok(sqlx::query_as(
            "SELECT id, kind, actor_id, body, created_at FROM events \
             WHERE kind = $1 ORDER BY id",
        )
        .bind(kind)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Events newer than a watermark, oldest first — the watch stream's
    /// incremental read (T10).
    pub async fn events_after(&self, watermark: i64) -> Result<Vec<EventRow>> {
        Ok(sqlx::query_as(
            "SELECT id, kind, actor_id, body, created_at FROM events \
             WHERE id > $1 ORDER BY id",
        )
        .bind(watermark)
        .fetch_all(&self.pool)
        .await?)
    }

    /// The body of the most recent event of a kind whose body carries a
    /// given string field value — the effect surface's idempotency replay
    /// lookup (T8).
    pub async fn find_event_body(
        &self,
        kind: &str,
        json_field: &str,
        value: &str,
    ) -> Result<Option<serde_json::Value>> {
        let row = sqlx::query(
            "SELECT body FROM events WHERE kind = $1 AND body->>$2 = $3 ORDER BY id DESC LIMIT 1",
        )
        .bind(kind)
        .bind(json_field)
        .bind(value)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| row.get(0)))
    }

    // ---- scheduler reads (T6) ----

    /// The channel internal wakeups travel on. One channel per database;
    /// the payload carries the company (schema) name.
    pub const NOTIFY_CHANNEL: &'static str = "restless_orgintel";

    /// When the most recent event of a kind happened (e.g. the last wake).
    pub async fn latest_event_at(&self, kind: &str) -> Result<Option<DateTime<Utc>>> {
        let row =
            sqlx::query("SELECT created_at FROM events WHERE kind = $1 ORDER BY id DESC LIMIT 1")
                .bind(kind)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|row| row.get(0)))
    }

    /// The most recent complete event row of one kind. Restart reconciliation
    /// needs the wake id and original trigger, not only its timestamp.
    pub async fn latest_event(&self, kind: &str) -> Result<Option<EventRow>> {
        Ok(sqlx::query_as(
            "SELECT id, kind, actor_id, body, created_at FROM events \
             WHERE kind = $1 ORDER BY id DESC LIMIT 1",
        )
        .bind(kind)
        .fetch_optional(&self.pool)
        .await?)
    }
}
