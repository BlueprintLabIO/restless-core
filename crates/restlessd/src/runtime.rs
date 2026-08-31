//! Runtime layer: the company computer's lifecycle, driven through the docker
//! CLI (mature infrastructure over bespoke machinery, §2.6). One persistent
//! container + one named volume per company; the volume is the company home.

use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::task::{Context as TaskContext, Poll};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use http_body_util::Empty;
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1;
use hyper::{HeaderMap, Method, Request, Response, Uri};
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

/// Resolve the company Runtime artifact the plane operates.
///
/// Local appliance development keeps the historical local tag. A hosted
/// plane supplies the exact manifest digest through `RESTLESS_COMPANY_IMAGE`;
/// the plane never resolves a release tag or builds Core source itself.
fn company_image() -> String {
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

/// A network-reachable account plane is a Cloud release consumer. Refuse to
/// start unless Fleet supplied the immutable Runtime artifact from its lock.
pub(crate) fn validate_company_image_config(network_mode: bool) -> Result<()> {
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
#[serde(rename_all = "snake_case")]
pub enum WorkerRuntime {
    /// Mature OMP/ACP transport retained as the compatible default.
    #[default]
    Omp,
    /// First-party Codex app-server transport for productive Staff Attempts.
    Codex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyConfig {
    /// Company name; also the container/volume suffix and schema name.
    pub name: String,
    /// Owner-set mission, seeded to /company/mission.md on `up`.
    #[serde(default)]
    pub mission: String,
    /// Per-company model spend ceiling in USD (T2). The fuse, not governance.
    #[serde(default = "default_ceiling")]
    pub spend_ceiling_usd: SpendCeiling,
    /// Provider-qualified model the agent runs on, e.g. `zai/glm-5.2`.
    /// Required: there is no sensible default provider, and the adapter-model
    /// indirection this replaced (`company-general-v1` → a gateway route)
    /// was vestigial once agents named providers directly.
    pub model: String,
    /// Cognitive transport for productive Staff Attempts. Exec and
    /// non-producing lead conversations remain on the mature coordination
    /// transport; this field changes the worker, not the organisation.
    #[serde(default)]
    pub worker_runtime: WorkerRuntime,
    /// Exact provider-supported reasoning effort for every actor launch.
    #[serde(default = "default_reasoning_effort")]
    pub reasoning_effort: String,
    /// Ordered provider-qualified fallbacks for the singleton Exec. Empty is
    /// an explicit no-fallback policy; providers are never inferred from
    /// ambient credentials or broker history.
    #[serde(default)]
    pub model_failover: Vec<String>,
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

impl CompanyConfig {
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
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("no company config at {}", path.display()))?;
        let config: Self =
            toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
        if config.name != name {
            bail!(
                "company config name mismatch: file {name}.toml says {}",
                config.name
            );
        }
        config.model_candidates()?;
        if !valid_reasoning_effort(&config.reasoning_effort) {
            bail!("unsupported reasoning effort {:?}", config.reasoning_effort);
        }
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
        validate_company_name(&config.name)?;
        config.model_candidates()?;
        if !valid_reasoning_effort(&config.reasoning_effort) {
            bail!("unsupported reasoning effort {:?}", config.reasoning_effort);
        }
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

    /// Primary followed by the exact owner-configured fallback order. The
    /// closed validation here protects both TOML and CLI writes.
    pub fn model_candidates(&self) -> Result<Vec<&str>> {
        let mut seen = std::collections::BTreeSet::new();
        let mut candidates = Vec::with_capacity(1 + self.model_failover.len());
        for model in std::iter::once(self.model.as_str())
            .chain(self.model_failover.iter().map(String::as_str))
        {
            let Some((provider, id)) = model.split_once('/') else {
                bail!("model {model:?} must be provider-qualified, e.g. moonshot/kimi-k3");
            };
            if provider.is_empty()
                || id.is_empty()
                || !provider
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            {
                bail!("invalid provider-qualified model {model:?}");
            }
            if !seen.insert(model) {
                bail!("duplicate model candidate {model:?}");
            }
            candidates.push(model);
        }
        Ok(candidates)
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

pub(crate) fn validate_company_name(name: &str) -> Result<()> {
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
        bail!("invalid company name {name:?}: use lowercase letters, digits or underscores, starting with a letter");
    }
    Ok(())
}

pub fn container_name(company: &str) -> String {
    format!("restless-co-{company}")
}

pub fn volume_name(company: &str) -> String {
    format!("restless-vol-{company}")
}

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

#[derive(Debug, Serialize)]
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
    pub reconciliation: ReconciliationStatus,
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coordination: Option<CoordinationDoctor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supervisor: Option<SupervisorDoctor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser: Option<BrowserDoctor>,
}

/// Observation of the Runtime's bounded, authenticated coordination path.
///
/// This deliberately performs an ordinary read through the Runtime Bridge
/// rather than inferring availability from a running container or its process
/// supervisor. It is a health observation, not another Runtime lifecycle.
#[derive(Debug, Serialize)]
pub struct CoordinationDoctor {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SupervisorDoctor {
    pub status: String,
    pub services: Vec<SupervisedService>,
}

#[derive(Debug, Serialize)]
pub struct SupervisedService {
    pub name: String,
    pub state: String,
}

#[derive(Debug, Serialize)]
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
        // A timed health probe must not leave an orphaned `docker exec`
        // process behind if the Runtime or coordinator has stopped answering.
        .kill_on_drop(true)
        .output()
        .await
        .context("spawn docker")
}

pub async fn status(company: &str) -> Result<ContainerStatus> {
    let name = container_name(company);
    let out = docker(&["inspect", "-f", "{{.State.Status}}", &name]).await?;
    if !out.status.success() {
        return Ok(ContainerStatus::Absent);
    }
    let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(match state.as_str() {
        "running" => ContainerStatus::Running,
        _ => ContainerStatus::Stopped,
    })
}

/// Create if absent, start if stopped, no-op if running. With `reconcile`,
/// fetch the configured company image and replace an outdated container while
/// keeping its named volume. Building and publishing that image belongs to the
/// release/Fleet path, not to the credential-holding account plane.
pub async fn up(config: &CompanyConfig, reconcile: bool) -> Result<String> {
    let company = &config.name;
    // A company the account plane could not admit a model route for cannot
    // think, so waking its Runtime would only defer the failure into its first
    // Attempt. Refuse here with the exact reason (cross-layer contract §1.4.1).
    if let Some(reason) = crate::model_gateway::unstartable_reason(company) {
        bail!("company {company} cannot start: {reason}");
    }
    let image = company_image();
    let mut fetched = false;
    let mut replaced = false;
    if reconcile {
        fetched = ensure_image_available(&image).await?;
        if status(company).await? != ContainerStatus::Absent
            && (container_uses_old_image(company).await?
                || !container_uses_company_volume(company).await?)
        {
            let name = container_name(company);
            // Give supervised Chromium long enough to flush its persistent
            // profile before replacing the shell. Docker's ten-second default
            // is shorter than Chromium's configured 20-second stop window and
            // was observed dropping a persistent cookie during reconciliation.
            if status(company).await? == ContainerStatus::Running {
                run_ok(&["stop", "--time", "30", &name]).await?;
            }
            // Only the replaceable container is removed; the named company
            // volume remains the durable computer (§13.4).
            run_ok(&["rm", &name]).await?;
            replaced = true;
        }
    }
    match status(company).await? {
        ContainerStatus::Running => {}
        ContainerStatus::Stopped => {
            let name = container_name(company);
            run_ok(&["start", &name]).await?;
        }
        ContainerStatus::Absent => {
            let volume = volume_name(company);
            run_ok(&["volume", "create", &volume]).await?;
            let name = container_name(company);
            let cpus = resource_bound("RESTLESS_COMPANY_CPUS", DEFAULT_CPUS);
            let memory = resource_bound("RESTLESS_COMPANY_MEMORY", DEFAULT_MEMORY);
            let pids = resource_bound("RESTLESS_COMPANY_PIDS_LIMIT", DEFAULT_PIDS_LIMIT);
            let mut args: Vec<&str> = vec!["run", "-d", "--name", &name, "--hostname", company];
            if let Some(cpus) = cpus.as_deref() {
                args.extend(["--cpus", cpus]);
            }
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
            // The image default names the appliance's established port, but an
            // isolated account plane may use RESTLESS_PORT_OFFSET.  ACP turns
            // already override this value; the persistent Runtime must receive
            // the same endpoint so ordinary bridge tools and doctor do not
            // silently talk to another plane (or fail while agent turns work).
            let coordinator_env = format!("RESTLESS_COORDINATOR={}", crate::runtime_coordinator()?);
            let volume_mount = format!("{volume}:/company");
            args.extend([
                "-e",
                &company_env,
                "-e",
                &coordinator_env,
                "-v",
                &volume_mount,
                &image,
            ]);
            run_ok(&args).await?;
        }
    }
    seed_mission(config).await?;
    let suffix = match (fetched, replaced) {
        (true, true) => " (image fetched; container replaced; volume kept)",
        (true, false) => " (image fetched)",
        (false, true) => " (container replaced; volume kept)",
        (false, false) if reconcile => " (runtime already current)",
        (false, false) => "",
    };
    Ok(format!("{}: running{suffix}", config.name))
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

/// Check the replaceable runtime image independently of an agent report.
pub async fn doctor(company: &str) -> Result<RuntimeDoctor> {
    let image = company_image();
    let container = status(company).await?;
    let volume = volume_name(company);
    let volume_exists = docker(&["volume", "inspect", &volume])
        .await?
        .status
        .success();
    let volume_mounted =
        container != ContainerStatus::Absent && container_uses_company_volume(company).await?;
    let container_image_id = if container == ContainerStatus::Absent {
        None
    } else {
        inspect_value(&["inspect", "-f", "{{.Image}}", &container_name(company)]).await?
    };
    let target_image_id = inspect_value(&["image", "inspect", "-f", "{{.Id}}", &image]).await?;
    let image_source_digest = inspect_value(&[
        "image",
        "inspect",
        "-f",
        &format!("{{{{index .Config.Labels \"{SOURCE_DIGEST_LABEL}\"}}}}"),
        &image,
    ])
    .await?
    .filter(|value| value != "<no value>");
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
        || matches!(
            (&container_image_id, &target_image_id),
            (Some(container_id), Some(target_id)) if container_id != target_id
        )
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

    let (coordination, supervisor, browser) = if container == ContainerStatus::Running {
        (
            Some(coordination_doctor(company).await),
            Some(supervisor_doctor(company).await),
            Some(browser_doctor(company).await),
        )
    } else {
        (None, None, None)
    };
    let coordination_requires_reconcile = container == ContainerStatus::Running
        && coordination
            .as_ref()
            .is_none_or(|value| value.status != "available");

    Ok(RuntimeDoctor {
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
        reconciliation,
        action: (reconciliation != ReconciliationStatus::Current
            || coordination_requires_reconcile)
            .then(|| format!("restless up -c {company} --reconcile")),
        coordination,
        supervisor,
        browser,
    })
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
        docker(&["exec", "-u", "company", &name, "restless", "status"]),
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
                "The Runtime could not complete an authenticated coordination status request."
                    .into(),
            ),
        },
        Ok(Err(_)) | Err(_) => CoordinationDoctor {
            status: "degraded".into(),
            detail: Some(
                "The Runtime coordination path could not be observed within five seconds.".into(),
            ),
        },
    }
}

async fn browser_doctor(company: &str) -> BrowserDoctor {
    let name = container_name(company);
    let process = async |program: &str| -> String {
        let output = docker(&[
            "exec",
            &name,
            "supervisorctl",
            "-c",
            "/etc/supervisor/conf.d/restless.conf",
            "status",
            program,
        ])
        .await;
        match output {
            Ok(output) if output.status.success() => {
                let line = String::from_utf8_lossy(&output.stdout);
                if line.contains("RUNNING") {
                    "available".into()
                } else {
                    "degraded".into()
                }
            }
            _ => "unavailable".into(),
        }
    };
    let desktop = process("desktop").await;
    let chromium = process("chromium").await;
    let web_transport = process("desktop-web").await;
    let automation: String = match docker(&[
        "exec",
        &name,
        "curl",
        // Preserve the broker's structured 423 body. `--fail` discarded it,
        // so an intentional owner pause was misreported as a broken browser.
        "--fail-with-body",
        "--silent",
        "--max-time",
        "2",
        "http://127.0.0.1:9223/json/version",
    ])
    .await
    {
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
    let status = if [&desktop, &chromium, &web_transport]
        .iter()
        .all(|part| part.as_str() == "available")
        && matches!(automation.as_str(), "available" | "owner_paused")
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
    let output = docker(&[
        "exec",
        &name,
        "supervisorctl",
        "-c",
        "/etc/supervisor/conf.d/restless.conf",
        "status",
    ])
    .await;
    let services = match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                Some(SupervisedService {
                    name: parts.next()?.to_string(),
                    state: parts.next()?.to_lowercase(),
                })
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let status =
        if !services.is_empty() && services.iter().all(|service| service.state == "running") {
            "available"
        } else {
            "degraded"
        };
    SupervisorDoctor {
        status: status.to_string(),
        services,
    }
}

/// Fetch one noVNC asset through an ephemeral Runtime Bridge process. The
/// desktop service remains bound inside the company computer; there is no host
/// port that can bypass owner authentication.
pub async fn desktop_asset(company: &str, asset: &str) -> Result<Vec<u8>> {
    if asset.is_empty() || asset.contains("..") || asset.contains('\0') {
        bail!("invalid desktop asset path");
    }
    let url = format!("http://127.0.0.1:6080/{asset}");
    let output = docker(&[
        "exec",
        &container_name(company),
        "curl",
        "--fail",
        "--silent",
        "--show-error",
        "--max-time",
        "5",
        &url,
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
        Ok(())
    } else {
        bail!("runtime review target returned {}", response.status())
    }
}

const MAX_REVIEW_TEXT_BYTES: usize = 256 * 1024;

fn runtime_review_text_path(value: &str) -> Result<&Path> {
    let path = Path::new(value);
    let mut components = path.components();
    if components.next() != Some(Component::RootDir)
        || components.next() != Some(Component::Normal("company".as_ref()))
        || components.clone().next().is_none()
        || components.any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("text ReviewTarget must be a file beneath /company");
    }
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

/// What a browser will actually render, mapped to the exact type to send. An
/// extension that is not here is not refused because it is dangerous — it is
/// refused because presenting it would be a blank frame, and a blank frame that
/// claims to be the outcome is worse than saying plainly what the file is.
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

/// The exact file a displayable ReviewTarget names, beneath `/company`.
fn runtime_review_file_path(value: &str) -> Result<&Path> {
    let path = Path::new(value);
    let mut components = path.components();
    if components.next() != Some(Component::RootDir)
        || components.next() != Some(Component::Normal("company".as_ref()))
        || components.clone().next().is_none()
        || components.any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("file ReviewTarget must be a file beneath /company");
    }
    if review_file_media_type(path).is_none() {
        bail!("file ReviewTarget is not a format the cockpit can display");
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
    validate_company_name(company)?;
    let path = runtime_review_file_path(value)?;
    let container = container_name(company);
    let output = docker(&[
        "exec",
        "-u",
        "company",
        &container,
        "stat",
        "-c",
        "%s",
        "--",
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
    if size > MAX_REVIEW_FILE_BYTES {
        bail!("file ReviewTarget is larger than the {MAX_REVIEW_FILE_BYTES}-byte review limit");
    }
    Ok(())
}

/// Resolve one requested path against a file review's confined root.
///
/// The request comes from a page the company itself authored, so this is the
/// exact place a traversal would be attempted. Only ordinary named components
/// survive: no `..`, no absolute re-root, no symlink chase, and the resolved
/// path must still be a displayable format.
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
    if !resolved.starts_with(root) || review_file_media_type(&resolved).is_none() {
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
    let media_type =
        review_file_media_type(path).context("review file is not a displayable format")?;
    let container = container_name(company);
    let limit = (MAX_REVIEW_FILE_BYTES + 1).to_string();
    let output = docker(&[
        "exec",
        "-u",
        "company",
        &container,
        "head",
        "-c",
        &limit,
        "--",
        path.to_str().context("review file path must be UTF-8")?,
    ])
    .await?;
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
    let container = container_name(company);
    let limit = (MAX_REVIEW_TEXT_BYTES + 1).to_string();
    let output = docker(&[
        "exec",
        "-u",
        "company",
        &container,
        "head",
        "-c",
        &limit,
        "--",
        path.to_str().context("ReviewTarget path must be UTF-8")?,
    ])
    .await?;
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
    private_tcp_stream(company, 6080).await
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
    let output = docker(&[
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

/// An owner lease is bounded even if the SPA vanishes without hand-back.
/// The Runtime file is reconstructable coordination, not durable truth, so a
/// reader projects an expired owner back to the requesting agent (or
/// unclaimed rescue state). The browser broker independently uses the same
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

    let requester = state["requester"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    state = match requester {
        Some(session_id) => serde_json::json!({
            "controller": "agent",
            "session_id": session_id,
            "reason": "owner_lease_expired",
            "expired_at": expires_at,
        }),
        None => serde_json::json!({
            "controller": "unclaimed",
            "reason": "owner_lease_expired",
            "expired_at": expires_at,
        }),
    };
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
    Ok(())
}

/// Ensure the release artifact selected by Fleet is present. This function is
/// intentionally incapable of building from a source checkout: doing that in
/// the plane would make the deployed Runtime differ from the pinned manifest.
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

async fn inspect_value(args: &[&str]) -> Result<Option<String>> {
    let output = docker(args).await?;
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
pub(crate) fn source_root() -> Result<PathBuf> {
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
    files.sort();
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
/// survives (§5, §17 step 2: the persistent company computer).
pub async fn down(company: &str) -> Result<String> {
    match status(company).await? {
        ContainerStatus::Running => {
            let name = container_name(company);
            // Chromium's supervisor stop window is 20 seconds. Use a longer
            // container deadline so cookies and profile state reach disk
            // before Docker escalates to SIGKILL.
            run_ok(&["stop", "--time", "30", &name]).await?;
            Ok(format!("{company}: stopped (volume kept)"))
        }
        ContainerStatus::Stopped => Ok(format!("{company}: already stopped")),
        ContainerStatus::Absent => Ok(format!("{company}: no container")),
    }
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
    company: &str,
    org: &restless_orgintel::OrgIntel,
    spend: &crate::spend::SpendLedger,
) -> Result<String> {
    let mut removed = Vec::new();

    if status(company).await? != ContainerStatus::Absent {
        let name = container_name(company);
        // `rm -f` covers running and stopped in one call; stopping first would
        // leave a window where a crash strands the container.
        run_ok(&["rm", "-f", &name]).await?;
        removed.push("container");
    }

    let volume = volume_name(company);
    if docker(&["volume", "inspect", &volume])
        .await?
        .status
        .success()
    {
        run_ok(&["volume", "rm", &volume]).await?;
        removed.push("volume");
    }

    // `drop_schema` has existed unused since sprint 01 and was nearly deleted
    // in the sprint-02 purge. Ephemeral companies are why it exists.
    org.drop_schema().await.context("drop orgintel schema")?;
    removed.push("schema");

    // The spool is ONE shared file, not one per company (`spend.jsonl`), so
    // this cannot be a file deletion — an earlier version of this function
    // removed `spend/<company>.jsonl`, a path that has never existed, and so
    // silently left every destroyed company's spend accounted. That is the
    // sprint-02 defect verbatim.
    spend
        .forget(company)
        .context("clearing the destroyed company's spend")?;
    removed.push("spend");

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
        name: to.to_string(),
        // A scenario gets no live secret bindings.
        credentials: std::collections::BTreeMap::new(),
        model_failover: Vec::new(),
        // Standing approvals are the owner's blessing of a *live* company's
        // counterparties. They do not travel to a throwaway.
        approved_parties: Vec::new(),
        ..source
    };
    CompanyConfig::save(root, &config)?;
    Ok(config)
}

async fn seed_mission(config: &CompanyConfig) -> Result<()> {
    let name = container_name(&config.name);
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            &name,
            "sh",
            "-c",
            "cat > /company/mission.md && chown company:company /company/mission.md",
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
pub(crate) async fn sync_mission_projection(config: &CompanyConfig) -> Result<&'static str> {
    match status(&config.name).await? {
        ContainerStatus::Running => {
            seed_mission(config).await?;
            Ok("updated")
        }
        ContainerStatus::Stopped | ContainerStatus::Absent => Ok("deferred"),
    }
}

/// Put an owner-supplied attachment on the persistent company computer as an
/// ordinary file. The UUID is generated by the daemon; user filenames never
/// become path components. Metadata is a sidecar so the authenticated owner
/// download can return the original name and media type without inventing an
/// asset-custody service.
pub async fn store_owner_attachment(
    company: &str,
    attachment_id: Uuid,
    bytes: &[u8],
    metadata: &[u8],
) -> Result<String> {
    validate_company_name(company)?;
    if status(company).await? != ContainerStatus::Running {
        bail!("the company computer is not running; attachments need its persistent filesystem");
    }
    let name = container_name(company);
    let directory = format!("/company/inbox/owner-attachments/{attachment_id}");
    let content_path = format!("{directory}/content");
    let metadata_path = format!("{directory}/metadata.json");
    write_container_file(&name, &directory, &content_path, bytes).await?;
    if let Err(error) = write_container_file(&name, &directory, &metadata_path, metadata).await {
        let _ = remove_owner_attachment(company, attachment_id).await;
        return Err(error);
    }
    Ok(content_path)
}

async fn write_container_file(
    container: &str,
    directory: &str,
    path: &str,
    bytes: &[u8],
) -> Result<()> {
    let command = format!(
        "umask 077; mkdir -p {directory}; cat > {path}; chown -R company:company {directory}"
    );
    let mut child = tokio::process::Command::new("docker")
        .args(["exec", "-i", container, "sh", "-c", &command])
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

/// Read one attachment and its small sidecar. Only a parsed UUID reaches the
/// fixed path template, so this cannot become an arbitrary company-file read.
pub async fn read_owner_attachment(
    company: &str,
    attachment_id: Uuid,
) -> Result<(Vec<u8>, Vec<u8>)> {
    validate_company_name(company)?;
    let name = container_name(company);
    let directory = format!("/company/inbox/owner-attachments/{attachment_id}");
    let content = docker(&["exec", &name, "cat", &format!("{directory}/content")]).await?;
    if !content.status.success() {
        bail!("attachment is absent from the company computer");
    }
    let metadata = docker(&["exec", &name, "cat", &format!("{directory}/metadata.json")]).await?;
    if !metadata.status.success() {
        bail!("attachment metadata is absent from the company computer");
    }
    Ok((content.stdout, metadata.stdout))
}

/// Roll back files whose message could not be recorded. The target is one
/// daemon-generated UUID directory beneath the fixed attachment root.
pub async fn remove_owner_attachment(company: &str, attachment_id: Uuid) -> Result<()> {
    validate_company_name(company)?;
    let name = container_name(company);
    let path = format!("/company/inbox/owner-attachments/{attachment_id}");
    let output = docker(&["exec", &name, "rm", "-r", "--", &path]).await?;
    if !output.status.success() {
        bail!("roll back owner attachment failed");
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
pub fn state_root() -> PathBuf {
    if let Ok(root) = std::env::var("RESTLESS_HOME") {
        return PathBuf::from(root);
    }
    let home = std::env::var("HOME").expect("HOME");
    PathBuf::from(home).join(".restless")
}

#[cfg(test)]
mod tests {
    use super::{
        is_immutable_image_digest, resolve_company_image, resolve_resource_bound, COMPANY_IMAGE,
        DEFAULT_CPUS, DEFAULT_MEMORY, DEFAULT_PIDS_LIMIT,
    };

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
        runtime_review_text_path, CompanyConfig, SpendCeiling, WorkerRuntime,
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
    fn an_expired_owner_lease_returns_to_its_requester() {
        let state = serde_json::json!({
            "controller": "owner",
            "client_id": "owner-tab",
            "requester": "exec/session-7",
            "expires_at": "2000-01-01T00:00:00Z",
        });
        let normalized = normalize_expired_browser_control(state);
        assert_eq!(normalized["controller"], "agent");
        assert_eq!(normalized["session_id"], "exec/session-7");
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
    fn worker_transport_and_reasoning_are_explicit_with_compatible_defaults() {
        let legacy: CompanyConfig = toml::from_str(
            r#"name = "legacy_test"
model = "moonshot/kimi-k3"
"#,
        )
        .unwrap();
        assert_eq!(legacy.worker_runtime, WorkerRuntime::Omp);
        assert_eq!(legacy.reasoning_effort, "medium");

        let codex: CompanyConfig = toml::from_str(
            r#"name = "codex_test"
model = "litellm/gpt-5.6-sol"
worker_runtime = "codex"
reasoning_effort = "high"
"#,
        )
        .unwrap();
        assert_eq!(codex.worker_runtime, WorkerRuntime::Codex);
        assert_eq!(codex.reasoning_effort, "high");
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
            name: "archive_contract_test".into(),
            mission: "Preserve me".into(),
            spend_ceiling_usd: SpendCeiling::from_micro_usd(5_000_000),
            model: "moonshot/kimi-k3".into(),
            worker_runtime: crate::runtime::WorkerRuntime::Omp,
            reasoning_effort: crate::acp::DEFAULT_REASONING_EFFORT.into(),
            model_failover: Vec::new(),
            credentials: std::collections::BTreeMap::new(),
            approved_parties: Vec::new(),
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

    /// A produced file the cockpit can display, and the exact boundary of what
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

        // Markdown keeps its own richer path: the cockpit renders it rather
        // than framing it, so this must not claim it.
        assert!(!is_runtime_review_file_target("/company/outputs/plan.md"));
        // Nothing outside the company computer, and nothing the cockpit would
        // present as a blank frame.
        assert!(!is_runtime_review_file_target("/etc/passwd"));
        assert!(!is_runtime_review_file_target(
            "/company/../etc/shadow.html"
        ));
        assert!(!is_runtime_review_file_target(
            "/company/outputs/build.tar.gz"
        ));
        assert!(!is_runtime_review_file_target("/company/outputs/run.sh"));

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
        // A file inside the outcome that the cockpit cannot display is refused
        // rather than framed blank.
        assert!(resolve_review_file(&root, &entry, "/notes.docx").is_err());
    }

    #[test]
    fn review_media_types_cover_only_what_a_browser_shows() {
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
        assert_eq!(review_file_media_type(Path::new("archive.zip")), None);
        assert_eq!(review_file_media_type(Path::new("Makefile")), None);
    }
}
