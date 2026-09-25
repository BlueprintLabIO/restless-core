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
const COMPANY_SUPERVISOR_CONFIG: &str = "/etc/supervisor/conf.d/company.conf";

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
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::RestlessManaged => "restless-managed",
            Self::Codex => "codex",
            Self::ClaudeAgent => "claude-agent",
            Self::CustomAcp => "custom-acp",
        }
    }

    pub(crate) fn parse_canonical(value: &str) -> Option<Self> {
        match value.trim() {
            "restless-managed" => Some(Self::RestlessManaged),
            "codex" => Some(Self::Codex),
            "claude-agent" => Some(Self::ClaudeAgent),
            _ => None,
        }
    }

    pub(crate) const fn build(self) -> &'static str {
        match self {
            Self::RestlessManaged => "omp-18.0.10",
            Self::Codex => "codex-cli-0.155.1",
            Self::ClaudeAgent => "claude-agent-acp-0.73.0",
            Self::CustomAcp => "custom-acp-v1",
        }
    }

    pub(crate) const fn native_agent_build(self) -> Option<&'static str> {
        match self {
            Self::ClaudeAgent => Some("claude-code-2.1.257"),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyConfig {
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub agent_intelligence: std::collections::BTreeMap<String, AgentIntelligence>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub native_harnesses: std::collections::BTreeMap<String, NativeHarnessConfig>,
    /// Owner-facing name; the durable company handle remains unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
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
    /// Optional monthly running-time cap. It gates new Runtime starts and does
    /// not stop work that is already running. This is a capacity policy, not a
    /// representation of the provider's invoice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthly_runtime_cap_hours: Option<u32>,
    /// Opt-in sleep after this many idle minutes. `None` preserves the
    /// persistent Runtime's existing always-on behaviour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_sleep_after_minutes: Option<u16>,
    /// Standing owner promise for newly commissioned outcomes. This selects
    /// ambition, not authority, topology, model, or a spend allocation.
    #[serde(default)]
    pub outcome_standard: restless_orgintel::OutcomeStandard,
    /// Provider-qualified model the agent runs on, e.g. `zai/glm-5.2`.
    /// A new company starts empty until the owner explicitly selects an
    /// intelligence connection. Empty never means "infer from host state".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    /// Certified harness used by Exec and non-producing lead conversations.
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

const LEGACY_UNCONFIGURED_MODEL: &str = "unconfigured/pending";

/// Native harness IDs are internal route markers, not direct provider IDs.
/// Keep rejecting them at model write boundaries while leaving old config
/// readable so an owner can repair it through the Intelligence provider UI.
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
        bail!("Native harness models must be selected through Company → Intelligence provider.");
    }
    Ok(())
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
        if let Some(route) = self
            .agent_intelligence
            .get(actor)
            .or_else(|| self.agent_intelligence.get("default"))
        {
            config.model_failover.clear();
            if let Some(provider) = route.connection.strip_prefix("direct:") {
                config.model = format!("{provider}/{}", route.model);
                config.coordination_harness = AgentHarness::RestlessManaged;
                config.worker_harness = AgentHarness::RestlessManaged;
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
        }
        config
    }
    pub fn agent_preference<'a>(
        &'a self,
        actor: &str,
        previous: Option<&'a str>,
    ) -> Option<&'a str> {
        if self.agent_intelligence.contains_key(actor)
            || self.agent_intelligence.contains_key("default")
        {
            Some(&self.model)
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

    fn validate_resource_policies(&self) -> Result<()> {
        if self
            .monthly_runtime_cap_hours
            .is_some_and(|hours| !(1..=744).contains(&hours))
        {
            bail!("monthly_runtime_cap_hours must be between 1 and 744 hours");
        }
        if self
            .auto_sleep_after_minutes
            .is_some_and(|minutes| !(1..=1440).contains(&minutes))
        {
            bail!("auto_sleep_after_minutes must be between 1 and 1440 minutes");
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

    pub(crate) fn validate_harness_models(&self) -> Result<()> {
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
                AgentHarness::Codex => "litellm",
                AgentHarness::ClaudeAgent => "anthropic",
            };
            if let Some(model) = models
                .iter()
                .find(|model| !model.starts_with(&format!("{required_provider}/")))
            {
                bail!(
                    "{} harness requires every configured model to use provider {required_provider}; got {model}",
                    harness.as_str()
                );
            }
            if let Some(model) = models.iter().find(|model| match harness {
                AgentHarness::Codex => {
                    !crate::model_gateway::responses_model_has_pinned_tariff(model)
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
        bail!(
            "invalid company name {name:?}: use lowercase letters, digits or underscores, starting with a letter"
        );
    }
    Ok(())
}

pub fn container_name(company: &str) -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => format!("restless-{namespace}-co-{company}"),
        _ => format!("restless-co-{company}"),
    }
}

pub fn volume_name(company: &str) -> String {
    match std::env::var("RESTLESS_RESOURCE_NAMESPACE") {
        Ok(namespace) if !namespace.is_empty() => format!("restless-{namespace}-vol-{company}"),
        _ => format!("restless-vol-{company}"),
    }
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
        // A timed health probe must not leave an orphaned `docker exec`
        // process behind if the Runtime or coordinator has stopped answering.
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
pub(crate) async fn docker_observe(args: &[&str]) -> Result<std::process::Output> {
    if STARTUP_RECOVERY_ACTIVE.load(Ordering::SeqCst) {
        bail!("Runtime observation is deferred while startup recovery owns Docker");
    }
    docker_bounded(args, DOCKER_OBSERVATION_TIMEOUT)
        .await
        .context("bounded Docker observation")
}

pub(crate) async fn docker_bounded(args: &[&str], bound: Duration) -> Result<std::process::Output> {
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
    let mut companies = configs.iter().map(|config| config.name.clone()).collect::<Vec<_>>();
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
pub async fn up(config: &CompanyConfig, reconcile: bool) -> Result<String> {
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
    if reconcile {
        fetched = ensure_image_available(&image).await?;
        if status(company).await? != ContainerStatus::Absent
            && (container_uses_old_image(company).await?
                || !container_uses_company_volume(company).await?)
        {
            // Reconciliation replaces the shell. A configured monthly hours
            // cap must gate that replacement before it stops live work.
            crate::runtime_usage::ensure_start_allowed(&state_root(), config).await?;
            let name = container_name(company);
            // Give supervised Chromium long enough to flush its persistent
            // profile before replacing the shell. Docker's ten-second default
            // is shorter than Chromium's configured 20-second stop window and
            // was observed dropping a persistent cookie during reconciliation.
            if status(company).await? == ContainerStatus::Running {
                run_ok(&["stop", "--time", "30", &name]).await?;
                // Record the exact FinishedAt before replacing the container,
                // so the next generation is not mistaken for a missed run.
                if config.monthly_runtime_cap_hours.is_some() {
                    crate::runtime_usage::observe(&state_root(), config).await?;
                }
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
        }
        ContainerStatus::Stopped => {
            if !replaced {
                crate::runtime_usage::ensure_start_allowed(&state_root(), config).await?;
            }
            let name = container_name(company);
            if let Some(network) = internal_network.as_deref() {
                ensure_container_on_network(company, network).await?;
            }
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
            let cpus = resource_bound("RESTLESS_COMPANY_CPUS", DEFAULT_CPUS);
            let memory = resource_bound("RESTLESS_COMPANY_MEMORY", DEFAULT_MEMORY);
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
    seed_mission(config).await?;
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

fn company_network_name(company: &str) -> String {
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
    let mut cached = slot.lock().await;
    if let Some((at, result)) = cached.as_ref() {
        if at.elapsed() < Duration::from_secs(15) {
            return Ok(result.clone());
        }
    }
    let result = (doctor(company).await?, Utc::now());
    *cached = Some((Instant::now(), result.clone()));
    Ok(result)
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
    if container == ContainerStatus::Running {
        let (document, room) = tokio::join!(
            collaboration_tool_doctor(company, "document"),
            collaboration_tool_doctor(company, "room"),
        );
        collaboration_tools.extend([document, room]);
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
        reconciliation,
        action: (reconciliation != ReconciliationStatus::Current
            || coordination_requires_reconcile)
            .then(|| format!("restless up -c {company} --reconcile")),
        release,
        coordination,
        supervisor,
        browser,
    })
}

async fn collaboration_tool_doctor(company: &str, tool: &str) -> CollaborationToolDoctor {
    let probe = tokio::time::timeout(
        Duration::from_secs(5),
        docker_observe(&[
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
        docker_observe(&[
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
        docker_observe(&["exec", "-u", "company", &name, "restless", "status"]),
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
    let chromium = process("chromium");
    let web_transport = process("desktop-web");
    let automation: String = match docker_observe(&[
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
    let output = docker_observe(&[
        "exec",
        &name,
        "supervisorctl",
        "-c",
        COMPANY_SUPERVISOR_CONFIG,
        "status",
    ])
    .await;
    let services = match output {
        // supervisorctl exits nonzero when any optional program is stopped,
        // but its stdout still contains the status of every other program.
        Ok(output) => String::from_utf8_lossy(&output.stdout)
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
    let output = docker_observe(&[
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

    let container = container_name(company);
    // A leading `=` tells curl to encode the entire value as an unnamed query
    // field. Without it, the first `=` inside an OAuth-style destination
    // query is misread as curl's own name/value separator.
    let destination = format!("={}", parsed.as_str());
    let opened = docker_observe(&[
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
    let activated = docker_observe(&[
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
    let output = docker_observe(&[
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
pub fn is_runtime_review_download(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx"
            )
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
        bail!("file ReviewTarget is not a format the cockpit can present");
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
    let output = docker_observe(&[
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
    let output = docker_observe(&[
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
    let output = docker_observe(&[
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
    let output = docker_observe(&[
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
    if !output.status.success() || output.stdout.is_empty() { return Ok(None); }
    let session: BrowserAgentSession = serde_json::from_slice(&output.stdout)
        .context("parse authenticated browser registration")?;
    if session.expires_at <= Utc::now() { return Ok(None); }
    Ok(Some(session))
}

pub async fn register_browser_agent_session(
    company: &str,
    grant: &crate::capability::CoordinationGrant,
) -> Result<String> {
    validate_company_name(company)?;
    if grant.company != company { bail!("browser registration company does not match the signed ActorSession"); }
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
        if current.session == grant.session && current.actor == grant.actor
            && current.work_id == grant.work_id && current.attempt_id == grant.attempt_id {
            return Ok(current.websocket_url);
        }
        bail!("the shared company browser is attached to another live Work Attempt");
    }
    // Only Core reads Chrome's private endpoint. The opaque ticket is minted
    // after Core verified the signed capability and is stored atomically for
    // the broker; the issuer key never enters the company container.
    let output = docker_observe(&[
        "exec", &container_name(company), "curl", "--fail", "--silent", "--show-error",
        "--max-time", "5", "http://127.0.0.1:9222/json/version",
    ]).await?;
    if !output.status.success() { bail!("could not inspect the company browser endpoint"); }
    let discovery: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("decode the company browser endpoint")?;
    let path = discovery["webSocketDebuggerUrl"].as_str()
        .and_then(|value| url::Url::parse(value).ok())
        .map(|value| value.path().to_string())
        .filter(|value| value.starts_with("/devtools/browser/"))
        .context("company browser did not return a browser-level CDP endpoint")?;
    let ticket = Uuid::new_v4().simple().to_string();
    let websocket_url = format!("ws://127.0.0.1:9223/session/{ticket}{path}");
    let registration = BrowserAgentSession {
        ticket, company: company.to_string(), actor: grant.actor.clone(),
        session: grant.session.clone(), work_id: grant.work_id, attempt_id: grant.attempt_id,
        websocket_url: websocket_url.clone(), expires_at: Utc::now() + chrono::Duration::minutes(45),
    };
    write_browser_agent_session(company, &registration).await?;
    Ok(websocket_url)
}

pub async fn release_browser_agent_session(
    company: &str,
    grant: &crate::capability::CoordinationGrant,
) -> Result<()> {
    let Some(current) = read_browser_agent_session(company).await? else { return Ok(()); };
    if current.company != grant.company || current.actor != grant.actor
        || current.session != grant.session || current.work_id != grant.work_id
        || current.attempt_id != grant.attempt_id {
        bail!("browser registration does not belong to this signed ActorSession");
    }
    clear_browser_agent_session(company, &current.ticket).await
}

pub async fn clear_browser_agent_session(company: &str, ticket: &str) -> Result<()> {
    validate_company_name(company)?;
    let _control_guard = browser_control_guard(company).await;
    let Some(current) = read_browser_agent_session(company).await? else { return Ok(()); };
    if current.ticket != ticket { bail!("browser registration changed before it could be cleared"); }
    let output = tokio::process::Command::new("docker").args([
        "exec", &container_name(company), "rm", "-f", "/company/run/browser-agent-session.json",
    ]).output().await.context("release company browser registration")?;
    if !output.status.success() { bail!("could not release company browser registration"); }
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
    if !output.status.success() { bail!("could not write authenticated company browser registration"); }
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
/// survives (§5, §17 step 2: the persistent company computer).
pub async fn down(company: &str) -> Result<String> {
    let _start = company_start_guard(company).await;
    down_locked(company).await
}

/// Stop a Runtime only if an async activity check still considers it idle.
/// The predicate runs under the same per-company lifecycle lock as `up`, so a
/// concurrent explicit start cannot be stopped after it has completed.
pub async fn down_if_idle<F, Fut>(company: &str, is_idle: F) -> Result<bool>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<bool>>,
{
    let _start = company_start_guard(company).await;
    if status(company).await? != ContainerStatus::Running || !is_idle().await? {
        return Ok(false);
    }
    down_running_locked(company).await?;
    invalidate_cockpit_health(company).await;
    Ok(true)
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
    // Docker escalates to SIGKILL.
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
    crate::local_documents::remove(root, org)
        .await
        .context("remove local Documents service before dropping its cell")?;

    if status(company).await? != ContainerStatus::Absent {
        let name = container_name(company);
        // `rm -f` covers running and stopped in one call; stopping first would
        // leave a window where a crash strands the container.
        run_ok(&["rm", "-f", &name]).await?;
        removed.push("container");
    }

    let volume = volume_name(company);
    if docker_observe(&["volume", "inspect", &volume])
        .await?
        .status
        .success()
    {
        run_ok(&["volume", "rm", &volume]).await?;
        removed.push("volume");
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
        name: to.to_string(),
        internal_network: false,
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
        bound_docker_call, company_statuses, container_name, docker_observe,
        is_immutable_image_digest, is_runtime_review_download, read_owner_attachment,
        remove_owner_attachment, resolve_company_image, resolve_resource_bound,
        running_company_names, sort_source_files, store_owner_attachment, AgentIntelligence,
        ContainerStatus, NativeHarnessConfig, COMPANY_IMAGE, DEFAULT_CPUS, DEFAULT_MEMORY,
        DEFAULT_PIDS_LIMIT,
    };

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
        runtime_review_text_path, AgentHarness, CompanyConfig, SpendCeiling,
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
    fn legacy_company_inherits_exceptional_and_explicit_standard_round_trips() {
        let legacy: CompanyConfig = toml::from_str(
            r#"name = "standard_test"
mission = "test"
model = "moonshot/kimi-k3"
"#,
        )
        .unwrap();
        assert_eq!(
            legacy.outcome_standard,
            restless_orgintel::OutcomeStandard::Exceptional
        );

        let explicit: CompanyConfig = toml::from_str(
            r#"name = "standard_test"
mission = "test"
model = "moonshot/kimi-k3"
outcome_standard = "frontier"
"#,
        )
        .unwrap();
        assert_eq!(
            explicit.outcome_standard,
            restless_orgintel::OutcomeStandard::Frontier
        );
        assert!(toml::to_string(&explicit)
            .unwrap()
            .contains("outcome_standard = \"frontier\""));
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
        assert!(inherited.model_failover.is_empty());
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
            name: "archive_contract_test".into(),
            internal_network: false,
            mission: "Preserve me".into(),
            spend_ceiling_usd: SpendCeiling::from_micro_usd(5_000_000),
            monthly_runtime_cap_hours: None,
            auto_sleep_after_minutes: None,
            outcome_standard: Default::default(),
            model: "moonshot/kimi-k3".into(),
            coordination_harness: AgentHarness::RestlessManaged,
            worker_harness: crate::runtime::AgentHarness::RestlessManaged,
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
        // Nothing outside the company computer, and nothing the cockpit cannot
        // render or offer as a safe download.
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
        // Office files stay within the selected outcome and are delivered as
        // downloads rather than framed as blank browser previews.
        assert_eq!(
            resolve_review_file(&root, &entry, "/notes.docx").unwrap(),
            root.join("notes.docx")
        );
        assert!(is_runtime_review_download(Path::new("notes.docx")));
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
    }
}
