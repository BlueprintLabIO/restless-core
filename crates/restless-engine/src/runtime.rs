//! Runtime layer: the company computer's lifecycle, driven through the docker
//! CLI (mature infrastructure over bespoke machinery, §2.6). One persistent
//! container + one named volume per company; the volume is the company home.

use std::collections::HashMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::task::{Context as TaskContext, Poll};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use http_body_util::Empty;
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1;
use hyper::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use hyper_util::rt::TokioIo;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::process::{Child, ChildStdin, ChildStdout};
use uuid::Uuid;

pub const COMPANY_IMAGE: &str = "restless-company-image:latest";
const COMPANY_IMAGE_ENV: &str = "RESTLESS_COMPANY_IMAGE";
const SOURCE_DIGEST_LABEL: &str = "io.restless.source-digest";
/// What a released company image was built from (Core's Dagger publish). Two images with the same
/// value are the same computer, even when a release rebuilt it with a new id (S104 A1).
const BUILD_INPUT_LABEL: &str = "io.restless.build-input-sha256";
const COMPANY_SUPERVISOR_CONFIG: &str = "/etc/supervisor/conf.d/company.conf";
/// The company computer's own supervisor (S104 F2). Its socket is on the company volume, so the
/// plane reaches it from the Runtime container like the company supervisor.
const COMPUTER_SUPERVISOR_CONFIG: &str = "/etc/supervisor/conf.d/computer.conf";

// Startup Doctor and an explicit `up` can arrive together. Serialize the
// observe/create/seed sequence for one computer while other companies proceed.
static COMPANY_START_LOCKS: LazyLock<
    tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
> = LazyLock::new(Default::default);

// Browser control is one small, reconstructable Runtime file. Keep its
// read-modify-write transitions serial per company so two owner tabs cannot
// both observe an available lease and then each publish a successor.
static BROWSER_CONTROL_LOCKS: LazyLock<
    tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
> = LazyLock::new(Default::default);
static BROWSER_CONTROL_STATES: LazyLock<
    tokio::sync::Mutex<HashMap<String, tokio::sync::watch::Sender<Option<serde_json::Value>>>>,
> = LazyLock::new(Default::default);

// A cancelled Docker CLI does not cancel an OCI startup already accepted by
// the engine. Serialize each diagnostic program, and retain its retry deadline
// across cancellation. Distinct programs remain independent: a coordination
// probe can call the host, which may need a bounded browser-file read.
type ExecObservationSlot = Arc<tokio::sync::Mutex<Option<Instant>>>;
static EXEC_OBSERVATIONS: LazyLock<
    tokio::sync::Mutex<HashMap<(String, String), ExecObservationSlot>>,
> = LazyLock::new(Default::default);
const EXEC_OBSERVATION_RETRY: Duration = Duration::from_secs(30);

type HealthCacheSlot<T> = Arc<tokio::sync::Mutex<Option<(Instant, T)>>>;
static COCKPIT_DOCTOR_CACHE: LazyLock<
    tokio::sync::Mutex<HashMap<String, HealthCacheSlot<(RuntimeDoctor, DateTime<Utc>)>>>,
> = LazyLock::new(Default::default);
static BROWSER_HEALTH_CACHE: LazyLock<
    tokio::sync::Mutex<HashMap<String, HealthCacheSlot<(ContainerStatus, Option<BrowserDoctor>)>>>,
> = LazyLock::new(Default::default);
static COMPANY_STATUSES_CACHE: LazyLock<
    tokio::sync::Mutex<
        HashMap<Vec<String>, HealthCacheSlot<std::collections::BTreeMap<String, ContainerStatus>>>,
    >,
> = LazyLock::new(Default::default);

async fn invalidate_cockpit_health(company: &str) {
    COCKPIT_DOCTOR_CACHE.lock().await.remove(company);
    BROWSER_HEALTH_CACHE.lock().await.remove(company);
    COMPANY_STATUSES_CACHE.lock().await.clear();
    crate::native_harness::forget_status(company).await;
}

async fn company_start_guard(company: &str) -> tokio::sync::OwnedMutexGuard<()> {
    let lock = COMPANY_START_LOCKS
        .lock()
        .await
        .entry(container_name(company))
        .or_default()
        .clone();
    lock.lock_owned().await
}

pub async fn browser_control_guard(company: &str) -> tokio::sync::OwnedMutexGuard<()> {
    let lock = BROWSER_CONTROL_LOCKS
        .lock()
        .await
        .entry(container_name(company))
        .or_default()
        .clone();
    lock.lock_owned().await
}

pub async fn watch_browser_control(
    company: &str,
) -> tokio::sync::watch::Receiver<Option<serde_json::Value>> {
    let mut states = BROWSER_CONTROL_STATES.lock().await;
    states
        .entry(container_name(company))
        .or_insert_with(|| tokio::sync::watch::channel(None).0)
        .subscribe()
}

pub async fn publish_browser_control(company: &str, state: Option<serde_json::Value>) {
    let mut states = BROWSER_CONTROL_STATES.lock().await;
    let sender = states
        .entry(container_name(company))
        .or_insert_with(|| tokio::sync::watch::channel(None).0);
    sender.send_replace(state);
    drop(states);
    BROWSER_HEALTH_CACHE.lock().await.remove(company);
}

/// Owner browsers reconnect eagerly across appliance replacement. While the
/// one startup inventory owns Docker observation, fail incidental health reads
/// fast instead of launching competing CLI processes that can starve Docker
/// Desktop and indefinitely postpone the safety barrier.
static STARTUP_RECOVERY_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn begin_startup_recovery() {
    STARTUP_RECOVERY_ACTIVE.store(true, Ordering::SeqCst);
}

pub fn finish_startup_recovery() {
    STARTUP_RECOVERY_ACTIVE.store(false, Ordering::SeqCst);
}

/// Resolve the company Runtime artifact the plane operates.
///
/// Local appliance development keeps the historical local tag. A hosted
/// plane supplies the exact manifest digest through `RESTLESS_COMPANY_IMAGE`;
/// the plane never resolves a release tag or builds Core source itself.
pub fn company_image() -> String {
    resolve_company_image(std::env::var(COMPANY_IMAGE_ENV).ok().as_deref())
}

fn resolve_company_image(configured: Option<&str>) -> String {
    configured
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(COMPANY_IMAGE)
        .to_string()
}

fn is_immutable_image_digest(image: &str) -> bool {
    let Some((repository, digest)) = image.rsplit_once("@sha256:") else {
        return false;
    };
    !repository.is_empty()
        && digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The plane runs a release's own company image, pinned by digest: every Cloud plane and every
/// installed appliance (install-core.sh writes the pin). A development profile builds a mutable tag
/// instead. A pinned plane rebuilds company computers left on an earlier release's image at boot.
pub fn company_image_is_pinned() -> bool {
    is_immutable_image_digest(company_image().trim())
}

/// A network-reachable account plane is a Cloud release consumer. Refuse to
/// start unless Fleet supplied the immutable Runtime artifact from its lock.
pub fn validate_company_image_config(network_mode: bool) -> Result<()> {
    if !network_mode {
        return Ok(());
    }
    let configured = std::env::var(COMPANY_IMAGE_ENV).unwrap_or_default();
    if configured.trim().is_empty() {
        bail!(
            "network entry mode requires {COMPANY_IMAGE_ENV}=<registry/repository>@sha256:<digest>; \
             the account plane must consume the Runtime artifact pinned by Fleet"
        );
    }
    if !is_immutable_image_digest(configured.trim()) {
        bail!(
            "{COMPANY_IMAGE_ENV} must be an immutable OCI digest in network entry mode, not {:?}",
            configured.trim()
        );
    }
    Ok(())
}

/// Per-company runtime resource bounds.
///
/// A company computer is an unattended machine running agent-authored
/// processes: dev servers, browsers, game engines. Any of them can spin, and
/// an unbounded container spins on the *host's* cores. One abandoned Godot
/// demo held ~6 of 12 cores for 23 hours and drove the host into swap while
/// every disk-oriented debt check reported clean, because a busy container is
/// not a leaked one — nothing was bounding CPU at all.
///
/// These bounds do not prevent a runaway; they make one survivable and local.
/// Defaults are measured against observed healthy load, then given headroom.
/// A company building several sites concurrently sat at 2.6 GiB and 720 PIDs
/// with no leak present, so a 3 GiB cap ran at 86% of the limit on ordinary
/// work — close enough that the first symptom of a bound set too tight would
/// have been an OOM-killed build blamed on the build. Bounds exist to make a
/// runaway survivable, not to right-size healthy work; when the two conflict,
/// loosen the bound. Every value is overridable.
const DEFAULT_CPUS: &str = "4.0";
const DEFAULT_MEMORY: &str = "4g";
const DEFAULT_PIDS_LIMIT: &str = "2048";

/// Read a resource bound, preferring the environment override.
///
/// An explicitly empty override (`RESTLESS_COMPANY_CPUS=`) disables that one
/// bound rather than passing an empty flag to docker — the escape hatch for
/// diagnosing whether a bound is itself the problem.
fn resource_bound(var: &str, default: &str) -> Option<String> {
    resolve_resource_bound(std::env::var(var).ok().as_deref(), default)
}

/// The CPU bound for a company container, fitted to this host. Docker refuses
/// to start a container whose `--cpus` exceeds the host's CPUs, and the default
/// of four left every company unable to start when the plane moved to a
/// two-core server (21 October). An explicit override is fitted the same way.
fn company_cpus() -> Option<String> {
    fit_cpus(
        resource_bound("RESTLESS_COMPANY_CPUS", DEFAULT_CPUS),
        std::thread::available_parallelism().map_or(1, usize::from),
    )
}

/// The memory bound for a company container, kept under this host's memory: a 4 GB bound on a
/// 4 GB server is no bound at all, so one company could take the whole plane down (21 October).
/// The company computer's memory bound (S104 F2): its own, so a heavy browser is stopped inside
/// the computer and never takes the agents' Runtime down with it.
fn computer_memory() -> Option<String> {
    fit_memory(
        resource_bound(
            "RESTLESS_COMPUTER_MEMORY",
            &host_memory_bytes().map_or_else(
                || "2g".into(),
                |bytes| format!("{}m", bytes / 3 / (1024 * 1024)),
            ),
        ),
        host_memory_bytes(),
    )
}

fn company_memory() -> Option<String> {
    fit_memory(
        resource_bound("RESTLESS_COMPANY_MEMORY", DEFAULT_MEMORY),
        host_memory_bytes(),
    )
}

/// A burst job's memory bound: what it asked for, or the company default, kept under the host.
pub fn fit_job_memory(requested: Option<&str>) -> Option<String> {
    fit_memory(
        Some(
            requested
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(DEFAULT_MEMORY)
                .to_string(),
        ),
        host_memory_bytes(),
    )
}

fn host_memory_bytes() -> Option<u64> {
    host_memory_field("MemTotal:")
}

fn host_memory_field(field: &str) -> Option<u64> {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|meminfo| {
            meminfo
                .lines()
                .find_map(|line| line.strip_prefix(field))
                .and_then(|rest| {
                    rest.trim()
                        .trim_end_matches("kB")
                        .trim()
                        .parse::<u64>()
                        .ok()
                })
                .map(|kib| kib * 1024)
        })
}

fn fit_memory(bound: Option<String>, host_bytes: Option<u64>) -> Option<String> {
    let bound = bound?;
    let (Some(wanted), Some(host)) = (memory_bytes(&bound), host_bytes) else {
        return Some(bound);
    };
    // Leave a quarter of the host for the plane itself and the other companies.
    let ceiling = host / 4 * 3;
    Some(if wanted > ceiling {
        format!("{}m", ceiling / (1024 * 1024))
    } else {
        bound
    })
}

/// Fleet installs one shared host slice and exposes its read-only cgroup to
/// the account plane. A configured but missing/unbounded slice must refuse new
/// work rather than silently allowing every company its independent ceiling.
pub fn workload_cgroup_parent() -> Result<Option<String>> {
    let parent = match std::env::var("RESTLESS_WORKLOAD_CGROUP_PARENT") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(error).context("read host workload parent"),
    };
    validate_workload_parent(&parent)?;
    let root = std::env::var("RESTLESS_WORKLOAD_CGROUP_ROOT")
        .context("host workload cgroup observation is not configured")?;
    validate_workload_cgroup(std::path::Path::new(&root))?;
    Ok(Some(parent))
}

fn validate_workload_cgroup(root: &std::path::Path) -> Result<()> {
    let memory = std::fs::read_to_string(root.join("memory.max"))
        .context("host workload memory cap is unavailable")?;
    let memory: u64 = memory
        .trim()
        .parse()
        .context("host workload memory cap must be finite")?;
    anyhow::ensure!(memory > 0, "host workload memory cap must be positive");
    if let Some(total) = host_memory_bytes() {
        anyhow::ensure!(
            memory < total,
            "host workload cap must leave memory outside the pool"
        );
    }

    let cpu = std::fs::read_to_string(root.join("cpu.max"))
        .context("host workload CPU cap is unavailable")?;
    let values: Vec<_> = cpu.split_whitespace().collect();
    anyhow::ensure!(
        values.len() == 2
            && values
                .iter()
                .all(|value| value.parse::<u64>().is_ok_and(|value| value > 0)),
        "host workload CPU cap must be finite and positive"
    );
    let swap = std::fs::read_to_string(root.join("memory.swap.max"))
        .context("host workload swap cap is unavailable")?;
    anyhow::ensure!(
        swap.trim() == "0",
        "host workloads must not consume shared swap"
    );
    Ok(())
}

async fn container_uses_workload_parent(name: &str) -> Result<bool> {
    let Some(parent) = workload_cgroup_parent()? else {
        return Ok(true);
    };
    Ok(
        inspect_value(&["inspect", "-f", "{{.HostConfig.CgroupParent}}", name])
            .await?
            .as_deref()
            == Some(parent.as_str()),
    )
}

#[derive(Debug, Serialize)]
pub struct ComputeSample {
    pub memory_used_bytes: u64,
    pub memory_limit_bytes: Option<u64>,
    pub cpu_used_cores: Option<f64>,
    pub cpu_limit_cores: Option<f64>,
}

/// Pressure admission for new Work only. OrgIntel keeps the ready Work;
/// running Attempts, owner reads and recovery are not interrupted.
#[derive(Debug, Serialize)]
pub struct WorkloadAdmission {
    pub scope: &'static str,
    pub waiting_for_memory: bool,
    pub working_set_bytes: u64,
    pub resume_below_bytes: u64,
}

/// Process-start capacity, shared by Exec, addressed conversations and Work.
/// A reservation covers startup before the harness appears in live counters.
#[derive(Debug, Clone, Copy)]
pub struct ActorCapacity {
    pub company_slots: usize,
    pub shared_slots: Option<usize>,
    pub company_memory_slots: usize,
    pub shared_memory_slots: Option<usize>,
    pub waiting_for_memory: bool,
}

impl ActorCapacity {
    pub fn waits_for_memory(self, company_active: usize, shared_active: usize) -> bool {
        self.waiting_for_memory
            || company_active >= self.company_memory_slots
            || self
                .shared_memory_slots
                .is_some_and(|slots| shared_active >= slots)
    }
}

const ACTOR_MEMORY_RESERVATION: u64 = 512 * 1024 * 1024;

fn actor_slots(memory: u64, cpu_max: &str) -> Result<usize> {
    let mut fields = cpu_max.split_whitespace();
    let quota = fields.next().context("actor CPU quota is unavailable")?;
    let period: u64 = fields
        .next()
        .context("actor CPU period is unavailable")?
        .parse()?;
    anyhow::ensure!(period > 0, "actor CPU period must be positive");
    let memory_slots = (memory / ACTOR_MEMORY_RESERVATION).max(1) as usize;
    if quota == "max" {
        return Ok(memory_slots);
    }
    let quota: u64 = quota.parse()?;
    anyhow::ensure!(quota > 0, "actor CPU quota must be positive");
    Ok(memory_slots.min(quota.div_ceil(period).max(1) as usize))
}

fn actor_cgroup_capacity(path: &Path) -> Result<(usize, usize, bool)> {
    let sample = admission_from_cgroup(path)?;
    let maximum: u64 = std::fs::read_to_string(path.join("memory.max"))?
        .trim()
        .parse()?;
    let threshold = sample
        .resume_below_bytes
        .min(maximum.saturating_sub(ACTOR_MEMORY_RESERVATION));
    let slots = actor_slots(maximum, &std::fs::read_to_string(path.join("cpu.max"))?)?;
    let memory_slots = maximum.saturating_sub(sample.working_set_bytes) / ACTOR_MEMORY_RESERVATION;
    Ok((
        slots,
        memory_slots as usize,
        sample.working_set_bytes >= threshold,
    ))
}

pub async fn actor_capacity(company: &str) -> Result<Option<ActorCapacity>> {
    validate_company_name(company)?;
    if workload_cgroup_parent()?.is_none() {
        return Ok(None);
    }
    let root = PathBuf::from(std::env::var("RESTLESS_WORKLOAD_CGROUP_ROOT")?);
    let (shared_slots, shared_memory_slots, shared_waiting) = actor_cgroup_capacity(&root)?;
    let path = workload_container_scope(&container_name(company))
        .await?
        .context("company Runtime has not joined its workload pool")?;
    let (company_slots, company_memory_slots, company_waiting) =
        if std::fs::read_to_string(path.join("memory.max"))?.trim() == "max" {
            (shared_slots, shared_memory_slots, false)
        } else {
            actor_cgroup_capacity(&path)?
        };
    Ok(Some(ActorCapacity {
        company_slots: company_slots.min(shared_slots),
        shared_slots: Some(shared_slots),
        company_memory_slots,
        shared_memory_slots: Some(shared_memory_slots),
        waiting_for_memory: shared_waiting || company_waiting,
    }))
}

pub fn shared_workload_admission() -> Result<Option<WorkloadAdmission>> {
    if workload_cgroup_parent()?.is_none() {
        return Ok(None);
    }
    let root = std::path::PathBuf::from(std::env::var("RESTLESS_WORKLOAD_CGROUP_ROOT")?);
    admission_from_cgroup(&root).map(Some)
}

/// Read the individual Runtime's existing cgroup without starting a process
/// inside it. The configured read-only pool mount is the only filesystem root.
async fn workload_container_scope(name: &str) -> Result<Option<PathBuf>> {
    let Some(parent) = workload_cgroup_parent()? else {
        return Ok(None);
    };
    let output = docker_bounded(
        &[
            "inspect",
            "-f",
            r#"{"id":{{json .Id}},"parent":{{json .HostConfig.CgroupParent}}}"#,
            name,
        ],
        Duration::from_secs(3),
    )
    .await?;
    anyhow::ensure!(
        output.status.success(),
        "company memory observation is unavailable"
    );
    let row: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    if row["parent"].as_str() != Some(parent.as_str()) {
        return Ok(None); // Platform-owned identity browsers are outside the pool.
    }
    let id = row["id"]
        .as_str()
        .filter(|id| id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .context("invalid company resource identity")?;
    Ok(Some(
        PathBuf::from(std::env::var("RESTLESS_WORKLOAD_CGROUP_ROOT")?)
            .join(format!("docker-{id}.scope")),
    ))
}

pub async fn workload_admission(company: &str) -> Result<Option<WorkloadAdmission>> {
    validate_company_name(company)?;
    let Some(shared) = shared_workload_admission()? else {
        return Ok(None);
    };
    if shared.waiting_for_memory {
        return Ok(Some(shared));
    }
    let path = workload_container_scope(&container_name(company))
        .await?
        .context("company Runtime has not joined the shared workload pool")?;
    Ok(Some(combine_admission(shared, container_admission(&path)?)))
}

fn container_admission(path: &Path) -> Result<Option<WorkloadAdmission>> {
    // An explicitly uncapped local Runtime still shares its finite parent.
    if std::fs::read_to_string(path.join("memory.max"))?.trim() == "max" {
        return Ok(None);
    }
    admission_from_cgroup(path).map(Some)
}

fn combine_admission(
    shared: WorkloadAdmission,
    own: Option<WorkloadAdmission>,
) -> WorkloadAdmission {
    if !shared.waiting_for_memory {
        if let Some(mut own) = own.filter(|own| own.waiting_for_memory) {
            own.scope = "company";
            return own;
        }
    }
    shared
}

fn admission_from_cgroup(root: &std::path::Path) -> Result<WorkloadAdmission> {
    let read = |name: &str| -> Result<u64> {
        std::fs::read_to_string(root.join(name))?
            .trim()
            .parse()
            .with_context(|| format!("host workload {name} is unavailable"))
    };
    let current = read("memory.current")?;
    let maximum = read("memory.max")?;
    // Clean mapped/active file cache is reclaimable too. Counting only the
    // inactive list stranded an idle Runtime after a long conversation.
    // Keep tmpfs/shared, dirty/writeback and pinned memory charged.
    // Counter definitions: docs.kernel.org/admin-guide/cgroup-v2.html#memory
    let stat = std::fs::read_to_string(root.join("memory.stat"))?;
    let counter = |name: &str| -> Result<u64> {
        stat.lines()
            .filter_map(|line| line.split_once(' '))
            .find_map(|(key, value)| (key == name).then_some(value))
            .with_context(|| format!("host workload memory {name} is unavailable"))?
            .trim()
            .parse()
            .with_context(|| format!("invalid memory {name}"))
    };
    let protected = counter("shmem")?
        .saturating_add(counter("file_dirty")?)
        .saturating_add(counter("file_writeback")?)
        .saturating_add(counter("unevictable")?);
    let reclaimable = counter("file")?.saturating_sub(protected);
    let high = std::fs::read_to_string(root.join("memory.high"))?;
    let high = if high.trim() == "max" {
        maximum / 10 * 9
    } else {
        high.trim().parse::<u64>()?.min(maximum)
    };
    anyhow::ensure!(high > 0, "host workload admission limit must be positive");
    // Keep startup room before Linux starts throttling the pool at memory.high.
    // This is admission headroom, not a reservation or a second memory limit.
    // Small runtimes need real process-start headroom too: ten percent of a
    // 768 MiB limit is less than one normal harness startup.
    let reserve = (high / 10).max(256 * 1024 * 1024).min(high / 2);
    let resume_below_bytes = high.saturating_sub(reserve);
    let working_set_bytes = current.saturating_sub(reclaimable);
    Ok(WorkloadAdmission {
        scope: "shared",
        waiting_for_memory: working_set_bytes >= resume_below_bytes,
        working_set_bytes,
        resume_below_bytes,
    })
}

#[derive(Debug, Serialize)]
pub struct ComputeUsage {
    pub work_admission: WorkloadAdmission,
    pub observed_at: chrono::DateTime<chrono::Utc>,
    pub host_memory_bytes: Option<u64>,
    pub host_available_bytes: Option<u64>,
    pub pool: ComputeSample,
    pub work: Option<ComputeSample>,
    pub computer: Option<ComputeSample>,
    pub placement_current: bool,
}

struct ComputeCounter {
    memory: u64,
    memory_limit: Option<u64>,
    cpu: u64,
    cpu_limit: Option<f64>,
}

fn compute_counter(root: &std::path::Path) -> Result<ComputeCounter> {
    let memory = std::fs::read_to_string(root.join("memory.current"))?
        .trim()
        .parse()?;
    let memory_limit = std::fs::read_to_string(root.join("memory.max"))?
        .trim()
        .parse()
        .ok();
    let cpu = std::fs::read_to_string(root.join("cpu.stat"))?
        .lines()
        .find_map(|line| line.strip_prefix("usage_usec "))
        .context("CPU usage is unavailable")?
        .trim()
        .parse()?;
    let limit = std::fs::read_to_string(root.join("cpu.max"))?;
    let parts: Vec<_> = limit.split_whitespace().collect();
    let cpu_limit = parts
        .first()
        .and_then(|value| value.parse::<f64>().ok())
        .zip(parts.get(1).and_then(|value| value.parse::<f64>().ok()))
        .filter(|(quota, period)| *quota > 0.0 && *period > 0.0)
        .map(|(quota, period)| quota / period);
    Ok(ComputeCounter {
        memory,
        memory_limit,
        cpu,
        cpu_limit,
    })
}

fn compute_sample(before: &ComputeCounter, after: ComputeCounter, seconds: f64) -> ComputeSample {
    ComputeSample {
        memory_used_bytes: after.memory,
        memory_limit_bytes: after.memory_limit,
        cpu_used_cores: after
            .cpu
            .checked_sub(before.cpu)
            .map(|used| used as f64 / 1_000_000.0 / seconds),
        cpu_limit_cores: after.cpu_limit,
    }
}

/// Owner-only observations, requested while Resources is visible. No model,
/// process contents, global business store, or permanent polling loop is used.
pub async fn compute_usage(company: &str) -> Result<Option<ComputeUsage>> {
    validate_company_name(company)?;
    let Some(parent) = workload_cgroup_parent()? else {
        return Ok(None);
    };
    let root = std::path::PathBuf::from(std::env::var("RESTLESS_WORKLOAD_CGROUP_ROOT")?);
    let work_name = container_name(company);
    let computer_name = computer_container_name(company);
    // Labels select auxiliary jobs, tool workers and Documents without reading
    // any process arguments, files, environments or another company's contents.
    let inventory = docker_bounded(&[
        "ps", "--no-trunc", "--format",
        r#"{"id":{{json .ID}},"name":{{json .Names}},"job_company":{{json (.Label "restless.company")}},"company":{{json (.Label "io.restless.company")}},"documents_company":{{json (.Label "com.restless.company")}},"namespace":{{json (.Label "io.restless.namespace")}},"documents_root":{{json (.Label "com.restless.local-documents-root")}}}"#,
    ], Duration::from_secs(5)).await?;
    anyhow::ensure!(
        inventory.status.success(),
        "container usage inventory is unavailable"
    );
    let namespace = std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default();
    let state = state_root();
    let authority_root = state
        .canonicalize()
        .unwrap_or(state)
        .to_string_lossy()
        .into_owned();
    let mut selected = vec![work_name.clone(), computer_name.clone()];
    for line in String::from_utf8_lossy(&inventory.stdout).lines() {
        let row: serde_json::Value = serde_json::from_str(line)?;
        if compute_container_belongs(&row, company, &namespace, &authority_root, &work_name) {
            if let Some(id) = row["id"]
                .as_str()
                .filter(|id| id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
            {
                selected.push(id.to_string());
            }
        }
    }
    let mut args = vec![
        "inspect",
        "-f",
        r#"{"id":{{json .Id}},"name":{{json .Name}},"parent":{{json .HostConfig.CgroupParent}},"running":{{json .State.Running}}}"#,
    ];
    args.extend(selected.iter().map(String::as_str));
    let output = docker_bounded(&args, Duration::from_secs(5)).await?;
    let mut seen = std::collections::HashSet::new();
    let rows: Vec<serde_json::Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|row| row["running"].as_bool() == Some(true))
        .filter(|row| seen.insert(row["id"].as_str().unwrap_or_default().to_string()))
        .collect();
    // Finished jobs do not keep a company's limits looking perpetually pending.
    let placement_current = rows
        .iter()
        .all(|row| row["parent"].as_str() == Some(parent.as_str()));
    let mut work_paths = Vec::new();
    let mut computer_path = None;
    let mut runtime_path = None;
    for row in &rows {
        let Some(id) = row["id"]
            .as_str()
            .filter(|id| id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
        else {
            continue;
        };
        if row["parent"].as_str() != Some(parent.as_str()) {
            continue;
        }
        let path = root.join(format!("docker-{id}.scope"));
        if row["name"]
            .as_str()
            .map(|name| name.trim_start_matches('/'))
            == Some(work_name.as_str())
        {
            runtime_path = Some(path.clone());
        }
        if row["name"]
            .as_str()
            .map(|name| name.trim_start_matches('/'))
            == Some(computer_name.as_str())
        {
            computer_path = Some(path);
        } else {
            work_paths.push(path);
        }
    }
    let pool_before = compute_counter(&root)?;
    let work_before: Vec<_> = work_paths
        .iter()
        .filter_map(|path| compute_counter(path).ok().map(|counter| (path, counter)))
        .collect();
    let computer_before = computer_path
        .as_ref()
        .and_then(|path| compute_counter(path).ok());
    let at = std::time::Instant::now();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let pool_after = compute_counter(&root)?;
    let seconds = at.elapsed().as_secs_f64();
    let observed_work_scopes = work_before.len();
    let work_samples: Vec<_> = work_before
        .into_iter()
        .filter_map(|(path, before)| {
            compute_counter(path)
                .ok()
                .map(|after| compute_sample(&before, after, seconds))
        })
        .collect();
    let lost_work_scope = work_samples.len() != observed_work_scopes;
    let mut work = aggregate_compute_samples(work_samples, &pool_after);
    if lost_work_scope {
        if let Some(work) = &mut work {
            work.cpu_used_cores = None;
        }
    }
    let computer = computer_before.and_then(|before| {
        Some(compute_sample(
            &before,
            compute_counter(computer_path.as_ref()?).ok()?,
            seconds,
        ))
    });
    Ok(Some(ComputeUsage {
        work_admission: combine_admission(
            admission_from_cgroup(&root)?,
            runtime_path
                .as_deref()
                .map(container_admission)
                .transpose()?
                .flatten(),
        ),
        observed_at: chrono::Utc::now(),
        host_memory_bytes: host_memory_bytes(),
        host_available_bytes: host_memory_field("MemAvailable:"),
        pool: compute_sample(&pool_before, pool_after, seconds),
        work,
        computer,
        placement_current,
    }))
}

fn compute_container_belongs(
    row: &serde_json::Value,
    company: &str,
    namespace: &str,
    root: &str,
    work_name: &str,
) -> bool {
    (row["documents_company"].as_str() == Some(company)
        && row["documents_root"].as_str() == Some(root))
        || (row["job_company"].as_str() == Some(company)
            && row["name"]
                .as_str()
                .is_some_and(|name| name.starts_with(&format!("{work_name}-job-"))))
        || (row["company"].as_str() == Some(company)
            && row["namespace"].as_str().unwrap_or_default() == namespace)
}

fn aggregate_compute_samples(
    samples: Vec<ComputeSample>,
    pool: &ComputeCounter,
) -> Option<ComputeSample> {
    if samples.is_empty() {
        return None;
    }
    let memory = samples.iter().map(|sample| sample.memory_used_bytes).sum();
    let memory_limit = samples
        .iter()
        .map(|sample| sample.memory_limit_bytes)
        .sum::<Option<u64>>()
        .zip(pool.memory_limit)
        .map(|(sum, cap)| sum.min(cap))
        .or(pool.memory_limit);
    let cpu = samples
        .iter()
        .map(|sample| sample.cpu_used_cores)
        .sum::<Option<f64>>();
    let cpu_limit = samples
        .iter()
        .map(|sample| sample.cpu_limit_cores)
        .sum::<Option<f64>>()
        .zip(pool.cpu_limit)
        .map(|(sum, cap)| sum.min(cap))
        .or(pool.cpu_limit);
    Some(ComputeSample {
        memory_used_bytes: memory,
        memory_limit_bytes: memory_limit,
        cpu_used_cores: cpu,
        cpu_limit_cores: cpu_limit,
    })
}

fn validate_workload_parent(parent: &str) -> Result<()> {
    anyhow::ensure!(
        parent.starts_with("restless-")
            && parent.ends_with(".slice")
            && parent.len() <= 128
            && parent.len() > "restless-.slice".len()
            && parent[..parent.len() - 6]
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'),
        "host workload parent must name a Restless systemd slice"
    );
    Ok(())
}

/// Docker's memory notation (`4g`, `512m`, `1073741824`) in bytes.
fn memory_bytes(value: &str) -> Option<u64> {
    let value = value.trim().to_ascii_lowercase();
    let (number, unit) = value.split_at(
        value
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(value.len()),
    );
    let number: u64 = number.parse().ok()?;
    let scale = match unit.trim_end_matches('b') {
        "" => 1,
        "k" => 1024,
        "m" => 1024 * 1024,
        "g" => 1024 * 1024 * 1024,
        _ => return None,
    };
    Some(number * scale)
}

fn fit_cpus(bound: Option<String>, host_cpus: usize) -> Option<String> {
    let bound = bound?;
    let Ok(wanted) = bound.parse::<f64>() else {
        return Some(bound);
    };
    // One core always stays with the host: a company building on every core starved the plane
    // until its readiness checks failed and every company showed as unavailable (24 October).
    let host = host_cpus.max(1);
    let ceiling = if host > 1 { host - 1 } else { host } as f64;
    Some(if wanted > ceiling {
        format!("{ceiling:.1}")
    } else {
        bound
    })
}

/// A stopped container keeps the CPU bound it was created with. Refit it
/// before starting, so a container made on a larger host still starts.
async fn refit_container_cpus(name: &str) {
    let Some(cpus) = company_cpus() else { return };
    let Ok(output) = docker(&["inspect", "-f", "{{.HostConfig.NanoCpus}}", name]).await else {
        return;
    };
    let current = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .unwrap_or(0.0)
        / 1e9;
    let Ok(wanted) = cpus.parse::<f64>() else {
        return;
    };
    if current > wanted + f64::EPSILON {
        if let Err(error) = run_ok(&["update", "--cpus", &cpus, name]).await {
            tracing::warn!(container = name, %error, "could not fit the container's CPU bound to this host");
        }
    }
}

/// The bound decision, split from the environment read so it is testable
/// without mutating process-global state from concurrent tests.
fn resolve_resource_bound(override_value: Option<&str>, default: &str) -> Option<String> {
    match override_value {
        Some(value) if value.trim().is_empty() => None,
        Some(value) => Some(value.trim().to_string()),
        None => Some(default.to_string()),
    }
}

/// Exact model-spend ceiling in micro-USD. This is an Authority value, not a
/// display float: `inf`, `NaN`, negative values and more than six fractional
/// cents are configuration errors rather than a route to an uncapped company.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpendCeiling(u64);

impl SpendCeiling {
    pub const fn from_micro_usd(micro_usd: u64) -> Self {
        Self(micro_usd)
    }

    #[must_use]
    pub const fn micro_usd(self) -> u64 {
        self.0
    }

    /// This conversion is presentation only. Authority comparisons use
    /// `micro_usd()` so binary floating-point never decides whether a company
    /// may spend.
    #[must_use]
    pub fn as_usd(self) -> f64 {
        self.0 as f64 / 1_000_000.0
    }

    pub fn parse(value: &str) -> Result<Self> {
        let value = value.trim();
        if value.is_empty() || value.starts_with('-') || value.starts_with('+') {
            bail!("spend ceiling must be a non-negative decimal USD amount");
        }
        let mut pieces = value.split('.');
        let whole = pieces.next().unwrap_or_default();
        let fraction = pieces.next();
        if pieces.next().is_some()
            || whole.is_empty()
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
        {
            bail!("spend ceiling must be a non-negative decimal USD amount");
        }
        let whole = whole
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("spend ceiling is too large"))?;
        let fraction = fraction.unwrap_or_default();
        if !fraction.bytes().all(|byte| byte.is_ascii_digit()) || fraction.len() > 6 {
            bail!("spend ceiling supports at most six fractional USD digits");
        }
        let fraction = if fraction.is_empty() {
            0
        } else {
            let parsed = fraction
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("invalid spend ceiling fraction"))?;
            parsed
                .checked_mul(10_u64.pow((6 - fraction.len()) as u32))
                .expect("fraction padding is bounded to six digits")
        };
        let micro_usd = whole
            .checked_mul(1_000_000)
            .and_then(|value| value.checked_add(fraction))
            .context("spend ceiling is too large")?;
        Ok(Self(micro_usd))
    }
}

impl fmt::Display for SpendCeiling {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.0 / 1_000_000;
        let fraction = self.0 % 1_000_000;
        if fraction == 0 {
            return write!(formatter, "{whole}");
        }
        let mut fraction = format!("{fraction:06}");
        while fraction.ends_with('0') {
            fraction.pop();
        }
        write!(formatter, "{whole}.{fraction}")
    }
}

impl Serialize for SpendCeiling {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // A TOML string retains every micro-USD exactly across a save/load.
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SpendCeiling {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SpendCeilingVisitor;

        impl<'de> Visitor<'de> for SpendCeilingVisitor {
            type Value = SpendCeiling;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .write_str("a finite, non-negative USD amount with at most six decimal places")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                SpendCeiling::parse(value).map_err(E::custom)
            }

            fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }

            fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                SpendCeiling::parse(&value.to_string()).map_err(E::custom)
            }

            fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value < 0 {
                    return Err(E::custom("spend ceiling must be non-negative"));
                }
                self.visit_u64(value as u64)
            }

            fn visit_f64<E>(self, value: f64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                if !value.is_finite() || value < 0.0 {
                    return Err(E::custom("spend ceiling must be finite and non-negative"));
                }
                // Rust's shortest round-trip representation preserves the
                // owner-supplied decimal intent without using float arithmetic
                // for the authority decision.
                SpendCeiling::parse(&value.to_string()).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(SpendCeilingVisitor)
    }
}

/// One company's identity and configuration, as a file — not a table (sprint
/// spec, kernel slice). Lives at `$RESTLESS_HOME/companies/<name>.toml`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentHarness {
    /// Restless' managed ACP harness, retained as the compatible default.
    #[serde(alias = "omp", alias = "restless_managed")]
    #[default]
    RestlessManaged,
    /// First-party Codex app-server transport for productive Staff Attempts.
    Codex,
    /// Certified Claude Code ACP adapter shipped in the company image.
    #[serde(alias = "claude_agent")]
    ClaudeAgent,
    /// Owner-installed ACP agent, resolved through its explicit intelligence assignment.
    CustomAcp,
}

impl AgentHarness {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RestlessManaged => "restless-managed",
            Self::Codex => "codex",
            Self::ClaudeAgent => "claude-agent",
            Self::CustomAcp => "custom-acp",
        }
    }

    pub fn parse_canonical(value: &str) -> Option<Self> {
        match value.trim() {
            "restless-managed" => Some(Self::RestlessManaged),
            "codex" => Some(Self::Codex),
            "claude-agent" => Some(Self::ClaudeAgent),
            _ => None,
        }
    }

    pub const fn build(self) -> &'static str {
        match self {
            Self::RestlessManaged => "omp-18.8.6",
            Self::Codex => "codex-cli-0.155.1",
            Self::ClaudeAgent => "claude-agent-acp-0.90.0",
            Self::CustomAcp => "custom-acp-v1",
        }
    }

    pub const fn native_agent_build(self) -> Option<&'static str> {
        match self {
            Self::ClaudeAgent => Some("claude-code-2.1.296"),
            Self::RestlessManaged | Self::Codex | Self::CustomAcp => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeHarnessConfig {
    pub mode: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIntelligence {
    pub connection: String,
    pub model: String,
}

/// Account assignments name the exact connection. The provider is repeated so
/// a saved company config can resolve its model without reading account state;
/// the owner API and model relay verify the ID against the current grant.
pub fn account_intelligence_route(connection: &str) -> Option<(&str, &str, AgentHarness)> {
    let (route, harness) = if let Some(route) = connection.strip_prefix("account:") {
        (route, AgentHarness::RestlessManaged)
    } else if let Some(route) = connection.strip_prefix("account-harness:codex:") {
        (route, AgentHarness::Codex)
    } else if let Some(route) = connection.strip_prefix("account-harness:claude-agent:") {
        (route, AgentHarness::ClaudeAgent)
    } else {
        return None;
    };
    let (provider, id) = route.split_once('@')?;
    if provider.is_empty()
        || id.len() != 32
        || !id.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !provider
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
    {
        return None;
    }
    Some((provider, id, harness))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyConfig {
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub agent_intelligence: std::collections::BTreeMap<String, AgentIntelligence>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub native_harnesses: std::collections::BTreeMap<String, NativeHarnessConfig>,
    /// Owner-facing name; the durable company handle remains unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Owner-uploaded logo: the name of a file beside this config, read only
    /// through [`company_logo`], and seeded to `/company/brand/logo.<ext>` on `up`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    /// Company name; also the container/volume suffix and schema name.
    pub name: String,
    /// Place this company's Runtime on an isolated Docker network with no
    /// external routing. Existing company files retain the production bridge
    /// default unless the owner opts in explicitly.
    #[serde(default)]
    pub internal_network: bool,
    /// Owner-set mission, seeded to /company/mission.md on `up`.
    #[serde(default)]
    pub mission: String,
    /// Per-company model spend ceiling in USD (T2). The fuse, not governance.
    #[serde(default = "default_ceiling")]
    pub spend_ceiling_usd: SpendCeiling,
    /// Whether the ceiling covers all spend so far or each UTC calendar month.
    #[serde(default, skip_serializing_if = "SpendPeriod::is_total")]
    pub spend_period: SpendPeriod,
    /// The share of the ceiling, in percent, at which the owner's Inbox warns before the ceiling
    /// stops model work. `None` warns at [`DEFAULT_SPEND_WARNING_PERCENT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spend_warning_percent: Option<u8>,
    /// Spend already counted when the owner last changed the limit (S104). The warning then
    /// fires at its share of the headroom that change gave, not again at once: a raise to US$220
    /// with US$193 spent left a fresh "Raise the limit" card, and a second save made it US$440.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spend_warning_baseline: Option<SpendWarningBaseline>,
    /// Sites this company reaches through its owner's own computer (a paired helper machine), from
    /// a home IP, rather than from the plane's datacenter address: marketplaces block datacenter
    /// IPs (S99 step 2). Each is a registrable host such as `ebay.com.au`, covering its subdomains.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub home_sites: Vec<String>,
    /// Optional monthly running-time cap. It gates new Runtime starts and does
    /// not stop work that is already running. This is a capacity policy, not a
    /// representation of the provider's invoice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthly_runtime_cap_hours: Option<u32>,
    /// Optional caps on native-harness use (Codex or Claude on their own
    /// sign-in) per UTC month. Reaching either refuses new native turns; a
    /// turn already running is never stopped. `None` is no cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_monthly_turn_limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_monthly_token_limit: Option<u64>,
    /// Sleep after this many minutes without company demand. `None` uses
    /// [`DEFAULT_SLEEP_AFTER_MINUTES`]; `Some(0)` keeps the computer always on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_sleep_after_minutes: Option<u16>,
    /// Provider-qualified model the agent runs on, e.g. `zai/glm-5.2`.
    /// A new company starts empty until the owner explicitly selects an
    /// intelligence connection. Empty never means "infer from host state".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    /// Certified harness used by Exec and lead conversations.
    #[serde(default)]
    pub coordination_harness: AgentHarness,
    /// Certified harness used by productive Staff Attempts. The legacy
    /// `worker_runtime` key is accepted during migration but never emitted.
    #[serde(default, alias = "worker_runtime")]
    pub worker_harness: AgentHarness,
    /// Exact provider-supported reasoning effort for every actor launch.
    #[serde(default = "default_reasoning_effort")]
    pub reasoning_effort: String,
    /// Ordered provider-qualified fallbacks for the singleton Exec. Empty is
    /// an explicit no-fallback policy; providers are never inferred from
    /// ambient credentials or broker history.
    #[serde(default)]
    pub model_failover: Vec<String>,
    /// A cheaper model for conversation turns and for routine first Attempts the decision model
    /// judges a small model can do well (S102). The primary `model` stays the failover and takes
    /// every repair. Unset: every turn uses `model`. Cheaper means cached input: about 93% of input
    /// is cached, and laguna-xs ($0.03/M cached) cost twice laguna-s ($0.009/M) in the S101 run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub everyday_model: Option<String>,
    /// The decision model that gates Exec wakes and tiers Work (S102), called on OpenRouter's
    /// decisions API with the `model.inference` credential, e.g. `typesafe/jev-1.14`. Unset: the
    /// hosted relay decides when this plane has one; otherwise no decisions run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_model: Option<String>,
    /// Named binding → `credential_reference`, e.g.
    /// `resend.production = "infisical:/companies/aris/RESEND_API_KEY"`.
    /// Only a governed child process that names the binding receives it.
    #[serde(default)]
    pub credentials: std::collections::BTreeMap<String, String>,
    /// Legacy S03 approval input. At daemon boot these values migrate into the
    /// Authority-owned governance store and this list is purged. It remains in
    /// the parser only so upgrading cannot silently discard an existing grant.
    #[serde(default)]
    pub approved_parties: Vec<String>,
}

/// Whether a company's spend ceiling is one running total or a budget for each UTC month.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpendPeriod {
    #[default]
    Total,
    Monthly,
}

impl SpendPeriod {
    #[must_use]
    pub fn is_total(&self) -> bool {
        *self == Self::Total
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Total => "total",
            Self::Monthly => "monthly",
        }
    }
}

/// The Inbox warns when spend reaches this share of the ceiling, unless the owner chose another.
pub const DEFAULT_SPEND_WARNING_PERCENT: u8 = 80;

fn default_ceiling() -> SpendCeiling {
    SpendCeiling::from_micro_usd(10_000_000)
}

fn default_reasoning_effort() -> String {
    crate::acp::DEFAULT_REASONING_EFFORT.to_string()
}

fn valid_reasoning_effort(value: &str) -> bool {
    matches!(
        value,
        "none" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
    )
}

const LEGACY_UNCONFIGURED_MODEL: &str = "unconfigured/pending";

/// Native harness IDs are internal route markers, not direct provider IDs.
/// Keep rejecting them at model write boundaries while leaving old config
/// readable so an owner can repair it through Company → Intelligence.
pub fn validate_company_model_selection(model: &str) -> Result<()> {
    validate_direct_provider(provider_for_model(model)?)
}

fn provider_for_model(model: &str) -> Result<&str> {
    let Some((provider, id)) = model.split_once('/') else {
        bail!("model {model:?} must be provider-qualified, e.g. moonshot/kimi-k3");
    };
    if provider.is_empty()
        || id.trim().is_empty()
        || !provider
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        bail!("invalid provider-qualified model {model:?}");
    }
    Ok(provider)
}

/// A direct connection must name a real provider, never an internal native
/// harness route marker.
pub fn validate_direct_provider(provider: &str) -> Result<()> {
    if provider.starts_with("native-") {
        bail!("Native harness models must be selected through Company → Intelligence.");
    }
    Ok(())
}

/// The provider a route's connection serves, when it is one provider's models rather than a
/// harness with its own model names.
pub fn route_provider(connection: &str) -> Option<&str> {
    connection
        .strip_prefix("direct:")
        .or_else(|| account_intelligence_route(connection).map(|(provider, _, _)| provider))
}

impl CompanyConfig {
    pub fn configured_model(&self) -> Option<&str> {
        let model = self.model.as_str();
        (!model.trim().is_empty() && model != LEGACY_UNCONFIGURED_MODEL).then_some(model)
    }

    pub fn has_configured_model_route(&self) -> bool {
        self.configured_model().is_some()
            || self
                .agent_intelligence
                .values()
                .any(|route| !route.connection.trim().is_empty() && !route.model.trim().is_empty())
    }

    pub fn has_effective_model_route(&self, harness: AgentHarness) -> bool {
        self.configured_model().is_some() || self.native_model(harness).is_some()
    }

    /// Resolve an agent override or the company default into an execution route.
    pub fn for_agent(&self, actor: &str) -> Self {
        let mut config = self.clone();
        let override_route = actor != "default" && self.agent_intelligence.contains_key(actor);
        if let Some(route) = self
            .agent_intelligence
            .get(actor)
            .or_else(|| self.agent_intelligence.get("default"))
        {
            // An owner's choice for one agent is exact. The company default keeps the owner's
            // ordered fallbacks, so a primary on quota or outage does not stop the company.
            if override_route {
                config.model_failover.clear();
            }
            if let Some(provider) = route.connection.strip_prefix("direct:") {
                config.model = if route.model.starts_with(&format!("{provider}/")) {
                    route.model.clone()
                } else {
                    format!("{provider}/{}", route.model)
                };
                config.coordination_harness = AgentHarness::RestlessManaged;
                config.worker_harness = AgentHarness::RestlessManaged;
            } else if let Some((provider, _, harness)) =
                account_intelligence_route(&route.connection)
            {
                config.model = if route.model.starts_with(&format!("{provider}/")) {
                    route.model.clone()
                } else {
                    format!("{provider}/{}", route.model)
                };
                config.coordination_harness = harness;
                config.worker_harness = harness;
                // The same company may retain an older CLI sign-in for an
                // explicit rollback. This assignment uses the account relay.
                config.native_harnesses.remove(harness.as_str());
            } else if let Some(id) = route.connection.strip_prefix("harness:custom:") {
                config.model = format!("native-custom-{id}/{}", route.model);
                config.coordination_harness = AgentHarness::CustomAcp;
                config.worker_harness = AgentHarness::CustomAcp;
                config.reasoning_effort = "default".into();
            } else if let Some(id) = route.connection.strip_prefix("harness:") {
                if let Some(harness) = AgentHarness::parse_canonical(id) {
                    config.coordination_harness = harness;
                    config.worker_harness = harness;
                    if let Some(native) = config.native_harnesses.get_mut(id) {
                        native.model = route.model.clone();
                    }
                }
            }
            let primary = config.model.clone();
            config.model_failover.retain(|model| *model != primary);
        }
        config
    }
    /// The model an actor runs on. An owner's choice for this agent is exact. An agent on the
    /// company default may run any model of that default's provider that Exec chose for it (Opus
    /// for a hard redo, Haiku for a sweep): the connection serves them all, so no new grant is
    /// needed. A model of another provider falls back to the default.
    pub fn agent_preference<'a>(
        &'a self,
        actor: &str,
        previous: Option<&'a str>,
    ) -> Option<&'a str> {
        if self.agent_intelligence.contains_key(actor) {
            Some(&self.model)
        } else if let Some(route) = self.agent_intelligence.get("default") {
            let same_provider =
                route_provider(&route.connection)
                    .zip(previous)
                    .and_then(|(provider, model)| {
                        model
                            .trim()
                            .split_once('/')
                            .filter(|(prefix, id)| *prefix == provider && !id.trim().is_empty())
                            .map(|_| model.trim())
                    });
            Some(same_provider.unwrap_or(&self.model))
        } else {
            // Older Staff runs could persist a native harness marker as an
            // actor preference. It is not a direct route after a harness change.
            previous.filter(|model| validate_company_model_selection(model).is_ok())
        }
    }

    pub fn native_model(&self, harness: AgentHarness) -> Option<String> {
        if harness == AgentHarness::CustomAcp {
            return Some(self.model.clone());
        }
        let connection = self.native_harnesses.get(harness.as_str())?;
        let provider = match harness {
            AgentHarness::Codex => "codex",
            AgentHarness::ClaudeAgent => "claude",
            _ => return None,
        };
        let mode = if connection.mode == "api_key" {
            "api"
        } else {
            "oauth"
        };
        Some(format!("native-{provider}-{mode}/{}", connection.model))
    }

    pub fn load(root: &Path, name: &str) -> Result<Self> {
        Self::load_from(root.join("companies").join(format!("{name}.toml")), name)
    }

    pub fn load_archived(root: &Path, name: &str) -> Result<Self> {
        Self::load_from(
            root.join("archived-companies").join(format!("{name}.toml")),
            name,
        )
    }

    fn load_from(path: PathBuf, name: &str) -> Result<Self> {
        validate_company_name(name)?;
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            // Owner surfaces show this; a host path means nothing there and discloses the layout.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                bail!("There is no company called {name}.")
            }
            Err(error) => {
                return Err(error).with_context(|| format!("read {}", path.display()));
            }
        };
        let mut config: Self =
            toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
        crate::credential::migrate_bindings(&mut config.credentials);
        if config.name != name {
            bail!(
                "company config name mismatch: file {name}.toml says {}",
                config.name
            );
        }
        config.model_candidates()?;
        config.validate_harness_models()?;
        if !valid_reasoning_effort(&config.reasoning_effort) {
            bail!("unsupported reasoning effort {:?}", config.reasoning_effort);
        }
        config.validate_resource_policies()?;
        Ok(config)
    }

    /// Write the config back. Used by the bounded company/credential CLI and
    /// by one-time approval migration cleanup.
    ///
    /// Writes to a temporary file and renames, because the alternative — a
    /// truncating write interrupted midway — leaves the company with no config
    /// at all, and a company that cannot load its config cannot be woken to be
    /// told why. Rename within a directory is atomic on every filesystem we run
    /// on.
    pub fn save(root: &Path, config: &Self) -> Result<()> {
        let mut canonical = config.clone();
        crate::credential::migrate_bindings(&mut canonical.credentials);
        let config = &canonical;
        validate_company_name(&config.name)?;
        config.model_candidates()?;
        config.validate_harness_models()?;
        if !valid_reasoning_effort(&config.reasoning_effort) {
            bail!("unsupported reasoning effort {:?}", config.reasoning_effort);
        }
        config.validate_resource_policies()?;
        let dir = root.join("companies");
        let path = dir.join(format!("{}.toml", config.name));
        let archived = root
            .join("archived-companies")
            .join(format!("{}.toml", config.name));
        if archived.exists() && !path.exists() {
            bail!(
                "company {} is archived; restore it instead of creating a second company with the same identity",
                config.name
            );
        }
        let temporary = dir.join(format!(".{}.toml.tmp", config.name));
        let rendered = toml::to_string_pretty(config).context("render company config")?;
        std::fs::write(&temporary, rendered)
            .with_context(|| format!("write {}", temporary.display()))?;
        std::fs::rename(&temporary, &path)
            .with_context(|| format!("replace {}", path.display()))?;
        Ok(())
    }

    /// The sleep timeout this company actually runs with; `None` is always on.
    pub fn sleep_after(&self) -> Option<std::time::Duration> {
        match self
            .auto_sleep_after_minutes
            .unwrap_or(DEFAULT_SLEEP_AFTER_MINUTES)
        {
            0 => None,
            minutes => Some(std::time::Duration::from_secs(u64::from(minutes) * 60)),
        }
    }

    fn validate_resource_policies(&self) -> Result<()> {
        if self
            .monthly_runtime_cap_hours
            .is_some_and(|hours| !(1..=744).contains(&hours))
        {
            bail!("monthly_runtime_cap_hours must be between 1 and 744 hours");
        }
        if self.native_monthly_turn_limit == Some(0) || self.native_monthly_token_limit == Some(0) {
            bail!("native usage limits must be at least 1, or absent for no limit");
        }
        if self
            .auto_sleep_after_minutes
            .is_some_and(|minutes| minutes > MAX_SLEEP_AFTER_MINUTES)
        {
            bail!("auto_sleep_after_minutes must be 0 (never) or 1 to 1440 minutes");
        }
        Ok(())
    }

    /// Primary followed by the exact owner-configured fallback order. The
    /// closed validation here protects both TOML and CLI writes.
    pub fn model_candidates(&self) -> Result<Vec<&str>> {
        let Some(primary) = self.configured_model() else {
            if self.model_failover.is_empty() {
                return Ok(Vec::new());
            }
            bail!("model failover candidates require a configured primary model");
        };
        let mut seen = std::collections::BTreeSet::new();
        let mut candidates = Vec::with_capacity(1 + self.model_failover.len());
        for model in std::iter::once(primary).chain(self.model_failover.iter().map(String::as_str))
        {
            provider_for_model(model)?;
            if !seen.insert(model) {
                bail!("duplicate model candidate {model:?}");
            }
            candidates.push(model);
        }
        Ok(candidates)
    }

    /// Every model a turn may run on: the candidates, then the everyday model (S102).
    pub fn routed_models(&self) -> Result<Vec<&str>> {
        let mut models = self.model_candidates()?;
        if let Some(everyday) = self
            .everyday_model
            .as_deref()
            .map(str::trim)
            .filter(|model| !model.is_empty())
        {
            provider_for_model(everyday)?;
            if !models.contains(&everyday) {
                models.push(everyday);
            }
        }
        Ok(models)
    }

    pub fn validate_harness_models(&self) -> Result<()> {
        let models = self.model_candidates()?;
        for harness in [self.coordination_harness, self.worker_harness] {
            if let Some(connection) = self.native_harnesses.get(harness.as_str()) {
                if !matches!(
                    connection.mode.as_str(),
                    "oauth" | "api_key" | "disconnected"
                ) || connection.model.trim().is_empty()
                    || connection.model.contains('/')
                {
                    bail!("invalid native harness connection");
                }
                continue;
            }
            if harness == AgentHarness::CustomAcp {
                crate::custom_harness::split_model(&self.model)?;
                continue;
            }
            let required_provider = match harness {
                AgentHarness::RestlessManaged | AgentHarness::CustomAcp => continue,
                AgentHarness::Codex => "litellm or openai-codex",
                AgentHarness::ClaudeAgent => "anthropic",
            };
            if let Some(model) = models.iter().find(|model| match harness {
                AgentHarness::Codex => {
                    !model.starts_with("litellm/") && !model.starts_with("openai-codex/")
                }
                _ => !model.starts_with(&format!("{required_provider}/")),
            }) {
                bail!(
                    "{} harness requires every configured model to use provider {required_provider}; got {model}",
                    harness.as_str()
                );
            }
            if let Some(model) = models.iter().find(|model| match harness {
                AgentHarness::Codex => {
                    !model.starts_with("openai-codex/")
                        && !crate::model_gateway::responses_model_has_pinned_tariff(model)
                }
                AgentHarness::ClaudeAgent => {
                    !crate::model_gateway::anthropic_model_has_pinned_tariff(model)
                }
                AgentHarness::RestlessManaged | AgentHarness::CustomAcp => false,
            }) {
                bail!(
                    "{} harness has no pinned metering tariff for configured model {model}",
                    harness.as_str()
                );
            }
            if harness == AgentHarness::ClaudeAgent
                && !matches!(
                    self.reasoning_effort.as_str(),
                    "low" | "medium" | "high" | "max"
                )
            {
                bail!(
                    "Claude Agent reasoning_effort must be low, medium, high, or max; got {:?}",
                    self.reasoning_effort
                );
            }
        }
        Ok(())
    }
}

/// A company logo is small: the cockpit draws it at 40px and agents copy it.
pub const COMPANY_LOGO_MAX_BYTES: usize = 1024 * 1024;

/// A logo's type from its leading bytes, never from a file name or a header,
/// as `(extension, media type)`. Only raster images: SVG can carry script.
pub fn sniff_company_logo(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("png", "image/png"))
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("jpg", "image/jpeg"))
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(("webp", "image/webp"))
    } else {
        None
    }
}

/// The owner's logo as the host keeps it, beside the company config.
pub struct CompanyLogo {
    pub path: PathBuf,
    /// The content digest in the file name; changes whenever the logo does.
    pub version: String,
    pub extension: &'static str,
    pub media_type: &'static str,
}

fn company_logo_prefix(company: &str) -> String {
    format!("{company}.logo-")
}

/// The recorded logo, when the config names one in the exact shape
/// `<company>.logo-<16 hex>.<png|jpg|webp>`. Any other value is ignored,
/// never joined to a path.
pub fn company_logo(root: &Path, config: &CompanyConfig) -> Option<CompanyLogo> {
    let name = config.logo.as_deref()?;
    let rest = name.strip_prefix(&company_logo_prefix(&config.name))?;
    let (version, extension) = rest.split_once('.')?;
    if version.len() != 16
        || !version
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return None;
    }
    let (extension, media_type) = match extension {
        "png" => ("png", "image/png"),
        "jpg" => ("jpg", "image/jpeg"),
        "webp" => ("webp", "image/webp"),
        _ => return None,
    };
    Some(CompanyLogo {
        path: root.join("companies").join(name),
        version: version.to_string(),
        extension,
        media_type,
    })
}

/// Write a logo beside the company config and return the file name to record
/// in [`CompanyConfig::logo`]. The name carries the content digest, so a new
/// logo never reuses an old name a browser may have cached.
pub fn store_company_logo(root: &Path, company: &str, bytes: &[u8]) -> Result<String> {
    validate_company_name(company)?;
    if bytes.is_empty() || bytes.len() > COMPANY_LOGO_MAX_BYTES {
        bail!("a company logo must be at most 1 MB");
    }
    let Some((extension, _)) = sniff_company_logo(bytes) else {
        bail!("a company logo must be a PNG, JPEG or WebP image");
    };
    let digest = format!("{:x}", Sha256::digest(bytes));
    let name = format!(
        "{}{}.{extension}",
        company_logo_prefix(company),
        &digest[..16]
    );
    let dir = root.join("companies");
    let temporary = dir.join(format!(".{company}.logo.tmp"));
    std::fs::write(&temporary, bytes).with_context(|| format!("write {}", temporary.display()))?;
    std::fs::rename(&temporary, dir.join(&name))
        .with_context(|| format!("store logo for {company}"))?;
    Ok(name)
}

/// Remove this company's logo files other than `keep`, including any a
/// crashed replacement left behind.
pub fn remove_company_logos(root: &Path, company: &str, keep: Option<&str>) -> Result<()> {
    validate_company_name(company)?;
    let prefix = company_logo_prefix(company);
    let dir = root.join("companies");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("read {}", dir.display())),
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with(&prefix) && Some(name) != keep {
            std::fs::remove_file(entry.path())
                .with_context(|| format!("remove old logo {name}"))?;
        }
    }
    Ok(())
}

/// Mirror the owner's logo to `/company/brand/logo.<ext>` for agents, as the
/// mission is mirrored: an earlier copy is replaced, or removed with the logo.
/// It runs as the company account, so a link planted under `/company` cannot
/// turn this write into one outside the company's own files.
async fn seed_brand(config: &CompanyConfig) -> Result<()> {
    let logo = company_logo(&state_root(), config);
    let bytes = match &logo {
        Some(logo) => match std::fs::read(&logo.path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error).context("read company logo"),
        },
        None => None,
    };
    let extension = match (&logo, &bytes) {
        (Some(logo), Some(_)) => logo.extension,
        _ => "",
    };
    let name = container_name(&config.name);
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            "-u",
            "company",
            &name,
            "sh",
            "-c",
            "set -e; rm -f /company/brand/logo.png /company/brand/logo.jpg /company/brand/logo.webp; \
             if [ -n \"$1\" ]; then mkdir -p /company/brand; cat > \"/company/brand/logo.$1\"; fi",
            "restless-brand",
            extension,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn docker exec for logo seed")?;
    let mut stdin = child.stdin.take().expect("piped");
    if let Some(bytes) = &bytes {
        stdin.write_all(bytes).await?;
    }
    drop(stdin);
    let out = child.wait_with_output().await?;
    if !out.status.success() {
        bail!("logo seed failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(())
}

/// Refresh the agents' copy of the logo now when the computer is running; a
/// sleeping one receives it on its next `up`, without being woken for it.
pub async fn sync_brand_projection(config: &CompanyConfig) -> Result<&'static str> {
    match status(&config.name).await? {
        ContainerStatus::Running => {
            seed_brand(config).await?;
            Ok("updated")
        }
        ContainerStatus::Stopped | ContainerStatus::Absent => Ok("deferred"),
    }
}

#[cfg(test)]
mod computer_container_tests {
    use super::*;

    #[test]
    fn the_computer_sits_beside_its_runtime_and_owns_the_desktop_programs() {
        if std::env::var("RESTLESS_RESOURCE_NAMESPACE").is_err() {
            assert_eq!(computer_container_name("reno"), "restless-pc-reno");
            assert_eq!(container_name("reno"), "restless-co-reno");
        }
        for program in [
            "desktop",
            "openbox",
            "desktop-panel",
            "chromium",
            "browser-broker",
            "desktop-web",
        ] {
            assert!(DESKTOP_PROGRAMS.contains(&program), "{program}");
        }
        assert!(!DESKTOP_PROGRAMS.contains(&"release-health"));
    }
}

#[cfg(test)]
mod build_input_tests {
    use super::same_build_input_label;

    #[test]
    fn only_two_real_equal_labels_keep_a_computer() {
        let digest = "a".repeat(64);
        assert!(same_build_input_label(
            Some(&digest),
            Some(&format!("{digest}\n"))
        ));
        assert!(!same_build_input_label(
            Some(&digest),
            Some(&"b".repeat(64))
        ));
        assert!(!same_build_input_label(
            Some("<no value>"),
            Some("<no value>")
        ));
        assert!(!same_build_input_label(None, Some(&digest)));
        assert!(!same_build_input_label(Some(""), Some("")));
    }
}

#[cfg(test)]
mod browser_locale_tests {
    use super::valid_browser_locale;

    #[test]
    fn only_timezones_and_language_tags_reach_the_company_browser() {
        for ok in [
            "Australia/Sydney",
            "en-AU",
            "America/Argentina/Buenos_Aires",
            "Etc/GMT+10",
        ] {
            assert!(valid_browser_locale(ok), "{ok}");
        }
        for bad in ["", "   ", "en AU", "x;rm -rf /", "a=b", &"A".repeat(65)] {
            assert!(!valid_browser_locale(bad), "{bad}");
        }
    }
}

#[cfg(test)]
mod company_logo_tests {
    use super::*;

    /// The logo is served under the type its bytes show, so markup or script
    /// dressed up as an image is never stored as a logo.
    #[test]
    fn only_raster_images_are_accepted_as_logos() {
        assert_eq!(
            sniff_company_logo(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"),
            Some(("png", "image/png"))
        );
        assert_eq!(
            sniff_company_logo(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(("jpg", "image/jpeg"))
        );
        assert_eq!(
            sniff_company_logo(b"RIFF\x24\0\0\0WEBPVP8 "),
            Some(("webp", "image/webp"))
        );
        for refused in [
            &b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script/></svg>"[..],
            b"<html><script>alert(1)</script>",
            b"GIF89a",
            b"RIFF\x24\0\0\0WAVE",
            b"",
        ] {
            assert_eq!(sniff_company_logo(refused), None);
        }
        let root = std::env::temp_dir().join(format!("logo-{}", Uuid::new_v4().simple()));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        assert!(store_company_logo(&root, "aris", b"<svg/>").is_err());
        assert!(
            store_company_logo(&root, "aris", &vec![0xFF; COMPANY_LOGO_MAX_BYTES + 1]).is_err()
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A config value is read back only in the exact shape this module writes,
    /// so an edited config cannot point the logo route at another file.
    #[test]
    fn a_recorded_logo_never_names_a_file_outside_its_own() {
        let root = Path::new("/plane");
        let mut config: CompanyConfig = toml::from_str("name = \"aris\"").unwrap();
        config.logo = Some("aris.logo-0123456789abcdef.png".into());
        let logo = company_logo(root, &config).unwrap();
        assert_eq!(
            logo.path,
            root.join("companies/aris.logo-0123456789abcdef.png")
        );
        assert_eq!(
            (logo.version.as_str(), logo.media_type),
            ("0123456789abcdef", "image/png")
        );
        for refused in [
            "../../etc/passwd",
            "aris.logo-0123456789abcdef.png/../../secret.toml",
            "aris.logo-../../../../etc/x.png",
            "bob.logo-0123456789abcdef.png",
            "aris.logo-0123456789ABCDEF.png",
            "aris.logo-0123456789abcdef.svg",
            "aris.logo-0123456789abcdef.png.png",
            "aris.toml",
        ] {
            config.logo = Some(refused.into());
            assert!(company_logo(root, &config).is_none(), "{refused}");
        }
    }
}

/// Names preserved outside the active config directory. Archived companies
/// remain inspectable and recoverable, but normal daemon scans cannot wake
/// them merely because their config still exists.
pub fn archived_company_names(root: &Path) -> Result<Vec<String>> {
    let directory = root.join("archived-companies");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut companies = Vec::new();
    for entry in
        std::fs::read_dir(&directory).with_context(|| format!("read {}", directory.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("toml") {
            if let Some(name) = path.file_stem().and_then(|value| value.to_str()) {
                validate_company_name(name)?;
                companies.push(name.to_string());
            }
        }
    }
    companies.sort();
    Ok(companies)
}

/// Archive is the owner-facing removal path. Stop execution, then atomically
/// move only the authority-owned identity/config marker. Runtime files,
/// OrgIntel, Authority records and spend remain in place for recovery.
pub async fn archive(root: &Path, company: &str) -> Result<String> {
    CompanyConfig::load(root, company)?;
    down(company).await?;
    move_active_config_to_archive(root, company)?;
    Ok(format!(
        "{company}: archived (runtime stopped; files and history preserved)"
    ))
}

fn move_active_config_to_archive(root: &Path, company: &str) -> Result<()> {
    CompanyConfig::load(root, company)?;
    let source = root.join("companies").join(format!("{company}.toml"));
    let directory = root.join("archived-companies");
    let destination = directory.join(format!("{company}.toml"));
    if destination.exists() {
        bail!("company {company} already has an archived config");
    }
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("create archive directory {}", directory.display()))?;
    std::fs::rename(&source, &destination).with_context(|| {
        format!(
            "archive company config {} as {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

/// Return an archived identity to the active portfolio. Restoring does not
/// start the runtime: the owner can inspect it first and resume deliberately.
pub fn restore(root: &Path, company: &str) -> Result<String> {
    move_archived_config_to_active(root, company)?;
    Ok(format!(
        "{company}: restored to the portfolio (runtime remains stopped)"
    ))
}

fn move_archived_config_to_active(root: &Path, company: &str) -> Result<()> {
    CompanyConfig::load_archived(root, company)?;
    let source = root
        .join("archived-companies")
        .join(format!("{company}.toml"));
    let destination = root.join("companies").join(format!("{company}.toml"));
    if destination.exists() {
        bail!("company {company} already has an active config");
    }
    std::fs::rename(&source, &destination).with_context(|| {
        format!(
            "restore company config {} as {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

pub fn validate_company_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 63
        || !name.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                byte.is_ascii_lowercase()
            } else {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
            }
        })
    {
        bail!(
            "invalid company name {name:?}: use lowercase letters, digits or underscores, starting with a letter"
        );
    }
    Ok(())
}

/// Top-level paths the cockpit and the owner API claim on every plane. A company with one of these
/// ids could be created, but its cockpit at `/<id>` would never open, because the static route wins.
/// Keep in step with `web/src/routes` and the owner router.
pub const RESERVED_COMPANY_NAMES: &[&str] = &[
    "account",
    "api",
    "company",
    "connections",
    "desktop",
    "documents",
    "gallery",
    "health",
    "review",
];

/// The rule for a company being created: a valid name that no app route shadows. Existing
/// companies are read with [`validate_company_name`] alone, so one created before this rule still
/// loads and can be renamed or archived.
pub fn validate_new_company_name(name: &str) -> Result<()> {
    validate_company_name(name)?;
    if RESERVED_COMPANY_NAMES.contains(&name) {
        bail!("invalid company name {name:?}: it is reserved for an app page; choose another");
    }
    Ok(())
}

/// The company computer runs in its own container (S104 F2): the desktop and the company browser,
/// in the Runtime's network namespace. `RESTLESS_COMPUTER_CONTAINER=0` keeps them in the Runtime.
pub fn computer_split() -> bool {
    std::env::var("RESTLESS_COMPUTER_CONTAINER").map_or(true, |value| value.trim() != "0")
}

/// The supervisor that runs the desktop and the company browser.
fn desktop_supervisor_config() -> &'static str {
    if computer_split() {
        COMPUTER_SUPERVISOR_CONFIG
    } else {
        COMPANY_SUPERVISOR_CONFIG
    }
}

/// The company computer's own container, beside its Runtime.
pub fn computer_container_name(company: &str) -> String {
    container_name(company).replacen("-co-", "-pc-", 1)
}

async fn container_state(name: &str) -> Result<ContainerStatus> {
    let out = docker_observe(&["inspect", "-f", "{{.State.Status}}", name]).await?;
    if !out.status.success() {
        return Ok(ContainerStatus::Absent);
    }
    Ok(match String::from_utf8_lossy(&out.stdout).trim() {
        "running" => ContainerStatus::Running,
        _ => ContainerStatus::Stopped,
    })
}

/// Start the company computer beside its running Runtime: created on first use, started when it
/// was stopped, and replaced when the Runtime it shared a network with was replaced.
async fn ensure_computer(config: &CompanyConfig) -> Result<()> {
    if !computer_split() {
        return Ok(());
    }
    let workload_parent = workload_cgroup_parent()?;
    let company = &config.name;
    let runtime = container_name(company);
    let runtime_id = inspect_value(&["inspect", "-f", "{{.Id}}", &runtime])
        .await?
        .context("the company Runtime is not running")?;
    let runtime_image = inspect_value(&["inspect", "-f", "{{.Config.Image}}", &runtime])
        .await?
        .context("the company Runtime has no image")?;
    let name = computer_container_name(company);
    let owner_label = format!("{{{{index .Config.Labels \"{COMPUTER_OF_LABEL}\"}}}}");
    let attached = inspect_value(&["inspect", "-f", &owner_label, &name]).await?;
    // The browser takes its home sites when it starts: a changed list gives a fresh computer.
    let sites_format = format!("{{{{index .Config.Labels \"{HOME_SITES_LABEL}\"}}}}");
    let sites_now = inspect_value(&["inspect", "-f", &sites_format, &name])
        .await?
        .filter(|value| value != "<no value>")
        .unwrap_or_default();
    let sites_current = sites_now == config.home_sites.join(",");
    let parent_current = container_uses_workload_parent(&name).await?;
    match attached.as_deref() {
        Some(id) if id == runtime_id && sites_current && parent_current => {
            if container_state(&name).await? != ContainerStatus::Running {
                run_ok(&["start", &name]).await?;
            }
            return Ok(());
        }
        Some(_) => {
            remove_computer(company).await;
        }
        None => {}
    }
    let network = format!("container:{runtime}");
    let computer_of = format!("{COMPUTER_OF_LABEL}={runtime_id}");
    let company_label = format!("io.restless.company={company}");
    let namespace_label = format!(
        "io.restless.namespace={}",
        std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default()
    );
    let company_env = format!("RESTLESS_COMPANY={company}");
    let volume_mount = format!("{}:/company", volume_name(company));
    let cpus = company_cpus();
    let memory = computer_memory();
    let pids = resource_bound("RESTLESS_COMPANY_PIDS_LIMIT", DEFAULT_PIDS_LIMIT);
    let mut browser_env = browser_environment();
    // Home sites (S99 step 2): the browser reaches the plane's home egress proxy at the Runtime's
    // gateway, since a container sharing the Runtime's network has no host.docker.internal entry.
    let home_sites = config.home_sites.join(",");
    if !config.home_sites.is_empty() {
        let gateway = inspect_value(&[
            "inspect",
            "-f",
            "{{range .NetworkSettings.Networks}}{{.Gateway}}{{end}}",
            &runtime,
        ])
        .await?
        .filter(|_| cfg!(target_os = "linux"))
        .unwrap_or_else(|| "host.docker.internal".into());
        browser_env.push(format!("RESTLESS_HOME_SITES={home_sites}"));
        browser_env.push(format!(
            "RESTLESS_HOME_PROXY={gateway}:{}",
            crate::port_with_offset(crate::home_egress::HOME_EGRESS_PORT)?
        ));
    }
    let sites_label = format!("{HOME_SITES_LABEL}={home_sites}");
    let mut args: Vec<&str> = vec![
        "run",
        "-d",
        "--name",
        &name,
        "--network",
        &network,
        "--log-driver",
        "local",
        "--log-opt",
        "max-size=10m",
        "--log-opt",
        "max-file=3",
        "--cpu-shares",
        "512",
    ];
    if let Some(parent) = workload_parent.as_deref() {
        args.extend(["--cgroup-parent", parent]);
    }
    if let Some(cpus) = cpus.as_deref() {
        args.extend(["--cpus", cpus]);
    }
    if let Some(memory) = memory.as_deref() {
        args.extend(["--memory", memory, "--memory-swap", memory]);
    }
    if let Some(pids) = pids.as_deref() {
        args.extend(["--pids-limit", pids]);
    }
    for setting in &browser_env {
        args.extend(["--env", setting.as_str()]);
    }
    args.extend([
        "--label",
        &computer_of,
        "--label",
        &sites_label,
        "--label",
        &company_label,
        "--label",
        &namespace_label,
        "-e",
        &company_env,
        "-v",
        &volume_mount,
        "--entrypoint",
        "/usr/local/bin/start-computer-supervisor",
        &runtime_image,
    ]);
    run_ok(&args).await?;
    Ok(())
}

/// Give a running company its computer as the config now says (home sites changed).
pub async fn refresh_computer(config: &CompanyConfig) -> Result<()> {
    let _start = company_start_guard(&config.name).await;
    ensure_computer(config).await
}

/// Stop the company computer, giving Chrome time to flush its profile.
async fn stop_computer(company: &str) {
    let name = computer_container_name(company);
    if container_state(&name).await.ok() == Some(ContainerStatus::Running) {
        let _ = run_ok(&["stop", "--time", "30", &name]).await;
    }
}

/// Remove the company computer's container; its files stay on the company volume.
async fn remove_computer(company: &str) {
    stop_computer(company).await;
    let _ = docker_observe(&["rm", "-f", &computer_container_name(company)]).await;
}

const COMPUTER_OF_LABEL: &str = "io.restless.computer-of";
const HOME_SITES_LABEL: &str = "io.restless.home-sites";

/// The programs that make the company computer: the desktop, its window manager and panel, the
/// company browser, its broker and the desktop's web viewer.
const DESKTOP_PROGRAMS: &[&str] = &[
    "desktop",
    "openbox",
    "desktop-panel",
    "chromium",
    "browser-broker",
    "desktop-web",
];

/// The owner's timezone and language for the company browser (S104 F3): the plane's environment
/// wins, else what the owner's own browser reported. Unset keeps the browser as it was.
fn browser_environment() -> Vec<String> {
    let reported = load_browser_locale();
    let mut settings: Vec<String> = [
        ("RESTLESS_BROWSER_TIMEZONE", reported.timezone),
        ("RESTLESS_BROWSER_LANG", reported.language),
    ]
    .into_iter()
    .filter_map(|(name, reported)| {
        let value = std::env::var(name)
            .ok()
            .filter(|value| valid_browser_locale(value))
            .or(reported.filter(|value| valid_browser_locale(value)))?;
        Some(format!("{name}={}", value.trim()))
    })
    .collect();
    // Keep the broker's documented idle policy configurable in the split computer too.
    // Forward a numeric value only; zero intentionally disables automatic sleeping.
    if let Ok(value) = std::env::var("RESTLESS_BROWSER_IDLE_MS") {
        if let Ok(milliseconds) = value.trim().parse::<u32>() {
            settings.push(format!("RESTLESS_BROWSER_IDLE_MS={milliseconds}"));
        }
    }
    settings
}

pub fn container_name(company: &str) -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => format!("restless-{namespace}-co-{company}"),
        _ => format!("restless-co-{company}"),
    }
}

/// Remove a volume once the local MCP workers that used it have finished
/// removing themselves: a `--rm` worker still exiting holds it for a moment.
async fn remove_volume_when_free(volume: &str, what: &str) -> Result<()> {
    let mut attempts = 0;
    loop {
        let output = docker_observe(&["volume", "rm", volume]).await?;
        if output.status.success() {
            return Ok(());
        }
        attempts += 1;
        if attempts >= 40 {
            bail!(
                "remove {what} {volume}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

pub fn volume_name(company: &str) -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => format!("restless-{namespace}-vol-{company}"),
        _ => format!("restless-vol-{company}"),
    }
}

/// Where a company's kept files (Library uploads, files sent in messages) live, so they outlast
/// the computer's shell. They were in the container's own layer, and every replacement (a new
/// image on release) lost them: Sydney Resale's uploads were gone from its computer on 19 October.
pub fn attachments_volume_name(company: &str) -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => format!("restless-{namespace}-att-{company}"),
        _ => format!("restless-att-{company}"),
    }
}

const OWNER_ATTACHMENT_ROOT: &str = "/var/lib/restless-owner-attachments";

/// Whether this company's computer keeps files on their own volume, so a file it does not have
/// is truly gone rather than waiting in an older shell.
pub async fn kept_files_are_durable(company: &str) -> bool {
    container_mounts_attachments(company).await.unwrap_or(false)
}

async fn container_mounts_attachments(company: &str) -> Result<bool> {
    let mounted = inspect_value(&[
        "inspect",
        "-f",
        &format!("{{{{range .Mounts}}}}{{{{if eq .Destination \"{OWNER_ATTACHMENT_ROOT}\"}}}}{{{{.Name}}}}{{{{end}}}}{{{{end}}}}"),
        &container_name(company),
    ])
    .await?;
    Ok(mounted.as_deref() == Some(attachments_volume_name(company).as_str()))
}

/// Before a shell that keeps attachments in its own layer is replaced, copy them into the
/// attachments volume so the replacement mounts them. Runs once per company: a shell that
/// already mounts the volume has nothing of its own to carry.
async fn preserve_attachments(company: &str, image: &str) -> Result<()> {
    let volume = attachments_volume_name(company);
    run_ok(&["volume", "create", &volume]).await?;
    if container_mounts_attachments(company).await.unwrap_or(false) {
        return Ok(());
    }
    let name = container_name(company);
    // `docker cp` reads a stopped container too, as a tar stream holding the directory itself.
    let mut source = tokio::process::Command::new("docker")
        .args(["cp", &format!("{name}:{OWNER_ATTACHMENT_ROOT}"), "-"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn docker cp for kept files")?;
    let mount = format!("{volume}:{OWNER_ATTACHMENT_ROOT}");
    let mut sink = tokio::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "-i",
            "--user",
            "0",
            "--network",
            "none",
            "-v",
            &mount,
            "--entrypoint",
            "tar",
            image,
            "-x",
            "-p",
            "-C",
            "/var/lib",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .context("spawn the copy of kept files")?;
    let mut from = source.stdout.take().expect("piped");
    let mut to = sink.stdin.take().expect("piped");
    let copied = tokio::io::copy(&mut from, &mut to).await;
    drop(to);
    let source_status = source.wait().await?;
    let sink_status = sink.wait().await?;
    // No directory yet (a computer that never kept a file) is nothing to carry.
    if !source_status.success() {
        return Ok(());
    }
    copied.context("copy kept files")?;
    if !sink_status.success() {
        bail!("kept files could not be copied into {volume}; the old computer was left in place");
    }
    Ok(())
}

/// The package cache shared by a company's host-side local MCP workers. It is
/// company data like the Runtime volume, so it is namespaced the same way and
/// removed with the company.
/// One local tool's own data, kept across its runs.
pub fn mcp_data_volume_name(company: &str, connection: &str) -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => {
            format!("restless-{namespace}-mcp-data-{company}-{connection}")
        }
        _ => format!("restless-mcp-data-{company}-{connection}"),
    }
}

/// Create a tool's data volume, labelled so backups and company removal find it.
pub async fn ensure_mcp_data_volume(company: &str, connection: &str) -> Result<()> {
    let volume = mcp_data_volume_name(company, connection);
    if docker_observe(&["volume", "inspect", &volume])
        .await?
        .status
        .success()
    {
        return Ok(());
    }
    let profile = std::env::var("RESTLESS_PROFILE").unwrap_or_else(|_| "stable".into());
    let namespace = std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default();
    let output = docker(&[
        "volume",
        "create",
        "--label",
        &format!("io.restless.profile={profile}"),
        "--label",
        &format!("io.restless.namespace={namespace}"),
        "--label",
        &format!("io.restless.company={company}"),
        "--label",
        &format!("io.restless.mcp-data={connection}"),
        &volume,
    ])
    .await?;
    if !output.status.success() {
        bail!(
            "create tool data {volume}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

pub fn mcp_cache_volume_name(company: &str) -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => {
            format!("restless-{namespace}-mcp-cache-{company}")
        }
        _ => format!("restless-mcp-cache-{company}"),
    }
}

/// A company computer with no demand sleeps after this long unless its owner
/// chose another timeout. Sleep keeps the volume; demand wakes it.
pub const DEFAULT_SLEEP_AFTER_MINUTES: u16 = 30;
pub const MAX_SLEEP_AFTER_MINUTES: u16 = 1440;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ContainerStatus {
    Running,
    Stopped,
    Absent,
}

/// Whether the running company computer is built from the current Restless
/// source. `Unknown` is not collapsed into `Current`: a missing source tree or
/// an unlabelled old image is precisely the version-skew case `doctor` exists
/// to make visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationStatus {
    Current,
    Required,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeDoctor {
    pub company: String,
    pub container: ContainerStatus,
    pub volume: String,
    pub volume_exists: bool,
    pub volume_mounted: bool,
    pub image: String,
    pub container_image_id: Option<String>,
    pub target_image_id: Option<String>,
    pub source_digest: Option<String>,
    pub image_source_digest: Option<String>,
    /// The container's image differs from the target's by id only: both name the same build
    /// inputs, so the computer is current.
    #[serde(default)]
    pub same_build_inputs: bool,
    pub reconciliation: ReconciliationStatus,
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release: Option<RuntimeReleaseIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coordination: Option<CoordinationDoctor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supervisor: Option<SupervisorDoctor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser: Option<BrowserDoctor>,
    pub collaboration_tools: Vec<CollaborationToolDoctor>,
    /// Disk, memory and CPU pressure inside a running computer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceDoctor>,
}

/// What the computer has left: free space on the company volume, memory against its limit,
/// processes the kernel killed for memory since it started, and how long work waited for CPU.
/// Each field is absent when this host does not expose it.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct ResourceDoctor {
    pub disk_free_bytes: Option<u64>,
    pub disk_total_bytes: Option<u64>,
    pub memory_used_bytes: Option<u64>,
    pub memory_limit_bytes: Option<u64>,
    pub oom_kills: Option<u64>,
    /// Share of the last minute some work waited for CPU (cgroup v2 PSI `some avg60`).
    pub cpu_pressure_percent: Option<f64>,
}

impl ResourceDoctor {
    /// Parse the probe's `key value` lines; anything missing stays unknown.
    pub fn parse(text: &str) -> Self {
        let mut doctor = Self::default();
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let (Some(key), Some(value)) = (parts.next(), parts.next()) else {
                continue;
            };
            match key {
                "disk_total_kb" => {
                    doctor.disk_total_bytes = value.parse::<u64>().ok().map(|kb| kb * 1024)
                }
                "disk_free_kb" => {
                    doctor.disk_free_bytes = value.parse::<u64>().ok().map(|kb| kb * 1024)
                }
                "memory_current" => doctor.memory_used_bytes = value.parse().ok(),
                "memory_max" => doctor.memory_limit_bytes = value.parse().ok(),
                "oom_kill" => doctor.oom_kills = value.parse().ok(),
                "cpu_some_avg60" => doctor.cpu_pressure_percent = value.parse().ok(),
                _ => {}
            }
        }
        doctor
    }

    /// The plain sentence for the first resource that is running low, if any.
    pub fn pressure(&self) -> Option<String> {
        if let (Some(free), Some(total)) = (self.disk_free_bytes, self.disk_total_bytes) {
            if total > 0 && (free < 1024 * 1024 * 1024 || free * 10 < total) {
                return Some(format!(
                    "Company files have {} free of {}. Ask Exec to clear old downloads and build output, or a build or download may fail.",
                    human_bytes(free),
                    human_bytes(total)
                ));
            }
        }
        if self.oom_kills.unwrap_or(0) > 0 {
            return Some(format!(
                "{} process{} ran out of memory and was stopped since the computer started. Restart the computer to clear it, and ask Exec to run fewer heavy jobs at once.",
                self.oom_kills.unwrap_or(0),
                if self.oom_kills == Some(1) { "" } else { "es" }
            ));
        }
        if let (Some(used), Some(limit)) = (self.memory_used_bytes, self.memory_limit_bytes) {
            if limit > 0 && used * 10 > limit * 9 {
                return Some(format!(
                    "Memory is nearly full ({} of {}). Restart the computer if apps slow down or close.",
                    human_bytes(used),
                    human_bytes(limit)
                ));
            }
        }
        if self.cpu_pressure_percent.is_some_and(|share| share >= 80.0) {
            return Some(
                "The processor has been busy for most of the last minute, so work is slow. It eases when the running jobs finish."
                    .into(),
            );
        }
        None
    }
}

fn human_bytes(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let gib = bytes as f64 / GIB;
    if gib >= 1.0 {
        format!("{gib:.1} GB")
    } else {
        format!("{:.0} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// One read of the computer's disk, memory, OOM and CPU pressure; bounded, never fatal.
async fn resource_doctor(company: &str) -> Option<ResourceDoctor> {
    const PROBE: &str = r#"set -- $(df -Pk /company 2>/dev/null | tail -n 1)
[ -n "$2" ] && echo "disk_total_kb $2" && echo "disk_free_kb $4"
[ -r /sys/fs/cgroup/memory.current ] && echo "memory_current $(cat /sys/fs/cgroup/memory.current)"
m=$(cat /sys/fs/cgroup/memory.max 2>/dev/null); [ -n "$m" ] && [ "$m" != max ] && echo "memory_max $m"
grep -s '^oom_kill ' /sys/fs/cgroup/memory.events
sed -n 's/^some avg10=[^ ]* avg60=\([^ ]*\).*/cpu_some_avg60 \1/p' /sys/fs/cgroup/cpu.pressure 2>/dev/null
true"#;
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        docker_diagnostic(&["exec", &container_name(company), "sh", "-c", PROBE]),
    )
    .await
    .ok()?
    .ok()?;
    output
        .status
        .success()
        .then(|| ResourceDoctor::parse(&String::from_utf8_lossy(&output.stdout)))
}

#[derive(Debug, Clone, Serialize)]
pub struct CollaborationToolDoctor {
    pub tool: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeReleaseIdentity {
    pub core_version: String,
    pub source_revision: String,
    pub api_contract_version: u32,
    pub assertion_contract_version: u32,
    pub schema_version: u32,
    pub harnesses: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub harness_agents: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub harness_dependencies: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RuntimeReleaseHealth {
    status: String,
    release: RuntimeReleaseIdentity,
}

/// Observation of the Runtime's bounded, authenticated coordination path.
///
/// This deliberately performs an ordinary read through the Runtime Bridge
/// rather than inferring availability from a running container or its process
/// supervisor. It is a health observation, not another Runtime lifecycle.
#[derive(Debug, Clone, Serialize)]
pub struct CoordinationDoctor {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupervisorDoctor {
    pub status: String,
    pub services: Vec<SupervisedService>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SupervisedService {
    pub name: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserDoctor {
    pub status: String,
    pub desktop: String,
    pub chromium: String,
    pub automation: String,
    pub web_transport: String,
    pub controller: String,
}

async fn docker(args: &[&str]) -> Result<std::process::Output> {
    tokio::process::Command::new("docker")
        .args(args)
        // Reap the host CLI when its caller is cancelled. Remote diagnostic
        // processes have their own deadline in docker_observe.
        .kill_on_drop(true)
        .output()
        .await
        .context("spawn docker")
}

const DOCKER_OBSERVATION_TIMEOUT: Duration = Duration::from_secs(8);

/// Run one bounded, read-only Docker observation for an owner-facing path.
///
/// Docker Desktop can accept a CLI request and then leave it waiting forever
/// on its VM. A cockpit refresh must degrade that one observation rather than
/// retain a daemon child and an HTTP request indefinitely. `docker` marks the
/// child kill-on-drop, so expiry also reaps the opaque CLI process.
pub async fn docker_observe(args: &[&str]) -> Result<std::process::Output> {
    if STARTUP_RECOVERY_ACTIVE.load(Ordering::SeqCst) {
        bail!("Runtime observation is deferred while startup recovery owns Docker");
    }
    docker_bounded(args, DOCKER_OBSERVATION_TIMEOUT)
        .await
        .context("bounded Docker observation")
}

/// Read-only health checks may defer under memory pressure. Lifecycle and
/// user operations retain their own existing deadlines and admission rules.
async fn docker_diagnostic(args: &[&str]) -> Result<std::process::Output> {
    if STARTUP_RECOVERY_ACTIVE.load(Ordering::SeqCst) {
        bail!("Runtime observation is deferred while startup recovery owns Docker");
    }
    bound_docker_call(
        async {
            if let Some(index) = observation_exec_index(args)? {
                let name = args[index];
                let program = args
                    .get(index + 1)
                    .context("Docker exec command is missing")?;
                let slot = EXEC_OBSERVATIONS
                    .lock()
                    .await
                    .entry((name.to_string(), program.to_string()))
                    .or_default()
                    .clone();
                observe_exec(&slot, async {
                    if let Some(path) = workload_container_scope(name).await? {
                        anyhow::ensure!(
                            !shared_workload_admission()?
                                .is_some_and(|sample| sample.waiting_for_memory),
                            "shared diagnostics are waiting for memory"
                        );
                        anyhow::ensure!(
                            !container_admission(&path)?
                                .is_some_and(|sample| sample.waiting_for_memory),
                            "company diagnostics are waiting for memory"
                        );
                    }
                    if name.starts_with(&container_name("")) {
                        // Bound the process inside our Ubuntu Runtime too. Killing
                        // the host CLI alone does not stop an already-started exec.
                        let mut bounded: Vec<String> = args.iter().map(|s| s.to_string()).collect();
                        bounded.splice(
                            index + 1..index + 1,
                            ["timeout", "--signal=TERM", "--kill-after=1s", "6s"]
                                .map(str::to_string),
                        );
                        let refs: Vec<&str> = bounded.iter().map(String::as_str).collect();
                        let output = docker(&refs).await?;
                        anyhow::ensure!(
                            !matches!(output.status.code(), Some(124 | 137)),
                            "company diagnostic exceeded its Runtime deadline"
                        );
                        Ok(output)
                    } else {
                        docker(args).await
                    }
                })
                .await
            } else {
                docker(args).await
            }
        },
        DOCKER_OBSERVATION_TIMEOUT,
    )
    .await
    .context("bounded Docker observation")
}

/// Only parse Docker's exec prefix; commands and their arguments remain opaque.
fn observation_exec_index(args: &[&str]) -> Result<Option<usize>> {
    if args.first() != Some(&"exec") {
        return Ok(None);
    }
    let mut index = 1;
    while let Some(value) = args.get(index) {
        match *value {
            "--" => {
                return args
                    .get(index + 1)
                    .map(|_| Some(index + 1))
                    .context("Docker exec target is missing")
            }
            "-u" | "--user" | "-w" | "--workdir" | "-e" | "--env" | "--env-file"
            | "--detach-keys" => index += 2,
            "-i" | "-t" | "-it" | "-ti" | "--interactive" | "--tty" | "-d" | "--detach"
            | "--privileged" => index += 1,
            value if value.starts_with("--") && value.contains('=') => index += 1,
            value if value.starts_with('-') => bail!("unsupported diagnostic exec option"),
            _ => return Ok(Some(index)),
        }
    }
    bail!("Docker exec target is missing")
}

async fn observe_exec<T>(
    slot: &ExecObservationSlot,
    call: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    let mut retry_after = slot.lock().await;
    anyhow::ensure!(
        retry_after.is_none_or(|at| Instant::now() >= at),
        "company diagnostics are waiting after an interrupted probe"
    );
    // Arm before polling: dropping this future must not clear its retry window.
    *retry_after = Some(Instant::now() + EXEC_OBSERVATION_RETRY);
    let result = call.await;
    if result.is_ok() {
        *retry_after = None;
    }
    result
}

pub async fn docker_bounded(args: &[&str], bound: Duration) -> Result<std::process::Output> {
    bound_docker_call(docker(args), bound).await
}

async fn bound_docker_call<T, F>(call: F, bound: Duration) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    tokio::time::timeout(bound, call)
        .await
        .with_context(|| format!("docker command exceeded {} seconds", bound.as_secs_f64()))?
}

pub async fn status(company: &str) -> Result<ContainerStatus> {
    let name = container_name(company);
    let out = docker_observe(&["inspect", "-f", "{{.State.Status}}", &name]).await?;
    if !out.status.success() {
        return Ok(ContainerStatus::Absent);
    }
    let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(match state.as_str() {
        "running" => ContainerStatus::Running,
        _ => ContainerStatus::Stopped,
    })
}

/// Observe all running configured Runtime shells with one Docker round trip.
/// Startup cost therefore stays constant when the owner retains many stopped
/// or historical company definitions.
pub async fn running_configured_companies(configs: &[CompanyConfig]) -> Result<Vec<String>> {
    // Recovery runs behind a closed admission gate and outside appliance
    // readiness. Give a congested Docker Desktop enough time to answer once;
    // unlike owner-facing probes, this does not hold an HTTP request or the
    // control-plane socket hostage.
    let output = docker_bounded(&["ps", "--format", "{{.Names}}"], Duration::from_secs(30)).await?;
    if !output.status.success() {
        bail!(
            "inspect running company Runtimes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(running_company_names(
        configs,
        &String::from_utf8_lossy(&output.stdout),
    ))
}

/// Observe the entire owner catalogue in one Docker round trip. Catalogue
/// rendering is a projection, so an unavailable Docker backend yields
/// `unavailable` rows rather than N sequential inspections and a blank UI.
pub async fn configured_company_statuses(
    configs: &[CompanyConfig],
) -> Result<std::collections::BTreeMap<String, ContainerStatus>> {
    let output = docker_observe(&["ps", "-a", "--format", "{{.Names}}\t{{.State}}"]).await?;
    if !output.status.success() {
        bail!(
            "inspect configured company Runtimes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(company_statuses(
        configs,
        &String::from_utf8_lossy(&output.stdout),
    ))
}

/// Share one Docker inventory across open company catalogs. An explicit
/// lifecycle change clears it, while unrelated tabs reuse the recent result.
pub async fn cockpit_company_statuses(
    configs: &[CompanyConfig],
) -> Result<std::collections::BTreeMap<String, ContainerStatus>> {
    let mut companies = configs
        .iter()
        .map(|config| config.name.clone())
        .collect::<Vec<_>>();
    companies.sort();
    let slot = COMPANY_STATUSES_CACHE
        .lock()
        .await
        .entry(companies)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(None)))
        .clone();
    let mut cached = slot.lock().await;
    if let Some((at, result)) = cached.as_ref() {
        if at.elapsed() < Duration::from_secs(10) {
            return Ok(result.clone());
        }
    }
    let result = configured_company_statuses(configs).await?;
    *cached = Some((Instant::now(), result.clone()));
    Ok(result)
}

fn company_statuses(
    configs: &[CompanyConfig],
    docker_rows: &str,
) -> std::collections::BTreeMap<String, ContainerStatus> {
    let observed = docker_rows
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect::<std::collections::BTreeMap<_, _>>();
    configs
        .iter()
        .map(|config| {
            let status = match observed.get(container_name(&config.name).as_str()) {
                Some(state) if *state == "running" => ContainerStatus::Running,
                Some(_) => ContainerStatus::Stopped,
                None => ContainerStatus::Absent,
            };
            (config.name.clone(), status)
        })
        .collect()
}

fn running_company_names(configs: &[CompanyConfig], docker_names: &str) -> Vec<String> {
    let running = docker_names
        .lines()
        .collect::<std::collections::BTreeSet<_>>();
    configs
        .iter()
        .filter(|config| running.contains(container_name(&config.name).as_str()))
        .map(|config| config.name.clone())
        .collect()
}

/// Create if absent, start if stopped, no-op if running. With `reconcile`,
/// fetch the configured company image and replace an outdated container while
/// keeping its named volume. Building and publishing that image belongs to the
/// release/Fleet path, not to the credential-holding account plane.
/// Bring a stopped (asleep) company onto this release without waking it: its old container is
/// removed, keeping the company volume and its kept files, so the next wake creates the new one.
/// Rebuilding by starting it woke every sleeping company at once on each release, and four awake
/// companies thrashed a 4 GB plane (22 October). True when a stale container was retired.
pub async fn retire_stale_stopped(config: &CompanyConfig) -> Result<bool> {
    let company = &config.name;
    let _start = company_start_guard(company).await;
    if status(company).await? != ContainerStatus::Stopped || !is_sleeping(company) {
        return Ok(false);
    }
    if !(container_uses_old_image(company).await?
        || !container_uses_company_volume(company).await?
        || !container_mounts_attachments(company).await?)
    {
        return Ok(false);
    }
    let image = company_image();
    ensure_image_available(&image).await?;
    // Kept files move to their own volume before the shell that held them goes; on failure the
    // old computer stays as it was.
    preserve_attachments(company, &image).await?;
    remove_computer(company).await;
    run_ok(&["rm", &container_name(company)]).await?;
    Ok(true)
}

pub async fn up(config: &CompanyConfig, reconcile: bool) -> Result<String> {
    let outcome = up_locked(config, reconcile).await;
    if outcome.is_ok() {
        clear_sleeping(&config.name);
    }
    outcome
}

async fn up_locked(config: &CompanyConfig, reconcile: bool) -> Result<String> {
    let company = &config.name;
    let _start = company_start_guard(company).await;
    let internal_network = if config.internal_network {
        Some(ensure_company_internal_network(company).await?)
    } else {
        None
    };
    // The computer must boot before native sign-in can happen. Model admission
    // belongs to the session boundary, not to creation of the company computer.
    let image = company_image();
    let mut fetched = false;
    let mut replaced = false;
    workload_cgroup_parent()?;
    if reconcile {
        fetched = ensure_image_available(&image).await?;
        if status(company).await? != ContainerStatus::Absent
            && (container_uses_old_image(company).await?
                || !container_uses_company_volume(company).await?
                || !container_mounts_attachments(company).await?
                || !container_uses_workload_parent(&container_name(company)).await?)
        {
            // Reconciliation replaces the shell. A configured monthly hours
            // cap must gate that replacement before it stops live work.
            crate::runtime_usage::ensure_start_allowed(&state_root(), config).await?;
            let name = container_name(company);
            // Give supervised Chromium long enough to flush its persistent
            // profile before replacing the shell. Docker's ten-second default
            // is shorter than Chromium's configured 20-second stop window and
            // was observed dropping a persistent cookie during reconciliation.
            remove_computer(company).await;
            if status(company).await? == ContainerStatus::Running {
                run_ok(&["stop", "--time", "30", &name]).await?;
                // Record the exact FinishedAt before replacing the container,
                // so the next generation is not mistaken for a missed run.
                if config.monthly_runtime_cap_hours.is_some() {
                    crate::runtime_usage::observe(&state_root(), config).await?;
                }
            }
            // Kept files move to their own volume before the shell that held them goes.
            if let Err(error) = preserve_attachments(company, &image).await {
                // Never remove the only copy: the old computer comes back as it was.
                let _ = run_ok(&["start", &name]).await;
                return Err(error);
            }
            // Only the replaceable container is removed; the named company
            // volume remains the durable computer (§13.4).
            run_ok(&["rm", &name]).await?;
            replaced = true;
        }
    }
    match status(company).await? {
        ContainerStatus::Running => {
            if let Some(network) = internal_network.as_deref() {
                ensure_container_on_network(company, network).await?;
            }
            // A running company made under an older, larger bound gets this host's now; Docker
            // applies it live.
            refit_container_cpus(&container_name(company)).await;
        }
        ContainerStatus::Stopped => {
            if !replaced {
                crate::runtime_usage::ensure_start_allowed(&state_root(), config).await?;
            }
            let name = container_name(company);
            if let Some(network) = internal_network.as_deref() {
                ensure_container_on_network(company, network).await?;
            }
            refit_container_cpus(&name).await;
            run_ok(&["start", &name]).await?;
        }
        ContainerStatus::Absent => {
            if !replaced {
                crate::runtime_usage::ensure_start_allowed(&state_root(), config).await?;
            }
            let volume = volume_name(company);
            let profile = std::env::var("RESTLESS_PROFILE").unwrap_or_else(|_| "stable".into());
            let namespace = std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default();
            let profile_label = format!("io.restless.profile={profile}");
            let namespace_label = format!("io.restless.namespace={namespace}");
            run_ok(&[
                "volume",
                "create",
                "--label",
                &profile_label,
                "--label",
                &namespace_label,
                &volume,
            ])
            .await?;
            let name = container_name(company);
            let cpus = company_cpus();
            let memory = company_memory();
            let pids = resource_bound("RESTLESS_COMPANY_PIDS_LIMIT", DEFAULT_PIDS_LIMIT);
            // Keep one noisy company from filling the shared host disk. The
            // local driver also supports `docker logs` for diagnosis.
            let mut args: Vec<&str> = vec![
                "run",
                "-d",
                "--name",
                &name,
                "--hostname",
                company,
                "--log-driver",
                "local",
                "--log-opt",
                "max-size=10m",
                "--log-opt",
                "max-file=3",
            ];
            if cfg!(target_os = "linux") && internal_network.is_none() {
                args.extend(["--add-host", "host.docker.internal:host-gateway"]);
            }
            let workload_parent = workload_cgroup_parent()?;
            if let Some(parent) = workload_parent.as_deref() {
                args.extend(["--cgroup-parent", parent]);
            }
            if let Some(cpus) = cpus.as_deref() {
                args.extend(["--cpus", cpus]);
            }
            // Under contention a company's work (builds, simulations) yields to the plane, which
            // serves every page and chat: half the default weight. Idle CPU is still all theirs.
            args.extend(["--cpu-shares", "512"]);
            if let Some(memory) = memory.as_deref() {
                // --memory-swap equal to --memory denies the container swap, so
                // a runaway is OOM-killed inside its own cgroup instead of
                // pushing the shared VM — and then the host — into swap thrash.
                args.extend(["--memory", memory, "--memory-swap", memory]);
            }
            if let Some(pids) = pids.as_deref() {
                args.extend(["--pids-limit", pids]);
            }
            let company_env = format!("RESTLESS_COMPANY={company}");
            let browser_env = browser_environment();
            for setting in &browser_env {
                args.extend(["--env", setting.as_str()]);
            }
            // The desktop and browser run in the computer's own container (S104 F2).
            if computer_split() {
                args.extend(["--env", "RESTLESS_DESKTOP_AUTOSTART=false"]);
            }
            // The image default names the appliance's established port, but an
            // isolated account plane may use RESTLESS_PORT_OFFSET.  ACP turns
            // already override this value; the persistent Runtime must receive
            // the same endpoint so ordinary bridge tools and doctor do not
            // silently talk to another plane (or fail while agent turns work).
            let coordinator_env = format!("RESTLESS_COORDINATOR={}", crate::runtime_coordinator()?);
            let volume_mount = format!("{volume}:/company");
            let attachments = attachments_volume_name(company);
            run_ok(&[
                "volume",
                "create",
                "--label",
                &profile_label,
                "--label",
                &namespace_label,
                &attachments,
            ])
            .await?;
            let attachments_mount = format!("{attachments}:{OWNER_ATTACHMENT_ROOT}");
            if let Some(network) = internal_network.as_deref() {
                args.extend(["--network", network]);
                // Native model calls from an isolated test Runtime may leave
                // only through its narrow, separately managed model sidecar.
                args.extend(["-e", "HTTPS_PROXY=http://host.docker.internal:8080"]);
                // Test authentication must disappear with this Runtime even
                // when the disposable company volume is retained for evidence.
                args.extend([
                    "--tmpfs",
                    "/company/home:rw,uid=2000,gid=2000,mode=0700,size=256m",
                ]);
            }
            args.extend([
                "--label",
                &profile_label,
                "--label",
                &namespace_label,
                "-e",
                &company_env,
                "-e",
                &coordinator_env,
                "-v",
                &volume_mount,
                "-v",
                &attachments_mount,
                &image,
            ]);
            run_ok(&args).await?;
        }
    }
    // Docker Desktop supplies this DNS name; native Linux Docker needs an
    // explicit mapping. Also repair computers created before the flag existed.
    if cfg!(target_os = "linux") && internal_network.is_none() {
        let name = container_name(company);
        if !docker_observe(&["exec", &name, "getent", "hosts", "host.docker.internal"])
            .await?
            .status
            .success()
        {
            if let Some(gateway) = inspect_value(&[
                "inspect",
                "-f",
                "{{range .NetworkSettings.Networks}}{{.Gateway}}{{end}}",
                &name,
            ])
            .await?
            {
                let address: std::net::IpAddr = gateway
                    .trim()
                    .parse()
                    .context("observe Linux Docker host gateway")?;
                run_ok(&[
                    "exec",
                    &name,
                    "sh",
                    "-c",
                    "printf '%s host.docker.internal\\n' \"$1\" >> /etc/hosts",
                    "restless-host-gateway",
                    &address.to_string(),
                ])
                .await?;
            }
        }
    }
    // The company computer beside its Runtime (S104 F2). A computer that fails to start leaves the
    // agents working; the doctor reports the missing desktop.
    if let Err(error) = ensure_computer(config).await {
        tracing::warn!(company = %config.name, error = %format!("{error:#}"), "company computer did not start");
    }
    seed_mission(config).await?;
    // A logo agents cannot copy is a lesser company, not a broken one.
    if let Err(error) = seed_brand(config).await {
        tracing::warn!(company = %config.name, error = %format!("{error:#}"), "logo seed failed");
    }
    let suffix = match (fetched, replaced) {
        (true, true) => " (image fetched; container replaced; volume kept)",
        (true, false) => " (image fetched)",
        (false, true) => " (container replaced; volume kept)",
        (false, false) if reconcile => " (runtime already current)",
        (false, false) => "",
    };
    invalidate_cockpit_health(company).await;
    Ok(format!("{}: running{suffix}", config.name))
}

const INTERNAL_NETWORK_LABEL: &str = "io.restless.company";
const STARTUP_DOCKER_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) fn company_network_name(company: &str) -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => {
            format!("restless-{namespace}-net-{company}")
        }
        _ => format!("restless-net-{company}"),
    }
}

/// Ensure the opt-in network exists and is exactly the internal network
/// owned by this company. Never reuse a same-named Docker network with
/// different routing or ownership metadata.
async fn ensure_company_internal_network(company: &str) -> Result<String> {
    let network = company_network_name(company);
    let inspect = [
        "network",
        "inspect",
        "-f",
        "{{.Internal}}\t{{index .Labels \"io.restless.company\"}}\t{{index .Labels \"io.restless.namespace\"}}\t{{index .Labels \"io.restless.profile\"}}",
        network.as_str(),
    ];
    let observed = docker_bounded(&inspect, STARTUP_DOCKER_TIMEOUT).await?;
    if !observed.status.success() {
        let profile = std::env::var("RESTLESS_PROFILE").unwrap_or_else(|_| "stable".into());
        let namespace = std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default();
        let internal_label = format!("{INTERNAL_NETWORK_LABEL}={company}");
        let profile_label = format!("io.restless.profile={profile}");
        let namespace_label = format!("io.restless.namespace={namespace}");
        // A concurrent creator may win this race. Regardless of create's
        // result, re-inspect below and accept only the exact expected shape.
        let _ = docker_bounded(
            &[
                "network",
                "create",
                "--driver",
                "bridge",
                "--internal",
                "--label",
                &internal_label,
                "--label",
                &profile_label,
                "--label",
                &namespace_label,
                &network,
            ],
            STARTUP_DOCKER_TIMEOUT,
        )
        .await?;
    }
    let observed = docker_bounded(&inspect, STARTUP_DOCKER_TIMEOUT).await?;
    if !observed.status.success() {
        bail!("configured internal Docker network {network} is absent or cannot be inspected");
    }
    let actual = String::from_utf8_lossy(&observed.stdout);
    let mut fields = actual.trim().split('\t');
    let is_internal = fields.next() == Some("true");
    let owner = fields.next();
    let namespace = fields.next();
    let expected_namespace = std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default();
    let profile = std::env::var("RESTLESS_PROFILE").unwrap_or_else(|_| "stable".into());
    let actual_profile = fields.next();
    if !is_internal
        || owner != Some(company)
        || namespace != Some(expected_namespace.as_str())
        || actual_profile != Some(profile.as_str())
    {
        bail!("configured Docker network {network} is not the labeled internal network for company {company}");
    }
    Ok(network)
}

async fn ensure_container_on_network(company: &str, network: &str) -> Result<()> {
    let name = container_name(company);
    let template = "{{range $name, $config := .NetworkSettings.Networks}}{{$name}}\n{{end}}";
    let observed =
        docker_bounded(&["inspect", "-f", template, &name], STARTUP_DOCKER_TIMEOUT).await?;
    if !observed.status.success() {
        bail!("cannot inspect company Runtime {name} network attachments; refusing to start it");
    }
    let attachments = String::from_utf8_lossy(&observed.stdout);
    let attached = attachments
        .lines()
        .map(str::trim)
        .filter(|attached| !attached.is_empty())
        .collect::<Vec<_>>();
    if attached.len() != 1 || attached[0] != network {
        bail!("company Runtime {name} must be attached only to its configured internal network {network}; observed {attached:?}");
    }
    Ok(())
}

/// Materialise the company-scoped bridge grant after the Runtime is known to
/// be running. The token travels over docker stdin rather than argv, and is
/// only readable by the company group inside its persistent computer.
pub async fn install_runtime_bridge_capability(company: &str, capability: &str) -> Result<()> {
    if capability.is_empty() || capability.bytes().any(|byte| byte.is_ascii_whitespace()) {
        bail!("refusing an invalid Runtime bridge capability");
    }
    let container = container_name(company);
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            "-u",
            "company",
            &container,
            "sh",
            "-c",
            "set -eu; dir=/company/run; mkdir -p \"$dir\"; umask 007; tmp=\"$dir/.restless-bridge.cap.$$\"; trap 'rm -f \"$tmp\"' EXIT; cat > \"$tmp\"; chmod 0640 \"$tmp\"; mv \"$tmp\" \"$dir/restless-bridge.cap\"; trap - EXIT",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("start Runtime bridge capability install")?;
    let mut stdin = child
        .stdin
        .take()
        .context("open Runtime bridge capability stdin")?;
    stdin.write_all(capability.as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.shutdown().await?;
    // `docker exec -i` keeps the remote `cat` alive while this pipe handle is
    // retained, even after Tokio has flushed it. Drop it before waiting so
    // the Company Runtime observes EOF and atomically installs the grant.
    drop(stdin);
    let output = child
        .wait_with_output()
        .await
        .context("finish Runtime bridge capability install")?;
    if !output.status.success() {
        bail!(
            "Runtime bridge capability install failed: {}",
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(300)
                .collect::<String>()
        );
    }
    Ok(())
}

/// Coalesce the expensive Runtime Doctor across owner tabs. Lifecycle actions
/// and the CLI call `doctor` directly, so recovery decisions remain fresh.
pub async fn cockpit_doctor(company: &str) -> Result<(RuntimeDoctor, DateTime<Utc>)> {
    let slot = COCKPIT_DOCTOR_CACHE
        .lock()
        .await
        .entry(company.to_string())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(None)))
        .clone();
    let Ok(mut cached) = slot.try_lock_owned() else {
        bail!("Company computer checks are already refreshing");
    };
    if let Some((at, result)) = cached.as_ref() {
        if at.elapsed() < Duration::from_secs(15) {
            return Ok(result.clone());
        }
    }
    let previous = cached.as_ref().map(|(_, result)| result.clone());
    let company = company.to_string();
    // Keep one bounded refresh alive when an owner read reaches its shorter
    // response deadline. It owns the cache guard, so further reads do not queue
    // tasks or cancel healthy probes and repeatedly arm their retry backoff.
    let refresh = tokio::spawn(async move {
        let result = (doctor(&company).await?, Utc::now());
        *cached = Some((Instant::now(), result.clone()));
        Ok(result)
    });
    // Owner refreshes can be farther apart than this cache's fresh interval.
    // Keep the last observation (including its actual timestamp) visible while
    // refreshing, rather than turning every slow refresh into unavailable.
    if let Some(previous) = previous {
        return Ok(previous);
    }
    refresh.await?
}

/// Check the replaceable runtime image independently of an agent report.
pub async fn doctor(company: &str) -> Result<RuntimeDoctor> {
    let image = company_image();
    let container = status(company).await?;
    let volume = volume_name(company);
    // Status decides whether container-specific fields can exist. The volume,
    // container details, and both image observations are then independent
    // read-only Docker probes; overlap them while retaining each probe's own
    // timeout and error handling.
    let image_source_format = format!("{{{{index .Config.Labels \"{SOURCE_DIGEST_LABEL}\"}}}}");
    let volume_probe = async {
        docker_observe(&["volume", "inspect", &volume])
            .await
            .map(|output| output.status.success())
    };
    let container_probe = async {
        if container == ContainerStatus::Absent {
            Ok((None, false))
        } else {
            inspect_container_image_and_volume(company).await
        }
    };
    let target_image_probe =
        async { inspect_value(&["image", "inspect", "-f", "{{.Id}}", &image]).await };
    let source_label_probe =
        async { inspect_value(&["image", "inspect", "-f", &image_source_format, &image]).await };
    let (volume_exists, container_details, target_image_id, image_source_digest) = tokio::join!(
        volume_probe,
        container_probe,
        target_image_probe,
        source_label_probe
    );
    // Resolve errors in the original observation order, even though the
    // independent subprocesses ran concurrently.
    let volume_exists = volume_exists?;
    let (container_image_id, volume_mounted) = container_details?;
    let target_image_id = target_image_id?;
    let image_source_digest = image_source_digest?.filter(|value| value != "<no value>");
    // A release that changed nothing the computer is built from (a UI-only release) still gives
    // the company image a new id. Keep the running computer when both images name the same
    // build inputs, so the release restarts no company (S104 A1).
    let same_build_inputs = match (&container_image_id, &target_image_id) {
        (Some(container_id), Some(target_id)) if container_id != target_id => {
            let label = format!("{{{{index .Config.Labels \"{BUILD_INPUT_LABEL}\"}}}}");
            let held_args = [
                "image",
                "inspect",
                "-f",
                label.as_str(),
                container_id.as_str(),
            ];
            let wanted_args = ["image", "inspect", "-f", label.as_str(), target_id.as_str()];
            let (held, wanted) =
                tokio::join!(inspect_value(&held_args), inspect_value(&wanted_args));
            same_build_input_label(
                held.ok().flatten().as_deref(),
                wanted.ok().flatten().as_deref(),
            )
        }
        _ => false,
    };
    // Source comparison is a local-development diagnostic only. A released
    // plane may contain no Core checkout and identifies the Runtime by the
    // configured OCI digest instead.
    let source_digest = (image == COMPANY_IMAGE)
        .then(|| {
            source_root()
                .ok()
                .and_then(|root| digest_source(&root).ok())
        })
        .flatten();

    let runtime_missing_or_stale = container == ContainerStatus::Absent
        || !volume_exists
        || !volume_mounted
        || (!same_build_inputs
            && matches!(
                (&container_image_id, &target_image_id),
                (Some(container_id), Some(target_id)) if container_id != target_id
            ))
        || matches!(
            (&source_digest, &image_source_digest),
            (Some(source), Some(image)) if source != image
        );
    let reconciliation = if runtime_missing_or_stale {
        ReconciliationStatus::Required
    } else if container_image_id.is_some()
        && target_image_id.is_some()
        && (image != COMPANY_IMAGE || (source_digest.is_some() && image_source_digest.is_some()))
    {
        ReconciliationStatus::Current
    } else {
        ReconciliationStatus::Unknown
    };

    let (release, coordination, supervisor, browser) = if container == ContainerStatus::Running {
        // These probes inspect independent Runtime surfaces. Browser service
        // state is derived from the supervisor result, so that one stays after
        // the parallel group.
        let (release, coordination, supervisor) = tokio::join!(
            release_identity_doctor(company),
            coordination_doctor(company),
            supervisor_doctor(company),
        );
        let browser = browser_doctor(company, &supervisor).await;
        (release, Some(coordination), Some(supervisor), Some(browser))
    } else {
        (None, None, None, None)
    };
    let coordination_requires_reconcile = container == ContainerStatus::Running
        && coordination
            .as_ref()
            .is_none_or(|value| value.status != "available");

    let mut collaboration_tools = Vec::new();
    let mut resources = None;
    if container == ContainerStatus::Running {
        let (document, room, observed) = tokio::join!(
            collaboration_tool_doctor(company, "document"),
            collaboration_tool_doctor(company, "room"),
            resource_doctor(company),
        );
        collaboration_tools.extend([document, room]);
        resources = observed;
    }
    Ok(RuntimeDoctor {
        collaboration_tools,
        company: company.to_string(),
        container,
        volume,
        volume_exists,
        volume_mounted,
        image,
        container_image_id,
        target_image_id,
        source_digest,
        image_source_digest,
        same_build_inputs,
        reconciliation,
        action: (reconciliation != ReconciliationStatus::Current
            || coordination_requires_reconcile)
            .then(|| format!("restless up -c {company} --reconcile")),
        release,
        coordination,
        supervisor,
        browser,
        resources,
    })
}

async fn collaboration_tool_doctor(company: &str, tool: &str) -> CollaborationToolDoctor {
    let probe = tokio::time::timeout(
        Duration::from_secs(5),
        docker_diagnostic(&[
            "exec",
            "-u",
            "company",
            &container_name(company),
            "restless",
            tool,
            "--help",
        ]),
    )
    .await;
    CollaborationToolDoctor {
        tool: tool.into(),
        installed: matches!(probe, Ok(Ok(output)) if output.status.success()),
    }
}

async fn release_identity_doctor(company: &str) -> Option<RuntimeReleaseIdentity> {
    let name = container_name(company);
    let probe = tokio::time::timeout(
        Duration::from_secs(3),
        docker_diagnostic(&[
            "exec",
            "-u",
            "company",
            &name,
            "curl",
            "--fail",
            "--silent",
            "--max-time",
            "2",
            "http://127.0.0.1:7789/health",
        ]),
    )
    .await
    .ok()?
    .ok()?;
    if !probe.status.success() || probe.stdout.len() > 64 * 1024 {
        return None;
    }
    let health: RuntimeReleaseHealth = serde_json::from_slice(&probe.stdout).ok()?;
    (health.status == "ok").then_some(health.release)
}

/// The container id is the V0 Runtime generation: it changes when the
/// replaceable shell changes and stays stable across ordinary process restarts.
pub async fn generation(company: &str) -> Result<Option<String>> {
    inspect_value(&["inspect", "-f", "{{.Id}}", &container_name(company)]).await
}

async fn coordination_doctor(company: &str) -> CoordinationDoctor {
    let name = container_name(company);
    let probe = tokio::time::timeout(
        Duration::from_secs(5),
        docker_diagnostic(&["exec", "-u", "company", &name, "restless", "status"]),
    )
    .await;
    match probe {
        Ok(Ok(output)) if output.status.success() => CoordinationDoctor {
            status: "available".into(),
            detail: None,
        },
        Ok(Ok(_)) => CoordinationDoctor {
            status: "degraded".into(),
            detail: Some(
                "The company computer could not complete a request to Restless. Check its connection and coordination grant; on Linux, also check Docker DNS and the host firewall."
                    .into(),
            ),
        },
        Ok(Err(_)) | Err(_) => CoordinationDoctor {
            status: "degraded".into(),
            detail: Some(
                "The company computer did not answer within five seconds. Check Docker and whether the host firewall permits its connection to Restless.".into(),
            ),
        },
    }
}

/// Reuse a recent browser probe when several open cockpit tabs ask together.
/// Browser controls themselves are still read live by the status endpoint.
pub async fn cockpit_browser_health(
    company: &str,
) -> Result<(ContainerStatus, Option<BrowserDoctor>)> {
    let slot = BROWSER_HEALTH_CACHE
        .lock()
        .await
        .entry(company.to_string())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(None)))
        .clone();
    let mut cached = slot.lock().await;
    if let Some((at, result)) = cached.as_ref() {
        if at.elapsed() < Duration::from_secs(10) {
            return Ok(result.clone());
        }
    }
    let result = browser_health(company).await?;
    *cached = Some((Instant::now(), result.clone()));
    Ok(result)
}

/// The cockpit needs live browser health, but not image reconciliation, source
/// hashing, or the collaboration probes performed by the full doctor.
pub async fn browser_health(company: &str) -> Result<(ContainerStatus, Option<BrowserDoctor>)> {
    let container = status(company).await?;
    let browser = if container == ContainerStatus::Running {
        let supervisor = supervisor_doctor(company).await;
        Some(browser_doctor(company, &supervisor).await)
    } else {
        None
    };
    Ok((container, browser))
}

async fn browser_doctor(company: &str, supervisor: &SupervisorDoctor) -> BrowserDoctor {
    let name = container_name(company);
    let process = |program: &str| -> String {
        supervisor
            .services
            .iter()
            .find(|service| service.name == program)
            .map(|service| {
                if service.state == "running" {
                    "available"
                } else {
                    "degraded"
                }
            })
            .unwrap_or("unavailable")
            .into()
    };
    let desktop = process("desktop");
    let mut chromium = process("chromium");
    let web_transport = process("desktop-web");
    let automation: String = match docker_diagnostic(&[
        "exec",
        &name,
        "curl",
        // Preserve the broker's structured 423 body. `--fail` discarded it,
        // so an intentional owner pause was misreported as a broken browser.
        "--fail-with-body",
        "--silent",
        "--max-time",
        "2",
        // A probe never wakes a sleeping browser (S104 F6); the broker says it is asleep.
        "--header",
        "x-restless-probe: 1",
        "http://127.0.0.1:9223/json/version",
    ])
    .await
    {
        Ok(output)
            if output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("\"asleep\":true") =>
        {
            chromium = "asleep".into();
            "asleep".into()
        }
        Ok(output) if output.status.success() => "available".into(),
        Ok(output) if String::from_utf8_lossy(&output.stdout).contains("owner_controls") => {
            "owner_paused".into()
        }
        _ => "unavailable".into(),
    };
    let controller = read_browser_control(company)
        .await
        .ok()
        .flatten()
        .and_then(|value| value["controller"].as_str().map(str::to_string))
        .unwrap_or_else(|| "unclaimed".into());
    // A browser asleep after idling is healthy: the next agent request or viewer wakes it.
    let status = if [&desktop, &web_transport]
        .iter()
        .all(|part| part.as_str() == "available")
        && matches!(chromium.as_str(), "available" | "asleep")
        && matches!(automation.as_str(), "available" | "owner_paused" | "asleep")
    {
        "available"
    } else {
        "degraded"
    }
    .into();
    BrowserDoctor {
        status,
        desktop,
        chromium,
        automation,
        web_transport,
        controller,
    }
}

async fn supervisor_doctor(company: &str) -> SupervisorDoctor {
    let name = container_name(company);
    let mut configs = vec![COMPANY_SUPERVISOR_CONFIG];
    if computer_split() {
        configs.push(COMPUTER_SUPERVISOR_CONFIG);
    }
    let mut services = Vec::new();
    for config in configs {
        let output =
            docker_diagnostic(&["exec", &name, "supervisorctl", "-c", config, "status"]).await;
        // supervisorctl exits nonzero when any optional program is stopped,
        // but its stdout still contains the status of every other program.
        let Ok(output) = output else {
            tracing::debug!(company, config, error = ?output.err(), "supervisor observation unavailable");
            continue;
        };
        services.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| {
                    let mut parts = line.split_whitespace();
                    Some(SupervisedService {
                        name: parts.next()?.to_string(),
                        state: parts.next()?.to_lowercase(),
                    })
                })
                // The Runtime keeps the desktop's programs, stopped, while the computer runs them.
                .filter(|service| {
                    config == COMPUTER_SUPERVISOR_CONFIG
                        || !computer_split()
                        || !DESKTOP_PROGRAMS.contains(&service.name.as_str())
                }),
        );
    }
    let status =
        // Chrome stopped on request is the browser asleep (S104 F6); a crash is BACKOFF, FATAL or
        // EXITED, never STOPPED.
        if !services.is_empty()
            && services.iter().all(|service| {
                service.state == "running" || (service.name == "chromium" && service.state == "stopped")
            })
        {
            "available"
        } else {
            "degraded"
        };
    SupervisorDoctor {
        status: status.to_string(),
        services,
    }
}

/// Where the image installs the noVNC web client: root-owned, so the company's own user cannot
/// change it.
const NOVNC_ROOT: &str = "/usr/share/novnc";

/// Whether `asset` names a file under the noVNC root: relative, plain characters, no `..`.
fn desktop_asset_path_ok(asset: &str) -> bool {
    !asset.is_empty()
        && asset.len() <= 200
        && !asset.starts_with('/')
        && asset
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && asset
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
}

/// One noVNC web client file, read from the image's own copy. The owner's browser runs it on the
/// cockpit's origin, so it must never come from the desktop port: that port is in the company's
/// network namespace, where an agent could answer it while websockify is down and serve its own
/// script to the owner.
pub async fn desktop_asset(company: &str, asset: &str) -> Result<Vec<u8>> {
    if !desktop_asset_path_ok(asset) {
        bail!("invalid desktop asset path");
    }
    let path = format!("{NOVNC_ROOT}/{asset}");
    let output = docker_diagnostic(&[
        "exec",
        "--user",
        "root",
        &container_name(company),
        "cat",
        "--",
        &path,
    ])
    .await?;
    if !output.status.success() {
        bail!(
            "desktop asset unavailable: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

/// Whether the desktop's web transport answers inside the company computer. Only its status is
/// read; what it serves never reaches a browser (see [`desktop_asset`]).
pub async fn desktop_web_ready(company: &str) -> bool {
    docker_diagnostic(&[
        "exec",
        &container_name(company),
        "curl",
        "--fail",
        "--silent",
        "--output",
        "/dev/null",
        "--max-time",
        "5",
        "http://127.0.0.1:6080/vnc.html",
    ])
    .await
    .is_ok_and(|output| output.status.success())
}

/// Open one exact web destination in a new tab of the persistent company
/// browser, then bring that tab to the front. The private browser broker is
/// the authority for the control lease: it returns `owner_controls` while an
/// owner has live input control, so this path cannot navigate underneath a
/// person using the shared browser.
pub async fn open_browser_url(company: &str, value: &str) -> Result<()> {
    validate_company_name(company)?;
    if value.len() > 16 * 1024 {
        bail!("browser URL exceeds 16 KiB");
    }
    let parsed = url::Url::parse(value).context("parse browser URL")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("company browser links must use HTTP or HTTPS");
    }

    // Cold browser startup has its own readiness window; the tab-opening request
    // must not time out halfway through it and leave an ambiguous duplicate on retry.
    wake_company_browser(company).await;
    let container = container_name(company);
    // A leading `=` tells curl to encode the entire value as an unnamed query
    // field. Without it, the first `=` inside an OAuth-style destination
    // query is misread as curl's own name/value separator.
    let destination = format!("={}", parsed.as_str());
    let opened = docker_diagnostic(&[
        "exec",
        &container,
        "curl",
        "--fail-with-body",
        "--silent",
        "--show-error",
        "--max-time",
        "5",
        "--request",
        "PUT",
        "--get",
        "--data-urlencode",
        &destination,
        "http://127.0.0.1:9223/json/new",
    ])
    .await?;
    if !opened.status.success() {
        let body = String::from_utf8_lossy(&opened.stdout);
        if body.contains("owner_controls") {
            bail!("company browser is owner-controlled");
        }
        bail!("company browser could not open the link");
    }
    let target: serde_json::Value =
        serde_json::from_slice(&opened.stdout).context("decode opened browser tab")?;
    let target_id = target["id"]
        .as_str()
        .filter(|id| {
            !id.is_empty()
                && id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
        .context("opened browser tab has no valid target id")?;
    let activate_url = format!("http://127.0.0.1:9223/json/activate/{target_id}");
    let activated = docker_diagnostic(&[
        "exec",
        &container,
        "curl",
        "--fail-with-body",
        "--silent",
        "--show-error",
        "--max-time",
        "5",
        &activate_url,
    ])
    .await?;
    if !activated.status.success() {
        let body = String::from_utf8_lossy(&activated.stdout);
        if body.contains("owner_controls") {
            bail!("company browser is owner-controlled");
        }
        bail!("company browser opened the link but could not focus its tab");
    }
    Ok(())
}

/// Enumerate live X11 application windows through the private company broker.
pub async fn desktop_windows(company: &str) -> Result<serde_json::Value> {
    let container = container_name(company);
    let output = docker_diagnostic(&[
        "exec",
        &container,
        "curl",
        "--fail-with-body",
        "--silent",
        "--show-error",
        "--max-time",
        "5",
        "http://127.0.0.1:9223/restless/desktop/windows",
    ])
    .await?;
    if !output.status.success() {
        bail!(
            "company desktop window enumeration failed: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    serde_json::from_slice(&output.stdout).context("decode company desktop windows")
}

/// Focus one enumerated X11 window, tied to the live owner browser lease.
pub async fn focus_desktop_window(
    company: &str,
    window_id: &str,
    client_id: &str,
    lease_id: &str,
) -> Result<()> {
    if !window_id.strip_prefix("0x").is_some_and(|digits| {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        bail!("invalid company desktop window id");
    }
    let body = serde_json::json!({ "client_id": client_id, "lease_id": lease_id }).to_string();
    let container = container_name(company);
    let url = format!("http://127.0.0.1:9223/restless/desktop/windows/{window_id}/focus");
    let output = docker_bounded(
        &[
            "exec",
            &container,
            "curl",
            "--fail-with-body",
            "--silent",
            "--show-error",
            "--max-time",
            "5",
            "--request",
            "POST",
            "--header",
            "content-type: application/json",
            "--data-raw",
            &body,
            &url,
        ],
        Duration::from_secs(8),
    )
    .await
    .context("bounded desktop focus request")?;
    if !output.status.success() {
        bail!(
            "company desktop window focus failed: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    Ok(())
}

/// A full-duplex byte stream backed by `docker exec socat`. This is the V0
/// Runtime Bridge for the private desktop transport: mature process tooling,
/// not a published port or a browser-action protocol.
pub struct DesktopStream {
    _child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

/// A reviewed web outcome is an ordinary HTTP project service inside the
/// company computer. Only loopback HTTP services are eligible, and the
/// desktop/browser control ports are deliberately excluded: review is a
/// read-only outcome surface, never a second way around the browser handoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHttpTarget {
    pub port: u16,
    pub path_and_query: String,
}

pub fn runtime_http_target(value: &str) -> Result<RuntimeHttpTarget> {
    if value.trim() != value || value.chars().any(char::is_whitespace) {
        bail!("review target must be a bare URL without surrounding notes");
    }
    let url = url::Url::parse(value).context("parse runtime review URL")?;
    if url.scheme() != "http"
        || !matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        bail!("review target must be an uncredentialed loopback http URL");
    }
    let port = url.port_or_known_default().unwrap_or(80);
    if matches!(port, 5901 | 6080 | 9222 | 9223) {
        bail!("review target names a reserved browser/desktop port");
    }
    let mut path_and_query = url.path().to_string();
    if path_and_query.is_empty() {
        path_and_query.push('/');
    }
    if let Some(query) = url.query() {
        path_and_query.push('?');
        path_and_query.push_str(query);
    }
    Ok(RuntimeHttpTarget {
        port,
        path_and_query,
    })
}

/// Issue one GET/HEAD over a private `docker exec socat` stream. The project
/// service remains unpublished; the owner gateway is the only host-side
/// transport and decides which headers and methods may cross it.
pub async fn runtime_http_request(
    company: &str,
    port: u16,
    method: Method,
    path_and_query: &str,
    headers: &HeaderMap,
) -> Result<Response<Incoming>> {
    if !matches!(method, Method::GET | Method::HEAD) {
        bail!("runtime review transport is read-only");
    }
    let uri: Uri = path_and_query
        .parse()
        .context("parse runtime request path")?;
    if uri.scheme().is_some() || uri.authority().is_some() {
        bail!("runtime request must use an origin-relative path");
    }
    let stream = private_tcp_stream(company, port).await?;
    let (mut sender, connection) = http1::handshake(TokioIo::new(stream))
        .await
        .context("open runtime HTTP connection")?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            tracing::debug!("runtime review HTTP connection ended: {error}");
        }
    });

    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("host", format!("127.0.0.1:{port}"))
        .header("connection", "close");
    for name in [
        hyper::header::ACCEPT,
        hyper::header::ACCEPT_LANGUAGE,
        hyper::header::IF_MODIFIED_SINCE,
        hyper::header::IF_NONE_MATCH,
        hyper::header::RANGE,
    ] {
        if let Some(value) = headers.get(&name) {
            request = request.header(name, value);
        }
    }
    sender
        .send_request(request.body(Empty::<Bytes>::new())?)
        .await
        .context("request runtime web outcome")
}

pub async fn probe_runtime_http(company: &str, value: &str) -> Result<()> {
    let target = runtime_http_target(value)?;
    let response = runtime_http_request(
        company,
        target.port,
        Method::HEAD,
        &target.path_and_query,
        &HeaderMap::new(),
    )
    .await?;
    if response.status().is_success() || response.status().is_redirection() {
        return Ok(());
    }

    // Some ordinary preview servers deliberately omit HEAD. A one-byte range
    // GET distinguishes that limitation from an unavailable preview without
    // turning this read-only availability probe into a content fetch.
    if matches!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_IMPLEMENTED
    ) {
        let mut headers = HeaderMap::new();
        headers.insert(
            hyper::header::RANGE,
            hyper::header::HeaderValue::from_static("bytes=0-0"),
        );
        let fallback = tokio::time::timeout(
            Duration::from_secs(5),
            runtime_http_request(
                company,
                target.port,
                Method::GET,
                &target.path_and_query,
                &headers,
            ),
        )
        .await
        .context("runtime review GET fallback timed out")??;
        if fallback.status().is_success() || fallback.status().is_redirection() {
            return Ok(());
        }
        bail!(
            "runtime review target rejected HEAD ({}) and GET fallback returned {}",
            response.status(),
            fallback.status()
        )
    }

    bail!("runtime review target returned {}", response.status())
}

const MAX_REVIEW_TEXT_BYTES: usize = 256 * 1024;

/// One exact path beneath `/company`, spelled with ordinary names only: no `..`, `.`, re-rooting
/// or control characters, so nothing in the spelling itself can leave the company home.
fn company_file_path(value: &str) -> Result<&Path> {
    let path = Path::new(value);
    let mut components = path.components();
    if value.len() > 4096
        || value.chars().any(char::is_control)
        || components.next() != Some(Component::RootDir)
        || components.next() != Some(Component::Normal("company".as_ref()))
        || components.clone().next().is_none()
        || components.any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("the file must be beneath /company");
    }
    Ok(path)
}

/// Resolve the path in `$1` the way every owner read does: through each symlink to an existing
/// regular file that is still inside the company home. A link Work left pointing elsewhere on the
/// computer is refused, not followed. Leaves the canonical path in `$p`.
const CANONICAL_COMPANY_FILE: &str = r#"root=$(realpath -e /company) || exit 66
p=$(realpath -e -- "$1") || exit 66
case "$p" in "$root"/*) ;; *) echo "the file links outside the company home" >&2; exit 67;; esac
[ -f "$p" ] || { echo "the path is not a regular file" >&2; exit 68; }
"#;

/// Read at most `limit` bytes of one canonical company file.
async fn read_company_file(company: &str, path: &Path, limit: u64) -> Result<std::process::Output> {
    let script = format!("{CANONICAL_COMPANY_FILE}exec head -c \"$2\" -- \"$p\"");
    docker_observe(&[
        "exec",
        "-u",
        "company",
        &container_name(company),
        "sh",
        "-c",
        &script,
        "sh",
        path.to_str().context("the file path must be UTF-8")?,
        &limit.to_string(),
    ])
    .await
}

/// How the cockpit reads a small text file in place: Markdown formatted, a table as a table,
/// anything else as plain text. Other files open on a review origin instead.
pub fn runtime_text_format(value: &str) -> Option<&'static str> {
    let extension = Path::new(value)
        .extension()
        .and_then(|value| value.to_str())?
        .to_ascii_lowercase();
    Some(match extension.as_str() {
        "md" | "markdown" => "markdown",
        "csv" => "csv",
        "tsv" => "tsv",
        "json" => "json",
        "txt" | "log" => "text",
        _ => return None,
    })
}

/// Read one small text file beneath `/company` for the owner to read in place. Bounded,
/// canonical and UTF-8; a file too large for this returns an error and opens on a review origin.
pub async fn read_runtime_text_file(company: &str, value: &str) -> Result<String> {
    validate_company_name(company)?;
    let path = company_file_path(value)?;
    if runtime_text_format(value).is_none() {
        bail!("the file is not one the cockpit reads as text");
    }
    let output = read_company_file(company, path, MAX_REVIEW_TEXT_BYTES as u64 + 1).await?;
    if !output.status.success() {
        bail!(
            "the file is unavailable: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    if output.stdout.len() > MAX_REVIEW_TEXT_BYTES {
        bail!("the file is larger than {MAX_REVIEW_TEXT_BYTES} bytes");
    }
    String::from_utf8(output.stdout).context("the file is not UTF-8 text")
}

fn runtime_review_text_path(value: &str) -> Result<&Path> {
    let Ok(path) = company_file_path(value) else {
        bail!("text ReviewTarget must be a file beneath /company");
    };
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if !matches!(extension.as_deref(), Some("md" | "markdown" | "txt")) {
        bail!("text ReviewTarget must be Markdown or plain text");
    }
    Ok(path)
}

pub fn is_runtime_review_text_target(value: &str) -> bool {
    runtime_review_text_path(value).is_ok()
}

/// One ordinary produced file the owner cockpit can actually display.
///
/// A rendered page, document, image, or recording sitting in the company
/// Runtime is the native outcome for a great deal of real work. Before S19-T5
/// only a running loopback service or a Markdown/plain-text file could be
/// reviewed, so a finished `index.html` — the single most obviously viewable
/// artifact a company produces — reached the owner as "this outcome does not
/// have a directly reviewable website" while sitting complete on disk.
///
/// This is a bounded read-only view of exact existing Runtime paths. It is not
/// a file-serving API, an export, or a custody lifecycle: every read is scoped
/// to one issued review ticket, pinned to one Runtime generation, and confined
/// to the directory of the exact ReviewTarget the accountable actor chose.
pub const MAX_REVIEW_FILE_BYTES: u64 = 32 * 1024 * 1024;

/// A format the owner can render or download during review, mapped to the
/// exact type to send. Unsupported extensions are refused because presenting
/// them would leave a blank frame while claiming to show the outcome.
pub fn review_file_media_type(path: &Path) -> Option<&'static str> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())?
        .to_ascii_lowercase();
    Some(match extension.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "txt" | "md" | "markdown" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "wav" => "audio/wav",
        "oga" | "ogg" => "audio/ogg",
        "flac" => "audio/flac",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => return None,
    })
}

/// Office formats are valid read-only review targets, but browsers do not
/// render them. Serve them as downloads instead of presenting a blank frame.
/// A format with no listed type (an archive, a recording in another container,
/// a data file) is still the owner's file: it downloads, never renders.
pub fn is_runtime_review_download(path: &Path) -> bool {
    review_file_media_type(path).is_none()
        || path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx"
                )
            })
}

/// The exact file a ReviewTarget names, beneath `/company`. It must name a file
/// (with an extension, so a repository or output directory is not mistaken for
/// one); a format the cockpit cannot present is served only as a download.
fn runtime_review_file_path(value: &str) -> Result<&Path> {
    let Ok(path) = company_file_path(value) else {
        bail!("file ReviewTarget must be a file beneath /company");
    };
    if path.extension().is_none() {
        bail!("file ReviewTarget must name a file, not a directory");
    }
    Ok(path)
}

/// Whether this ReviewTarget is a Runtime file the cockpit can display. Text
/// targets keep their own richer path: the cockpit renders their Markdown
/// rather than framing them.
pub fn is_runtime_review_file_target(value: &str) -> bool {
    runtime_review_file_path(value).is_ok() && !is_runtime_review_text_target(value)
}

/// The directory a file ReviewTarget's review is confined to, and the entry
/// path within it. A rendered page's own stylesheet, script and images are part
/// of the outcome; nothing above its directory is.
pub fn runtime_review_file_root(value: &str) -> Result<(PathBuf, String)> {
    let path = runtime_review_file_path(value)?;
    let root = path
        .parent()
        .filter(|parent| parent.components().count() >= 2)
        .context("file ReviewTarget must sit inside a directory beneath /company")?;
    let entry = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("file ReviewTarget must name a file")?;
    Ok((root.to_path_buf(), entry.to_string()))
}

/// Observe that the exact chosen file is present and within the size the owner
/// gateway will carry. This is the file equivalent of the live HTTP probe: the
/// cockpit must never claim an outcome is ready without observing it.
pub async fn probe_runtime_review_file(company: &str, value: &str) -> Result<()> {
    let size = runtime_review_file_size(company, value).await?;
    // The ticket's own file streams, a page as much as a recording or a long PDF;
    // only the assets a page loads are read whole.
    if size > MAX_STREAMED_REVIEW_FILE_BYTES {
        return Err(ReviewFileTooLarge.into());
    }
    Ok(())
}

/// The largest single file the cockpit streams from the company computer.
pub const MAX_STREAMED_REVIEW_FILE_BYTES: u64 = 16 * 1024 * 1024 * 1024;

/// A file past what the cockpit opens or downloads. Its own refusal, so the owner is told the
/// size is the reason rather than that the file may be gone.
#[derive(Debug)]
pub struct ReviewFileTooLarge;
impl std::fmt::Display for ReviewFileTooLarge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the file is larger than the 16 GiB the cockpit can open or download")
    }
}
impl std::error::Error for ReviewFileTooLarge {}

/// The size of one company file, observed through its canonical path.
async fn runtime_review_file_size(company: &str, value: &str) -> Result<u64> {
    validate_company_name(company)?;
    let path = runtime_review_file_path(value)?;
    let container = container_name(company);
    let script = format!("{CANONICAL_COMPANY_FILE}exec stat -c %s -- \"$p\"");
    let output = docker_observe(&[
        "exec",
        "-u",
        "company",
        &container,
        "sh",
        "-c",
        &script,
        "sh",
        path.to_str().context("ReviewTarget path must be UTF-8")?,
    ])
    .await?;
    if !output.status.success() {
        bail!(
            "file ReviewTarget is unavailable: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let size: u64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .context("could not observe the ReviewTarget's size")?;
    Ok(size)
}

/// One company file streamed rather than buffered: its type, its size, and its bytes as they
/// come off the company computer. Resolved through its symlinks to a regular file inside the
/// company home, exactly as a buffered read is. The stream owns the `docker exec`; dropping it
/// (the owner closed the tab) kills that process. A file that changes while it streams may end
/// short; accepted, since the owner can open it again.
///
/// A browser's single `Range` request (Safari and iPhone will not play a recording without one)
/// reads only that slice; any other range asks for the whole file.
pub async fn stream_runtime_review_file(
    company: &str,
    path: &Path,
    range: Option<&str>,
) -> Result<StreamedReviewFile> {
    use futures_util::StreamExt;
    use tokio::io::AsyncReadExt;
    let value = path.to_str().context("review file path must be UTF-8")?;
    let path = company_file_path(value)?;
    let media_type = review_file_media_type(path).unwrap_or("application/octet-stream");
    let size = runtime_review_file_size(company, value).await?;
    if size > MAX_STREAMED_REVIEW_FILE_BYTES {
        return Err(ReviewFileTooLarge.into());
    }
    let range = range.and_then(|header| review_byte_range(header, size));
    let (start, length) = range.map_or((0, size), |(first, last)| (first, last - first + 1));
    let script = format!("{CANONICAL_COMPANY_FILE}tail -c +\"$2\" -- \"$p\" | head -c \"$3\"");
    let (offset, length_arg) = ((start + 1).to_string(), length.to_string());
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            &container_name(company),
            "sh",
            "-c",
            &script,
            "sh",
            value,
            &offset,
            &length_arg,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("spawn docker")?;
    let stdout = child.stdout.take().context("docker exec has no output")?;
    let body = tokio_util::io::ReaderStream::new(stdout.take(length))
        .map(move |chunk| {
            let _owned = &child;
            chunk
        })
        .boxed();
    Ok(StreamedReviewFile {
        media_type,
        size,
        range,
        body,
    })
}

/// One streamed company file: its type, its whole size, the inclusive byte range served when a
/// single range was asked for, and the bytes of that range (or of the whole file).
pub struct StreamedReviewFile {
    pub media_type: &'static str,
    pub size: u64,
    pub range: Option<(u64, u64)>,
    pub body: futures_util::stream::BoxStream<'static, std::io::Result<Bytes>>,
}

/// The inclusive byte range one `Range` header asks of a file of `size` bytes: a single
/// `bytes=first-last`, `bytes=first-` or `bytes=-suffix`. Several ranges, another unit or a range
/// past the end yield None, and the whole file is served instead, as HTTP allows.
pub fn review_byte_range(header: &str, size: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?.trim();
    if size == 0 || spec.contains(',') {
        return None;
    }
    let (first, last) = spec.split_once('-')?;
    let (first, last) = (first.trim(), last.trim());
    let (first, last) = if first.is_empty() {
        let suffix: u64 = last.parse().ok().filter(|suffix| *suffix > 0)?;
        (size.saturating_sub(suffix), size - 1)
    } else {
        let first: u64 = first.parse().ok()?;
        let last = if last.is_empty() {
            size - 1
        } else {
            last.parse::<u64>().ok()?.min(size - 1)
        };
        (first, last)
    };
    (first <= last && first < size).then_some((first, last))
}

/// Resolve one requested path against a file review's confined root.
///
/// The request comes from a page the company itself authored, so this is the
/// exact place a traversal would be attempted. Only ordinary named components
/// survive: no `..`, no absolute re-root, and the resolved path must still be a
/// displayable format. Only the exact entry the ticket names may be a format
/// with no listed type (it downloads). The read itself refuses a symlink that
/// leads outside the company home.
pub fn resolve_review_file(root: &Path, entry: &str, request_path: &str) -> Result<PathBuf> {
    let requested = request_path.split(['?', '#']).next().unwrap_or("");
    let decoded = percent_decode(requested);
    let relative = decoded.trim_start_matches('/');
    let relative = if relative.is_empty() { entry } else { relative };
    let mut resolved = root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            _ => bail!("review file path must not leave the prepared outcome"),
        }
    }
    if !resolved.starts_with(root)
        || (review_file_media_type(&resolved).is_none() && resolved != root.join(entry))
    {
        bail!("review file path must not leave the prepared outcome");
    }
    Ok(resolved)
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Read one file from a confined review root. Bounded by the same limit the
/// probe observed, so an outcome that grew past it fails honestly rather than
/// streaming without end.
pub async fn read_runtime_review_file(
    company: &str,
    path: &Path,
) -> Result<(&'static str, Vec<u8>)> {
    validate_company_name(company)?;
    let path = company_file_path(path.to_str().context("review file path must be UTF-8")?)?;
    let media_type = review_file_media_type(path).unwrap_or("application/octet-stream");
    let output = read_company_file(company, path, MAX_REVIEW_FILE_BYTES + 1).await?;
    if !output.status.success() {
        bail!(
            "review file is unavailable: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    if output.stdout.len() as u64 > MAX_REVIEW_FILE_BYTES {
        bail!("review file exceeds the {MAX_REVIEW_FILE_BYTES}-byte review limit");
    }
    Ok((media_type, output.stdout))
}

/// Materialise one observed text ReviewTarget for the owner projection. The
/// file remains Runtime truth: this is a bounded, read-only view of the exact
/// existing path, not an import, export, or general file-serving interface.
pub async fn read_runtime_review_text(company: &str, value: &str) -> Result<String> {
    validate_company_name(company)?;
    let path = runtime_review_text_path(value)?;
    let output = read_company_file(company, path, MAX_REVIEW_TEXT_BYTES as u64 + 1).await?;
    if !output.status.success() {
        bail!(
            "text ReviewTarget is unavailable: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    if output.stdout.len() > MAX_REVIEW_TEXT_BYTES {
        bail!("text ReviewTarget exceeds {MAX_REVIEW_TEXT_BYTES} bytes");
    }
    String::from_utf8(output.stdout).context("text ReviewTarget is not UTF-8")
}

impl AsyncRead for DesktopStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdout).poll_read(cx, buffer)
    }
}

impl AsyncWrite for DesktopStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.stdin).poll_write(cx, buffer)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdin).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdin).poll_shutdown(cx)
    }
}

pub async fn desktop_stream(company: &str) -> Result<DesktopStream> {
    wake_company_browser(company).await;
    private_tcp_stream(company, 6080).await
}

/// The same private bridge to Chrome's DevTools port, for host-side automation only.
pub async fn devtools_stream(company: &str) -> Result<DesktopStream> {
    wake_company_browser(company).await;
    private_tcp_stream(company, 9222).await
}

/// Start the company browser if it went to sleep after idling (S104 F6), and wait for its DevTools
/// port. Core and apps that use Chrome's port directly call this first; the broker wakes it for
/// agents. Best effort: a browser that will not start is reported by whatever uses it next.
pub async fn wake_company_browser(company: &str) {
    // Let the broker record real activity too. Direct supervisor startup alone
    // leaves lastUse stale and can immediately sleep the just-opened browser.
    // Owner control refuses broker discovery; its live lease protects the fallback.
    let wake = format!(
        "[ -f /company/run/browser-asleep ] || exit 0; \
        curl -fsS -m 40 http://127.0.0.1:9223/json/version >/dev/null 2>&1 && exit 0; \
        supervisorctl -c {} start chromium >/dev/null 2>&1; \
        i=0; while [ $i -lt 60 ]; do curl -fsS -m 1 http://127.0.0.1:9222/json/version >/dev/null 2>&1 && break; i=$((i+1)); sleep 0.5; done; \
        rm -f /company/run/browser-asleep",
        desktop_supervisor_config()
    );
    let _ = docker_bounded(
        &[
            "exec",
            "-u",
            "company",
            &container_name(company),
            "sh",
            "-c",
            &wake,
        ],
        Duration::from_secs(45),
    )
    .await;
}

async fn private_tcp_stream(company: &str, port: u16) -> Result<DesktopStream> {
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            &container_name(company),
            "socat",
            "STDIO",
            &format!("TCP:127.0.0.1:{port}"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("open private desktop bridge")?;
    let stdin = child.stdin.take().context("desktop bridge stdin")?;
    let stdout = child.stdout.take().context("desktop bridge stdout")?;
    Ok(DesktopStream {
        _child: child,
        stdin,
        stdout,
    })
}

pub async fn read_browser_control(company: &str) -> Result<Option<serde_json::Value>> {
    let name = container_name(company);
    let output = docker_diagnostic(&[
        "exec",
        &name,
        "sh",
        "-c",
        "test -f /company/run/browser-control.json && cat /company/run/browser-control.json",
    ])
    .await?;
    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }
    let state = serde_json::from_slice(&output.stdout).context("parse browser controller state")?;
    Ok(Some(normalize_expired_browser_control(state)))
}

/// Host-verified identity for one agent's short-lived shared-browser attach.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAgentSession {
    pub ticket: String,
    pub company: String,
    pub actor: String,
    pub session: String,
    pub work_id: Option<Uuid>,
    pub attempt_id: Option<Uuid>,
    pub websocket_url: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn read_browser_agent_session(company: &str) -> Result<Option<BrowserAgentSession>> {
    validate_company_name(company)?;
    let output = docker_observe(&[
        "exec", &container_name(company), "sh", "-c",
        "test -f /company/run/browser-agent-session.json && cat /company/run/browser-agent-session.json",
    ]).await?;
    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }
    let session: BrowserAgentSession = serde_json::from_slice(&output.stdout)
        .context("parse authenticated browser registration")?;
    if session.expires_at <= Utc::now() {
        return Ok(None);
    }
    Ok(Some(session))
}

/// `None` means another actor currently owns the shared browser. A caller may
/// wait and retry after revalidating its signed Work Attempt.
pub async fn try_register_browser_agent_session(
    company: &str,
    grant: &crate::capability::CoordinationGrant,
) -> Result<Option<String>> {
    validate_company_name(company)?;
    if grant.company != company {
        bail!("browser registration company does not match the signed ActorSession");
    }
    if grant.work_id.is_some() != grant.attempt_id.is_some() {
        bail!("browser attachment needs both Work and Attempt coordinates, or neither");
    }
    let _control_guard = browser_control_guard(company).await;
    let control = read_browser_control(company).await?.unwrap_or_default();
    // read_browser_control normalizes expired owner leases before returning.
    if control["controller"] == "owner" {
        bail!("the owner currently controls the company browser");
    }
    if let Some(current) = read_browser_agent_session(company).await? {
        if current.session == grant.session
            && current.actor == grant.actor
            && current.work_id == grant.work_id
            && current.attempt_id == grant.attempt_id
        {
            return Ok(Some(current.websocket_url));
        }
        return Ok(None);
    }
    // Only Core reads Chrome's private endpoint. The opaque ticket is minted
    // after Core verified the signed capability and is stored atomically for
    // the broker; the issuer key never enters the company container.
    wake_company_browser(company).await;
    let output = docker_observe(&[
        "exec",
        &container_name(company),
        "curl",
        "--fail",
        "--silent",
        "--show-error",
        "--max-time",
        "5",
        "http://127.0.0.1:9222/json/version",
    ])
    .await?;
    if !output.status.success() {
        bail!("could not inspect the company browser endpoint");
    }
    let discovery: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("decode the company browser endpoint")?;
    let path = discovery["webSocketDebuggerUrl"]
        .as_str()
        .and_then(|value| url::Url::parse(value).ok())
        .map(|value| value.path().to_string())
        .filter(|value| value.starts_with("/devtools/browser/"))
        .context("company browser did not return a browser-level CDP endpoint")?;
    let ticket = Uuid::new_v4().simple().to_string();
    let websocket_url = format!("ws://127.0.0.1:9223/session/{ticket}{path}");
    let registration = BrowserAgentSession {
        ticket,
        company: company.to_string(),
        actor: grant.actor.clone(),
        session: grant.session.clone(),
        work_id: grant.work_id,
        attempt_id: grant.attempt_id,
        websocket_url: websocket_url.clone(),
        expires_at: Utc::now() + chrono::Duration::minutes(45),
    };
    write_browser_agent_session(company, &registration).await?;
    Ok(Some(websocket_url))
}

pub async fn release_browser_agent_session(
    company: &str,
    grant: &crate::capability::CoordinationGrant,
) -> Result<()> {
    let Some(current) = read_browser_agent_session(company).await? else {
        return Ok(());
    };
    if current.company != grant.company
        || current.actor != grant.actor
        || current.session != grant.session
        || current.work_id != grant.work_id
        || current.attempt_id != grant.attempt_id
    {
        bail!("browser registration does not belong to this signed ActorSession");
    }
    clear_browser_agent_session(company, &current.ticket).await
}

pub async fn clear_browser_agent_session(company: &str, ticket: &str) -> Result<()> {
    validate_company_name(company)?;
    let _control_guard = browser_control_guard(company).await;
    let Some(current) = read_browser_agent_session(company).await? else {
        return Ok(());
    };
    if current.ticket != ticket {
        bail!("browser registration changed before it could be cleared");
    }
    let output = tokio::process::Command::new("docker")
        .args([
            "exec",
            &container_name(company),
            "rm",
            "-f",
            "/company/run/browser-agent-session.json",
        ])
        .output()
        .await
        .context("release company browser registration")?;
    if !output.status.success() {
        bail!("could not release company browser registration");
    }
    Ok(())
}

async fn write_browser_agent_session(company: &str, session: &BrowserAgentSession) -> Result<()> {
    let mut child = tokio::process::Command::new("docker").args([
        "exec", "-i", "-u", "company", &container_name(company), "sh", "-c",
        "umask 077; cat > /company/run/browser-agent-session.json.tmp && mv /company/run/browser-agent-session.json.tmp /company/run/browser-agent-session.json",
    ]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn()
        .context("write authenticated company browser registration")?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin.write_all(&serde_json::to_vec(session)?).await?;
    drop(stdin);
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        bail!("could not write authenticated company browser registration");
    }
    Ok(())
}

/// An owner lease is bounded even if the SPA vanishes without hand-back.
/// The Runtime file is reconstructable coordination, not durable truth, so a
/// reader projects an expired owner back to the unclaimed state. The browser
/// broker independently uses the same
/// expiry to reopen CDP; keeping the health projection stale at `owner` while
/// automation had resumed was precisely the split-brain this lease prevents.
fn normalize_expired_browser_control(mut state: serde_json::Value) -> serde_json::Value {
    if state["controller"] != "owner" {
        return state;
    }
    let Some(expires_at) = state["expires_at"].as_str().map(str::to_string) else {
        return state;
    };
    let Ok(expires) = DateTime::parse_from_rfc3339(&expires_at) else {
        return state;
    };
    if expires.with_timezone(&Utc) > Utc::now() {
        return state;
    }

    state = serde_json::json!({
        "controller": "unclaimed",
        "reason": "owner_lease_expired",
        "expired_at": expires_at,
    });
    state
}

/// Replace the reconstructable lease atomically inside the persistent Runtime.
/// This is deterministic process coordination, not an Authority effect.
pub async fn write_browser_control(company: &str, state: &serde_json::Value) -> Result<()> {
    let name = container_name(company);
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            "-u",
            "company",
            &name,
            "sh",
            "-c",
            "umask 077; cat > /company/run/browser-control.json.tmp && mv /company/run/browser-control.json.tmp /company/run/browser-control.json",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .context("write browser controller state")?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin
        .write_all(serde_json::to_string(state)?.as_bytes())
        .await?;
    drop(stdin);
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        bail!(
            "write browser controller state failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    publish_browser_control(company, Some(state.clone())).await;
    Ok(())
}

/// Ensure the release artifact selected by Fleet is present. This function is
/// intentionally incapable of building from a source checkout: doing that in
/// the plane would make the deployed Runtime differ from the pinned manifest.
/// Pull the plane's pinned company image if it is not here yet. A fresh plane boots before its
/// image is pulled, and a computer cannot be found to lag behind an image the host does not have.
pub async fn fetch_company_image() -> Result<bool> {
    ensure_image_available(&company_image()).await
}

async fn ensure_image_available(image: &str) -> Result<bool> {
    if inspect_value(&["image", "inspect", "-f", "{{.Id}}", image])
        .await?
        .is_some()
    {
        return Ok(false);
    }

    let output = docker(&["pull", image]).await?;
    if !output.status.success() {
        bail!(
            "company image {image} is unavailable and could not be pulled: {}. \
             Fleet must publish and pin it; local development can build it with scripts/restless-dev --reconcile",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(true)
}

async fn container_uses_old_image(company: &str) -> Result<bool> {
    let image = company_image();
    let name = container_name(company);
    let container_id = inspect_value(&["inspect", "-f", "{{.Image}}", &name]).await?;
    let target_id = inspect_value(&["image", "inspect", "-f", "{{.Id}}", &image]).await?;
    Ok(match (container_id, target_id) {
        (Some(container_id), Some(target_id)) => container_id != target_id,
        // If the target disappeared after a successful build, state is
        // unknowable and replacement would be a guess.
        (_, None) => bail!("company image {image} is unavailable after reconciliation"),
        (None, Some(_)) => true,
    })
}

async fn container_uses_company_volume(company: &str) -> Result<bool> {
    let mounted = inspect_value(&[
        "inspect",
        "-f",
        "{{range .Mounts}}{{if eq .Destination \"/company\"}}{{.Name}}{{end}}{{end}}",
        &container_name(company),
    ])
    .await?;
    Ok(mounted.as_deref() == Some(volume_name(company).as_str()))
}

async fn inspect_container_image_and_volume(company: &str) -> Result<(Option<String>, bool)> {
    let output = docker_observe(&[
        "inspect",
        "-f",
        "{{.Image}}|{{range .Mounts}}{{if eq .Destination \"/company\"}}{{.Name}}{{end}}{{end}}",
        &container_name(company),
    ])
    .await?;
    if !output.status.success() {
        return Ok((None, false));
    }
    let output = String::from_utf8_lossy(&output.stdout);
    let Some((image, mounted_volume)) = output.trim_end().split_once('|') else {
        return Ok((None, false));
    };
    let image = (!image.trim().is_empty()).then(|| image.trim().to_string());
    Ok((image, mounted_volume.trim() == volume_name(company)))
}

async fn inspect_value(args: &[&str]) -> Result<Option<String>> {
    let output = docker_observe(args).await?;
    if !output.status.success() {
        return Ok(None);
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!value.is_empty()).then_some(value))
}

/// The local source tree is the V0 image source. An explicit environment
/// override supports a daemon launched outside the repository; otherwise walk
/// upward from its working directory. We do not silently build from an
/// arbitrary directory.
pub fn source_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("RESTLESS_SOURCE_ROOT") {
        let root = PathBuf::from(root);
        validate_source_root(&root)?;
        return Ok(root);
    }
    let mut cursor = std::env::current_dir().context("read daemon working directory")?;
    loop {
        if validate_source_root(&cursor).is_ok() {
            return Ok(cursor);
        }
        if !cursor.pop() {
            break;
        }
    }
    let compiled = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("resolve compiled Restless source root")?;
    validate_source_root(&compiled)?;
    Ok(compiled)
}

fn validate_source_root(root: &Path) -> Result<()> {
    if !root.join("Cargo.toml").is_file() || !root.join("infra/company-image/Dockerfile").is_file()
    {
        bail!(
            "{} is not a Restless source tree (set RESTLESS_SOURCE_ROOT)",
            root.display()
        );
    }
    Ok(())
}

fn digest_source(root: &Path) -> Result<String> {
    let mut files = Vec::new();
    for relative in ["Cargo.toml", "Cargo.lock", "crates", "infra/company-image"] {
        collect_files(&root.join(relative), &mut files)?;
    }
    sort_source_files(root, &mut files);
    let mut digest = Sha256::new();
    for path in files {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        digest.update(relative.to_string_lossy().as_bytes());
        digest.update([0]);
        digest.update(
            std::fs::read(&path).with_context(|| format!("read image input {}", path.display()))?,
        );
        digest.update([0]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn sort_source_files(root: &Path, files: &mut [PathBuf]) {
    files.sort_by(|left, right| {
        let left = left.strip_prefix(root).unwrap_or(left).to_string_lossy();
        let right = right.strip_prefix(root).unwrap_or(right).to_string_lossy();
        left.as_bytes().cmp(right.as_bytes())
    });
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("inspect image input {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("image input may not be a symlink: {}", path.display());
    }
    if metadata.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }
    let mut children: Vec<PathBuf> = std::fs::read_dir(path)
        .with_context(|| format!("read image input directory {}", path.display()))?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|entry| entry.path())
        .collect();
    children.sort();
    for child in children {
        collect_files(&child, files)?;
    }
    Ok(())
}

/// Stop the container. The volume — files, Git history, browser profile —
/// survives (§5, §17 step 2: the persistent company computer). An explicit
/// stop is owner intent, so it also clears any sleep marker: demand must not
/// restart a computer somebody deliberately turned off.
pub async fn down(company: &str) -> Result<String> {
    let _start = company_start_guard(company).await;
    let result = down_locked(company).await;
    clear_sleeping(company);
    result
}

/// Stop a Runtime only if an async activity check still considers it idle.
/// The predicate runs under the same per-company lifecycle lock as `up`, so a
/// concurrent explicit start cannot be stopped after it has completed.
/// `sleep` marks the stop as policy sleep, which any owed demand may undo.
pub async fn down_if_idle<F, Fut>(company: &str, sleep: bool, is_idle: F) -> Result<bool>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<bool>>,
{
    let _start = company_start_guard(company).await;
    if status(company).await? != ContainerStatus::Running || !is_idle().await? {
        return Ok(false);
    }
    down_running_locked(company).await?;
    if sleep {
        mark_sleeping(company)?;
    } else {
        clear_sleeping(company);
    }
    invalidate_cockpit_health(company).await;
    Ok(true)
}

fn sleep_marker(company: &str) -> PathBuf {
    state_root().join("runtime-sleep").join(company)
}

/// Whether the company computer is stopped because the sleep policy put it
/// to sleep, rather than because the owner stopped it or it never started.
/// Only a sleeping computer is woken by demand.
pub fn is_sleeping(company: &str) -> bool {
    sleep_marker(company).is_file()
}

fn mark_sleeping(company: &str) -> Result<()> {
    let marker = sleep_marker(company);
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(&marker, chrono::Utc::now().to_rfc3339())
        .with_context(|| format!("write {}", marker.display()))
}

fn clear_sleeping(company: &str) {
    let _ = std::fs::remove_file(sleep_marker(company));
}

/// When the current sleep began, if the computer is asleep.
pub fn sleeping_since(company: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    std::fs::read_to_string(sleep_marker(company))
        .ok()?
        .trim()
        .parse()
        .ok()
}

async fn down_locked(company: &str) -> Result<String> {
    let result = match status(company).await? {
        ContainerStatus::Running => {
            down_running_locked(company).await?;
            format!("{company}: stopped (volume kept)")
        }
        ContainerStatus::Stopped => format!("{company}: already stopped"),
        ContainerStatus::Absent => format!("{company}: no container"),
    };
    invalidate_cockpit_health(company).await;
    Ok(result)
}

async fn down_running_locked(company: &str) -> Result<()> {
    let name = container_name(company);
    // Chromium's supervisor stop window is 20 seconds. Use a longer
    // container deadline so cookies and profile state reach disk before
    // Docker escalates to SIGKILL. The computer goes first: it lives in the Runtime's network.
    stop_computer(company).await;
    run_ok(&["stop", "--time", "30", &name]).await?;
    Ok(())
}

/// S04-T1. Remove a throwaway company entirely: container, volume, OrgIntel
/// schema **and spend spool**.
///
/// The spend spool is the part that looks optional and is not. The sprint-02
/// comparison harness reset container, volume and schema between its three
/// arms and never the spool, so the arms ran with $2.45 / $10.51 / $12.85 of
/// headroom against a nominal $15 ceiling — three "identical" runs that were
/// not comparable, and nobody could see it. Destroying three of four states is
/// how you get a clean-looking run with a hidden variable in it.
pub async fn destroy(
    root: &Path,
    database_url: &str,
    company: &str,
    org: &restless_orgintel::OrgIntel,
    spend: &crate::spend::SpendLedger,
) -> Result<String> {
    let mut removed = Vec::new();
    clear_sleeping(company);
    crate::local_documents::remove(root, org)
        .await
        .context("remove local Documents service before dropping its cell")?;

    remove_computer(company).await;
    if status(company).await? != ContainerStatus::Absent {
        let name = container_name(company);
        // `rm -f` covers running and stopped in one call; stopping first would
        // leave a window where a crash strands the container.
        run_ok(&["rm", "-f", &name]).await?;
        removed.push("container");
    }

    // Local MCP workers hold the package cache and /company/tools (on the company volume)
    // open; end them before either volume goes.
    crate::connections::forget_company_upstreams(company).await;
    let company_label = format!("label=io.restless.company={company}");
    let namespace_label = format!(
        "label=io.restless.namespace={}",
        std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default()
    );
    let workers = docker_observe(&[
        "ps",
        "-aq",
        "--filter",
        "label=io.restless.mcp-worker=true",
        "--filter",
        &company_label,
        "--filter",
        &namespace_label,
    ])
    .await?;
    let workers = String::from_utf8_lossy(&workers.stdout).to_string();
    for worker in workers.split_whitespace() {
        // A closed session lets an `--rm` worker remove itself, so this may
        // race its own removal; the volume check below waits for both.
        let _ = docker_observe(&["rm", "-f", worker]).await;
        removed.push("local MCP worker");
    }
    let volume = volume_name(company);
    if docker_observe(&["volume", "inspect", &volume])
        .await?
        .status
        .success()
    {
        remove_volume_when_free(&volume, "company volume").await?;
        removed.push("volume");
    }
    let attachments = attachments_volume_name(company);
    if docker_observe(&["volume", "inspect", &attachments])
        .await?
        .status
        .success()
    {
        remove_volume_when_free(&attachments, "kept files").await?;
        removed.push("kept files");
    }
    let cache = mcp_cache_volume_name(company);
    if docker_observe(&["volume", "inspect", &cache])
        .await?
        .status
        .success()
    {
        remove_volume_when_free(&cache, "local MCP cache").await?;
        removed.push("local MCP cache");
    }
    let namespace = std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default();
    let data = docker_observe(&[
        "volume",
        "ls",
        "-q",
        "--filter",
        &format!("label=io.restless.company={company}"),
        "--filter",
        &format!("label=io.restless.namespace={namespace}"),
        "--filter",
        "label=io.restless.mcp-data",
    ])
    .await?;
    for volume in String::from_utf8_lossy(&data.stdout).split_whitespace() {
        remove_volume_when_free(volume, "tool data").await?;
        removed.push("local tool data");
    }

    // A cell is a database and role, not only a schema. Closing the shared
    // pool first makes `down --destroy` a real clean-room reset rather than a
    // success message over a hidden database that survives into the next run.
    org.close().await;
    crate::cell::destroy_database(root, database_url, company)
        .await
        .context("drop OrgIntel cell database, role and credential")?;
    removed.push("cell");

    // The spool is ONE shared file, not one per company (`spend.jsonl`), so
    // this cannot be a file deletion — an earlier version of this function
    // removed `spend/<company>.jsonl`, a path that has never existed, and so
    // silently left every destroyed company's spend accounted. That is the
    // sprint-02 defect verbatim.
    spend
        .forget(company)
        .context("clearing the destroyed company's spend")?;
    removed.push("spend");

    remove_company_logos(root, company, None).context("remove the company logo")?;

    // The config last: while it exists, the company is nameable and the earlier
    // steps are re-runnable if one of them failed.
    let config = root.join("companies").join(format!("{company}.toml"));
    if config.exists() {
        std::fs::remove_file(&config)
            .with_context(|| format!("remove config {}", config.display()))?;
        removed.push("config");
    }

    Ok(format!("{company}: destroyed ({})", removed.join(", ")))
}

/// Is this a throwaway company? The name is the marker, so the property is
/// visible in every log line, schema name and container name without anyone
/// looking up config (`S03-T7`: underscores, because a company name becomes a
/// Postgres schema name and `aris-test` is rejected at creation).
pub fn is_test_company(company: &str) -> bool {
    company.ends_with("_test")
}

/// S04-T1. Clone a live company's mission and configuration under a new name,
/// with every live credential, approval and external-effect identity stripped.
///
/// The guarantee is structural, not a rule someone remembers: a `_test`
/// company receives no live secret bindings, so a scenario must install fake
/// CLIs. We contaminated a live company's beliefs with
/// a synthetic webhook once already — "the strongest single demand signal so
/// far" turned out to be us — and that happened because the live company was
/// the only one available to try things on.
pub fn clone_config(root: &Path, from: &str, to: &str) -> Result<CompanyConfig> {
    if !is_test_company(to) {
        bail!(
            "refusing to clone into {to:?}: a cloned company must be a throwaway, \
             and a throwaway's name must end in `_test` so it is visible everywhere"
        );
    }
    let source =
        CompanyConfig::load(root, from).with_context(|| format!("load source company {from}"))?;
    let config = CompanyConfig {
        agent_intelligence: Default::default(),
        native_harnesses: Default::default(),
        display_name: None,
        logo: None,
        name: to.to_string(),
        internal_network: false,
        // A scenario gets no live secret bindings.
        credentials: std::collections::BTreeMap::new(),
        model_failover: Vec::new(),
        // Standing approvals are the owner's blessing of a *live* company's
        // counterparties. They do not travel to a throwaway.
        approved_parties: Vec::new(),
        everyday_model: None,
        decision_model: None,
        ..source
    };
    CompanyConfig::save(root, &config)?;
    Ok(config)
}

async fn seed_mission(config: &CompanyConfig) -> Result<()> {
    let name = container_name(&config.name);
    let mut child = tokio::process::Command::new("docker")
        .args([
            // As the company, which owns the file: /company is sticky and group-writable, so the
            // kernel's protected_regular refuses even root opening a file it does not own there.
            "exec",
            "-i",
            "-u",
            "2000:2000",
            &name,
            "sh",
            "-c",
            "cat > /company/mission.md",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .context("spawn docker exec for mission seed")?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin.write_all(config.mission.as_bytes()).await?;
    drop(stdin);
    let out = child.wait_with_output().await?;
    if !out.status.success() {
        bail!(
            "mission seed failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

/// Refresh the Runtime's read-only projection of the owner mandate without
/// restarting the persistent company computer. A stopped or absent Runtime
/// will receive the canonical value through the ordinary `up` seed path.
pub async fn sync_mission_projection(config: &CompanyConfig) -> Result<&'static str> {
    match status(&config.name).await? {
        ContainerStatus::Running => {
            seed_mission(config).await?;
            Ok("updated")
        }
        ContainerStatus::Stopped | ContainerStatus::Absent => Ok("deferred"),
    }
}

/// Put owner-supplied bytes in a daemon-owned, company-readable directory on
/// the persistent company computer. PostgreSQL is the authority for filename,
/// media type, size, and digest; no mutable filesystem sidecar is trusted.
/// The parent lives under root-owned `/var/lib`, so the `company` account
/// cannot replace, rename, or delete committed content after Message ingress.
pub async fn store_owner_attachment(
    company: &str,
    attachment_id: Uuid,
    bytes: &[u8],
) -> Result<String> {
    validate_company_name(company)?;
    if status(company).await? != ContainerStatus::Running {
        bail!("the company computer is not running; attachments need its persistent filesystem");
    }
    let name = container_name(company);
    let root = "/var/lib/restless-owner-attachments";
    let directory = format!("{root}/{attachment_id}");
    let content_path = format!("{directory}/content");
    write_sealed_container_file(&name, root, &directory, &content_path, bytes).await?;
    Ok(content_path)
}

/// One safe file name for an owner attachment's readable alias: the original
/// name reduced to `[A-Za-z0-9_-]` plus one `.` before its extension, which is
/// kept (lowercased) so tools that choose a reader by extension work. `None`
/// when nothing usable remains; never the canonical `content` file's name.
pub fn owner_attachment_alias(name: &str) -> Option<String> {
    let name = name.rsplit(['/', '\\']).next().unwrap_or_default().trim();
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem, Some(extension)),
        _ => (name, None),
    };
    let clean = |value: &str, limit: usize| {
        let mut cleaned = String::new();
        for character in value.chars() {
            let next = if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '-'
            };
            if next == '-' && (cleaned.is_empty() || cleaned.ends_with('-')) {
                continue;
            }
            cleaned.push(next);
            if cleaned.len() >= limit {
                break;
            }
        }
        cleaned.trim_end_matches('-').to_string()
    };
    let stem = clean(stem, 80);
    let extension = extension
        .map(|extension| clean(extension, 8).to_ascii_lowercase())
        .filter(|extension| !extension.is_empty());
    let alias = match (stem.is_empty(), extension) {
        (true, None) => return None,
        (true, Some(extension)) => format!("file.{extension}"),
        (false, None) => stem,
        (false, Some(extension)) => format!("{stem}.{extension}"),
    };
    if alias == "content" || alias.starts_with("content.") {
        return Some(format!("file-{alias}"));
    }
    Some(alias)
}

/// Give a stored attachment a second, readable name beside its canonical
/// `content` file: a hard link in the same sealed directory, so it moves and
/// disappears with the attachment. Returns the alias path, or `None` when the
/// name has no safe alias. The canonical path stays the authority.
pub async fn link_owner_attachment_name(
    company: &str,
    attachment_id: Uuid,
    name: &str,
) -> Result<Option<String>> {
    validate_company_name(company)?;
    let Some(alias) = owner_attachment_alias(name) else {
        return Ok(None);
    };
    let container = container_name(company);
    let directory = format!("/var/lib/restless-owner-attachments/{attachment_id}");
    let alias_path = format!("{directory}/{alias}");
    let output = docker(&[
        "exec",
        &container,
        "sh",
        "-c",
        "set -eu; test ! -L \"$1\"; test -f \"$1/content\"; ln -f \"$1/content\" \"$2\"",
        "sh",
        &directory,
        &alias_path,
    ])
    .await?;
    if !output.status.success() {
        bail!(
            "link owner attachment name failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(Some(alias_path))
}

async fn write_sealed_container_file(
    container: &str,
    root: &str,
    directory: &str,
    path: &str,
    bytes: &[u8],
) -> Result<()> {
    let command = concat!(
        "set -eu; ",
        "test ! -L \"$1\"; test ! -L \"$2\"; ",
        "mkdir -p \"$1\" \"$2\"; ",
        "chown root:company \"$1\" \"$2\"; chmod 0550 \"$1\" \"$2\"; ",
        "umask 077; cat > \"$3.tmp\"; ",
        "chown root:company \"$3.tmp\"; chmod 0440 \"$3.tmp\"; ",
        "mv -f \"$3.tmp\" \"$3\""
    );
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec", "-i", container, "sh", "-c", command, "sh", root, directory, path,
        ])
        .stdin(Stdio::piped())
        .spawn()
        .context("spawn docker exec for owner attachment")?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin.write_all(bytes).await?;
    drop(stdin);
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        bail!(
            "store owner attachment failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Read one sealed attachment. Only a parsed UUID reaches the fixed path
/// template; metadata and integrity are checked against OrgIntel by the
/// authenticated daemon route.
pub async fn read_owner_attachment(company: &str, attachment_id: Uuid) -> Result<Vec<u8>> {
    validate_company_name(company)?;
    let name = container_name(company);
    let directory = format!("/var/lib/restless-owner-attachments/{attachment_id}");
    let content = docker_observe(&["exec", &name, "cat", &format!("{directory}/content")]).await?;
    if !content.status.success() {
        bail!("attachment is absent from the company computer");
    }
    Ok(content.stdout)
}

/// Move an OrgIntel-fenced stale attachment out of its published path before
/// deletion. The durable claim remains until deletion completes, so a crash
/// during collection is recovered by the next bounded pass.
pub async fn quarantine_owner_attachment(company: &str, attachment_id: Uuid) -> Result<()> {
    validate_company_name(company)?;
    let name = container_name(company);
    let source = format!("/var/lib/restless-owner-attachments/{attachment_id}");
    let quarantine_root = "/var/lib/restless-owner-attachment-quarantine";
    let target = format!("{quarantine_root}/{attachment_id}");
    let script = "mkdir -p \"$1\"; chown root:root \"$1\"; chmod 0700 \"$1\"; if [ -e \"$2\" ]; then mv \"$2\" \"$3\"; elif [ -e \"$3\" ]; then :; else :; fi";
    let output = docker(&[
        "exec",
        &name,
        "sh",
        "-c",
        script,
        "sh",
        quarantine_root,
        &source,
        &target,
    ])
    .await?;
    if !output.status.success() {
        bail!("quarantine owner attachment failed");
    }
    Ok(())
}

/// Permanently remove only a quarantined, durably unlinked candidate. The
/// published path is never touched here.
pub async fn remove_quarantined_owner_attachment(company: &str, attachment_id: Uuid) -> Result<()> {
    validate_company_name(company)?;
    let name = container_name(company);
    let quarantine = format!("/var/lib/restless-owner-attachment-quarantine/{attachment_id}");
    let output = docker(&["exec", &name, "rm", "-r", "-f", "--", &quarantine]).await?;
    if !output.status.success() {
        bail!("remove quarantined owner attachment failed");
    }
    Ok(())
}

/// Roll back files whose message could not be recorded. The target is one
/// daemon-generated UUID directory beneath the fixed attachment root.
pub async fn remove_owner_attachment(company: &str, attachment_id: Uuid) -> Result<()> {
    validate_company_name(company)?;
    let name = container_name(company);
    let path = format!("/var/lib/restless-owner-attachments/{attachment_id}");
    let output = docker(&["exec", &name, "rm", "-r", "-f", "--", &path]).await?;
    if !output.status.success() {
        bail!("roll back owner attachment failed");
    }
    let quarantine_path = format!("/var/lib/restless-owner-attachment-quarantine/{attachment_id}");
    let output = docker(&["exec", &name, "rm", "-r", "-f", "--", &quarantine_path]).await?;
    if !output.status.success() {
        bail!("remove quarantined owner attachment failed");
    }
    Ok(())
}

async fn run_ok(args: &[&str]) -> Result<()> {
    let out = docker(args).await?;
    if !out.status.success() {
        bail!(
            "docker {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// The state root: $RESTLESS_HOME or ~/.restless.
/// Period spend at the owner's last limit change, and the UTC month it was counted in (`None`
/// for a total budget). A baseline from an earlier month no longer applies.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SpendWarningBaseline {
    pub micro_usd: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<String>,
}

/// A site the owner may send through their own computer: a plain host name with a dot, no scheme,
/// port, path or address.
pub fn valid_home_site(site: &str) -> bool {
    let site = site.trim();
    site.len() <= 253
        && site.contains('.')
        && !site.starts_with('.')
        && !site.ends_with('.')
        && site.parse::<std::net::IpAddr>().is_err()
        && site.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'.'
        })
}

/// The owner's timezone and language as their own browser reported them, for
/// the company browsers (S104 F3).
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BrowserLocale {
    pub timezone: Option<String>,
    pub language: Option<String>,
}

/// An IANA zone (`Australia/Sydney`) or a language tag (`en-AU`): short, and
/// safe to pass through `docker run --env` and Chrome's `--lang`.
pub fn valid_browser_locale(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "/_+-".contains(c))
}

fn browser_locale_path() -> PathBuf {
    state_root().join("browser-locale.json")
}

pub fn load_browser_locale() -> BrowserLocale {
    std::fs::read(browser_locale_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Record the owner's reported locale. Returns whether it changed; company
/// browsers take it the next time their computer starts.
pub fn save_browser_locale(locale: &BrowserLocale) -> Result<bool> {
    for value in [&locale.timezone, &locale.language].into_iter().flatten() {
        anyhow::ensure!(
            valid_browser_locale(value),
            "not a timezone or language: {value:?}"
        );
    }
    if load_browser_locale() == *locale {
        return Ok(false);
    }
    let path = browser_locale_path();
    let staged = path.with_extension("json.tmp");
    std::fs::write(&staged, serde_json::to_vec(locale)?)?;
    std::fs::rename(&staged, &path)?;
    Ok(true)
}

/// Whether two images' build-input labels name the same inputs. A missing label (a local
/// development build) never matches.
fn same_build_input_label(held: Option<&str>, wanted: Option<&str>) -> bool {
    let real = |value: &str| {
        let value = value.trim();
        value.len() >= 32 && value != "<no value>"
    };
    matches!((held, wanted), (Some(held), Some(wanted)) if real(held) && held.trim() == wanted.trim())
}

pub fn state_root() -> PathBuf {
    if let Ok(root) = std::env::var("RESTLESS_HOME") {
        return PathBuf::from(root);
    }
    let home = std::env::var("HOME").expect("HOME");
    PathBuf::from(home).join(".restless")
}

#[cfg(test)]
mod kept_files_e2e {
    /// A shell that kept files in its own layer is replaced without losing them (opt in:
    /// RESTLESS_RUNTIME_E2E=1 with RESTLESS_COMPANY_IMAGE and a RESTLESS_RESOURCE_NAMESPACE).
    #[tokio::test]
    async fn kept_files_survive_the_shell_being_replaced() {
        if std::env::var("RESTLESS_RUNTIME_E2E").as_deref() != Ok("1") {
            return;
        }
        let company = format!("keep{}_test", uuid::Uuid::new_v4().simple());
        let name = super::container_name(&company);
        let image = super::company_image();
        let volume = super::attachments_volume_name(&company);
        let result = async {
            // The old shape: no attachments volume, the file in the container's own layer.
            super::run_ok(&["run", "-d", "--name", &name, "--user", "0", "--entrypoint", "sleep", &image, "600"]).await?;
            super::run_ok(&["exec", &name, "sh", "-c", "mkdir -p /var/lib/restless-owner-attachments/a1 && printf kept > /var/lib/restless-owner-attachments/a1/content"]).await?;
            super::run_ok(&["stop", "--time", "1", &name]).await?;
            super::preserve_attachments(&company, &image).await?;
            let mount = format!("{volume}:/var/lib/restless-owner-attachments");
            let read = super::docker_observe(&["run", "--rm", "-v", &mount, "--entrypoint", "cat", &image, "/var/lib/restless-owner-attachments/a1/content"]).await?;
            anyhow::ensure!(String::from_utf8_lossy(&read.stdout) == "kept", "the kept file did not reach its volume");
            // A second replacement of a shell that never kept anything is nothing to carry.
            super::preserve_attachments(&company, &image).await?;
            anyhow::Ok(())
        }
        .await;
        let _ = super::docker_observe(&["rm", "-f", &name]).await;
        let _ = super::docker_observe(&["volume", "rm", &volume]).await;
        result.unwrap();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn clean_mapped_cache_does_not_strand_an_idle_actor() {
        let root = std::env::temp_dir().join(format!(
            "actor_cache_{}_test",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir(&root).unwrap();
        for (name, value) in [
            ("memory.max", "805306368"),
            ("memory.high", "max"),
            ("memory.current", "358072320"),
            ("cpu.max", "50000 100000"),
        ] {
            std::fs::write(root.join(name), value).unwrap();
        }
        let write_stat = |protected: &str| {
            let mut values = std::collections::BTreeMap::from([
                ("file", 303079424_u64),
                ("inactive_file", 80924672),
                ("active_file", 222154752),
                ("file_mapped", 222154752),
                ("shmem", 0),
                ("file_dirty", 0),
                ("file_writeback", 0),
                ("unevictable", 0),
            ]);
            if !protected.is_empty() {
                values.insert(protected, 262144000);
            }
            std::fs::write(
                root.join("memory.stat"),
                values
                    .iter()
                    .map(|(key, value)| format!("{key} {value}\n"))
                    .collect::<String>(),
            )
            .unwrap();
        };
        write_stat("");
        assert!(
            !super::actor_cgroup_capacity(&root).unwrap().2,
            "the real idle snapshot must admit a turn without waiting for cache aging"
        );
        for protected in ["shmem", "file_dirty", "file_writeback", "unevictable"] {
            write_stat(protected);
            assert!(
                super::actor_cgroup_capacity(&root).unwrap().2,
                "{protected} is not clean reclaimable cache"
            );
        }
        std::fs::write(root.join("memory.stat"), "file 303079424\n").unwrap();
        assert!(
            super::actor_cgroup_capacity(&root).is_err(),
            "unknown protected memory is not zero"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn actor_capacity_follows_cpu_and_reserves_startup_memory() {
        let mib = 1024 * 1024;
        assert_eq!(super::actor_slots(768 * mib, "50000 100000").unwrap(), 1);
        assert_eq!(super::actor_slots(4096 * mib, "300000 100000").unwrap(), 3);
        assert_eq!(super::actor_slots(1024 * mib, "max 100000").unwrap(), 2);
        assert!(super::actor_slots(1024 * mib, "100000 0").is_err());
        let root = std::env::temp_dir().join(format!(
            "actor_capacity_{}_test",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir(&root).unwrap();
        for (name, value) in [
            ("memory.max", "805306368"),
            ("memory.high", "max"),
            ("memory.current", "314572800"),
            (
                "memory.stat",
                "file 0\nshmem 0\nfile_dirty 0\nfile_writeback 0\nunevictable 0\n",
            ),
            ("cpu.max", "50000 100000"),
        ] {
            std::fs::write(root.join(name), value).unwrap();
        }
        assert!(
            !super::admission_from_cgroup(&root)
                .unwrap()
                .waiting_for_memory
        );
        assert_eq!(
            super::actor_cgroup_capacity(&root).unwrap(),
            (1, 0, true),
            "a new harness needs startup room"
        );
        std::fs::write(root.join("memory.current"), "52428800").unwrap();
        assert_eq!(super::actor_cgroup_capacity(&root).unwrap(), (1, 1, false));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_resend_binding_migrates_on_load_and_persists_on_save() {
        let root = std::env::temp_dir().join(format!("restless-binding-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        let path = root.join("companies/bindings_test.toml");
        std::fs::write(&path, "name = \"bindings_test\"\nmission = \"test\"\nmodel = \"openai/test\"\n[credentials]\nresend = \"infisical:/companies/bindings_test/resend\"\n").unwrap();
        let mut config = super::CompanyConfig::load(&root, "bindings_test").unwrap();
        assert!(!config.credentials.contains_key("resend"));
        assert_eq!(
            config.credentials["resend.production"],
            "infisical:/companies/bindings_test/resend"
        );
        // A deployed production binding wins when an older alias also exists; no backend value
        // is deleted by migrating configuration names.
        config
            .credentials
            .insert("resend".into(), "env:OTHER_KEY".into());
        super::CompanyConfig::save(&root, &config).unwrap();
        let persisted = std::fs::read_to_string(&path).unwrap();
        assert!(!persisted.contains("OTHER_KEY"));
        let loaded = super::CompanyConfig::load(&root, "bindings_test").unwrap();
        assert_eq!(
            loaded.credentials["resend.production"],
            "infisical:/companies/bindings_test/resend"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn desktop_assets_stay_inside_the_novnc_root() {
        for good in [
            "vnc.html",
            "core/rfb.js",
            "app/images/icons/novnc-192x192.png",
            "vendor/pako/lib/zlib/inflate.js",
        ] {
            assert!(super::desktop_asset_path_ok(good), "{good}");
        }
        for bad in [
            "",
            "/etc/passwd",
            "../x",
            "app/../../etc/shadow",
            "a//b",
            "./vnc.html",
            "vnc.html?x=1",
            "a b",
            "app/%2e%2e/x",
        ] {
            assert!(!super::desktop_asset_path_ok(bad), "{bad}");
        }
    }

    use super::{
        bound_docker_call, company_statuses, container_name, docker_observe,
        is_immutable_image_digest, is_runtime_review_download, owner_attachment_alias,
        read_owner_attachment, remove_owner_attachment, resolve_company_image,
        resolve_resource_bound, running_company_names, sort_source_files, store_owner_attachment,
        AgentIntelligence, ContainerStatus, NativeHarnessConfig, COMPANY_IMAGE, DEFAULT_CPUS,
        DEFAULT_MEMORY, DEFAULT_PIDS_LIMIT,
    };

    #[test]
    fn a_new_company_cannot_take_an_app_route_but_an_existing_one_still_loads() {
        for name in super::RESERVED_COMPANY_NAMES {
            assert!(super::validate_new_company_name(name).is_err(), "{name}");
            assert!(super::validate_company_name(name).is_ok(), "{name}");
        }
        assert!(super::validate_new_company_name("acme_goods").is_ok());
    }

    #[test]
    fn legacy_unconfigured_sentinel_round_trips_without_becoming_a_model_route() {
        let root = std::env::temp_dir().join(format!(
            "restless-legacy-unconfigured-company-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        let config: CompanyConfig = toml::from_str(
            r#"name = "legacy_unconfigured_test"
mission = "Choose intelligence later"
model = "unconfigured/pending"
"#,
        )
        .unwrap();
        assert_eq!(config.configured_model(), None);
        assert!(!config.has_configured_model_route());
        assert!(config.model_candidates().unwrap().is_empty());
        assert_eq!(config.for_agent("exec").configured_model(), None);
        CompanyConfig::save(&root, &config).unwrap();
        let restored = CompanyConfig::load(&root, "legacy_unconfigured_test").unwrap();
        assert_eq!(restored.model, "unconfigured/pending");
        assert_eq!(restored.configured_model(), None);
        assert!(restored.model_candidates().unwrap().is_empty());

        let explicit_real_route: CompanyConfig = toml::from_str(
            r#"name = "explicit_unconfigured_provider"
mission = "Use an explicitly configured provider"
model = "unconfigured/real-model"
"#,
        )
        .unwrap();
        assert_eq!(
            explicit_real_route.configured_model(),
            Some("unconfigured/real-model")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn effective_routes_accept_native_assignments_without_weakening_direct_guards() {
        let mut empty: CompanyConfig = toml::from_str(
            r#"name = "effective_route_test"
mission = "Choose intelligence"
"#,
        )
        .unwrap();
        assert!(!empty.has_effective_model_route(AgentHarness::RestlessManaged));

        empty.model = "openai/gpt-5".into();
        assert!(empty.has_effective_model_route(AgentHarness::RestlessManaged));
        empty.model = "unconfigured/pending".into();
        assert!(!empty.has_effective_model_route(AgentHarness::RestlessManaged));

        empty.native_harnesses.insert(
            "codex".into(),
            NativeHarnessConfig {
                mode: "oauth".into(),
                model: "gpt-5".into(),
                credential_reference: None,
            },
        );
        empty.agent_intelligence.insert(
            "default".into(),
            AgentIntelligence {
                connection: "harness:codex".into(),
                model: "gpt-5".into(),
            },
        );
        let exec = empty.for_agent("exec");
        assert!(exec.configured_model().is_none());
        assert!(exec.has_effective_model_route(exec.coordination_harness));

        let mut actor_specific = empty.clone();
        actor_specific.model.clear();
        actor_specific.agent_intelligence.remove("default");
        actor_specific.agent_intelligence.insert(
            "writer".into(),
            AgentIntelligence {
                connection: "harness:codex".into(),
                model: "gpt-5".into(),
            },
        );
        let writer = actor_specific.for_agent("writer");
        assert!(writer.configured_model().is_none());
        assert!(writer.has_effective_model_route(writer.worker_harness));
        let exec = actor_specific.for_agent("exec");
        assert!(!exec.has_effective_model_route(exec.coordination_harness));
    }
    use uuid::Uuid;

    #[tokio::test]
    async fn an_opaque_docker_observation_cannot_wait_forever() {
        let result = bound_docker_call(
            std::future::pending::<anyhow::Result<()>>(),
            std::time::Duration::from_millis(1),
        )
        .await;
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("docker command exceeded"));
    }

    #[tokio::test]
    async fn company_actor_cannot_mutate_daemon_sealed_owner_attachment() {
        let Ok(company) = std::env::var("RESTLESS_ATTACHMENT_TEST_COMPANY") else {
            eprintln!("RESTLESS_ATTACHMENT_TEST_COMPANY unset; skipping Runtime custody test");
            return;
        };
        let attachment_id = Uuid::new_v4();
        let original = b"daemon sealed evidence";
        let path = store_owner_attachment(&company, attachment_id, original)
            .await
            .expect("store attachment in daemon-owned Runtime custody");
        let container = container_name(&company);
        let mutation = docker_observe(&[
            "exec",
            "--user",
            "company",
            &container,
            "sh",
            "-c",
            "printf compromised > \"$1\"",
            "sh",
            &path,
        ])
        .await
        .expect("attempt mutation as company account");
        assert!(
            !mutation.status.success(),
            "the Runtime company account unexpectedly mutated daemon-owned evidence"
        );
        assert_eq!(
            read_owner_attachment(&company, attachment_id)
                .await
                .expect("read sealed evidence"),
            original
        );
        remove_owner_attachment(&company, attachment_id)
            .await
            .expect("remove test evidence as daemon root");
    }

    #[test]
    fn startup_runtime_inventory_is_one_exact_name_filter() {
        let config = |name: &str| {
            toml::from_str::<super::CompanyConfig>(&format!(
                "name = {name:?}\nmodel = \"zai/glm-5.3\"\n"
            ))
            .unwrap()
        };
        let configs = vec![config("alpha_test"), config("beta_test")];
        let names = format!("{}\nunrelated\n", super::container_name("beta_test"));
        assert_eq!(running_company_names(&configs, &names), vec!["beta_test"]);
    }

    #[test]
    fn owner_catalogue_status_is_one_exact_batch_projection() {
        let config = |name: &str| {
            toml::from_str::<super::CompanyConfig>(&format!(
                "name = {name:?}\nmodel = \"zai/glm-5.3\"\n"
            ))
            .unwrap()
        };
        let configs = vec![config("alpha"), config("alpha_two"), config("missing")];
        let rows = format!(
            "{}\trunning\n{}\texited\n{}\trunning\n",
            super::container_name("alpha"),
            super::container_name("alpha_two"),
            "restless-co-alpha-prefix"
        );
        let statuses = company_statuses(&configs, &rows);
        assert_eq!(statuses.get("alpha"), Some(&ContainerStatus::Running));
        assert_eq!(statuses.get("alpha_two"), Some(&ContainerStatus::Stopped));
        assert_eq!(statuses.get("missing"), Some(&ContainerStatus::Absent));
    }

    /// Run against a disposable company from scripts/restless-dev with
    /// RESTLESS_BROWSER_IDLE_MS=1500. Tests the real Runtime entry points.
    #[tokio::test]
    #[ignore = "requires an isolated RESTLESS_COMPUTE_SMOKE_COMPANY and a short browser idle timeout"]
    async fn cold_browser_opens_for_owner_and_survives_broker_restart() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let company = std::env::var("RESTLESS_COMPUTE_SMOKE_COMPANY").unwrap();
        assert!(company.ends_with("_test"));
        super::validate_company_name(&company).unwrap();
        assert!(!std::env::var("RESTLESS_RESOURCE_NAMESPACE")
            .unwrap_or_default()
            .is_empty());
        let container = super::container_name(&company);
        async fn probe(container: &str) -> serde_json::Value {
            let output = super::docker_bounded(
                &[
                    "exec",
                    container,
                    "curl",
                    "-fsS",
                    "-m",
                    "5",
                    "-H",
                    "x-restless-probe: 1",
                    "http://127.0.0.1:9223/json/version",
                ],
                std::time::Duration::from_secs(8),
            )
            .await
            .unwrap();
            assert!(output.status.success());
            serde_json::from_slice(&output.stdout).unwrap()
        }
        assert_eq!(probe(&container).await["asleep"], true);
        // The viewer path must wake Chrome before returning a useful transport.
        let mut stream = super::desktop_stream(&company).await.unwrap();
        stream.write_all(b"GET /websockify HTTP/1.1\r\nHost: localhost\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n").await.unwrap();
        let mut bytes = [0; 16];
        let count =
            tokio::time::timeout(std::time::Duration::from_secs(5), stream.read(&mut bytes))
                .await
                .unwrap()
                .unwrap();
        assert!(count > 0 && bytes.starts_with(b"HTTP/"));
        assert_ne!(probe(&container).await["asleep"], true);
        drop(stream);
        tokio::time::sleep(std::time::Duration::from_secs(12)).await;
        assert_eq!(probe(&container).await["asleep"], true);
        // Opening a link has a distinct cold-start readiness window.
        super::open_browser_url(&company, "https://example.com/?restless-resource-smoke=1")
            .await
            .unwrap();
        assert_ne!(probe(&container).await["asleep"], true);
        let restarted = super::docker_bounded(
            &[
                "exec",
                "-u",
                "company",
                &container,
                "supervisorctl",
                "-c",
                "/etc/supervisor/conf.d/computer.conf",
                "restart",
                "browser-broker",
            ],
            std::time::Duration::from_secs(10),
        )
        .await
        .unwrap();
        assert!(restarted.status.success());
        assert_ne!(
            probe(&container).await["asleep"],
            true,
            "broker-only restart must recognize an already running browser"
        );
    }

    #[tokio::test]
    #[ignore = "requires an explicitly owned disposable probe container and pool"]
    async fn shared_pool_pressure_defers_diagnostic_execs() {
        let name = std::env::var("RESTLESS_TEST_PROBE_CONTAINER").unwrap();
        assert!(name.starts_with("restless-s109probe-") && name.ends_with("_test"));
        assert_eq!(
            super::workload_cgroup_parent().unwrap().as_deref(),
            Some("restless-s109probe.slice")
        );
        let path = super::workload_container_scope(&name)
            .await
            .unwrap()
            .unwrap();
        assert!(
            !super::container_admission(&path)
                .unwrap()
                .unwrap()
                .waiting_for_memory
        );
        assert!(
            super::shared_workload_admission()
                .unwrap()
                .unwrap()
                .waiting_for_memory
        );
        let procs = || std::fs::read_to_string(path.join("cgroup.procs")).unwrap();
        let events = || std::fs::read_to_string(path.join("memory.events")).unwrap();
        let before = procs();
        let before_events = events();
        let mut probes = tokio::task::JoinSet::new();
        for index in 0..30 {
            let name = name.clone();
            let program = ["true", "cat", "sh", "node", "restless", "supervisorctl"][index % 6];
            probes.spawn(async move { super::docker_diagnostic(&["exec", &name, program]).await });
        }
        while let Some(result) = probes.join_next().await {
            assert!(result.unwrap().is_err());
        }
        assert!(super::resource_doctor("guard_test").await.is_none());
        assert_eq!(procs(), before);
        assert_eq!(events(), before_events);
        println!("30 probes deferred by the busy shared pool while individual Runtime still had headroom; no OCI growth");
    }

    #[tokio::test]
    #[ignore = "requires an explicitly owned disposable probe container and pool"]
    async fn diagnostic_exec_pressure_defers_without_oci_growth_and_recovers() {
        let name = std::env::var("RESTLESS_TEST_PROBE_CONTAINER").unwrap();
        assert!(name.starts_with("restless-s109probe-") && name.ends_with("_test"));
        assert_eq!(
            super::workload_cgroup_parent().unwrap().as_deref(),
            Some("restless-s109probe.slice")
        );
        let path = super::workload_container_scope(&name)
            .await
            .unwrap()
            .unwrap();
        let process_ids = || {
            std::fs::read_to_string(path.join("cgroup.procs"))
                .unwrap()
                .lines()
                .map(str::to_string)
                .collect::<std::collections::BTreeSet<_>>()
        };
        let events = || std::fs::read_to_string(path.join("memory.events")).unwrap();
        assert!(
            super::container_admission(&path)
                .unwrap()
                .unwrap()
                .waiting_for_memory
        );
        assert!(
            !super::shared_workload_admission()
                .unwrap()
                .unwrap()
                .waiting_for_memory
        );
        let before = process_ids();
        let before_events = events();
        let mut probes = tokio::task::JoinSet::new();
        for index in 0..30 {
            let name = name.clone();
            let program = ["true", "cat", "sh", "node", "restless", "supervisorctl"][index % 6];
            probes.spawn(async move { super::docker_diagnostic(&["exec", &name, program]).await });
        }
        while let Some(result) = probes.join_next().await {
            assert!(result.unwrap().is_err());
        }
        assert!(super::resource_doctor("guard_test").await.is_none());
        assert_eq!(
            process_ids(),
            before,
            "deferred probes must not create OCI startup processes"
        );
        assert_eq!(
            events(),
            before_events,
            "deferred probes must not add child OOM/max events"
        );
        let updated = super::docker_bounded(
            &["update", "--memory", "512m", "--memory-swap", "512m", &name],
            std::time::Duration::from_secs(10),
        )
        .await
        .unwrap();
        assert!(updated.status.success());
        assert!(
            !super::container_admission(&path)
                .unwrap()
                .unwrap()
                .waiting_for_memory
        );
        assert!(
            super::docker_diagnostic(&["exec", &name, "true"])
                .await
                .is_err(),
            "free memory does not erase a still-uncertain probe's retry window"
        );
        tokio::time::sleep(super::EXEC_OBSERVATION_RETRY + std::time::Duration::from_secs(1)).await;
        let recovered = super::docker_diagnostic(&["exec", &name, "true"])
            .await
            .unwrap();
        assert!(recovered.status.success());
        assert_eq!(
            process_ids(),
            before,
            "completed recovery must not leave exec processes"
        );
        assert_eq!(events(), before_events);
        let slow = super::docker_diagnostic(&["exec", &name, "sleep", "60"]).await;
        assert!(
            slow.is_err(),
            "an actual slow diagnostic must hit its Runtime deadline"
        );
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert_eq!(
            process_ids(),
            before,
            "a timed-out probe must not leave its remote process"
        );
        println!("30 deferred probes: no process growth or child OOM; actual exec recovered after backoff; remote timeout left no process");
    }

    #[tokio::test]
    async fn cancelled_diagnostic_does_not_launch_replacement_execs() {
        let slot = std::sync::Arc::new(tokio::sync::Mutex::new(None));
        let started = std::sync::Arc::new(tokio::sync::Notify::new());
        let task = tokio::spawn({
            let slot = slot.clone();
            let started = started.clone();
            async move {
                super::observe_exec(&slot, async {
                    started.notify_one();
                    std::future::pending::<anyhow::Result<()>>().await
                })
                .await
            }
        });
        started.notified().await;
        task.abort();
        let _ = task.await;
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let retry = super::observe_exec(&slot, async {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
        .await;
        assert!(retry.is_err());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        *slot.lock().await = Some(std::time::Instant::now());
        super::observe_exec(&slot, async { Ok(()) }).await.unwrap();
        assert!(slot.lock().await.is_none());
    }

    #[test]
    fn diagnostic_exec_options_never_become_the_container() {
        assert_eq!(
            super::observation_exec_index(&[
                "exec",
                "-u",
                "company",
                "-i",
                "test-container",
                "sh",
                "-c",
                "anything"
            ])
            .unwrap(),
            Some(4)
        );
        assert_eq!(
            super::observation_exec_index(&["inspect", "test-container"]).unwrap(),
            None
        );
        assert!(super::observation_exec_index(&["exec", "-u"]).is_err());
        assert!(super::observation_exec_index(&["exec", "--unknown", "x"]).is_err());
    }

    #[test]
    fn individual_runtime_waits_even_when_shared_memory_is_available() {
        let root = std::env::temp_dir().join(format!(
            "runtime_admission_{}_test",
            Uuid::new_v4().simple()
        ));
        std::fs::create_dir(&root).unwrap();
        let write = |name: &str, value: &str| std::fs::write(root.join(name), value).unwrap();
        write("memory.max", "805306368");
        write("memory.high", "max");
        write("memory.current", "629145600");
        write(
            "memory.stat",
            "file 0\nshmem 0\nfile_dirty 0\nfile_writeback 0\nunevictable 0\n",
        );
        let shared = || super::WorkloadAdmission {
            scope: "shared",
            waiting_for_memory: false,
            working_set_bytes: 100,
            resume_below_bytes: 1000,
        };
        let busy = super::combine_admission(shared(), super::container_admission(&root).unwrap());
        assert!(busy.waiting_for_memory);
        assert_eq!(busy.scope, "company");
        assert!(
            busy.resume_below_bytes <= 512 * 1024 * 1024,
            "small runtimes need room for an actual harness startup"
        );
        write("memory.current", "209715200");
        assert!(
            !super::combine_admission(shared(), super::container_admission(&root).unwrap())
                .waiting_for_memory
        );
        write("memory.max", "max");
        assert!(super::container_admission(&root).unwrap().is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workload_admission_waits_for_live_memory_and_recovers_without_cache_deadlock() {
        let root = std::env::temp_dir().join(format!(
            "workload_admission_{}_test",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir(&root).unwrap();
        let write = |name: &str, value: &str| std::fs::write(root.join(name), value).unwrap();
        write("memory.max", "1000");
        write("memory.high", "900");
        write("memory.current", "950");
        assert!(
            super::admission_from_cgroup(&root).is_err(),
            "unknown cache is not zero"
        );
        write(
            "memory.stat",
            "anon 900\nfile 50\nshmem 0\nfile_dirty 0\nfile_writeback 0\nunevictable 0\n",
        );
        assert!(
            super::admission_from_cgroup(&root)
                .unwrap()
                .waiting_for_memory
        );
        write(
            "memory.stat",
            "anon 100\nfile 850\nshmem 0\nfile_dirty 0\nfile_writeback 0\nunevictable 0\n",
        );
        assert!(
            !super::admission_from_cgroup(&root)
                .unwrap()
                .waiting_for_memory,
            "reclaimable file cache must not strand queued Work"
        );
        write("memory.current", "200");
        write(
            "memory.stat",
            "file 0\nshmem 0\nfile_dirty 0\nfile_writeback 0\nunevictable 0\n",
        );
        assert!(
            !super::admission_from_cgroup(&root)
                .unwrap()
                .waiting_for_memory
        );
        write("memory.high", "max");
        assert_eq!(
            super::admission_from_cgroup(&root)
                .unwrap()
                .resume_below_bytes,
            450
        );
        write("memory.high", "0");
        assert!(super::admission_from_cgroup(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workload_parent_is_a_scoped_systemd_slice() {
        for valid in ["restless-workloads.slice", "restless-s109caps.slice"] {
            assert!(super::validate_workload_parent(valid).is_ok());
        }
        for invalid in [
            "",
            "system.slice",
            "restless-.slice",
            "restless-workloads.slice/other",
            "../restless-workloads.slice",
            "restless-workloads.service",
            "restless-☃.slice",
        ] {
            assert!(
                super::validate_workload_parent(invalid).is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn configured_workload_pool_refuses_missing_unbounded_or_swappable_caps() {
        let root = std::env::temp_dir().join(format!(
            "workload_caps_{}_test",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir(&root).unwrap();
        assert!(super::validate_workload_cgroup(&root).is_err());
        std::fs::write(root.join("memory.max"), "1073741824\n").unwrap();
        std::fs::write(root.join("cpu.max"), "50000 100000\n").unwrap();
        std::fs::write(root.join("memory.swap.max"), "0\n").unwrap();
        assert!(super::validate_workload_cgroup(&root).is_ok());
        std::fs::write(root.join("memory.max"), "max\n").unwrap();
        assert!(super::validate_workload_cgroup(&root).is_err());
        std::fs::write(root.join("memory.max"), "1073741824\n").unwrap();
        std::fs::write(root.join("cpu.max"), "max 100000\n").unwrap();
        assert!(super::validate_workload_cgroup(&root).is_err());
        std::fs::write(root.join("cpu.max"), "50000 100000\n").unwrap();
        std::fs::write(root.join("memory.swap.max"), "1073741824\n").unwrap();
        assert!(super::validate_workload_cgroup(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    #[ignore = "requires an existing disposable company in an explicitly configured temporary pool"]
    async fn missing_pool_refuses_reconcile_without_stopping_live_company() {
        let company = std::env::var("RESTLESS_TEST_RESOURCE_COMPANY").unwrap();
        assert!(company.ends_with("_test"));
        let parent = super::workload_cgroup_parent().unwrap().unwrap();
        assert_ne!(parent, "restless-workloads.slice");
        let root = std::env::var("RESTLESS_WORKLOAD_CGROUP_ROOT").unwrap();
        let config = super::CompanyConfig::load(&super::state_root(), &company).unwrap();
        let name = super::container_name(&company);
        assert_eq!(
            super::status(&company).await.unwrap(),
            super::ContainerStatus::Running
        );
        let before = super::inspect_value(&["inspect", "-f", "{{.Id}}", &name])
            .await
            .unwrap();
        std::env::set_var(
            "RESTLESS_WORKLOAD_CGROUP_ROOT",
            std::env::temp_dir().join(format!("missing_pool_{}_test", uuid::Uuid::new_v4())),
        );
        let result = super::up(&config, true).await;
        std::env::set_var("RESTLESS_WORKLOAD_CGROUP_ROOT", root);
        assert!(result.is_err());
        assert_eq!(
            super::status(&company).await.unwrap(),
            super::ContainerStatus::Running
        );
        assert_eq!(
            super::inspect_value(&["inspect", "-f", "{{.Id}}", &name])
                .await
                .unwrap(),
            before
        );
    }

    #[test]
    fn compute_inventory_keeps_company_and_profile_boundaries() {
        let own = serde_json::json!({"company":"acme_test", "namespace":"own_test"});
        assert!(super::compute_container_belongs(
            &own,
            "acme_test",
            "own_test",
            "/own",
            "own-co"
        ));
        assert!(!super::compute_container_belongs(
            &own,
            "other_test",
            "own_test",
            "/own",
            "own-co"
        ));
        assert!(!super::compute_container_belongs(
            &own,
            "acme_test",
            "another_test",
            "/own",
            "own-co"
        ));
        let document =
            serde_json::json!({"documents_company":"acme_test","documents_root":"/another"});
        assert!(!super::compute_container_belongs(
            &document,
            "acme_test",
            "own_test",
            "/own",
            "own-co"
        ));
        let job = serde_json::json!({"job_company":"acme_test","name":"another-co-job-123"});
        assert!(!super::compute_container_belongs(
            &job,
            "acme_test",
            "own_test",
            "/own",
            "own-co"
        ));
    }

    #[test]
    fn compute_usage_keeps_counter_resets_unknown() {
        let before = super::ComputeCounter {
            memory: 100,
            memory_limit: Some(1000),
            cpu: 1000000,
            cpu_limit: Some(1.0),
        };
        let after = super::ComputeCounter {
            memory: 200,
            memory_limit: Some(1000),
            cpu: 1100000,
            cpu_limit: Some(1.0),
        };
        let sample = super::compute_sample(&before, after, 0.2);
        assert_eq!(sample.cpu_used_cores, Some(0.5));
        assert_eq!(sample.memory_used_bytes, 200);
        let reset = super::ComputeCounter {
            memory: 20,
            memory_limit: Some(1000),
            cpu: 1,
            cpu_limit: Some(1.0),
        };
        assert!(super::compute_sample(&before, reset, 0.2)
            .cpu_used_cores
            .is_none());
    }

    #[test]
    fn source_digest_uses_portable_relative_byte_order() {
        let root = std::path::Path::new("/source");
        let mut files = vec![
            root.join("crates/restless/src/main.rs"),
            root.join("crates/restless-model-gateway/src/lib.rs"),
            root.join("crates/restlessd/src/staff.rs"),
            root.join("crates/restlessd/src/staff/context.rs"),
        ];
        sort_source_files(root, &mut files);
        assert_eq!(
            files,
            vec![
                root.join("crates/restless-model-gateway/src/lib.rs"),
                root.join("crates/restless/src/main.rs"),
                root.join("crates/restlessd/src/staff.rs"),
                root.join("crates/restlessd/src/staff/context.rs"),
            ]
        );
    }

    #[test]
    fn hosted_runtime_reference_accepts_only_an_exact_oci_digest() {
        let digest = "a".repeat(64);
        assert!(is_immutable_image_digest(&format!(
            "registry.example/restless/company@sha256:{digest}"
        )));
        for mutable_or_malformed in [
            "restless-company-image:latest",
            "registry.example/restless/company:0.0.0",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "registry.example/restless/company@sha256:abc",
            "registry.example/restless/company@sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ] {
            assert!(
                !is_immutable_image_digest(mutable_or_malformed),
                "accepted {mutable_or_malformed:?} as immutable"
            );
        }
    }

    #[test]
    fn owner_attachment_alias_is_one_safe_name_beside_content() {
        assert_eq!(
            owner_attachment_alias("Invoice March (final).PDF").as_deref(),
            Some("Invoice-March-final.pdf")
        );
        assert_eq!(
            owner_attachment_alias("q3.report.xlsx").as_deref(),
            Some("q3-report.xlsx")
        );
        assert_eq!(
            owner_attachment_alias("合同.docx").as_deref(),
            Some("file.docx")
        );
        assert_eq!(
            owner_attachment_alias("content").as_deref(),
            Some("file-content")
        );
        assert_eq!(
            owner_attachment_alias("content.tmp").as_deref(),
            Some("file-content.tmp")
        );
        assert_eq!(owner_attachment_alias("..."), None);
        assert_eq!(owner_attachment_alias(""), None);
        for hostile in [
            "../../etc/passwd",
            "..\\..\\boot.ini",
            "/var/lib/restless-owner-attachments/x/content",
            ".hidden",
            "a\nb.pdf",
            "$(reboot).sh",
            "-rf .txt",
        ] {
            let alias = owner_attachment_alias(hostile);
            if let Some(alias) = alias.as_deref() {
                assert!(
                    !alias.starts_with(['.', '-'])
                        && alias != "content"
                        && alias.matches('.').count() <= 1
                        && alias
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')),
                    "{hostile:?} produced unsafe alias {alias:?}"
                );
            }
        }
    }

    #[test]
    fn local_runtime_reference_keeps_the_appliance_default() {
        assert_eq!(resolve_company_image(None), COMPANY_IMAGE);
        assert_eq!(resolve_company_image(Some("  ")), COMPANY_IMAGE);
        assert_eq!(
            resolve_company_image(Some(" registry.example/company@sha256:abc ")),
            "registry.example/company@sha256:abc"
        );
    }

    #[test]
    fn an_absent_override_keeps_the_default_bound() {
        assert_eq!(
            resolve_resource_bound(None, DEFAULT_CPUS).as_deref(),
            Some(DEFAULT_CPUS)
        );
        assert_eq!(
            resolve_resource_bound(None, DEFAULT_MEMORY).as_deref(),
            Some(DEFAULT_MEMORY)
        );
        assert_eq!(
            resolve_resource_bound(None, DEFAULT_PIDS_LIMIT).as_deref(),
            Some(DEFAULT_PIDS_LIMIT)
        );
    }

    #[test]
    fn an_explicit_override_replaces_the_default_bound() {
        assert_eq!(
            resolve_resource_bound(Some("8.0"), DEFAULT_CPUS).as_deref(),
            Some("8.0")
        );
        // Surrounding whitespace is an editing artefact, not a value; passing
        // it through would hand docker an argument it rejects at create time.
        assert_eq!(
            resolve_resource_bound(Some("  6g \n"), DEFAULT_MEMORY).as_deref(),
            Some("6g")
        );
    }

    #[test]
    fn the_memory_bound_leaves_room_for_the_rest_of_the_host() {
        let gib = 1024 * 1024 * 1024;
        assert_eq!(
            super::fit_memory(Some("4g".into()), Some(4 * gib)).as_deref(),
            Some("3072m")
        );
        assert_eq!(
            super::fit_memory(Some("4g".into()), Some(16 * gib)).as_deref(),
            Some("4g")
        );
        assert_eq!(
            super::fit_memory(Some("4g".into()), None).as_deref(),
            Some("4g")
        );
        assert_eq!(super::fit_memory(None, Some(4 * gib)), None);
        assert_eq!(super::memory_bytes("512m"), Some(512 * 1024 * 1024));
        assert_eq!(super::memory_bytes("2GB"), Some(2 * gib));
        assert_eq!(super::memory_bytes("lots"), None);
    }

    #[test]
    fn the_cpu_bound_never_exceeds_the_host() {
        // One core stays with the host.
        assert_eq!(
            super::fit_cpus(Some("4.0".into()), 2).as_deref(),
            Some("1.0")
        );
        assert_eq!(
            super::fit_cpus(Some("4.0".into()), 4).as_deref(),
            Some("3.0")
        );
        assert_eq!(
            super::fit_cpus(Some("4.0".into()), 8).as_deref(),
            Some("4.0")
        );
        assert_eq!(
            super::fit_cpus(Some("1.5".into()), 2).as_deref(),
            Some("1.0")
        );
        assert_eq!(
            super::fit_cpus(Some("1.5".into()), 4).as_deref(),
            Some("1.5")
        );
        assert_eq!(
            super::fit_cpus(Some("4.0".into()), 0).as_deref(),
            Some("1.0")
        );
        assert_eq!(super::fit_cpus(None, 2), None);
    }

    #[test]
    fn an_empty_override_disables_that_bound_rather_than_passing_an_empty_flag() {
        // The escape hatch for diagnosing whether a bound is itself the
        // problem. It must yield None so the flag is omitted entirely — an
        // empty string would become `--cpus ""` and fail the run.
        assert_eq!(resolve_resource_bound(Some(""), DEFAULT_CPUS), None);
        assert_eq!(resolve_resource_bound(Some("   "), DEFAULT_MEMORY), None);
    }

    #[test]
    fn default_bounds_leave_headroom_over_observed_healthy_load() {
        // Measured peak for a company building sites concurrently was 2.6 GiB
        // and 720 PIDs. A bound at or under that would throttle real work and
        // teach operators to disable bounding altogether, so the memory bound
        // must keep a clear margin above the observed peak rather than hug it.
        assert_eq!(DEFAULT_MEMORY, "4g");
        assert!(DEFAULT_PIDS_LIMIT.parse::<u32>().expect("pids limit") > 720);
        assert!(DEFAULT_CPUS.parse::<f64>().expect("cpus") > 0.0);
    }

    use std::path::Path;

    use super::{
        is_runtime_review_file_target, move_active_config_to_archive,
        move_archived_config_to_active, normalize_expired_browser_control, resolve_review_file,
        review_file_media_type, runtime_http_target, runtime_review_file_root,
        runtime_review_text_path, AgentHarness, CompanyConfig, SpendCeiling,
        CANONICAL_COMPANY_FILE,
    };

    #[test]
    fn model_ceiling_is_exact_and_refuses_non_finite_or_negative_values() {
        assert_eq!(
            SpendCeiling::parse("10.000001").unwrap().micro_usd(),
            10_000_001
        );
        assert_eq!(SpendCeiling::parse("0").unwrap().to_string(), "0");
        assert_eq!(SpendCeiling::parse("1.250000").unwrap().to_string(), "1.25");
        for invalid in ["inf", "NaN", "-1", "+1", "1e3", "0.0000001"] {
            assert!(SpendCeiling::parse(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn legacy_numeric_toml_loads_but_saved_ceiling_retains_exact_micro_usd() {
        let config: CompanyConfig = toml::from_str(
            r#"name = "ceiling_test"
mission = "test"
spend_ceiling_usd = 1.000001
model = "moonshot/kimi-k3"
"#,
        )
        .unwrap();
        assert_eq!(config.spend_ceiling_usd.micro_usd(), 1_000_001);
        let rendered = toml::to_string(&config).unwrap();
        assert!(rendered.contains("spend_ceiling_usd = \"1.000001\""));
    }

    #[test]
    fn a_company_file_with_a_removed_setting_still_loads() {
        // Company files written before S103 carry `outcome_standard`; they must load, and the
        // next save drops the line.
        let config: CompanyConfig = toml::from_str(
            r#"name = "retired_setting_test"
mission = "test"
model = "moonshot/kimi-k3"
outcome_standard = "frontier"
"#,
        )
        .unwrap();
        assert!(!toml::to_string(&config)
            .unwrap()
            .contains("outcome_standard"));
    }

    #[test]
    fn an_expired_owner_lease_becomes_unclaimed() {
        let state = serde_json::json!({
            "controller": "owner",
            "client_id": "owner-tab",
            "requester": "exec/session-7",
            "expires_at": "2000-01-01T00:00:00Z",
        });
        let normalized = normalize_expired_browser_control(state);
        assert_eq!(normalized["controller"], "unclaimed");
        assert_eq!(normalized["reason"], "owner_lease_expired");
    }

    #[test]
    fn a_live_owner_lease_stays_exclusive() {
        let state = serde_json::json!({
            "controller": "owner",
            "client_id": "owner-tab",
            "requester": "exec/session-7",
            "expires_at": "2999-01-01T00:00:00Z",
        });
        assert_eq!(normalize_expired_browser_control(state.clone()), state);
    }

    #[test]
    fn custom_harness_assignment_preserves_native_model_ids_and_actor_independence() {
        let config: CompanyConfig = toml::from_str(
            r#"
name = "custom_route_test"
mission = "test"
model = "openai/existing"
[agent_intelligence.alice]
connection = "harness:custom:hermes"
model = "custom:local:vendor/model"
"#,
        )
        .unwrap();
        let alice = config.for_agent("alice");
        assert_eq!(alice.worker_harness, AgentHarness::CustomAcp);
        assert_eq!(
            alice.native_model(AgentHarness::CustomAcp).as_deref(),
            Some("native-custom-hermes/custom:local:vendor/model")
        );
        assert_eq!(alice.reasoning_effort, "default");
        alice.validate_harness_models().unwrap();
        assert_eq!(config.for_agent("exec").model, "openai/existing");
        assert_eq!(
            crate::custom_harness::split_model(&alice.model).unwrap(),
            ("hermes", "custom:local:vendor/model")
        );
    }

    #[test]
    fn agent_intelligence_routes_are_independent_and_override_old_preferences() {
        let config: CompanyConfig = toml::from_str(
            r#"
name = "intelligence_test"
mission = "test"
model = "moonshot/kimi-k3"
model_failover = ["zai/glm-5"]
[agent_intelligence.exec]
connection = "direct:openai"
model = "gpt-5.4"
[agent_intelligence.alice]
connection = "direct:anthropic"
model = "claude-sonnet-4-6"
"#,
        )
        .unwrap();
        let exec = config.for_agent("exec");
        let alice = config.for_agent("alice");
        assert_eq!(exec.model, "openai/gpt-5.4");
        assert_eq!(alice.model, "anthropic/claude-sonnet-4-6");
        assert_eq!(
            exec.agent_preference("exec", Some("moonshot/kimi-k3")),
            Some("openai/gpt-5.4")
        );
        assert!(exec.model_failover.is_empty());
        assert_eq!(exec.coordination_harness, AgentHarness::RestlessManaged);
        assert_eq!(alice.worker_harness, AgentHarness::RestlessManaged);
        assert_eq!(config.for_agent("bob").model, "moonshot/kimi-k3");
        assert_eq!(config.model_failover.len(), 1);
        assert_eq!(
            config.agent_preference("bob", Some("zai/glm-5")),
            Some("zai/glm-5")
        );
        let restored: CompanyConfig = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(restored.for_agent("alice").model, alice.model);
        let mut config = config;
        config.agent_intelligence.insert(
            "default".into(),
            super::AgentIntelligence {
                connection: "direct:anthropic".into(),
                model: "claude-sonnet-4-6".into(),
            },
        );
        assert_eq!(config.for_agent("bob").model, "anthropic/claude-sonnet-4-6");
        assert_eq!(config.for_agent("exec").model, "openai/gpt-5.4");
        config.agent_intelligence.remove("exec");
        let inherited = config.for_agent("exec");
        assert_eq!(
            inherited.agent_preference("exec", Some("openai/gpt-5.6-terra")),
            Some("anthropic/claude-sonnet-4-6")
        );
        // The company default keeps the owner's ordered fallbacks; an exact override does not.
        assert_eq!(inherited.model_failover, vec!["zai/glm-5".to_string()]);
        inherited.model_candidates().unwrap();
        // On the default route, Exec may staff any model the same connection serves.
        let bob = config.for_agent("bob");
        assert_eq!(
            bob.agent_preference("bob", Some("anthropic/claude-opus-5-5")),
            Some("anthropic/claude-opus-5-5")
        );
        assert_eq!(
            bob.agent_preference("bob", Some("anthropic/claude-haiku-5-5")),
            Some("anthropic/claude-haiku-5-5")
        );
        assert_eq!(
            bob.agent_preference("bob", None),
            Some("anthropic/claude-sonnet-4-6")
        );
        // The owner's choice for one agent stays exact.
        assert_eq!(
            alice.agent_preference("alice", Some("anthropic/claude-opus-5-5")),
            Some("anthropic/claude-sonnet-4-6")
        );
    }

    #[test]
    fn model_policy_preserves_order_and_rejects_duplicates() {
        let mut config: CompanyConfig = toml::from_str(
            r#"name = "policy_test"
mission = "test"
model = "moonshot/kimi-k3"
model_failover = ["anthropic/claude-haiku-4-5", "zai/glm-5"]
"#,
        )
        .unwrap();
        assert_eq!(
            config.model_candidates().unwrap(),
            vec![
                "moonshot/kimi-k3",
                "anthropic/claude-haiku-4-5",
                "zai/glm-5"
            ]
        );
        config.model_failover.push("moonshot/kimi-k3".into());
        assert!(config.model_candidates().is_err());
    }

    #[test]
    fn unconfigured_company_round_trips_without_inventing_a_model_route() {
        let root = std::env::temp_dir().join(format!(
            "restless-unconfigured-company-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        let config: CompanyConfig = toml::from_str(
            r#"name = "unconfigured_test"
mission = "Choose intelligence later"
"#,
        )
        .unwrap();

        assert_eq!(config.configured_model(), None);
        assert!(!config.has_configured_model_route());
        assert!(config.model_candidates().unwrap().is_empty());
        assert_eq!(config.for_agent("exec").configured_model(), None);
        CompanyConfig::save(&root, &config).unwrap();

        let persisted =
            std::fs::read_to_string(root.join("companies").join("unconfigured_test.toml")).unwrap();
        assert!(!persisted.lines().any(|line| line.starts_with("model =")));
        let restored = CompanyConfig::load(&root, "unconfigured_test").unwrap();
        assert_eq!(restored.configured_model(), None);
        assert!(restored.model_candidates().unwrap().is_empty());

        let mut invalid = restored;
        invalid.model_failover.push("openai/gpt-5".into());
        assert!(invalid.model_candidates().is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn harnesses_and_reasoning_are_explicit_with_compatible_defaults() {
        assert_eq!(
            AgentHarness::parse_canonical("restless-managed"),
            Some(AgentHarness::RestlessManaged)
        );
        assert_eq!(
            AgentHarness::parse_canonical("claude-agent"),
            Some(AgentHarness::ClaudeAgent)
        );
        assert!(AgentHarness::parse_canonical("omp").is_none());
        assert!(AgentHarness::parse_canonical("claude_agent").is_none());

        let legacy: CompanyConfig = toml::from_str(
            r#"name = "legacy_test"
model = "moonshot/kimi-k3"
"#,
        )
        .unwrap();
        assert_eq!(legacy.coordination_harness, AgentHarness::RestlessManaged);
        assert_eq!(legacy.worker_harness, AgentHarness::RestlessManaged);
        assert_eq!(legacy.reasoning_effort, "medium");

        let codex: CompanyConfig = toml::from_str(
            r#"name = "codex_test"
model = "litellm/gpt-5.6-sol"
worker_runtime = "codex"
reasoning_effort = "high"
"#,
        )
        .unwrap();
        assert_eq!(codex.coordination_harness, AgentHarness::RestlessManaged);
        assert_eq!(codex.worker_harness, AgentHarness::Codex);
        assert_eq!(codex.reasoning_effort, "high");
        codex.validate_harness_models().unwrap();

        let rendered = toml::to_string(&codex).unwrap();
        assert!(rendered.contains("worker_harness = \"codex\""));
        assert!(!rendered.contains("worker_runtime"));

        let mut unknown_codex_tariff = codex.clone();
        unknown_codex_tariff.model = "litellm/gpt-5.7-unpriced".into();
        assert!(unknown_codex_tariff
            .validate_harness_models()
            .unwrap_err()
            .to_string()
            .contains("no pinned metering tariff"));

        let mut claude: CompanyConfig = toml::from_str(
            r#"name = "claude_test"
model = "anthropic/claude-sonnet-4-6"
coordination_harness = "claude-agent"
worker_harness = "claude_agent"
"#,
        )
        .unwrap();
        assert_eq!(claude.coordination_harness, AgentHarness::ClaudeAgent);
        assert_eq!(claude.worker_harness, AgentHarness::ClaudeAgent);
        claude.validate_harness_models().unwrap();

        claude.model = "anthropic/claude-sonnet-future".into();
        assert!(claude
            .validate_harness_models()
            .unwrap_err()
            .to_string()
            .contains("no pinned metering tariff"));
        claude.model = "anthropic/claude-sonnet-4-6".into();
        claude.reasoning_effort = "xhigh".into();
        assert!(claude
            .validate_harness_models()
            .unwrap_err()
            .to_string()
            .contains("reasoning_effort"));
    }

    #[test]
    fn review_targets_are_loopback_http_but_never_browser_control() {
        let target =
            runtime_http_target("http://127.0.0.1:4173/for-tutoring-centres?language=en").unwrap();
        assert_eq!(target.port, 4173);
        assert_eq!(target.path_and_query, "/for-tutoring-centres?language=en");
        for refused in [
            "https://127.0.0.1:4173/",
            "http://example.com:4173/",
            "http://localhost:6080/vnc.html",
            "http://127.0.0.1:9223/json/version",
            "http://user:secret@localhost:4173/",
            "http://127.0.0.1:4173/ (local preview)",
            " http://127.0.0.1:4173/",
        ] {
            assert!(runtime_http_target(refused).is_err(), "accepted {refused}");
        }
    }

    #[test]
    fn text_review_targets_are_bounded_to_company_markdown_or_text() {
        for accepted in [
            "/company/outputs/customer-response.md",
            "/company/report.markdown",
            "/company/notes/result.txt",
        ] {
            assert!(
                runtime_review_text_path(accepted).is_ok(),
                "refused {accepted}"
            );
        }
        for refused in [
            "/company",
            "/company/../etc/passwd",
            "/etc/passwd",
            "company/result.md",
            "/company/result.pdf",
        ] {
            assert!(
                runtime_review_text_path(refused).is_err(),
                "accepted {refused}"
            );
        }
    }

    #[test]
    fn archived_identity_moves_out_of_active_scans_and_restores_without_data_rewrite() {
        let root = std::env::temp_dir().join(format!(
            "restless-archive-contract-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        let config = CompanyConfig {
            agent_intelligence: Default::default(),
            native_harnesses: Default::default(),
            display_name: None,
            logo: None,
            name: "archive_contract_test".into(),
            internal_network: false,
            mission: "Preserve me".into(),
            spend_ceiling_usd: SpendCeiling::from_micro_usd(5_000_000),
            spend_period: Default::default(),
            spend_warning_percent: None,
            spend_warning_baseline: None,
            home_sites: Vec::new(),
            monthly_runtime_cap_hours: None,
            native_monthly_turn_limit: None,
            native_monthly_token_limit: None,
            auto_sleep_after_minutes: None,
            model: "moonshot/kimi-k3".into(),
            coordination_harness: AgentHarness::RestlessManaged,
            worker_harness: crate::runtime::AgentHarness::RestlessManaged,
            reasoning_effort: crate::acp::DEFAULT_REASONING_EFFORT.into(),
            model_failover: Vec::new(),
            credentials: std::collections::BTreeMap::new(),
            approved_parties: Vec::new(),
            everyday_model: None,
            decision_model: None,
        };
        CompanyConfig::save(&root, &config).unwrap();

        move_active_config_to_archive(&root, &config.name).unwrap();
        assert!(CompanyConfig::load(&root, &config.name).is_err());
        let archived = CompanyConfig::load_archived(&root, &config.name).unwrap();
        assert_eq!(archived.mission, "Preserve me");
        assert!(CompanyConfig::save(&root, &config).is_err());

        move_archived_config_to_active(&root, &config.name).unwrap();
        assert!(CompanyConfig::load_archived(&root, &config.name).is_err());
        assert_eq!(
            CompanyConfig::load(&root, &config.name).unwrap().mission,
            "Preserve me"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Real Docker/daemon proof for the EOF-sensitive bridge installer. It is
    /// opt-in because it writes a fresh bounded capability into a persistent
    /// Runtime, and it refuses any company that is not explicitly a test
    /// company.
    #[tokio::test]
    async fn bridge_capability_reaches_a_named_test_runtime_when_requested() {
        let Ok(company) = std::env::var("RESTLESS_RUNTIME_BRIDGE_TEST_COMPANY") else {
            return;
        };
        assert!(
            company.ends_with("_test"),
            "bridge integration test requires a *_test company"
        );

        let issuer = crate::capability::CapabilityIssuer::open(&super::state_root())
            .expect("open local Runtime capability issuer");
        let bridge = issuer
            .issue_runtime_bridge(&company)
            .expect("issue test Runtime bridge capability");
        super::install_runtime_bridge_capability(&company, &bridge)
            .await
            .expect("install Runtime bridge capability");

        let observation = super::doctor(&company)
            .await
            .expect("inspect test Runtime after bridge installation")
            .coordination
            .expect("running Runtime has a coordination observation");
        assert_eq!(
            observation.status, "available",
            "Runtime bridge should complete an authenticated status request: {:?}",
            observation.detail
        );
    }

    /// A produced file the cockpit can present, and the exact boundary of what
    /// it will serve (S19-T5). The reported failure was a finished
    /// `index.html` — a real, complete website — reaching the owner as "this
    /// outcome does not have a directly reviewable website".
    #[test]
    fn a_produced_page_is_a_reviewable_outcome_and_stays_inside_it() {
        let page = "/company/outputs/redesign/2026-08-larder-sample/index.html";
        assert!(
            is_runtime_review_file_target(page),
            "a rendered page in the company Runtime is the native outcome"
        );
        assert!(is_runtime_review_file_target("/company/outputs/plan.pdf"));
        assert!(is_runtime_review_file_target("/company/outputs/shot.png"));
        assert!(is_runtime_review_file_target("/company/outputs/demo.mp4"));
        assert!(is_runtime_review_file_target(
            "/company/outputs/board-deck.pptx"
        ));

        // Markdown keeps its own richer path: the cockpit renders it rather
        // than framing it, so this must not claim it.
        assert!(!is_runtime_review_file_target("/company/outputs/plan.md"));
        // Nothing outside the company computer, and no directory mistaken for a file.
        assert!(!is_runtime_review_file_target("/etc/passwd"));
        assert!(!is_runtime_review_file_target(
            "/company/../etc/shadow.html"
        ));
        assert!(!is_runtime_review_file_target("/company/outputs/site"));
        assert!(!is_runtime_review_file_target("/company/outputs/a\nb.pdf"));
        // A format the cockpit cannot render is still the owner's file: it is
        // offered only as a download, and only as the exact entry.
        for download in ["/company/outputs/build.tar.gz", "/company/outputs/run.sh"] {
            assert!(is_runtime_review_file_target(download), "{download}");
            let (root, entry) = runtime_review_file_root(download).unwrap();
            let resolved = resolve_review_file(&root, &entry, "/").unwrap();
            assert!(is_runtime_review_download(&resolved));
            assert_eq!(
                resolve_review_file(&root, &entry, &format!("/{entry}?download=1")).unwrap(),
                resolved
            );
        }
        let (root, entry) = runtime_review_file_root("/company/outputs/build.tar.gz").unwrap();
        assert!(resolve_review_file(&root, &entry, "/other.tar.gz").is_err());
        assert!(resolve_review_file(&root, &entry, "/run.sh").is_err());

        let (root, entry) = runtime_review_file_root(page).expect("a page has a root");
        assert_eq!(
            root,
            Path::new("/company/outputs/redesign/2026-08-larder-sample")
        );
        assert_eq!(entry, "index.html");

        // The page's own stylesheet and images are part of the outcome.
        assert_eq!(
            resolve_review_file(&root, &entry, "/styles/site.css").unwrap(),
            root.join("styles/site.css")
        );
        assert_eq!(
            resolve_review_file(&root, &entry, "/photo.jpg?v=2").unwrap(),
            root.join("photo.jpg")
        );
        assert_eq!(
            resolve_review_file(&root, &entry, "/a%20space.png").unwrap(),
            root.join("a space.png")
        );
        // An empty path is the entry the accountable actor actually chose.
        assert_eq!(
            resolve_review_file(&root, &entry, "/").unwrap(),
            root.join("index.html")
        );

        // The page being served is company-authored, so this is exactly where
        // a traversal would be attempted.
        for escape in [
            "/../../../etc/passwd",
            "/..%2f..%2fetc%2fpasswd",
            "/subdir/../../secrets.html",
            "//etc/passwd",
            "/company/mandate.html",
        ] {
            if let Ok(path) = resolve_review_file(&root, &entry, escape) {
                assert!(
                    path.starts_with(&root),
                    "{escape:?} resolved to {path:?}, outside {root:?}"
                );
            }
        }
        // Office files stay within the selected outcome and are delivered as
        // downloads rather than framed as blank browser previews.
        assert_eq!(
            resolve_review_file(&root, &entry, "/notes.docx").unwrap(),
            root.join("notes.docx")
        );
        assert!(is_runtime_review_download(Path::new("notes.docx")));
    }

    #[test]
    fn review_byte_range_serves_one_slice_inside_the_file() {
        use super::review_byte_range as range;
        // What Safari asks of a recording before it plays, and as it seeks.
        assert_eq!(range("bytes=0-1", 1000), Some((0, 1)));
        assert_eq!(range("bytes=500-", 1000), Some((500, 999)));
        assert_eq!(range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(range("bytes=900-5000", 1000), Some((900, 999)));
        assert_eq!(range("bytes=-5000", 1000), Some((0, 999)));
        // Anything else is answered with the whole file, never a slice past its end.
        for refused in [
            "bytes=1000-",
            "bytes=5-2",
            "bytes=0-1,4-5",
            "items=0-1",
            "bytes=-0",
            "bytes=x-",
        ] {
            assert_eq!(range(refused, 1000), None, "{refused}");
        }
        assert_eq!(range("bytes=0-", 0), None);
    }

    #[test]
    fn review_media_types_cover_rendered_and_downloadable_formats() {
        assert_eq!(
            review_file_media_type(Path::new("a/b/index.HTML")),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(
            review_file_media_type(Path::new("plan.pdf")),
            Some("application/pdf")
        );
        assert_eq!(
            review_file_media_type(Path::new("clip.webm")),
            Some("video/webm")
        );
        assert_eq!(
            review_file_media_type(Path::new("report.docx")),
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
        );
        assert_eq!(
            review_file_media_type(Path::new("budget.xlsx")),
            Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        );
        assert_eq!(
            review_file_media_type(Path::new("slides.pptx")),
            Some("application/vnd.openxmlformats-officedocument.presentationml.presentation")
        );
        assert_eq!(review_file_media_type(Path::new("archive.zip")), None);
        assert_eq!(review_file_media_type(Path::new("Makefile")), None);
        assert!(is_runtime_review_download(Path::new("archive.zip")));
        assert!(!is_runtime_review_download(Path::new("plan.pdf")));
    }

    /// Every owner read of a company file resolves it through its symlinks first. A link Work
    /// left inside the company home that leads elsewhere on the computer is refused; a link
    /// between two places inside the home (a stable review alias) still reads.
    #[cfg(unix)]
    #[test]
    fn owner_reads_refuse_a_symlink_that_leaves_the_company_home() {
        use std::os::unix::fs::symlink;
        let base = std::env::temp_dir().join(format!("restless-canonical_test-{}", Uuid::new_v4()));
        let home = base.join("company");
        std::fs::create_dir_all(home.join("outputs/run")).unwrap();
        std::fs::write(home.join("outputs/run/plan.md"), "# Plan").unwrap();
        std::fs::write(base.join("secret.txt"), "outside").unwrap();
        symlink(
            home.join("outputs/run/plan.md"),
            home.join("outputs/alias.md"),
        )
        .unwrap();
        symlink(base.join("secret.txt"), home.join("outputs/escape.md")).unwrap();
        symlink(&base, home.join("outputs/up")).unwrap();
        let script = format!(
            "{}exec head -c 64 -- \"$p\"",
            CANONICAL_COMPANY_FILE.replace("/company", home.to_str().unwrap())
        );
        let read = |path: &str| {
            std::process::Command::new("sh")
                .args(["-c", &script, "sh", home.join(path).to_str().unwrap()])
                .output()
                .unwrap()
        };
        let ok = read("outputs/run/plan.md");
        assert!(ok.status.success());
        assert_eq!(ok.stdout, b"# Plan");
        assert_eq!(read("outputs/alias.md").stdout, b"# Plan");
        for escape in ["outputs/escape.md", "outputs/up/secret.txt"] {
            let refused = read(escape);
            assert!(!refused.status.success(), "{escape} was read");
            assert!(refused.stdout.is_empty(), "{escape} leaked bytes");
        }
        assert!(
            !read("outputs/run").status.success(),
            "a directory is not a file"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
