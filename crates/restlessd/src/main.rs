//! restlessd — the stable coordination core (ARCHITECTURE.md §4.4).
//!
//! Sprint 01 slice: company environment lifecycle over a unix socket.
//! JSON-lines protocol: one request line, one response line.

mod runtime;

use anyhow::{Context, Result};
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

#[tokio::main]
async fn main() -> Result<()> {
    let root = runtime::state_root();
    std::fs::create_dir_all(root.join("companies"))
        .with_context(|| format!("create state root {}", root.display()))?;
    let sock = root.join("restlessd.sock");
    if sock.exists() {
        std::fs::remove_file(&sock)?;
    }
    let listener = UnixListener::bind(&sock).with_context(|| format!("bind {}", sock.display()))?;
    eprintln!("restlessd: listening on {}", sock.display());

    loop {
        let (stream, _) = listener.accept().await?;
        let root = root.clone();
        tokio::spawn(async move {
            if let Err(error) = serve(stream, &root).await {
                eprintln!("restlessd: connection error: {error:#}");
            }
        });
    }
}

async fn serve(stream: tokio::net::UnixStream, root: &std::path::Path) -> Result<()> {
    let (read, mut write) = stream.into_split();
    let mut lines = BufReader::new(read).lines();
    while let Some(line) = lines.next_line().await? {
        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => dispatch(request, root).await,
            Err(error) => Response::err(format!("bad request: {error}")),
        };
        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        write.write_all(out.as_bytes()).await?;
    }
    Ok(())
}

async fn dispatch(request: Request, root: &std::path::Path) -> Response {
    let company = match request.company.as_deref() {
        Some(name) => name,
        None => return Response::err("missing company"),
    };
    match request.cmd.as_str() {
        "up" => match runtime::CompanyConfig::load(root, company) {
            Ok(config) => match runtime::up(&config).await {
                Ok(message) => Response::ok(message),
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
        other => Response::err(format!("unknown command {other:?}")),
    }
}
