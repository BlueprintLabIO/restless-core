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
    if restoring(&daemon.lifecycle, company) {
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
    let schedule_due = org
        .next_schedule_due_at()
        .await?
        .is_some_and(|at| at <= now);
    let opportunity_due = org
        .next_opportunity_due_at()
        .await?
        .is_some_and(|at| at <= now);
    if schedule_due || opportunity_due {
        demand.push(Demand::Due);
    }
    Ok(demand)
}

/// A recovery, a drain, or lifecycle work admitted for this company. Only this company's leases
/// count: the plane-wide count also holds Core's own upkeep (document history every ten seconds
/// across every company, fault delivery, stopped-Work checks) and every other company's scans and
/// effects, so with it no computer could sleep while anything ran anywhere ("busy (restore)").
fn restoring(lifecycle: &restless_contracts::appliance::LifecycleGate, company: &str) -> bool {
    lifecycle.is_draining()
        || lifecycle.company_is_recovering(company)
        || lifecycle.company_active(company) > 0
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
                    tracing::warn!(
                        company,
                        "sleep config unavailable; keeping awake: {error:#}"
                    );
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
            if config.compute_limits.is_some() && running.contains(company) {
                for target in [
                    runtime::ComputeTarget::Runtime,
                    runtime::ComputeTarget::Computer,
                ] {
                    if let Err(error) = runtime::apply_compute_limits(&config, target).await {
                        tracing::debug!(company, %error, "saved compute limits remain pending");
                    }
                }
            }
            if !running.contains(company) {
                quiet_since.remove(company);
                bring_back_if_interrupted(&daemon, &config).await;
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
                    tracing::warn!(
                        company,
                        "demand observation failed; keeping awake: {error:#}"
                    );
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

/// A computer that stopped without anyone asking comes back on its own: after a crash, an update
/// rollout that was interrupted, or a host restart (4 November: Reno's computer stayed off after a
/// rollout until the owner found it). One the owner stopped, one asleep, or one never created is
/// left as it is. The account memory pool and the start backoff apply as they do to any wake, and
/// nothing is started while the plane or this company is recovering.
async fn bring_back_if_interrupted(daemon: &Daemon, config: &CompanyConfig) -> bool {
    let company = config.name.as_str();
    if daemon.lifecycle.is_draining()
        || daemon.lifecycle.is_recovering()
        || daemon.lifecycle.company_is_recovering(company)
    {
        return false;
    }
    let Ok(status) = runtime::status(company).await else {
        return false;
    };
    if runtime::power_state(company, status) != runtime::PowerState::Interrupted {
        return false;
    }
    // A used-up monthly limit holds it until the owner raises it; say nothing every minute.
    if config.monthly_runtime_cap_hours.is_some() && hours_used_up(daemon, company).await {
        return false;
    }
    if runtime::automatic_start_waits_for_memory() {
        tracing::info!(
            company,
            "interrupted computer waits for account memory before starting"
        );
        return false;
    }
    wake(daemon, config, "it stopped without its owner asking").await
}

/// Put a quiet local computer to sleep now, as the owner asked. It refuses
/// while demand exists so an explicit sleep cannot cut live work.
pub async fn sleep_now(daemon: &Daemon, company: &str) -> Result<String> {
    let _browser_control = runtime::browser_control_guard(company).await;
    if runtime::down_if_idle(company, true, || is_quiet(daemon, company)).await? {
        return Ok(format!(
            "{company}: asleep (volume kept; owed demand wakes it)"
        ));
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

/// Wake a sleeping or interrupted local computer when company demand is owed. A computer the
/// owner stopped, or one that was never created, is never started here. `status` is the container
/// status the caller just observed. Returns whether the computer is running afterwards.
pub async fn wake_if_owed(
    daemon: &Daemon,
    config: &CompanyConfig,
    status: ContainerStatus,
) -> bool {
    let company = config.name.as_str();
    if !runtime::power_state(company, status).starts_on_demand() {
        return false;
    }
    let owed = match daemon.orgintel.get(company).await {
        Ok(org) => owed_demand(daemon, company, &org).await,
        Err(error) => Err(error.into()),
    };
    let owed = match owed {
        Ok(demand) => demand
            .into_iter()
            .filter(|kind| kind.wakes())
            .collect::<Vec<_>>(),
        Err(error) => {
            tracing::warn!(
                company,
                "could not observe demand for a sleeping computer: {error:#}"
            );
            return false;
        }
    };
    if owed.is_empty() {
        return false;
    }
    // Owed demand keeps; the next scan wakes the computer once the account pool has room.
    if runtime::automatic_start_waits_for_memory() {
        tracing::info!(
            company,
            "sleeping computer waits for account memory before waking"
        );
        return false;
    }
    wake(daemon, config, &names(&owed)).await
}

/// Wake a sleeping or interrupted local computer for a demand the caller already knows is owed,
/// such as a message addressed to the Exec or the owner opening a file. Returns whether it runs.
/// A computer the owner stopped stays stopped: [`not_running`] says why, for the refusal.
pub async fn wake_for(daemon: &Daemon, company: &str, reason: &str) -> bool {
    if daemon.runtime_bridges.is_hosted() {
        return true;
    }
    match runtime::status(company).await {
        Ok(ContainerStatus::Running) => true,
        Ok(status) if runtime::power_state(company, status).starts_on_demand() => {
            match CompanyConfig::load(&daemon.root, company) {
                Ok(config) => wake(daemon, &config, reason).await,
                Err(_) => false,
            }
        }
        _ => false,
    }
}

/// Why a wake did not leave the computer running, so a refusal can say it and offer the way on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotRunning {
    /// The owner stopped it. The refusal offers Start computer right there.
    StoppedByOwner,
    /// This month's computer hours are used up.
    HoursUsedUp,
    /// The company has no computer yet.
    Absent,
    /// It is starting, or the start failed and is retried: owed demand keeps trying.
    Starting,
}

impl NotRunning {
    /// The `error` code the owner API answers with, which the cockpit reads to offer its door.
    pub fn code(self) -> &'static str {
        match self {
            Self::StoppedByOwner => "runtime_stopped",
            Self::HoursUsedUp => "runtime_hours",
            Self::Absent => "runtime_absent",
            Self::Starting => "runtime_starting",
        }
    }

    /// One sentence for the owner about `what` ("this file can't open").
    pub fn sentence(self, what: &str) -> String {
        match self {
            Self::StoppedByOwner => format!(
                "The company computer was stopped by its owner, so {what} until it starts again."
            ),
            Self::HoursUsedUp => format!(
                "This month's computer hours are used up, so {what}. Raise or clear the limit in Company → Resources."
            ),
            Self::Absent => format!("This company has no computer yet, so {what}. Company → Health can build it."),
            Self::Starting => format!(
                "The company computer has not started yet, so {what}. Restless keeps starting it; try again in a moment."
            ),
        }
    }
}

/// After [`wake_for`] returned false: why the computer is not running.
pub async fn not_running(daemon: &Daemon, company: &str) -> NotRunning {
    let status = runtime::status(company).await;
    if let Ok(status) = status {
        match runtime::power_state(company, status) {
            runtime::PowerState::StoppedByOwner => return NotRunning::StoppedByOwner,
            runtime::PowerState::Absent => return NotRunning::Absent,
            _ => {}
        }
    }
    if hours_used_up(daemon, company).await {
        return NotRunning::HoursUsedUp;
    }
    NotRunning::Starting
}

/// Whether this month's computer hours are used up, so no wake can start it until the owner raises
/// or clears the limit. Read by the owner API to say so instead of "try again".
pub async fn hours_used_up(daemon: &Daemon, company: &str) -> bool {
    let Ok(config) = CompanyConfig::load(&daemon.root, company) else {
        return false;
    };
    config.monthly_runtime_cap_hours.is_some()
        && crate::runtime_usage::ensure_start_allowed(&runtime::state_root(), &config)
            .await
            .is_err()
}

async fn wake(daemon: &Daemon, config: &CompanyConfig, reason: &str) -> bool {
    let company = config.name.as_str();
    // A used-up monthly limit is a setting, not a failure: refuse without the start backoff, so
    // raising the limit lets the very next wake succeed.
    if config.monthly_runtime_cap_hours.is_some()
        && crate::runtime_usage::ensure_start_allowed(&runtime::state_root(), config)
            .await
            .is_err()
    {
        tracing::info!(
            company,
            reason,
            "company computer stays asleep: monthly hours used up"
        );
        return false;
    }
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
    match runtime::up_automatically(config).await {
        Ok(Some(_)) => {}
        Ok(None) => return false,
        Err(error) => return failed(format!("could not wake the company computer: {error:#}")),
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
        "interrupted" => {
            format!("{company}: stopped without its owner asking; it starts again on its own")
        }
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
    let (state, demand) = match runtime::power_state(company, status) {
        runtime::PowerState::Awake if daemon.runtime_bridges.is_hosted() => ("awake", Vec::new()),
        runtime::PowerState::Awake => ("awake", running_demand(daemon, company).await?),
        // An asleep computer whose old container a release retired (`retire_stale_stopped`) is
        // still asleep: its next wake creates the new one.
        power @ (runtime::PowerState::Asleep | runtime::PowerState::Interrupted) => {
            // Only waking demand is meaningful for a computer that is not running.
            let org = daemon.orgintel.get(company).await?;
            let owed = owed_demand(daemon, company, &org).await?;
            (
                if power == runtime::PowerState::Asleep {
                    "asleep"
                } else {
                    "interrupted"
                },
                owed.into_iter().filter(|kind| kind.wakes()).collect(),
            )
        }
        runtime::PowerState::StoppedByOwner => ("stopped", Vec::new()),
        runtime::PowerState::Absent => ("absent", Vec::new()),
    };
    Ok(serde_json::json!({
        "company": company,
        "state": state,
        "since": runtime::sleeping_since(company),
        "demand": demand.iter().map(|kind| kind.as_str()).collect::<Vec<_>>(),
        "sleep_after_minutes": config.sleep_after().map(|after| after.as_secs() / 60),
    }))
}

#[cfg(test)]
mod restore_tests {
    use restless_contracts::appliance::LifecycleGate;

    #[test]
    fn upkeep_and_other_companies_never_keep_a_quiet_computer_awake() {
        let gate = LifecycleGate::default();
        let _upkeep = gate.try_enter().expect("plane upkeep lease");
        let _neighbour = gate
            .try_enter_company("busy_test")
            .expect("another company's scan");
        assert!(!super::restoring(&gate, "quiet_test"));
        assert!(super::restoring(&gate, "busy_test"));
        let own = gate
            .try_enter_company("quiet_test")
            .expect("its own lifecycle work");
        assert!(super::restoring(&gate, "quiet_test"));
        drop(own);
        assert!(!super::restoring(&gate, "quiet_test"));
        gate.begin_company_recovery("quiet_test");
        assert!(super::restoring(&gate, "quiet_test"));
        gate.finish_company_recovery("quiet_test");
        gate.begin_drain();
        assert!(super::restoring(&gate, "quiet_test"));
        gate.resume();
        gate.begin_recovery();
        assert!(super::restoring(&gate, "quiet_test"));
    }
}

#[cfg(test)]
mod not_running_tests {
    use super::NotRunning;

    #[test]
    fn only_an_owner_stop_offers_the_start_door_and_no_sentence_sends_the_owner_away() {
        assert_eq!(NotRunning::StoppedByOwner.code(), "runtime_stopped");
        for why in [
            NotRunning::StoppedByOwner,
            NotRunning::Starting,
            NotRunning::Absent,
            NotRunning::HoursUsedUp,
        ] {
            let sentence = why.sentence("this file can't open");
            assert!(sentence.contains("this file can't open"), "{sentence}");
            assert!(!sentence.contains("Company → Computer"), "{sentence}");
            if why != NotRunning::StoppedByOwner {
                assert_ne!(why.code(), "runtime_stopped");
            }
        }
        assert!(NotRunning::StoppedByOwner
            .sentence("this file can't open")
            .starts_with("The company computer was stopped by its owner"));
    }
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
