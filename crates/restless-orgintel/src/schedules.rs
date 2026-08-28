//! Time-based coordination wakes.

use super::*;

impl OrgIntel {
    pub async fn add_schedule(
        &self,
        actor_id: &str,
        work_id: Option<Uuid>,
        reason: &str,
        fire_at: DateTime<Utc>,
    ) -> Result<Uuid> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "a time dependency needs a reason".into(),
            ));
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO schedules (id, actor_id, work_id, reason, fire_at) VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id)
        .bind(actor_id)
        .bind(work_id)
        .bind(reason)
        .bind(fire_at)
        .execute(&mut *tx)
        .await?;
        if let Some(work_id) = work_id {
            sqlx::query("UPDATE work SET status='blocked', resolution=$2 WHERE id=$1")
                .bind(work_id)
                .bind(format!("waiting for schedule {id}: {reason}"))
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(id)
    }

    pub async fn claim_due_schedules(&self) -> Result<Vec<ScheduleRow>> {
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query_as::<_, ScheduleRow>(
            "SELECT id, actor_id, work_id, reason, fire_at, fired_at, cancelled_at, created_at \
             FROM schedules WHERE fire_at <= now() AND fired_at IS NULL AND cancelled_at IS NULL \
             ORDER BY fire_at FOR UPDATE SKIP LOCKED",
        )
        .fetch_all(&mut *tx)
        .await?;
        for row in &rows {
            sqlx::query("UPDATE schedules SET fired_at=now() WHERE id=$1")
                .bind(row.id)
                .execute(&mut *tx)
                .await?;
            if let Some(work_id) = row.work_id {
                sqlx::query(
                    "UPDATE work SET status='active', resolution='time condition reached' \
                     WHERE id=$1 AND resolution LIKE $2 \
                       AND NOT EXISTS (SELECT 1 FROM schedules s WHERE s.work_id=$1 \
                         AND s.id<>$3 AND s.fired_at IS NULL AND s.cancelled_at IS NULL)",
                )
                .bind(work_id)
                .bind(format!("waiting for schedule {}:%", row.id))
                .bind(row.id)
                .execute(&mut *tx)
                .await?;
            } else if row.actor_id != "exec" {
                // Consume the time fact and create its recoverable actor wake
                // in one transaction. A crash can therefore leave both
                // pending or neither, never a fired schedule with no delivery.
                // Work-linked schedules already release Work above and must
                // not race that deterministic kickoff with conversation.
                sqlx::query(
                    "INSERT INTO messages (from_actor,to_actor,body) VALUES ('daemon',$1,$2)",
                )
                .bind(&row.actor_id)
                .bind(format!(
                    "[SCHEDULE DUE {}] {}\n\nThis is a time-based opportunity to inspect current facts. It is not evidence that production is necessary or complete.",
                    row.id, row.reason
                ))
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(rows)
    }

    pub async fn list_schedules(
        &self,
        actor_id: Option<&str>,
        include_settled: bool,
    ) -> Result<Vec<ScheduleRow>> {
        let rows = match (actor_id, include_settled) {
            (Some(actor), true) => sqlx::query_as::<_, ScheduleRow>(
                "SELECT id, actor_id, work_id, reason, fire_at, fired_at, cancelled_at, created_at \
                 FROM schedules WHERE actor_id=$1 ORDER BY fire_at, created_at",
            )
            .bind(actor)
            .fetch_all(&self.pool)
            .await?,
            (Some(actor), false) => sqlx::query_as::<_, ScheduleRow>(
                "SELECT id, actor_id, work_id, reason, fire_at, fired_at, cancelled_at, created_at \
                 FROM schedules WHERE actor_id=$1 AND fired_at IS NULL AND cancelled_at IS NULL \
                 ORDER BY fire_at, created_at",
            )
            .bind(actor)
            .fetch_all(&self.pool)
            .await?,
            (None, true) => sqlx::query_as::<_, ScheduleRow>(
                "SELECT id, actor_id, work_id, reason, fire_at, fired_at, cancelled_at, created_at \
                 FROM schedules ORDER BY fire_at, created_at",
            )
            .fetch_all(&self.pool)
            .await?,
            (None, false) => sqlx::query_as::<_, ScheduleRow>(
                "SELECT id, actor_id, work_id, reason, fire_at, fired_at, cancelled_at, created_at \
                 FROM schedules WHERE fired_at IS NULL AND cancelled_at IS NULL \
                 ORDER BY fire_at, created_at",
            )
            .fetch_all(&self.pool)
            .await?,
        };
        Ok(rows)
    }
}
