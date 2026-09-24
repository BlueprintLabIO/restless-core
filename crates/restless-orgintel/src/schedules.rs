//! Time-based coordination wakes.

use super::*;
use chrono::{Datelike as _, Days, LocalResult, NaiveDateTime, NaiveTime, TimeZone as _, Weekday};
use chrono_tz::Tz;

const SCHEDULE_COLUMNS: &str = "id, actor_id, work_id, reason, fire_at, fired_at, cancelled_at, recurrence, timezone, local_time, last_fired_at, missed_policy, catch_up_grace_seconds, last_missed_at, last_considered_at, machine_requirement, created_at, interval_seconds, responsibility_id, responsibility_version";
const MISSED_TOLERANCE_SECONDS: i64 = 30;
const DEFAULT_OPPORTUNITY_WINDOW_SECONDS: i64 = 2 * 60 * 60;

fn opportunity_window_seconds(policy: &serde_json::Value) -> Result<i64> {
    let seconds = match policy.get("window_seconds") {
        None => DEFAULT_OPPORTUNITY_WINDOW_SECONDS,
        Some(value) => value.as_i64().ok_or_else(|| {
            OrgIntelError::InvalidWork("responsibility window_seconds must be an integer".into())
        })?,
    };
    if !(300..=604_800).contains(&seconds) {
        return Err(OrgIntelError::InvalidWork(
            "responsibility window_seconds must be between five minutes and seven days".into(),
        ));
    }
    Ok(seconds)
}

fn required_outcome_areas(policy: &serde_json::Value) -> Result<Vec<String>> {
    let Some(value) = policy.get("required_outcome_areas") else {
        return Ok(Vec::new());
    };
    let areas = value.as_array().ok_or_else(|| {
        OrgIntelError::InvalidWork("required_outcome_areas must be an array".into())
    })?;
    if areas.is_empty() || areas.len() > 8 {
        return Err(OrgIntelError::InvalidWork(
            "required_outcome_areas must contain one to eight areas".into(),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    areas
        .iter()
        .map(|value| {
            let area = value.as_str().ok_or_else(|| {
                OrgIntelError::InvalidWork("required outcome area must be a string".into())
            })?;
            if area.is_empty()
                || area.len() > 64
                || !area
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
                || !seen.insert(area)
            {
                return Err(OrgIntelError::InvalidWork(
                    "required outcome areas must be distinct lowercase identifiers".into(),
                ));
            }
            Ok(area.to_owned())
        })
        .collect()
}
const OPPORTUNITY_WAKE_ACK_SECONDS: i64 = 300;

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

/// The first interval slot strictly after `now`. Missed slots are coalesced:
/// a laptop that slept through six `/loop 30m` slots wakes the actor once.
fn next_interval_fire(
    fire_at: DateTime<Utc>,
    now: DateTime<Utc>,
    interval_seconds: Option<i32>,
) -> Result<DateTime<Utc>> {
    let interval = i64::from(
        interval_seconds
            .filter(|seconds| *seconds > 0)
            .ok_or_else(|| {
                OrgIntelError::InvalidWork("interval schedule is missing its interval".into())
            })?,
    );
    let behind = now.signed_duration_since(fire_at).num_seconds().max(0);
    let steps = behind / interval + 1;
    Ok(fire_at + chrono::Duration::seconds(steps * interval))
}

/// Bounds for `/loop`: at least five minutes, at most thirty days.
pub const MIN_INTERVAL_SECONDS: i32 = 300;
pub const MAX_INTERVAL_SECONDS: i32 = 2_592_000;

impl OrgIntel {
    pub async fn get_opportunity(&self, opportunity_id: Uuid) -> Result<Option<OpportunityRow>> {
        Ok(sqlx::query_as::<_, OpportunityRow>(
            "SELECT id, actor_id, responsibility_id, responsibility_version, state, outcome, outcome_reason, \
               evidence_refs, revision, owner_epoch, lease_owner, lease_expires_at, next_wake_at, \
               deadline_at, last_progress_at, wake_message_id, wake_count, created_at, settled_at \
             FROM opportunities WHERE id=$1",
        )
        .bind(opportunity_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn list_open_opportunities(&self) -> Result<Vec<OpportunityRow>> {
        Ok(sqlx::query_as::<_, OpportunityRow>(
            "SELECT id, actor_id, responsibility_id, responsibility_version, state, outcome, outcome_reason, \
               evidence_refs, revision, owner_epoch, lease_owner, lease_expires_at, next_wake_at, \
               deadline_at, last_progress_at, wake_message_id, wake_count, created_at, settled_at \
             FROM opportunities WHERE state NOT IN ('completed','needs_human','blocked','cancelled') \
             ORDER BY created_at, id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// A linked Work result is new evidence, so revisit its waiting
    /// Opportunity without waiting for the ordinary retry backoff. Advance
    /// last_progress_at in the same update to avoid redelivering the same
    /// result on every reconciliation pass.
    pub async fn expedite_waiting_opportunities_with_work_result(
        &self,
        now: DateTime<Utc>,
    ) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE opportunities o SET next_wake_at=$1, last_progress_at=$1, revision=revision+1 \
             WHERE o.state='waiting_retry' AND o.next_wake_at > $1 \
               AND EXISTS (SELECT 1 FROM opportunity_work ow JOIN work w ON w.id=ow.work_id \
                           WHERE ow.opportunity_id=o.id AND w.status IN ('completed','blocked') \
                             AND w.updated_at > o.last_progress_at)",
        )
        .bind(now)
        .execute(&self.pool)
        .await?
        .rows_affected())
    }

    pub async fn list_opportunities(
        &self,
        responsibility_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<OpportunityRow>> {
        if !(1..=500).contains(&limit) {
            return Err(OrgIntelError::InvalidWork(
                "opportunity history limit must be between 1 and 500".into(),
            ));
        }
        Ok(sqlx::query_as::<_, OpportunityRow>(
            "SELECT id, actor_id, responsibility_id, responsibility_version, state, outcome, outcome_reason, \
               evidence_refs, revision, owner_epoch, lease_owner, lease_expires_at, next_wake_at, \
               deadline_at, last_progress_at, wake_message_id, wake_count, created_at, settled_at \
             FROM opportunities WHERE ($1::uuid IS NULL OR responsibility_id=$1) \
             ORDER BY created_at DESC, id LIMIT $2",
        )
        .bind(responsibility_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn opportunity_for_wake_message(
        &self,
        message_id: i64,
    ) -> Result<Option<OpportunityRow>> {
        Ok(sqlx::query_as::<_, OpportunityRow>(
            "SELECT id, actor_id, responsibility_id, responsibility_version, state, outcome, outcome_reason, \
               evidence_refs, revision, owner_epoch, lease_owner, lease_expires_at, next_wake_at, \
               deadline_at, last_progress_at, wake_message_id, wake_count, created_at, settled_at \
             FROM opportunities WHERE wake_message_id=$1 OR id=(SELECT opportunity_id \
               FROM schedule_occurrences WHERE wake_message_id=$1 LIMIT 1)",
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn list_opportunity_occurrences(
        &self,
        opportunity_id: Uuid,
    ) -> Result<Vec<OpportunityOccurrenceLink>> {
        Ok(sqlx::query_as::<_, OpportunityOccurrenceLink>(
            "SELECT schedule_id, scheduled_for, opportunity_id, responsibility_id, \
               responsibility_version, admission, wake_message_id FROM schedule_occurrences \
             WHERE opportunity_id=$1 ORDER BY scheduled_for",
        )
        .bind(opportunity_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// The most recent durable opportunities admitted by one exact schedule.
    /// Query by schedule before limiting so other wakeups cannot hide its runs.
    pub async fn list_schedule_opportunity_occurrences(
        &self,
        schedule_id: Uuid,
        limit: i64,
    ) -> Result<Vec<OpportunityOccurrenceLink>> {
        let limit = limit.clamp(1, 200);
        Ok(sqlx::query_as::<_, OpportunityOccurrenceLink>(
            "SELECT schedule_id, scheduled_for, opportunity_id, responsibility_id, \
               responsibility_version, admission, wake_message_id FROM schedule_occurrences \
             WHERE schedule_id=$1 AND opportunity_id IS NOT NULL \
             ORDER BY scheduled_for DESC LIMIT $2",
        )
        .bind(schedule_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn list_opportunity_work(
        &self,
        opportunity_id: Uuid,
    ) -> Result<Vec<OpportunityWorkLink>> {
        Ok(sqlx::query_as::<_, OpportunityWorkLink>(
            "SELECT opportunity_id, work_id, linked_at, relation FROM opportunity_work \
             WHERE opportunity_id=$1 ORDER BY linked_at, work_id",
        )
        .bind(opportunity_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn link_opportunity_work(
        &self,
        opportunity_id: Uuid,
        owner_epoch: i64,
        work_id: Uuid,
        relation: &str,
        now: DateTime<Utc>,
    ) -> Result<bool> {
        if !matches!(relation, "primary" | "supporting") {
            return Err(OrgIntelError::InvalidWork(
                "opportunity Work relation must be primary or supporting".into(),
            ));
        }
        let linked = sqlx::query(
            "INSERT INTO opportunity_work (opportunity_id, work_id, relation) \
             SELECT $1,$3,$4 WHERE EXISTS (SELECT 1 FROM opportunities WHERE id=$1 \
               AND owner_epoch=$2 AND lease_expires_at > $5 \
               AND state NOT IN ('completed','needs_human','blocked','cancelled')) \
             ON CONFLICT (opportunity_id, work_id) DO NOTHING",
        )
        .bind(opportunity_id)
        .bind(owner_epoch)
        .bind(work_id)
        .bind(relation)
        .bind(now)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if linked == 1 {
            return Ok(true);
        }
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM opportunity_work WHERE opportunity_id=$1 AND work_id=$2)",
        )
        .bind(opportunity_id)
        .bind(work_id)
        .fetch_one(&self.pool)
        .await?;
        if exists {
            Ok(false)
        } else {
            Err(OrgIntelError::InvalidWork(
                "current opportunity claim is required to link Work".into(),
            ))
        }
    }

    /// Re-deliver due opportunities after a lost or consumed wake. Each
    /// delivery is committed with its inbox message and sequence number, so a
    /// crash or repeated scan cannot emit the same retry twice. Expired
    /// deadlines and wake budgets settle as blocked instead of looping forever.
    pub async fn wake_due_opportunities_at(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<OpportunityWake>> {
        self.wake_due_opportunities_with_policy_at(now, 4, OPPORTUNITY_WAKE_ACK_SECONDS)
            .await
    }

    pub async fn wake_due_opportunities_with_policy_at(
        &self,
        now: DateTime<Utc>,
        max_wakes: i32,
        retry_after_seconds: i64,
    ) -> Result<Vec<OpportunityWake>> {
        if !(1..=20).contains(&max_wakes) || !(1..=86_400).contains(&retry_after_seconds) {
            return Err(OrgIntelError::InvalidWork(
                "opportunity wake limits must be 1-20 deliveries and 1-86400 seconds backoff"
                    .into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query_as::<_, OpportunityRow>(
            "SELECT id, actor_id, responsibility_id, responsibility_version, state, outcome, outcome_reason, \
               evidence_refs, revision, owner_epoch, lease_owner, lease_expires_at, next_wake_at, \
               deadline_at, last_progress_at, wake_message_id, wake_count, created_at, settled_at \
             FROM opportunities WHERE state NOT IN ('completed','needs_human','blocked','cancelled') \
               AND ((lease_expires_at IS NULL AND next_wake_at <= $1) OR lease_expires_at <= $1 OR deadline_at <= $1) \
             ORDER BY COALESCE(next_wake_at, lease_expires_at, deadline_at), created_at \
             FOR UPDATE SKIP LOCKED",
        )
        .bind(now)
        .fetch_all(&mut *tx)
        .await?;
        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            if row.deadline_at.is_some_and(|deadline| deadline <= now)
                || row.wake_count >= max_wakes
            {
                let reason = if row.deadline_at.is_some_and(|deadline| deadline <= now) {
                    "absolute outcome deadline expired"
                } else {
                    "bounded wake delivery budget exhausted"
                };
                let outcome = serde_json::json!({
                    "reason": reason,
                    "evidence_refs": [format!("orgintel://opportunities/{}", row.id)]
                });
                sqlx::query(
                    "UPDATE opportunities SET state='blocked', outcome=$2, outcome_reason=$3, \
                       evidence_refs=$2->'evidence_refs', settled_at=$4, last_progress_at=$4, \
                       lease_owner=NULL, lease_expires_at=NULL, next_wake_at=NULL, revision=revision+1 \
                     WHERE id=$1",
                )
                .bind(row.id)
                .bind(&outcome)
                .bind(reason)
                .bind(now)
                .execute(&mut *tx)
                .await?;
                // The watcher must not silently close a business obligation.
                // Commit one addressed exception notice with the terminal
                // state so a crash cannot lose the only actionable alert.
                let room_id =
                    ensure_direct_message_room_in_tx(&mut tx, "daemon", Some(&row.actor_id))
                        .await?;
                let message_id = sqlx::query_scalar::<_, i64>(
                    "INSERT INTO messages (room_id,from_actor,to_actor,body) \
                     VALUES ($1,'daemon',$2,$3) RETURNING id",
                )
                .bind(room_id)
                .bind(&row.actor_id)
                .bind(format!(
                    "[OPPORTUNITY BLOCKED {}] {}. Inspect preserved Work and effects, then escalate one precise blocker. Do not replay external effects.",
                    row.id, reason
                ))
                .fetch_one(&mut *tx)
                .await?;
                results.push(OpportunityWake {
                    opportunity_id: row.id,
                    sequence: row.wake_count,
                    message_id: Some(message_id),
                    state: "blocked".into(),
                    settled: true,
                });
                continue;
            }
            let sequence = row.wake_count + 1;
            let room_id =
                ensure_direct_message_room_in_tx(&mut tx, "daemon", Some(&row.actor_id)).await?;
            let message_id = sqlx::query_scalar::<_, i64>(
                "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,'daemon',$2,$3) RETURNING id",
            )
            .bind(room_id)
            .bind(&row.actor_id)
            .bind(format!(
                "[OPPORTUNITY RECOVERY {} SEQUENCE {}] The prior wake was not acknowledged before its lease or delivery window expired. Resume this responsibility from its durable checkpoint; inspect current facts before acting.",
                row.id, sequence
            ))
            .fetch_one(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO opportunity_wakes (opportunity_id, sequence, message_id, reason) \
                 VALUES ($1,$2,$3,'expired lease or unacknowledged wake')",
            )
            .bind(row.id)
            .bind(sequence)
            .bind(message_id)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE opportunities SET state='queued', wake_count=$2, wake_message_id=$3, \
                   next_wake_at=$4, lease_owner=NULL, lease_expires_at=NULL, revision=revision+1 \
                 WHERE id=$1",
            )
            .bind(row.id)
            .bind(sequence)
            .bind(message_id)
            .bind(now + chrono::Duration::seconds(retry_after_seconds))
            .execute(&mut *tx)
            .await?;
            results.push(OpportunityWake {
                opportunity_id: row.id,
                sequence,
                message_id: Some(message_id),
                state: row.state,
                settled: false,
            });
        }
        tx.commit().await?;
        Ok(results)
    }

    /// Create an immutable responsibility version. Repeating the same exact
    /// write is idempotent; attempting to reuse a version with changed intent
    /// or policy is rejected.
    pub async fn put_responsibility_version(
        &self,
        responsibility_id: Uuid,
        version: i32,
        objective: &str,
        policy: serde_json::Value,
    ) -> Result<()> {
        if version <= 0 || objective.trim().is_empty() || !policy.is_object() {
            return Err(OrgIntelError::InvalidWork(
                "a responsibility version needs a positive version, objective, and policy object"
                    .into(),
            ));
        }
        opportunity_window_seconds(&policy)?;
        required_outcome_areas(&policy)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO responsibilities (id, current_version) VALUES ($1,$2) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(responsibility_id)
        .bind(version)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO responsibility_versions (responsibility_id, version, objective, policy) \
             VALUES ($1,$2,$3,$4) ON CONFLICT (responsibility_id, version) DO NOTHING",
        )
        .bind(responsibility_id)
        .bind(version)
        .bind(objective.trim())
        .bind(&policy)
        .execute(&mut *tx)
        .await?;
        let stored: (String, serde_json::Value) = sqlx::query_as(
            "SELECT objective, policy FROM responsibility_versions WHERE responsibility_id=$1 AND version=$2",
        )
        .bind(responsibility_id)
        .bind(version)
        .fetch_one(&mut *tx)
        .await?;
        if stored.0 != objective.trim() || stored.1 != policy {
            return Err(OrgIntelError::InvalidWork(
                "responsibility versions are immutable; use a new version".into(),
            ));
        }
        sqlx::query(
            "UPDATE responsibilities SET current_version=$2 WHERE id=$1 AND current_version < $2",
        )
        .bind(responsibility_id)
        .bind(version)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_responsibility_version(
        &self,
        responsibility_id: Uuid,
        version: i32,
    ) -> Result<Option<ResponsibilityVersionRow>> {
        Ok(sqlx::query_as::<_, ResponsibilityVersionRow>(
            "SELECT responsibility_id, version, objective, policy, created_at \
             FROM responsibility_versions WHERE responsibility_id=$1 AND version=$2",
        )
        .bind(responsibility_id)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Bind a not-yet-fired schedule to an immutable responsibility version.
    pub async fn bind_schedule_responsibility(
        &self,
        schedule_id: Uuid,
        responsibility_id: Uuid,
        version: i32,
    ) -> Result<()> {
        let updated = sqlx::query(
            "UPDATE schedules SET responsibility_id=$2, responsibility_version=$3 \
             WHERE id=$1 AND actor_id='exec' AND fired_at IS NULL AND cancelled_at IS NULL \
               AND EXISTS (SELECT 1 FROM responsibility_versions v \
                 WHERE v.responsibility_id=$2 AND v.version=$3)",
        )
        .bind(schedule_id)
        .bind(responsibility_id)
        .bind(version)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if updated != 1 {
            return Err(OrgIntelError::InvalidWork(
                "schedule must be live and responsibility version must exist".into(),
            ));
        }
        Ok(())
    }

    /// Claim an unresolved opportunity with a fenced lease. Expired leases can
    /// be reclaimed, incrementing the epoch so stale workers cannot settle it.
    pub async fn claim_opportunity(
        &self,
        opportunity_id: Uuid,
        worker_id: &str,
        lease_seconds: i64,
        now: DateTime<Utc>,
    ) -> Result<Option<OpportunityClaim>> {
        if worker_id.trim().is_empty() || lease_seconds <= 0 {
            return Err(OrgIntelError::InvalidWork(
                "opportunity claims need a worker id and positive lease".into(),
            ));
        }
        Ok(sqlx::query_as::<_, OpportunityClaim>(
            "UPDATE opportunities SET lease_owner=$2, lease_expires_at=$3, state='inspecting', \
               next_wake_at=NULL, owner_epoch=owner_epoch+1, revision=revision+1 \
             WHERE id=$1 AND actor_id=$2 AND state NOT IN ('completed','needs_human','blocked','cancelled') \
               AND (lease_expires_at IS NULL OR lease_expires_at <= $4) \
               AND (state <> 'waiting_retry' OR next_wake_at <= $4) \
               AND (deadline_at IS NULL OR deadline_at > $4) \
             RETURNING id AS opportunity_id, owner_epoch, lease_owner, lease_expires_at, state, revision",
        )
        .bind(opportunity_id)
        .bind(worker_id.trim())
        .bind(now + chrono::Duration::seconds(lease_seconds))
        .bind(now)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Settle or advance an opportunity only for the current fenced owner.
    /// Terminal outcomes are immutable and require an evidence-bearing object.
    pub async fn settle_opportunity(
        &self,
        opportunity_id: Uuid,
        owner_epoch: i64,
        state: &str,
        outcome: serde_json::Value,
        now: DateTime<Utc>,
    ) -> Result<OpportunityRow> {
        let terminal = matches!(state, "completed" | "needs_human" | "blocked" | "cancelled");
        let outcome_reason = outcome
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let evidence_refs = outcome.get("evidence_refs").cloned();
        if !matches!(
            state,
            "queued"
                | "inspecting"
                | "executing"
                | "verifying"
                | "recovering"
                | "waiting_retry"
                | "completed"
                | "needs_human"
                | "blocked"
                | "cancelled"
        ) || !outcome.is_object()
            || (terminal
                && (outcome
                    .get("evidence_refs")
                    .and_then(serde_json::Value::as_array)
                    .is_none_or(|refs| refs.is_empty())
                    || outcome
                        .get("reason")
                        .and_then(serde_json::Value::as_str)
                        .is_none_or(str::is_empty)))
        {
            return Err(OrgIntelError::InvalidWork(
                "opportunity state or evidence-bearing outcome is invalid".into(),
            ));
        }
        if state == "waiting_retry"
            && outcome
                .get("next_wake_at")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .is_none_or(|value| value.with_timezone(&Utc) <= now)
        {
            return Err(OrgIntelError::InvalidWork(
                "waiting_retry requires a future RFC3339 next_wake_at".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let responsibility_policy = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT rv.policy FROM opportunities o \
               JOIN responsibility_versions rv ON rv.responsibility_id=o.responsibility_id \
                 AND rv.version=o.responsibility_version \
             WHERE o.id=$1 AND o.owner_epoch=$2 \
               AND o.lease_expires_at > $3 AND o.state NOT IN ('completed','needs_human','blocked','cancelled') \
             FOR UPDATE OF o",
        )
        .bind(opportunity_id)
        .bind(owner_epoch)
        .bind(now)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(responsibility_policy) = responsibility_policy else {
            return Err(OrgIntelError::InvalidWork(
                "opportunity claim expired, was superseded, or already settled".into(),
            ));
        };
        let required_areas = required_outcome_areas(&responsibility_policy)?;
        if state == "completed" {
            if !required_areas.is_empty() {
                let area_evidence = outcome
                    .get("area_evidence")
                    .and_then(serde_json::Value::as_array)
                    .ok_or_else(|| {
                        OrgIntelError::InvalidWork(
                            "completed responsibility needs area evidence".into(),
                        )
                    })?;
                if area_evidence.len() != required_areas.len() {
                    return Err(OrgIntelError::InvalidWork(
                        "completed responsibility needs one linked Work for each required outcome area".into(),
                    ));
                }
                let mut seen_areas = std::collections::HashSet::new();
                let mut seen_work = std::collections::HashSet::new();
                for entry in area_evidence {
                    let area = entry
                        .get("area")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            OrgIntelError::InvalidWork("area evidence needs an area".into())
                        })?;
                    let work_id = entry
                        .get("work_id")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|value| Uuid::parse_str(value).ok())
                        .ok_or_else(|| {
                            OrgIntelError::InvalidWork("area evidence needs a Work UUID".into())
                        })?;
                    if !required_areas.iter().any(|required| required == area)
                        || !seen_areas.insert(area)
                        || !seen_work.insert(work_id)
                    {
                        return Err(OrgIntelError::InvalidWork(
                            "area evidence must cover each required area with distinct Work".into(),
                        ));
                    }
                    let completed: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM opportunity_work ow JOIN work w ON w.id=ow.work_id \
                           WHERE ow.opportunity_id=$1 AND ow.work_id=$2 AND w.status='completed')",
                    )
                    .bind(opportunity_id)
                    .bind(work_id)
                    .fetch_one(&mut *tx)
                    .await?;
                    if !completed {
                        return Err(OrgIntelError::InvalidWork(
                            "area evidence must name completed Work linked to this Opportunity"
                                .into(),
                        ));
                    }
                }
            }
        }
        if terminal {
            let refs = evidence_refs
                .as_ref()
                .and_then(serde_json::Value::as_array)
                .expect("validated terminal evidence array");
            let mut grounded_completion_evidence = false;
            let mut pending_human_handoff = false;
            for evidence in refs {
                let kind = evidence.get("kind").and_then(serde_json::Value::as_str);
                let id = evidence
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|value| Uuid::parse_str(value).ok());
                let exists = match (kind, id) {
                    (Some("artifact_ref"), Some(id)) => {
                        let exists: bool = sqlx::query_scalar(
                            "SELECT EXISTS (SELECT 1 FROM artifact_refs ar WHERE ar.id=$1 \
                               AND ar.state='available' AND ar.work_id IS NOT NULL AND EXISTS ( \
                                 SELECT 1 FROM opportunity_work ow JOIN work w ON w.id=ow.work_id \
                                 WHERE ow.opportunity_id=$2 AND ow.work_id=ar.work_id AND w.status='completed'))",
                        )
                        .bind(id)
                        .bind(opportunity_id)
                        .fetch_one(&mut *tx)
                        .await?;
                        grounded_completion_evidence |= exists;
                        exists
                    }
                    (Some("work"), Some(id)) => {
                        let is_completed: bool = sqlx::query_scalar(
                            "SELECT EXISTS (SELECT 1 FROM opportunity_work ow JOIN work w ON w.id=ow.work_id \
                               WHERE ow.opportunity_id=$1 AND ow.work_id=$2 AND w.status='completed')",
                        )
                        .bind(opportunity_id)
                        .bind(id)
                        .fetch_one(&mut *tx)
                        .await?;
                        grounded_completion_evidence |= is_completed;
                        is_completed || sqlx::query_scalar::<_, bool>(
                            "SELECT EXISTS (SELECT 1 FROM opportunity_work WHERE opportunity_id=$1 AND work_id=$2)",
                        )
                        .bind(opportunity_id)
                        .bind(id)
                        .fetch_one(&mut *tx)
                        .await?
                    }
                    (Some("handoff"), Some(id)) => {
                        let handoff_state: Option<String> = sqlx::query_scalar(
                            "SELECT h.state::text FROM owner_handoffs h JOIN opportunity_work ow ON ow.work_id=h.work_id \
                               WHERE ow.opportunity_id=$1 AND h.id=$2",
                        )
                        .bind(opportunity_id)
                        .bind(id)
                        .fetch_optional(&mut *tx)
                        .await?;
                        pending_human_handoff |= handoff_state
                            .as_deref()
                            .is_some_and(|state| state == "pending" || state == "preparing");
                        handoff_state.is_some()
                    }
                    (Some("schedule_occurrence"), _) => {
                        let schedule_id = evidence
                            .get("schedule_id")
                            .and_then(serde_json::Value::as_str)
                            .and_then(|value| Uuid::parse_str(value).ok());
                        let scheduled_for = evidence
                            .get("scheduled_for")
                            .and_then(serde_json::Value::as_str)
                            .and_then(|value| DateTime::parse_from_rfc3339(value).ok());
                        if let (Some(schedule_id), Some(scheduled_for)) =
                            (schedule_id, scheduled_for)
                        {
                            sqlx::query_scalar::<_, bool>(
                                "SELECT EXISTS (SELECT 1 FROM schedule_occurrences WHERE opportunity_id=$1 \
                                   AND schedule_id=$2 AND scheduled_for=$3)",
                            )
                            .bind(opportunity_id)
                            .bind(schedule_id)
                            .bind(scheduled_for.with_timezone(&Utc))
                            .fetch_one(&mut *tx)
                            .await?
                        } else {
                            false
                        }
                    }
                    (Some("admin_action"), _) if state == "cancelled" => evidence
                        .get("actor")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|actor| !actor.trim().is_empty()),
                    _ => false,
                };
                if !exists {
                    return Err(OrgIntelError::InvalidWork(
                        "terminal outcome references missing or unrelated evidence".into(),
                    ));
                }
            }
            if state == "completed" && !grounded_completion_evidence {
                return Err(OrgIntelError::InvalidWork(
                    "completed requires an available linked ArtifactRef or a completed linked Work"
                        .into(),
                ));
            }
            if state == "needs_human" && !required_areas.is_empty() && !pending_human_handoff {
                return Err(OrgIntelError::InvalidWork(
                    "needs_human requires a pending owner handoff linked to this Opportunity"
                        .into(),
                ));
            }
        }
        let row = sqlx::query_as::<_, OpportunityRow>(
            "UPDATE opportunities SET state=$3, outcome=$4, outcome_reason=$7, evidence_refs=COALESCE($8,'[]'::jsonb), revision=revision+1, \
               last_progress_at=$5, settled_at=CASE WHEN $6 THEN $5 ELSE NULL END, \
               next_wake_at=CASE WHEN $3='waiting_retry' THEN ($4->>'next_wake_at')::timestamptz ELSE next_wake_at END, \
               lease_owner=CASE WHEN $6 OR $3 IN ('queued','waiting_retry') THEN NULL ELSE lease_owner END, \
               lease_expires_at=CASE WHEN $6 OR $3 IN ('queued','waiting_retry') THEN NULL ELSE lease_expires_at END \
             WHERE id=$1 AND owner_epoch=$2 AND lease_expires_at > $5 \
               AND state NOT IN ('completed','needs_human','blocked','cancelled') \
             RETURNING id, actor_id, responsibility_id, responsibility_version, state, outcome, outcome_reason, evidence_refs, revision, \
               owner_epoch, lease_owner, lease_expires_at, next_wake_at, deadline_at, \
               last_progress_at, wake_message_id, wake_count, created_at, settled_at",
        )
        .bind(opportunity_id)
        .bind(owner_epoch)
        .bind(state)
        .bind(outcome)
        .bind(now)
        .bind(terminal)
        .bind(outcome_reason)
        .bind(evidence_refs)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork(
            "opportunity claim expired, was superseded, or already settled".into(),
        ))?;
        tx.commit().await?;
        Ok(row)
    }
}

impl OrgIntel {
    /// Create a weekday schedule and its immutable responsibility version in
    /// one transaction. Repeating the exact request is idempotent; an
    /// existing live schedule with a different binding is left untouched and
    /// rejected.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_weekday_schedule_with_responsibility(
        &self,
        actor_id: &str,
        reason: &str,
        local_time: NaiveTime,
        timezone: &str,
        after: DateTime<Utc>,
        missed_policy: &str,
        catch_up_grace_seconds: Option<i64>,
        machine_requirement: &str,
        responsibility_id: Uuid,
        version: i32,
        objective: &str,
        policy: serde_json::Value,
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
        if version <= 0 || objective.trim().is_empty() || !policy.is_object() {
            return Err(OrgIntelError::InvalidWork(
                "a responsibility version needs a positive version, objective, and policy object"
                    .into(),
            ));
        }
        opportunity_window_seconds(&policy)?;
        let fire_at = next_weekday_fire(after, local_time, timezone)?;
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO responsibilities (id, current_version) VALUES ($1,$2) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(responsibility_id)
        .bind(version)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO responsibility_versions (responsibility_id, version, objective, policy) \
             VALUES ($1,$2,$3,$4) ON CONFLICT (responsibility_id, version) DO NOTHING",
        )
        .bind(responsibility_id)
        .bind(version)
        .bind(objective.trim())
        .bind(&policy)
        .execute(&mut *tx)
        .await?;
        let stored: (String, serde_json::Value) = sqlx::query_as(
            "SELECT objective, policy FROM responsibility_versions WHERE responsibility_id=$1 AND version=$2",
        )
        .bind(responsibility_id)
        .bind(version)
        .fetch_one(&mut *tx)
        .await?;
        if stored.0 != objective.trim() || stored.1 != policy {
            return Err(OrgIntelError::InvalidWork(
                "responsibility versions are immutable; use a new version".into(),
            ));
        }
        sqlx::query(
            "UPDATE responsibilities SET current_version=$2 WHERE id=$1 AND current_version < $2",
        )
        .bind(responsibility_id)
        .bind(version)
        .execute(&mut *tx)
        .await?;

        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO schedules (id, actor_id, reason, fire_at, recurrence, timezone, local_time, missed_policy, catch_up_grace_seconds, machine_requirement, responsibility_id, responsibility_version) \
             VALUES ($1,$2,$3,$4,'weekdays',$5,$6,$7,$8,$9,$10,$11) \
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(actor_id)
        .bind(reason)
        .bind(fire_at)
        .bind(timezone)
        .bind(local_time)
        .bind(missed_policy)
        .bind(catch_up_grace_seconds)
        .bind(machine_requirement)
        .bind(responsibility_id)
        .bind(version)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(id) = inserted {
            tx.commit().await?;
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
        .fetch_one(&mut *tx)
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
        if existing.responsibility_id != Some(responsibility_id)
            || existing.responsibility_version != Some(version)
        {
            return Err(OrgIntelError::InvalidWork(
                "the recurring schedule already exists with a different responsibility binding; cancel it before creating a replacement".into(),
            ));
        }
        tx.commit().await?;
        Ok((existing.id, existing.fire_at, false))
    }

    /// Create an interval schedule and responsibility binding atomically.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_interval_schedule_with_responsibility(
        &self,
        actor_id: &str,
        reason: &str,
        interval_seconds: i32,
        after: DateTime<Utc>,
        responsibility_id: Uuid,
        version: i32,
        objective: &str,
        policy: serde_json::Value,
    ) -> Result<(Uuid, DateTime<Utc>, bool)> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "a recurring time opportunity needs a reason".into(),
            ));
        }
        if !(MIN_INTERVAL_SECONDS..=MAX_INTERVAL_SECONDS).contains(&interval_seconds) {
            return Err(OrgIntelError::InvalidWork(format!(
                "an interval must be between {} minutes and {} days",
                MIN_INTERVAL_SECONDS / 60,
                MAX_INTERVAL_SECONDS / 86_400
            )));
        }
        if version <= 0 || objective.trim().is_empty() || !policy.is_object() {
            return Err(OrgIntelError::InvalidWork(
                "a responsibility version needs a positive version, objective, and policy object"
                    .into(),
            ));
        }
        opportunity_window_seconds(&policy)?;
        let fire_at = after + chrono::Duration::seconds(i64::from(interval_seconds));
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO responsibilities (id, current_version) VALUES ($1,$2) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(responsibility_id)
        .bind(version)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO responsibility_versions (responsibility_id, version, objective, policy) \
             VALUES ($1,$2,$3,$4) ON CONFLICT (responsibility_id, version) DO NOTHING",
        )
        .bind(responsibility_id)
        .bind(version)
        .bind(objective.trim())
        .bind(&policy)
        .execute(&mut *tx)
        .await?;
        let stored: (String, serde_json::Value) = sqlx::query_as(
            "SELECT objective, policy FROM responsibility_versions WHERE responsibility_id=$1 AND version=$2",
        )
        .bind(responsibility_id)
        .bind(version)
        .fetch_one(&mut *tx)
        .await?;
        if stored.0 != objective.trim() || stored.1 != policy {
            return Err(OrgIntelError::InvalidWork(
                "responsibility versions are immutable; use a new version".into(),
            ));
        }
        sqlx::query(
            "UPDATE responsibilities SET current_version=$2 WHERE id=$1 AND current_version < $2",
        )
        .bind(responsibility_id)
        .bind(version)
        .execute(&mut *tx)
        .await?;
        let inserted = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO schedules (id, actor_id, reason, fire_at, recurrence, interval_seconds, missed_policy, catch_up_grace_seconds, machine_requirement, responsibility_id, responsibility_version) \
             VALUES ($1,$2,$3,$4,'interval',$5,'catch_up_once',$6,'local_mac',$7,$8) \
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(actor_id)
        .bind(reason)
        .bind(fire_at)
        .bind(interval_seconds)
        .bind(i64::from(interval_seconds))
        .bind(responsibility_id)
        .bind(version)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(id) = inserted {
            tx.commit().await?;
            return Ok((id, fire_at, true));
        }
        let existing = sqlx::query_as::<_, ScheduleRow>(&format!(
            "SELECT {SCHEDULE_COLUMNS} FROM schedules \
             WHERE actor_id=$1 AND recurrence='interval' AND interval_seconds=$2 \
               AND reason=$3 AND cancelled_at IS NULL"
        ))
        .bind(actor_id)
        .bind(interval_seconds)
        .bind(reason)
        .fetch_one(&mut *tx)
        .await?;
        if existing.missed_policy != "catch_up_once"
            || existing.catch_up_grace_seconds != Some(i64::from(interval_seconds))
            || existing.machine_requirement != "local_mac"
        {
            return Err(OrgIntelError::InvalidWork(
                "the recurring interval already exists with a different schedule policy".into(),
            ));
        }
        if existing.responsibility_id != Some(responsibility_id)
            || existing.responsibility_version != Some(version)
        {
            return Err(OrgIntelError::InvalidWork(
                "the recurring interval already exists with a different responsibility binding; cancel it before creating a replacement".into(),
            ));
        }
        tx.commit().await?;
        Ok((existing.id, existing.fire_at, false))
    }

    /// Create a free-standing one-shot actor wake and its immutable
    /// responsibility version atomically. The exact `(actor, reason,
    /// fire_at)` request is the idempotency key. Repeating it with the same
    /// responsibility and policy returns the existing schedule, including
    /// after it has fired; changing its intent is rejected.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_exact_schedule_with_responsibility(
        &self,
        actor_id: &str,
        reason: &str,
        fire_at: DateTime<Utc>,
        machine_requirement: &str,
        responsibility_id: Uuid,
        version: i32,
        objective: &str,
        policy: serde_json::Value,
    ) -> Result<(Uuid, bool)> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "a time opportunity needs a reason".into(),
            ));
        }
        if !matches!(machine_requirement, "local_mac" | "always_on") {
            return Err(OrgIntelError::InvalidWork(
                "machine requirement must be local_mac|always_on".into(),
            ));
        }
        if version <= 0 || objective.trim().is_empty() || !policy.is_object() {
            return Err(OrgIntelError::InvalidWork(
                "a responsibility version needs a positive version, objective, and policy object"
                    .into(),
            ));
        }
        opportunity_window_seconds(&policy)?;

        let mut tx = self.pool.begin().await?;
        // This serializes concurrent retries with the same natural key. The
        // hash is only a lock bucket; schedule identity is checked below.
        let lock_key =
            serde_json::json!([actor_id, reason.trim(), fire_at.to_rfc3339()]).to_string();
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(lock_key)
            .execute(&mut *tx)
            .await?;

        if let Some(existing) = sqlx::query_as::<_, ScheduleRow>(&format!(
            "SELECT {SCHEDULE_COLUMNS} FROM schedules \
             WHERE actor_id=$1 AND reason=$2 AND fire_at=$3 AND recurrence IS NULL \
               AND work_id IS NULL ORDER BY created_at LIMIT 1"
        ))
        .bind(actor_id)
        .bind(reason.trim())
        .bind(fire_at)
        .fetch_optional(&mut *tx)
        .await?
        {
            if existing.machine_requirement != machine_requirement
                || existing.responsibility_id != Some(responsibility_id)
                || existing.responsibility_version != Some(version)
            {
                return Err(OrgIntelError::InvalidWork(
                    "the exact schedule already exists with different responsibility intent; cancel it before creating a replacement".into(),
                ));
            }
            let stored = sqlx::query_as::<_, (String, serde_json::Value)>(
                "SELECT objective, policy FROM responsibility_versions \
                 WHERE responsibility_id=$1 AND version=$2",
            )
            .bind(responsibility_id)
            .bind(version)
            .fetch_one(&mut *tx)
            .await?;
            if stored.0 != objective.trim() || stored.1 != policy {
                return Err(OrgIntelError::InvalidWork(
                    "responsibility versions are immutable; use a new version".into(),
                ));
            }
            if existing.cancelled_at.is_some() {
                return Err(OrgIntelError::InvalidWork(
                    "the exact schedule was cancelled; create a new time opportunity with a new reason or instant".into(),
                ));
            }
            tx.commit().await?;
            return Ok((existing.id, false));
        }

        sqlx::query(
            "INSERT INTO responsibilities (id, current_version) VALUES ($1,$2) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(responsibility_id)
        .bind(version)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO responsibility_versions (responsibility_id, version, objective, policy) \
             VALUES ($1,$2,$3,$4) ON CONFLICT (responsibility_id, version) DO NOTHING",
        )
        .bind(responsibility_id)
        .bind(version)
        .bind(objective.trim())
        .bind(&policy)
        .execute(&mut *tx)
        .await?;
        let stored = sqlx::query_as::<_, (String, serde_json::Value)>(
            "SELECT objective, policy FROM responsibility_versions \
             WHERE responsibility_id=$1 AND version=$2",
        )
        .bind(responsibility_id)
        .bind(version)
        .fetch_one(&mut *tx)
        .await?;
        if stored.0 != objective.trim() || stored.1 != policy {
            return Err(OrgIntelError::InvalidWork(
                "responsibility versions are immutable; use a new version".into(),
            ));
        }
        sqlx::query(
            "UPDATE responsibilities SET current_version=$2 \
             WHERE id=$1 AND current_version < $2",
        )
        .bind(responsibility_id)
        .bind(version)
        .execute(&mut *tx)
        .await?;

        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO schedules \
             (id, actor_id, reason, fire_at, missed_policy, catch_up_grace_seconds, \
              machine_requirement, responsibility_id, responsibility_version) \
             VALUES ($1,$2,$3,$4,'skip',NULL,$5,$6,$7)",
        )
        .bind(id)
        .bind(actor_id)
        .bind(reason.trim())
        .bind(fire_at)
        .bind(machine_requirement)
        .bind(responsibility_id)
        .bind(version)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((id, true))
    }

    /// Release one existing Work item at an exact time. Free-standing actor
    /// wakes use `create_exact_schedule_with_responsibility` instead.
    pub async fn add_work_release_schedule(
        &self,
        actor_id: &str,
        work_id: Uuid,
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
        // A time dependency may pause unfinished Work, but must never reopen
        // historical Work or leave a schedule pointing at a missing Work row.
        let paused = sqlx::query(
            "UPDATE work SET status='blocked', resolution=$2 WHERE id=$1 \
             AND status IN ('proposed','active','blocked')",
        )
        .bind(work_id)
        .bind(format!("waiting for schedule {id}: {reason}"))
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if paused != 1 {
            return Err(OrgIntelError::InvalidWork(
                "a Work time dependency requires existing, unsettled Work".into(),
            ));
        }
        // Misfire policies apply only to recurring schedules. Keep one-shot
        // rows inside the durable schedule constraint without implying a
        // catch-up window that is never consulted for them.
        sqlx::query(
            "INSERT INTO schedules \
             (id, actor_id, work_id, reason, fire_at, missed_policy, catch_up_grace_seconds) \
             VALUES ($1,$2,$3,$4,$5,'skip',NULL)",
        )
        .bind(id)
        .bind(actor_id)
        .bind(work_id)
        .bind(reason)
        .bind(fire_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
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

    pub async fn next_opportunity_due_at(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            "SELECT min(CASE \
               WHEN deadline_at IS NOT NULL \
                 AND (CASE WHEN lease_expires_at IS NOT NULL THEN lease_expires_at ELSE next_wake_at END IS NULL \
                   OR deadline_at < CASE WHEN lease_expires_at IS NOT NULL THEN lease_expires_at ELSE next_wake_at END) THEN deadline_at \
               ELSE CASE WHEN lease_expires_at IS NOT NULL THEN lease_expires_at ELSE next_wake_at END END) \
             FROM opportunities WHERE state NOT IN ('completed','needs_human','blocked','cancelled')",
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
                    if let (Some(responsibility_id), Some(version)) =
                        (row.responsibility_id, row.responsibility_version)
                    {
                        sqlx::query(
                            "UPDATE schedule_occurrences SET responsibility_id=$3, responsibility_version=$4, admission='skipped' \
                             WHERE schedule_id=$1 AND scheduled_for=$2",
                        )
                        .bind(row.id)
                        .bind(original_fire_at)
                        .bind(responsibility_id)
                        .bind(version)
                        .execute(&mut *tx)
                        .await?;
                    }
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
            if matches!(row.recurrence.as_deref(), Some("weekdays" | "interval")) {
                let next = match row.recurrence.as_deref() {
                    Some("interval") => next_interval_fire(row.fire_at, now, row.interval_seconds)?,
                    _ => {
                        let timezone = row.timezone.as_deref().ok_or_else(|| {
                            OrgIntelError::InvalidWork(
                                "weekday schedule is missing timezone".into(),
                            )
                        })?;
                        let local_time = row.local_time.ok_or_else(|| {
                            OrgIntelError::InvalidWork(
                                "weekday schedule is missing local time".into(),
                            )
                        })?;
                        next_weekday_fire(now, local_time, timezone)?
                    }
                };
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
                if row.responsibility_id.is_some() {
                    sqlx::query(
                        "UPDATE schedule_occurrences SET admission='skipped', responsibility_id=$3, responsibility_version=$4 \
                         WHERE schedule_id=$1 AND scheduled_for=$2",
                    )
                    .bind(row.id)
                    .bind(row.fire_at)
                    .bind(row.responsibility_id)
                    .bind(row.responsibility_version)
                    .execute(&mut *tx)
                    .await?;
                }
                continue;
            }
            let admitted_opportunity = if let (Some(responsibility_id), Some(version)) =
                (row.responsibility_id, row.responsibility_version)
            {
                let policy = sqlx::query_scalar::<_, serde_json::Value>(
                    "SELECT policy FROM responsibility_versions WHERE responsibility_id=$1 AND version=$2",
                )
                .bind(responsibility_id)
                .bind(version)
                .fetch_one(&mut *tx)
                .await?;
                let deadline_at =
                    now + chrono::Duration::seconds(opportunity_window_seconds(&policy)?);
                let candidate_id = Uuid::new_v4();
                let created = sqlx::query_scalar::<_, Uuid>(
                    "INSERT INTO opportunities (id, actor_id, responsibility_id, responsibility_version, next_wake_at, deadline_at) \
                     VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING RETURNING id",
                )
                .bind(candidate_id)
                .bind(&row.actor_id)
                .bind(responsibility_id)
                .bind(version)
                .bind(now)
                .bind(deadline_at)
                .fetch_optional(&mut *tx)
                .await?;
                let (opportunity_id, is_new) = match created {
                    Some(id) => (id, true),
                    None => {
                        let id = sqlx::query_scalar::<_, Uuid>(
                            "SELECT id FROM opportunities WHERE responsibility_id=$1 \
                             AND state NOT IN ('completed','needs_human','blocked','cancelled') \
                             ORDER BY created_at LIMIT 1",
                        )
                        .bind(responsibility_id)
                        .fetch_one(&mut *tx)
                        .await?;
                        (id, false)
                    }
                };
                sqlx::query(
                    "UPDATE opportunities SET next_wake_at=LEAST(COALESCE(next_wake_at,$2),$2), \
                       revision=revision+CASE WHEN $3 THEN 0 ELSE 1 END \
                     WHERE id=$1",
                )
                .bind(opportunity_id)
                .bind(now)
                .bind(is_new)
                .execute(&mut *tx)
                .await?;
                sqlx::query(
                    "UPDATE schedule_occurrences SET opportunity_id=$3, responsibility_id=$4, \
                       responsibility_version=$5, admission=$6 \
                     WHERE schedule_id=$1 AND scheduled_for=$2",
                )
                .bind(row.id)
                .bind(row.fire_at)
                .bind(opportunity_id)
                .bind(responsibility_id)
                .bind(version)
                .bind(if is_new { "admitted" } else { "coalesced" })
                .execute(&mut *tx)
                .await?;
                if let Some(work_id) = row.work_id {
                    let has_primary: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM opportunity_work WHERE opportunity_id=$1 AND relation='primary')",
                    )
                    .bind(opportunity_id)
                    .fetch_one(&mut *tx)
                    .await?;
                    sqlx::query(
                        "INSERT INTO opportunity_work (opportunity_id, work_id, relation) \
                         VALUES ($1,$2,$3) ON CONFLICT (opportunity_id, work_id) DO NOTHING",
                    )
                    .bind(opportunity_id)
                    .bind(work_id)
                    .bind(if has_primary { "supporting" } else { "primary" })
                    .execute(&mut *tx)
                    .await?;
                }
                Some((opportunity_id, is_new))
            } else {
                None
            };
            let released_work = if let Some(work_id) = row.work_id {
                sqlx::query(
                    "UPDATE work SET status='active', resolution='time condition reached' \
                     WHERE id=$1 AND resolution LIKE $2 \
                       AND NOT EXISTS (SELECT 1 FROM owner_handoffs h WHERE h.work_id=$1 AND h.state='pending') \
                       AND NOT EXISTS (SELECT 1 FROM schedules s WHERE s.work_id=$1 \
                         AND s.id<>$3 AND s.fired_at IS NULL AND s.cancelled_at IS NULL)",
                )
                .bind(work_id)
                .bind(format!("waiting for schedule {}:%", row.id))
                .bind(row.id)
                .execute(&mut *tx)
                .await?.rows_affected() > 0
            } else {
                false
            };
            if !released_work || admitted_opportunity.is_some() {
                // Consume the time fact and create its recoverable actor wake
                // in one transaction. A crash can therefore leave both
                // pending or neither, never a fired schedule with no delivery.
                // Exec uses this same durable inbox: an in-memory wake after
                // commit can be lost on restart or while Exec is in backoff.
                // Only a successfully released Work uses the deterministic
                // kickoff. A pending human step still needs its scheduled
                // observer to inspect consent/expiry; otherwise the time fact
                // would be consumed without waking anyone.
                let room_id =
                    ensure_direct_message_room_in_tx(&mut tx, "daemon", Some(&row.actor_id))
                        .await?;
                let wake_message_id = sqlx::query_scalar::<_, i64>(
                    "INSERT INTO messages (room_id,from_actor,to_actor,body) \
                     VALUES ($1,'daemon',$2,$3) RETURNING id",
                )
                .bind(room_id)
                .bind(&row.actor_id)
                .bind(format!(
                    "[SCHEDULE DUE {} AT {}] {}{}{}\n\nThis is a time-based opportunity to inspect current facts. It is not evidence that production is necessary or complete.",
                    row.id, row.fire_at, row.reason,
                    row.work_id.map(|id| format!("\nLinked Work: {id}")).unwrap_or_default(),
                    admitted_opportunity.map(|(id, _)| format!("\nOpportunity: {id}")).unwrap_or_default()
                ))
                .fetch_one(&mut *tx)
                .await?;
                if let Some((opportunity_id, _)) = admitted_opportunity {
                    sqlx::query(
                        "UPDATE opportunities SET wake_message_id=COALESCE(wake_message_id,$2), \
                           wake_count=CASE WHEN wake_count=0 THEN 1 ELSE wake_count END, \
                           next_wake_at=CASE WHEN wake_count=0 THEN $3 ELSE next_wake_at END \
                         WHERE id=$1",
                    )
                    .bind(opportunity_id)
                    .bind(wake_message_id)
                    .bind(now + chrono::Duration::seconds(OPPORTUNITY_WAKE_ACK_SECONDS))
                    .execute(&mut *tx)
                    .await?;
                    sqlx::query(
                        "INSERT INTO opportunity_wakes (opportunity_id, sequence, message_id, reason) \
                         SELECT $1,1,$2,'scheduled occurrence admitted' WHERE \
                           (SELECT wake_count FROM opportunities WHERE id=$1)=1 \
                         ON CONFLICT DO NOTHING",
                    )
                    .bind(opportunity_id)
                    .bind(wake_message_id)
                    .execute(&mut *tx)
                    .await?;
                    sqlx::query(
                        "UPDATE schedule_occurrences SET wake_message_id=$3 \
                         WHERE schedule_id=$1 AND scheduled_for=$2",
                    )
                    .bind(row.id)
                    .bind(row.fire_at)
                    .bind(wake_message_id)
                    .execute(&mut *tx)
                    .await?;
                }
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
        let room_id = ensure_direct_message_room_in_tx(&mut tx, "daemon", Some(actor_id)).await?;
        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (room_id,from_actor,to_actor,body) \
             VALUES ($1,'daemon',$2,$3) RETURNING id",
        )
        .bind(room_id)
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
        let room_id = ensure_direct_message_room_in_tx(&mut tx, "daemon", Some(actor_id)).await?;
        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (room_id,from_actor,to_actor,body) \
             VALUES ($1,'daemon',$2,$3) RETURNING id",
        )
        .bind(room_id)
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
