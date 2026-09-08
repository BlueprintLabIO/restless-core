//! Compactable operational events and ordinary decisions.

use super::*;

const MAX_EVENT_KIND_BYTES: usize = 96;
const MAX_EVENT_REPLAY_LIMIT: i64 = 500;
const EVENT_COMPACTION_KIND: &str = "event.history.compacted.v1";

/// One bounded page from the existing compactable operational event stream.
/// Delivery is at-least-once: consumers persist `next_after_event_id` and
/// deduplicate by stable `EventRow::id`. `resync_required` means the requested
/// cursor predates retained history (or belongs to a future/other stream), so
/// the caller must refetch current projections and resume at `snapshot_cursor`.
#[derive(Debug, serde::Serialize)]
pub struct EventReplayPage {
    pub events: Vec<EventRow>,
    pub requested_after_event_id: i64,
    pub next_after_event_id: i64,
    pub snapshot_cursor: i64,
    pub compacted_through_event_id: i64,
    pub oldest_available_event_id: Option<i64>,
    pub has_more: bool,
    pub resync_required: bool,
}

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
        let kind = kind.trim();
        if kind.is_empty() || kind.len() > MAX_EVENT_KIND_BYTES {
            return Err(OrgIntelError::InvalidEventCursor(format!(
                "event kind must contain 1 to {MAX_EVENT_KIND_BYTES} bytes"
            )));
        }
        if kind == EVENT_COMPACTION_KIND {
            return Err(OrgIntelError::InvalidEventCursor(
                "the retention marker is reserved for transactional compaction".into(),
            ));
        }
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

    /// Cursor read **before** an independently queried current-state snapshot.
    /// A short table lock first drains earlier event writers, making the
    /// returned identity watermark a committed prefix rather than merely the
    /// largest currently visible sequence value. The client then reads its
    /// projection and replays after this cursor, tolerating duplicate state: an
    /// event committed between the cursor and projection reads may appear in
    /// both. Reading this cursor after an independent projection is unsafe,
    /// because an intervening event could be hidden by the later watermark.
    pub async fn event_stream_snapshot_cursor(&self) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("LOCK TABLE events IN SHARE MODE")
            .execute(&mut *tx)
            .await?;
        let cursor = sqlx::query_scalar("SELECT COALESCE(MAX(id),0) FROM events")
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(cursor)
    }

    /// Oldest-first bounded replay from an exclusive event watermark.
    ///
    /// Bounds, retention floor, and rows are all read while one short shared
    /// table lock keeps the stream stable. Postgres identity values are
    /// watermarks rather than a promise of contiguity, so ordinary rolled-back
    /// sequence gaps never imply data loss. Only the explicit compaction marker
    /// expires a prior cursor.
    pub async fn replay_events_after(
        &self,
        after_event_id: i64,
        limit: i64,
    ) -> Result<EventReplayPage> {
        if after_event_id < 0 {
            return Err(OrgIntelError::InvalidEventCursor(
                "after_event_id must be non-negative".into(),
            ));
        }
        if !(1..=MAX_EVENT_REPLAY_LIMIT).contains(&limit) {
            return Err(OrgIntelError::InvalidEventCursor(format!(
                "event replay limit must be between 1 and {MAX_EVENT_REPLAY_LIMIT}"
            )));
        }

        let mut tx = self.pool.begin().await?;
        // BIGINT identities are allocated before commit. Without waiting for
        // all earlier writers, a lower id could commit after a later id had
        // already advanced the replay cursor and would then be skipped forever.
        sqlx::query("LOCK TABLE events IN SHARE MODE")
            .execute(&mut *tx)
            .await?;
        let compacted_through_event_id: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX((body->>'through_event_id')::BIGINT),0) \
             FROM events WHERE kind=$1",
        )
        .bind(EVENT_COMPACTION_KIND)
        .fetch_one(&mut *tx)
        .await?;
        let (oldest_available_event_id, newest_event_id): (Option<i64>, Option<i64>) =
            sqlx::query_as("SELECT MIN(id) FILTER (WHERE id>$1),MAX(id) FROM events")
                .bind(compacted_through_event_id)
                .fetch_one(&mut *tx)
                .await?;
        let snapshot_cursor = newest_event_id.unwrap_or(0);
        let resync_required = after_event_id < compacted_through_event_id
            || after_event_id > snapshot_cursor
            || (snapshot_cursor == 0 && after_event_id > 0);

        let mut events = if resync_required {
            Vec::new()
        } else {
            sqlx::query_as::<_, EventRow>(
                "SELECT id,kind,actor_id,body,created_at FROM events \
                 WHERE id>$1 AND id>$2 AND id<=$3 ORDER BY id LIMIT $4",
            )
            .bind(after_event_id)
            .bind(compacted_through_event_id)
            .bind(snapshot_cursor)
            .bind(limit + 1)
            .fetch_all(&mut *tx)
            .await?
        };
        tx.commit().await?;

        let has_more = events.len() as i64 > limit;
        if has_more {
            events.truncate(limit as usize);
        }
        let next_after_event_id = events
            .last()
            .map(|event| event.id)
            .unwrap_or(after_event_id);
        Ok(EventReplayPage {
            events,
            requested_after_event_id: after_event_id,
            next_after_event_id,
            snapshot_cursor,
            compacted_through_event_id,
            oldest_available_event_id,
            has_more,
            resync_required,
        })
    }

    /// Compact an acknowledged prefix while retaining an explicit floor in the
    /// same event stream. The marker and deletion commit together. This does
    /// not promise exactly-once delivery: lagging clients receive
    /// `resync_required` and rebuild from authoritative projections.
    pub async fn compact_events_through(&self, through_event_id: i64) -> Result<i64> {
        if through_event_id < 0 {
            return Err(OrgIntelError::InvalidEventCursor(
                "compaction cursor must be non-negative".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        // This self-conflicting lock serializes compactors and waits for all
        // in-flight event inserts. Therefore no event with an id at or below
        // the declared floor can commit after the prefix has been removed.
        sqlx::query("LOCK TABLE events IN SHARE ROW EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await?;
        let current_cursor: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(id),0) FROM events")
            .fetch_one(&mut *tx)
            .await?;
        if through_event_id > current_cursor {
            return Err(OrgIntelError::InvalidEventCursor(format!(
                "compaction cursor {through_event_id} is newer than stream cursor {current_cursor}"
            )));
        }
        let prior_floor: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX((body->>'through_event_id')::BIGINT),0) \
             FROM events WHERE kind=$1",
        )
        .bind(EVENT_COMPACTION_KIND)
        .fetch_one(&mut *tx)
        .await?;
        if through_event_id <= prior_floor {
            tx.commit().await?;
            return Ok(prior_floor);
        }
        if through_event_id == 0 {
            tx.commit().await?;
            return Ok(0);
        }

        sqlx::query("INSERT INTO events (kind,body) VALUES ($1,$2)")
            .bind(EVENT_COMPACTION_KIND)
            .bind(serde_json::json!({ "through_event_id": through_event_id }))
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM events WHERE id<=$1")
            .bind(through_event_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(through_event_id)
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
