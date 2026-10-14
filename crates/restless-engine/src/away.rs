//! The owner's away mode: one setting with an end date. While it holds, the schedules the owner
//! chose stay paused, an admin they name may answer single approvals, and Telegram carries only
//! what blocks the company. It ends when the owner is back or the date passes; the schedules it
//! paused, and only those, resume.
//!
//! The state is two Authority records, so who went away, until when and with which deputy stays
//! in the governance record. Pausing a schedule is the ordinary OrgIntel schedule change.

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::authority::AuthorityStore;

pub const AWAY_STARTED_KIND: &str = "away_started";
pub const AWAY_ENDED_KIND: &str = "away_ended";
const LONGEST_AWAY_DAYS: i64 = 92;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Away {
    pub until: DateTime<Utc>,
    /// An admin who may answer single approvals while the owner is away.
    #[serde(default)]
    pub deputy: Option<String>,
    /// The schedules this away paused; ending it resumes exactly these.
    #[serde(default)]
    pub paused_schedules: Vec<Uuid>,
    #[serde(default)]
    pub since: Option<DateTime<Utc>>,
}

/// The away mode in force, if any: started, not ended, and its date not yet passed.
pub async fn current(authority: &AuthorityStore, company: &str) -> Result<Option<Away>> {
    Ok(recorded(authority, company)
        .await?
        .filter(|away| away.until > Utc::now()))
}

/// The latest away that was started and not ended, whether or not its date has passed.
async fn recorded(authority: &AuthorityStore, company: &str) -> Result<Option<Away>> {
    let started = authority
        .recent_records_of_kind(company, AWAY_STARTED_KIND, 1)
        .await?
        .pop();
    let ended = authority
        .recent_records_of_kind(company, AWAY_ENDED_KIND, 1)
        .await?
        .pop();
    Ok(match (started, ended) {
        (Some(started), ended) if ended.as_ref().is_none_or(|ended| ended.id < started.id) => {
            Some(serde_json::from_value(started.body).context("decode away mode")?)
        }
        _ => None,
    })
}

/// Whether `actor` may answer single approvals now, as the away owner's deputy.
pub async fn is_deputy(authority: &AuthorityStore, company: &str, actor: &str) -> bool {
    matches!(
        current(authority, company).await,
        Ok(Some(Away { deputy: Some(deputy), .. })) if deputy == actor
    )
}

/// Whether this person may answer an approval as the away owner's deputy: an admin, named as
/// the deputy of an away in force, for a single decision (allow once or decline). Standing
/// permission and revocation are never a deputy's.
pub async fn deputy_may_decide(
    authority: &AuthorityStore,
    company: &str,
    role: &str,
    actor: &str,
    standing: bool,
) -> bool {
    role == "admin" && !standing && is_deputy(authority, company, actor).await
}

/// Go away until `until`: pause the chosen live schedules and name an optional admin deputy.
/// A new away replaces the one in force, resuming what that one paused first.
pub async fn start(
    authority: &AuthorityStore,
    org: &restless_orgintel::OrgIntel,
    company: &str,
    owner: &str,
    until: DateTime<Utc>,
    deputy: Option<String>,
    pause: Vec<Uuid>,
) -> Result<Away> {
    let now = Utc::now();
    if until <= now || until > now + Duration::days(LONGEST_AWAY_DAYS) {
        bail!("choose a return date in the next three months");
    }
    let deputy = deputy.map(|deputy| deputy.trim().to_string()).filter(|deputy| !deputy.is_empty());
    if let Some(deputy) = deputy.as_deref() {
        let admin = org.human_members().await?.into_iter().any(|member| {
            member.actor_id == deputy
                && member.membership_role == "admin"
                && member.membership_status == "active"
        });
        if !admin {
            bail!("the deputy must be an active admin of this company");
        }
    }
    if recorded(authority, company).await?.is_some() {
        end(authority, org, company, owner).await?;
    }
    let mut paused = Vec::new();
    for id in pause {
        let Some(schedule) = org.get_schedule(id).await? else {
            continue;
        };
        if schedule.paused_at.is_some() || schedule.recurrence.is_none() {
            continue;
        }
        if set_paused(org, &schedule, true).await? {
            paused.push(id);
        }
    }
    let away = Away {
        until,
        deputy,
        paused_schedules: paused,
        since: Some(now),
    };
    authority
        .emit(company, AWAY_STARTED_KIND, Some(owner), serde_json::to_value(&away)?)
        .await?;
    Ok(away)
}

/// The owner is back, or the date passed: resume exactly the schedules this away paused.
pub async fn end(
    authority: &AuthorityStore,
    org: &restless_orgintel::OrgIntel,
    company: &str,
    by: &str,
) -> Result<()> {
    let Some(away) = recorded(authority, company).await? else {
        return Ok(());
    };
    let mut resumed = Vec::new();
    for id in &away.paused_schedules {
        let Some(schedule) = org.get_schedule(*id).await? else {
            continue;
        };
        if schedule.paused_at.is_some() && set_paused(org, &schedule, false).await? {
            resumed.push(*id);
        }
    }
    authority
        .emit(
            company,
            AWAY_ENDED_KIND,
            Some(by),
            serde_json::json!({"resumed_schedules": resumed, "was_until": away.until}),
        )
        .await?;
    Ok(())
}

/// End an away whose date has passed. The scheduler calls this on each company scan.
pub async fn end_if_over(
    authority: &AuthorityStore,
    org: &restless_orgintel::OrgIntel,
    company: &str,
) -> Result<()> {
    if recorded(authority, company)
        .await?
        .is_some_and(|away| away.until <= Utc::now())
    {
        end(authority, org, company, "away-ended").await?;
    }
    Ok(())
}

async fn set_paused(
    org: &restless_orgintel::OrgIntel,
    schedule: &restless_orgintel::ScheduleRow,
    paused: bool,
) -> Result<bool> {
    Ok(org
        .change_recurring_schedule(
            schedule.id,
            schedule.fire_at,
            schedule.paused_at.is_some(),
            restless_orgintel::ScheduleChange {
                paused,
                interval_seconds: None,
                local_time: None,
                timezone: None,
                calendar_days: None,
                reason: None,
            },
        )
        .await?
        .is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only the named admin deputy, only while the away holds, and only for single decisions.
    #[tokio::test]
    async fn only_the_named_deputy_decides_and_only_single_approvals_while_away() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping away deputy scenario");
            return;
        };
        let authority = AuthorityStore::connect(&database_url).await.unwrap();
        let company = format!("away_{}_test", Uuid::new_v4().simple());
        let may = |actor: &'static str, role: &'static str, standing: bool| {
            let authority = &authority;
            let company = &company;
            async move { deputy_may_decide(authority, company, role, actor, standing).await }
        };
        assert!(!may("deputy", "admin", false).await, "no away in force");
        let away = |until: DateTime<Utc>| Away {
            until,
            deputy: Some("deputy".into()),
            paused_schedules: Vec::new(),
            since: Some(Utc::now()),
        };
        authority
            .emit(&company, AWAY_STARTED_KIND, Some("owner"), serde_json::to_value(away(Utc::now() + Duration::days(3))).unwrap())
            .await
            .unwrap();
        assert!(may("deputy", "admin", false).await, "the named deputy allows once or declines");
        assert!(!may("deputy", "admin", true).await, "never standing permission or revoke");
        assert!(!may("other", "admin", false).await, "an admin who is not the deputy");
        assert!(!may("deputy", "member", false).await, "a deputy who is no longer an admin");
        authority
            .emit(&company, AWAY_ENDED_KIND, Some("owner"), serde_json::json!({}))
            .await
            .unwrap();
        assert!(!may("deputy", "admin", false).await, "the owner is back");
        authority
            .emit(&company, AWAY_STARTED_KIND, Some("owner"), serde_json::to_value(away(Utc::now() - Duration::minutes(1))).unwrap())
            .await
            .unwrap();
        assert!(!may("deputy", "admin", false).await, "the return date passed");
        authority.delete_test_company(&company).await.unwrap();
    }
}
