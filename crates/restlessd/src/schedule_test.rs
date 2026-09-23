//! Manual exercise of one durable responsibility trigger in a disposable company.
//!
//! This probe verifies scheduler admission only. It never starts Runtime or an actor.

use std::time::Duration;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{runtime, Daemon};

pub(crate) async fn run(
    daemon: &Daemon,
    source_company: &str,
    source_schedule_id: Uuid,
    timeout_seconds: u64,
) -> Result<serde_json::Value> {
    if !(1..=3_600).contains(&timeout_seconds) {
        bail!("schedule test timeout must be between 1 and 3,600 seconds");
    }

    runtime::CompanyConfig::load(&daemon.root, source_company)
        .with_context(|| format!("load source company {source_company}"))?;
    let source_org = daemon.orgintel.get(source_company).await?;
    let source_schedule = source_org
        .list_schedules(None, true)
        .await?
        .into_iter()
        .find(|schedule| schedule.id == source_schedule_id)
        .context("source schedule does not exist")?;
    if source_schedule.actor_id != "exec" {
        bail!("schedule test currently supports only an Exec-owned schedule");
    }
    if source_schedule.cancelled_at.is_some() {
        bail!("cannot test a cancelled source schedule");
    }
    let responsibility_id = source_schedule.responsibility_id.context(
        "source schedule is not bound to a versioned responsibility; put and bind one first",
    )?;
    let responsibility_version = source_schedule
        .responsibility_version
        .context("source schedule is missing its responsibility version")?;
    let version = source_org
        .get_responsibility_version(responsibility_id, responsibility_version)
        .await?
        .context("source responsibility version is missing")?;

    let test_name = format!(
        "schedule_test_{}_test",
        &Uuid::new_v4().simple().to_string()[..8]
    );
    let mut config = runtime::CompanyConfig::load(&daemon.root, source_company)?;
    config.name = test_name;
    config.display_name = None;
    config.model.clear();
    config.agent_intelligence.clear();
    config.native_harnesses.clear();
    config.credentials.clear();
    config.approved_parties.clear();
    config.model_failover.clear();
    config.spend_ceiling_usd = runtime::SpendCeiling::from_micro_usd(1_000_000);
    runtime::CompanyConfig::save(&daemon.root, &config)?;

    let test_company = config.name.clone();
    let test_org = daemon.orgintel.get(&test_company).await?;
    test_org
        .ensure_actor("owner", "owner", "owner", "The Owner")
        .await?;
    test_org
        .ensure_actor("exec", "exec", "exec", "The Exec")
        .await?;
    test_org
        .ensure_actor("daemon", "system", "system-sender", "The daemon")
        .await?;

    let scheduled_for = Utc::now();
    let (test_schedule_id, _) = test_org
        .create_exact_schedule_with_responsibility(
            "exec",
            &source_schedule.reason,
            scheduled_for,
            "local_mac",
            responsibility_id,
            responsibility_version,
            &version.objective,
            version.policy,
        )
        .await?;

    // Claim the disposable occurrence directly without starting Runtime.
    let claimed = test_org.claim_due_schedules_at(scheduled_for).await?;
    if !claimed.iter().any(|row| row.id == test_schedule_id) {
        bail!("manual scheduler trigger did not claim the disposable occurrence");
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_seconds);
    loop {
        let opportunities = test_org
            .list_opportunities(Some(responsibility_id), 50)
            .await?;
        for opportunity in opportunities {
            let occurrences = test_org
                .list_opportunity_occurrences(opportunity.id)
                .await?;
            if let Some(occurrence) = occurrences
                .into_iter()
                .find(|row| row.schedule_id == test_schedule_id)
            {
                let linked_work = test_org.list_opportunity_work(opportunity.id).await?;
                let mut work_outcomes = Vec::new();
                for link in &linked_work {
                    let work = test_org.get_work(link.work_id).await?;
                    let attempts = test_org.list_work_attempts(Some(link.work_id)).await?;
                    work_outcomes.push(json!({"work": work, "attempts": attempts}));
                }
                return Ok(json!({
                    "status": "admitted",
                    "scope": "scheduler_only",
                    "actor_run": "not_started",
                    "source_files_copied": false,
                    "effect_boundary": "no Runtime or actor starts; this mode only records schedule admission",
                    "source_company": source_company,
                    "source_schedule_id": source_schedule_id,
                    "test_company": test_company,
                    "test_schedule_id": test_schedule_id,
                    "occurrence": occurrence,
                    "opportunity": opportunity,
                    "linked_work": linked_work,
                    "work_outcomes": work_outcomes,
                    "clone_retained": true,
                }));
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(json!({
                "status": "timed_out_before_admission",
                "scope": "scheduler_only",
                "actor_run": "not_started",
                "source_files_copied": false,
                "source_company": source_company,
                "source_schedule_id": source_schedule_id,
                "test_company": test_company,
                "test_schedule_id": test_schedule_id,
                "scheduled_for": scheduled_for,
                "observation": null,
                "clone_retained": true,
            }));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}
