//! Opt-in, fail-open suspension for quiet local company Runtimes.
//!
//! This only stops the replaceable Runtime container. Company files and OrgIntel
//! remain available, and owner reads never wake the Runtime. Hosted cells are
//! suspended by Fleet and are deliberately outside this local Docker adapter.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use tokio::process::Command;

use crate::{runtime, Daemon};

const SCAN_INTERVAL: Duration = Duration::from_secs(60);
const MAX_OBSERVATION: Duration = Duration::from_secs(8);
const SUPERVISOR_CONFIG: &str = "/etc/supervisor/conf.d/company.conf";

/// Monitor eligible local company computers after startup recovery has opened
/// admission. The timeout is deliberately opt-in in CompanyConfig; restarting
/// the owner plane starts a fresh grace period rather than guessing when an
/// unobserved company became idle.
pub(crate) async fn run(daemon: Arc<Daemon>) {
    if daemon.runtime_bridges.is_hosted() {
        tracing::info!("local Runtime auto-sleep disabled for hosted cells; Fleet owns suspension");
        return;
    }

    let mut idle_since = HashMap::<String, Instant>::new();
    let mut interval = tokio::time::interval(SCAN_INTERVAL);
    loop {
        interval.tick().await;
        let companies = match crate::configured_companies(&daemon.root) {
            Ok(companies) => companies,
            Err(error) => {
                tracing::warn!(
                    "Runtime auto-sleep inventory unavailable; keeping cells awake: {error:#}"
                );
                continue;
            }
        };
        let mut configs = Vec::with_capacity(companies.len());
        for company in companies {
            match runtime::CompanyConfig::load(&daemon.root, &company) {
                Ok(config) => configs.push(config),
                Err(error) => {
                    idle_since.remove(&company);
                    tracing::warn!(
                        company,
                        "Runtime auto-sleep config unavailable; keeping it awake: {error:#}"
                    );
                    continue;
                }
            }
        }
        let configured = configs
            .iter()
            .map(|config| config.name.clone())
            .collect::<std::collections::HashSet<_>>();
        idle_since.retain(|company, _| configured.contains(company));

        let running = match runtime::running_configured_companies(&configs).await {
            Ok(companies) => companies
                .into_iter()
                .collect::<std::collections::HashSet<_>>(),
            Err(error) => {
                idle_since.clear();
                tracing::warn!(
                    "Runtime auto-sleep inventory unavailable; keeping cells awake: {error:#}"
                );
                continue;
            }
        };

        for config in configs {
            let company = config.name.as_str();
            if !running.contains(company) {
                idle_since.remove(company);
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
            let idle_timeout = match config.auto_sleep_after_minutes {
                Some(minutes @ 1..=1440) => Some(Duration::from_secs(u64::from(minutes) * 60)),
                Some(minutes) => {
                    tracing::warn!(
                        company,
                        minutes,
                        "Runtime auto-sleep timeout is invalid; keeping it awake"
                    );
                    None
                }
                None => None,
            };
            if !cap_reached && idle_timeout.is_none() {
                idle_since.remove(company);
                continue;
            }

            match is_idle(&daemon, company).await {
                Ok(true) => {}
                Ok(false) => {
                    idle_since.insert(company.to_string(), Instant::now());
                    continue;
                }
                Err(error) => {
                    idle_since.insert(company.to_string(), Instant::now());
                    tracing::warn!(
                        company,
                        "Runtime auto-sleep observation failed; keeping it awake: {error:#}"
                    );
                    continue;
                }
            }

            let since = *idle_since
                .entry(company.to_string())
                .or_insert_with(Instant::now);
            if !cap_reached && since.elapsed() < idle_timeout.unwrap_or_default() {
                continue;
            }

            // Owner browser acquisition/renewal uses this same lease lock.
            // Hold it through the last eligibility check and graceful stop so
            // a newly arriving interactive session cannot be cut mid-attach.
            let _browser_control = runtime::browser_control_guard(company).await;
            let stopped = runtime::down_if_idle(company, || is_idle(&daemon, company)).await;
            match stopped {
                Ok(true) => {
                    idle_since.remove(company);
                    if cap_reached {
                        tracing::info!(
                            company,
                            "company Runtime stopped after reaching its monthly cap"
                        );
                    } else {
                        tracing::info!(
                            company,
                            idle_minutes = idle_timeout.unwrap_or_default().as_secs() / 60,
                            "company Runtime auto-slept"
                        );
                    }
                }
                Ok(false) => {
                    idle_since.insert(company.to_string(), Instant::now());
                }
                Err(error) => {
                    idle_since.insert(company.to_string(), Instant::now());
                    tracing::warn!(
                        company,
                        "Runtime auto-sleep stop failed; retaining the company computer: {error:#}"
                    );
                }
            }
        }
    }
}

/// `true` means every source was observed and none protects the Runtime.
/// Unavailable or malformed signals are errors so callers fail open.
async fn is_idle(daemon: &Daemon, company: &str) -> Result<bool> {
    let org = daemon.orgintel.get(company).await?;
    if !crate::owner::capacity_activity::protected_activity_kinds(daemon, company, &org)
        .await?
        .is_empty()
    {
        return Ok(false);
    }

    if owner_holds_browser(&runtime::read_browser_control(company).await?)? {
        return Ok(false);
    }
    if project_service_is_active(company).await? {
        return Ok(false);
    }
    Ok(true)
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
