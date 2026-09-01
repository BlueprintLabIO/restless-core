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
mod inbound;
mod ingress;
mod legal;
mod model_gateway;
mod owner;
mod owner_brief;
mod plane;
mod reconcile;
mod release;
mod runtime;
mod schedule;
mod spend;
mod staff;
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
    fn load_or_seed(root: &Path) -> Result<Self> {
        if let Ok(path) = std::env::var("RESTLESS_DATABASE_URL_FILE") {
            if path.trim().is_empty() {
                anyhow::bail!("RESTLESS_DATABASE_URL_FILE is set but empty");
            }
            let database_url = std::fs::read_to_string(&path)
                .with_context(|| format!("read RESTLESS_DATABASE_URL_FILE {path}"))?;
            let database_url = database_url.trim().to_string();
            if database_url.is_empty() {
                anyhow::bail!("RESTLESS_DATABASE_URL_FILE contains an empty value");
            }
            return Ok(Self { database_url });
        }
        if let Ok(database_url) = std::env::var("RESTLESS_DATABASE_URL") {
            if database_url.trim().is_empty() {
                anyhow::bail!("RESTLESS_DATABASE_URL is set but empty");
            }
            return Ok(Self { database_url });
        }
        let path = root.join("orgintel.toml");
        if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            return toml::from_str(&raw).with_context(|| format!("parse {}", path.display()));
        }
        let user = std::env::var("USER").unwrap_or_else(|_| "postgres".to_string());
        let config = Self {
            database_url: format!("postgres://{user}@localhost/restless"),
        };
        let rendered = toml::to_string_pretty(&config).context("render orgintel.toml")?;
        std::fs::write(&path, rendered).with_context(|| format!("seed {}", path.display()))?;
        Ok(config)
    }
}

fn configured_companies(root: &Path) -> Result<Vec<String>> {
    let directory = root.join("companies");
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
    pub(crate) orgintel: OrgIntelRegistry,
    pub(crate) staff: staff::StaffRegistry,
    /// Reconnectable live projections for agent turns. Completed messages,
    /// Work, and Attempts remain OrgIntel truth; this state is ephemeral.
    pub(crate) activities: activity::AgentActivityStreams,
    /// One wake at a time per company, however the wake was requested —
    /// the scheduler (T6) and the owner-typed socket path share this set.
    pub(crate) in_flight: schedule::InFlight,
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

/// Base TCP port the company containers reach the daemon on (T10). Next to the
/// model gateway's 7790; reachable as host.docker.internal from containers.
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
    // Local source checkouts conventionally keep bootstrap credentials in an
    // ignored `.env`. Load it before any subsystem reads configuration, while
    // preserving explicitly inherited service-manager variables. Infisical is
    // the durable backend; this is the one-time/local migration source.
    match dotenvy::dotenv() {
        Ok(path) => eprintln!("loaded local environment from {}", path.display()),
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("load local .env"),
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
    if std::env::args().nth(1).as_deref() == Some("test-entry-jwks") {
        println!("{}", entry::test_jwks_from_env()?);
        return Ok(());
    }

    let root = runtime::state_root();
    std::fs::create_dir_all(root.join("companies"))
        .with_context(|| format!("create state root {}", root.display()))?;
    let capabilities = capability::CapabilityIssuer::open(&root)?;
    // Two supported topologies (ADR 0007): direct loopback, or a network
    // entry that verifies a signed assertion. Resolve and validate the entry
    // configuration before starting provider or scheduler work, so a plane
    // that cannot describe how it verifies fails here rather than serving.
    let owner_config = owner::OwnerConfig::from_env().await?;
    runtime::validate_company_image_config(owner_config.is_network())?;

    // Open authoritative charged-use accounting before the model relay. The
    // relay receives this exact ledger and is the only model path permitted to
    // append charged records.
    let spend = spend::SpendLedger::open(&root)?;

    // Model access is a host authority boundary. OMP's imported broker and
    // gateway hold the provider credential; company processes receive only a
    // signed, scoped relay capability. A failed start is a failed daemon start
    // because no configured Exec can think without it.
    let company_configs = configured_companies(&root)?
        .into_iter()
        .map(|company| runtime::CompanyConfig::load(&root, &company))
        .collect::<Result<Vec<_>>>()?;
    let _model_gateway =
        model_gateway::start(&company_configs, &root, capabilities.clone(), spend.clone()).await?;
    // T5: coordination state. The database must answer at boot — probe,
    // never guess that it will be there when a company wakes.
    let orgintel_config = OrgIntelConfig::load_or_seed(&root)?;
    OrgIntel::probe(&orgintel_config.database_url)
        .await
        .context("orgintel database is not reachable at boot")?;

    let authority = authority::AuthorityStore::connect(&orgintel_config.database_url).await?;

    // One-time custody transfer from the old recoverable event stream. Do it
    // before listeners open so no effect can race its own migration.
    for company in configured_companies(&root)? {
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
    }

    let daemon = std::sync::Arc::new(Daemon {
        root: root.clone(),
        capabilities,
        spend,
        authority,
        orgintel: OrgIntelRegistry {
            database_url: orgintel_config.database_url,
            root: root.clone(),
            handles: std::sync::Mutex::new(HashMap::new()),
        },
        staff: staff::StaffRegistry::default(),
        activities: activity::AgentActivityStreams::default(),
        in_flight: std::sync::Arc::new(std::sync::Mutex::new(schedule::WakeClaims::default())),
    });

    // Effect children carry the narrowest live secret boundary. Reap an old
    // daemon's dedicated effect UID before a new child or scheduler may start;
    // Authority keeps the interrupted intent unknown until explicit evidence.
    effect::sweep_orphans(&daemon.root).await;

    // T9: agent processes outliving their supervising daemon are orphans —
    // reap them and close their running Work Attempts before anything new wakes.
    staff::sweep_orphans(&daemon.root, &daemon.orgintel).await;

    // T6: the scheduler is what makes the company act without the owner
    // typing — time triggers (exec-set schedules + periodic tick) and
    // OrgIntel LISTEN/NOTIFY events share one loop. Product integration tests
    // may drive the exact semantic loop themselves; a narrowly named escape
    // hatch prevents the resident scheduler racing that controller. It is
    // refused if any real company is configured.
    let test_scheduler_disabled =
        std::env::var("RESTLESS_TEST_DISABLE_SCHEDULER").is_ok_and(|value| value == "1");
    if test_scheduler_disabled
        && company_configs
            .iter()
            .any(|config| !config.name.ends_with("_test"))
    {
        anyhow::bail!("RESTLESS_TEST_DISABLE_SCHEDULER is allowed only on an all-test plane");
    }
    if test_scheduler_disabled {
        tracing::warn!("automatic scheduler disabled for an isolated test plane");
    } else {
        tokio::spawn(schedule::run(std::sync::Arc::clone(&daemon)));
    }

    // S05-T1/T5: the local-owner projection and browser transport are
    // a separate failure boundary. Losing the SPA must not stop schedules,
    // coordination or provider ingress.
    let owner_daemon = std::sync::Arc::clone(&daemon);
    tokio::spawn(async move {
        if let Err(error) = owner::serve(owner_daemon, owner_config).await {
            tracing::error!("owner gateway stopped: {error:#}");
        }
    });

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
    let coord_addr = format!("0.0.0.0:{}", port_with_offset(COORD_TCP_PORT)?);
    match tokio::net::TcpListener::bind(&coord_addr).await {
        Ok(tcp) => {
            tracing::info!(addr = %coord_addr, "coordination TCP listening (company containers)");
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
                agents in containers will have no coordination channel");
        }
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
        "actor-create" | "actor-model" | "actor-retire" | "team-create" | "team-update"
        | "team-assign" | "team-lead" | "team-disband" | "goal-add" | "work-goal"
        | "work-assign" | "work-artifact" | "work-gate" | "work-handoff" | "effect"
        | "effect-reconcile" => pin_actor(&mut request.orgintel.actor, actor, "actor")?,
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
    let company = match request.company.as_deref() {
        Some(name) => name,
        None => return Response::err("missing company"),
    };
    match request.cmd.as_str() {
        // S04-T1. Clone-then-up, so a throwaway is one command rather than a
        // config file someone hand-copies and forgets to strip.
        "up" if request.lifecycle.from_company.is_some() => {
            let from = request.lifecycle.from_company.as_deref().unwrap_or_default();
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
            match runtime::destroy(&daemon.root, company, &org, &daemon.spend).await {
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
                            approval::purge_legacy_config_approvals(
                                &daemon.root,
                                &mut config,
                            )?;
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
        "company-set" => match (request.common.state.as_deref(), request.common.body.as_deref()) {
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
                        _ if key.starts_with("credentials.") => {
                            config.credentials.insert(key[12..].to_string(), value.to_string());
                            Ok(())
                        }
                        _ => Err(anyhow::anyhow!(
                            "unknown company key {key:?}; use mission, model, model_failover, worker_runtime, reasoning_effort, spend_ceiling_usd, or credentials.<binding>"
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
        "credential-set" => match (request.authority.capability.as_deref(), request.common.body.as_deref()) {
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
        "credential-promote" => match (request.authority.capability.as_deref(), request.common.body.as_deref()) {
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
                Ok(input) => match legal::set_profile(&daemon.authority, company, input, "owner").await
                {
                    Ok(profile) => Response::ok_serialized(profile),
                    Err(error) => Response::err(format!("{error:#}")),
                },
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
                    match airwallex::set_connection(&daemon.authority, company, input, "owner").await
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
                            Ok(()) => match finance::reserve(&daemon.authority, company, input).await {
                                Ok(reservation) => Response::ok_serialized(reservation),
                                Err(error) => Response::err(format!("{error:#}")),
                            },
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
            (Ok(org), Ok(config)) => match ensure_standing_actors(&org, Some(&config.model)).await {
                Ok(()) => match org.table_names().await {
                    Ok(tables) => Response::ok(serde_json::json!({
                        "schema": org.schema(),
                        "tables": tables,
                    })),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            },
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
                    let reason = request.common.reason.as_deref().unwrap_or("owner-requested wake");
                    match schedule::run_exec_turn(daemon, &config, &org, reason, &cancellation).await {
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
            _ => Response::err(
                "actor model needs --actor, --model, --reason, and an acting actor",
            ),
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
                        .filter(|actor| {
                            actor.team_id.is_none()
                                && actor.kind == "staff"
                        })
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
                    Ok(org) => match org.create_team(name, brief, lead, created_by).await {
                        Ok(id) => Response::ok(serde_json::json!({ "team_id": id })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
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
            request.common.id.as_deref().map(uuid::Uuid::parse_str).transpose(),
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
        "work-attempts" => match daemon.orgintel.get(company).await {
            Ok(org) => {
                let work_id = match request.common.id.as_deref().map(uuid::Uuid::parse_str).transpose() {
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
            request.common.id.as_deref().map(uuid::Uuid::parse_str).transpose(),
            request.common.to.as_deref(),
            request.orgintel.actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Ok(Some(work_id)), Some(new_owner), Some(changed_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => {
                        let staff_lead = match org.team_lead_for(new_owner).await {
                            Ok(Some(lead)) => lead,
                            Ok(None) => return Response::err(format!(
                                "new Work owner {new_owner:?} must be Staff under an accountable lead; Exec, unassigned actors, and team leads cannot own production Work"
                            )),
                            Err(error) => return Response::err(format!("{error:#}")),
                        };
                        match org.reassign_work(work_id, new_owner, changed_by, reason).await {
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
                            Ok(None) => return Response::err(format!(
                                "Work owner {owner:?} must be Staff under an accountable lead; Exec, unassigned actors, and team leads cannot own production Work"
                            )),
                            Err(error) => return Response::err(format!("{error:#}")),
                        };
                        if commissioned_by != accountable_lead {
                            return Response::err(format!(
                                "production Work for {owner:?} must be commissioned by its accountable lead {accountable_lead:?}, not {commissioned_by:?}"
                            ));
                        }
                        let actor = match org.active_actor(owner).await {
                            Ok(Some(actor)) => actor,
                            Ok(None) => return Response::err(format!(
                                "Work owner {owner:?} is not an existing active actor; inspect `restless people` and commission one stable specialist if none fits"
                            )),
                            Err(error) => return Response::err(format!("{error:#}")),
                        };
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
                                    return Response::err(format!("bad Goal id: {error}"))
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
                            expected_artifact: request.orgintel.expected_artifact.as_deref().unwrap_or(""),
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
                        let added = if let Some(source_message_id) = request.orgintel.source_message_id
                        {
                            org.add_work_from_external_message_with_edges_and_gates(
                                work,
                                &requires,
                                &revises,
                                &gates,
                                request.orgintel.owner_review,
                                source_message_id,
                                commissioned_by,
                            )
                            .await
                        } else if request.orgintel.owner_review {
                            org.add_review_required_work_with_edges_and_gates(
                                work, &requires, &revises, &gates,
                            )
                            .await
                        } else {
                            org.add_work_with_edges_and_gates(work, &requires, &revises, &gates)
                                .await
                        };
                        match added {
                            Ok(id) => Response::ok(serde_json::json!({
                                "work_id": id,
                                "accountable_lead_id": accountable_lead,
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
                        ))
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
                    _ => return Response::err(format!("unsupported handoff category {category:?}")),
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
                        return Response::err(format!("unsupported owner brief kind {other:?}"))
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
                        ))
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
            request.common.id.as_deref().map(uuid::Uuid::parse_str).transpose(),
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
            request.common.id.as_deref().map(uuid::Uuid::parse_str).transpose(),
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
            request.common.id.as_deref().map(uuid::Uuid::parse_str).transpose(),
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
        "work-review" => match (request.common.id.as_deref(), request.common.state.as_deref()) {
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
                        ))
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
                        None => org.send_message(&from, request.common.to.as_deref(), &body).await,
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
        "schedule-add" => match (
            request.common.as_actor.as_deref(),
            request.common.reason.as_deref(),
        ) {
            (Some(actor), Some(reason)) => {
                let recurring = request.orgintel.recurrence.as_deref() == Some("weekdays");
                if request.common.id.is_some() && recurring {
                    return Response::err("recurring schedules wake actors directly and cannot block Work");
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
                        Err(error) => return Response::err(format!("schedule --at-local must be HH:MM: {error}")),
                    };
                    let Some(timezone) = request.orgintel.timezone.as_deref() else {
                        return Response::err("schedule --weekdays needs an IANA --timezone");
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
                            return Response::err("a free-standing schedule must target Exec or an accountable team lead; Staff time dependencies belong to Work");
                        }
                    }
                    return match org
                        .add_weekday_schedule(actor, reason, local_time, timezone, chrono::Utc::now())
                        .await
                    {
                        Ok((schedule_id, next_fire_at, created)) => Response::ok(serde_json::json!({
                            "schedule_id": schedule_id,
                            "actor_id": actor,
                            "recurrence": "weekdays",
                            "local_time": local_time.format("%H:%M").to_string(),
                            "timezone": timezone,
                            "next_fire_at": next_fire_at,
                            "created": created,
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    };
                }
                if request.orgintel.recurrence.is_some()
                    || request.orgintel.local_time.is_some()
                    || request.orgintel.timezone.is_some()
                {
                    return Response::err("recurring schedule fields require --weekdays");
                }
                let Some(fire_at) = request.orgintel.fire_at.as_deref() else {
                    return Response::err("schedule add needs --at, or --weekdays with --at-local and --timezone");
                };
                let fire_at = match chrono::DateTime::parse_from_rfc3339(fire_at) {
                    Ok(fire_at) => fire_at.with_timezone(&chrono::Utc),
                    Err(error) => {
                        return Response::err(format!(
                            "schedule --at must be RFC3339: {error}"
                        ))
                    }
                };
                let work_id = match request.common.id.as_deref() {
                    Some(work) => match uuid::Uuid::parse_str(work) {
                        Ok(work) => Some(work),
                        Err(error) => {
                            return Response::err(format!("bad scheduled Work id: {error}"))
                        }
                    },
                    None => None,
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => {
                        if work_id.is_none() && actor != "exec" {
                            let is_lead = match org.list_teams().await {
                                Ok(teams) => {
                                    teams.iter().any(|team| team.lead_actor_id == actor)
                                }
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

    fn decoded_request(value: serde_json::Value) -> Request {
        Request::decode(&value.to_string()).expect("decode request through the transport boundary")
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
