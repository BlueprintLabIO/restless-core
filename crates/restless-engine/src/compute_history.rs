//! Recent account compute history for the owner's Resources page (Sprint 109 A0.5).
//!
//! An observation record for display, not a meter: nothing enforces, bills or schedules from it.
//! The account workload pool's own counters are read from the read-only cgroup mount every five
//! minutes (file reads only: no Docker, no process inside a company, no model). One account plane
//! holds one pool, so this is account state with no company column. Seven days are kept, in one
//! small file that survives a daemon restart; a missing or unreadable file starts empty.

use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const INTERVAL: Duration = Duration::from_secs(300);
const KEEP: usize = 7 * 24 * 12;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputePoint {
    pub at: DateTime<Utc>,
    /// Live memory: charged minus clean file cache, the reading admission uses.
    pub working_set_bytes: u64,
    pub memory_limit_bytes: u64,
    /// Mean cores over the interval since the previous point; absent for the first point.
    pub cpu_cores: Option<f64>,
    pub cpu_limit_cores: Option<f64>,
    /// New AI turns and Work were held for memory at this moment.
    pub waiting: bool,
    /// Times the pool reached its soft limit (Linux slowed it) since the previous point.
    pub throttled: u64,
    /// Processes the kernel killed for memory since the previous point.
    pub oom_kills: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct History {
    points: Vec<ComputePoint>,
    #[serde(skip)]
    last: Option<Counters>,
}

#[derive(Clone, Copy)]
struct Counters {
    cpu_usec: u64,
    high: u64,
    oom_kill: u64,
    at: std::time::Instant,
}

static HISTORY: LazyLock<Mutex<Option<History>>> = LazyLock::new(|| Mutex::new(None));

fn file() -> PathBuf {
    crate::runtime::state_root().join("compute-history.json")
}

fn load(path: &Path) -> History {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<History>(&bytes).ok())
        .unwrap_or_default()
}

fn counters(root: &Path) -> anyhow::Result<Counters> {
    let stat = std::fs::read_to_string(root.join("cpu.stat"))?;
    let cpu_usec = stat
        .lines()
        .find_map(|line| line.strip_prefix("usage_usec "))
        .ok_or_else(|| anyhow::anyhow!("CPU usage is unavailable"))?
        .trim()
        .parse()?;
    let events = std::fs::read_to_string(root.join("memory.events"))?;
    let event = |name: &str| {
        events
            .lines()
            .filter_map(|line| line.split_once(' '))
            .find_map(|(key, value)| (key == name).then(|| value.trim().parse::<u64>().ok()))
            .flatten()
            .unwrap_or(0)
    };
    Ok(Counters {
        cpu_usec,
        high: event("high"),
        oom_kill: event("oom_kill"),
        at: std::time::Instant::now(),
    })
}

fn cpu_limit(root: &Path) -> Option<f64> {
    let limit = std::fs::read_to_string(root.join("cpu.max")).ok()?;
    let mut parts = limit.split_whitespace();
    let quota: f64 = parts.next()?.parse().ok()?;
    let period: f64 = parts.next()?.parse().ok()?;
    (quota > 0.0 && period > 0.0).then_some(quota / period)
}

/// One observation of the pool at `root`, given the previous counters.
fn observe(root: &Path, history: &mut History) -> anyhow::Result<()> {
    let admission = crate::runtime::pool_admission(root)?;
    let memory_limit_bytes = std::fs::read_to_string(root.join("memory.max"))?
        .trim()
        .parse()?;
    let now = counters(root)?;
    let previous = history.last.replace(now);
    let delta = |current: u64, before: u64| current.saturating_sub(before);
    history.points.push(ComputePoint {
        at: Utc::now(),
        working_set_bytes: admission.working_set_bytes,
        memory_limit_bytes,
        cpu_cores: previous.and_then(|before| {
            let seconds = now.at.duration_since(before.at).as_secs_f64();
            (seconds > 0.0).then(|| delta(now.cpu_usec, before.cpu_usec) as f64 / 1e6 / seconds)
        }),
        cpu_limit_cores: cpu_limit(root),
        waiting: admission.waiting_for_memory,
        throttled: previous.map_or(0, |before| delta(now.high, before.high)),
        oom_kills: previous.map_or(0, |before| delta(now.oom_kill, before.oom_kill)),
    });
    let excess = history.points.len().saturating_sub(KEEP);
    history.points.drain(..excess);
    Ok(())
}

/// The kept history, oldest first. Empty where the account has no workload pool.
pub fn recent() -> Vec<ComputePoint> {
    HISTORY
        .lock()
        .ok()
        .and_then(|history| history.as_ref().map(|history| history.points.clone()))
        .unwrap_or_default()
}

/// Sample the account pool every five minutes for as long as the daemon runs.
pub async fn run() {
    let Ok(Some(_)) = crate::runtime::workload_cgroup_parent() else {
        return;
    };
    let Ok(root) = std::env::var("RESTLESS_WORKLOAD_CGROUP_ROOT").map(PathBuf::from) else {
        return;
    };
    let path = file();
    if let Ok(mut slot) = HISTORY.lock() {
        *slot = Some(load(&path));
    }
    let mut tick = tokio::time::interval(INTERVAL);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        let written = {
            let Ok(mut slot) = HISTORY.lock() else { return };
            let history = slot.get_or_insert_with(History::default);
            match observe(&root, history) {
                Ok(()) => serde_json::to_vec(&*history).ok(),
                Err(error) => {
                    tracing::debug!("compute history observation skipped: {error:#}");
                    None
                }
            }
        };
        if let Some(bytes) = written {
            let next = path.with_extension("json.next");
            if let Err(error) =
                std::fs::write(&next, bytes).and_then(|()| std::fs::rename(&next, &path))
            {
                tracing::debug!("compute history not saved: {error:#}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(dir: &Path, current: u64, file: u64, usec: u64, high: u64) {
        std::fs::write(dir.join("memory.current"), current.to_string()).unwrap();
        std::fs::write(dir.join("memory.max"), (2u64 << 30).to_string()).unwrap();
        std::fs::write(dir.join("memory.high"), (1800u64 << 20).to_string()).unwrap();
        std::fs::write(
            dir.join("memory.stat"),
            format!("file {file}\nshmem 0\nfile_dirty 0\nfile_writeback 0\nunevictable 0\n"),
        )
        .unwrap();
        std::fs::write(dir.join("cpu.stat"), format!("usage_usec {usec}\n")).unwrap();
        std::fs::write(dir.join("cpu.max"), "300000 100000\n").unwrap();
        std::fs::write(
            dir.join("memory.events"),
            format!("low 0\nhigh {high}\nmax 0\noom 0\noom_kill 0\n"),
        )
        .unwrap();
    }

    #[test]
    fn points_record_live_memory_and_interval_deltas_and_stay_bounded() {
        let dir = std::env::temp_dir().join(format!("compute-history-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let mut history = History::default();
        pool(&dir, 900 << 20, 300 << 20, 1_000_000, 5);
        observe(&dir, &mut history).unwrap();
        let first = &history.points[0];
        assert_eq!(
            first.working_set_bytes,
            600 << 20,
            "clean file cache is not live memory"
        );
        assert_eq!(
            (first.cpu_cores, first.throttled),
            (None, 0),
            "no interval before the first point"
        );
        assert_eq!(first.cpu_limit_cores, Some(3.0));
        assert!(!first.waiting);
        pool(&dir, 1700 << 20, 0, 2_000_000, 9);
        observe(&dir, &mut history).unwrap();
        let second = &history.points[1];
        assert_eq!(second.throttled, 4);
        assert!(second.cpu_cores.is_some_and(|cores| cores > 0.0));
        assert!(
            second.waiting,
            "above the admission threshold, new work waits"
        );
        history.points = vec![second.clone(); KEEP];
        observe(&dir, &mut history).unwrap();
        assert_eq!(history.points.len(), KEEP);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
