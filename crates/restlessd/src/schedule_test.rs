//! Manual exercise of one durable responsibility trigger in a disposable company.
//!
//! Scheduler-only is the default. Explicit actor mode preserves only the Exec
//! model route and starts the throwaway Runtime; source business credentials
//! and approval state are never copied.

use std::{path::PathBuf, time::Duration};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{runtime, Daemon};

struct RuntimeStopGuard {
    company: String,
    root: PathBuf,
    stopped: bool,
}

impl RuntimeStopGuard {
    fn new(company: &str, root: PathBuf) -> Self {
        Self {
            company: company.to_string(),
            root,
            stopped: false,
        }
    }

    async fn stop(&mut self) -> Result<()> {
        runtime::down(&self.company).await?;
        disable_test_model(&self.root, &self.company)?;
        self.stopped = true;
        Ok(())
    }
}

impl Drop for RuntimeStopGuard {
    fn drop(&mut self) {
        if !self.stopped {
            let company = self.company.clone();
            let root = self.root.clone();
            tokio::spawn(async move {
                if let Err(error) = runtime::down(&company).await {
                    tracing::warn!(
                        company,
                        "could not stop schedule-test Runtime after early exit: {error:#}"
                    );
                }
                if let Err(error) = disable_test_model(&root, &company) {
                    tracing::warn!(
                        company,
                        "could not disable schedule-test model after early exit: {error:#}"
                    );
                }
            });
        }
    }
}

fn disable_test_model(root: &std::path::Path, company: &str) -> Result<()> {
    let mut config = runtime::CompanyConfig::load(root, company)?;
    config.model.clear();
    config.agent_intelligence.clear();
    config.native_harnesses.clear();
    runtime::CompanyConfig::save(root, &config)
}

pub(crate) async fn run(
    daemon: &Daemon,
    source_company: &str,
    source_schedule_id: Uuid,
    timeout_seconds: u64,
    run_actor: bool,
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
    // Build the disposable config deliberately: scheduler-only clears every
    // model route, while actor mode copies only Exec's explicit route.
    let source_config = runtime::CompanyConfig::load(&daemon.root, source_company)?;
    let mut config = source_config.clone();
    config.name = test_name;
    config.display_name = None;
    if run_actor {
        // Carry only Exec's explicit source route into the disposable company.
        // The actor must not inherit a worker's separate provider assignment.
        let route = source_config
            .agent_intelligence
            .get("exec")
            .or_else(|| source_config.agent_intelligence.get("default"))
            .cloned();
        config.agent_intelligence.clear();
        if let Some(route) = route {
            config.agent_intelligence.insert("exec".into(), route);
        }
        let exec_config = config.for_agent("exec");
        config.model = exec_config.model;
        config.coordination_harness = exec_config.coordination_harness;
        config.worker_harness = exec_config.worker_harness;
        config.native_harnesses.retain(|id, _| id == "codex");
        if config.model.starts_with("native-codex-oauth/") {
            bail!("actor-pipeline test cannot reuse a company-scoped Codex sign-in; run the safe trigger test or configure a separate test model connection");
        }
        config.mission = format!(
            "Disposable schedule actor-pipeline check for source schedule {source_schedule_id}. This is a synthetic exercise. Do not perform business work or external actions."
        );
        if !config.has_effective_model_route(config.coordination_harness) {
            bail!("synthetic actor pipeline test requires an explicit Exec model route or native Codex harness");
        }
    } else {
        config.model.clear();
        config.agent_intelligence.clear();
        config.native_harnesses.clear();
    }
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
    let test_objective = if run_actor {
        "Synthetic actor-pipeline check: acknowledge the scheduled wake and record a concise outcome. Do not perform business work or external actions."
    } else {
        &version.objective
    };
    let test_policy = if run_actor {
        json!({"window_seconds": 7200, "test_mode": true, "external_actions": "forbidden", "source_schedule_id": source_schedule_id})
    } else {
        version.policy
    };

    let scheduled_for = Utc::now();
    let test_reason = if run_actor {
        format!("Synthetic actor-pipeline test; source schedule {source_schedule_id}")
    } else {
        source_schedule.reason.clone()
    };
    let (test_schedule_id, _) = test_org
        .create_exact_schedule_with_responsibility(
            "exec",
            &test_reason,
            scheduled_for,
            "local_mac",
            responsibility_id,
            responsibility_version,
            test_objective,
            test_policy,
        )
        .await?;

    let mut runtime_guard = None;
    if run_actor {
        runtime::up(&config, false).await?;
        runtime_guard = Some(RuntimeStopGuard::new(&test_company, daemon.root.clone()));
        // Wake the resident scanner. It claims the occurrence and dispatches
        // the Exec through the ordinary durable inbox path.
        daemon.schedule_wake.notify_one();
    } else {
        // Scheduler-only remains the safe default: claim the same durable
        // occurrence directly and observe admission without starting Runtime.
        let claimed = test_org.claim_due_schedules_at(scheduled_for).await?;
        if !claimed.iter().any(|row| row.id == test_schedule_id) {
            bail!("manual scheduler trigger did not claim the disposable occurrence");
        }
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
                let terminal = matches!(
                    opportunity.state.as_str(),
                    "completed" | "needs_human" | "blocked" | "cancelled"
                );
                let attempts_settled = work_outcomes.iter().all(|row| {
                    row["attempts"].as_array().is_some_and(|attempts| {
                        attempts.iter().all(|attempt| attempt["state"] != "running")
                    })
                });
                if !run_actor || (terminal && attempts_settled) {
                    if let Some(guard) = runtime_guard.as_mut() {
                        guard.stop().await?;
                    }
                    return Ok(json!({
                        "status": if run_actor { "actor_observed" } else { "admitted" },
                        "scope": if run_actor { "actor_pipeline" } else { "scheduler_only" },
                        "actor_run": if run_actor { "observed" } else { "not_started" },
                        "source_files_copied": false,
                        "effect_boundary": if run_actor {
                            "source business credential bindings and approved parties are absent; the company spend ceiling is $1, but ordinary Runtime network egress remains possible"
                        } else {
                            "no Runtime or actor starts; this mode only records schedule admission"
                        },
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
        }
        if tokio::time::Instant::now() >= deadline {
            let mut observed = None;
            if run_actor {
                for opportunity in test_org
                    .list_opportunities(Some(responsibility_id), 50)
                    .await?
                {
                    let occurrences = test_org
                        .list_opportunity_occurrences(opportunity.id)
                        .await?;
                    if occurrences
                        .iter()
                        .any(|row| row.schedule_id == test_schedule_id)
                    {
                        let linked_work = test_org.list_opportunity_work(opportunity.id).await?;
                        let mut work_outcomes = Vec::new();
                        for link in &linked_work {
                            work_outcomes.push(json!({
                                "work": test_org.get_work(link.work_id).await?,
                                "attempts": test_org.list_work_attempts(Some(link.work_id)).await?,
                            }));
                        }
                        observed = Some(
                            json!({"opportunity": opportunity, "linked_work": linked_work, "work_outcomes": work_outcomes}),
                        );
                        break;
                    }
                }
                if let Some(guard) = runtime_guard.as_mut() {
                    guard.stop().await?;
                }
            }
            return Ok(json!({
                "status": if observed.is_some() { "actor_timeout_with_observation" } else { "timed_out_before_admission" },
                "scope": if run_actor { "actor_pipeline" } else { "scheduler_only" },
                "actor_run": if run_actor { "runtime_started; actor invocation not confirmed" } else { "not_started" },
                "source_files_copied": false,
                "source_company": source_company,
                "source_schedule_id": source_schedule_id,
                "test_company": test_company,
                "test_schedule_id": test_schedule_id,
                "scheduled_for": scheduled_for,
                "observation": observed,
                "clone_retained": true,
            }));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}
