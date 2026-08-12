//! restlessd — the stable coordination core (ARCHITECTURE.md §4.4).
//!
//! Sprint 01 slice: company environment lifecycle over a unix socket, plus
//! the embedded model gateway (T2). JSON-lines protocol: one request line,
//! one response line.

mod gateway;
mod runtime;

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
    database_url: String,
    handles: std::sync::Mutex<HashMap<String, OrgIntel>>,
}

impl OrgIntelRegistry {
    async fn get(&self, company: &str) -> Result<OrgIntel> {
        if let Some(handle) = self.handles.lock().expect("orgintel registry").get(company) {
            return Ok(handle.clone());
        }
        let handle = OrgIntel::ensure(&self.database_url, company).await?;
        self.handles
            .lock()
            .expect("orgintel registry")
            .insert(company.to_string(), handle.clone());
        Ok(handle)
    }
}

struct Daemon {
    root: PathBuf,
    gateway: gateway::GatewayHandle,
    orgintel: OrgIntelRegistry,
}

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
    let gateway = gateway::start(&root).await?;
    // T5: coordination state. The database must answer at boot — probe,
    // never guess that it will be there when a company wakes.
    let orgintel_config = OrgIntelConfig::load_or_seed(&root)?;
    OrgIntel::probe(&orgintel_config.database_url)
        .await
        .context("orgintel database is not reachable at boot")?;

    let daemon = std::sync::Arc::new(Daemon {
        root: root.clone(),
        gateway,
        orgintel: OrgIntelRegistry {
            database_url: orgintel_config.database_url,
            handles: std::sync::Mutex::new(HashMap::new()),
        },
    });

    let sock = root.join("restlessd.sock");
    if sock.exists() {
        std::fs::remove_file(&sock)?;
    }
    let listener = UnixListener::bind(&sock).with_context(|| format!("bind {}", sock.display()))?;
    tracing::info!(socket = %sock.display(), "restlessd listening");

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

async fn serve(stream: tokio::net::UnixStream, daemon: &Daemon) -> Result<()> {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read).lines();
    while let Some(line) = lines.next_line().await? {
        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => dispatch(request, daemon).await,
            Err(error) => Response::err(format!("bad request: {error}")),
        };
        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        write.write_all(out.as_bytes()).await?;
    }
    Ok(())
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
        // T2 acceptance seam (also the T4 agent-wake path): mint one ≤1h
        // purpose token for the company's configured model.
        "mint-token" => match runtime::CompanyConfig::load(&daemon.root, company) {
            Ok(config) => match daemon.gateway.mint_token(&config, "exec") {
                Ok(minted) => match serde_json::to_value(&minted) {
                    Ok(value) => Response::ok(value),
                    Err(error) => Response::err(format!("encode token: {error}")),
                },
                Err(error) => Response::err(format!("{error:#}")),
            },
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
        other => Response::err(format!("unknown command {other:?}")),
    }
}
