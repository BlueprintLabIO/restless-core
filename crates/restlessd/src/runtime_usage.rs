//! Durable per-company Runtime wall-clock usage.
//!
//! This is an hours-of-container-uptime monitor, not a cloud invoice meter.
//! Docker's current run timestamps survive daemon restarts. Completed runs are
//! folded into a per-cell ledger; if an entire stop/restart could have fallen
//! between observations, the total remains a lower bound and is marked
//! incomplete instead of inventing elapsed time.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, LazyLock},
};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::runtime::{self, CompanyConfig, ContainerStatus};

static COMPANY_USAGE_LOCKS: LazyLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> =
    LazyLock::new(Default::default);

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RuntimeUsage {
    pub status: String,
    pub month_utc: String,
    /// Known or directly observed Runtime seconds in the current UTC month.
    pub used_seconds: u64,
    pub monthly_cap_hours: Option<u32>,
    pub remaining_seconds: Option<u64>,
    /// False means an unobserved container transition may have omitted usage.
    pub complete: bool,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Ledger {
    version: u32,
    month_utc: String,
    completed_seconds: u64,
    last_run_started_at: Option<String>,
    last_run_finished_at: Option<String>,
    last_observed_running: bool,
    complete: bool,
}

#[derive(Debug)]
struct ContainerObservation {
    status: ContainerStatus,
    created_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
}

fn month_key(now: DateTime<Utc>) -> String {
    format!("{:04}-{:02}", now.year(), now.month())
}

fn month_start(now: DateTime<Utc>) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .expect("valid UTC month start")
}

fn duration_in_month(start: DateTime<Utc>, end: DateTime<Utc>, now: DateTime<Utc>) -> u64 {
    let start = start.max(month_start(now));
    end.signed_duration_since(start).num_seconds().max(0) as u64
}

fn docker_time(value: Option<&str>) -> Option<DateTime<Utc>> {
    let value = value?;
    let parsed = DateTime::parse_from_rfc3339(value)
        .ok()?
        .with_timezone(&Utc);
    (parsed.year() >= 2000).then_some(parsed)
}

async fn container_observation(company: &str) -> Result<ContainerObservation> {
    let name = runtime::container_name(company);
    let template = "{{json .State}}|{{.Created}}";
    let output = runtime::docker_observe(&["inspect", "-f", template, &name]).await?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        if !error.to_ascii_lowercase().contains("no such") {
            bail!("Docker Runtime inspection failed: {}", error.trim());
        }
        return Ok(ContainerObservation {
            status: ContainerStatus::Absent,
            created_at: None,
            started_at: None,
            finished_at: None,
        });
    }
    let value = String::from_utf8_lossy(&output.stdout);
    let (state_json, created) = value
        .trim()
        .split_once('|')
        .context("Docker Runtime inspection did not include state and creation time")?;
    let state: serde_json::Value =
        serde_json::from_str(state_json).context("parse Docker Runtime state")?;
    let created_at = docker_time(Some(created));
    let started_at = docker_time(state.get("StartedAt").and_then(serde_json::Value::as_str));
    let finished_at = docker_time(state.get("FinishedAt").and_then(serde_json::Value::as_str));
    let status = if state.get("Running").and_then(serde_json::Value::as_bool) == Some(true) {
        ContainerStatus::Running
    } else {
        ContainerStatus::Stopped
    };
    Ok(ContainerObservation {
        status,
        created_at,
        started_at,
        finished_at,
    })
}

fn ledger_path(root: &Path, company: &str) -> std::path::PathBuf {
    root.join("cells").join(company).join("runtime-usage.json")
}

fn load_ledger(path: &Path, month: &str) -> Result<(Ledger, bool)> {
    if !path.exists() {
        return Ok((
            Ledger {
                version: 1,
                month_utc: month.to_string(),
                completed_seconds: 0,
                complete: true,
                ..Ledger::default()
            },
            false,
        ));
    }
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let mut ledger: Ledger =
        serde_json::from_slice(&bytes).with_context(|| format!("parse {}", path.display()))?;
    if ledger.version != 1 {
        bail!(
            "unsupported Runtime usage ledger version {}",
            ledger.version
        );
    }
    if ledger.month_utc != month {
        ledger.month_utc = month.to_string();
        ledger.completed_seconds = 0;
        ledger.last_run_started_at = None;
        ledger.last_run_finished_at = None;
        ledger.last_observed_running = false;
        ledger.complete = true;
    }
    Ok((ledger, true))
}

fn save_ledger(path: &Path, ledger: &Ledger) -> Result<()> {
    let parent = path
        .parent()
        .context("Runtime usage ledger has no parent")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(parent).or_else(
            |error| {
                if parent.is_dir() {
                    Ok(())
                } else {
                    Err(error)
                }
            },
        )?;
    }
    #[cfg(not(unix))]
    std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let temporary = path.with_extension(format!("json.{}.tmp", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec(ledger).context("serialize Runtime usage ledger")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        use std::io::Write;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    #[cfg(not(unix))]
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path).with_context(|| format!("persist {}", path.display()))
}

/// Observe and persist this company's current monthly Runtime hours.
pub(crate) async fn observe(root: &Path, config: &CompanyConfig) -> Result<RuntimeUsage> {
    let company = &config.name;
    runtime::validate_company_name(company)?;
    let lock = COMPANY_USAGE_LOCKS
        .lock()
        .await
        .entry(company.clone())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone();
    let _guard = lock.lock().await;
    let observed = container_observation(company).await?;
    let now = Utc::now();
    let month = month_key(now);
    let path = ledger_path(root, company);
    let (mut ledger, existed) = load_ledger(&path, &month)?;
    let old_start = ledger.last_run_started_at.clone();
    let mut active_seconds = 0;

    match observed.status {
        ContainerStatus::Running => {
            let start = observed
                .started_at
                .context("running Runtime has no Docker start timestamp")?;
            let start_text = start.to_rfc3339();
            if old_start.as_deref().is_some_and(|old| old != start_text)
                && ledger.last_observed_running
            {
                ledger.complete = false;
            }
            // A first observation of a pre-existing container cannot prove
            // that earlier runs in this month were observed.
            if ledger.last_run_started_at.is_none()
                && observed
                    .created_at
                    .is_some_and(|created| start > created + Duration::seconds(2))
                && start >= month_start(now)
            {
                ledger.complete = false;
            }
            ledger.last_run_started_at = Some(start_text);
            ledger.last_run_finished_at = None;
            ledger.last_observed_running = true;
            active_seconds = duration_in_month(start, now, now);
        }
        ContainerStatus::Stopped => {
            if let (Some(start), Some(finished)) = (observed.started_at, observed.finished_at) {
                let start_text = start.to_rfc3339();
                let finish_text = finished.to_rfc3339();
                if ledger.last_observed_running
                    && old_start.as_deref().is_some_and(|old| old != start_text)
                {
                    ledger.complete = false;
                }
                if ledger.last_run_finished_at.as_deref() != Some(finish_text.as_str()) {
                    ledger.completed_seconds = ledger
                        .completed_seconds
                        .saturating_add(duration_in_month(start, finished, now));
                }
                if ledger.last_run_started_at.is_none()
                    && observed
                        .created_at
                        .is_some_and(|created| start > created + Duration::seconds(2))
                    && start >= month_start(now)
                {
                    ledger.complete = false;
                }
                ledger.last_run_started_at = Some(start_text);
                ledger.last_run_finished_at = Some(finish_text);
            }
            ledger.last_observed_running = false;
        }
        ContainerStatus::Absent => {
            if ledger.last_observed_running {
                ledger.complete = false;
            }
            ledger.last_observed_running = false;
            if !existed {
                // A newly configured company with no Runtime has no usage to
                // reconcile, so an owner may configure a cap before first up.
                ledger.complete = true;
            }
        }
    }

    save_ledger(&path, &ledger)?;
    let used_seconds = ledger.completed_seconds.saturating_add(active_seconds);
    let cap_seconds = config
        .monthly_runtime_cap_hours
        .map(|hours| u64::from(hours) * 3_600);
    Ok(RuntimeUsage {
        status: match observed.status {
            ContainerStatus::Running => "running",
            ContainerStatus::Stopped => "stopped",
            ContainerStatus::Absent => "absent",
        }
        .into(),
        month_utc: month,
        used_seconds,
        monthly_cap_hours: config.monthly_runtime_cap_hours,
        remaining_seconds: cap_seconds.map(|cap| cap.saturating_sub(used_seconds)),
        complete: ledger.complete,
        observed_at: now,
    })
}

/// Reject a new start once an owner-configured monthly cap is exhausted.
/// Existing work remains available and is never stopped by this guard.
pub(crate) async fn ensure_start_allowed(root: &Path, config: &CompanyConfig) -> Result<()> {
    let Some(cap_hours) = config.monthly_runtime_cap_hours else {
        return Ok(());
    };
    let usage = observe(root, config).await?;
    if !usage.complete {
        bail!("monthly Runtime usage is incomplete; clear the cap or wait for the next UTC month before starting this company");
    }
    if usage.used_seconds >= u64::from(cap_hours) * 3_600 {
        bail!("monthly Runtime cap of {cap_hours} hours is exhausted; raise or clear the cap before starting this company");
    }
    Ok(())
}

/// Report whether the configured cap has elapsed while a Runtime is already
/// active. Callers decide whether work or another protected activity makes a
/// stop safe; this function only reads metering truth.
pub(crate) async fn cap_reached(root: &Path, config: &CompanyConfig) -> Result<bool> {
    let Some(cap_hours) = config.monthly_runtime_cap_hours else {
        return Ok(false);
    };
    let usage = observe(root, config).await?;
    if !usage.complete {
        bail!("monthly Runtime usage is incomplete; cannot determine whether the cap has been reached");
    }
    Ok(usage.used_seconds >= u64::from(cap_hours) * 3_600)
}
