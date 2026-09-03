//! Time-based coordination wakes.

use super::*;
use chrono::{Datelike as _, Days, LocalResult, NaiveDateTime, NaiveTime, TimeZone as _, Weekday};
use chrono_tz::Tz;

const SCHEDULE_COLUMNS: &str = "id, actor_id, work_id, reason, fire_at, fired_at, cancelled_at, recurrence, timezone, local_time, last_fired_at, missed_policy, catch_up_grace_seconds, last_missed_at, last_considered_at, machine_requirement, created_at";
const MISSED_TOLERANCE_SECONDS: i64 = 30;

fn recurring_occurrence_should_fire(
    missed_policy: &str,
    catch_up_grace_seconds: Option<i64>,
    lateness_seconds: i64,
) -> bool {
    if lateness_seconds <= MISSED_TOLERANCE_SECONDS {
        return true;
    }
    match missed_policy {
        "skip" => false,
        "skip_if_late" => catch_up_grace_seconds.is_some_and(|grace| lateness_seconds <= grace),
        "catch_up_once" => catch_up_grace_seconds.is_none_or(|grace| lateness_seconds <= grace),
        "coalesce_latest" => catch_up_grace_seconds.is_some_and(|grace| lateness_seconds <= grace),
        _ => false,
    }
}

fn validate_missed_policy(policy: &str, grace: Option<i64>) -> Result<()> {
    match (policy, grace) {
        ("skip", None) => Ok(()),
        ("skip_if_late" | "catch_up_once" | "coalesce_latest", Some(seconds)) if seconds > 0 => Ok(()),
        _ => Err(OrgIntelError::InvalidWork(
            "use skip with no grace, or skip_if_late|catch_up_once|coalesce_latest with a positive maximum-lateness window".into(),
        )),
    }
}

fn latest_weekday_fire_at_or_before(
    now: DateTime<Utc>,
    local_time: NaiveTime,
    timezone: &str,
) -> Result<DateTime<Utc>> {
    let timezone: Tz = timezone
        .parse()
        .map_err(|_| OrgIntelError::InvalidWork(format!("unknown IANA timezone `{timezone}`")))?;
    let first_date = now.with_timezone(&timezone).date_naive();
    for offset in 0..=8 {
        let date = first_date
            .checked_sub_days(Days::new(offset))
            .ok_or_else(|| {
                OrgIntelError::InvalidWork("recurring schedule date underflow".into())
            })?;
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
            if candidate <= now {
                return Ok(candidate);
            }
        }
    }
    Err(OrgIntelError::InvalidWork(
        "could not resolve the latest weekday occurrence".into(),
    ))
}

fn weekday_occurrence_count(
    first: DateTime<Utc>,
    last: DateTime<Utc>,
    timezone: &str,
) -> Result<i64> {
    if last < first {
        return Ok(0);
    }
    let timezone: Tz = timezone
        .parse()
        .map_err(|_| OrgIntelError::InvalidWork(format!("unknown IANA timezone `{timezone}`")))?;
    let first = first.with_timezone(&timezone).date_naive();
    let last = last.with_timezone(&timezone).date_naive();
    let days = (last - first).num_days();
    let full_weeks = days / 7;
    let mut count = full_weeks * 5;
    let remainder_start = first
        .checked_add_days(Days::new((full_weeks * 7) as u64))
        .ok_or_else(|| OrgIntelError::InvalidWork("recurring schedule range overflow".into()))?;
    for offset in 0..=(days - full_weeks * 7) {
        let date = remainder_start
            .checked_add_days(Days::new(offset as u64))
            .ok_or_else(|| {
                OrgIntelError::InvalidWork("recurring schedule range overflow".into())
            })?;
        if !matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
            count += 1;
        }
    }
    Ok(count)
}

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
        self.add_weekday_schedule_with_policy(
            actor_id,
            reason,
            local_time,
            timezone,
            after,
            "catch_up_once",
            Some(86_400),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn add_weekday_schedule_with_policy(
        &self,
        actor_id: &str,
        reason: &str,
        local_time: NaiveTime,
        timezone: &str,
        after: DateTime<Utc>,
        missed_policy: &str,
        catch_up_grace_seconds: Option<i64>,
    ) -> Result<(Uuid, DateTime<Utc>, bool)> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "a recurring time opportunity needs a reason".into(),
            ));
        }
        self.add_weekday_schedule_with_policy_and_requirement(
            actor_id,
            reason,
            local_time,
            timezone,
            after,
            missed_policy,
            catch_up_grace_seconds,
            "local_mac",
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn add_weekday_schedule_with_policy_and_requirement(
        &self,
        actor_id: &str,
        reason: &str,
        local_time: NaiveTime,
        timezone: &str,
        after: DateTime<Utc>,
        missed_policy: &str,
        catch_up_grace_seconds: Option<i64>,
        machine_requirement: &str,
    ) -> Result<(Uuid, DateTime<Utc>, bool)> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "a recurring time opportunity needs a reason".into(),
            ));
        }
        validate_missed_policy(missed_policy, catch_up_grace_seconds)?;
        if !matches!(machine_requirement, "local_mac" | "always_on") {
            return Err(OrgIntelError::InvalidWork(
                "machine requirement must be local_mac|always_on".into(),
            ));
        }
        let fire_at = next_weekday_fire(after, local_time, timezone)?;
        let id = Uuid::new_v4();
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO schedules (id, actor_id, reason, fire_at, recurrence, timezone, local_time, missed_policy, catch_up_grace_seconds, machine_requirement) \
             VALUES ($1,$2,$3,$4,'weekdays',$5,$6,$7,$8,$9) \
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(id)
        .bind(actor_id)
        .bind(reason)
        .bind(fire_at)
        .bind(timezone)
        .bind(local_time)
        .bind(missed_policy)
        .bind(catch_up_grace_seconds)
        .bind(machine_requirement)
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
        if existing.missed_policy != missed_policy
            || existing.catch_up_grace_seconds != catch_up_grace_seconds
            || existing.machine_requirement != machine_requirement
        {
            return Err(OrgIntelError::InvalidWork(format!(
                "the recurring schedule already exists with missed policy `{}`; update that schedule explicitly",
                existing.missed_policy
            )));
        }
        Ok((existing.id, existing.fire_at, false))
    }

    pub async fn set_schedule_missed_policy(
        &self,
        schedule_id: Uuid,
        actor_id: &str,
        missed_policy: &str,
        catch_up_grace_seconds: Option<i64>,
    ) -> Result<bool> {
        validate_missed_policy(missed_policy, catch_up_grace_seconds)?;
        let updated = sqlx::query(
            "UPDATE schedules SET missed_policy=$3, catch_up_grace_seconds=$4 \
             WHERE id=$1 AND actor_id=$2 AND recurrence IS NOT NULL \
               AND fired_at IS NULL AND cancelled_at IS NULL",
        )
        .bind(schedule_id)
        .bind(actor_id)
        .bind(missed_policy)
        .bind(catch_up_grace_seconds)
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1;
        Ok(updated)
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
        self.claim_due_schedules_at(Utc::now()).await
    }

    /// The local appliance timer deliberately ignores workloads that declare
    /// an always-on requirement. They remain visible and due for a Cloud
    /// runner rather than being falsely claimed by a sleeping laptop.
    pub async fn next_schedule_due_at(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            "SELECT min(fire_at) FROM schedules \
             WHERE fired_at IS NULL AND cancelled_at IS NULL \
               AND machine_requirement='local_mac'",
        )
        .fetch_one(&self.pool)
        .await?)
    }

    /// Frozen-clock entry for restart, sleep and DST corpus tests. Production
    /// calls [`Self::claim_due_schedules`] and supplies the real current time.
    pub async fn claim_due_schedules_at(&self, now: DateTime<Utc>) -> Result<Vec<ScheduleRow>> {
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query_as::<_, ScheduleRow>(&format!(
            "SELECT {SCHEDULE_COLUMNS} \
             FROM schedules WHERE fire_at <= $1 AND fired_at IS NULL AND cancelled_at IS NULL \
               AND machine_requirement='local_mac' \
             ORDER BY fire_at FOR UPDATE SKIP LOCKED"
        ))
        .bind(now)
        .fetch_all(&mut *tx)
        .await?;
        let mut claimed = Vec::with_capacity(rows.len());
        for mut row in rows {
            let original_fire_at = row.fire_at;
            let mut superseded_count = 0i64;
            if row.recurrence.as_deref() == Some("weekdays")
                && row.missed_policy == "coalesce_latest"
            {
                let timezone = row.timezone.as_deref().ok_or_else(|| {
                    OrgIntelError::InvalidWork("weekday schedule is missing timezone".into())
                })?;
                let local_time = row.local_time.ok_or_else(|| {
                    OrgIntelError::InvalidWork("weekday schedule is missing local time".into())
                })?;
                let latest = latest_weekday_fire_at_or_before(now, local_time, timezone)?;
                if latest > original_fire_at {
                    superseded_count =
                        weekday_occurrence_count(original_fire_at, latest, timezone)?
                            .saturating_sub(1);
                    let supersedes_through = latest_weekday_fire_at_or_before(
                        latest - chrono::Duration::milliseconds(1),
                        local_time,
                        timezone,
                    )?;
                    sqlx::query(
                        "INSERT INTO schedule_occurrences \
                         (schedule_id, scheduled_for, disposition, detail, supersedes_through, superseded_count) \
                         VALUES ($1,$2,'skipped',$3,$4,$5) ON CONFLICT DO NOTHING",
                    )
                    .bind(row.id)
                    .bind(original_fire_at)
                    .bind(format!(
                        "coalesced {superseded_count} superseded weekday occurrences into latest useful instant {latest}"
                    ))
                    .bind(supersedes_through)
                    .bind(superseded_count)
                    .execute(&mut *tx)
                    .await?;
                    row.fire_at = latest;
                }
            }
            let lateness_seconds = now.signed_duration_since(row.fire_at).num_seconds().max(0);
            let should_fire = if row.recurrence.is_none() {
                true
            } else {
                recurring_occurrence_should_fire(
                    &row.missed_policy,
                    row.catch_up_grace_seconds,
                    lateness_seconds,
                )
            };
            let disposition = if should_fire { "fired" } else { "skipped" };
            let detail = (!should_fire).then(|| {
                format!(
                    "missed by {lateness_seconds}s; policy {} did not permit catch-up",
                    row.missed_policy
                )
            });
            let occurrence_inserted = sqlx::query(
                "INSERT INTO schedule_occurrences \
                 (schedule_id, scheduled_for, disposition, detail, supersedes_through, superseded_count) \
                 VALUES ($1,$2,$3,$4,NULL,0) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(row.id)
            .bind(row.fire_at)
            .bind(disposition)
            .bind(detail)
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
                let next = next_weekday_fire(now, local_time, timezone)?;
                sqlx::query(
                    "UPDATE schedules SET fire_at=$2, last_considered_at=$4, \
                       last_fired_at=CASE WHEN $3 THEN $4 ELSE last_fired_at END, \
                       last_missed_at=CASE WHEN $3 AND $5=0 THEN last_missed_at ELSE $4 END \
                     WHERE id=$1",
                )
                .bind(row.id)
                .bind(next)
                .bind(should_fire)
                .bind(now)
                .bind(superseded_count)
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query("UPDATE schedules SET fired_at=$2, last_considered_at=$2 WHERE id=$1")
                    .bind(row.id)
                    .bind(now)
                    .execute(&mut *tx)
                    .await?;
            }
            if !occurrence_inserted {
                continue;
            }
            if !should_fire {
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
            row.last_considered_at = Some(now);
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

    pub async fn list_schedule_occurrences(
        &self,
        schedule_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ScheduleOccurrenceRow>> {
        let limit = limit.clamp(1, 200);
        Ok(sqlx::query_as::<_, ScheduleOccurrenceRow>(
            "SELECT o.schedule_id, o.scheduled_for, o.fired_at, o.disposition, o.detail, \
                    o.supersedes_through, o.superseded_count, \
                    r.recovered_at, r.message_id AS recovery_message_id, r.recovered_by, \
                    r.reason AS recovery_reason \
             FROM schedule_occurrences o \
             LEFT JOIN schedule_recoveries r USING (schedule_id, scheduled_for) \
             WHERE o.schedule_id=$1 ORDER BY o.scheduled_for DESC LIMIT $2",
        )
        .bind(schedule_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Explicitly recover one skipped occurrence by creating one durable actor wake.
    /// The occurrence row is locked with the recovery lookup and insert in one
    /// transaction, so concurrent or repeated requests return the original wake.
    pub async fn recover_skipped_schedule(
        &self,
        schedule_id: Uuid,
        scheduled_for: DateTime<Utc>,
        actor_id: &str,
        recovered_by: &str,
        reason: &str,
    ) -> Result<ScheduleRecoveryRow> {
        if recovered_by.trim().is_empty() || reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "schedule recovery needs an attributable requester and reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let occurrence = sqlx::query_as::<_, (String, String, String, Option<Uuid>)>(
            "SELECT o.disposition, s.actor_id, s.reason, s.work_id \
             FROM schedule_occurrences o JOIN schedules s ON s.id=o.schedule_id \
             WHERE o.schedule_id=$1 AND o.scheduled_for=$2 FOR UPDATE OF o",
        )
        .bind(schedule_id)
        .bind(scheduled_for)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("no such schedule occurrence".into()))?;
        if occurrence.0 != "skipped" {
            return Err(OrgIntelError::InvalidWork(
                "only a skipped schedule occurrence can be recovered".into(),
            ));
        }
        if occurrence.1 != actor_id {
            return Err(OrgIntelError::InvalidWork(
                "schedule occurrence belongs to a different actor".into(),
            ));
        }
        if occurrence.3.is_some() {
            return Err(OrgIntelError::InvalidWork(
                "Work-linked schedules must be recovered through Work".into(),
            ));
        }
        if let Some(existing) = sqlx::query_as::<_, (i64, DateTime<Utc>, String, String)>(
            "SELECT message_id, recovered_at, recovered_by, reason \
             FROM schedule_recoveries WHERE schedule_id=$1 AND scheduled_for=$2",
        )
        .bind(schedule_id)
        .bind(scheduled_for)
        .fetch_optional(&mut *tx)
        .await?
        {
            tx.commit().await?;
            return Ok(ScheduleRecoveryRow {
                schedule_id,
                scheduled_for,
                actor_id: actor_id.to_string(),
                message_id: existing.0,
                recovered_at: existing.1,
                recovered_by: existing.2,
                reason: existing.3,
                created: false,
            });
        }
        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor,to_actor,body) VALUES ('daemon',$1,$2) RETURNING id",
        )
        .bind(actor_id)
        .bind(format!(
            "[SCHEDULE RECOVERY {schedule_id} FOR {scheduled_for}] {}\n\nRecovery requested by {recovered_by}: {reason}\n\nThis is one explicit recovery of a recorded skipped occurrence. It wakes judgement only and does not itself authorise or prove any external effect.",
            occurrence.2
        ))
        .fetch_one(&mut *tx)
        .await?;
        let recovered_at: DateTime<Utc> = sqlx::query_scalar(
            "INSERT INTO schedule_recoveries \
             (schedule_id, scheduled_for, message_id, recovered_by, reason) \
             VALUES ($1,$2,$3,$4,$5) RETURNING recovered_at",
        )
        .bind(schedule_id)
        .bind(scheduled_for)
        .bind(message_id)
        .bind(recovered_by)
        .bind(reason)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ScheduleRecoveryRow {
            schedule_id,
            scheduled_for,
            actor_id: actor_id.to_string(),
            message_id,
            recovered_at,
            recovered_by: recovered_by.to_string(),
            reason: reason.to_string(),
            created: true,
        })
    }

    /// Retry a failed recovery wake under an operator-supplied idempotency key.
    /// The caller must name the exact prior wake it reconciled. Repeating the
    /// same key returns the same message; a new attempt is always explicit.
    #[allow(clippy::too_many_arguments)]
    pub async fn retry_schedule_recovery(
        &self,
        schedule_id: Uuid,
        scheduled_for: DateTime<Utc>,
        actor_id: &str,
        retry_key: &str,
        prior_message_id: i64,
        retried_by: &str,
        reason: &str,
    ) -> Result<ScheduleRecoveryRetryRow> {
        if retry_key.trim().is_empty() || retried_by.trim().is_empty() || reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "schedule recovery retry needs a key, attributable requester and reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let schedule_actor: String = sqlx::query_scalar(
            "SELECT s.actor_id FROM schedule_recoveries r \
             JOIN schedules s ON s.id=r.schedule_id \
             WHERE r.schedule_id=$1 AND r.scheduled_for=$2 FOR UPDATE OF r",
        )
        .bind(schedule_id)
        .bind(scheduled_for)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("recover the skipped occurrence first".into()))?;
        if schedule_actor != actor_id {
            return Err(OrgIntelError::InvalidWork(
                "schedule recovery belongs to a different actor".into(),
            ));
        }
        if let Some(existing) = sqlx::query_as::<_, (i64, DateTime<Utc>, String, String, i64)>(
            "SELECT message_id, retried_at, retried_by, reason, prior_message_id \
             FROM schedule_recovery_retries \
             WHERE schedule_id=$1 AND scheduled_for=$2 AND retry_key=$3",
        )
        .bind(schedule_id)
        .bind(scheduled_for)
        .bind(retry_key)
        .fetch_optional(&mut *tx)
        .await?
        {
            tx.commit().await?;
            return Ok(ScheduleRecoveryRetryRow {
                schedule_id,
                scheduled_for,
                actor_id: actor_id.to_string(),
                retry_key: retry_key.to_string(),
                prior_message_id: existing.4,
                message_id: existing.0,
                retried_at: existing.1,
                retried_by: existing.2,
                reason: existing.3,
                created: false,
            });
        }
        let expected_prior: i64 = sqlx::query_scalar(
            "SELECT COALESCE( \
               (SELECT message_id FROM schedule_recovery_retries \
                WHERE schedule_id=$1 AND scheduled_for=$2 ORDER BY retried_at DESC LIMIT 1), \
               (SELECT message_id FROM schedule_recoveries \
                WHERE schedule_id=$1 AND scheduled_for=$2))",
        )
        .bind(schedule_id)
        .bind(scheduled_for)
        .fetch_one(&mut *tx)
        .await?;
        if expected_prior != prior_message_id {
            return Err(OrgIntelError::InvalidWork(format!(
                "recovery retry is stale; latest wake message is {expected_prior}"
            )));
        }
        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor,to_actor,body) VALUES ('daemon',$1,$2) RETURNING id",
        )
        .bind(actor_id)
        .bind(format!(
            "[SCHEDULE RECOVERY RETRY {schedule_id} FOR {scheduled_for}; KEY {retry_key}; AFTER MESSAGE {prior_message_id}]\n\nRetry requested by {retried_by}: {reason}\n\nThis is one explicit retry after reconciliation of the named prior wake. Repeating this retry key cannot create another message."
        ))
        .fetch_one(&mut *tx)
        .await?;
        let retried_at: DateTime<Utc> = sqlx::query_scalar(
            "INSERT INTO schedule_recovery_retries \
             (schedule_id, scheduled_for, retry_key, prior_message_id, message_id, retried_by, reason) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING retried_at",
        )
        .bind(schedule_id)
        .bind(scheduled_for)
        .bind(retry_key)
        .bind(prior_message_id)
        .bind(message_id)
        .bind(retried_by)
        .bind(reason)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ScheduleRecoveryRetryRow {
            schedule_id,
            scheduled_for,
            actor_id: actor_id.to_string(),
            retry_key: retry_key.to_string(),
            prior_message_id,
            message_id,
            retried_at,
            retried_by: retried_by.to_string(),
            reason: reason.to_string(),
            created: true,
        })
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

    #[test]
    fn weekday_schedule_crosses_sydney_fall_back_once() {
        let friday_after_window = DateTime::parse_from_rfc3339("2026-04-02T23:30:00Z")
            .unwrap()
            .with_timezone(&Utc); // Friday 10:30 AEDT.
        let next = next_weekday_fire(
            friday_after_window,
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            "Australia/Sydney",
        )
        .unwrap();
        // DST ends on Sunday; Monday's 09:00 is one deterministic AEST instant.
        assert_eq!(next.to_rfc3339(), "2026-04-05T23:00:00+00:00");
    }

    #[test]
    fn missed_schedule_policy_distinguishes_normal_jitter_skip_and_bounded_catch_up() {
        assert!(recurring_occurrence_should_fire("skip", None, 5));
        assert!(!recurring_occurrence_should_fire("skip", None, 31));
        assert!(recurring_occurrence_should_fire(
            "catch_up_once",
            Some(7_200),
            3_600
        ));
        assert!(!recurring_occurrence_should_fire(
            "catch_up_once",
            Some(7_200),
            7_201
        ));
        assert!(recurring_occurrence_should_fire(
            "catch_up_once",
            None,
            86_400
        ));
    }
}
