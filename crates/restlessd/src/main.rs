//! restlessd — the stable coordination core (ARCHITECTURE.md §4.4).
//!
//! Sprint 01 slice: company environment lifecycle over a unix socket and the
//! stable coordination core. JSON-lines protocol: one request line, one
//! response line.

mod acp;
mod activity;
mod airwallex;
mod airwallex_ingress;
mod approval;
mod attention;
mod authority;
mod capability;
mod capability_sourcing;
mod cell;
mod codex;
mod company;
mod connected_tool;
mod context;
mod credential;
mod effect;
mod entry;
mod exec;
mod finance;
mod health;
mod hosted_control;
mod inbound;
mod ingress;
mod launch;
mod legal;
mod model_gateway;
mod owner;
mod owner_brief;
mod plane;
mod publication;
mod reconcile;
mod release;
mod runtime;
mod schedule;
mod spend;
mod staff;
mod telemetry;
mod wire;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use restless_orgintel::OrgIntel;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

#[cfg(test)]
use wire::OWNER_ONLY;
use wire::{authorize, Principal, Request, Response};

/// Money to four decimal places, and never `-0.0` — a negative zero in an
/// owner-facing figure reads as a bug in the accounting, which is exactly the
/// impression a spend report must not give.
fn round_usd(usd: f64) -> f64 {
    let rounded = (usd * 10_000.0).round() / 10_000.0;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

/// OrgIntel connection settings at `$RESTLESS_HOME/orgintel.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrgIntelConfig {
    database_url: String,
}

impl OrgIntelConfig {
    fn default_for_user() -> Self {
        let user = std::env::var("USER").unwrap_or_else(|_| "postgres".to_string());
        Self {
            database_url: format!("postgres://{user}@localhost/restless"),
        }
    }

    fn read_only(root: &Path) -> Result<Self> {
        if let Some(config) = Self::from_plane_environment()? {
            return Ok(config);
        }
        let path = root.join("orgintel.toml");
        if !path.exists() {
            return Ok(Self::default_for_user());
        }
        let raw =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))
    }

    fn load_or_seed(root: &Path) -> Result<Self> {
        if let Some(config) = Self::from_plane_environment()? {
            return Ok(config);
        }
        let path = root.join("orgintel.toml");
        if path.exists() {
            return Self::read_only(root);
        }
        let config = Self::default_for_user();
        let rendered = toml::to_string_pretty(&config).context("render orgintel.toml")?;
        std::fs::write(&path, rendered).with_context(|| format!("seed {}", path.display()))?;
        Ok(config)
    }

    /// Cloud injects the account-plane database connection as a secret
    /// environment value. It must remain an in-memory deployment input: a
    /// hosted plane has a read-only image and must never copy the credential
    /// into its persistent state volume as `orgintel.toml`.
    fn from_plane_environment() -> Result<Option<Self>> {
        let Some(raw) = std::env::var_os("RESTLESS_PLANE_DATABASE_URL") else {
            return Ok(None);
        };
        let raw = raw
            .into_string()
            .map_err(|_| anyhow::anyhow!("RESTLESS_PLANE_DATABASE_URL must be valid UTF-8"))?;
        validate_plane_database_url(&raw)?;
        Ok(Some(Self { database_url: raw }))
    }
}

fn validate_plane_database_url(raw: &str) -> Result<()> {
    if raw.is_empty() || raw.trim() != raw || raw.contains(['\r', '\n']) {
        anyhow::bail!("RESTLESS_PLANE_DATABASE_URL must be one bounded URL value");
    }
    // Parse without ever including the credential in an error or log field.
    let parsed = url::Url::parse(raw).map_err(|_| {
        anyhow::anyhow!("RESTLESS_PLANE_DATABASE_URL must be a valid PostgreSQL URL")
    })?;
    if !matches!(parsed.scheme(), "postgres" | "postgresql")
        || parsed.host_str().is_none()
        || parsed.username().is_empty()
        || parsed.password().is_none_or(str::is_empty)
        || parsed.path().trim_matches('/').is_empty()
        || parsed.fragment().is_some()
    {
        anyhow::bail!(
            "RESTLESS_PLANE_DATABASE_URL must identify a password-authenticated PostgreSQL database"
        );
    }
    Ok(())
}

fn configured_companies(root: &Path) -> Result<Vec<String>> {
    let directory = root.join("companies");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut companies = Vec::new();
    for entry in
        std::fs::read_dir(&directory).with_context(|| format!("read {}", directory.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("toml") {
            if let Some(name) = path.file_stem().and_then(|value| value.to_str()) {
                companies.push(name.to_string());
            }
        }
    }
    companies.sort();
    Ok(companies)
}

/// Provision this company's cell storage, import a legacy shared schema if one
/// is still the only copy, and return a handle bound to the cell's own
/// database and role. This is the single path to an OrgIntel handle — there is
/// no shared-database fallback, because two ways to reach company state is
/// exactly the split brain the cell boundary exists to remove.
async fn ensure_cell_orgintel(
    root: &std::path::Path,
    admin_url: &str,
    company: &str,
) -> Result<OrgIntel> {
    let cell_url = cell::ensure_database(root, admin_url, company).await?;
    if cell::import_legacy_schema(admin_url, &cell_url, company).await? {
        tracing::info!(
            company,
            "imported the legacy shared OrgIntel schema into this cell's own database; \
             the legacy schema is left in place for verification"
        );
    }
    OrgIntel::ensure(&cell_url, company)
        .await
        .with_context(|| format!("open cell OrgIntel for {company}"))
}

/// Lazily ensured per-cell OrgIntel handles (one pool per company, against
/// that company's **own** database and role — see `cell.rs`). `database_url`
/// is the account plane's admin connection, used only to provision cells and
/// to read a legacy shared schema during import; no company query runs on it.
pub(crate) struct OrgIntelRegistry {
    pub(crate) database_url: String,
    pub(crate) root: std::path::PathBuf,
    handles: std::sync::Mutex<HashMap<String, OrgIntel>>,
}

impl OrgIntelRegistry {
    /// This cell's own connection string, provisioning it if absent.
    /// Idempotent, and the only way anything reaches a cell's database.
    pub(crate) async fn cell_database_url(&self, company: &str) -> Result<String> {
        cell::ensure_database(&self.root, &self.database_url, company).await
    }

    async fn get(&self, company: &str) -> Result<OrgIntel> {
        let config = self.root.join("companies").join(format!("{company}.toml"));
        if !config.is_file() {
            anyhow::bail!(
                "company {company:?} is not configured; refusing to provision or reopen an OrgIntel cell"
            );
        }
        let cached = self
            .handles
            .lock()
            .expect("orgintel registry")
            .get(company)
            .cloned();
        if let Some(handle) = cached {
            // A cached handle can outlive its schema: a scenario reset, an
            // operator drop, or a restore removes the tables and every later
            // query fails with `relation "actors" does not exist`. Re-ensure
            // instead of serving a handle to nothing.
            if handle.is_live().await {
                return Ok(handle);
            }
            tracing::warn!(
                company,
                "orgintel schema vanished under a cached handle; re-ensuring"
            );
        }
        let handle = ensure_cell_orgintel(&self.root, &self.database_url, company).await?;
        self.handles
            .lock()
            .expect("orgintel registry")
            .insert(company.to_string(), handle.clone());
        Ok(handle)
    }

    /// Drop a cached handle after its schema is destroyed (S04-T1). `get`
    /// already re-ensures a handle whose schema vanished, so this is an
    /// optimisation rather than a correctness fix — but leaving a handle to a
    /// dropped schema in the map means the next `up` of the same name pays a
    /// failed query first, and that noise is what made the sprint-02 reuse bug
    /// hard to read.
    fn forget(&self, company: &str) {
        self.handles
            .lock()
            .expect("orgintel registry")
            .remove(company);
    }
}

/// Owner and Exec exist for the lifetime of a company, not only after its
/// first conversation or wake. Keeping that lifecycle fact here prevents a
/// freshly created company from presenting an empty People surface or making
/// its first staff creation depend on an unrelated `tell` command.
async fn ensure_standing_actors(org: &OrgIntel, model: Option<&str>) -> Result<()> {
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .context("ensure standing Owner")?;
    org.ensure_actor_with_model("exec", "exec", "exec", "The Exec", model)
        .await
        .context("ensure standing Exec")?;
    Ok(())
}

pub(crate) struct Daemon {
    pub(crate) root: PathBuf,
    pub(crate) capabilities: capability::CapabilityIssuer,
    pub(crate) spend: spend::SpendLedger,
    pub(crate) authority: authority::AuthorityStore,
    pub(crate) publication: publication::PublicationManager,
    pub(crate) launch: launch::LaunchBroker,
    pub(crate) orgintel: OrgIntelRegistry,
    /// One product-facing company-computer boundary. Local Core installs the
    /// Docker transport; hosted Core installs the outbound Runtime Bridge.
    /// Callers above this field never branch on deployment topology.
    pub(crate) runtime_transport:
        std::sync::Arc<dyn restlessd::runtime_transport::RuntimeTransport>,
    pub(crate) staff: staff::StaffRegistry,
    /// Reconnectable live projections for agent turns. Completed messages,
    /// Work, and Attempts remain OrgIntel truth; this state is ephemeral.
    pub(crate) activities: activity::AgentActivityStreams,
    /// One wake at a time per company, however the wake was requested —
    /// the scheduler (T6) and the owner-typed socket path share this set.
    pub(crate) in_flight: schedule::InFlight,
    /// Wake-only hints from launchd/systemd or the owner. The durable schedule
    /// ledger decides whether anything is due; this signal carries no work.
    pub(crate) schedule_wake: std::sync::Arc<tokio::sync::Notify>,
}

/// A Runtime is only ready for coordination after the host-issued bridge grant
/// is materialised inside its persistent computer. Keep the issuer and the
/// Runtime file write together here: neither the Runtime nor OrgIntel owns
/// that authority boundary.
pub(crate) async fn materialize_runtime_bridge(daemon: &Daemon, company: &str) -> Result<()> {
    let bridge = daemon
        .capabilities
        .issue_runtime_bridge(company)
        .context("issue Runtime bridge capability")?;
    runtime::install_runtime_bridge_capability(company, &bridge)
        .await
        .context("install Runtime bridge capability")
}

/// Base TCP port local company containers reach for coordination. Hosted
/// Runtimes use the TLS WebSocket endpoint on their exact account-plane host.
pub(crate) const COORD_TCP_PORT: u16 = 7791;

/// Namespace every host listener owned by one daemon with a single bounded
/// offset. The default remains the established port map. A second isolated
/// daemon can set `RESTLESS_PORT_OFFSET` and receive a coherent model relay,
/// coordination plane, owner surface, and ingress set without borrowing or
/// terminating the first daemon's processes.
pub(crate) fn port_offset() -> Result<u16> {
    let raw = std::env::var("RESTLESS_PORT_OFFSET").unwrap_or_else(|_| "0".to_string());
    raw.parse::<u16>()
        .with_context(|| format!("parse RESTLESS_PORT_OFFSET {raw:?} as a non-negative integer"))
}

pub(crate) fn port_with_offset(base: u16) -> Result<u16> {
    base.checked_add(port_offset()?).with_context(|| {
        format!("RESTLESS_PORT_OFFSET places base port {base} outside the TCP port range")
    })
}

pub(crate) fn runtime_coordinator() -> Result<String> {
    if restlessd::hosted_runtime::RuntimeBackendKind::from_entry_mode(
        std::env::var("RESTLESS_ENTRY_MODE").ok().as_deref(),
    )
    .map_err(|error| anyhow::anyhow!(error))?
        == restlessd::hosted_runtime::RuntimeBackendKind::HostedRuntimeBridge
    {
        let plane = restlessd::hosted_runtime::HostedPlaneConfig::from_environment()
            .map_err(|error| anyhow::anyhow!(error))?
            .context("network entry mode requires the exact hosted plane configuration")?;
        return Ok(format!("wss://{}/internal/v1/coordination", plane.hostname));
    }
    Ok(format!(
        "host.docker.internal:{}",
        port_with_offset(COORD_TCP_PORT)?
    ))
}

/// The listener is identity evidence. A Unix socket is the local appliance
/// owner boundary; TCP is only the Company Runtime bridge and therefore must
/// present a signed capability before it can reach dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionOrigin {
    LocalOwner,
    RuntimeTcp,
}

/// Advance ordinary Work only after authenticated provider state proves the
/// owner's irreducible approval step has ended. Submission and IN_APPROVAL are
/// intentionally not enough; repeated reconciliation is idempotent.
pub(crate) async fn continue_after_payment_observation(
    daemon: &Daemon,
    company: &str,
    payment: &finance::PaymentIntent,
    state_changed: bool,
) -> Result<()> {
    use finance::PaymentState;
    if !matches!(
        payment.state,
        PaymentState::Scheduled
            | PaymentState::Processing
            | PaymentState::Settled
            | PaymentState::Rejected
            | PaymentState::Cancelled
            | PaymentState::Failed
    ) {
        return Ok(());
    }
    let org = daemon.orgintel.get(company).await?;
    let provider_id = payment
        .provider_transfer_id
        .as_deref()
        .unwrap_or("unavailable");
    let resolution = format!(
        "Airwallex authenticated API observed transfer {provider_id} as {} (raw status {}).",
        payment.state.as_str(),
        payment.raw_provider_status.as_deref().unwrap_or("unknown")
    );
    let newly_resolved = org
        .resolve_observed_handoff(payment.request.owner_handoff_id, "daemon", &resolution)
        .await?;
    if !newly_resolved && state_changed {
        org.ensure_actor("daemon", "system", "system-sender", "The daemon")
            .await?;
        let work = org
            .get_work(payment.request.work_id)
            .await?
            .context("payment Work disappeared before provider continuation")?;
        org.send_work_message("daemon", &work.owner_id, work.id, &resolution)
            .await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let machine_profile = restlessd::appliance::MachineProfile::from_env()?;
    // Local source checkouts conventionally keep bootstrap credentials in an
    // ignored `.env`. Load it before any subsystem reads configuration, while
    // preserving explicitly inherited service-manager variables. Infisical is
    // the durable backend; this is the one-time/local migration source.
    match dotenvy::dotenv() {
        Ok(path) => eprintln!("loaded local environment from {}", path.display()),
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("load local .env"),
    }
    restlessd::appliance::load_profile_environment(&machine_profile)?;
    if matches!(std::env::args().nth(1).as_deref(), Some("--help" | "-h")) {
        println!(
            "restlessd\n\nThe supervised Restless account plane. Run it without arguments; use `restless appliance status` for lifecycle status."
        );
        return Ok(());
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    // The test issuer for network entry (S27-T5). Restless Cloud is the real
    // issuer; this exists so an end-to-end run can mint against the same wire
    // format the verifier reads.
    if std::env::args().nth(1).as_deref() == Some("mint-entry-assertion") {
        println!("{}", entry::mint_from_env()?);
        return Ok(());
    }

    if std::env::args().nth(1).as_deref() == Some("appliance-preflight") {
        if machine_profile.kind != restlessd::appliance::ProfileKind::Stable {
            anyhow::bail!("appliance preflight requires the stable profile");
        }
        let cockpit = std::env::var_os("RESTLESS_COCKPIT_DIR")
            .map(PathBuf::from)
            .context("RESTLESS_COCKPIT_DIR is required for appliance preflight")?;
        if !cockpit.join("index.html").is_file() {
            anyhow::bail!("the staged Cockpit has no index.html");
        }
        for company in configured_companies(&machine_profile.state_root)? {
            runtime::CompanyConfig::load(&machine_profile.state_root, &company)
                .with_context(|| format!("preflight company configuration {company}"))?;
        }
        let orgintel = OrgIntelConfig::read_only(&machine_profile.state_root)?;
        OrgIntel::probe(&orgintel.database_url)
            .await
            .context("preflight could not reach OrgIntel")?;
        println!(
            "{}",
            serde_json::json!({
                "status": "ready_to_activate",
                "profile": "stable",
                "state_root": machine_profile.state_root,
                "cockpit": cockpit,
            })
        );
        return Ok(());
    }

    // Cloud's reviewed owner-plane template is one atomic contract. Parse it
    // before opening state or listeners so a partial credential set, mutable
    // artifact, dirty source build or mismatched deployment identity cannot
    // degrade into a network-reachable plane with weaker defaults.
    let hosted_config = hosted_control::HostedDeploymentConfig::from_environment()?;
    let network_entry = hosted_config.is_some();

    // Resolve and lock the machine profile before any migration, provider
    // listener or schedule reconciliation can mutate state. The Unix socket
    // is a transport and may be stale; it is not a singleton primitive.
    let root = machine_profile.state_root.clone();
    std::fs::create_dir_all(root.join("companies"))
        .with_context(|| format!("create state root {}", root.display()))?;
    std::fs::create_dir_all(machine_profile.log_dir())?;
    std::fs::create_dir_all(machine_profile.launch_cache_dir())?;
    let _singleton = restlessd::appliance::SingletonGuard::acquire(&machine_profile)?;
    tracing::info!(
        profile = machine_profile.kind.as_str(),
        root = %machine_profile.state_root.display(),
        lock = %_singleton.path().display(),
        "machine profile locked"
    );
    let capabilities = capability::CapabilityIssuer::open(&root)?;
    // Open authoritative charged-use accounting before the model relay. The
    // relay receives this exact ledger and is the only model path permitted to
    // append charged records.
    let spend = spend::SpendLedger::open(&root)?;

    // Model access is a host authority boundary. OMP's imported broker and
    // gateway hold the provider credential; company processes receive only a
    // signed, scoped relay capability. Its network/provider startup is kept
    // off the owner-surface critical path below.
    let company_configs = configured_companies(&root)?
        .into_iter()
        .map(|company| runtime::CompanyConfig::load(&root, &company))
        .collect::<Result<Vec<_>>>()?;
    // T5: coordination state. The database must answer at boot — probe,
    // never guess that it will be there when a company wakes.
    let orgintel_config = OrgIntelConfig::load_or_seed(&root)?;
    OrgIntel::probe(&orgintel_config.database_url)
        .await
        .context("orgintel database is not reachable at boot")?;

    let authority = authority::AuthorityStore::connect(&orgintel_config.database_url).await?;

    // Two supported topologies (ADR 0007): direct loopback, or a network
    // entry that verifies a signed assertion. Network assertions consume
    // their JTI through the durable Authority store, so a daemon restart or
    // another replica cannot replay one. Resolve this before any provider or
    // scheduler work; incomplete network configuration remains a boot error.
    let owner_config =
        owner::OwnerConfig::from_env_with_replay(std::sync::Arc::new(authority.clone()))?;
    runtime::validate_company_image_config(owner_config.is_network())?;

    // One-time custody transfer from the old recoverable event stream. Do it
    // before listeners open so no effect can race its own migration.
    for company in configured_companies(&root)? {
        let bootstrap = async {
            let mut config = runtime::CompanyConfig::load(&root, &company)?;
            // Bootstrap is a serial migration pass, not the live handle registry.
            // Caching one pool per historical test company here exhausted
            // PostgreSQL before the daemon could finish booting. Keep only the
            // current company's pool alive; the runtime registry below remains
            // lazy and caches only companies that are actually used.
            let org = ensure_cell_orgintel(&root, &orgintel_config.database_url, &company).await?;
            ensure_standing_actors(&org, Some(&config.model)).await?;
            let imported = authority
                .import_legacy_company(&company, &org, &approval::legacy_config_approvals(&config))
                .await?;
            if imported > 0 {
                tracing::info!(
                    company,
                    imported,
                    "migrated governance truth into Authority"
                );
            }
            approval::purge_legacy_config_approvals(&root, &mut config)?;
            drop(org);
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if let Err(error) = bootstrap {
            // One historical or experimental cell must not take every live
            // company and every schedule offline. Keep the exact company
            // unavailable and visible through its own failing cell probe,
            // while the rest of the account plane continues to operate.
            tracing::error!(company, "company cell quarantined during boot: {error:#}");
        }
    }

    let publication = publication::PublicationManager::new(&root, authority.clone())?;
    let launch = launch::LaunchBroker::new(&root)?;
    let runtime_transport_slot = restlessd::runtime_transport::RuntimeTransportSlot::default();
    if hosted_config.is_none() {
        runtime_transport_slot
            .install(std::sync::Arc::new(
                restlessd::local_runtime_transport::LocalDockerRuntimeTransport::from_environment()
                    .map_err(|error| anyhow::anyhow!(error))?,
            ))
            .map_err(|error| anyhow::anyhow!(error))?;
    }
    let daemon = std::sync::Arc::new(Daemon {
        root: root.clone(),
        capabilities,
        spend,
        authority,
        publication,
        launch,
        orgintel: OrgIntelRegistry {
            database_url: orgintel_config.database_url,
            root: root.clone(),
            handles: std::sync::Mutex::new(HashMap::new()),
        },
        runtime_transport: std::sync::Arc::new(runtime_transport_slot.clone()),
        staff: staff::StaffRegistry::default(),
        activities: activity::AgentActivityStreams::default(),
        in_flight: std::sync::Arc::new(std::sync::Mutex::new(schedule::WakeClaims::default())),
        schedule_wake: std::sync::Arc::new(tokio::sync::Notify::new()),
    });

    let (hosted_model_admission, hosted_model_requests) = if hosted_config.is_some() {
        let (admission, requests) = model_gateway::hosted_admission_channel();
        (Some(admission), Some(requests))
    } else {
        (None, None)
    };

    // Finish the hosted control boundary before detaching the owner listener.
    // A missing schema or unsafe installation key must terminate startup, not
    // leave a daemon process alive with its authenticated HTTP surface absent.
    let hosted_control = match hosted_config {
        Some(config) => {
            let control = std::sync::Arc::new(
                hosted_control::HostedControl::open(
                    config,
                    daemon.clone(),
                    hosted_model_admission.expect("network mode created hosted model admission"),
                )
                .await?,
            );
            runtime_transport_slot
                .install(std::sync::Arc::new(control.runtime_transport()))
                .map_err(|error| anyhow::anyhow!(error))?;
            Some(control)
        }
        None => None,
    };

    // Owner entry is the appliance's control surface, not proof that every
    // external model provider is reachable. Open it before slower repair and
    // provider reconciliation so a founder can always see and repair a
    // degraded company instead of staring at a dead localhost port.
    let owner_daemon = std::sync::Arc::clone(&daemon);
    tokio::spawn(async move {
        if let Err(error) = owner::serve(owner_daemon, owner_config, hosted_control).await {
            tracing::error!("owner gateway stopped: {error:#}");
        }
    });

    // Start the provider boundary as soon as the owner surface exists. Large
    // historical stores can make orphan and publication repair expensive;
    // model readiness must not queue behind those reads. The scheduler still
    // waits on the explicit recovery barrier below, so no autonomous work can
    // race cleanup from a previous daemon generation.
    let test_scheduler_disabled =
        std::env::var("RESTLESS_TEST_DISABLE_SCHEDULER").is_ok_and(|value| value == "1");
    if test_scheduler_disabled
        && company_configs
            .iter()
            .any(|config| !config.name.ends_with("_test"))
    {
        anyhow::bail!("RESTLESS_TEST_DISABLE_SCHEDULER is allowed only on an all-test plane");
    }
    let (recovery_ready_tx, mut recovery_ready_rx) = tokio::sync::watch::channel(false);
    let mut model_configs = company_configs.clone();
    let model_root = root.clone();
    let model_capabilities = daemon.capabilities.clone();
    let model_spend = daemon.spend.clone();
    let schedule_daemon = std::sync::Arc::clone(&daemon);
    tokio::spawn(async move {
        let mut model_processes = loop {
            match model_gateway::start(
                &model_configs,
                &model_root,
                model_capabilities.clone(),
                model_spend.clone(),
            )
            .await
            {
                Ok(processes) => {
                    tracing::info!(
                        ready = processes.is_some(),
                        "model gateway initial admission reconciled"
                    );
                    break processes;
                }
                Err(error) => {
                    tracing::error!(
                        "model gateway unavailable; owner plane remains ready and retry is scheduled: {error:#}"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            }
        };
        while !*recovery_ready_rx.borrow() {
            if recovery_ready_rx.changed().await.is_err() {
                return;
            }
        }
        if test_scheduler_disabled {
            tracing::warn!("automatic scheduler disabled for an isolated test plane");
        } else {
            tokio::spawn(schedule::run(std::sync::Arc::clone(&schedule_daemon)));
        }

        if let Some(mut requests) = hosted_model_requests {
            while requests
                .reconcile_next(
                    &mut model_configs,
                    &mut model_processes,
                    &model_root,
                    &model_capabilities,
                    &model_spend,
                )
                .await
            {}
        }
        let _model_processes = model_processes;
        std::future::pending::<()>().await;
    });

    // Effect children carry the narrowest live secret boundary. Reap an old
    // daemon's dedicated effect UID before a new child or scheduler may start;
    // Authority keeps the interrupted intent unknown until explicit evidence.
    effect::sweep_orphans(&daemon.root, &daemon.runtime_transport).await;

    // T9: agent processes outliving their supervising daemon are orphans —
    // reap them and close their running Work Attempts before anything new wakes.
    staff::sweep_orphans(&daemon.root, &daemon.orgintel, &daemon.runtime_transport).await;

    // Published services are provider-owned processes, not Runtime children.
    // Reconcile their receipts after the daemon's own orphan sweeps: a live
    // fixture is adopted, a dead authorized fixture is restarted once, and an
    // expired grant is torn down. One broken experimental company cannot block
    // the account plane from opening its listeners.
    for company in configured_companies(&root)? {
        if !company.ends_with("_test") {
            continue;
        }
        let outcome = async {
            let org = daemon.orgintel.get(&company).await?;
            daemon.publication.reconcile_company(&org, &company).await
        }
        .await;
        if let Err(error) = outcome {
            tracing::error!(
                company,
                "published-service reconciliation failed: {error:#}"
            );
        }
    }

    recovery_ready_tx.send_replace(true);

    // T6: the scheduler is what makes the company act without the owner
    // typing — time triggers (exec-set schedules + periodic tick) and
    // OrgIntel LISTEN/NOTIFY events share one loop. Product integration tests
    // may drive the exact semantic loop themselves; a narrowly named escape
    // hatch prevents the resident scheduler racing that controller. It is
    // refused if any real company is configured.
    let sock = root.join("restlessd.sock");
    if sock.exists() {
        std::fs::remove_file(&sock)?;
    }
    let listener = UnixListener::bind(&sock).with_context(|| format!("bind {}", sock.display()))?;
    tracing::info!(socket = %sock.display(), "restlessd listening");

    // Publish this plane so a CLI pointed at another home can name the live
    // planes instead of reporting that none is running. Dropped on exit.
    let _plane_registration = match plane::register(
        &root,
        &sock,
        port_offset()?,
        company_configs
            .iter()
            .map(|config| config.name.clone())
            .collect(),
    ) {
        Ok(registration) => Some(registration),
        Err(error) => {
            tracing::warn!("could not publish this plane for CLI discovery: {error:#}");
            None
        }
    };

    // Unix sockets do not cross the Docker Desktop file share (probed: the
    // mount hangs), so containers reach the daemon over TCP on the same
    // proven path as the model relay. TCP is capability-authenticated before
    // dispatch; company identity is never trusted as JSON sent by the caller.
    if !network_entry {
        let coord_addr = format!("0.0.0.0:{}", port_with_offset(COORD_TCP_PORT)?);
        match tokio::net::TcpListener::bind(&coord_addr).await {
            Ok(tcp) => {
                tracing::info!(addr = %coord_addr, "coordination TCP listening (local company containers)");
                let tcp_daemon = std::sync::Arc::clone(&daemon);
                tokio::spawn(async move {
                    loop {
                        match tcp.accept().await {
                            Ok((stream, _)) => {
                                let daemon = std::sync::Arc::clone(&tcp_daemon);
                                tokio::spawn(async move {
                                    if let Err(error) =
                                        serve(stream, &daemon, ConnectionOrigin::RuntimeTcp).await
                                    {
                                        tracing::warn!("tcp connection error: {error:#}");
                                    }
                                });
                            }
                            Err(error) => tracing::warn!("tcp accept: {error:#}"),
                        }
                    }
                });
            }
            Err(error) => {
                tracing::error!(addr = %coord_addr, "coordination TCP bind failed: {error:#} — \
                    local agents will have no coordination channel");
            }
        }
    } else {
        tracing::info!(
            "hosted Runtime coordination is available only through authenticated WSS entry"
        );
    }

    // S03-T2: the world's front door, on its OWN listener and its own task.
    // The failure boundary is the point (AC6): a slow, malformed or flooded
    // inbound request must not be able to stall the scheduler — F12's lesson,
    // where one company's hung Docker took down all three. Absent secret means
    // absent rail: we do not open an unauthenticated public port, ever, and a
    // company that cannot receive is honest about it rather than silently open.
    let webhook_reference = std::env::var("RESTLESS_RESEND_WEBHOOK_CREDENTIAL")
        .unwrap_or_else(|_| "env:RESEND_WEBHOOK_SECRET".to_string());
    match credential::resolve_reference(&webhook_reference).await {
        Ok(secret) => {
            let sink = std::sync::Arc::new(inbound::AuthoritySink {
                daemon: std::sync::Arc::clone(&daemon),
            });
            tokio::spawn(async move {
                let port = match port_with_offset(ingress::INGRESS_PORT) {
                    Ok(port) => port,
                    Err(error) => {
                        tracing::error!("event ingress port is invalid: {error:#}");
                        return;
                    }
                };
                if let Err(error) = ingress::serve(port, secret, sink).await {
                    tracing::error!("event ingress stopped: {error:#}");
                }
            });
        }
        Err(error) => tracing::warn!(
            reference = %webhook_reference,
            "event ingress is NOT listening because its credential did not resolve: {error:#}. \
             The company can send but cannot receive; inbound replies will not wake it"
        ),
    }
    // Repair the only durable seam after Authority custody. The immediate
    // projection is event-driven; this bounded cursor scan exists solely for
    // restart/crash recovery and is idempotent at the OrgIntel source ref.
    let inbound_daemon = std::sync::Arc::clone(&daemon);
    tokio::spawn(async move {
        loop {
            match inbound::reconcile_pending(&inbound_daemon).await {
                Ok(count) if count > 0 => {
                    tracing::info!(count, "reconciled pending inbound projections")
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!("inbound projection reconciliation deferred: {error:#}")
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    // Finance webhooks use Airwallex's own timestamp+body HMAC and trigger a
    // direct authenticated provider read. The listener is independent of the
    // Resend/Svix rail so one provider's credential outage cannot disable the
    // other or the scheduler.
    let finance_daemon = std::sync::Arc::clone(&daemon);
    tokio::spawn(async move {
        if let Err(error) = airwallex_ingress::serve(finance_daemon).await {
            tracing::error!("Airwallex event ingress stopped: {error:#}");
        }
    });

    // `restless-dev` stops the daemon with SIGTERM. Observe it inside the
    // runtime so owned model-broker/gateway child handles are dropped and
    // killed before the process exits; an abrupt default signal exit leaves
    // those children holding 7789/7790 and the next daemon half-attached to
    // stale supervision.
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let daemon = std::sync::Arc::clone(&daemon);
                tokio::spawn(async move {
                    if let Err(error) =
                        serve(stream, &daemon, ConnectionOrigin::LocalOwner).await
                    {
                        tracing::warn!("connection error: {error:#}");
                    }
                });
            }
            () = &mut shutdown => {
                tracing::info!("shutdown requested; stopping supervised daemon children");
                break;
            }
        }
    }
    Ok(())
}

async fn shutdown_signal() {
    let interrupt = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = interrupt => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = interrupt.await;
    }
}

async fn await_stopped_company_idle(daemon: &Daemon, company: &str) -> bool {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let exec_active = daemon
            .in_flight
            .lock()
            .map(|claims| claims.is_active(company))
            .unwrap_or(true);
        if !exec_active && daemon.staff.running_actors(company).is_empty() {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

async fn serve<S>(stream: S, daemon: &Daemon, origin: ConnectionOrigin) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (read, mut write) = tokio::io::split(stream);
    let mut lines = BufReader::new(read).lines();
    while let Some(line) = lines.next_line().await? {
        let mut request = match Request::decode(&line) {
            Ok(request) => request,
            Err(error) => {
                let response = Response::err(format!("bad request: {error}"));
                let mut out = serde_json::to_string(&response)?;
                out.push('\n');
                write.write_all(out.as_bytes()).await?;
                continue;
            }
        };
        // The gate sits above the watch/dispatch branch, so a streaming
        // command cannot slip past it. Unix derives local owner access; TCP
        // derives Runtime scope from a signed, expiring capability.
        let principal = match authenticate_request(&mut request, &daemon.capabilities, origin) {
            Ok(principal) => principal,
            Err(refusal) => {
                tracing::warn!(
                    cmd = %request.cmd,
                    company = ?request.company,
                    ?origin,
                    "refused: {refusal}"
                );
                let response = Response::err_kind("authority", refusal);
                let mut out = serde_json::to_string(&response)?;
                out.push('\n');
                write.write_all(out.as_bytes()).await?;
                continue;
            }
        };
        // watch is the one streaming command: it owns the connection until
        // the client goes away, writing one JSON event per line.
        if request.cmd == "watch" {
            watch_events(&mut write, daemon, request.company.as_deref()).await?;
            continue;
        }
        let response = dispatch(request, daemon, principal).await;
        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        write.write_all(out.as_bytes()).await?;
    }
    Ok(())
}

fn authenticate_request(
    request: &mut Request,
    capabilities: &capability::CapabilityIssuer,
    origin: ConnectionOrigin,
) -> std::result::Result<Principal, String> {
    match origin {
        ConnectionOrigin::LocalOwner => authorize(Principal::Owner, &request.cmd),
        ConnectionOrigin::RuntimeTcp => {
            Principal::legacy_runtime_claim(request.principal.as_deref())?;
            let token = request
                .session_capability
                .as_deref()
                .ok_or_else(|| "TCP Runtime request carries no session capability".to_string())?;
            let grant = capabilities
                .verify_coordination(token)
                .map_err(|error| format!("invalid Runtime capability: {error:#}"))?;
            let requested_company = request
                .company
                .as_deref()
                .ok_or_else(|| "TCP Runtime request needs a company".to_string())?;
            if requested_company != grant.company {
                return Err(format!(
                    "Runtime capability is scoped to company {:?}, not {:?}",
                    grant.company, requested_company
                ));
            }
            request.company = Some(grant.company);
            bind_runtime_actor(request, &grant.actor)?;
            authorize(Principal::CompanyExec, &request.cmd)
        }
    }
}

/// A Runtime actor cannot re-label an attribution field after the daemon has
/// authenticated the session. Fields that name another person as a target
/// remain ordinary OrgIntel work, while every acting-attribution field is
/// pinned here.
fn bind_runtime_actor(request: &mut Request, actor: &str) -> std::result::Result<(), String> {
    let command = request.cmd.clone();
    match command.as_str() {
        "actor-create"
        | "actor-model"
        | "actor-retire"
        | "team-create"
        | "team-update"
        | "team-assign"
        | "team-lead"
        | "team-disband"
        | "goal-add"
        | "work-goal"
        | "work-assign"
        | "work-artifact"
        | "work-gate"
        | "work-handoff"
        | "effect"
        | "effect-reconcile"
        | "identity-evidence-add"
        | "identity-propose"
        | "identity-brief"
        | "voice-evidence-add"
        | "voice-bind"
        | "voice-brief"
        | "voice-render"
        | "voice-review"
        | "voice-learn" => pin_actor(&mut request.orgintel.actor, actor, "actor")?,
        "publish-candidate" | "publish-request" => {
            pin_actor(&mut request.publication.actor, actor, "publication actor")?
        }
        "work-add"
        | "work-edge"
        | "work-gate-retire"
        | "work-handoff-escalate"
        | "work-handoff-refresh"
        | "work-handoff-prepare-brief"
        | "work-handoff-resolve"
        | "work-interrupt"
        | "work-resume"
        | "work-abandon"
        | "judgement"
        | "schedule-list"
        | "schedule-add"
        | "schedule-policy"
        | "schedule-cancel" => pin_actor(&mut request.common.as_actor, actor, "acting actor")?,
        "message" => pin_actor(&mut request.common.from, actor, "message sender")?,
        "inbox" => {
            pin_actor(&mut request.orgintel.actor, actor, "inbox actor")?;
            pin_actor(&mut request.common.as_actor, actor, "inbox actor")?;
        }
        "browser-request" => pin_actor(&mut request.common.id, actor, "browser requester")?,
        _ => {}
    }
    Ok(())
}

fn pin_actor(
    supplied: &mut Option<String>,
    actor: &str,
    field: &str,
) -> std::result::Result<(), String> {
    match supplied.as_deref() {
        Some(value) if value != actor => Err(format!(
            "Runtime session for actor {actor:?} cannot claim {field} {value:?}"
        )),
        Some(_) => Ok(()),
        None => {
            *supplied = Some(actor.to_string());
            Ok(())
        }
    }
}

/// Stream the operational event stream: a recent snapshot, then new events
/// as they land (2s poll against the durable table — the stream survives
/// the client's attention, not the other way around). A dead client is a
/// write error and ends the stream.
async fn watch_events<W>(
    write: &mut tokio::io::WriteHalf<W>,
    daemon: &Daemon,
    company: Option<&str>,
) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let Some(company) = company else {
        let mut out = serde_json::to_string(&Response::err("missing company"))?;
        out.push('\n');
        write.write_all(out.as_bytes()).await?;
        return Ok(());
    };
    let org = match daemon.orgintel.get(company).await {
        Ok(org) => org,
        Err(error) => {
            let mut out = serde_json::to_string(&Response::err(format!("{error:#}")))?;
            out.push('\n');
            write.write_all(out.as_bytes()).await?;
            return Ok(());
        }
    };
    let mut watermark: i64 = 0;
    for event in org.list_events(20).await?.iter().rev() {
        let mut out = serde_json::to_string(&event)?;
        out.push('\n');
        write.write_all(out.as_bytes()).await?;
        watermark = watermark.max(event.id);
    }
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        match org.events_after(watermark).await {
            Ok(events) => {
                for event in events {
                    let mut out = serde_json::to_string(&event)?;
                    out.push('\n');
                    write.write_all(out.as_bytes()).await?;
                    watermark = event.id;
                }
            }
            Err(error) => {
                let mut out = serde_json::to_string(&Response::err(format!("{error:#}")))?;
                out.push('\n');
                let _ = write.write_all(out.as_bytes()).await;
                return Ok(());
            }
        }
    }
}

fn parse_optional_uuid(
    value: Option<&str>,
    label: &str,
) -> std::result::Result<Option<uuid::Uuid>, String> {
    value
        .map(|value| uuid::Uuid::parse_str(value).map_err(|error| format!("bad {label}: {error}")))
        .transpose()
}

fn parse_required_uuid(
    value: Option<&str>,
    label: &str,
) -> std::result::Result<uuid::Uuid, String> {
    parse_optional_uuid(value, label)?.ok_or_else(|| format!("{label} is required"))
}

fn parse_voice_channel(
    value: Option<&str>,
) -> std::result::Result<Option<restless_orgintel::VoiceChannel>, String> {
    match value {
        None => Ok(None),
        Some("newsletter") => Ok(Some(restless_orgintel::VoiceChannel::Newsletter)),
        Some("founder_email") => Ok(Some(restless_orgintel::VoiceChannel::FounderEmail)),
        Some("support") => Ok(Some(restless_orgintel::VoiceChannel::Support)),
        Some("transactional_email") => {
            Ok(Some(restless_orgintel::VoiceChannel::TransactionalEmail))
        }
        Some("product_ui") => Ok(Some(restless_orgintel::VoiceChannel::ProductUi)),
        Some("blog") => Ok(Some(restless_orgintel::VoiceChannel::Blog)),
        Some(other) => Err(format!(
            "bad voice channel {other:?}; expected newsletter, founder_email, support, transactional_email, product_ui or blog"
        )),
    }
}

fn parse_visual_channel(
    value: Option<&str>,
) -> std::result::Result<Option<restless_orgintel::VisualChannel>, String> {
    match value {
        None => Ok(None),
        Some("landing_page") => Ok(Some(restless_orgintel::VisualChannel::LandingPage)),
        Some("email") => Ok(Some(restless_orgintel::VisualChannel::Email)),
        Some("product") => Ok(Some(restless_orgintel::VisualChannel::Product)),
        Some("social") => Ok(Some(restless_orgintel::VisualChannel::Social)),
        Some(other) => Err(format!(
            "bad visual channel {other}; expected landing_page|email|product|social"
        )),
    }
}

fn parse_culture_case(
    value: Option<&str>,
) -> std::result::Result<Option<restless_orgintel::CultureCase>, String> {
    match value {
        None => Ok(None),
        Some("disagreement") => Ok(Some(restless_orgintel::CultureCase::Disagreement)),
        Some("uncertain_incident") => Ok(Some(restless_orgintel::CultureCase::UncertainIncident)),
        Some("customer_recovery") => Ok(Some(restless_orgintel::CultureCase::CustomerRecovery)),
        Some("quality_tradeoff") => Ok(Some(restless_orgintel::CultureCase::QualityTradeoff)),
        Some("hiring") => Ok(Some(restless_orgintel::CultureCase::Hiring)),
        Some(other) => Err(format!(
            "bad culture case {other}; expected disagreement|uncertain_incident|customer_recovery|quality_tradeoff|hiring"
        )),
    }
}

async fn dispatch(request: Request, daemon: &Daemon, principal: Principal) -> Response {
    // The company catalogue exists above any one company. Keep it explicit
    // instead of inventing a fake global company.
    if request.cmd == "company-list" {
        let directory = daemon.root.join("companies");
        let mut companies = Vec::new();
        match std::fs::read_dir(&directory) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|value| value.to_str()) == Some("toml") {
                        if let Some(name) = path.file_stem().and_then(|value| value.to_str()) {
                            companies.push(name.to_string());
                        }
                    }
                }
                companies.sort();
                return Response::ok(serde_json::json!(companies));
            }
            Err(error) => return Response::err(format!("read {}: {error}", directory.display())),
        }
    }
    if request.cmd == "schedule-wake" {
        let adapter = request.lifecycle.adapter.as_deref().unwrap_or("manual");
        if !matches!(adapter, "manual" | "launchd" | "systemd") {
            return Response::err("schedule wake adapter must be manual|launchd|systemd");
        }
        let observed_at = chrono::Utc::now();
        let evidence = serde_json::json!({
            "adapter": adapter,
            "observed_at": observed_at,
            "pid": std::process::id(),
        });
        let path = daemon.root.join("machine/last-schedule-wake.json");
        let result = (|| -> Result<()> {
            let parent = path.parent().expect("wake evidence has a parent");
            std::fs::create_dir_all(parent)?;
            let temporary = parent.join(format!(".last-schedule-wake-{}.json", std::process::id()));
            std::fs::write(&temporary, serde_json::to_vec_pretty(&evidence)?)?;
            std::fs::rename(&temporary, &path)?;
            Ok(())
        })();
        if let Err(error) = result {
            return Response::err(format!("record schedule wake transport: {error:#}"));
        }
        daemon.schedule_wake.notify_one();
        return Response::ok(evidence);
    }
    let company = match request.company.as_deref() {
        Some(name) => name,
        None => return Response::err("missing company"),
    };
    match request.cmd.as_str() {
        "publish-candidate" => {
            let actor = match request.publication.actor.as_deref() {
                Some(actor) => actor,
                None => return Response::err("publish-candidate needs actor"),
            };
            let source = match request.publication.source_artifact_ref_id.as_deref() {
                Some(source) => source,
                None => return Response::err("publish-candidate needs source_artifact_ref_id"),
            };
            let manifest = match request.publication.service_manifest.clone() {
                Some(value) => match serde_json::from_value(value) {
                    Ok(manifest) => manifest,
                    Err(error) => return Response::err(format!("bad service_manifest: {error}")),
                },
                None => return Response::err("publish-candidate needs service_manifest"),
            };
            match daemon.orgintel.get(company).await {
                Ok(org) => match daemon
                    .publication
                    .create_candidate(&org, company, actor, source, manifest)
                    .await
                {
                    Ok(value) => Response::ok(value),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "publish-request" => {
            let Some(actor) = request.publication.actor.as_deref() else {
                return Response::err("publish-request needs actor");
            };
            let Some(candidate) = request.publication.candidate_artifact_ref_id.as_deref() else {
                return Response::err("publish-request needs candidate_artifact_ref_id");
            };
            let audience = match request.publication.publication_audience.as_deref() {
                Some("owner-only") => restlessd::published_service_contract::Audience::OwnerOnly,
                Some("named-invitees") => {
                    restlessd::published_service_contract::Audience::NamedInvitees
                }
                Some("public") => restlessd::published_service_contract::Audience::Public,
                _ => {
                    return Response::err(
                        "publish-request needs publication_audience owner-only|named-invitees|public",
                    );
                }
            };
            let expires_at = match request
                .publication
                .publication_expires_at
                .as_deref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            {
                Some(value) => value.with_timezone(&chrono::Utc),
                None => {
                    return Response::err("publish-request needs RFC3339 publication_expires_at");
                }
            };
            let start_deadline = match request
                .publication
                .publication_start_deadline
                .as_deref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            {
                Some(value) => value.with_timezone(&chrono::Utc),
                None => {
                    return Response::err(
                        "publish-request needs RFC3339 publication_start_deadline",
                    );
                }
            };
            let resources = restlessd::published_service_contract::ResourceLimits {
                cpu_millis: request.publication.cpu_millis.unwrap_or(500),
                memory_mib: request.publication.memory_mib.unwrap_or(512),
                ephemeral_storage_mib: request.publication.ephemeral_storage_mib.unwrap_or(512),
                max_connections: request.publication.max_connections.unwrap_or(32),
            };
            let Some(key) = request.publication.idempotency_key.as_deref() else {
                return Response::err("publish-request needs idempotency_key");
            };
            match daemon.orgintel.get(company).await {
                Ok(org) => match daemon
                    .publication
                    .request(
                        &org,
                        company,
                        actor,
                        candidate,
                        audience,
                        start_deadline,
                        expires_at,
                        resources,
                        key,
                    )
                    .await
                {
                    Ok(value) => Response::ok(value),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "publish-authorize" => {
            let Some(id) = request.publication.publication_id.as_deref() else {
                return Response::err("publish-authorize needs publication_id");
            };
            match daemon.orgintel.get(company).await {
                Ok(org) => match daemon.publication.authorize(&org, company, id).await {
                    Ok(value) => Response::ok(value),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "publish-invite" => {
            let Some(id) = request.publication.publication_id.as_deref() else {
                return Response::err("publish-invite needs publication_id");
            };
            let Some(invitation_id) = request.publication.invitation_id.as_deref() else {
                return Response::err("publish-invite needs invitation_id");
            };
            let Some(invitee) = request.publication.invitee.as_deref() else {
                return Response::err("publish-invite needs invitee");
            };
            let expires_at = match request
                .publication
                .publication_expires_at
                .as_deref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            {
                Some(value) => value.with_timezone(&chrono::Utc),
                None => {
                    return Response::err("publish-invite needs RFC3339 publication_expires_at");
                }
            };
            match daemon.orgintel.get(company).await {
                Ok(org) => match daemon
                    .publication
                    .invite(&org, company, id, invitation_id, invitee, expires_at)
                    .await
                {
                    Ok(value) => Response::ok(value),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "publish-revoke" => {
            let Some(invitation_id) = request.publication.invitation_id.as_deref() else {
                return Response::err("publish-revoke needs invitation_id");
            };
            match daemon
                .publication
                .revoke_invitation(company, invitation_id)
                .await
            {
                Ok(value) => Response::ok(value),
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "publish-observe" => {
            let Some(id) = request.publication.publication_id.as_deref() else {
                return Response::err("publish-observe needs publication_id");
            };
            match daemon.publication.observe(company, id).await {
                Ok(value) => Response::ok(value),
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "publish-reconcile" => {
            let Some(id) = request.publication.publication_id.as_deref() else {
                return Response::err("publish-reconcile needs publication_id");
            };
            match daemon.orgintel.get(company).await {
                Ok(org) => match daemon.publication.reconcile(&org, company, id).await {
                    Ok(value) => Response::ok(value),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "publish-stop" => {
            let Some(id) = request.publication.publication_id.as_deref() else {
                return Response::err("publish-stop needs publication_id");
            };
            let Some(reason) = request.publication.stop_reason.as_deref() else {
                return Response::err("publish-stop needs stop_reason");
            };
            match daemon.orgintel.get(company).await {
                Ok(org) => match daemon.publication.stop(&org, company, id, reason).await {
                    Ok(value) => Response::ok(value),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "publish-show" => match daemon
            .publication
            .show(company, request.publication.publication_id.as_deref())
            .await
        {
            Ok(value) => Response::ok(value),
            Err(error) => Response::err(format!("{error:#}")),
        },
        "publish-list" => match daemon.publication.show(company, None).await {
            Ok(value) => Response::ok(value),
            Err(error) => Response::err(format!("{error:#}")),
        },
        // S04-T1. Clone-then-up, so a throwaway is one command rather than a
        // config file someone hand-copies and forgets to strip.
        "up" if request.lifecycle.from_company.is_some() => {
            let from = request
                .lifecycle
                .from_company
                .as_deref()
                .unwrap_or_default();
            match runtime::clone_config(&daemon.root, from, company) {
                Ok(config) => match runtime::up(&config, request.lifecycle.reconcile).await {
                    Ok(message) => {
                        if let Err(error) = materialize_runtime_bridge(daemon, company).await {
                            return Response::err(format!(
                                "cloned container is up but Runtime bridge capability could not be materialized: {error:#}"
                            ));
                        }
                        match daemon.orgintel.get(company).await {
                            Ok(_) => match daemon
                                .authority
                                .initialise_company(
                                    company,
                                    &approval::legacy_config_approvals(&config),
                                )
                                .await
                            {
                                Ok(()) => Response::ok(format!(
                                    "{message} (cloned from {from}, live authority stripped)"
                                )),
                                Err(error) => Response::err(format!(
                                    "container up but Authority initialisation failed: {error:#}"
                                )),
                            },
                            Err(error) => Response::err(format!(
                                "container up but orgintel schema failed: {error:#}"
                            )),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "up" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(mut config) => {
                let _reconcile_guard = if request.lifecycle.reconcile {
                    // An explicit `down` is the owner's instruction to stop
                    // supervised work before replacement. Docker can stop
                    // slightly before the ACP task observes transport close
                    // and releases its in-memory claim. Await that observable
                    // release for a stopped container; otherwise the documented
                    // down -> up --reconcile sequence races itself.
                    if matches!(
                        runtime::status(company).await,
                        Ok(runtime::ContainerStatus::Stopped | runtime::ContainerStatus::Absent)
                    ) && !await_stopped_company_idle(daemon, company).await
                    {
                        return Response::err_kind(
                            "conflict",
                            format!(
                                "{company} is stopped, but supervised actor shutdown did not settle within 30 seconds; inspect the daemon before reconciling"
                            ),
                        );
                    }
                    // Claim the same company-wide slot as an Exec wake before
                    // the image build starts. The build awaits Docker; without
                    // this claim the scheduler could start an Exec in that
                    // window and reconciliation would replace its container.
                    let claimed = daemon
                        .in_flight
                        .lock()
                        .map(|mut running| running.claim(company))
                        .unwrap_or(false);
                    if !claimed {
                        return Response::err_kind(
                            "conflict",
                            format!(
                                "refusing to reconcile {company} while supervised actors are running: exec. Let it finish, or use `restless down -c {company}` to stop the runtime explicitly before reconciling"
                            ),
                        );
                    }
                    let guard = schedule::WakeGuard::new(company, &daemon.in_flight);
                    let staff_running = daemon.staff.running_actors(company);
                    if !staff_running.is_empty() {
                        return Response::err_kind(
                            "conflict",
                            format!(
                                "refusing to reconcile {company} while supervised actors are running: {}. Let them finish, or use `restless down -c {company}` to stop the runtime explicitly before reconciling",
                                staff_running.join(", ")
                            ),
                        );
                    }
                    Some(guard)
                } else {
                    None
                };
                match runtime::up(&config, request.lifecycle.reconcile).await {
                    Ok(message) => {
                        if let Err(error) = materialize_runtime_bridge(daemon, company).await {
                            return Response::err(format!(
                                "container up but Runtime bridge capability could not be materialized: {error:#}"
                            ));
                        }
                        // Company up = environment AND coordination state ready.
                        match daemon.orgintel.get(company).await {
                            Ok(org) => match daemon
                                .authority
                                .import_legacy_company(
                                    company,
                                    &org,
                                    &approval::legacy_config_approvals(&config),
                                )
                                .await
                            {
                                Ok(_) => match approval::purge_legacy_config_approvals(
                                    &daemon.root,
                                    &mut config,
                                ) {
                                    Ok(()) => Response::ok(message),
                                    Err(error) => Response::err(format!(
                                        "Authority ready but legacy config cleanup failed: {error:#}"
                                    )),
                                },
                                Err(error) => Response::err(format!(
                                    "container up but Authority initialisation failed: {error:#}"
                                )),
                            },
                            Err(error) => Response::err(format!(
                                "container up but orgintel schema failed: {error:#}"
                            )),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            Err(error) => Response::err(format!("{error:#}")),
        },
        "down" if request.lifecycle.destroy => {
            // S04-T1. Destroying a live company is not something to make
            // convenient. `_test` is the marker, and the refusal is on the
            // daemon side so no client can decide otherwise.
            if !runtime::is_test_company(company) {
                return Response::err_kind(
                    "authority",
                    format!(
                        "refusing to destroy {company}: only a throwaway company (name ending \
                         `_test`) may be destroyed. Its history is evidence"
                    ),
                );
            }
            let org = match daemon.orgintel.get(company).await {
                Ok(org) => org,
                Err(error) => return Response::err(format!("{error:#}")),
            };
            match runtime::destroy(
                &daemon.root,
                &daemon.orgintel.database_url,
                company,
                &org,
                &daemon.spend,
            )
            .await
            {
                Ok(message) => {
                    daemon.orgintel.forget(company);
                    match daemon.authority.delete_test_company(company).await {
                        Ok(()) => Response::ok(message),
                        Err(error) => Response::err(format!(
                            "test runtime destroyed but Authority cleanup failed: {error:#}"
                        )),
                    }
                }
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "down" => match runtime::down(company).await {
            Ok(message) => Response::ok(message),
            Err(error) => Response::err(format!("{error:#}")),
        },
        "status" => match runtime::status(company).await {
            Ok(status) => Response::ok(format!("{company}: {status:?}")),
            Err(error) => Response::err(format!("{error:#}")),
        },
        "doctor" => match runtime::doctor(company).await {
            Ok(report) => match serde_json::to_value(report) {
                Ok(value) => Response::ok(value),
                Err(error) => Response::err(format!("encode runtime report: {error}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "company-create" => match request.common.body {
            Some(raw) => {
                let path = daemon
                    .root
                    .join("companies")
                    .join(format!("{company}.toml"));
                if path.exists() {
                    return Response::err_kind(
                        "conflict",
                        format!("company {company} already exists at {}", path.display()),
                    );
                }
                match toml::from_str::<runtime::CompanyConfig>(&raw) {
                    Ok(config) if config.name != company => Response::err(format!(
                        "company config name mismatch: command names {company}, file says {}",
                        config.name
                    )),
                    Ok(mut config) => {
                        let initialise = async {
                            runtime::CompanyConfig::save(&daemon.root, &config)?;
                            daemon
                                .authority
                                .initialise_company(
                                    company,
                                    &approval::legacy_config_approvals(&config),
                                )
                                .await
                                .context("initialise company Authority")?;
                            let org = daemon.orgintel.get(company).await?;
                            ensure_standing_actors(&org, Some(&config.model)).await?;
                            approval::purge_legacy_config_approvals(&daemon.root, &mut config)?;
                            Result::<()>::Ok(())
                        }
                        .await;
                        match initialise {
                            Ok(()) => Response::ok(format!("created company {company}")),
                            Err(error) => Response::err(format!(
                                "company initialisation was incomplete: {error:#}"
                            )),
                        }
                    }
                    Err(error) => Response::err(format!("invalid company TOML: {error}")),
                }
            }
            None => Response::err("company-create needs config TOML"),
        },
        "company-show" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => match toml::to_string_pretty(&config) {
                Ok(rendered) => Response::ok(rendered),
                Err(error) => Response::err(format!("render company config: {error}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "company-set" => match (
            request.common.state.as_deref(),
            request.common.body.as_deref(),
        ) {
            (Some(key), Some(value)) => match runtime::CompanyConfig::load(&daemon.root, company) {
                Ok(mut config) => {
                    let result = match key {
                        "mission" => {
                            config.mission = value.to_string();
                            Ok(())
                        }
                        "model" => {
                            config.model = value.to_string();
                            Ok(())
                        }
                        "worker_runtime" => match value.trim() {
                            "omp" => {
                                config.worker_runtime = runtime::WorkerRuntime::Omp;
                                Ok(())
                            }
                            "codex" => {
                                config.worker_runtime = runtime::WorkerRuntime::Codex;
                                Ok(())
                            }
                            _ => Err(anyhow::anyhow!(
                                "worker_runtime must be omp or codex"
                            )),
                        },
                        "reasoning_effort" => {
                            config.reasoning_effort = value.trim().to_string();
                            Ok(())
                        }
                        "model_failover" => {
                            config.model_failover = value
                                .split(',')
                                .map(str::trim)
                                .filter(|model| !model.is_empty())
                                .map(str::to_string)
                                .collect();
                            config.model_candidates().map(|_| ())
                        }
                        "spend_ceiling_usd" => runtime::SpendCeiling::parse(value)
                            .map(|parsed| config.spend_ceiling_usd = parsed),
                        "outcome_standard" => restless_orgintel::OutcomeStandard::parse(value)
                            .ok_or_else(|| anyhow::anyhow!(
                                "outcome_standard must be fast, thorough, exceptional, or frontier"
                            ))
                            .map(|parsed| config.outcome_standard = parsed),
                        _ if key.starts_with("credentials.") => {
                            config.credentials.insert(key[12..].to_string(), value.to_string());
                            Ok(())
                        }
                        _ => Err(anyhow::anyhow!(
                            "unknown company key {key:?}; use mission, model, model_failover, worker_runtime, reasoning_effort, spend_ceiling_usd, outcome_standard, or credentials.<binding>"
                        )),
                    };
                    match result.and_then(|()| runtime::CompanyConfig::save(&daemon.root, &config))
                    {
                        Ok(()) => Response::ok(format!("set {key} for {company}")),
                        Err(error) => Response::err(format!("{error:#}")),
                    }
                }
                Err(error) => Response::err(format!("{error:#}")),
            },
            _ => Response::err("company-set needs key and value"),
        },
        "company-unset" => match request.common.state.as_deref() {
            Some(key) if key.starts_with("credentials.") && key.len() > 12 => {
                match runtime::CompanyConfig::load(&daemon.root, company) {
                    Ok(mut config) => {
                        let binding = &key[12..];
                        if config.credentials.remove(binding).is_none() {
                            Response::err(format!(
                                "company {company} has no credential binding {binding:?}"
                            ))
                        } else {
                            match runtime::CompanyConfig::save(&daemon.root, &config) {
                                Ok(()) => Response::ok(format!("unset {key} for {company}")),
                                Err(error) => Response::err(format!("{error:#}")),
                            }
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            Some(key) => Response::err(format!(
                "cannot unset {key:?}; only credentials.<binding> is removable"
            )),
            None => Response::err("company-unset needs a key"),
        },
        "credential-set" => match (
            request.authority.capability.as_deref(),
            request.common.body.as_deref(),
        ) {
            (Some(binding), Some(reference)) => {
                if let Some(value) = request.authority.secret_value.as_deref() {
                    if let Err(error) = credential::store_reference(reference, value).await {
                        return Response::err(format!("{error:#}"));
                    }
                } else if credential::probe_reference(reference).await.status
                    == credential::ProbeStatus::Invalid
                {
                    // Probe syntax/configuration before persisting. Absence is
                    // a valid reference state; malformed or unsupported is not.
                    let probe = credential::probe_reference(reference).await;
                    return Response::err(
                        probe
                            .detail
                            .unwrap_or_else(|| "invalid credential reference".to_string()),
                    );
                }
                match runtime::CompanyConfig::load(&daemon.root, company) {
                    Ok(mut config) => {
                        config
                            .credentials
                            .insert(binding.to_string(), reference.to_string());
                        match runtime::CompanyConfig::save(&daemon.root, &config) {
                            Ok(()) => Response::ok(format!(
                                "stored reference for {binding}; no secret value was stored"
                            )),
                            Err(error) => Response::err(format!("{error:#}")),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("credential-set needs binding and reference"),
        },
        "credential-promote" => match (
            request.authority.capability.as_deref(),
            request.common.body.as_deref(),
        ) {
            (Some(binding), Some(destination)) => {
                match runtime::CompanyConfig::load(&daemon.root, company) {
                    Ok(mut config) => {
                        let Some(source) = config.credentials.get(binding).cloned() else {
                            return Response::err(format!(
                                "company {company} has no credential binding {binding:?}"
                            ));
                        };
                        match credential::promote_env_to_infisical(&source, destination).await {
                            Ok(()) => {
                                config
                                    .credentials
                                    .insert(binding.to_string(), destination.to_string());
                                match runtime::CompanyConfig::save(&daemon.root, &config) {
                                    Ok(()) => Response::ok(format!(
                                        "promoted bootstrap reference for {binding} into Infisical"
                                    )),
                                    Err(error) => Response::err(format!("{error:#}")),
                                }
                            }
                            Err(error) => Response::err(format!("{error:#}")),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("credential-promote needs binding and destination reference"),
        },
        "credential-check" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => {
                let mut rows = Vec::with_capacity(config.credentials.len());
                for (binding, reference) in &config.credentials {
                    let probe = credential::probe_reference(reference).await;
                    rows.push(serde_json::json!({
                        "binding": binding,
                        "reference": reference,
                        "status": probe.status.as_str(),
                        "detail": probe.detail,
                    }));
                }
                Response::ok(serde_json::Value::Array(rows))
            }
            Err(error) => Response::err(format!("{error:#}")),
        },
        "connected-tools" => match connected_tool::list(daemon.authority.pool(), company).await {
            Ok(connections) => Response::ok_serialized(connections),
            Err(error) => Response::err(format!("{error:#}")),
        },
        "connected-tool-install" | "connected-tool-reconnect" => match (
            request.connected_tool.tool_name.as_deref(),
            request.connected_tool.endpoint.as_deref(),
            request.authority.purpose.as_deref(),
            request.connected_tool.assigned_actor.as_deref(),
            request.connected_tool.work_id.as_deref(),
            request.connected_tool.attempt_id.as_deref(),
            request.orgintel.actor.as_deref(),
        ) {
            (
                Some(name),
                Some(endpoint),
                Some(purpose),
                Some(assigned_actor),
                Some(work),
                Some(attempt),
                Some(requested_by),
            ) if !request.connected_tool.requested_scopes.is_empty() => {
                let work_id = match uuid::Uuid::parse_str(work) {
                    Ok(id) => id,
                    Err(error) => return Response::err(format!("bad Work id: {error}")),
                };
                let attempt_id = match uuid::Uuid::parse_str(attempt) {
                    Ok(id) => id,
                    Err(error) => return Response::err(format!("bad Attempt id: {error}")),
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match connected_tool::begin_oauth_install(
                        &daemon.root,
                        &daemon.authority,
                        &org,
                        company,
                        name,
                        endpoint,
                        purpose,
                        assigned_actor,
                        &request.connected_tool.requested_scopes,
                        work_id,
                        attempt_id,
                        requested_by,
                        request.cmd == "connected-tool-reconnect",
                    )
                    .await
                    {
                        Ok(launch) => Response::ok_serialized(launch),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err(
                "connected-tool install needs name, endpoint, purpose, actor, Work, Attempt and scopes",
            ),
        },
        "connected-tool-observe" => match (
            request.connected_tool.tool_name.as_deref(),
            request.connected_tool.workspace_reference.as_deref(),
            request.orgintel.actor.as_deref(),
        ) {
            (Some(name), Some(workspace), Some(actor))
                if !request.connected_tool.observed_tools.is_empty() =>
            {
                match connected_tool::observe_workspace(
                    daemon.authority.pool(),
                    company,
                    name,
                    actor,
                    workspace,
                    &request.connected_tool.observed_tools,
                )
                .await
                {
                    Ok(connection) => {
                        let _ = daemon
                            .authority
                            .emit(
                                company,
                                "provider_connection_observed",
                                Some(actor),
                                serde_json::json!({
                                    "name": name,
                                    "workspace_reference": workspace,
                                    "observed_tools": request.connected_tool.observed_tools,
                                }),
                            )
                            .await;
                        Response::ok_serialized(connection)
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err(
                "connected-tool observe needs name, workspace identity and observed tools",
            ),
        },
        "connected-tool-disable" => match (
            request.connected_tool.tool_name.as_deref(),
            request.orgintel.actor.as_deref(),
        ) {
            (Some(name), Some(actor)) => {
                match connected_tool::disable(daemon.authority.pool(), company, name).await {
                    Ok(connection) => {
                        let _ = daemon
                            .authority
                            .emit(
                                company,
                                "provider_connection_disabled",
                                Some(actor),
                                serde_json::json!({ "name": name }),
                            )
                            .await;
                        Response::ok_serialized(connection)
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("connected-tool disable needs name and actor"),
        },
        "legal-show" => match legal::get_profile(&daemon.authority, company).await {
            Ok(profile) => Response::ok(serde_json::json!({
                "profile": profile,
                "source": "authority",
            })),
            Err(error) => Response::err(format!("{error:#}")),
        },
        "legal-probe" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => match legal::probe_abr(&daemon.authority, &config).await {
                Ok(profile) => Response::ok_serialized(profile),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "legal-set" => match request.common.body.as_deref() {
            Some(body) => match serde_json::from_str::<legal::LegalProfileInput>(body) {
                Ok(input) => {
                    match legal::set_profile(&daemon.authority, company, input, "owner").await {
                        Ok(profile) => Response::ok_serialized(profile),
                        Err(error) => Response::err(format!("{error:#}")),
                    }
                }
                Err(error) => Response::err(format!("invalid safe legal profile: {error}")),
            },
            None => Response::err("legal-set needs a safe legal profile"),
        },
        "finance-show" => {
            let provider = airwallex::connection(&daemon.authority, company).await;
            let envelopes = finance::envelopes(&daemon.authority, company).await;
            let payments = finance::payments(&daemon.authority, company).await;
            let balances = daemon
                .authority
                .records_of_kind(company, "finance_balance_observed")
                .await;
            match (provider, envelopes, payments, balances) {
                (Ok(provider), Ok(envelopes), Ok(payments), Ok(balances)) => {
                    Response::ok(serde_json::json!({
                        "provider": provider,
                        "envelopes": envelopes,
                        "payments": payments,
                        "last_balance_observation": balances.last().map(|row| serde_json::json!({
                            "observed_at": row.created_at,
                            "body": row.body,
                        })),
                    }))
                }
                (Err(error), _, _, _)
                | (_, Err(error), _, _)
                | (_, _, Err(error), _)
                | (_, _, _, Err(error)) => Response::err(format!("{error:#}")),
            }
        }
        "finance-envelope-set" => match request.common.body.as_deref() {
            Some(body) => match serde_json::from_str::<finance::MoneyEnvelopeInput>(body) {
                Ok(input) => {
                    match finance::set_envelope(&daemon.authority, company, input, "owner").await {
                        Ok(envelope) => Response::ok_serialized(envelope),
                        Err(error) => Response::err(format!("{error:#}")),
                    }
                }
                Err(error) => Response::err(format!("invalid money envelope: {error}")),
            },
            None => Response::err("finance-envelope-set needs an envelope"),
        },
        "finance-freeze" => match request.common.state.as_deref() {
            Some(currency) => match finance::set_frozen(
                &daemon.authority,
                company,
                currency,
                request.authority.apply,
                "owner",
            )
            .await
            {
                Ok(envelope) => Response::ok_serialized(envelope),
                Err(error) => Response::err(format!("{error:#}")),
            },
            None => Response::err("finance-freeze needs a currency"),
        },
        "finance-connect-airwallex" => match request.common.body.as_deref() {
            Some(body) => match serde_json::from_str::<airwallex::ConnectionInput>(body) {
                Ok(input) => {
                    match airwallex::set_connection(&daemon.authority, company, input, "owner")
                        .await
                    {
                        Ok(connection) => Response::ok_serialized(connection),
                        Err(error) => Response::err(format!("{error:#}")),
                    }
                }
                Err(error) => Response::err(format!("invalid Airwallex connection: {error}")),
            },
            None => Response::err("finance-connect-airwallex needs connection evidence"),
        },
        "finance-balances" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => match airwallex::observe_balances(&config, &daemon.authority).await {
                Ok(observation) => Response::ok_serialized(observation),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "finance-probe" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => match airwallex::probe_connection(&config, &daemon.authority).await {
                Ok(probe) => Response::ok_serialized(probe),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "finance-reserve" => match request.common.body.as_deref() {
            Some(body) => match serde_json::from_str::<finance::PaymentIntentInput>(body) {
                Ok(mut input) => {
                    input.requesting_actor = if principal == Principal::Owner {
                        "owner".into()
                    } else {
                        "exec".into()
                    };
                    match daemon.orgintel.get(company).await {
                        Ok(org) => match finance::validate_work_link(&org, &input).await {
                            Ok(()) => {
                                match finance::reserve(&daemon.authority, company, input).await {
                                    Ok(reservation) => Response::ok_serialized(reservation),
                                    Err(error) => Response::err(format!("{error:#}")),
                                }
                            }
                            Err(error) => Response::err(format!("{error:#}")),
                        },
                        Err(error) => Response::err(format!("{error:#}")),
                    }
                }
                Err(error) => Response::err(format!("invalid payment intent: {error}")),
            },
            None => Response::err("finance-reserve needs an exact payment intent"),
        },
        "finance-submit" => match (
            runtime::CompanyConfig::load(&daemon.root, company),
            request.authority.key.as_deref(),
        ) {
            (Ok(config), Some(key)) => {
                match airwallex::submit_reserved(&config, &daemon.authority, key).await {
                    Ok(payment) => Response::ok_serialized(payment),
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            (Err(error), _) => Response::err(format!("{error:#}")),
            (_, None) => Response::err("finance-submit needs an idempotency key"),
        },
        "finance-reconcile" => match (
            runtime::CompanyConfig::load(&daemon.root, company),
            request.authority.key.as_deref(),
        ) {
            (Ok(config), Some(key)) => {
                match airwallex::reconcile_payment(&config, &daemon.authority, key).await {
                    Ok(observation) => {
                        let continuation = continue_after_payment_observation(
                            daemon,
                            company,
                            &observation.payment,
                            observation.changed,
                        )
                        .await;
                        match continuation {
                            Ok(()) => Response::ok_serialized(observation.payment),
                            Err(error) => Response::err(format!(
                                "provider state was recorded but Work continuation failed: {error:#}"
                            )),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            (Err(error), _) => Response::err(format!("{error:#}")),
            (_, None) => Response::err("finance-reconcile needs an idempotency key"),
        },
        // T5 probe: ensure the company schema and report what is in it.
        "orgintel-init" => match (
            daemon.orgintel.get(company).await,
            runtime::CompanyConfig::load(&daemon.root, company),
        ) {
            (Ok(org), Ok(config)) => {
                match ensure_standing_actors(&org, Some(&config.model)).await {
                    Ok(()) => match org.table_names().await {
                        Ok(tables) => Response::ok(serde_json::json!({
                            "schema": org.schema(),
                            "tables": tables,
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            (Err(error), _) | (_, Err(error)) => Response::err(format!("{error:#}")),
        },
        // T4: one Exec wake — rehydrate, work a turn, decide termination.
        // One wake at a time per company, whoever asked: a second exec
        // mid-turn would race the first in the same filesystem. Refuse
        // honestly rather than queue — queuing is machinery nobody needs.
        "wake" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => match daemon.orgintel.get(company).await {
                Ok(org) => {
                    if daemon.staff.is_actor_running(company, "exec") {
                        return Response::err(format!(
                            "Exec is running claimed Work for {company}; its Attempt owns the actor"
                        ));
                    }
                    let cancellation = {
                        let mut claims = daemon.in_flight.lock().expect("in-flight guard");
                        let Some(cancellation) = claims.claim_with_cancellation(company) else {
                            return Response::err(format!(
                                "a wake is already in flight for {company}; \
                                 its outcome lands in the event stream"
                            ));
                        };
                        cancellation
                    };
                    let _guard = schedule::WakeGuard::new(company, &daemon.in_flight);
                    let reason = request
                        .common
                        .reason
                        .as_deref()
                        .unwrap_or("owner-requested wake");
                    match schedule::run_exec_turn(daemon, &config, &org, reason, &cancellation)
                        .await
                    {
                        Ok(report) => match serde_json::to_value(&report) {
                            Ok(value) => Response::ok(value),
                            Err(error) => Response::err(format!("encode report: {error}")),
                        },
                        Err(error) => Response::err(format!("{error:#}")),
                    }
                }
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        // ---- T10: coordination surface — one interface for the owner
        // (host socket) and for agents (container TCP). OrgIntel learns via
        // these reports; it is not authoritative about what happened (§4.7).
        // An owner directive: mail to the Exec. The T6 message trigger
        // turns it into a wake — answering a blocked judgement request is
        // this same command.
        "tell" => match request.common.body {
            Some(body) => match daemon.orgintel.get(company).await {
                Ok(org) => {
                    // The lifecycle path owns standing actors; retain this as
                    // a repair for restored/legacy schemas missing either row.
                    if let Err(error) = ensure_standing_actors(&org, None).await {
                        return Response::err(format!("{error:#}"));
                    }
                    match org.send_message("owner", Some("exec"), &body).await {
                        Ok(_) => {
                            Response::ok("delivered to the exec; the message wakes the company")
                        }
                        Err(error) => Response::err(format!("{error:#}")),
                    }
                }
                Err(error) => Response::err(format!("{error:#}")),
            },
            None => Response::err("tell needs a body"),
        },
        // S04-T9. The owner's reads. AC5 and AC7 ask "which role, which model,
        // what it cost, what it produced" — until now answerable only by
        // reading the Exec's assembled prompt or the database.
        "people" => match daemon.orgintel.get(company).await {
            Ok(org) => {
                let actors = if request.orgintel.include_retired {
                    org.list_actors_including_retired().await
                } else {
                    org.list_actors().await
                };
                match (actors, org.list_work().await) {
                    (Ok(actors), Ok(work)) => {
                        let breakdown = daemon.spend.breakdown(company);
                        let cooldowns = daemon
                            .authority
                            .active_model_cooldowns(company)
                            .await
                            .unwrap_or_default();
                        let rows: Vec<serde_json::Value> = actors
                            .iter()
                            .map(|actor| {
                                // Cost is joined here rather than stored on the
                                // actor: OrgIntel owns role, the ledger owns cost,
                                // and neither becomes a second writer of the other.
                                let spent: f64 = breakdown
                                    .iter()
                                    .filter(|(id, _, _)| id == &actor.id)
                                    .map(|(_, _, usd)| usd)
                                    .sum();
                                let session_running = if actor.id == "exec" {
                                    daemon
                                        .in_flight
                                        .lock()
                                        .map(|running| running.is_active(company))
                                        .unwrap_or(false)
                                } else {
                                    daemon.staff.is_actor_running(company, &actor.id)
                                };
                                let owned_work: Vec<&restless_orgintel::WorkRow> = work
                                    .iter()
                                    .filter(|item| item.owner_id == actor.id)
                                    .collect();
                                serde_json::json!({
                                    "actor_id": actor.id,
                                    "kind": actor.kind,
                                    "role": actor.role,
                                    "display": actor.display,
                                    "model": actor.model,
                                    "team_id": actor.team_id,
                                    "retired_at": actor.retired_at,
                                    "retired_by": actor.retired_by,
                                    "retirement_reason": actor.retirement_reason,
                                    "work_count": owned_work.len(),
                                    "completed_work_count": owned_work.iter().filter(|item| {
                                        item.status == restless_orgintel::WorkStatus::Completed
                                    }).count(),
                                    "recent_work": owned_work.iter().take(5).map(|item| {
                                        serde_json::json!({
                                            "id": item.id,
                                            "title": item.title,
                                            "status": item.status,
                                            "revision": item.revision,
                                        })
                                    }).collect::<Vec<_>>(),
                                    "spent_usd": round_usd(spent),
                                    "session_running": session_running,
                                    "model_cooldown": actor.model.as_deref().and_then(|model| {
                                        cooldowns.iter().find(|cooldown| cooldown.model == model)
                                    }),
                                })
                            })
                            .collect();
                        Response::ok(serde_json::Value::Array(rows))
                    }
                    (Err(error), _) | (_, Err(error)) => Response::err(format!("{error:#}")),
                }
            }
            Err(error) => Response::err(format!("{error:#}")),
        },
        "actor-create" => match (
            request.common.as_actor.as_deref(),
            request.orgintel.role.as_deref(),
            request.orgintel.name.as_deref(),
            request.orgintel.actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(actor_id), Some(role), Some(display), Some(created_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .create_actor(
                            actor_id,
                            role,
                            display,
                            request.orgintel.model.as_deref(),
                            created_by,
                            reason,
                        )
                        .await
                    {
                        Ok(()) => Response::ok(serde_json::json!({
                            "actor_id": actor_id,
                            "role": role,
                            "display": display,
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err(
                "actor create needs --id, --role, --display, --reason, and an acting actor",
            ),
        },
        "actor-model" => match (
            request.common.as_actor.as_deref(),
            request.orgintel.model.as_deref(),
            request.orgintel.actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(actor_id), Some(model), Some(changed_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .change_actor_model(actor_id, model, changed_by, reason)
                        .await
                    {
                        Ok(()) => Response::ok(serde_json::json!({
                            "actor_id": actor_id,
                            "model": model,
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("actor model needs --actor, --model, --reason, and an acting actor"),
        },
        "actor-retire" => match (
            request.common.as_actor.as_deref(),
            request.orgintel.actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(actor_id), Some(retired_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org.retire_actor(actor_id, retired_by, reason).await {
                        Ok(()) => Response::ok(serde_json::json!({
                            "actor_id": actor_id,
                            "retired": true,
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("actor retire needs --actor, --reason, and an acting actor"),
        },
        // ---- teams and the judgement queue below the owner (S06-T4/T5) ----
        "teams" => match daemon.orgintel.get(company).await {
            Ok(org) => match (org.list_teams().await, org.list_actors().await) {
                (Ok(teams), Ok(actors)) => {
                    let rows: Vec<serde_json::Value> = teams
                        .iter()
                        .map(|team| {
                            let members: Vec<serde_json::Value> = actors
                                .iter()
                                .filter(|actor| actor.team_id == Some(team.id))
                                .map(|actor| {
                                    serde_json::json!({
                                        "actor_id": actor.id,
                                        "display": actor.display,
                                        "kind": actor.kind,
                                        "role": actor.role,
                                        "lead": actor.id == team.lead_actor_id,
                                    })
                                })
                                .collect();
                            serde_json::json!({
                                "id": team.id,
                                "name": team.name,
                                "brief": team.brief,
                                "lead_actor_id": team.lead_actor_id,
                                "members": members,
                                "created_at": team.created_at,
                            })
                        })
                        .collect();
                    let unassigned: Vec<&str> = actors
                        .iter()
                        .filter(|actor| actor.team_id.is_none() && actor.kind == "staff")
                        .map(|actor| actor.id.as_str())
                        .collect();
                    Response::ok(serde_json::json!({ "teams": rows, "unassigned": unassigned }))
                }
                (Err(error), _) | (_, Err(error)) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "team-create" => match (
            request.orgintel.name.as_deref(),
            request.common.to.as_deref(),
            request.common.body.as_deref(),
            request.orgintel.actor.as_deref(),
        ) {
            (Some(name), Some(lead), Some(brief), Some(created_by)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => {
                        let config = match runtime::CompanyConfig::load(&daemon.root, company) {
                            Ok(config) => config,
                            Err(error) => return Response::err(format!("{error:#}")),
                        };
                        let standard = match request.orgintel.outcome_standard.as_deref() {
                            Some(value) => match restless_orgintel::OutcomeStandard::parse(value) {
                                Some(value) => value,
                                None => {
                                    return Response::err(
                                        "outcome standard must be fast, thorough, exceptional, or frontier",
                                    );
                                }
                            },
                            None => config.outcome_standard,
                        };
                        let source = match request.orgintel.outcome_standard_source.as_deref() {
                            Some(value) => {
                                match restless_orgintel::OutcomeStandardSource::parse(value) {
                                    Some(value) => value,
                                    None => {
                                        return Response::err(
                                            "standard source must be company_default, owner_override, or owner_language",
                                        );
                                    }
                                }
                            }
                            None => restless_orgintel::OutcomeStandardSource::CompanyDefault,
                        };
                        if source == restless_orgintel::OutcomeStandardSource::CompanyDefault
                            && standard != config.outcome_standard
                        {
                            return Response::err(format!(
                                "company_default is {}; use owner_override or owner_language with its source message to commission {standard}",
                                config.outcome_standard
                            ));
                        }
                        match org
                            .create_team_with_standard(
                                name,
                                brief,
                                lead,
                                created_by,
                                standard,
                                source,
                                request.orgintel.source_message_id,
                            )
                            .await
                        {
                            Ok(id) => Response::ok(serde_json::json!({
                                "team_id": id,
                                "outcome_standard": standard,
                                "outcome_standard_source": source,
                            })),
                            Err(error) => Response::err(format!("{error:#}")),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("team create needs --name, --lead, --brief, and an acting actor"),
        },
        "team-update" => match (
            request.orgintel.name.as_deref(),
            request.orgintel.actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(team), Some(changed_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => match resolve_team(&org, team).await {
                        Ok(id) => match org
                            .update_team(
                                id,
                                request.orgintel.new_name.as_deref(),
                                request.common.body.as_deref(),
                                changed_by,
                                reason,
                            )
                            .await
                        {
                            Ok(()) => Response::ok(serde_json::json!({ "team_id": id })),
                            Err(error) => Response::err(format!("{error:#}")),
                        },
                        Err(error) => Response::err(error),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("team update needs --team, --reason, and an acting actor"),
        },
        "team-assign" => match (
            request.common.as_actor.as_deref(),
            request.orgintel.name.as_deref(),
            request.orgintel.actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(actor), Some(team), Some(changed_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => {
                        let target = if team.eq_ignore_ascii_case("none") {
                            Ok(None)
                        } else {
                            resolve_team(&org, team).await.map(Some)
                        };
                        match target {
                            Ok(target) => {
                                match org.set_actor_team(actor, target, changed_by, reason).await {
                                    Ok(()) => Response::ok(serde_json::json!({
                                        "actor": actor, "team_id": target,
                                    })),
                                    Err(error) => Response::err(format!("{error:#}")),
                                }
                            }
                            Err(error) => Response::err(error),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("team assign needs --actor, --team, --reason, and an acting actor"),
        },
        "team-lead" => match (
            request.orgintel.name.as_deref(),
            request.common.to.as_deref(),
            request.orgintel.actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(team), Some(actor), Some(changed_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => match resolve_team(&org, team).await {
                        Ok(id) => match org.set_team_lead(id, actor, changed_by, reason).await {
                            Ok(()) => {
                                Response::ok(serde_json::json!({ "team_id": id, "lead": actor }))
                            }
                            Err(error) => Response::err(format!("{error:#}")),
                        },
                        Err(error) => Response::err(error),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("team lead needs --team, --actor, --reason, and an acting actor"),
        },
        "team-disband" => match (
            request.orgintel.name.as_deref(),
            request.orgintel.actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(team), Some(changed_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => match resolve_team(&org, team).await {
                        Ok(id) => match org.disband_team(id, changed_by, reason).await {
                            Ok(stranded) => Response::ok(serde_json::json!({
                                "team_id": id,
                                // Judgement the team still owed did not vanish with
                                // it: it fell through to the Exec, recorded.
                                "reassigned_judgements": stranded,
                            })),
                            Err(error) => Response::err(format!("{error:#}")),
                        },
                        Err(error) => Response::err(error),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("team disband needs --team, --reason, and an acting actor"),
        },
        "judgement" => match request.common.as_actor.as_deref() {
            Some(actor) => match daemon.orgintel.get(company).await {
                Ok(org) => match org.handoffs_assigned_to(actor).await {
                    Ok(rows) => Response::ok(serde_json::to_value(rows).unwrap_or_default()),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            },
            None => Response::err("judgement needs --as <actor>"),
        },
        "work-handoff-escalate" => match (
            request
                .common
                .id
                .as_deref()
                .map(uuid::Uuid::parse_str)
                .transpose(),
            request.common.as_actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Ok(Some(id)), Some(actor), Some(reason)) => match daemon.orgintel.get(company).await {
                Ok(org) => match org.escalate_handoff(id, actor, reason).await {
                    Ok(()) => Response::ok(serde_json::json!({
                        "handoff_id": id, "escalated_from": actor,
                        "now_owed_by": if actor == "exec" { "owner" } else { "exec" },
                    })),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            },
            (Err(error), _, _) => Response::err(format!("bad handoff id: {error}")),
            _ => Response::err("escalate needs --handoff, --as and --reason"),
        },
        "receipts" => match daemon.authority.records_of_kind(company, "effect").await {
            Ok(events) => {
                let wanted = request.authority.capability.as_deref();
                let limit = request.common.limit.unwrap_or(50).max(1) as usize;
                let rows: Vec<serde_json::Value> = events
                    .iter()
                    .rev()
                    .filter(|event| wanted.is_none_or(|class| {
                        event.body.get("effect_class").or_else(|| event.body.get("capability"))
                            .and_then(serde_json::Value::as_str) == Some(class)
                    }))
                    .take(limit)
                    .map(|event| {
                        let evidence_quality = if reconcile::is_governed_receipt(&event.body) {
                            "governed"
                        } else {
                            "legacy_unverified"
                        };
                        serde_json::json!({
                            "effect_class": event.body.get("effect_class").or_else(|| event.body.get("capability")),
                            "tool": event.body["tool"],
                            "success": event.body["success"],
                            "party": event.body["party"],
                            "actor": event.body["actor"],
                            "outcome": event.body["outcome"],
                            "idempotency_key": event.body["idempotency_key"],
                            "evidence_quality": evidence_quality,
                            "at": event.created_at,
                        })
                    })
                    .collect();
                Response::ok(serde_json::Value::Array(rows))
            }
            Err(error) => Response::err(format!("{error:#}")),
        },
        // Spend is Authority-owned truth. Role labels are an OrgIntel
        // projection and must not make a ceiling, poison, or exact charge
        // unreadable when organisational state is unavailable or saturated.
        "spend" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => {
                let by_actor = daemon.spend.breakdown(company);
                let budget = daemon.spend.budget_state(&config);
                let accounted = budget.accounted_micro_usd() as f64 / 1_000_000.0;
                let status = match budget {
                    spend::ModelBudgetState::Available { .. } => "available",
                    spend::ModelBudgetState::Exhausted { .. } => "exhausted",
                    spend::ModelBudgetState::MeteringUnknown { .. } => "metering_unknown",
                };
                let rows: Vec<serde_json::Value> = by_actor
                    .into_iter()
                    .map(|(actor, model, usd)| {
                        serde_json::json!({
                            "actor": actor,
                            "role": serde_json::Value::Null,
                            "model": model,
                            "spent_usd": round_usd(usd),
                        })
                    })
                    .collect();
                Response::ok(serde_json::json!({
                    "accounted_usd": round_usd(accounted),
                    "ceiling_usd": config.spend_ceiling_usd.as_usd(),
                    "remaining_usd": budget.remaining_micro_usd().map(|remaining| round_usd(remaining as f64 / 1_000_000.0)),
                    "status": status,
                    "note": if status == "metering_unknown" {
                        serde_json::json!(
                            "fail-closed: a provider stream could not be exactly accounted, so charged work is paused until model metering is reconciled. `accounted_usd` remains the real known spend."
                        )
                    } else { serde_json::Value::Null },
                    "by_actor": rows,
                }))
            }
            Err(error) => Response::err(format!("{error:#}")),
        },
        "spend-correct" => {
            if let Err(error) = runtime::CompanyConfig::load(&daemon.root, company) {
                return Response::err(format!("{error:#}"));
            }
            let Some(correction_id) = request.authority.correction_id.as_deref() else {
                return Response::err("spend-correct needs --correction-id");
            };
            let Ok(correction_id) = uuid::Uuid::parse_str(correction_id) else {
                return Response::err("spend-correct correction id must be a UUID");
            };
            let request_ids = match request
                .authority
                .request_ids
                .iter()
                .map(|request_id| uuid::Uuid::parse_str(request_id))
                .collect::<std::result::Result<Vec<_>, _>>()
            {
                Ok(request_ids) => request_ids,
                Err(_) => return Response::err("spend-correct request ids must be UUIDs"),
            };
            let Some(delta_micro_usd) = request.authority.delta_micro_usd else {
                return Response::err("spend-correct needs --delta-micro-usd");
            };
            let Some(reason) = request.common.reason.as_deref() else {
                return Response::err("spend-correct needs --reason");
            };
            if request.authority.apply {
                match daemon.spend.correct(
                    correction_id,
                    company,
                    &request_ids,
                    delta_micro_usd,
                    reason,
                    principal.as_str(),
                ) {
                    Ok((correction, preview)) => Response::ok(serde_json::json!({
                        "applied": true,
                        "spool_written": true,
                        "correction": correction,
                        "current_total_micro_usd": preview.current_total_micro_usd,
                        "post_correction_total_micro_usd": preview.post_correction_total_micro_usd,
                    })),
                    Err(error) => Response::err(format!("{error:#}")),
                }
            } else {
                match daemon.spend.preview_correction(
                    correction_id,
                    company,
                    &request_ids,
                    delta_micro_usd,
                    reason,
                    principal.as_str(),
                ) {
                    Ok(preview) => Response::ok(serde_json::json!({
                        "applied": false,
                        "spool_written": false,
                        "preview": preview,
                    })),
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
        }
        "goals" => match daemon.orgintel.get(company).await {
            Ok(org) => match org.list_goals().await {
                Ok(goals) => Response::ok(serde_json::to_value(goals).unwrap_or_default()),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "goal-add" => match (
            request.orgintel.title.as_deref(),
            request.common.body.as_deref(),
            request.orgintel.actor.as_deref(),
        ) {
            (Some(title), Some(body), Some(actor)) => match daemon.orgintel.get(company).await {
                Ok(org) => match org.add_goal(title, body, actor).await {
                    Ok(goal_id) => Response::ok(serde_json::json!({ "goal_id": goal_id })),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            },
            _ => Response::err("goal-add needs title, body and actor attribution"),
        },
        "work-goal" => match (
            request.common.id.as_deref(),
            request.orgintel.goal.as_deref(),
            request.orgintel.actor.as_deref(),
        ) {
            (Some(work_id), Some(goal_id), Some(actor)) => {
                let work_id = match uuid::Uuid::parse_str(work_id) {
                    Ok(work_id) => work_id,
                    Err(error) => return Response::err(format!("bad Work id: {error}")),
                };
                let goal_id = match uuid::Uuid::parse_str(goal_id) {
                    Ok(goal_id) => goal_id,
                    Err(error) => return Response::err(format!("bad Goal id: {error}")),
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org.set_work_goal(work_id, goal_id, actor).await {
                        Ok(previous_goal_id) => Response::ok(serde_json::json!({
                            "work_id": work_id,
                            "goal_id": goal_id,
                            "previous_goal_id": previous_goal_id,
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("work-goal needs Work id, Goal id and actor attribution"),
        },
        "work" => match daemon.orgintel.get(company).await {
            Ok(org) => match org.list_work().await {
                Ok(work) => Response::ok(serde_json::to_value(work).unwrap_or_default()),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "work-graph" => match daemon.orgintel.get(company).await {
            Ok(org) => match org.work_graph_snapshot().await {
                Ok(graph) => Response::ok(serde_json::to_value(graph).unwrap_or_default()),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "telemetry" => match daemon.orgintel.get(company).await {
            Ok(org) => match telemetry::collect(company, &org, &daemon.spend).await {
                Ok(report) => Response::ok(serde_json::to_value(report).unwrap_or_default()),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "work-attempts" => match daemon.orgintel.get(company).await {
            Ok(org) => {
                let work_id = match request
                    .common
                    .id
                    .as_deref()
                    .map(uuid::Uuid::parse_str)
                    .transpose()
                {
                    Ok(value) => value,
                    Err(error) => return Response::err(format!("bad Work id: {error}")),
                };
                match org.list_work_attempts(work_id).await {
                    Ok(rows) => Response::ok(serde_json::to_value(rows).unwrap_or_default()),
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            Err(error) => Response::err(format!("{error:#}")),
        },
        "work-assign" => match (
            request
                .common
                .id
                .as_deref()
                .map(uuid::Uuid::parse_str)
                .transpose(),
            request.common.to.as_deref(),
            request.orgintel.actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Ok(Some(work_id)), Some(new_owner), Some(changed_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => {
                        let staff_lead = match org.team_lead_for(new_owner).await {
                            Ok(Some(lead)) => lead,
                            Ok(None) => {
                                return Response::err(format!(
                                    "new Work owner {new_owner:?} must be Staff under an accountable lead; Exec, unassigned actors, and team leads cannot own production Work"
                                ));
                            }
                            Err(error) => return Response::err(format!("{error:#}")),
                        };
                        match org
                            .reassign_work(work_id, new_owner, changed_by, reason)
                            .await
                        {
                            Ok(previous_owner) => Response::ok(serde_json::json!({
                                "work_id": work_id,
                                "from_actor_id": previous_owner,
                                "to_actor_id": new_owner,
                                "accountable_lead_id": staff_lead,
                            })),
                            Err(error) => Response::err(format!("{error:#}")),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            (Err(error), _, _, _) => Response::err(format!("bad Work id: {error}")),
            _ => Response::err("work-assign needs Work id, new owner, reason and acting actor"),
        },
        "work-add" => match (
            request.orgintel.actor.as_deref(),
            request.orgintel.role.as_deref(),
            request.orgintel.title.as_deref(),
            request.common.body.as_deref(),
            request.common.as_actor.as_deref(),
        ) {
            (Some(owner), Some(role), Some(title), Some(outcome), Some(commissioned_by)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => {
                        let accountable_lead = match org.team_lead_for(owner).await {
                            Ok(Some(lead)) => lead,
                            Ok(None) => {
                                return Response::err(format!(
                                    "Work owner {owner:?} must be Staff under an accountable lead; Exec, unassigned actors, and team leads cannot own production Work"
                                ));
                            }
                            Err(error) => return Response::err(format!("{error:#}")),
                        };
                        let actor = match org.active_actor(owner).await {
                            Ok(Some(actor)) => actor,
                            Ok(None) => {
                                return Response::err(format!(
                                    "Work owner {owner:?} is not an existing active actor; inspect `restless people` and commission one stable specialist if none fits"
                                ));
                            }
                            Err(error) => return Response::err(format!("{error:#}")),
                        };
                        let requested_topology = request
                            .orgintel
                            .producing_topology
                            .as_deref()
                            .unwrap_or("coherent_single_worker");
                        let producing_topology = match restless_orgintel::ProducingTopology::parse(
                            requested_topology,
                        ) {
                            Some(topology) => topology,
                            None => {
                                return Response::err(format!(
                                    "unknown producing topology {requested_topology:?}; use coherent-single-worker or locally-closing-parallel-unit"
                                ));
                            }
                        };
                        let active_workers = if commissioned_by == "exec" {
                            let Some(team_id) = actor.team_id else {
                                return Response::err(format!(
                                    "Work owner {owner:?} has no active accountable team"
                                ));
                            };
                            let active_workers = match org.list_actors().await {
                                Ok(actors) => actors
                                    .into_iter()
                                    .filter(|member| {
                                        member.team_id == Some(team_id)
                                            && member.id != accountable_lead
                                    })
                                    .map(|member| member.id)
                                    .collect::<Vec<_>>(),
                                Err(error) => return Response::err(format!("{error:#}")),
                            };
                            active_workers
                        } else {
                            Vec::new()
                        };
                        if !work_commission_is_admitted(
                            commissioned_by,
                            &accountable_lead,
                            producing_topology,
                            &active_workers,
                            owner,
                        ) {
                            return Response::err(format!(
                                "production Work for {owner:?} must be commissioned by accountable lead {accountable_lead:?}; Exec may route coherent-single-worker only when that team has exactly one active non-lead worker"
                            ));
                        }
                        if actor.role != role {
                            return Response::err(format!(
                                "Work requested role {role:?}, but durable actor {owner:?} has role {:?}; reuse the actor's recorded role",
                                actor.role
                            ));
                        }
                        if let Some(requested_model) = request.orgintel.model.as_deref() {
                            if actor.model.as_deref() != Some(requested_model) {
                                return Response::err(format!(
                                    "Work requested model {requested_model:?}, but durable actor {owner:?} uses {:?}; model changes belong to the actor/session, not a Work assignment",
                                    actor.model
                                ));
                            }
                        }
                        let goal_id = match request.orgintel.goal.as_deref() {
                            Some(goal_id) => match uuid::Uuid::parse_str(goal_id) {
                                Ok(goal_id) => Some(goal_id),
                                Err(error) => {
                                    return Response::err(format!("bad Goal id: {error}"));
                                }
                            },
                            None => None,
                        };
                        let work = restless_orgintel::NewWork {
                            owner_id: owner,
                            title,
                            outcome,
                            goal_id,
                            priority: request.orgintel.priority.unwrap_or(0),
                            expected_artifact: request
                                .orgintel
                                .expected_artifact
                                .as_deref()
                                .unwrap_or(""),
                            workspace: restless_orgintel::WorkspaceSpec {
                                repo: request.orgintel.repo,
                                base_ref: request.orgintel.base_ref,
                                integration_branch: request.orgintel.integration_branch,
                                worktree: request.orgintel.worktree,
                            },
                            attempt_limit: request.orgintel.attempt_limit,
                        };
                        let parse_edges = |values: &[String], label: &str| {
                            values
                                .iter()
                                .map(|value| {
                                    uuid::Uuid::parse_str(value).map_err(|error| {
                                        format!("bad {label} Work id {value:?}: {error}")
                                    })
                                })
                                .collect::<Result<Vec<_>, _>>()
                        };
                        let requires = match parse_edges(&request.orgintel.requires, "required") {
                            Ok(values) => values,
                            Err(error) => return Response::err(error),
                        };
                        let revises = match parse_edges(&request.orgintel.revises, "revised") {
                            Ok(values) => values,
                            Err(error) => return Response::err(error),
                        };
                        let gates = request
                            .orgintel
                            .gates
                            .iter()
                            .map(|gate| restless_orgintel::InitialWorkGate {
                                name: &gate.name,
                                command: &gate.command,
                                stage: &gate.stage,
                                timeout_seconds: gate.timeout_seconds,
                                resources: &gate.resources,
                            })
                            .collect::<Vec<_>>();
                        let added = if let Some(contracts) =
                            request.orgintel.constitution_contracts.as_ref()
                        {
                            org.add_commissioned_work_with_constitution(
                                work,
                                &requires,
                                &revises,
                                &gates,
                                request.orgintel.owner_review,
                                request.orgintel.source_message_id,
                                commissioned_by,
                                producing_topology,
                                contracts,
                            )
                            .await
                        } else {
                            org.add_commissioned_work_with_edges_and_gates(
                                work,
                                &requires,
                                &revises,
                                &gates,
                                request.orgintel.owner_review,
                                request.orgintel.source_message_id,
                                commissioned_by,
                                producing_topology,
                            )
                            .await
                        };
                        match added {
                            Ok(id) => Response::ok(serde_json::json!({
                                "work_id": id,
                                "accountable_lead_id": accountable_lead,
                                "producer_actor_id": owner,
                                "commissioned_by": commissioned_by,
                                "producing_topology": producing_topology,
                            })),
                            Err(error) => Response::err(format!("{error:#}")),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err(
                "work-add needs Staff owner, role, title, outcome, and accountable lead attribution",
            ),
        },
        "work-edge" => match (
            request.common.from.as_deref(),
            request.common.to.as_deref(),
            request.orgintel.kind.as_deref(),
        ) {
            (Some(from), Some(to), Some(kind)) => {
                let (Ok(from), Ok(to)) = (uuid::Uuid::parse_str(from), uuid::Uuid::parse_str(to))
                else {
                    return Response::err("work-edge needs UUID from/to");
                };
                let kind = match kind {
                    "requires" => restless_orgintel::WorkEdgeKind::Requires,
                    "revises" => restless_orgintel::WorkEdgeKind::Revises,
                    other => {
                        return Response::err(format!(
                            "edge kind must be requires|revises, got {other:?}"
                        ));
                    }
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => {
                        if request.owner.action.as_deref() == Some("remove") {
                            let Some(changed_by) = request.common.as_actor.as_deref() else {
                                return Response::err("removing a Work edge needs --as");
                            };
                            let Some(reason) = request.common.reason.as_deref() else {
                                return Response::err("removing a Work edge needs --reason");
                            };
                            match org
                                .remove_work_edge(from, to, kind, changed_by, reason)
                                .await
                            {
                                Ok(()) => Response::ok("removed"),
                                Err(error) => Response::err(format!("{error:#}")),
                            }
                        } else {
                            match org.add_work_edge(from, to, kind).await {
                                Ok(()) => Response::ok("recorded"),
                                Err(error) => Response::err(format!("{error:#}")),
                            }
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("work-edge needs from, to and kind"),
        },
        "work-artifact" => match (
            request.common.id.as_deref(),
            request.orgintel.attempt.as_deref(),
            request.orgintel.kind.as_deref(),
            request.orgintel.uri.as_deref(),
        ) {
            (Some(work), Some(attempt), Some(kind), Some(uri)) => {
                let (Ok(work_id), Ok(attempt_id)) =
                    (uuid::Uuid::parse_str(work), uuid::Uuid::parse_str(attempt))
                else {
                    return Response::err("work-artifact needs UUID work and attempt");
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .link_work_artifact(restless_orgintel::NewArtifactRef {
                            kind,
                            uri,
                            note: request.common.body.as_deref().unwrap_or(""),
                            created_by: request.orgintel.actor.as_deref().unwrap_or("owner"),
                            work_id: Some(work_id),
                            attempt_id: Some(attempt_id),
                            digest: request.orgintel.digest.as_deref(),
                            source_commit: request.orgintel.source_commit.as_deref(),
                            runtime_generation: None,
                            label: request.orgintel.label.as_deref().unwrap_or("output"),
                        })
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({ "artifact_ref_id": id })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("work-artifact needs work, attempt, kind and uri"),
        },
        "work-artifact-retire" => match (
            request.common.id.as_deref(),
            request.common.reason.as_deref(),
            request.orgintel.actor.as_deref(),
        ) {
            (Some(artifact), Some(reason), Some(actor)) => {
                let Ok(artifact_id) = uuid::Uuid::parse_str(artifact) else {
                    return Response::err("work-artifact-retire needs a UUID artifact");
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org.retire_work_artifact(artifact_id, actor, reason).await {
                        Ok(changed) => Response::ok(serde_json::json!({
                            "artifact_ref_id": artifact_id,
                            "retired": changed,
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("work-artifact-retire needs artifact, actor and reason"),
        },
        "work-gate" => match (
            request.common.id.as_deref(),
            request.orgintel.name.as_deref(),
            request.orgintel.cwd.as_deref(),
            request.orgintel.argv.as_deref(),
        ) {
            (Some(work), Some(name), Some(cwd), Some(argv)) => {
                let Ok(work_id) = uuid::Uuid::parse_str(work) else {
                    return Response::err("bad Work id");
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .add_work_gate(restless_orgintel::NewWorkGate {
                            work_id,
                            name,
                            cwd,
                            command: argv,
                            created_by: request.orgintel.actor.as_deref().unwrap_or("owner"),
                        })
                        .await
                    {
                        Ok(id) => match org
                            .configure_work_gate(
                                id,
                                request.orgintel.stage.as_deref().unwrap_or("cumulative"),
                                request.orgintel.timeout_seconds.unwrap_or(900),
                                &request.orgintel.resources,
                            )
                            .await
                        {
                            Ok(()) => Response::ok(serde_json::json!({ "gate_id": id })),
                            Err(error) => Response::err(format!("{error:#}")),
                        },
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("work-gate needs work, name, cwd and command"),
        },
        "work-gate-retire" => match (
            request.common.id.as_deref(),
            request.common.as_actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(gate), Some(actor), Some(reason)) => {
                let Ok(gate_id) = uuid::Uuid::parse_str(gate) else {
                    return Response::err("bad Work gate id");
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org.retire_work_gate(gate_id, actor, reason).await {
                        Ok(retired) => Response::ok(serde_json::json!({
                            "gate_id": gate_id,
                            "retired": retired,
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("work-gate-retire needs gate, actor and reason"),
        },
        "work-handoff" => match (
            request.common.id.as_deref(),
            request.owner.category.as_deref(),
            request.owner.action.as_deref(),
            request.owner.prepared.as_deref(),
            request.owner.resume_when.as_deref(),
        ) {
            (Some(work), Some(category), Some(action), Some(prepared), Some(resume_when)) => {
                let Ok(work_id) = uuid::Uuid::parse_str(work) else {
                    return Response::err("bad Work id");
                };
                let attempt_id = match request
                    .orgintel
                    .attempt
                    .as_deref()
                    .map(uuid::Uuid::parse_str)
                    .transpose()
                {
                    Ok(value) => value,
                    Err(error) => return Response::err(format!("bad Attempt id: {error}")),
                };
                // Owner-facing prose and CLI help use kebab-case; the stored
                // protocol historically used snake_case. Accept both spellings
                // at this boundary so an irreducible handoff is not blocked by
                // serialization punctuation.
                let normalized_category = category.replace('-', "_");
                let category = match normalized_category.as_str() {
                    "identity" => restless_orgintel::OwnerHandoffCategory::Identity,
                    "captcha" => restless_orgintel::OwnerHandoffCategory::Captcha,
                    "mfa" => restless_orgintel::OwnerHandoffCategory::Mfa,
                    "legal_attestation" => {
                        restless_orgintel::OwnerHandoffCategory::LegalAttestation
                    }
                    "payment_confirmation" => {
                        restless_orgintel::OwnerHandoffCategory::PaymentConfirmation
                    }
                    "owner_judgement" => restless_orgintel::OwnerHandoffCategory::OwnerJudgement,
                    _ => {
                        return Response::err(format!("unsupported handoff category {category:?}"));
                    }
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .request_owner_handoff(restless_orgintel::NewOwnerHandoff {
                            work_id,
                            attempt_id,
                            requested_by: request.orgintel.actor.as_deref().unwrap_or("owner"),
                            category,
                            requested_action: action,
                            prepared_state: prepared,
                            resume_condition: resume_when,
                        })
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({ "handoff_id": id })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err(
                "work-handoff needs work, category, action, prepared and resume condition",
            ),
        },
        "work-handoff-refresh" => match (
            request.common.id.as_deref(),
            request.common.as_actor.as_deref(),
            request.owner.action.as_deref(),
            request.owner.prepared.as_deref(),
            request.owner.resume_when.as_deref(),
        ) {
            (Some(id), Some(changed_by), Some(action), Some(prepared), Some(resume_when)) => {
                let Ok(id) = uuid::Uuid::parse_str(id) else {
                    return Response::err("bad handoff id");
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .refresh_owner_handoff(id, changed_by, action, prepared, resume_when)
                        .await
                    {
                        Ok(()) => Response::ok(serde_json::json!({
                            "handoff_id": id,
                            "refreshed_by": changed_by,
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err(
                "work-handoff-refresh needs handoff, action, prepared, resume condition and --as",
            ),
        },
        "work-handoff-prepare-brief" => match (
            request.common.id.as_deref(),
            request.common.as_actor.as_deref(),
            request.owner.owner_kind.as_deref(),
            request.owner.headline.as_deref(),
            request.owner.situation.as_deref(),
            request.owner.impact.as_deref(),
            request.owner.recommendation.as_deref(),
            request.owner.no_action.as_deref(),
        ) {
            (
                Some(id),
                Some(briefed_by),
                Some(kind),
                Some(headline),
                Some(situation),
                Some(impact),
                Some(recommendation),
                Some(no_action),
            ) => {
                let Ok(id) = uuid::Uuid::parse_str(id) else {
                    return Response::err("bad handoff id");
                };
                let kind = match kind {
                    "outcome_review" => restless_orgintel::OwnerBriefKind::OutcomeReview,
                    "decision" => restless_orgintel::OwnerBriefKind::Decision,
                    "blocker" => restless_orgintel::OwnerBriefKind::Blocker,
                    "opportunity" => restless_orgintel::OwnerBriefKind::Opportunity,
                    "contradiction" => restless_orgintel::OwnerBriefKind::Contradiction,
                    "human_step" => restless_orgintel::OwnerBriefKind::HumanStep,
                    other => {
                        return Response::err(format!("unsupported owner brief kind {other:?}"));
                    }
                };
                let brief = restless_orgintel::OwnerBrief {
                    kind,
                    headline: headline.to_string(),
                    situation: situation.to_string(),
                    impact: impact.to_string(),
                    recommendation: recommendation.to_string(),
                    no_action: no_action.to_string(),
                    uncertainty: request.owner.uncertainty.clone(),
                    deadline: request.owner.deadline.clone(),
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org.prepare_owner_brief(id, briefed_by, brief).await {
                        Ok(()) => Response::ok(serde_json::json!({
                            "handoff_id": id,
                            "briefed_by": briefed_by,
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err(
                "work-handoff-prepare-brief needs handoff, kind, headline, situation, impact, recommendation, no-action and --as",
            ),
        },
        "work-handoff-resolve" => match (
            request.common.id.as_deref(),
            request.common.state.as_deref(),
            request.common.resolution.as_deref(),
            request.common.as_actor.as_deref(),
        ) {
            (Some(id), Some(state), Some(resolution), Some(resolved_by))
                if !resolution.trim().is_empty() =>
            {
                let Ok(id) = uuid::Uuid::parse_str(id) else {
                    return Response::err("bad handoff id");
                };
                let state = match state {
                    "resolved" => restless_orgintel::OwnerHandoffState::Resolved,
                    "declined" => restless_orgintel::OwnerHandoffState::Declined,
                    "withdrawn" => restless_orgintel::OwnerHandoffState::Withdrawn,
                    other => {
                        return Response::err(format!(
                            "handoff state must be resolved|declined|withdrawn, got {other:?}"
                        ));
                    }
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .resolve_handoff_as(id, resolved_by, state, resolution)
                        .await
                    {
                        Ok(()) => Response::ok("recorded"),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("work-handoff-resolve needs handoff, state, resolution and --as"),
        },
        "work-interrupt" => match (
            request
                .common
                .id
                .as_deref()
                .map(uuid::Uuid::parse_str)
                .transpose(),
            request.common.as_actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Ok(Some(id)), Some(actor), Some(reason)) => match daemon.orgintel.get(company).await {
                Ok(org) => {
                    let owner = match org.get_work(id).await {
                        Ok(Some(work)) => work.owner_id,
                        Ok(None) => return Response::err("Work does not exist"),
                        Err(error) => return Response::err(format!("{error:#}")),
                    };
                    match org.request_attempt_interrupt(id, actor, reason).await {
                        Ok(attempt_id) => {
                            let signalled = daemon.staff.interrupt_work(company, &owner, id);
                            Response::ok(serde_json::json!({
                                "work_id": id, "attempt_id": attempt_id,
                                "requested_by": actor, "signalled": signalled,
                            }))
                        }
                        Err(error) => Response::err(format!("{error:#}")),
                    }
                }
                Err(error) => Response::err(format!("{error:#}")),
            },
            (Err(error), _, _) => Response::err(format!("bad Work id: {error}")),
            _ => Response::err("work-interrupt needs work, reason and --as"),
        },
        "work-resume" => match (
            request
                .common
                .id
                .as_deref()
                .map(uuid::Uuid::parse_str)
                .transpose(),
            request.common.as_actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Ok(Some(id)), Some(actor), Some(reason)) => match daemon.orgintel.get(company).await {
                Ok(org) => match org.resume_work(id, actor, reason).await {
                    Ok(()) => Response::ok(serde_json::json!({
                        "work_id": id, "resumed_by": actor, "repair": reason,
                    })),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            },
            (Err(error), _, _) => Response::err(format!("bad Work id: {error}")),
            _ => Response::err("work-resume needs work, reason and --as"),
        },
        "work-abandon" => match (
            request
                .common
                .id
                .as_deref()
                .map(uuid::Uuid::parse_str)
                .transpose(),
            request.common.as_actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Ok(Some(id)), Some(actor), Some(reason)) => match daemon.orgintel.get(company).await {
                Ok(org) => match org.abandon_work(id, actor, reason).await {
                    Ok(()) => Response::ok(serde_json::json!({
                        "work_id": id, "abandoned_by": actor, "reason": reason,
                    })),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            },
            (Err(error), _, _) => Response::err(format!("bad Work id: {error}")),
            _ => Response::err("work-abandon needs work, reason and --as"),
        },
        "work-review" => match (
            request.common.id.as_deref(),
            request.common.state.as_deref(),
        ) {
            (Some(id), Some(state)) => {
                let Ok(id) = uuid::Uuid::parse_str(id) else {
                    return Response::err("bad handoff id");
                };
                let decision = match state {
                    "accept" => restless_orgintel::OwnerReviewDecision::Accepted,
                    "request_changes" => restless_orgintel::OwnerReviewDecision::ChangesRequested,
                    other => {
                        return Response::err(format!(
                            "review decision must be accept|request_changes, got {other:?}"
                        ));
                    }
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .decide_owner_review(
                            id,
                            decision,
                            request.common.resolution.as_deref().unwrap_or(""),
                        )
                        .await
                    {
                        Ok(()) => Response::ok("recorded"),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("work-review needs handoff and accept|request_changes decision"),
        },
        // Reading your own inbox marks read; inspecting another actor's
        // (--as) does not — an observer must not hide mail from its
        // addressee. A company actor carries its durable actor id, so its
        // self-read can also prove which live Attempt actually received a
        // Work-linked message. This is ordinary inbox delivery, not a second
        // Work/message protocol.
        "inbox" => match daemon.orgintel.get(company).await {
            Ok(org) => {
                let self_read_actor = if principal == Principal::CompanyExec {
                    request.orgintel.actor.as_deref().filter(|actor| {
                        request
                            .common
                            .as_actor
                            .as_deref()
                            .is_none_or(|requested| requested == *actor)
                    })
                } else {
                    None
                };
                let result = match self_read_actor {
                    Some(actor) => org.consume_inbox_for_actor(actor).await,
                    None => org.inbox(request.common.as_actor.as_deref()).await,
                };
                match result {
                    Ok(messages) => {
                        if self_read_actor.is_none() && request.common.as_actor.is_none() {
                            for message in &messages {
                                if let Err(error) = org.mark_read(message.id).await {
                                    tracing::warn!("mark_read {}: {error:#}", message.id);
                                }
                            }
                        }
                        Response::ok(serde_json::to_value(messages).unwrap_or_default())
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            Err(error) => Response::err(format!("{error:#}")),
        },
        "message" => match (request.common.from, request.common.body) {
            (Some(from), Some(body)) => match daemon.orgintel.get(company).await {
                Ok(org) => {
                    let result = match request.common.id.as_deref() {
                        Some(work_id) => {
                            let Ok(work_id) = uuid::Uuid::parse_str(work_id) else {
                                return Response::err("bad Work id on message");
                            };
                            match request.common.to.as_deref() {
                                Some(to) => org.send_work_message(&from, to, work_id, &body).await,
                                None => org.send_work_message_to_owner(&from, work_id, &body).await,
                            }
                        }
                        None => {
                            org.send_message(&from, request.common.to.as_deref(), &body)
                                .await
                        }
                    };
                    match result {
                        Ok(id) => Response::ok(serde_json::json!({ "message_id": id })),
                        Err(error) => Response::err(format!("{error:#}")),
                    }
                }
                Err(error) => Response::err(format!("{error:#}")),
            },
            _ => Response::err("message needs from and body"),
        },
        "schedule-list" => match daemon.orgintel.get(company).await {
            Ok(org) => match org
                .list_schedules(
                    request.common.as_actor.as_deref(),
                    request.orgintel.include_fired,
                )
                .await
            {
                Ok(schedules) => Response::ok_serialized(schedules),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "schedule-history" => match request.common.id.as_deref() {
            Some(id) => {
                let schedule_id = match uuid::Uuid::parse_str(id) {
                    Ok(id) => id,
                    Err(error) => return Response::err(format!("bad schedule id: {error}")),
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .list_schedule_occurrences(schedule_id, request.common.limit.unwrap_or(20))
                        .await
                    {
                        Ok(occurrences) => Response::ok_serialized(occurrences),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            None => Response::err("schedule-history needs schedule"),
        },
        "schedule-recover" => match (
            request.common.id.as_deref(),
            request.orgintel.fire_at.as_deref(),
            request.common.as_actor.as_deref(),
            request.common.from.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(id), Some(scheduled_for), Some(actor), Some(requested_by), Some(reason)) => {
                let schedule_id = match uuid::Uuid::parse_str(id) {
                    Ok(id) => id,
                    Err(error) => return Response::err(format!("bad schedule id: {error}")),
                };
                let scheduled_for = match chrono::DateTime::parse_from_rfc3339(scheduled_for) {
                    Ok(value) => value.with_timezone(&chrono::Utc),
                    Err(error) => {
                        return Response::err(format!(
                            "schedule --scheduled-for must be RFC3339: {error}"
                        ));
                    }
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .recover_skipped_schedule(
                            schedule_id,
                            scheduled_for,
                            actor,
                            requested_by,
                            reason,
                        )
                        .await
                    {
                        Ok(recovery) => Response::ok_serialized(recovery),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err(
                "schedule-recover needs schedule, scheduled-for, actor, requester and reason",
            ),
        },
        "schedule-retry-recovery" => match (
            request.common.id.as_deref(),
            request.orgintel.fire_at.as_deref(),
            request.common.as_actor.as_deref(),
            request.common.from.as_deref(),
            request.common.reason.as_deref(),
            request.orgintel.prior_message_id,
            request.orgintel.retry_key.as_deref(),
        ) {
            (
                Some(id),
                Some(scheduled_for),
                Some(actor),
                Some(requested_by),
                Some(reason),
                Some(prior_message),
                Some(key),
            ) => {
                let schedule_id = match uuid::Uuid::parse_str(id) {
                    Ok(id) => id,
                    Err(error) => return Response::err(format!("bad schedule id: {error}")),
                };
                let scheduled_for = match chrono::DateTime::parse_from_rfc3339(scheduled_for) {
                    Ok(value) => value.with_timezone(&chrono::Utc),
                    Err(error) => {
                        return Response::err(format!(
                            "schedule --scheduled-for must be RFC3339: {error}"
                        ));
                    }
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .retry_schedule_recovery(
                            schedule_id,
                            scheduled_for,
                            actor,
                            key,
                            prior_message,
                            requested_by,
                            reason,
                        )
                        .await
                    {
                        Ok(retry) => Response::ok_serialized(retry),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err(
                "schedule-retry-recovery needs schedule, scheduled-for, actor, prior message, key, requester and reason",
            ),
        },
        "schedule-add" => match (
            request.common.as_actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(actor), Some(reason)) => {
                let recurring = request.orgintel.recurrence.as_deref() == Some("weekdays");
                if request.common.id.is_some() && recurring {
                    return Response::err(
                        "recurring schedules wake actors directly and cannot block Work",
                    );
                }
                if recurring {
                    if request.orgintel.fire_at.is_some() {
                        return Response::err("use either schedule --at or --weekdays, not both");
                    }
                    let Some(local_time) = request.orgintel.local_time.as_deref() else {
                        return Response::err("schedule --weekdays needs --at-local HH:MM");
                    };
                    let local_time = match chrono::NaiveTime::parse_from_str(local_time, "%H:%M") {
                        Ok(value) => value,
                        Err(error) => {
                            return Response::err(format!(
                                "schedule --at-local must be HH:MM: {error}"
                            ));
                        }
                    };
                    let Some(timezone) = request.orgintel.timezone.as_deref() else {
                        return Response::err("schedule --weekdays needs an IANA --timezone");
                    };
                    let Some(missed_policy) = request.orgintel.missed_policy.as_deref() else {
                        return Response::err(
                            "schedule --weekdays needs --on-missed skip|skip-if-late|catch-up|coalesce-latest",
                        );
                    };
                    let (missed_policy, catch_up_grace_seconds) = match missed_policy {
                        "skip" if request.orgintel.catch_up_grace_seconds.is_none() => {
                            ("skip", None)
                        }
                        "skip-if-late" | "catch-up" | "coalesce-latest" => {
                            match request.orgintel.catch_up_grace_seconds {
                                Some(seconds) if seconds > 0 => (
                                    match missed_policy {
                                        "skip-if-late" => "skip_if_late",
                                        "catch-up" => "catch_up_once",
                                        _ => "coalesce_latest",
                                    },
                                    Some(seconds),
                                ),
                                _ => {
                                    return Response::err(
                                        "bounded missed policies need a positive --catch-up-within-minutes",
                                    );
                                }
                            }
                        }
                        "skip" => {
                            return Response::err(
                                "--catch-up-within-minutes is invalid with unbounded skip",
                            );
                        }
                        _ => {
                            return Response::err(
                                "--on-missed must be skip|skip-if-late|catch-up|coalesce-latest",
                            );
                        }
                    };
                    let machine_requirement = match request
                        .orgintel
                        .execution_requirement
                        .as_deref()
                        .unwrap_or("local-mac")
                    {
                        "local-mac" => "local_mac",
                        "always-on" => "always_on",
                        _ => return Response::err("--execution must be local-mac|always-on"),
                    };
                    let org = match daemon.orgintel.get(company).await {
                        Ok(org) => org,
                        Err(error) => return Response::err(format!("{error:#}")),
                    };
                    if actor != "exec" {
                        let is_lead = match org.list_teams().await {
                            Ok(teams) => teams.iter().any(|team| team.lead_actor_id == actor),
                            Err(error) => return Response::err(format!("{error:#}")),
                        };
                        if !is_lead {
                            return Response::err(
                                "a free-standing schedule must target Exec or an accountable team lead; Staff time dependencies belong to Work",
                            );
                        }
                    }
                    return match org
                        .add_weekday_schedule_with_policy_and_requirement(
                            actor,
                            reason,
                            local_time,
                            timezone,
                            chrono::Utc::now(),
                            missed_policy,
                            catch_up_grace_seconds,
                            machine_requirement,
                        )
                        .await
                    {
                        Ok((schedule_id, next_fire_at, created)) => {
                            Response::ok(serde_json::json!({
                                "schedule_id": schedule_id,
                                "actor_id": actor,
                                "recurrence": "weekdays",
                                "local_time": local_time.format("%H:%M").to_string(),
                                "timezone": timezone,
                                "missed_policy": missed_policy,
                                "catch_up_grace_seconds": catch_up_grace_seconds,
                                "machine_requirement": machine_requirement,
                                "next_fire_at": next_fire_at,
                                "created": created,
                            }))
                        }
                        Err(error) => Response::err(format!("{error:#}")),
                    };
                }
                if request.orgintel.recurrence.is_some()
                    || request.orgintel.local_time.is_some()
                    || request.orgintel.timezone.is_some()
                    || request.orgintel.missed_policy.is_some()
                    || request.orgintel.catch_up_grace_seconds.is_some()
                    || request.orgintel.execution_requirement.is_some()
                {
                    return Response::err("recurring schedule fields require --weekdays");
                }
                let Some(fire_at) = request.orgintel.fire_at.as_deref() else {
                    return Response::err(
                        "schedule add needs --at, or --weekdays with --at-local and --timezone",
                    );
                };
                let fire_at = match chrono::DateTime::parse_from_rfc3339(fire_at) {
                    Ok(fire_at) => fire_at.with_timezone(&chrono::Utc),
                    Err(error) => {
                        return Response::err(format!("schedule --at must be RFC3339: {error}"));
                    }
                };
                let work_id = match request.common.id.as_deref() {
                    Some(work) => match uuid::Uuid::parse_str(work) {
                        Ok(work) => Some(work),
                        Err(error) => {
                            return Response::err(format!("bad scheduled Work id: {error}"));
                        }
                    },
                    None => None,
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => {
                        if work_id.is_none() && actor != "exec" {
                            let is_lead = match org.list_teams().await {
                                Ok(teams) => teams.iter().any(|team| team.lead_actor_id == actor),
                                Err(error) => return Response::err(format!("{error:#}")),
                            };
                            if !is_lead {
                                return Response::err(
                                    "a free-standing schedule must target Exec or an accountable team lead; Staff time dependencies belong to Work",
                                );
                            }
                        }
                        match org.add_schedule(actor, work_id, reason, fire_at).await {
                            Ok(schedule_id) => Response::ok(serde_json::json!({
                                "schedule_id": schedule_id,
                                "actor_id": actor,
                                "work_id": work_id,
                                "fire_at": fire_at,
                            })),
                            Err(error) => Response::err(format!("{error:#}")),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("schedule-add needs actor and reason"),
        },
        "schedule-policy" => match (
            request.common.id.as_deref(),
            request.common.as_actor.as_deref(),
            request.orgintel.missed_policy.as_deref(),
        ) {
            (Some(id), Some(actor), Some(policy)) => {
                let schedule_id = match uuid::Uuid::parse_str(id) {
                    Ok(id) => id,
                    Err(error) => return Response::err(format!("bad schedule id: {error}")),
                };
                let (policy, grace) = match policy {
                    "skip" if request.orgintel.catch_up_grace_seconds.is_none() => ("skip", None),
                    "skip-if-late" | "catch-up" | "coalesce-latest" => {
                        match request.orgintel.catch_up_grace_seconds {
                            Some(seconds) if seconds > 0 => (
                                match policy {
                                    "skip-if-late" => "skip_if_late",
                                    "catch-up" => "catch_up_once",
                                    _ => "coalesce_latest",
                                },
                                Some(seconds),
                            ),
                            _ => {
                                return Response::err(
                                    "bounded missed policies need a positive --catch-up-within-minutes",
                                );
                            }
                        }
                    }
                    "skip" => {
                        return Response::err(
                            "--catch-up-within-minutes is invalid with unbounded skip",
                        );
                    }
                    _ => {
                        return Response::err(
                            "--on-missed must be skip|skip-if-late|catch-up|coalesce-latest",
                        );
                    }
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .set_schedule_missed_policy(schedule_id, actor, policy, grace)
                        .await
                    {
                        Ok(true) => Response::ok(serde_json::json!({
                            "schedule_id": schedule_id,
                            "actor_id": actor,
                            "missed_policy": policy,
                            "catch_up_grace_seconds": grace,
                        })),
                        Ok(false) => {
                            Response::err("no live recurring schedule matched that id and actor")
                        }
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("schedule-policy needs schedule, actor and missed policy"),
        },
        "schedule-cancel" => match (
            request.common.as_actor.as_deref(),
            request.common.id.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(actor), Some(schedule), Some(reason)) => {
                let schedule = match uuid::Uuid::parse_str(schedule) {
                    Ok(schedule) => schedule,
                    Err(error) => return Response::err(format!("bad schedule id: {error}")),
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org.cancel_schedule(schedule, actor, reason).await {
                        Ok(true) => Response::ok(serde_json::json!({
                            "schedule_id": schedule,
                            "actor_id": actor,
                            "cancelled": true,
                            "reason": reason,
                        })),
                        Ok(false) => Response::err(
                            "schedule is absent, already settled, or owned by a different actor",
                        ),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("schedule-cancel needs schedule, actor and reason"),
        },
        "identity-show" => match daemon.orgintel.get(company).await {
            Ok(org) => match org.company_identity_snapshot().await {
                Ok(snapshot) => Response::ok(serde_json::to_value(snapshot).unwrap_or_default()),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "identity-evidence-add" => {
            let parse_pillar = |value: &str| match value {
                "truth" => Some(restless_orgintel::IdentityPillar::Truth),
                "voice" => Some(restless_orgintel::IdentityPillar::Voice),
                "visual" => Some(restless_orgintel::IdentityPillar::Visual),
                "culture" => Some(restless_orgintel::IdentityPillar::Culture),
                _ => None,
            };
            let parse_kind = |value: &str| match value {
                "fact" => Some(restless_orgintel::IdentityStatementKind::Fact),
                "belief" => Some(restless_orgintel::IdentityStatementKind::Belief),
                "guidance" => Some(restless_orgintel::IdentityStatementKind::Guidance),
                "observation" => Some(restless_orgintel::IdentityStatementKind::Observation),
                "example" => Some(restless_orgintel::IdentityStatementKind::Example),
                "exception" => Some(restless_orgintel::IdentityStatementKind::Exception),
                _ => None,
            };
            let parse_polarity = |value: Option<&str>| match value.unwrap_or("neutral") {
                "neutral" => Some(restless_orgintel::IdentityPolarity::Neutral),
                "positive" => Some(restless_orgintel::IdentityPolarity::Positive),
                "negative" => Some(restless_orgintel::IdentityPolarity::Negative),
                _ => None,
            };
            let parse_status = |value: Option<&str>| match value.unwrap_or("active") {
                "active" => Some(restless_orgintel::IdentityEvidenceStatus::Active),
                "disputed" => Some(restless_orgintel::IdentityEvidenceStatus::Disputed),
                "corrected" => Some(restless_orgintel::IdentityEvidenceStatus::Corrected),
                _ => None,
            };
            let Some(pillar) = request
                .orgintel
                .identity_pillar
                .as_deref()
                .and_then(parse_pillar)
            else {
                return Response::err(
                    "identity evidence needs --pillar truth|voice|visual|culture",
                );
            };
            let Some(statement_kind) = request
                .orgintel
                .identity_kind
                .as_deref()
                .and_then(parse_kind)
            else {
                return Response::err("identity evidence needs a valid --kind");
            };
            let Some(polarity) = parse_polarity(request.orgintel.polarity.as_deref()) else {
                return Response::err(
                    "identity evidence polarity must be neutral, positive or negative",
                );
            };
            let Some(status) = parse_status(request.orgintel.evidence_status.as_deref()) else {
                return Response::err(
                    "identity evidence status must be active, disputed or corrected",
                );
            };
            let supersedes = match request.orgintel.supersedes.as_deref() {
                Some(value) => match uuid::Uuid::parse_str(value) {
                    Ok(id) => Some(id),
                    Err(error) => {
                        return Response::err(format!("bad superseded evidence id: {error}"));
                    }
                },
                None => None,
            };
            let exception_expires_at = match request.orgintel.exception_expires_at.as_deref() {
                Some(value) => match chrono::DateTime::parse_from_rfc3339(value) {
                    Ok(value) => Some(value.with_timezone(&chrono::Utc)),
                    Err(error) => return Response::err(format!("bad exception expiry: {error}")),
                },
                None => None,
            };
            match (
                request.orgintel.claim_key.as_deref(),
                request.orgintel.statement.as_deref(),
                request.orgintel.actor.as_deref(),
                request.orgintel.source.as_deref(),
                request.orgintel.identity_authority.as_deref(),
                request.orgintel.scope.as_deref(),
                request.orgintel.evidence_locator.as_deref(),
            ) {
                (
                    Some(claim_key),
                    Some(statement),
                    Some(actor),
                    Some(source),
                    Some(authority),
                    Some(scope),
                    Some(locator),
                ) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .add_identity_evidence(restless_orgintel::NewIdentityEvidence {
                            pillar,
                            statement_kind,
                            claim_key,
                            statement,
                            author_id: actor,
                            source,
                            authority,
                            scope,
                            observed_at: chrono::Utc::now(),
                            evidence_locator: locator,
                            polarity,
                            status,
                            channel: request.orgintel.channel.as_deref(),
                            audience: request.orgintel.audience.as_deref(),
                            supersedes_evidence_id: supersedes,
                            exception_expires_at,
                            exception_indefinite: request.orgintel.exception_indefinite,
                        })
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({ "evidence_id": id })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                },
                _ => Response::err(
                    "identity evidence needs claim, statement, source, authority, scope, locator and an acting actor",
                ),
            }
        }
        "identity-propose" => match (
            request.orgintel.actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(actor), Some(rationale)) => {
                let evidence = match request
                    .orgintel
                    .evidence_ids
                    .iter()
                    .map(|value| uuid::Uuid::parse_str(value))
                    .collect::<std::result::Result<Vec<_>, _>>()
                {
                    Ok(ids) => ids,
                    Err(error) => {
                        return Response::err(format!("bad identity evidence id: {error}"));
                    }
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .propose_identity_release(actor, rationale, &evidence)
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({ "proposal_id": id })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("identity proposal needs --reason and an acting actor"),
        },
        "identity-brief" => match (
            request.common.body.as_deref(),
            request.orgintel.channel.as_deref(),
            request.orgintel.audience.as_deref(),
            request.orgintel.actor.as_deref(),
        ) {
            (Some(outcome), Some(channel), Some(audience), Some(actor)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => {
                        let release_id = match request.orgintel.release_id.as_deref() {
                            Some(value) => match uuid::Uuid::parse_str(value) {
                                Ok(id) => id,
                                Err(error) => {
                                    return Response::err(format!(
                                        "bad identity release id: {error}"
                                    ));
                                }
                            },
                            None => match org.company_identity_snapshot().await {
                                Ok(snapshot) => match snapshot.current_release {
                                    Some(release) => release.id,
                                    None => {
                                        return Response::err(
                                            "company has no released expression identity",
                                        );
                                    }
                                },
                                Err(error) => return Response::err(format!("{error:#}")),
                            },
                        };
                        match org
                            .compile_identity_brief(restless_orgintel::IdentityBriefRequest {
                                release_id,
                                outcome,
                                channel,
                                audience,
                                author: actor,
                                max_bytes: request.orgintel.max_bytes.unwrap_or(8 * 1024),
                                now: chrono::Utc::now(),
                            })
                            .await
                        {
                            Ok(brief) => {
                                Response::ok(serde_json::to_value(brief).unwrap_or_default())
                            }
                            Err(error) => Response::err(format!("{error:#}")),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => {
                Response::err("identity brief needs outcome, channel, audience and an acting actor")
            }
        },
        "voice-evidence-add" => {
            let kind = match request.orgintel.voice_kind.as_deref() {
                Some("approved_passage") => {
                    Some(restless_orgintel::VoiceEvidenceKind::ApprovedPassage)
                }
                Some("rejected_passage") => {
                    Some(restless_orgintel::VoiceEvidenceKind::RejectedPassage)
                }
                Some("expression_principle") => {
                    Some(restless_orgintel::VoiceEvidenceKind::ExpressionPrinciple)
                }
                Some("vocabulary") => Some(restless_orgintel::VoiceEvidenceKind::Vocabulary),
                Some("named_author") => Some(restless_orgintel::VoiceEvidenceKind::NamedAuthor),
                Some("channel_observation") => {
                    Some(restless_orgintel::VoiceEvidenceKind::ChannelObservation)
                }
                _ => None,
            };
            let channel = match parse_voice_channel(request.orgintel.channel.as_deref()) {
                Ok(value) => value,
                Err(error) => return Response::err(error),
            };
            let polarity = match request.orgintel.polarity.as_deref().unwrap_or("positive") {
                "neutral" => Some(restless_orgintel::IdentityPolarity::Neutral),
                "positive" => Some(restless_orgintel::IdentityPolarity::Positive),
                "negative" => Some(restless_orgintel::IdentityPolarity::Negative),
                _ => None,
            };
            let supersedes = match parse_optional_uuid(
                request.orgintel.supersedes.as_deref(),
                "superseded evidence id",
            ) {
                Ok(value) => value,
                Err(error) => return Response::err(error),
            };
            match (
                kind,
                polarity,
                request.orgintel.claim_key.as_deref(),
                request.orgintel.statement.as_deref(),
                request.orgintel.actor.as_deref(),
                request.orgintel.source.as_deref(),
                request.orgintel.identity_authority.as_deref(),
                request.orgintel.scope.as_deref(),
                request.orgintel.evidence_locator.as_deref(),
                request.orgintel.judgement_reason.as_deref(),
            ) {
                (
                    Some(kind),
                    Some(polarity),
                    Some(claim),
                    Some(statement),
                    Some(actor),
                    Some(source),
                    Some(authority),
                    Some(scope),
                    Some(locator),
                    Some(judgement),
                ) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .add_voice_evidence(restless_orgintel::NewVoiceEvidence {
                            kind,
                            claim_key: claim,
                            passage_or_principle: statement,
                            author_id: actor,
                            named_author: request.orgintel.named_author.as_deref(),
                            source,
                            authority,
                            scope,
                            observed_at: chrono::Utc::now(),
                            evidence_locator: locator,
                            judgement_reason: judgement,
                            polarity,
                            channel,
                            audience: request.orgintel.audience.as_deref(),
                            supersedes_evidence_id: supersedes,
                        })
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({"evidence_id": id})),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                },
                _ => Response::err(
                    "voice evidence needs a valid kind, polarity, claim, statement, source, authority, scope, locator, judgement and acting actor",
                ),
            }
        }
        "voice-bind" => {
            let channel = match parse_voice_channel(request.orgintel.channel.as_deref()) {
                Ok(Some(value)) => value,
                Ok(None) => return Response::err("voice bind needs a channel"),
                Err(error) => return Response::err(error),
            };
            let work_id =
                match parse_required_uuid(request.orgintel.voice_work_id.as_deref(), "Work id") {
                    Ok(value) => value,
                    Err(error) => return Response::err(error),
                };
            match (
                request.orgintel.actor.as_deref(),
                request.orgintel.voice_author.as_deref(),
                request.orgintel.audience.as_deref(),
                request.orgintel.reader_situation.as_deref(),
                request.orgintel.desired_understanding.as_deref(),
                request.orgintel.desired_action.as_deref(),
                request.orgintel.proof.as_deref(),
                request.orgintel.consequence.as_deref(),
            ) {
                (
                    Some(actor),
                    Some(author),
                    Some(audience),
                    Some(reader),
                    Some(understanding),
                    Some(action),
                    Some(proof),
                    Some(consequence),
                ) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .bind_voice_work_contract(restless_orgintel::NewVoiceWorkContract {
                            work_id,
                            channel,
                            author,
                            bound_by: actor,
                            audience,
                            reader_situation: reader,
                            desired_understanding: understanding,
                            desired_action: action,
                            proof,
                            consequence,
                        })
                        .await
                    {
                        Ok(contract) => Response::ok_serialized(contract),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                },
                _ => Response::err(
                    "voice bind needs an acting binder, named author, audience, reader situation, understanding, action, proof and consequence",
                ),
            }
        }
        "voice-brief" => {
            let work_id =
                match parse_required_uuid(request.orgintel.voice_work_id.as_deref(), "Work id") {
                    Ok(value) => value,
                    Err(error) => return Response::err(error),
                };
            match daemon.orgintel.get(company).await {
                Ok(org) => match org
                    .compile_voice_contract(work_id, request.orgintel.max_bytes.unwrap_or(8 * 1024))
                    .await
                {
                    Ok(brief) => Response::ok_serialized(brief),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "voice-render" => {
            let channel = match parse_voice_channel(request.orgintel.channel.as_deref()) {
                Ok(Some(value)) => value,
                Ok(None) => return Response::err("voice render needs a channel"),
                Err(error) => return Response::err(error),
            };
            let artifact_ref_id = match parse_required_uuid(
                request.orgintel.artifact_ref_id.as_deref(),
                "artifact ref id",
            ) {
                Ok(value) => value,
                Err(error) => return Response::err(error),
            };
            match (
                request.orgintel.renderer.as_deref(),
                request.orgintel.renderer_version.as_deref(),
                request.orgintel.semantic_checks.as_ref(),
                request.orgintel.actor.as_deref(),
            ) {
                (Some(renderer), Some(version), Some(checks), Some(actor)) => {
                    match daemon.orgintel.get(company).await {
                        Ok(org) => match org
                            .record_voice_render_evidence(
                                restless_orgintel::NewVoiceRenderEvidence {
                                    artifact_ref_id,
                                    channel,
                                    renderer,
                                    renderer_version: version,
                                    semantic_checks: checks,
                                    captured_by: actor,
                                },
                            )
                            .await
                        {
                            Ok(id) => Response::ok(serde_json::json!({"render_evidence_id": id})),
                            Err(error) => Response::err(format!("{error:#}")),
                        },
                        Err(error) => Response::err(format!("{error:#}")),
                    }
                }
                _ => Response::err(
                    "voice render needs renderer, version, JSON checks and an acting actor",
                ),
            }
        }
        "voice-review" => {
            let render_evidence_id = match parse_required_uuid(
                request.orgintel.render_evidence_id.as_deref(),
                "render evidence id",
            ) {
                Ok(value) => value,
                Err(error) => return Response::err(error),
            };
            let verdict = match request.orgintel.review_verdict.as_deref() {
                Some("accept") => Some(restless_orgintel::VoiceReviewVerdict::Accept),
                Some("revise") => Some(restless_orgintel::VoiceReviewVerdict::Revise),
                Some("reject") => Some(restless_orgintel::VoiceReviewVerdict::Reject),
                _ => None,
            };
            match (verdict, request.orgintel.actor.as_deref()) {
                (Some(verdict), Some(actor)) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .record_voice_review(restless_orgintel::NewVoiceReview {
                            render_evidence_id,
                            reviewer: actor,
                            verdict,
                            factual_findings: request
                                .orgintel
                                .factual_findings
                                .as_deref()
                                .unwrap_or(""),
                            abstraction_findings: request
                                .orgintel
                                .abstraction_findings
                                .as_deref()
                                .unwrap_or(""),
                            repetition_findings: request
                                .orgintel
                                .repetition_findings
                                .as_deref()
                                .unwrap_or(""),
                            channel_findings: request
                                .orgintel
                                .channel_findings
                                .as_deref()
                                .unwrap_or(""),
                            authorship_findings: request
                                .orgintel
                                .authorship_findings
                                .as_deref()
                                .unwrap_or(""),
                            concepts_removed: request
                                .orgintel
                                .concepts_removed
                                .as_deref()
                                .unwrap_or(""),
                        })
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({"review_id": id})),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                },
                _ => Response::err(
                    "voice review needs verdict accept|revise|reject and an acting reviewer",
                ),
            }
        }
        "voice-learn" => {
            let before = match parse_required_uuid(
                request.orgintel.before_artifact_ref_id.as_deref(),
                "before artifact ref id",
            ) {
                Ok(value) => value,
                Err(error) => return Response::err(error),
            };
            let after = match parse_required_uuid(
                request.orgintel.after_artifact_ref_id.as_deref(),
                "after artifact ref id",
            ) {
                Ok(value) => value,
                Err(error) => return Response::err(error),
            };
            let kind = match request.orgintel.learning_kind.as_deref() {
                Some("typo") => Some(restless_orgintel::VoiceLearningKind::Typo),
                Some("fact_correction") => {
                    Some(restless_orgintel::VoiceLearningKind::FactCorrection)
                }
                Some("voice_observation") => {
                    Some(restless_orgintel::VoiceLearningKind::VoiceObservation)
                }
                _ => None,
            };
            let channel = match parse_voice_channel(request.orgintel.channel.as_deref()) {
                Ok(value) => value,
                Err(error) => return Response::err(error),
            };
            match (
                kind,
                request.orgintel.actor.as_deref(),
                request.orgintel.claim_key.as_deref(),
                request.orgintel.observation.as_deref(),
                request.orgintel.motivating_decision.as_deref(),
                request.orgintel.scope.as_deref(),
                request.orgintel.source.as_deref(),
                request.orgintel.evidence_locator.as_deref(),
            ) {
                (
                    Some(change_kind),
                    Some(actor),
                    Some(claim),
                    Some(observation),
                    Some(decision),
                    Some(scope),
                    Some(source),
                    Some(locator),
                ) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .propose_voice_learning(restless_orgintel::NewVoiceLearningProposal {
                            created_by: actor,
                            before_artifact_ref_id: before,
                            after_artifact_ref_id: after,
                            change_kind,
                            claim_key: claim,
                            observation,
                            motivating_decision: decision,
                            scope,
                            source,
                            evidence_locator: locator,
                            named_author: request.orgintel.named_author.as_deref(),
                            channel,
                            audience: request.orgintel.audience.as_deref(),
                            observed_at: chrono::Utc::now(),
                        })
                        .await
                    {
                        Ok(Some(id)) => Response::ok(serde_json::json!({
                            "proposal_id": id,
                            "learning": "pending_owner_identity_decision"
                        })),
                        Ok(None) => Response::ok(serde_json::json!({
                            "proposal_id": serde_json::Value::Null,
                            "learning": "ignored_non_voice_edit"
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                },
                _ => Response::err(
                    "voice learning needs kind typo|fact_correction|voice_observation, exact before/after artifacts, claim, observation, decision, scope, source, locator and acting owner",
                ),
            }
        }
        "visual-evidence-add" => {
            let kind = match request.orgintel.visual_kind.as_deref() {
                Some("semantic_token") => {
                    Some(restless_orgintel::VisualEvidenceKind::SemanticToken)
                }
                Some("typography_role") => {
                    Some(restless_orgintel::VisualEvidenceKind::TypographyRole)
                }
                Some("composition_principle") => {
                    Some(restless_orgintel::VisualEvidenceKind::CompositionPrinciple)
                }
                Some("imagery_direction") => {
                    Some(restless_orgintel::VisualEvidenceKind::ImageryDirection)
                }
                Some("motion_pattern") => {
                    Some(restless_orgintel::VisualEvidenceKind::MotionPattern)
                }
                Some("product_representation_rule") => {
                    Some(restless_orgintel::VisualEvidenceKind::ProductRepresentationRule)
                }
                Some("primitive") => Some(restless_orgintel::VisualEvidenceKind::Primitive),
                Some("approved_composition") => {
                    Some(restless_orgintel::VisualEvidenceKind::ApprovedComposition)
                }
                Some("rejected_example") => {
                    Some(restless_orgintel::VisualEvidenceKind::RejectedExample)
                }
                _ => None,
            };
            let channel = match parse_visual_channel(request.orgintel.channel.as_deref()) {
                Ok(v) => v,
                Err(e) => return Response::err(e),
            };
            let polarity = match request.orgintel.polarity.as_deref().unwrap_or("positive") {
                "neutral" => Some(restless_orgintel::IdentityPolarity::Neutral),
                "positive" => Some(restless_orgintel::IdentityPolarity::Positive),
                "negative" => Some(restless_orgintel::IdentityPolarity::Negative),
                _ => None,
            };
            match (
                kind,
                polarity,
                request.orgintel.claim_key.as_deref(),
                request.orgintel.statement.as_deref(),
                request.orgintel.actor.as_deref(),
                request.orgintel.source.as_deref(),
                request.orgintel.identity_authority.as_deref(),
                request.orgintel.scope.as_deref(),
                request.orgintel.evidence_locator.as_deref(),
                request.orgintel.visual_purpose.as_deref(),
                request.orgintel.visual_rationale.as_deref(),
                request.orgintel.accessibility_notes.as_deref(),
                request.orgintel.primitive_dependencies.as_ref(),
            ) {
                (
                    Some(kind),
                    Some(polarity),
                    Some(claim),
                    Some(statement),
                    Some(actor),
                    Some(source),
                    Some(authority),
                    Some(scope),
                    Some(locator),
                    Some(purpose),
                    Some(rationale),
                    Some(accessibility),
                    Some(dependencies),
                ) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .add_visual_evidence(restless_orgintel::NewVisualEvidence {
                            kind,
                            claim_key: claim,
                            statement,
                            author_id: actor,
                            source,
                            authority,
                            scope,
                            observed_at: chrono::Utc::now(),
                            evidence_locator: locator,
                            rationale,
                            purpose,
                            polarity,
                            channel,
                            semantic_role: request.orgintel.semantic_role.as_deref(),
                            value: request.orgintel.visual_value.as_deref(),
                            reduced_motion_replacement: request
                                .orgintel
                                .reduced_motion_replacement
                                .as_deref(),
                            product_truth_locator: request
                                .orgintel
                                .product_truth_locator
                                .as_deref(),
                            origin: request.orgintel.primitive_origin.as_deref(),
                            licence: request.orgintel.primitive_licence.as_deref(),
                            framework: request.orgintel.primitive_framework.as_deref(),
                            dependencies,
                            adaptation_status: request.orgintel.adaptation_status.as_deref(),
                            accessibility_notes: accessibility,
                            supersedes_evidence_id: None,
                        })
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({"evidence_id":id})),
                        Err(e) => Response::err(format!("{e:#}")),
                    },
                    Err(e) => Response::err(format!("{e:#}")),
                },
                _ => Response::err(
                    "visual evidence needs valid kind, polarity, claim, statement, source, authority, scope, locator, purpose, rationale, accessibility, dependencies and acting actor",
                ),
            }
        }
        "visual-bind" => {
            let channel = match parse_visual_channel(request.orgintel.channel.as_deref()) {
                Ok(Some(v)) => v,
                Ok(None) => return Response::err("visual bind needs a channel"),
                Err(e) => return Response::err(e),
            };
            let work_id =
                match parse_required_uuid(request.orgintel.visual_work_id.as_deref(), "Work id") {
                    Ok(v) => v,
                    Err(e) => return Response::err(e),
                };
            let representation = match request.orgintel.product_representation.as_deref() {
                Some("exact_product") => {
                    Some(restless_orgintel::VisualRepresentation::ExactProduct)
                }
                Some("clearly_abstract") => {
                    Some(restless_orgintel::VisualRepresentation::ClearlyAbstract)
                }
                Some("none") => Some(restless_orgintel::VisualRepresentation::None),
                _ => None,
            };
            match (
                representation,
                request.orgintel.actor.as_deref(),
                request.orgintel.audience.as_deref(),
                request.common.body.as_deref(),
                request.orgintel.information_hierarchy.as_deref(),
                request.orgintel.proof.as_deref(),
                request.orgintel.visual_density.as_deref(),
                request.orgintel.imagery_role.as_deref(),
                request.orgintel.motion_role.as_deref(),
            ) {
                (
                    Some(product_representation),
                    Some(actor),
                    Some(audience),
                    Some(outcome),
                    Some(hierarchy),
                    Some(proof),
                    Some(density),
                    Some(imagery),
                    Some(motion),
                ) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .bind_visual_work_contract(restless_orgintel::NewVisualWorkContract {
                            work_id,
                            channel,
                            bound_by: actor,
                            audience,
                            outcome,
                            information_hierarchy: hierarchy,
                            proof,
                            density,
                            imagery_role: imagery,
                            motion_role: motion,
                            product_representation,
                            product_truth_locator: request
                                .orgintel
                                .product_truth_locator
                                .as_deref(),
                            requested_departure: request.orgintel.requested_departure.as_deref(),
                        })
                        .await
                    {
                        Ok(row) => Response::ok_serialized(row),
                        Err(e) => Response::err(format!("{e:#}")),
                    },
                    Err(e) => Response::err(format!("{e:#}")),
                },
                _ => Response::err(
                    "visual bind needs valid representation, actor, audience, outcome, hierarchy, proof, density, imagery and motion",
                ),
            }
        }
        "visual-brief" => {
            let work_id =
                match parse_required_uuid(request.orgintel.visual_work_id.as_deref(), "Work id") {
                    Ok(v) => v,
                    Err(e) => return Response::err(e),
                };
            match daemon.orgintel.get(company).await {
                Ok(org) => match org
                    .compile_visual_direction(
                        work_id,
                        request.orgintel.max_bytes.unwrap_or(10 * 1024),
                    )
                    .await
                {
                    Ok(brief) => Response::ok_serialized(brief),
                    Err(e) => Response::err(format!("{e:#}")),
                },
                Err(e) => Response::err(format!("{e:#}")),
            }
        }
        "visual-use" => {
            let work_id =
                match parse_required_uuid(request.orgintel.visual_work_id.as_deref(), "Work id") {
                    Ok(v) => v,
                    Err(e) => return Response::err(e),
                };
            let evidence_id = match parse_required_uuid(
                request.orgintel.visual_evidence_id.as_deref(),
                "visual evidence id",
            ) {
                Ok(v) => v,
                Err(e) => return Response::err(e),
            };
            match (
                request.orgintel.primitive_version.as_deref(),
                request.orgintel.visual_purpose.as_deref(),
            ) {
                (Some(version), Some(purpose)) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .record_visual_primitive_use(restless_orgintel::NewVisualPrimitiveUse {
                            work_id,
                            evidence_id,
                            primitive_version: version,
                            purpose,
                        })
                        .await
                    {
                        Ok(()) => Response::ok(serde_json::json!({"recorded":true})),
                        Err(e) => Response::err(format!("{e:#}")),
                    },
                    Err(e) => Response::err(format!("{e:#}")),
                },
                _ => Response::err("visual use needs exact primitive version and purpose"),
            }
        }
        "visual-render" => {
            let work_id =
                match parse_required_uuid(request.orgintel.visual_work_id.as_deref(), "Work id") {
                    Ok(v) => v,
                    Err(e) => return Response::err(e),
                };
            let artifact_ref_id = match parse_required_uuid(
                request.orgintel.artifact_ref_id.as_deref(),
                "artifact ref id",
            ) {
                Ok(v) => v,
                Err(e) => return Response::err(e),
            };
            let channel = match parse_visual_channel(request.orgintel.channel.as_deref()) {
                Ok(Some(v)) => v,
                Ok(None) => return Response::err("visual render needs a channel"),
                Err(e) => return Response::err(e),
            };
            let motion_state = match request.orgintel.motion_state.as_deref() {
                Some("full") => Some(restless_orgintel::VisualMotionState::Full),
                Some("reduced") => Some(restless_orgintel::VisualMotionState::Reduced),
                Some("static") => Some(restless_orgintel::VisualMotionState::Static),
                _ => None,
            };
            match (
                motion_state,
                request.orgintel.renderer.as_deref(),
                request.orgintel.renderer_version.as_deref(),
                request.orgintel.viewport_width,
                request.orgintel.viewport_height,
                request.orgintel.semantic_checks.as_ref(),
                request.orgintel.actor.as_deref(),
            ) {
                (
                    Some(motion_state),
                    Some(renderer),
                    Some(version),
                    Some(width),
                    Some(height),
                    Some(checks),
                    Some(actor),
                ) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .record_visual_render_evidence(restless_orgintel::NewVisualRenderEvidence {
                            work_id,
                            artifact_ref_id,
                            channel,
                            renderer,
                            renderer_version: version,
                            viewport_width: width,
                            viewport_height: height,
                            motion_state,
                            native_checks: checks,
                            captured_by: actor,
                        })
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({"render_evidence_id":id})),
                        Err(e) => Response::err(format!("{e:#}")),
                    },
                    Err(e) => Response::err(format!("{e:#}")),
                },
                _ => Response::err(
                    "visual render needs motion state, renderer, version, viewport, JSON checks and acting capturer",
                ),
            }
        }
        "visual-review" => {
            let render_evidence_id = match parse_required_uuid(
                request.orgintel.render_evidence_id.as_deref(),
                "render evidence id",
            ) {
                Ok(v) => v,
                Err(e) => return Response::err(e),
            };
            let control_render_evidence_id = match parse_optional_uuid(
                request.orgintel.control_render_evidence_id.as_deref(),
                "control render evidence id",
            ) {
                Ok(v) => v,
                Err(e) => return Response::err(e),
            };
            let verdict = match request.orgintel.review_verdict.as_deref() {
                Some("accept") => Some(restless_orgintel::VisualReviewVerdict::Accept),
                Some("revise") => Some(restless_orgintel::VisualReviewVerdict::Revise),
                Some("reject") => Some(restless_orgintel::VisualReviewVerdict::Reject),
                _ => None,
            };
            match (verdict, request.orgintel.actor.as_deref()) {
                (Some(verdict), Some(actor)) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .record_visual_review(restless_orgintel::NewVisualReview {
                            render_evidence_id,
                            control_render_evidence_id,
                            reviewer: actor,
                            verdict,
                            identity_findings: request
                                .orgintel
                                .visual_identity_findings
                                .as_deref()
                                .unwrap_or(""),
                            hierarchy_findings: request
                                .orgintel
                                .hierarchy_findings
                                .as_deref()
                                .unwrap_or(""),
                            density_findings: request
                                .orgintel
                                .density_findings
                                .as_deref()
                                .unwrap_or(""),
                            proof_findings: request
                                .orgintel
                                .proof_findings
                                .as_deref()
                                .unwrap_or(""),
                            product_fidelity_findings: request
                                .orgintel
                                .product_fidelity_findings
                                .as_deref()
                                .unwrap_or(""),
                            motion_findings: request
                                .orgintel
                                .motion_findings
                                .as_deref()
                                .unwrap_or(""),
                            defect_findings: request
                                .orgintel
                                .defect_findings
                                .as_deref()
                                .unwrap_or(""),
                            departure_decision: request
                                .orgintel
                                .departure_decision
                                .as_deref()
                                .unwrap_or(""),
                        })
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({"review_id":id})),
                        Err(e) => Response::err(format!("{e:#}")),
                    },
                    Err(e) => Response::err(format!("{e:#}")),
                },
                _ => Response::err(
                    "visual review needs verdict accept|revise|reject and acting reviewer",
                ),
            }
        }
        "culture-evidence-add" => {
            let kind = match request.orgintel.culture_kind.as_deref() {
                Some("founding_decision") => {
                    Some(restless_orgintel::CultureEvidenceKind::FoundingDecision)
                }
                Some("observed_conduct") => {
                    Some(restless_orgintel::CultureEvidenceKind::ObservedConduct)
                }
                Some("counterexample") => {
                    Some(restless_orgintel::CultureEvidenceKind::Counterexample)
                }
                Some("promoted_norm") => Some(restless_orgintel::CultureEvidenceKind::PromotedNorm),
                Some("bounded_exception") => {
                    Some(restless_orgintel::CultureEvidenceKind::BoundedException)
                }
                _ => None,
            };
            let case_kind = match parse_culture_case(request.orgintel.culture_case_kind.as_deref())
            {
                Ok(v) => v,
                Err(e) => return Response::err(e),
            };
            let confidence = match request.orgintel.culture_confidence.as_deref() {
                Some("tentative") => Some(restless_orgintel::CultureConfidence::Tentative),
                Some("corroborated") => Some(restless_orgintel::CultureConfidence::Corroborated),
                Some("owner_founded") => Some(restless_orgintel::CultureConfidence::OwnerFounded),
                _ => None,
            };
            match (
                kind,
                confidence,
                request.orgintel.claim_key.as_deref(),
                request.orgintel.statement.as_deref(),
                request.orgintel.actor.as_deref(),
                request.orgintel.source.as_deref(),
                request.orgintel.identity_authority.as_deref(),
                request.orgintel.scope.as_deref(),
                request.orgintel.evidence_locator.as_deref(),
                request.orgintel.culture_situation.as_deref(),
                request.orgintel.consequence.as_deref(),
                request.orgintel.culture_actors.as_deref(),
                request.orgintel.decision_authority.as_deref(),
                request.orgintel.observed_conduct.as_deref(),
                request.orgintel.observed_outcome.as_deref(),
                request.orgintel.counterexample.as_deref(),
                request.orgintel.boundary_conditions.as_deref(),
                request.orgintel.operational_implication.as_deref(),
                request.orgintel.actor_scope.as_deref(),
            ) {
                (
                    Some(kind),
                    Some(confidence),
                    Some(claim),
                    Some(statement),
                    Some(actor),
                    Some(source),
                    Some(authority),
                    Some(scope),
                    Some(locator),
                    Some(situation),
                    Some(consequence),
                    Some(actors),
                    Some(decision_authority),
                    Some(conduct),
                    Some(outcome),
                    Some(counterexample),
                    Some(boundary),
                    Some(implication),
                    Some(actor_scope),
                ) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .add_culture_evidence(restless_orgintel::NewCultureEvidence {
                            kind,
                            case_kind,
                            claim_key: claim,
                            statement,
                            author_id: actor,
                            source,
                            authority,
                            scope,
                            observed_at: chrono::Utc::now(),
                            evidence_locator: locator,
                            polarity: if kind
                                == restless_orgintel::CultureEvidenceKind::Counterexample
                            {
                                restless_orgintel::IdentityPolarity::Negative
                            } else {
                                restless_orgintel::IdentityPolarity::Positive
                            },
                            situation,
                            consequence,
                            actors,
                            decision_authority,
                            conduct,
                            observed_outcome: outcome,
                            confidence,
                            counterexample,
                            boundary_conditions: boundary,
                            operational_implication: implication,
                            actor_scope,
                            supersedes_evidence_id: None,
                        })
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({"evidence_id":id})),
                        Err(e) => Response::err(format!("{e:#}")),
                    },
                    Err(e) => Response::err(format!("{e:#}")),
                },
                _ => Response::err(
                    "culture evidence needs typed conduct, consequence, counterexample, boundary, confidence and acting author",
                ),
            }
        }
        "culture-bind" => {
            let work_id =
                match parse_required_uuid(request.orgintel.culture_work_id.as_deref(), "Work id") {
                    Ok(v) => v,
                    Err(e) => return Response::err(e),
                };
            let case_kind = match parse_culture_case(request.orgintel.culture_case_kind.as_deref())
            {
                Ok(Some(v)) => v,
                Ok(None) => return Response::err("culture bind needs a case"),
                Err(e) => return Response::err(e),
            };
            match (
                request.orgintel.culture_actor.as_deref(),
                request.orgintel.actor_role.as_deref(),
                request.orgintel.team_name.as_deref(),
                request.orgintel.consequence.as_deref(),
                request.orgintel.decision_boundary.as_deref(),
                request.orgintel.actor.as_deref(),
            ) {
                (
                    Some(actor),
                    Some(role),
                    Some(team),
                    Some(consequence),
                    Some(boundary),
                    Some(bound_by),
                ) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .bind_culture_work_contract(restless_orgintel::NewCultureWorkContract {
                            work_id,
                            case_kind,
                            actor,
                            actor_role: role,
                            team,
                            consequence,
                            decision_boundary: boundary,
                            bound_by,
                        })
                        .await
                    {
                        Ok(row) => Response::ok_serialized(row),
                        Err(e) => Response::err(format!("{e:#}")),
                    },
                    Err(e) => Response::err(format!("{e:#}")),
                },
                _ => Response::err(
                    "culture bind needs actor, role, team, consequence, decision boundary and binder",
                ),
            }
        }
        "culture-brief" => {
            let work_id =
                match parse_required_uuid(request.orgintel.culture_work_id.as_deref(), "Work id") {
                    Ok(v) => v,
                    Err(e) => return Response::err(e),
                };
            match daemon.orgintel.get(company).await {
                Ok(org) => match org
                    .compile_culture_posture(work_id, request.orgintel.max_bytes.unwrap_or(8192))
                    .await
                {
                    Ok(brief) => Response::ok_serialized(brief),
                    Err(e) => Response::err(format!("{e:#}")),
                },
                Err(e) => Response::err(format!("{e:#}")),
            }
        }
        "culture-case" => {
            let work_id =
                match parse_required_uuid(request.orgintel.culture_work_id.as_deref(), "Work id") {
                    Ok(v) => v,
                    Err(e) => return Response::err(e),
                };
            let artifact_ref_id = match parse_required_uuid(
                request.orgintel.artifact_ref_id.as_deref(),
                "artifact ref id",
            ) {
                Ok(v) => v,
                Err(e) => return Response::err(e),
            };
            let case_kind = match parse_culture_case(request.orgintel.culture_case_kind.as_deref())
            {
                Ok(Some(v)) => v,
                Ok(None) => return Response::err("culture record needs a case"),
                Err(e) => return Response::err(e),
            };
            let correction_of = match parse_optional_uuid(
                request.orgintel.correction_of.as_deref(),
                "correction record id",
            ) {
                Ok(v) => v,
                Err(e) => return Response::err(e),
            };
            match (
                request.orgintel.culture_decision.as_deref(),
                request.orgintel.culture_alternatives.as_ref(),
                request.orgintel.culture_unknowns.as_deref(),
                request.orgintel.semantic_checks.as_ref(),
                request.orgintel.actor.as_deref(),
            ) {
                (Some(decision), Some(alternatives), Some(unknowns), Some(checks), Some(actor)) => {
                    match daemon.orgintel.get(company).await {
                        Ok(org) => match org
                            .record_culture_case(restless_orgintel::NewCultureCaseRecord {
                                work_id,
                                artifact_ref_id,
                                case_kind,
                                decision,
                                alternatives,
                                unknowns,
                                correction_of,
                                correction_account: request
                                    .orgintel
                                    .correction_account
                                    .as_deref()
                                    .unwrap_or(""),
                                customer_action: request
                                    .orgintel
                                    .customer_action
                                    .as_deref()
                                    .unwrap_or(""),
                                native_checks: checks,
                                recorded_by: actor,
                            })
                            .await
                        {
                            Ok(id) => Response::ok(serde_json::json!({"case_record_id":id})),
                            Err(e) => Response::err(format!("{e:#}")),
                        },
                        Err(e) => Response::err(format!("{e:#}")),
                    }
                }
                _ => Response::err(
                    "culture case needs exact artifact, decision, alternatives, unknowns, native checks and recorder",
                ),
            }
        }
        "culture-review" => {
            let case_record_id = match parse_required_uuid(
                request.orgintel.culture_case_record_id.as_deref(),
                "culture case record id",
            ) {
                Ok(v) => v,
                Err(e) => return Response::err(e),
            };
            let verdict = match request.orgintel.review_verdict.as_deref() {
                Some("accept") => Some(restless_orgintel::CultureReviewVerdict::Accept),
                Some("revise") => Some(restless_orgintel::CultureReviewVerdict::Revise),
                Some("reject") => Some(restless_orgintel::CultureReviewVerdict::Reject),
                _ => None,
            };
            match (verdict, request.orgintel.actor.as_deref()) {
                (Some(verdict), Some(actor)) => match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .record_culture_review(restless_orgintel::NewCultureReview {
                            case_record_id,
                            reviewer: actor,
                            verdict,
                            conduct_findings: request
                                .orgintel
                                .conduct_findings
                                .as_deref()
                                .unwrap_or(""),
                            dissent_findings: request
                                .orgintel
                                .dissent_findings
                                .as_deref()
                                .unwrap_or(""),
                            uncertainty_findings: request
                                .orgintel
                                .uncertainty_findings
                                .as_deref()
                                .unwrap_or(""),
                            correction_findings: request
                                .orgintel
                                .correction_findings
                                .as_deref()
                                .unwrap_or(""),
                            authority_findings: request
                                .orgintel
                                .authority_findings
                                .as_deref()
                                .unwrap_or(""),
                            customer_or_hiring_findings: request
                                .orgintel
                                .customer_or_hiring_findings
                                .as_deref()
                                .unwrap_or(""),
                            slogan_recitation_detected: request.orgintel.slogan_recitation_detected,
                        })
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({"review_id":id})),
                        Err(e) => Response::err(format!("{e:#}")),
                    },
                    Err(e) => Response::err(format!("{e:#}")),
                },
                _ => Response::err("culture review needs verdict and independent reviewer"),
            }
        }
        "events" => match daemon.orgintel.get(company).await {
            Ok(org) => match org.list_events(request.common.limit.unwrap_or(50)).await {
                Ok(events) => Response::ok(serde_json::to_value(events).unwrap_or_default()),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "clear-poison" => match daemon.spend.clear_poison(company) {
            Ok(()) => Response::ok(serde_json::json!({
                "company": company,
                "cleared": true,
                "note": "spend accounting resumes from the company's real recorded cost",
            })),
            Err(error) => Response::err(format!("{error:#}")),
        },
        // S03-T5: the owner's yes. Authority is the one writer; OrgIntel gets
        // only a best-effort wake message. Idempotent: approving twice is not
        // an error, because an owner re-confirming should never look like a
        // failure.
        "approve" | "revoke" | "decline" => match request.authority.party {
            Some(party) => {
                let org = daemon.orgintel.get(company).await.ok();
                let result = match request.cmd.as_str() {
                    "approve" => {
                        approval::grant(
                            &daemon.root,
                            company,
                            &party,
                            &daemon.authority,
                            org.as_ref(),
                            principal.as_str(),
                        )
                        .await
                    }
                    "revoke" => {
                        approval::revoke(
                            &daemon.root,
                            company,
                            &party,
                            &daemon.authority,
                            org.as_ref(),
                            principal.as_str(),
                        )
                        .await
                    }
                    _ => {
                        approval::decline(
                            &daemon.root,
                            company,
                            &party,
                            &daemon.authority,
                            org.as_ref(),
                            principal.as_str(),
                        )
                        .await
                    }
                };
                match result {
                    Ok(message) => Response::ok(message),
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            None => Response::err("approval action needs --party"),
        },
        "attention" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => {
                let org = daemon.orgintel.get(company).await.ok();
                match attention::project(&config, &daemon.authority, org.as_ref()).await {
                    Ok(view) => match serde_json::to_value(view) {
                        Ok(value) => Response::ok(value),
                        Err(error) => Response::err(format!("encode attention: {error}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            Err(error) => Response::err(format!("{error:#}")),
        },
        "browser-status" => match runtime::doctor(company).await {
            Ok(report) => Response::ok(serde_json::json!({
                "generation": runtime::generation(company).await.ok().flatten(),
                "browser": report.browser,
                "control": runtime::read_browser_control(company).await.ok().flatten(),
            })),
            Err(error) => Response::err(format!("{error:#}")),
        },
        "browser-request" => {
            let requester = request.common.id.as_deref().unwrap_or("exec");
            let current = runtime::read_browser_control(company).await.ok().flatten();
            if current.as_ref().is_some_and(|value| {
                value["controller"] == "owner"
                    && value["expires_at"]
                        .as_str()
                        .and_then(|value| value.parse::<chrono::DateTime<chrono::Utc>>().ok())
                        .is_some_and(|expires| expires > chrono::Utc::now())
            }) {
                return Response::err_kind("conflict", "owner already controls the browser");
            }
            let state = serde_json::json!({
                "controller": "owner_requested",
                "requester": requester,
                "requested_at": chrono::Utc::now(),
            });
            match runtime::write_browser_control(company, &state).await {
                Ok(()) => Response::ok(state),
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "browser-release" => {
            let state =
                serde_json::json!({ "controller": "unclaimed", "released_at": chrono::Utc::now() });
            match runtime::write_browser_control(company, &state).await {
                Ok(()) => Response::ok(state),
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "effect" => match (
            request.authority.effect_class,
            request.authority.purpose,
            request.authority.key,
            request.orgintel.cwd,
            request.orgintel.argv,
        ) {
            (Some(effect_class), Some(purpose), Some(key), Some(cwd), Some(argv)) => {
                let actor = request.orgintel.actor.as_deref().unwrap_or("owner");
                match runtime::CompanyConfig::load(&daemon.root, company) {
                    Ok(config) => {
                        let org = daemon.orgintel.get(company).await.ok();
                        match effect::request_effect(
                            effect::EffectEnvironment {
                                config: &config,
                                authority: &daemon.authority,
                                org: org.as_ref(),
                                runtime_transport: &daemon.runtime_transport,
                            },
                            &effect_class,
                            request.authority.party.as_deref(),
                            &purpose,
                            request.authority.artifacts.unwrap_or_default(),
                            &cwd,
                            argv,
                            request.authority.secret_bindings.unwrap_or_default(),
                            &key,
                            actor,
                        )
                        .await
                        {
                            Ok(receipt) => match serde_json::to_value(&receipt) {
                                Ok(value) => Response::ok(value),
                                Err(error) => Response::err(format!("encode receipt: {error}")),
                            },
                            Err(error) => Response::err(format!("{error:#}")),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("effect needs class, purpose, key, cwd and command"),
        },
        "effect-reconcile" => match (
            request.authority.key.as_deref(),
            request.authority.execution_no,
            request.common.state.as_deref(),
            request.common.id.as_deref(),
        ) {
            (Some(key), Some(execution_no), Some(result), Some(evidence_receipt)) => {
                match effect::reconcile_unknown(
                    &daemon.authority,
                    company,
                    key,
                    execution_no,
                    result,
                    evidence_receipt,
                    request.orgintel.actor.as_deref().unwrap_or("owner"),
                )
                .await
                {
                    Ok(receipt) => match serde_json::to_value(receipt) {
                        Ok(value) => Response::ok(value),
                        Err(error) => Response::err(format!("encode receipt: {error}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err(
                "effect-reconcile needs key, execution, succeeded|failed and evidence receipt",
            ),
        },
        other => Response::err(format!("unknown command {other:?}")),
    }
}

fn work_commission_is_admitted(
    commissioned_by: &str,
    accountable_lead: &str,
    producing_topology: restless_orgintel::ProducingTopology,
    active_workers: &[String],
    requested_owner: &str,
) -> bool {
    commissioned_by == accountable_lead
        || (commissioned_by == "exec"
            && producing_topology == restless_orgintel::ProducingTopology::CoherentSingleWorker
            && active_workers == [requested_owner])
}

/// Accept a team by name or id. Names are what an Exec or owner actually types;
/// ids are what another command just returned. Refusing one of the two would
/// make the surface awkward for no safety gain — both resolve to one live team
/// or to an error that says so.
async fn resolve_team(
    org: &restless_orgintel::OrgIntel,
    reference: &str,
) -> std::result::Result<uuid::Uuid, String> {
    let teams = org
        .list_teams()
        .await
        .map_err(|error| format!("{error:#}"))?;
    if let Ok(id) = uuid::Uuid::parse_str(reference) {
        if teams.iter().any(|team| team.id == id) {
            return Ok(id);
        }
        return Err(format!("no live team with id {reference}"));
    }
    let mut matched = teams
        .iter()
        .filter(|team| team.name.eq_ignore_ascii_case(reference));
    match (matched.next(), matched.next()) {
        (Some(team), None) => Ok(team.id),
        (Some(_), Some(_)) => Err(format!(
            "{reference:?} matches more than one live team; use the team id"
        )),
        _ => Err(format!("no live team named {reference:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_plane_database_url_is_password_authenticated_and_bounded() {
        assert!(validate_plane_database_url(
            "postgresql://restless:secret@plane-database/restless?sslmode=disable"
        )
        .is_ok());
        for invalid in [
            "postgresql://restless@plane-database/restless",
            "postgresql://:secret@plane-database/restless",
            "postgresql://restless:secret@plane-database/",
            "sqlite:///state/restless.db",
            " postgresql://restless:secret@plane-database/restless",
            "postgresql://restless:secret@plane-database/restless\n",
        ] {
            assert!(
                validate_plane_database_url(invalid).is_err(),
                "accepted invalid hosted database URL shape"
            );
        }
    }

    fn decoded_request(value: serde_json::Value) -> Request {
        Request::decode(&value.to_string()).expect("decode request through the transport boundary")
    }

    #[tokio::test]
    async fn an_unconfigured_company_cannot_recreate_a_destroyed_cell() {
        let root =
            std::env::temp_dir().join(format!("restless-cell-tombstone-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        let registry = OrgIntelRegistry {
            database_url: "postgres://must-not-connect.invalid/restless".into(),
            root: root.clone(),
            handles: std::sync::Mutex::new(HashMap::new()),
        };
        let error = match registry.get("destroyed_test").await {
            Ok(_) => panic!("an unconfigured company unexpectedly acquired an OrgIntel cell"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("not configured"));
        assert!(!root.join("cells/destroyed_test/database.url").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exec_fast_path_is_only_the_unambiguous_coherent_single_worker_route() {
        let one_worker = vec!["product-builder".to_string()];
        assert!(work_commission_is_admitted(
            "exec",
            "product-direction",
            restless_orgintel::ProducingTopology::CoherentSingleWorker,
            &one_worker,
            "product-builder",
        ));
        assert!(!work_commission_is_admitted(
            "exec",
            "product-direction",
            restless_orgintel::ProducingTopology::LocallyClosingParallelUnit,
            &one_worker,
            "product-builder",
        ));
        assert!(!work_commission_is_admitted(
            "exec",
            "product-direction",
            restless_orgintel::ProducingTopology::CoherentSingleWorker,
            &["product-builder".into(), "product-reviewer".into()],
            "product-builder",
        ));
        assert!(work_commission_is_admitted(
            "product-direction",
            "product-direction",
            restless_orgintel::ProducingTopology::LocallyClosingParallelUnit,
            &[],
            "product-builder",
        ));
    }

    /// The hole this ticket closes: an agent inside the container asking for
    /// the owner's authority act. `main.rs:215` accepted this with the expiry
    /// "before any real external effect" — sprint 03 sent real email.
    #[test]
    fn the_company_may_not_perform_an_owner_authority_act() {
        for cmd in OWNER_ONLY {
            let refusal = authorize(Principal::CompanyExec, cmd)
                .expect_err("company/exec must not perform {cmd}");
            assert!(refusal.contains("owner authority"), "{cmd}: {refusal}");
        }
    }

    /// Sprint 05's administrative-surface guard. The dispatcher remains an
    /// intentionally ordinary match rather than a universal command algebra,
    /// so this test reads that match and fails when a new coordination verb is
    /// added without an owner CLI spelling. Open-ended Linux/browser work is
    /// deliberately outside this enumeration (`attach` is its door).
    #[test]
    fn every_dispatch_verb_has_a_cli_spelling() {
        let daemon_source = include_str!("main.rs");
        let cli_source = include_str!("../../restless/src/main.rs");
        let dispatch = daemon_source
            .split("match request.cmd.as_str() {")
            .nth(1)
            .expect("dispatch match")
            .split("other =>")
            .next()
            .expect("dispatch end");
        let mut verbs = std::collections::BTreeSet::new();
        for line in dispatch.lines() {
            if !line.starts_with("        \"") {
                continue;
            }
            let trimmed = line.trim_start();
            if !trimmed.contains("=>") {
                continue;
            }
            let before_arrow = trimmed.split("=>").next().unwrap_or_default();
            for quoted in before_arrow.split('"').skip(1).step_by(2) {
                if !quoted.is_empty() {
                    verbs.insert(quoted.to_string());
                }
            }
        }
        verbs.insert("company-list".to_string());
        for verb in verbs {
            assert!(
                cli_source.contains(&format!("\"{verb}\"")),
                "daemon dispatch verb {verb:?} has no CLI spelling"
            );
        }

        for field in [
            "name",
            "mission",
            "spend_ceiling_usd",
            "model",
            "model_failover",
            "credentials.",
            "approved_parties",
        ] {
            let reachable = match field {
                "approved_parties" => cli_source.contains("Approve"),
                "name" => cli_source.contains("CompanyCommand::Create"),
                other => cli_source.contains(other),
            };
            assert!(
                reachable,
                "CompanyConfig field {field} is not owner-reachable"
            );
        }
    }

    /// ...and the gate must not break the agents' ordinary channel, which
    /// would be a worse bug than the one it fixes.
    #[test]
    fn the_company_keeps_its_coordination_channel() {
        for cmd in ["work", "message", "work-handoff-resolve", "effect", "inbox"] {
            assert_eq!(
                authorize(Principal::CompanyExec, cmd).unwrap(),
                Principal::CompanyExec,
                "{cmd} must stay open to the company"
            );
        }
    }

    /// TCP has no principal fallback. Local Unix derives owner identity from
    /// its listener, but a Runtime must carry a valid capability.
    #[test]
    fn runtime_legacy_principal_spelling_cannot_claim_owner() {
        assert!(Principal::legacy_runtime_claim(None).is_ok());
        assert!(Principal::legacy_runtime_claim(Some("company/exec")).is_ok());
        assert!(Principal::legacy_runtime_claim(Some("owner")).is_err());
        assert!(Principal::legacy_runtime_claim(Some("root")).is_err());
    }

    #[test]
    fn tcp_capability_derives_company_and_actor_before_dispatch() {
        let root =
            std::env::temp_dir().join(format!("restless-tcp-auth-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let issuer = capability::CapabilityIssuer::open(&root).unwrap();
        let token = issuer
            .issue_actor_session("acme_test", "delivery-lead", "session_1")
            .unwrap();

        let mut valid = decoded_request(serde_json::json!({
            "cmd": "message",
            "company": "acme_test",
            "principal": "company/exec",
            "session_capability": token,
            "from": "delivery-lead",
            "to": "exec",
            "body": "native result is ready"
        }));
        assert_eq!(
            authenticate_request(&mut valid, &issuer, ConnectionOrigin::RuntimeTcp).unwrap(),
            Principal::CompanyExec
        );
        assert_eq!(valid.company.as_deref(), Some("acme_test"));
        assert_eq!(valid.common.from.as_deref(), Some("delivery-lead"));

        let mut owner_claim = decoded_request(serde_json::json!({
            "cmd": "approve",
            "company": "acme_test",
            "principal": "owner",
            "session_capability": issuer
                .issue_actor_session("acme_test", "delivery-lead", "session_2")
                .unwrap()
        }));
        assert!(
            authenticate_request(&mut owner_claim, &issuer, ConnectionOrigin::RuntimeTcp)
                .unwrap_err()
                .contains("may not claim owner")
        );

        let mut foreign_company = decoded_request(serde_json::json!({
            "cmd": "message",
            "company": "other_test",
            "session_capability": issuer
                .issue_actor_session("acme_test", "delivery-lead", "session_3")
                .unwrap(),
            "from": "delivery-lead",
            "body": "forged"
        }));
        assert!(
            authenticate_request(&mut foreign_company, &issuer, ConnectionOrigin::RuntimeTcp)
                .is_err()
        );

        let mut foreign_actor = decoded_request(serde_json::json!({
            "cmd": "message",
            "company": "acme_test",
            "session_capability": issuer
                .issue_actor_session("acme_test", "delivery-lead", "session_4")
                .unwrap(),
            "from": "exec",
            "body": "forged"
        }));
        assert!(
            authenticate_request(&mut foreign_actor, &issuer, ConnectionOrigin::RuntimeTcp)
                .unwrap_err()
                .contains("cannot claim")
        );

        let mut local = decoded_request(serde_json::json!({
            "cmd": "approve",
            "company": "acme_test",
            "principal": "company/exec"
        }));
        assert_eq!(
            authenticate_request(&mut local, &issuer, ConnectionOrigin::LocalOwner).unwrap(),
            Principal::Owner
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_owner_may_do_everything_the_company_may_and_more() {
        for cmd in OWNER_ONLY {
            assert_eq!(authorize(Principal::Owner, cmd).unwrap(), Principal::Owner);
        }
        assert_eq!(
            authorize(Principal::Owner, "goals").unwrap(),
            Principal::Owner
        );
    }
}
