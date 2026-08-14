//! restlessd — the stable coordination core (ARCHITECTURE.md §4.4).
//!
//! Sprint 01 slice: company environment lifecycle over a unix socket, plus
//! the embedded model gateway (T2). JSON-lines protocol: one request line,
//! one response line.

mod acp;
mod health;
mod reconcile;
mod context;
mod effect;
mod exec;
mod schedule;
mod spend;
mod runtime;
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
}

#[derive(Debug, Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl Response {
    fn ok(data: impl Into<serde_json::Value>) -> Self {
        Self { ok: true, data: Some(data.into()), error: None }
    }
    fn err(message: impl Into<String>) -> Self {
        Self { ok: false, data: None, error: Some(message.into()) }
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
        let config = Self { database_url: format!("postgres://{user}@localhost/restless") };
        let rendered = toml::to_string_pretty(&config).context("render orgintel.toml")?;
        std::fs::write(&path, rendered).with_context(|| format!("seed {}", path.display()))?;
        Ok(config)
    }
}

/// Lazily ensured per-company OrgIntel handles (one pool per company).
struct OrgIntelRegistry {
    pub(crate) database_url: String,
    handles: std::sync::Mutex<HashMap<String, OrgIntel>>,
}

impl OrgIntelRegistry {
    async fn get(&self, company: &str) -> Result<OrgIntel> {
        let cached = self.handles.lock().expect("orgintel registry").get(company).cloned();
        if let Some(handle) = cached {
            // A cached handle can outlive its schema: a scenario reset, an
            // operator drop, or a restore removes the tables and every later
            // query fails with `relation "actors" does not exist`. Re-ensure
            // instead of serving a handle to nothing.
            if handle.is_live().await {
                return Ok(handle);
            }
            tracing::warn!(company, "orgintel schema vanished under a cached handle; re-ensuring");
        }
        let handle = OrgIntel::ensure(&self.database_url, company).await?;
        self.handles
            .lock()
            .expect("orgintel registry")
            .insert(company.to_string(), handle.clone());
        Ok(handle)
    }
}

pub(crate) struct Daemon {
    pub(crate) root: PathBuf,
    pub(crate) spend: spend::SpendLedger,
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
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let root = runtime::state_root();
    std::fs::create_dir_all(root.join("companies"))
        .with_context(|| format!("create state root {}", root.display()))?;

    // T2: the model gateway is part of the coordination core. Without it no
    // agent can think, so a failed start is a failed daemon start.
    let spend = spend::SpendLedger::open(&root)?;
    // T5: coordination state. The database must answer at boot — probe,
    // never guess that it will be there when a company wakes.
    let orgintel_config = OrgIntelConfig::load_or_seed(&root)?;
    OrgIntel::probe(&orgintel_config.database_url)
        .await
        .context("orgintel database is not reachable at boot")?;

    let daemon = std::sync::Arc::new(Daemon {
        root: root.clone(),
        spend,
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
        // watch is the one streaming command: it owns the connection until
        // the client goes away, writing one JSON event per line.
        if request.cmd == "watch" {
            watch_events(&mut write, daemon, request.company.as_deref()).await?;
            continue;
        }
        let response = dispatch(request, daemon).await;
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

async fn dispatch(request: Request, daemon: &Daemon) -> Response {
    let company = match request.company.as_deref() {
        Some(name) => name,
        None => return Response::err("missing company"),
    };
    match request.cmd.as_str() {
        "up" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => match runtime::up(&config).await {
                Ok(message) => {
                    // Company up = environment AND coordination state ready.
                    match daemon.orgintel.get(company).await {
                        Ok(_) => Response::ok(message),
                        Err(error) => Response::err(format!(
                            "container up but orgintel schema failed: {error:#}"
                        )),
                    }
                }
                Err(error) => Response::err(format!("{error:#}")),
            },
            Err(error) => Response::err(format!("{error:#}")),
        },
        "down" => match runtime::down(company).await {
            Ok(message) => Response::ok(message),
            Err(error) => Response::err(format!("{error:#}")),
        },
        "status" => match runtime::status(company).await {
            Ok(status) => Response::ok(format!("{company}: {status:?}")),
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
                    match exec::wake(&config, &daemon.spend, &org, reason).await {
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
                        Ok(_) => Response::ok("delivered to the exec; the message wakes the company"),
                        Err(error) => Response::err(format!("{error:#}")),
                    }
                }
                Err(error) => Response::err(format!("{error:#}")),
            },
            None => Response::err("tell needs a body"),
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
                    other => return Response::err(format!("state must be completed|blocked, got {other:?}")),
                };
                let Ok(id) = uuid::Uuid::parse_str(&id) else {
                    return Response::err(format!("bad commitment id {id:?}"));
                };
                match daemon.orgintel.get(company).await {
                    Ok(org) => match org
                        .set_commitment_state(id, state, request.resolution.as_deref().unwrap_or(""))
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
        // T8: the effect surface. Ungoverned this sprint (accepted risk) —
        // the receipt and the idempotency replay are what exist.
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
        "spawn" => match (request.name, request.body) {
            (Some(name), Some(task)) => {
                match runtime::CompanyConfig::load(&daemon.root, company) {
                    Ok(config) => match daemon.orgintel.get(company).await {
                        Ok(org) => {
                            let ask = staff::SpawnRequest { name, task, repo: request.repo };
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
                }
            }
            _ => Response::err("spawn needs --name and a task".to_string()),
        },
        "effect" => match (request.capability, request.key) {
            (Some(capability), Some(key)) => {
                let actor = request.actor.as_deref().unwrap_or("owner");
                match runtime::CompanyConfig::load(&daemon.root, company) {
                    Ok(config) => match daemon.orgintel.get(company).await {
                        Ok(org) => match effect::request_effect(
                            &daemon.root,
                            &config,
                            &org,
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
                        },
                        Err(error) => Response::err(format!("{error:#}")),
                    },
                    Err(error) => Response::err(format!("{error:#}")),
                }
            }
            _ => Response::err("effect needs capability and key"),
        },
        other => Response::err(format!("unknown command {other:?}")),
    }
}
