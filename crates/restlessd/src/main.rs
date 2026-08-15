//! restlessd — the stable coordination core (ARCHITECTURE.md §4.4).
//!
//! Sprint 01 slice: company environment lifecycle over a unix socket and the
//! stable coordination core. JSON-lines protocol: one request line, one
//! response line.

mod acp;
mod approval;
mod attention;
mod authority;
mod context;
mod credential;
mod effect;
mod exec;
mod health;
mod inbound;
mod ingress;
mod model_gateway;
mod owner;
mod provider;
mod reconcile;
mod runtime;
mod schedule;
mod spend;
mod staff;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use restless_orgintel::OrgIntel;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

#[derive(Debug, Deserialize)]
struct Request {
    cmd: String,
    company: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    // T10 coordination fields (presence depends on cmd).
    #[serde(default)]
    body: Option<String>,
    /// Secret material forwarded once to the configured credential backend.
    /// This field is never persisted or logged; company config stores only
    /// `body`, the credential reference.
    #[serde(default)]
    secret_value: Option<String>,
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    as_actor: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    resolution: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
    // S02-T2 spawn fields.
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    repo: Option<String>,
    // T8 effect fields.
    #[serde(default)]
    capability: Option<String>,
    #[serde(default)]
    args: Option<serde_json::Value>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    actor: Option<String>,
    // S03-T5 approval field.
    #[serde(default)]
    party: Option<String>,
    // S04-T5 role/model fields: what a spawned actor IS, and what it thinks with.
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    model: Option<String>,
    /// S04-T10. Who is asking, at the authority boundary. `authority-plane §4.1`
    /// names the V0 set; `cross-layer §2.2` keeps `principal_id` distinct from
    /// `actor_id` (an actor is who did the work, a principal is who was allowed
    /// to ask). Absent is rejected, never defaulted — a missing principal is
    /// exactly the case this field exists to catch.
    #[serde(default)]
    principal: Option<String>,
    /// S04-T1. `up --from <live>` clones a company into a throwaway;
    /// `down --destroy` removes container, volume, schema and spend spool.
    #[serde(default)]
    from_company: Option<String>,
    #[serde(default)]
    destroy: bool,
    /// Rebuild and reconcile the Company Runtime image. This remains one
    /// generic lifecycle operation; Docker is an implementation detail in
    /// `runtime.rs`, not part of the owner's operating transcript.
    #[serde(default)]
    reconcile: bool,
}

/// The V0 principal set (`authority-plane §4.1`). Two, because two is what
/// exists: the human on the host, and the company running in a container.
/// Going from here to N is adding variants, not changing shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Principal {
    Owner,
    CompanyExec,
}

impl Principal {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "owner" => Some(Self::Owner),
            "company/exec" => Some(Self::CompanyExec),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::CompanyExec => "company/exec",
        }
    }
}

/// Commands that are acts of owner authority. Everything else is open to both
/// principals. A list and not a policy engine: `authority-plane §6.5` warns off
/// a DSL before a workload demands one, and this sprint puts the Kernel proper
/// out of scope.
///
/// The membership rule: could a company, by running this, widen what it is
/// allowed to do to the world — or stop the owner from watching it try?
const OWNER_ONLY: &[&str] = &[
    "approve",
    "decline",
    "revoke",
    "up",
    "down",
    "clear-poison",
    "company-create",
    "company-set",
    "credential-set",
    "owner-token",
];

/// The whole gate, as one pure decision so it can be tested adversarially
/// rather than only observed through a socket.
///
/// Returns the authenticated principal, or the refusal to send back.
fn authorize(raw: Option<&str>, cmd: &str) -> std::result::Result<Principal, String> {
    let raw = raw.map(str::trim).filter(|value| !value.is_empty());
    let Some(raw) = raw else {
        return Err("request carries no principal; this daemon does not default one".into());
    };
    let Some(principal) = Principal::parse(raw) else {
        return Err(format!("unknown principal {raw:?}"));
    };
    if principal != Principal::Owner && OWNER_ONLY.contains(&cmd) {
        return Err(format!(
            "{cmd} is an act of owner authority; principal {} may not perform it",
            principal.as_str()
        ));
    }
    Ok(principal)
}

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

/// A poisoned company's ledger total is pinned at `u64::MAX` micro-USD. Any
/// value near it is the sentinel, not spend.
const POISON_SENTINEL_USD: f64 = 1_000_000_000.0;

/// A typed refusal. The UI (and the agent) must be able to tell "authority
/// denied" from "daemon unreachable" from "already resolved" without switching
/// on prose — `S03-T8` item 2. This ticket does not un-flatten every existing
/// error; it declines to add a new flattened one.
#[derive(Debug, Serialize)]
struct ErrorBody {
    kind: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorBody>,
}

impl Response {
    fn ok(data: impl Into<serde_json::Value>) -> Self {
        Self {
            ok: true,
            data: Some(data.into()),
            error: None,
        }
    }
    fn err(message: impl Into<String>) -> Self {
        Self::err_kind("error", message)
    }
    fn err_kind(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(ErrorBody {
                kind: kind.into(),
                message: message.into(),
            }),
        }
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

/// Lazily ensured per-company OrgIntel handles (one pool per company).
pub(crate) struct OrgIntelRegistry {
    pub(crate) database_url: String,
    handles: std::sync::Mutex<HashMap<String, OrgIntel>>,
}

impl OrgIntelRegistry {
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
        let handle = OrgIntel::ensure(&self.database_url, company).await?;
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

pub(crate) struct Daemon {
    pub(crate) root: PathBuf,
    pub(crate) spend: spend::SpendLedger,
    pub(crate) authority: authority::AuthorityStore,
    pub(crate) orgintel: OrgIntelRegistry,
    pub(crate) staff: staff::StaffRegistry,
    /// One wake at a time per company, however the wake was requested —
    /// the scheduler (T6) and the owner-typed socket path share this set.
    pub(crate) in_flight: schedule::InFlight,
}

/// TCP port the company containers reach the daemon on (T10). Next to the
/// model gateway's 7790; reachable as host.docker.internal from containers.
pub(crate) const COORD_TCP_PORT: u16 = 7791;

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
    let root = runtime::state_root();
    std::fs::create_dir_all(root.join("companies"))
        .with_context(|| format!("create state root {}", root.display()))?;

    // Model access is a host authority boundary. OMP's imported broker and
    // gateway hold the provider credential; company processes receive only a
    // narrower gateway bearer. A failed start is a failed daemon start because
    // no configured Exec can think without it.
    let company_configs = configured_companies(&root)?
        .into_iter()
        .map(|company| runtime::CompanyConfig::load(&root, &company))
        .collect::<Result<Vec<_>>>()?;
    let _model_gateway = model_gateway::start(&company_configs).await?;

    // The spend ledger remains in the coordination core, but metering comes
    // from OMP's ACP usage updates rather than from a custom HTTP proxy.
    let spend = spend::SpendLedger::open(&root)?;
    // T5: coordination state. The database must answer at boot — probe,
    // never guess that it will be there when a company wakes.
    let orgintel_config = OrgIntelConfig::load_or_seed(&root)?;
    OrgIntel::probe(&orgintel_config.database_url)
        .await
        .context("orgintel database is not reachable at boot")?;

    let authority = authority::AuthorityStore::connect(&orgintel_config.database_url).await?;

    // One-time custody transfer from the old recoverable event stream. Do it
    // before listeners open so no effect can race its own migration.
    let bootstrap_orgintel = OrgIntelRegistry {
        database_url: orgintel_config.database_url.clone(),
        handles: std::sync::Mutex::new(HashMap::new()),
    };
    for company in configured_companies(&root)? {
        let mut config = runtime::CompanyConfig::load(&root, &company)?;
        let org = bootstrap_orgintel.get(&company).await?;
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
    }

    let daemon = std::sync::Arc::new(Daemon {
        root: root.clone(),
        spend,
        authority,
        orgintel: OrgIntelRegistry {
            database_url: orgintel_config.database_url,
            handles: std::sync::Mutex::new(HashMap::new()),
        },
        staff: staff::StaffRegistry::default(),
        in_flight: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
    });

    // T9: agent processes outliving the daemon that spawned them are orphans
    // — kill them and mark their staff commitments before anything new wakes.
    staff::sweep_orphans(&daemon.root, &daemon.orgintel).await;

    // T6: the scheduler is what makes the company act without the owner
    // typing — time triggers (exec-set schedules + periodic tick) and
    // OrgIntel LISTEN/NOTIFY events share one loop.
    tokio::spawn(schedule::run(std::sync::Arc::clone(&daemon)));

    // S05-T1/T5: the authenticated owner projection and browser transport are
    // a separate failure boundary. Losing the SPA must not stop schedules,
    // coordination or provider ingress.
    let owner_daemon = std::sync::Arc::clone(&daemon);
    tokio::spawn(async move {
        if let Err(error) = owner::serve(owner_daemon).await {
            tracing::error!("owner gateway stopped: {error:#}");
        }
    });

    let sock = root.join("restlessd.sock");
    if sock.exists() {
        std::fs::remove_file(&sock)?;
    }
    let listener = UnixListener::bind(&sock).with_context(|| format!("bind {}", sock.display()))?;
    tracing::info!(socket = %sock.display(), "restlessd listening");

    // T10: the agents' channel. Unix sockets do not cross the Docker Desktop
    // file share (probed: the mount hangs), so containers reach the daemon
    // over TCP on the same proven path as the model gateway. The trust
    // boundary is these listeners, not the CLI binary (§6.1); company
    // identity on a request is trusted as-sent — accepted risk this sprint
    // (single-operator host), expiry: before any real external effect.
    let coord_addr = format!("0.0.0.0:{COORD_TCP_PORT}");
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
                                if let Err(error) = serve(stream, &daemon).await {
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
                if let Err(error) = ingress::serve(ingress::INGRESS_PORT, secret, sink).await {
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

    loop {
        let (stream, _) = listener.accept().await?;
        let daemon = std::sync::Arc::clone(&daemon);
        tokio::spawn(async move {
            if let Err(error) = serve(stream, &daemon).await {
                tracing::warn!("connection error: {error:#}");
            }
        });
    }
}

async fn serve<S>(stream: S, daemon: &Daemon) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (read, mut write) = tokio::io::split(stream);
    let mut lines = BufReader::new(read).lines();
    while let Some(line) = lines.next_line().await? {
        let request = match serde_json::from_str::<Request>(&line) {
            Ok(request) => request,
            Err(error) => {
                let response = Response::err(format!("bad request: {error}"));
                let mut out = serde_json::to_string(&response)?;
                out.push('\n');
                write.write_all(out.as_bytes()).await?;
                continue;
            }
        };
        // S04-T10. The gate sits HERE, above the watch/dispatch branch, so a
        // streaming command cannot slip past it. It gates on the principal the
        // request carries, never on which listener the request arrived at:
        // "the TCP socket may not do X" would work today and would have to be
        // torn out the moment a second human exists (`ARCHITECTURE.md:690`).
        let principal = match authorize(request.principal.as_deref(), &request.cmd) {
            Ok(principal) => principal,
            Err(refusal) => {
                tracing::warn!(
                    cmd = %request.cmd,
                    company = ?request.company,
                    principal = ?request.principal,
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
    // The one-owner credential and company catalogue exist above any one
    // company. Keep these explicit instead of inventing a fake global company.
    if request.cmd == "owner-token" {
        return match owner::rotate_token(&daemon.root) {
            Ok(token) => Response::ok(serde_json::json!({
                "token": token,
                "note": "shown once; only its digest is stored. Sign in at the owner gateway, then remove this output from scrollback if needed.",
            })),
            Err(error) => Response::err(format!("{error:#}")),
        };
    }
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
        "up" if request.from_company.is_some() => {
            let from = request.from_company.as_deref().unwrap_or_default();
            match runtime::clone_config(&daemon.root, from, company) {
                Ok(config) => match runtime::up(&config, request.reconcile).await {
                    Ok(message) => match daemon.orgintel.get(company).await {
                        Ok(_) => match daemon
                            .authority
                            .initialise_company(
                                company,
                                &approval::legacy_config_approvals(&config),
                            )
                            .await
                        {
                            Ok(()) => {
                                Response::ok(format!("{message} (cloned from {from}, simulated)"))
                            }
                            Err(error) => Response::err(format!(
                                "container up but Authority initialisation failed: {error:#}"
                            )),
                        },
                        Err(error) => Response::err(format!(
                            "container up but orgintel schema failed: {error:#}"
                        )),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            }
        }
        "up" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(mut config) => {
                let _reconcile_guard = if request.reconcile {
                    // Claim the same company-wide slot as an Exec wake before
                    // the image build starts. The build awaits Docker; without
                    // this claim the scheduler could start an Exec in that
                    // window and reconciliation would replace its container.
                    let claimed = daemon
                        .in_flight
                        .lock()
                        .map(|mut running| running.insert(company.to_string()))
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
                match runtime::up(&config, request.reconcile).await {
                    Ok(message) => {
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
        "down" if request.destroy => {
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
        "company-create" => match request.body {
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
                    Ok(mut config) => match runtime::CompanyConfig::save(&daemon.root, &config) {
                        Ok(()) => match daemon
                            .authority
                            .initialise_company(
                                company,
                                &approval::legacy_config_approvals(&config),
                            )
                            .await
                        {
                            Ok(()) => match approval::purge_legacy_config_approvals(
                                &daemon.root,
                                &mut config,
                            ) {
                                Ok(()) => Response::ok(format!("created company {company}")),
                                Err(error) => Response::err(format!(
                                    "company and Authority created but legacy config cleanup failed: {error:#}"
                                )),
                            },
                            Err(error) => Response::err(format!(
                                "company config created but Authority initialisation failed: {error:#}"
                            )),
                        },
                        Err(error) => Response::err(format!("{error:#}")),
                    },
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
        "company-set" => match (request.state.as_deref(), request.body.as_deref()) {
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
                        "spend_ceiling_usd" => value
                            .parse::<f64>()
                            .map(|parsed| config.spend_ceiling_usd = parsed)
                            .map_err(|_| anyhow::anyhow!("spend_ceiling_usd must be a number")),
                        "from_address" => {
                            config.from_address = (!value.trim().is_empty()).then(|| value.to_string());
                            Ok(())
                        }
                        _ if key.starts_with("providers.") => {
                            config.providers.insert(key[10..].to_string(), value.to_string());
                            Ok(())
                        }
                        _ if key.starts_with("credentials.") => {
                            config.credentials.insert(key[12..].to_string(), value.to_string());
                            Ok(())
                        }
                        _ => Err(anyhow::anyhow!(
                            "unknown company key {key:?}; use mission, model, spend_ceiling_usd, from_address, providers.<capability>, or credentials.<capability>"
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
        "credential-set" => match (request.capability.as_deref(), request.body.as_deref()) {
            (Some(capability), Some(reference)) => {
                if let Some(value) = request.secret_value.as_deref() {
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
                            .insert(capability.to_string(), reference.to_string());
                        match runtime::CompanyConfig::save(&daemon.root, &config) {
                            Ok(()) => Response::ok(format!(
                                "stored reference for {capability}; no secret value was stored"
                            )),
                            Err(error) => Response::err(format!("{error:#}")),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("credential-set needs capability and reference"),
        },
        "credential-check" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => {
                let mut rows = Vec::with_capacity(config.credentials.len());
                for (capability, reference) in &config.credentials {
                    let probe = credential::probe_reference(reference).await;
                    rows.push(serde_json::json!({
                        "capability": capability,
                        "reference": reference,
                        "status": probe.status.as_str(),
                        "detail": probe.detail,
                    }));
                }
                Response::ok(serde_json::Value::Array(rows))
            }
            Err(error) => Response::err(format!("{error:#}")),
        },
        // T5 probe: ensure the company schema and report what is in it.
        "orgintel-init" => match daemon.orgintel.get(company).await {
            Ok(org) => match org.table_names().await {
                Ok(tables) => Response::ok(serde_json::json!({
                    "schema": org.schema(),
                    "tables": tables,
                })),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        // T4: one Exec wake — rehydrate, work a turn, decide termination.
        // One wake at a time per company, whoever asked: a second exec
        // mid-turn would race the first in the same filesystem. Refuse
        // honestly rather than queue — queuing is machinery nobody needs.
        "wake" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => match daemon.orgintel.get(company).await {
                Ok(org) => {
                    {
                        let mut claims = daemon.in_flight.lock().expect("in-flight guard");
                        if !claims.insert(company.to_string()) {
                            return Response::err(format!(
                                "a wake is already in flight for {company}; \
                                 its outcome lands in the event stream"
                            ));
                        }
                    }
                    let _guard = schedule::WakeGuard::new(company, &daemon.in_flight);
                    let reason = request.reason.as_deref().unwrap_or("owner-requested wake");
                    match exec::wake(&config, &daemon.spend, &daemon.authority, &org, reason).await
                    {
                        Ok(report) => {
                            // T9: the Exec's spawn requests are honored after
                            // its outcome is recorded; refusals reach it by mail.
                            staff::process_spawns(
                                &config,
                                &daemon.spend,
                                &org,
                                &daemon.staff,
                                &report.spawn_requests,
                            )
                            .await;
                            match serde_json::to_value(&report) {
                                Ok(value) => Response::ok(value),
                                Err(error) => Response::err(format!("encode report: {error}")),
                            }
                        }
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
        "tell" => match request.body {
            Some(body) => match daemon.orgintel.get(company).await {
                Ok(org) => {
                    // Both ends of the message carry an FK: on a fresh
                    // company (no wake yet) neither row exists, and the
                    // owner's first ever interaction must not fail on a
                    // machinery detail.
                    if let Err(error) = org.add_actor("owner", "owner", "The Owner").await {
                        return Response::err(format!("{error:#}"));
                    }
                    if let Err(error) = org.add_actor("exec", "exec", "The Exec").await {
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
            Ok(org) => match org.list_actors().await {
                Ok(actors) => {
                    let breakdown = daemon.spend.breakdown(company);
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
                                    .map(|running| running.contains(company))
                                    .unwrap_or(false)
                            } else {
                                daemon.staff.is_actor_running(company, &actor.id)
                            };
                            serde_json::json!({
                                "actor_id": actor.id,
                                "role": actor.kind,
                                "display": actor.display,
                                "model": actor.model,
                                "spent_usd": round_usd(spent),
                                "session_running": session_running,
                            })
                        })
                        .collect();
                    Response::ok(serde_json::Value::Array(rows))
                }
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "receipts" => match daemon.authority.records_of_kind(company, "effect").await {
            Ok(events) => {
                let wanted = request.capability.as_deref();
                let limit = request.limit.unwrap_or(50).max(1) as usize;
                let rows: Vec<serde_json::Value> = events
                    .iter()
                    .rev()
                    .filter(|event| wanted.is_none_or(|cap| event.body["capability"] == cap))
                    .take(limit)
                    .map(|event| {
                        serde_json::json!({
                            "capability": event.body["capability"],
                            "provider": event.body["provider"],
                            "party": event.body["party"],
                            "actor": event.body["actor"],
                            "outcome": event.body["outcome"],
                            "idempotency_key": event.body["idempotency_key"],
                            "at": event.created_at,
                        })
                    })
                    .collect();
                Response::ok(serde_json::Value::Array(rows))
            }
            Err(error) => Response::err(format!("{error:#}")),
        },
        "spend" => match (
            runtime::CompanyConfig::load(&daemon.root, company),
            daemon.orgintel.get(company).await,
        ) {
            (Ok(config), Ok(org)) => match org.list_actors().await {
                Ok(actors) => {
                    let roles: HashMap<String, String> = actors
                        .into_iter()
                        .map(|actor| (actor.id, actor.kind))
                        .collect();
                    let by_actor = daemon.spend.breakdown(company);
                    // Accounted spend is summed from the real records. It is NOT
                    // the same number as the ledger total, and the difference is
                    // the point: a fail-closed poison pins the total at u64::MAX so
                    // every preflight refuses, which renders as $18,446,744,073,709
                    // if you print it as money. That is a fabricated figure where
                    // the honest answer is "poisoned, and here is what was actually
                    // accounted before it happened" (`owner-cockpit §2.6`).
                    let accounted: f64 = by_actor.iter().map(|(_, _, usd)| usd).sum();
                    let poisoned = daemon.spend.spent_usd(company) > POISON_SENTINEL_USD;
                    let rows: Vec<serde_json::Value> = by_actor
                        .into_iter()
                        .map(|(actor, model, usd)| {
                            let role = roles.get(&actor).cloned();
                            serde_json::json!({
                                "actor": actor,
                                "role": role,
                                "model": model,
                                "spent_usd": round_usd(usd),
                            })
                        })
                        .collect();
                    Response::ok(serde_json::json!({
                        "accounted_usd": round_usd(accounted),
                        "ceiling_usd": config.spend_ceiling_usd,
                        "remaining_usd": if poisoned {
                            serde_json::Value::Null
                        } else {
                            serde_json::json!(round_usd((config.spend_ceiling_usd - accounted).max(0.0)))
                        },
                        "poisoned": poisoned,
                        "note": if poisoned {
                            serde_json::json!(
                                "fail-closed: a turn could not be accounted, so this company is \
                                 stopped until `restless clear-poison` runs. `accounted_usd` is real \
                                 spend; the ledger total is pinned and is not a dollar figure"
                            )
                        } else {
                            serde_json::Value::Null
                        },
                        "by_actor": rows,
                    }))
                }
                Err(error) => Response::err(format!("{error:#}")),
            },
            (Err(error), _) | (_, Err(error)) => Response::err(format!("{error:#}")),
        },
        "goals" => match daemon.orgintel.get(company).await {
            Ok(org) => match org.list_goals().await {
                Ok(goals) => Response::ok(serde_json::to_value(goals).unwrap_or_default()),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "commitments" => match daemon.orgintel.get(company).await {
            Ok(org) => match org.list_commitments().await {
                Ok(commitments) => {
                    Response::ok(serde_json::to_value(commitments).unwrap_or_default())
                }
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        // Reading your own inbox marks read; inspecting another actor's
        // (--as) does not — an observer must not hide mail from its
        // addressee.
        "inbox" => match daemon.orgintel.get(company).await {
            Ok(org) => match org.inbox(request.as_actor.as_deref()).await {
                Ok(messages) => {
                    if request.as_actor.is_none() {
                        for message in &messages {
                            if let Err(error) = org.mark_read(message.id).await {
                                tracing::warn!("mark_read {}: {error:#}", message.id);
                            }
                        }
                    }
                    Response::ok(serde_json::to_value(messages).unwrap_or_default())
                }
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "message" => match (request.from, request.body) {
            (Some(from), Some(body)) => match daemon.orgintel.get(company).await {
                Ok(org) => match org.send_message(&from, request.to.as_deref(), &body).await {
                    Ok(id) => Response::ok(serde_json::json!({ "message_id": id })),
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            },
            _ => Response::err("message needs from and body"),
        },
        // The agents' report path: complete or block a commitment.
        "commitment-state" => match (request.id, request.state.as_deref()) {
            (Some(id), Some(state)) => {
                let state = match state {
                    "completed" | "complete" => restless_orgintel::CommitmentState::Completed,
                    "blocked" | "block" => restless_orgintel::CommitmentState::Blocked,
                    "abandoned" | "abandon" => restless_orgintel::CommitmentState::Abandoned,
                    other => {
                        return Response::err(format!(
                            "state must be completed|blocked|abandoned, got {other:?}"
                        ))
                    }
                };
                let Ok(id) = uuid::Uuid::parse_str(&id) else {
                    return Response::err(format!("bad commitment id {id:?}"));
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .set_commitment_state(
                            id,
                            state,
                            request.resolution.as_deref().unwrap_or(""),
                        )
                        .await
                    {
                        Ok(()) => Response::ok("recorded"),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("commitment-state needs id and state"),
        },
        "events" => match daemon.orgintel.get(company).await {
            Ok(org) => match org.list_events(request.limit.unwrap_or(50)).await {
                Ok(events) => Response::ok(serde_json::to_value(events).unwrap_or_default()),
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        // S02-T2: delegation as a tool. The Exec calls this the moment it
        // decides to delegate, rather than remembering a JSON field when it
        // stops. Refusals (bad name, cap reached, empty task) come back on
        // this call instead of arriving later as mail nobody connected to the
        // decision.
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
        "approve" | "revoke" | "decline" => match request.party {
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
            let requester = request.id.as_deref().unwrap_or("exec");
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
        "spawn" => match (request.name, request.body) {
            (Some(name), Some(task)) => match runtime::CompanyConfig::load(&daemon.root, company) {
                Ok(config) => match daemon.orgintel.get(company).await {
                    Ok(org) => {
                        let ask = staff::SpawnRequest {
                            name,
                            task,
                            repo: request.repo,
                            role: request.role,
                            model: request.model,
                        };
                        match staff::spawn_now(&config, &daemon.spend, &org, &daemon.staff, &ask)
                            .await
                        {
                            Ok(()) => Response::ok(serde_json::json!({
                                "spawned": ask.name,
                                "workdir": ask.repo.as_ref().map(|_| {
                                    format!("/company/worktrees/{}", ask.name)
                                }),
                                "note": "supervised; its completion or blockage will wake you",
                            })),
                            Err(error) => Response::err(format!("{error:#}")),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            },
            _ => Response::err("spawn needs --name and a task".to_string()),
        },
        "effect" => match (request.capability, request.key) {
            (Some(capability), Some(key)) => {
                let actor = request.actor.as_deref().unwrap_or("owner");
                match runtime::CompanyConfig::load(&daemon.root, company) {
                    Ok(config) => {
                        let org = daemon.orgintel.get(company).await.ok();
                        match effect::request_effect(
                            effect::EffectEnvironment {
                                root: &daemon.root,
                                config: &config,
                                authority: &daemon.authority,
                                org: org.as_ref(),
                            },
                            &capability,
                            request.args.unwrap_or(serde_json::Value::Null),
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
            _ => Response::err("effect needs capability and key"),
        },
        other => Response::err(format!("unknown command {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hole this ticket closes: an agent inside the container asking for
    /// the owner's authority act. `main.rs:215` accepted this with the expiry
    /// "before any real external effect" — sprint 03 sent real email.
    #[test]
    fn the_company_may_not_perform_an_owner_authority_act() {
        for cmd in OWNER_ONLY {
            let refusal = authorize(Some("company/exec"), cmd)
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
        verbs.extend(["owner-token".to_string(), "company-list".to_string()]);
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
            "providers.",
            "from_address",
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
        for cmd in [
            "commitments",
            "message",
            "commitment-state",
            "spawn",
            "effect",
            "inbox",
        ] {
            assert_eq!(
                authorize(Some("company/exec"), cmd).unwrap(),
                Principal::CompanyExec,
                "{cmd} must stay open to the company"
            );
        }
    }

    /// Absent is refused, not defaulted. A daemon that defaults a principal
    /// grants authority to whoever forgot to send one.
    #[test]
    fn a_missing_or_unknown_principal_is_refused_never_defaulted() {
        for raw in [None, Some(""), Some("   ")] {
            assert!(authorize(raw, "goals").is_err(), "{raw:?} must be refused");
        }
        // Not a fallback: an unrecognised principal is an error, for the same
        // reason an unknown credential scheme is (credential.rs:49).
        assert!(authorize(Some("root"), "goals").is_err());
        assert!(
            authorize(Some("owner "), "approve").is_ok(),
            "whitespace must not deny the owner"
        );
    }

    #[test]
    fn the_owner_may_do_everything_the_company_may_and_more() {
        for cmd in OWNER_ONLY {
            assert_eq!(authorize(Some("owner"), cmd).unwrap(), Principal::Owner);
        }
        assert_eq!(authorize(Some("owner"), "goals").unwrap(), Principal::Owner);
    }
}
