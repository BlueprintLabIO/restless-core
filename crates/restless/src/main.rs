//! restless — one surface, two consumers (sprint 01 T10). The owner drives
//! the company from the host over the daemon's unix socket; agents inside
//! the company container drive coordination through the same binary over
//! TCP (unix sockets do not survive the Docker Desktop file share — probed,
//! not guessed). The CLI is a dumb client: the trust boundary is the
//! daemon's listeners, not this binary (§6.1).
//!
//! Environment defaults (so agents can just type `restless commitments`):
//!   RESTLESS_COMPANY      — whose coordination state to touch
//!   RESTLESS_ACTOR        — who "message send" is from
//!   RESTLESS_COORDINATOR  — host:port; when set, TCP instead of unix socket

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "restless", about = "Restless owner surface")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Bring a company environment up (create if absent, then start).
    Up { #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String> },
    /// Stop a company environment. The volume — files, Git, browser — survives.
    Down { #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String> },
    /// Company environment status.
    Status { #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String> },
    /// Wake the company's Exec for one turn (rehydrate → work → decide).
    Wake {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Why this wake is happening; defaults to an owner-requested wake.
        #[arg(long)]
        reason: Option<String>,
    },
    /// Issue a directive to the Exec — also how a blocked judgement request
    /// is answered. Wakes the company.
    Tell { #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>, body: String },
    /// Stream the company's operational event stream (snapshot, then live).
    Watch { #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String> },
    /// Drop into a shell on the company computer.
    Attach { #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String> },
    /// List goals.
    Goals { #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String> },
    /// List commitments (all states).
    Commitments { #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String> },
    /// Recent events from the operational stream (newest first).
    Events {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long, default_value = "50")]
        limit: i64,
    },
    /// Read unread mail — yours, or an actor's with --as. Reading marks read.
    Inbox {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long = "as")]
        as_actor: Option<String>,
    },
    /// Send mail between actors (omit --to for the owner).
    Message {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long, env = "RESTLESS_ACTOR")]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        body: String,
    },
    /// Report a commitment completed or blocked (the agents' report path).
    Commitment {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        state: String,
        id: String,
        #[arg(long, default_value = "")]
        resolution: String,
    },
    /// Hand a task to a staff member (S02-T2). Delegation is a tool the Exec
    /// reaches for mid-turn, like every other capability — not a field in the
    /// end-of-turn envelope, which is why three sprint-01 runs decomposed work
    /// correctly and dispatched none of it.
    Spawn {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Staff name: lowercase, digits and dashes. Becomes the actor id,
        /// the worktree path, and the branch name.
        #[arg(long)]
        name: String,
        /// Repository under /company/repos to give this staff a worktree of.
        /// Omit for non-code work.
        #[arg(long)]
        repo: Option<String>,
        /// What to do and why, in enough detail to work unsupervised.
        task: String,
    },
    /// Clear a fail-closed spend poison after inspecting why it happened.
    /// A poison stops a company dead; without this it stops it forever.
    ClearPoison {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Approve a party for real external effects (S03-T5). First contact
    /// through a real provider needs a human yes; this is that yes, and it is
    /// per-party rather than per-send so the owner governs rather than
    /// dispatches (`owner-cockpit` §2.3).
    Approve {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// The address or party identifier being approved.
        #[arg(long)]
        party: String,
    },
    /// Request an external effect (T8). Args are JSON.
    Effect {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        capability: String,
        #[arg(long, default_value = "{}")]
        args: String,
        /// Idempotency key: a retry with the same key replays the receipt.
        #[arg(long)]
        key: String,
    },
}

fn state_root() -> PathBuf {
    if let Ok(root) = std::env::var("RESTLESS_HOME") {
        return PathBuf::from(root);
    }
    let home = std::env::var("HOME").expect("HOME");
    PathBuf::from(home).join(".restless")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Attach { company: name } => {
            let name = name.context("no company: pass -c or set RESTLESS_COMPANY")?;
            // Owner-only path: a shell on the company computer, straight
            // through docker. The coordination env travels so the CLI works
            // inside the shell too.
            let status = std::process::Command::new("docker")
                .args([
                    "exec", "-it", "-u", "company",
                    "-e", &format!("RESTLESS_COMPANY={name}"),
                    "-e", "RESTLESS_ACTOR=owner",
                    &format!("restless-co-{name}"),
                    "bash",
                ])
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()
                .context("docker exec — is docker running?")?;
            if !status.success() {
                bail!("attach failed (is the company up?)");
            }
            Ok(())
        }
        Command::Watch { company: name } => {
            let name = name.context("no company: pass -c or set RESTLESS_COMPANY")?;
            let request = serde_json::json!({ "cmd": "watch", "company": name });
            watch(&request.to_string())
        }
        other => {
            let request = request_json(other)?;
            let response = request_once(&request.to_string())?;
            print_response(&response)
        }
    }
}

/// One request/response pair; `watch` and `attach` handle their own I/O.
fn request_json(command: Command) -> Result<serde_json::Value> {
    Ok(match command {
        Command::Up { company: c } => serde_json::json!({ "cmd": "up", "company": c }),
        Command::Down { company: c } => serde_json::json!({ "cmd": "down", "company": c }),
        Command::Status { company: c } => {
            serde_json::json!({ "cmd": "status", "company": c })
        }
        Command::Wake { company: c, reason } => {
            serde_json::json!({ "cmd": "wake", "company": c, "reason": reason })
        }
        Command::Tell { company: c, body } => {
            serde_json::json!({ "cmd": "tell", "company": c, "body": body })
        }
        Command::Goals { company: c } => {
            serde_json::json!({ "cmd": "goals", "company": c })
        }
        Command::Commitments { company: c } => {
            serde_json::json!({ "cmd": "commitments", "company": c })
        }
        Command::Events { company: c, limit } => {
            serde_json::json!({ "cmd": "events", "company": c, "limit": limit })
        }
        Command::Inbox { company: c, as_actor } => {
            serde_json::json!({ "cmd": "inbox", "company": c, "as_actor": as_actor })
        }
        Command::ClearPoison { company: c } => serde_json::json!({
            "cmd": "clear-poison",
            "company": c,
        }),
        Command::Approve { company: c, party } => serde_json::json!({
            "cmd": "approve",
            "company": c,
            "party": party,
        }),
        Command::Spawn { company: c, name, repo, task } => serde_json::json!({
            "cmd": "spawn",
            "company": c,
            "name": name,
            "repo": repo,
            "body": task,
            "from": std::env::var("RESTLESS_ACTOR").ok(),
        }),
        Command::Message { company: c, from, to, body } => serde_json::json!({
            "cmd": "message",
            "company": c,
            "from": from.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                .unwrap_or_else(|| "owner".to_string()),
            "to": to,
            "body": body,
        }),
        Command::Commitment { company: c, state, id, resolution } => serde_json::json!({
            "cmd": "commitment-state",
            "company": c,
            "id": id,
            "state": state,
            "resolution": resolution,
        }),
        Command::Effect { company: c, capability, args, key } => {
            let args: serde_json::Value =
                serde_json::from_str(&args).context("--args must be JSON")?;
            serde_json::json!({
                "cmd": "effect",
                "company": c,
                "capability": capability,
                "args": args,
                "key": key,
                "actor": std::env::var("RESTLESS_ACTOR").unwrap_or_else(|_| "owner".to_string()),
            })
        }
        Command::Watch { .. } | Command::Attach { .. } => unreachable!("handled above"),
    })
}

fn print_response(response: &str) -> Result<()> {
    let parsed: serde_json::Value = serde_json::from_str(response).context("parse response")?;
    if parsed["ok"].as_bool() == Some(true) {
        match &parsed["data"] {
            serde_json::Value::String(message) => println!("{message}"),
            other => println!("{}", serde_json::to_string_pretty(other)?),
        }
        Ok(())
    } else {
        bail!("{}", parsed["error"].as_str().unwrap_or("unknown error"))
    }
}

/// Stream the event log: one rendered line per event until the daemon goes
/// away or the owner interrupts.
fn watch(line: &str) -> Result<()> {
    let mut stream = connect()?;
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut reader = BufReader::new(stream);
    let mut event = String::new();
    loop {
        event.clear();
        if reader.read_line(&mut event)? == 0 {
            return Ok(()); // daemon closed
        }
        let parsed: serde_json::Value = match serde_json::from_str(event.trim()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if parsed["ok"].as_bool() == Some(false) {
            bail!("{}", parsed["error"].as_str().unwrap_or("watch failed"));
        }
        let at = parsed["created_at"].as_str().unwrap_or("");
        let kind = parsed["kind"].as_str().unwrap_or("?");
        let actor = parsed["actor_id"].as_str().unwrap_or("-");
        println!("{at} {kind:<20} {actor:<12} {}", parsed["body"]);
    }
}

enum Stream {
    Unix(UnixStream),
    Tcp(std::net::TcpStream),
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Stream::Unix(stream) => stream.write(buf),
            Stream::Tcp(stream) => stream.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Stream::Unix(stream) => stream.flush(),
            Stream::Tcp(stream) => stream.flush(),
        }
    }
}

impl std::io::Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Stream::Unix(stream) => stream.read(buf),
            Stream::Tcp(stream) => stream.read(buf),
        }
    }
}

/// Inside a company container the coordinator is TCP (RESTLESS_COORDINATOR
/// is set in the image); on the host it is the unix socket.
fn connect() -> Result<Stream> {
    if let Ok(coordinator) = std::env::var("RESTLESS_COORDINATOR") {
        return std::net::TcpStream::connect(&coordinator)
            .map(Stream::Tcp)
            .with_context(|| format!("connect {coordinator} — is restlessd running?"));
    }
    let sock = state_root().join("restlessd.sock");
    UnixStream::connect(&sock)
        .map(Stream::Unix)
        .with_context(|| format!("connect {} — is restlessd running?", sock.display()))
}

fn request_once(line: &str) -> Result<String> {
    let mut stream = connect()?;
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    Ok(response)
}
