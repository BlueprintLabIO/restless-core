//! restlessd — the stable coordination core (ARCHITECTURE.md §4.4).
//!
//! Sprint 01 slice: company environment lifecycle over a unix socket and the
//! stable coordination core. JSON-lines protocol: one request line, one
//! response line.

mod acp;
mod airwallex;
mod airwallex_ingress;
mod approval;
mod attention;
mod authority;
mod capability_sourcing;
mod company;
mod context;
mod conversation;
mod credential;
mod effect;
mod exec;
mod finance;
mod health;
mod inbound;
mod ingress;
mod legal;
mod model_gateway;
mod owner;
mod owner_brief;
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
    // Kernel spend-recovery fields. These are deliberately distinct from
    // Work ids and effect costs: hard-budget truth belongs to Authority.
    #[serde(default)]
    correction_id: Option<String>,
    #[serde(default)]
    request_ids: Vec<String>,
    #[serde(default)]
    delta_micro_usd: Option<i64>,
    #[serde(default)]
    apply: bool,
    #[serde(default)]
    include_retired: bool,
    // Work graph fields.
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    new_name: Option<String>,
    #[serde(default)]
    repo: Option<String>,
    // T8 effect fields.
    #[serde(default)]
    capability: Option<String>,
    #[serde(default)]
    effect_class: Option<String>,
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default)]
    artifacts: Option<Vec<String>>,
    #[serde(default)]
    secret_bindings: Option<std::collections::BTreeMap<String, String>>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    execution_no: Option<i32>,
    #[serde(default)]
    actor: Option<String>,
    // S03-T5 approval field.
    #[serde(default)]
    party: Option<String>,
    // Work actor fields: who owns the Attempt and what it thinks with.
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    priority: Option<i16>,
    #[serde(default)]
    expected_artifact: Option<String>,
    #[serde(default)]
    base_ref: Option<String>,
    #[serde(default)]
    integration_branch: Option<String>,
    #[serde(default)]
    worktree: Option<String>,
    #[serde(default)]
    attempt_limit: Option<i32>,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    revises: Vec<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    attempt: Option<String>,
    #[serde(default)]
    uri: Option<String>,
    #[serde(default)]
    digest: Option<String>,
    #[serde(default)]
    source_commit: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    argv: Option<Vec<String>>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    prepared: Option<String>,
    #[serde(default)]
    resume_when: Option<String>,
    #[serde(default)]
    owner_kind: Option<String>,
    #[serde(default)]
    headline: Option<String>,
    #[serde(default)]
    situation: Option<String>,
    #[serde(default)]
    impact: Option<String>,
    #[serde(default)]
    recommendation: Option<String>,
    #[serde(default)]
    no_action: Option<String>,
    #[serde(default)]
    uncertainty: Option<String>,
    #[serde(default)]
    deadline: Option<String>,
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
    "spend-correct",
    "company-create",
    "company-set",
    "company-unset",
    "credential-set",
    "legal-set",
    "finance-envelope-set",
    "finance-freeze",
    "finance-connect-airwallex",
    "work-review",
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

    fn ok_serialized(data: impl serde::Serialize) -> Self {
        match serde_json::to_value(data) {
            Ok(data) => Self::ok(data),
            Err(error) => Self::err(format!("encode response: {error}")),
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
    pub(crate) spend: spend::SpendLedger,
    pub(crate) authority: authority::AuthorityStore,
    pub(crate) orgintel: OrgIntelRegistry,
    pub(crate) staff: staff::StaffRegistry,
    /// Reconnectable live projections for owner/lead turns. Completed messages
    /// remain OrgIntel truth; this state is intentionally ephemeral.
    pub(crate) conversations: conversation::ConversationStreams,
    /// One wake at a time per company, however the wake was requested —
    /// the scheduler (T6) and the owner-typed socket path share this set.
    pub(crate) in_flight: schedule::InFlight,
}

/// TCP port the company containers reach the daemon on (T10). Next to the
/// model gateway's 7790; reachable as host.docker.internal from containers.
pub(crate) const COORD_TCP_PORT: u16 = 7791;

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
    let root = runtime::state_root();
    std::fs::create_dir_all(root.join("companies"))
        .with_context(|| format!("create state root {}", root.display()))?;
    // The current owner cockpit has one supported topology: direct loopback.
    // Refuse network configuration before starting provider or scheduler work.
    let owner_config = owner::OwnerConfig::from_env()?;

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
        conversations: conversation::ConversationStreams::default(),
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
    // OrgIntel LISTEN/NOTIFY events share one loop.
    tokio::spawn(schedule::run(std::sync::Arc::clone(&daemon)));

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
                        "model_failover" => {
                            config.model_failover = value
                                .split(',')
                                .map(str::trim)
                                .filter(|model| !model.is_empty())
                                .map(str::to_string)
                                .collect();
                            config.model_candidates().map(|_| ())
                        }
                        "spend_ceiling_usd" => value
                            .parse::<f64>()
                            .map(|parsed| config.spend_ceiling_usd = parsed)
                            .map_err(|_| anyhow::anyhow!("spend_ceiling_usd must be a number")),
                        _ if key.starts_with("credentials.") => {
                            config.credentials.insert(key[12..].to_string(), value.to_string());
                            Ok(())
                        }
                        _ => Err(anyhow::anyhow!(
                            "unknown company key {key:?}; use mission, model, model_failover, spend_ceiling_usd, or credentials.<binding>"
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
        "company-unset" => match request.state.as_deref() {
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
        "credential-set" => match (request.capability.as_deref(), request.body.as_deref()) {
            (Some(binding), Some(reference)) => {
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
        "legal-set" => match request.body.as_deref() {
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
        "finance-envelope-set" => match request.body.as_deref() {
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
        "finance-freeze" => match request.state.as_deref() {
            Some(currency) => match finance::set_frozen(
                &daemon.authority,
                company,
                currency,
                request.apply,
                "owner",
            )
            .await
            {
                Ok(envelope) => Response::ok_serialized(envelope),
                Err(error) => Response::err(format!("{error:#}")),
            },
            None => Response::err("finance-freeze needs a currency"),
        },
        "finance-connect-airwallex" => match request.body.as_deref() {
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
        "finance-reserve" => match request.body.as_deref() {
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
            request.key.as_deref(),
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
            request.key.as_deref(),
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
                    {
                        let mut claims = daemon.in_flight.lock().expect("in-flight guard");
                        if !claims.claim(company) {
                            return Response::err(format!(
                                "a wake is already in flight for {company}; \
                                 its outcome lands in the event stream"
                            ));
                        }
                    }
                    let _guard = schedule::WakeGuard::new(company, &daemon.in_flight);
                    let reason = request.reason.as_deref().unwrap_or("owner-requested wake");
                    match schedule::run_exec_turn(daemon, &config, &org, reason).await {
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
        "tell" => match request.body {
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
                let actors = if request.include_retired {
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
            request.as_actor.as_deref(),
            request.role.as_deref(),
            request.name.as_deref(),
            request.actor.as_deref(),
            request.reason.as_deref(),
        ) {
            (Some(actor_id), Some(role), Some(display), Some(created_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .create_actor(
                            actor_id,
                            role,
                            display,
                            request.model.as_deref(),
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
        "actor-retire" => match (
            request.as_actor.as_deref(),
            request.actor.as_deref(),
            request.reason.as_deref(),
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
            request.name.as_deref(),
            request.to.as_deref(),
            request.body.as_deref(),
            request.actor.as_deref(),
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
            request.name.as_deref(),
            request.actor.as_deref(),
            request.reason.as_deref(),
        ) {
            (Some(team), Some(changed_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => match resolve_team(&org, team).await {
                        Ok(id) => match org
                            .update_team(
                                id,
                                request.new_name.as_deref(),
                                request.body.as_deref(),
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
            request.as_actor.as_deref(),
            request.name.as_deref(),
            request.actor.as_deref(),
            request.reason.as_deref(),
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
            request.name.as_deref(),
            request.to.as_deref(),
            request.actor.as_deref(),
            request.reason.as_deref(),
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
            request.name.as_deref(),
            request.actor.as_deref(),
            request.reason.as_deref(),
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
        "judgement" => match request.as_actor.as_deref() {
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
            request.id.as_deref().map(uuid::Uuid::parse_str).transpose(),
            request.as_actor.as_deref(),
            request.reason.as_deref(),
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
                let wanted = request.capability.as_deref();
                let limit = request.limit.unwrap_or(50).max(1) as usize;
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
        "spend" => match (
            runtime::CompanyConfig::load(&daemon.root, company),
            daemon.orgintel.get(company).await,
        ) {
            (Ok(config), Ok(org)) => match org.list_actors().await {
                Ok(actors) => {
                    let roles: HashMap<String, String> = actors
                        .into_iter()
                        .map(|actor| (actor.id, actor.role))
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
        "spend-correct" => {
            if let Err(error) = runtime::CompanyConfig::load(&daemon.root, company) {
                return Response::err(format!("{error:#}"));
            }
            let Some(correction_id) = request.correction_id.as_deref() else {
                return Response::err("spend-correct needs --correction-id");
            };
            let Ok(correction_id) = uuid::Uuid::parse_str(correction_id) else {
                return Response::err("spend-correct correction id must be a UUID");
            };
            let request_ids = match request
                .request_ids
                .iter()
                .map(|request_id| uuid::Uuid::parse_str(request_id))
                .collect::<std::result::Result<Vec<_>, _>>()
            {
                Ok(request_ids) => request_ids,
                Err(_) => return Response::err("spend-correct request ids must be UUIDs"),
            };
            let Some(delta_micro_usd) = request.delta_micro_usd else {
                return Response::err("spend-correct needs --delta-micro-usd");
            };
            let Some(reason) = request.reason.as_deref() else {
                return Response::err("spend-correct needs --reason");
            };
            if request.apply {
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
            request.title.as_deref(),
            request.body.as_deref(),
            request.actor.as_deref(),
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
            request.id.as_deref(),
            request.goal.as_deref(),
            request.actor.as_deref(),
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
                let work_id = match request.id.as_deref().map(uuid::Uuid::parse_str).transpose() {
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
            request.id.as_deref().map(uuid::Uuid::parse_str).transpose(),
            request.to.as_deref(),
            request.actor.as_deref(),
            request.reason.as_deref(),
        ) {
            (Ok(Some(work_id)), Some(new_owner), Some(changed_by), Some(reason)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .reassign_work(work_id, new_owner, changed_by, reason)
                        .await
                    {
                        Ok(previous_owner) => Response::ok(serde_json::json!({
                            "work_id": work_id,
                            "from_actor_id": previous_owner,
                            "to_actor_id": new_owner,
                        })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            (Err(error), _, _, _) => Response::err(format!("bad Work id: {error}")),
            _ => Response::err("work-assign needs Work id, new owner, reason and acting actor"),
        },
        "work-add" => match (
            request.actor.as_deref(),
            request.role.as_deref(),
            request.title.as_deref(),
            request.body.as_deref(),
        ) {
            (Some(owner), Some(role), Some(title), Some(outcome)) => {
                match daemon.orgintel.get(company).await {
                    Ok(org) => {
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
                        if let Some(requested_model) = request.model.as_deref() {
                            if actor.model.as_deref() != Some(requested_model) {
                                return Response::err(format!(
                                    "Work requested model {requested_model:?}, but durable actor {owner:?} uses {:?}; model changes belong to the actor/session, not a Work assignment",
                                    actor.model
                                ));
                            }
                        }
                        let goal_id = match request.goal.as_deref() {
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
                            priority: request.priority.unwrap_or(0),
                            expected_artifact: request.expected_artifact.as_deref().unwrap_or(""),
                            workspace: restless_orgintel::WorkspaceSpec {
                                repo: request.repo,
                                base_ref: request.base_ref,
                                integration_branch: request.integration_branch,
                                worktree: request.worktree,
                            },
                            attempt_limit: request.attempt_limit,
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
                        let requires = match parse_edges(&request.requires, "required") {
                            Ok(values) => values,
                            Err(error) => return Response::err(error),
                        };
                        let revises = match parse_edges(&request.revises, "revised") {
                            Ok(values) => values,
                            Err(error) => return Response::err(error),
                        };
                        match org.add_work_with_edges(work, &requires, &revises).await {
                            Ok(id) => Response::ok(serde_json::json!({ "work_id": id })),
                            Err(error) => Response::err(format!("{error:#}")),
                        }
                    }
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("work-add needs owner, role, title and outcome"),
        },
        "work-edge" => match (
            request.from.as_deref(),
            request.to.as_deref(),
            request.kind.as_deref(),
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
                        if request.action.as_deref() == Some("remove") {
                            let Some(changed_by) = request.as_actor.as_deref() else {
                                return Response::err("removing a Work edge needs --as");
                            };
                            let Some(reason) = request.reason.as_deref() else {
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
            request.id.as_deref(),
            request.attempt.as_deref(),
            request.kind.as_deref(),
            request.uri.as_deref(),
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
                            note: request.body.as_deref().unwrap_or(""),
                            created_by: request.actor.as_deref().unwrap_or("owner"),
                            work_id: Some(work_id),
                            attempt_id: Some(attempt_id),
                            digest: request.digest.as_deref(),
                            source_commit: request.source_commit.as_deref(),
                            runtime_generation: None,
                            label: request.label.as_deref().unwrap_or("output"),
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
            request.id.as_deref(),
            request.name.as_deref(),
            request.cwd.as_deref(),
            request.argv.as_deref(),
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
                            created_by: request.actor.as_deref().unwrap_or("owner"),
                        })
                        .await
                    {
                        Ok(id) => Response::ok(serde_json::json!({ "gate_id": id })),
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("work-gate needs work, name, cwd and command"),
        },
        "work-handoff" => match (
            request.id.as_deref(),
            request.category.as_deref(),
            request.action.as_deref(),
            request.prepared.as_deref(),
            request.resume_when.as_deref(),
        ) {
            (Some(work), Some(category), Some(action), Some(prepared), Some(resume_when)) => {
                let Ok(work_id) = uuid::Uuid::parse_str(work) else {
                    return Response::err("bad Work id");
                };
                let attempt_id = match request
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
                            requested_by: request.actor.as_deref().unwrap_or("owner"),
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
            request.id.as_deref(),
            request.as_actor.as_deref(),
            request.action.as_deref(),
            request.prepared.as_deref(),
            request.resume_when.as_deref(),
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
            request.id.as_deref(),
            request.as_actor.as_deref(),
            request.owner_kind.as_deref(),
            request.headline.as_deref(),
            request.situation.as_deref(),
            request.impact.as_deref(),
            request.recommendation.as_deref(),
            request.no_action.as_deref(),
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
                    uncertainty: request.uncertainty.clone(),
                    deadline: request.deadline.clone(),
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
            request.id.as_deref(),
            request.state.as_deref(),
            request.resolution.as_deref(),
            request.as_actor.as_deref(),
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
        "work-resume" => match (
            request.id.as_deref().map(uuid::Uuid::parse_str).transpose(),
            request.as_actor.as_deref(),
            request.reason.as_deref(),
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
            request.id.as_deref().map(uuid::Uuid::parse_str).transpose(),
            request.as_actor.as_deref(),
            request.reason.as_deref(),
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
        "work-review" => match (request.id.as_deref(), request.state.as_deref()) {
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
            _ => Response::err("work-review needs handoff and accept|request_changes decision"),
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
                Ok(org) => {
                    let result = match request.id.as_deref() {
                        Some(work_id) => {
                            let Ok(work_id) = uuid::Uuid::parse_str(work_id) else {
                                return Response::err("bad Work id on message");
                            };
                            match request.to.as_deref() {
                                Some(to) => org.send_work_message(&from, to, work_id, &body).await,
                                None => org.send_work_message_to_owner(&from, work_id, &body).await,
                            }
                        }
                        None => org.send_message(&from, request.to.as_deref(), &body).await,
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
        "events" => match daemon.orgintel.get(company).await {
            Ok(org) => match org.list_events(request.limit.unwrap_or(50)).await {
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
        "effect" => match (
            request.effect_class,
            request.purpose,
            request.key,
            request.cwd,
            request.argv,
        ) {
            (Some(effect_class), Some(purpose), Some(key), Some(cwd), Some(argv)) => {
                let actor = request.actor.as_deref().unwrap_or("owner");
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
                            request.party.as_deref(),
                            &purpose,
                            request.artifacts.unwrap_or_default(),
                            &cwd,
                            argv,
                            request.secret_bindings.unwrap_or_default(),
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
            request.key.as_deref(),
            request.execution_no,
            request.state.as_deref(),
            request.id.as_deref(),
        ) {
            (Some(key), Some(execution_no), Some(result), Some(evidence_receipt)) => {
                match effect::reconcile_unknown(
                    &daemon.authority,
                    company,
                    key,
                    execution_no,
                    result,
                    evidence_receipt,
                    request.actor.as_deref().unwrap_or("owner"),
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
