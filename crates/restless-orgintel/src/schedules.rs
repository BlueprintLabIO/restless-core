//! Time-based coordination wakes.

use super::*;
use chrono::{Datelike as _, Days, LocalResult, NaiveDateTime, NaiveTime, TimeZone as _, Weekday};
use chrono_tz::Tz;

const SCHEDULE_COLUMNS: &str = "id, actor_id, work_id, reason, fire_at, fired_at, cancelled_at, recurrence, timezone, local_time, last_fired_at, created_at";

fn next_weekday_fire(
    after: DateTime<Utc>,
    local_time: NaiveTime,
    timezone: &str,
) -> Result<DateTime<Utc>> {
    let timezone: Tz = timezone
        .parse()
        .map_err(|_| OrgIntelError::InvalidWork(format!("unknown IANA timezone `{timezone}`")))?;
    let first_date = after.with_timezone(&timezone).date_naive();
    for offset in 0..=8 {
        let date = first_date
            .checked_add_days(Days::new(offset))
            .ok_or_else(|| OrgIntelError::InvalidWork("recurring schedule date overflow".into()))?;
        if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
            continue;
        }
        let local = NaiveDateTime::new(date, local_time);
        let candidate = match timezone.from_local_datetime(&local) {
            LocalResult::Single(value) => Some(value),
            LocalResult::Ambiguous(first, second) => Some(first.min(second)),
            LocalResult::None => None,
        };
        if let Some(candidate) = candidate {
            let candidate = candidate.with_timezone(&Utc);
            if candidate > after {
                return Ok(candidate);
            }
        }
    }
    Err(OrgIntelError::InvalidWork(
        "could not resolve the next weekday occurrence".into(),
    ))
}

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

    pub async fn add_weekday_schedule(
        &self,
        actor_id: &str,
        reason: &str,
        local_time: NaiveTime,
        timezone: &str,
        after: DateTime<Utc>,
    ) -> Result<(Uuid, DateTime<Utc>, bool)> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "a recurring time opportunity needs a reason".into(),
            ));
        }
        let fire_at = next_weekday_fire(after, local_time, timezone)?;
        let id = Uuid::new_v4();
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO schedules (id, actor_id, reason, fire_at, recurrence, timezone, local_time) \
             VALUES ($1,$2,$3,$4,'weekdays',$5,$6) \
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(id)
        .bind(actor_id)
        .bind(reason)
        .bind(fire_at)
        .bind(timezone)
        .bind(local_time)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(id) = inserted {
            return Ok((id, fire_at, true));
        }
        let existing = sqlx::query_as::<_, ScheduleRow>(&format!(
            "SELECT {SCHEDULE_COLUMNS} FROM schedules \
             WHERE actor_id=$1 AND recurrence='weekdays' AND timezone=$2 \
               AND local_time=$3 AND reason=$4 AND cancelled_at IS NULL"
        ))
        .bind(actor_id)
        .bind(timezone)
        .bind(local_time)
        .bind(reason)
        .fetch_one(&self.pool)
        .await?;
        Ok((existing.id, existing.fire_at, false))
    }

    pub async fn cancel_schedule(
        &self,
        schedule_id: Uuid,
        actor_id: &str,
        reason: &str,
    ) -> Result<bool> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "cancelling a schedule needs a reason".into(),
            ));
        }
        let cancelled = sqlx::query(
            "UPDATE schedules SET cancelled_at=now() \
             WHERE id=$1 AND actor_id=$2 AND fired_at IS NULL AND cancelled_at IS NULL",
        )
        .bind(schedule_id)
        .bind(actor_id)
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1;
        Ok(cancelled)
    }

    pub async fn claim_due_schedules(&self) -> Result<Vec<ScheduleRow>> {
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query_as::<_, ScheduleRow>(&format!(
            "SELECT {SCHEDULE_COLUMNS} \
             FROM schedules WHERE fire_at <= now() AND fired_at IS NULL AND cancelled_at IS NULL \
             ORDER BY fire_at FOR UPDATE SKIP LOCKED"
        ))
        .fetch_all(&mut *tx)
        .await?;
        let mut claimed = Vec::with_capacity(rows.len());
        for row in rows {
            let occurrence_inserted = sqlx::query(
                "INSERT INTO schedule_occurrences (schedule_id, scheduled_for) VALUES ($1,$2) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(row.id)
            .bind(row.fire_at)
            .execute(&mut *tx)
            .await?
            .rows_affected()
                == 1;
            if row.recurrence.as_deref() == Some("weekdays") {
                let timezone = row.timezone.as_deref().ok_or_else(|| {
                    OrgIntelError::InvalidWork("weekday schedule is missing timezone".into())
                })?;
                let local_time = row.local_time.ok_or_else(|| {
                    OrgIntelError::InvalidWork("weekday schedule is missing local time".into())
                })?;
                let next = next_weekday_fire(Utc::now(), local_time, timezone)?;
                sqlx::query("UPDATE schedules SET fire_at=$2, last_fired_at=now() WHERE id=$1")
                    .bind(row.id)
                    .bind(next)
                    .execute(&mut *tx)
                    .await?;
            } else {
                sqlx::query("UPDATE schedules SET fired_at=now() WHERE id=$1")
                    .bind(row.id)
                    .execute(&mut *tx)
                    .await?;
            }
            if !occurrence_inserted {
                continue;
            }
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
                    "[SCHEDULE DUE {} AT {}] {}\n\nThis is a time-based opportunity to inspect current facts. It is not evidence that production is necessary or complete.",
                    row.id, row.fire_at, row.reason
                ))
                .execute(&mut *tx)
                .await?;
            }
            claimed.push(row);
        }
        tx.commit().await?;
        Ok(claimed)
    }

    pub async fn list_schedules(
        &self,
        actor_id: Option<&str>,
        include_settled: bool,
    ) -> Result<Vec<ScheduleRow>> {
        let rows = match (actor_id, include_settled) {
            (Some(actor), true) => sqlx::query_as::<_, ScheduleRow>(&format!(
                "SELECT {SCHEDULE_COLUMNS} FROM schedules WHERE actor_id=$1 ORDER BY fire_at, created_at"
            ))
            .bind(actor)
            .fetch_all(&self.pool)
            .await?,
            (Some(actor), false) => sqlx::query_as::<_, ScheduleRow>(&format!(
                "SELECT {SCHEDULE_COLUMNS} FROM schedules WHERE actor_id=$1 AND fired_at IS NULL AND cancelled_at IS NULL ORDER BY fire_at, created_at"
            ))
            .bind(actor)
            .fetch_all(&self.pool)
            .await?,
            (None, true) => sqlx::query_as::<_, ScheduleRow>(&format!(
                "SELECT {SCHEDULE_COLUMNS} FROM schedules ORDER BY fire_at, created_at"
            ))
            .fetch_all(&self.pool)
            .await?,
            (None, false) => sqlx::query_as::<_, ScheduleRow>(&format!(
                "SELECT {SCHEDULE_COLUMNS} FROM schedules WHERE fired_at IS NULL AND cancelled_at IS NULL ORDER BY fire_at, created_at"
            ))
            .fetch_all(&self.pool)
            .await?,
        };
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekday_schedule_uses_local_timezone_and_skips_weekends() {
        let friday_after_window = DateTime::parse_from_rfc3339("2026-08-28T00:30:00Z")
            .unwrap()
            .with_timezone(&Utc); // Friday 10:30 in Sydney.
        let next = next_weekday_fire(
            friday_after_window,
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            "Australia/Sydney",
        )
        .unwrap();
        assert_eq!(next.to_rfc3339(), "2026-08-30T23:00:00+00:00");
    }

    #[test]
    fn weekday_schedule_tracks_sydney_daylight_saving() {
        let before = DateTime::parse_from_rfc3339("2026-10-04T21:59:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let next = next_weekday_fire(
            before,
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            "Australia/Sydney",
        )
        .unwrap();
        assert_eq!(next.to_rfc3339(), "2026-10-04T22:00:00+00:00");
    }
}
