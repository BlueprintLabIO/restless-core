//! Company computer sleep and wake.
//!
//! Core owns the single definition of company *demand*. A computer with no
//! demand for its sleep timeout is put to sleep; a sleeping computer with owed
//! demand is woken. Sleep only stops the replaceable Runtime container: the
//! volume, OrgIntel and owner reads stay available, and reads never wake it.
//!
//! Locally this module performs both transitions. Hosted cells expose the same
//! demand through the capacity-activity endpoint and Fleet performs the power
//! change; see the cross-layer contract, "Runtime sleep and wake".

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use tokio::process::Command;

use crate::runtime::{self, CompanyConfig, ContainerStatus};
use crate::Daemon;

const SCAN_INTERVAL: Duration = Duration::from_secs(60);
const MAX_OBSERVATION: Duration = Duration::from_secs(8);
const SUPERVISOR_CONFIG: &str = "/etc/supervisor/conf.d/company.conf";

/// One reason a company needs its computer. Only [`Demand::wakes`] kinds are
/// owed durable facts that justify starting a sleeping computer; the others
/// can only be observed on, and keep awake, a running one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Demand {
    /// An Attempt or actor session is running.
    Attempt,
    /// A daemon recovery, drain or lifecycle operation is in progress.
    Restore,
    /// An actor owes an answer: a mention, addressed mail, an owner message
    /// to the Exec, or an undelivered judgement.
    Conversation,
    /// Work is ready for its owner to start.
    ReadyWork,
    /// A schedule or recurring opportunity is due.
    Due,
    /// The owner holds the company browser or desktop.
    BrowserClaim,
    /// A company-defined supervised service is running.
    SupervisedService,
}

impl Demand {
    pub fn wakes(self) -> bool {
        matches!(self, Self::Conversation | Self::ReadyWork | Self::Due)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Attempt => "attempt",
            Self::Restore => "restore",
            Self::Conversation => "conversation",
            Self::ReadyWork => "ready_work",
            Self::Due => "due",
            Self::BrowserClaim => "browser_claim",
            Self::SupervisedService => "supervised_service",
        }
    }

    /// The capacity-activity v1 enum Fleet validates. v1 predates the names
    /// `conversation` and `due`; they are reported as the owed kinds Fleet
    /// already wakes for, so hosted cells gain those wakes without a new
    /// contract version.
    pub fn capacity_v1_kind(self) -> &'static str {
        match self {
            Self::Conversation => "mention",
            Self::Due => "ready_work",
            other => other.as_str(),
        }
    }
}

/// Demand read from durable company state and daemon supervision. It is
/// observable while the computer sleeps, which is what lets it wake one.
pub async fn owed_demand(
    daemon: &Daemon,
    company: &str,
    org: &restless_orgintel::OrgIntel,
) -> Result<Vec<Demand>> {
    let mut demand = Vec::with_capacity(4);
    let in_flight = daemon
        .in_flight
        .lock()
        .map_err(|_| anyhow::anyhow!("in-flight activity lock poisoned"))?
        .is_active(company);
    if in_flight || !daemon.staff.running_actors(company).is_empty() {
        demand.push(Demand::Attempt);
    }
    if daemon.lifecycle.is_recovering()
        || daemon.lifecycle.is_draining()
        || daemon.lifecycle.active() > 0
    {
        demand.push(Demand::Restore);
    }
    if !org.actors_owing_message_mentions(1).await?.is_empty()
        || !org.actors_owing_document_mentions().await?.is_empty()
        || !org.actors_owing_conversation_mail(1).await?.is_empty()
        || org.owed_conversation_count("exec").await? > 0
        || org.undelivered_handoff_count("exec").await? > 0
    {
        demand.push(Demand::Conversation);
    }
    if org.has_ready_work().await? {
        demand.push(Demand::ReadyWork);
    }
    let now = Utc::now();
    let schedule_due = org.next_schedule_due_at().await?.is_some_and(|at| at <= now);
    let opportunity_due = org
        .next_opportunity_due_at()
        .await?
        .is_some_and(|at| at <= now);
    if schedule_due || opportunity_due {
        demand.push(Demand::Due);
    }
    Ok(demand)
}

/// Everything that keeps a *running* local computer awake: owed demand plus
/// the owner's browser lease and company-defined services.
async fn running_demand(daemon: &Daemon, company: &str) -> Result<Vec<Demand>> {
    let org = daemon.orgintel.get(company).await?;
    let mut demand = owed_demand(daemon, company, &org).await?;
    if owner_holds_browser(&runtime::read_browser_control(company).await?)? {
        demand.push(Demand::BrowserClaim);
    }
    if project_service_is_active(company).await? {
        demand.push(Demand::SupervisedService);
    }
    Ok(demand)
}

async fn is_quiet(daemon: &Daemon, company: &str) -> Result<bool> {
    let demand = running_demand(daemon, company).await?;
    if !demand.is_empty() {
        tracing::debug!(company, ?demand, "company computer kept awake");
    }
    Ok(demand.is_empty())
}

/// Sleep quiet local company computers after startup recovery has opened
/// admission. Restarting the owner plane starts a fresh quiet period rather
/// than guessing when an unobserved company became quiet. Any observation
/// failure keeps the computer awake.
/// [`run`] as a [`crate::DaemonTask`], so it is compiled here rather than again in restlessd.
pub fn run_task(daemon: Arc<Daemon>) -> crate::DaemonTask<'static, ()> {
    Box::pin(run(daemon))
}

pub async fn run(daemon: Arc<Daemon>) {
    if daemon.runtime_bridges.is_hosted() {
        tracing::info!("hosted cell: Fleet performs sleep and wake from Core's demand");
        return;
    }

    let mut quiet_since = HashMap::<String, Instant>::new();
    let mut interval = tokio::time::interval(SCAN_INTERVAL);
    loop {
        interval.tick().await;
        let companies = match crate::configured_companies(&daemon.root) {
            Ok(companies) => companies,
            Err(error) => {
                tracing::warn!("sleep inventory unavailable; keeping computers awake: {error:#}");
                continue;
            }
        };
        let mut configs = Vec::with_capacity(companies.len());
        for company in companies {
            match CompanyConfig::load(&daemon.root, &company) {
                Ok(config) => configs.push(config),
                Err(error) => {
                    quiet_since.remove(&company);
                    tracing::warn!(company, "sleep config unavailable; keeping awake: {error:#}");
                }
            }
        }
        let configured = configs
            .iter()
            .map(|config| config.name.clone())
            .collect::<std::collections::HashSet<_>>();
        quiet_since.retain(|company, _| configured.contains(company));

        let running = match runtime::running_configured_companies(&configs).await {
            Ok(companies) => companies
                .into_iter()
                .collect::<std::collections::HashSet<_>>(),
            Err(error) => {
                quiet_since.clear();
                tracing::warn!("sleep inventory unavailable; keeping computers awake: {error:#}");
                continue;
            }
        };

        for config in configs {
            let company = config.name.as_str();
            if !running.contains(company) {
                quiet_since.remove(company);
                continue;
            }

            let cap_reached = if config.monthly_runtime_cap_hours.is_some() {
                match crate::runtime_usage::cap_reached(&daemon.root, &config).await {
                    Ok(reached) => reached,
                    Err(error) => {
                        tracing::warn!(company, "Runtime cap observation unavailable: {error:#}");
                        false
                    }
                }
            } else {
                false
            };
            let sleep_after = config.sleep_after();
            if !cap_reached && sleep_after.is_none() {
                quiet_since.remove(company);
                continue;
            }

            match is_quiet(&daemon, company).await {
                Ok(true) => {}
                Ok(false) => {
                    quiet_since.insert(company.to_string(), Instant::now());
                    continue;
                }
                Err(error) => {
                    quiet_since.insert(company.to_string(), Instant::now());
                    tracing::warn!(company, "demand observation failed; keeping awake: {error:#}");
                    continue;
                }
            }

            let since = *quiet_since
                .entry(company.to_string())
                .or_insert_with(Instant::now);
            if !cap_reached && since.elapsed() < sleep_after.unwrap_or_default() {
                continue;
            }

            // Owner browser acquisition/renewal uses this same lease lock.
            // Hold it through the last eligibility check and graceful stop so
            // a newly arriving interactive session cannot be cut mid-attach.
            // A cap stop is not sleep: demand must not restart a capped computer.
            let _browser_control = runtime::browser_control_guard(company).await;
            let stopped =
                runtime::down_if_idle(company, !cap_reached, || is_quiet(&daemon, company)).await;
            match stopped {
                Ok(true) => {
                    quiet_since.remove(company);
                    if cap_reached {
                        tracing::info!(company, "company computer stopped at its monthly cap");
                    } else {
                        tracing::info!(
                            company,
                            quiet_minutes = sleep_after.unwrap_or_default().as_secs() / 60,
                            "company computer fell asleep"
                        );
                    }
                }
                Ok(false) => {
                    quiet_since.insert(company.to_string(), Instant::now());
                }
                Err(error) => {
                    quiet_since.insert(company.to_string(), Instant::now());
                    tracing::warn!(company, "sleep failed; keeping the computer: {error:#}");
                }
            }
        }
    }
}

/// Put a quiet local computer to sleep now, as the owner asked. It refuses
/// while demand exists so an explicit sleep cannot cut live work.
pub async fn sleep_now(daemon: &Daemon, company: &str) -> Result<String> {
    let _browser_control = runtime::browser_control_guard(company).await;
    if runtime::down_if_idle(company, true, || is_quiet(daemon, company)).await? {
        return Ok(format!("{company}: asleep (volume kept; owed demand wakes it)"));
    }
    match runtime::status(company).await? {
        ContainerStatus::Running => {
            let demand = running_demand(daemon, company).await?;
            bail!(
                "{company} is busy ({}); it will sleep once quiet, or use `restless down` to stop it now",
                names(&demand)
            )
        }
        _ if runtime::is_sleeping(company) => Ok(format!("{company}: already asleep")),
        _ => Ok(format!("{company}: not running")),
    }
}

/// Wake a sleeping local computer when company demand is owed. A computer
/// the owner stopped, or one that was never created, is never started here.
/// Returns whether the computer is running afterwards.
pub async fn wake_if_owed(daemon: &Daemon, config: &CompanyConfig) -> bool {
    let company = config.name.as_str();
    if !runtime::is_sleeping(company) {
        return false;
    }
    let owed = match daemon.orgintel.get(company).await {
        Ok(org) => owed_demand(daemon, company, &org).await,
        Err(error) => Err(error.into()),
    };
    let owed = match owed {
        Ok(demand) => demand.into_iter().filter(|kind| kind.wakes()).collect::<Vec<_>>(),
        Err(error) => {
            tracing::warn!(company, "could not observe demand for a sleeping computer: {error:#}");
            return false;
        }
    };
    if owed.is_empty() {
        return false;
    }
    wake(daemon, config, &names(&owed)).await
}

/// Wake a sleeping local computer for a demand the caller already knows is
/// owed, such as a message addressed to the Exec. Returns whether it runs.
pub async fn wake_for(daemon: &Daemon, company: &str, reason: &str) -> bool {
    if daemon.runtime_bridges.is_hosted() {
        return true;
    }
    match runtime::status(company).await {
        Ok(ContainerStatus::Running) => true,
        Ok(ContainerStatus::Stopped) if runtime::is_sleeping(company) => {
            match CompanyConfig::load(&daemon.root, company) {
                Ok(config) => wake(daemon, &config, reason).await,
                Err(_) => false,
            }
        }
        _ => false,
    }
}

async fn wake(daemon: &Daemon, config: &CompanyConfig, reason: &str) -> bool {
    let company = config.name.as_str();
    let backing_off = daemon
        .in_flight
        .lock()
        .is_ok_and(|guard| guard.is_runtime_start_backing_off(company));
    if backing_off || CompanyConfig::load_archived(&daemon.root, company).is_ok() {
        return false;
    }
    let failed = |message: String| {
        if let Ok(mut guard) = daemon.in_flight.lock() {
            guard.record_runtime_start_failure(company);
        }
        tracing::warn!(company, reason, "{message}");
        false
    };
    if let Err(error) = runtime::up(config, false).await {
        return failed(format!("could not wake the company computer: {error:#}"));
    }
    if !matches!(runtime::status(company).await, Ok(ContainerStatus::Running)) {
        return failed("the company computer did not come up after waking".into());
    }
    if let Err(error) = crate::materialize_runtime_bridge(daemon, company).await {
        let stop = runtime::down_if_idle(company, true, || async { Ok(true) }).await;
        return failed(format!(
            "could not prepare the Runtime bridge after waking; put it back to sleep ({stop:?}): {error:#}"
        ));
    }
    if let Ok(mut guard) = daemon.in_flight.lock() {
        guard.clear_runtime_start_backoff(company);
    }
    tracing::info!(company, reason, "company computer woke");
    true
}

/// One owner-readable line for `restless status`.
pub fn describe(report: &serde_json::Value) -> String {
    let company = report["company"].as_str().unwrap_or_default();
    let state = report["state"].as_str().unwrap_or("unknown");
    let demand = report["demand"]
        .as_array()
        .map(|kinds| {
            kinds
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let policy = match report["sleep_after_minutes"].as_u64() {
        Some(minutes) => format!("sleeps after {minutes} quiet minutes"),
        None => "never sleeps".into(),
    };
    match state {
        "awake" if demand.is_empty() => format!("{company}: awake, quiet; {policy}"),
        "awake" => format!("{company}: awake, busy with {demand}; {policy}"),
        "asleep" if demand.is_empty() => format!("{company}: asleep; owed demand wakes it"),
        "asleep" => format!("{company}: asleep, waking for {demand}"),
        "stopped" => format!("{company}: stopped by its owner; `restless up` starts it"),
        _ => format!("{company}: not created; `restless up` creates it"),
    }
}

fn names(demand: &[Demand]) -> String {
    demand
        .iter()
        .map(|kind| kind.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Power state as the owner sees it, plus why the computer is awake.
pub async fn report(daemon: &Daemon, company: &str) -> Result<serde_json::Value> {
    let config = CompanyConfig::load(&daemon.root, company)?;
    let status = runtime::status(company).await?;
    let (state, demand) = match status {
        ContainerStatus::Running if daemon.runtime_bridges.is_hosted() => ("awake", Vec::new()),
        ContainerStatus::Running => ("awake", running_demand(daemon, company).await?),
        ContainerStatus::Stopped if runtime::is_sleeping(company) => {
            // Only waking demand is meaningful for a sleeping computer.
            let org = daemon.orgintel.get(company).await?;
            let owed = owed_demand(daemon, company, &org).await?;
            ("asleep", owed.into_iter().filter(|kind| kind.wakes()).collect())
        }
        ContainerStatus::Stopped => ("stopped", Vec::new()),
        ContainerStatus::Absent => ("absent", Vec::new()),
    };
    Ok(serde_json::json!({
        "company": company,
        "state": state,
        "since": runtime::sleeping_since(company),
        "demand": demand.iter().map(|kind| kind.as_str()).collect::<Vec<_>>(),
        "sleep_after_minutes": config.sleep_after().map(|after| after.as_secs() / 60),
    }))
}

fn owner_holds_browser(control: &Option<serde_json::Value>) -> Result<bool> {
    let Some(control) = control else {
        return Ok(false);
    };
    if control["controller"] != "owner" {
        return Ok(false);
    }
    let expires = control["expires_at"]
        .as_str()
        .context("owner browser lease omitted its expiry")?
        .parse::<chrono::DateTime<chrono::Utc>>()
        .context("owner browser lease had an invalid expiry")?;
    Ok(expires > chrono::Utc::now())
}

/// Company-defined supervised services are durable project processes. Any
/// such process that may still be starting, running, or stopping keeps its
/// Runtime awake; unknown Supervisor output also fails open.
async fn project_service_is_active(company: &str) -> Result<bool> {
    let container = runtime::container_name(company);
    let output = tokio::time::timeout(
        MAX_OBSERVATION,
        Command::new("docker")
            .args([
                "exec",
                &container,
                "supervisorctl",
                "-c",
                SUPERVISOR_CONFIG,
                "status",
            ])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("company Supervisor observation timed out")??;
    let stdout = String::from_utf8(output.stdout).context("Supervisor output was not UTF-8")?;
    if stdout.trim().is_empty() {
        bail!("company Supervisor returned no service status");
    }

    const BUILT_IN: &[&str] = &[
        "desktop",
        "openbox",
        "desktop-panel",
        "chromium",
        "browser-broker",
        "release-health",
        "desktop-web",
    ];
    let mut saw_status = false;
    for line in stdout.lines() {
        let mut fields = line.split_whitespace();
        let Some(name) = fields.next() else { continue };
        let Some(state) = fields.next() else {
            bail!("unrecognised company Supervisor status line");
        };
        saw_status = true;
        if !BUILT_IN.contains(&name) && state != "STOPPED" {
            return Ok(true);
        }
    }
    if !saw_status {
        bail!("company Supervisor returned no parseable service status");
    }
    Ok(false)
}
