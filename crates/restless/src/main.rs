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

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "restless", about = "Restless owner surface")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create and configure companies without hand-editing daemon state.
    Company {
        #[command(subcommand)]
        command: CompanyCommand,
    },
    /// Configure and probe secret references. Secret values remain in their backend.
    Credential {
        #[command(subcommand)]
        command: CredentialCommand,
    },
    /// Generate or rotate the one-owner web credential (shown once).
    OwnerToken {
        #[arg(
            long,
            help = "Required acknowledgement: invalidates the previous web credential"
        )]
        rotate: bool,
    },
    /// Bring a company environment up (create if absent, then start).
    Up {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Clone a live company's mission and config into this one as a
        /// throwaway (S04-T1). Target name must end in `_test`; every real
        /// provider, credential, standing approval and the sender address are
        /// stripped, so the worst outcome of a mistake is a simulated send.
        #[arg(long)]
        from: Option<String>,
        /// Rebuild the company image from this Restless source tree and
        /// replace an outdated container while preserving its volume.
        #[arg(long)]
        reconcile: bool,
    },
    /// Stop a company environment. The volume — files, Git, browser — survives.
    Down {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Remove container, volume, OrgIntel schema AND spend spool.
        /// Throwaway companies only — a live company's history is evidence.
        #[arg(long)]
        destroy: bool,
    },
    /// Company environment lifecycle status.
    Status {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Probe the runtime image and in-container CLI for version skew.
    Doctor {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Ensure and inspect the company's OrgIntel schema.
    OrgintelInit {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Outstanding owner work, projected from Authority and OrgIntel.
    Attention {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Persistent browser controller coordination and health.
    Browser {
        #[command(subcommand)]
        command: BrowserCommand,
    },
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
    Tell {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        body: String,
    },
    /// Stream the company's operational event stream (snapshot, then live).
    Watch {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Drop into a shell on the company computer.
    Attach {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Run an ordinary command on the company computer. Omit it for an
        /// interactive shell. This is the unbounded runtime door: do not grow
        /// one Restless verb per Linux command.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Who is in this company: role, model, and what each has cost (S04-T9).
    People {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// What the company's receipts actually record — the strongest evidence
    /// the system holds about what it did to the world.
    Receipts {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Only this capability, e.g. `email.send`.
        #[arg(long)]
        capability: Option<String>,
        #[arg(long, default_value = "50")]
        limit: i64,
    },
    /// Spend against the ceiling, broken down by actor and model.
    Spend {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// List goals.
    Goals {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// List commitments (all states).
    Commitments {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
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
        /// What this actor IS — `copywriter`, `critic`, `engineer`. Becomes its
        /// durable role, so the owner can ask who did what. Absent means a
        /// generalist, which is honest but is not a team.
        #[arg(long)]
        role: Option<String>,
        /// Provider-qualified model for this role. Absent inherits the
        /// company's. A critic running the producer's own model on the
        /// producer's own context is an echo chamber with a second invoice.
        #[arg(long)]
        model: Option<String>,
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
        /// Withdraw standing approval instead of granting it.
        #[arg(long)]
        revoke: bool,
        /// Close the current request without granting standing authority.
        #[arg(long, conflicts_with = "revoke")]
        decline: bool,
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

#[derive(Subcommand)]
enum CompanyCommand {
    /// Validate and create a company from the canonical TOML interface.
    Create {
        #[arg(long)]
        from_file: PathBuf,
    },
    /// Print canonical TOML; credential references are shown, never values.
    Show {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Set one deterministic configuration key: mission, model,
    /// spend_ceiling_usd, from_address, providers.<capability>, or
    /// credentials.<capability>. Name is immutable; create a new company.
    Set {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        key: String,
        value: String,
    },
    /// List configured companies.
    List,
}

#[derive(Subcommand)]
enum CredentialCommand {
    /// Store a capability's scheme:locator reference, never its value.
    Set {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        capability: String,
        reference: String,
        /// Forward a value from @<file> or `-` (stdin) to a supporting backend.
        /// Secret material is never accepted as an argv value.
        #[arg(long)]
        value: Option<String>,
    },
    /// Probe every configured reference as present, absent or invalid.
    Check {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
}

#[derive(Subcommand)]
enum BrowserCommand {
    /// Probe desktop, Chromium, automation and controller state.
    Status {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Yield the shared visible browser to the owner.
    Request {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long, env = "RESTLESS_ACTOR")]
        session: Option<String>,
    },
    /// Release an agent-held browser controller claim.
    Release {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
}

/// S04-T10. Who this invocation is, at the authority boundary.
///
/// Inside a company container `RESTLESS_COORDINATOR` is set by the image, and
/// that is already how the CLI knows where to connect — so it is also how it
/// knows what it is. On the host, the caller is the owner.
///
/// This is a claim, not a proof: on a single-operator host the container is
/// still trusted to say what it is. What changes is that it now *says* it, the
/// daemon acts on it, and the audit record carries it. Hardening the claim is
/// the Authority Kernel's job.
fn principal() -> &'static str {
    if std::env::var_os("RESTLESS_COORDINATOR").is_some() {
        "company/exec"
    } else {
        "owner"
    }
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
        Command::Attach {
            company: name,
            command,
        } => {
            let name = name.context("no company: pass -c or set RESTLESS_COMPANY")?;
            // `attach` is the generic door to the company computer. Docker is
            // the V0 Runtime Bridge implementation, not an owner operation.
            // A supplied command is deliberately passed through unchanged;
            // judgement and productive work stay in ordinary Linux.
            let mut args = vec!["exec".to_string()];
            if command.is_empty() {
                args.push("-it".to_string());
            } else {
                args.push("-i".to_string());
            }
            args.extend([
                "-u".to_string(),
                "company".to_string(),
                "-e".to_string(),
                format!("RESTLESS_COMPANY={name}"),
                "-e".to_string(),
                "RESTLESS_ACTOR=owner".to_string(),
                format!("restless-co-{name}"),
            ]);
            if command.is_empty() {
                args.push("bash".to_string());
            } else {
                args.extend(command);
            }
            let status = std::process::Command::new("docker")
                .args(&args)
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
            let request = stamp(serde_json::json!({ "cmd": "watch", "company": name }));
            watch(&request.to_string())
        }
        other => {
            let request = stamp(request_json(other)?);
            let response = request_once(&request.to_string())?;
            print_response(&response)
        }
    }
}

/// Every request carries its principal. One place, so a new command cannot
/// forget to — the daemon rejects an unstamped request rather than defaulting
/// one, which makes forgetting loud instead of silently privileged.
fn stamp(mut request: serde_json::Value) -> serde_json::Value {
    if let Some(object) = request.as_object_mut() {
        object.insert("principal".into(), principal().into());
    }
    request
}

/// One request/response pair; `watch` and `attach` handle their own I/O.
fn request_json(command: Command) -> Result<serde_json::Value> {
    Ok(match command {
        Command::Company { command } => match command {
            CompanyCommand::Create { from_file } => {
                let body = std::fs::read_to_string(&from_file)
                    .with_context(|| format!("read {}", from_file.display()))?;
                let value: toml::Value = toml::from_str(&body)
                    .with_context(|| format!("parse {}", from_file.display()))?;
                let company = value
                    .get("name")
                    .and_then(toml::Value::as_str)
                    .context("company TOML needs a string name")?;
                serde_json::json!({ "cmd": "company-create", "company": company, "body": body })
            }
            CompanyCommand::Show { company } => {
                serde_json::json!({ "cmd": "company-show", "company": company })
            }
            CompanyCommand::Set {
                company,
                key,
                value,
            } => serde_json::json!({
                "cmd": "company-set", "company": company, "state": key, "body": value,
            }),
            CompanyCommand::List => serde_json::json!({ "cmd": "company-list" }),
        },
        Command::Credential { command } => match command {
            CredentialCommand::Set {
                company,
                capability,
                reference,
                value,
            } => {
                let secret_value = value
                    .map(|source| read_secret_source(&source))
                    .transpose()?;
                serde_json::json!({
                    "cmd": "credential-set", "company": company,
                    "capability": capability, "body": reference,
                    "secret_value": secret_value,
                })
            }
            CredentialCommand::Check { company } => {
                serde_json::json!({ "cmd": "credential-check", "company": company })
            }
        },
        Command::OwnerToken { rotate } => {
            if !rotate {
                bail!("pass --rotate to generate a new owner credential; only its digest will be stored");
            }
            serde_json::json!({ "cmd": "owner-token" })
        }
        Command::Up {
            company: c,
            from,
            reconcile,
        } => {
            serde_json::json!({
                "cmd": "up",
                "company": c,
                "from_company": from,
                "reconcile": reconcile,
            })
        }
        Command::Down {
            company: c,
            destroy,
        } => {
            serde_json::json!({ "cmd": "down", "company": c, "destroy": destroy })
        }
        Command::Status { company: c } => {
            serde_json::json!({ "cmd": "status", "company": c })
        }
        Command::Doctor { company: c } => {
            serde_json::json!({ "cmd": "doctor", "company": c })
        }
        Command::OrgintelInit { company: c } => {
            serde_json::json!({ "cmd": "orgintel-init", "company": c })
        }
        Command::Attention { company: c } => {
            serde_json::json!({ "cmd": "attention", "company": c })
        }
        Command::Browser { command } => match command {
            BrowserCommand::Status { company } => {
                serde_json::json!({ "cmd": "browser-status", "company": company })
            }
            BrowserCommand::Request { company, session } => serde_json::json!({
                "cmd": "browser-request", "company": company,
                "id": session.unwrap_or_else(|| "exec".to_string()),
            }),
            BrowserCommand::Release { company } => {
                serde_json::json!({ "cmd": "browser-release", "company": company })
            }
        },
        Command::Wake { company: c, reason } => {
            serde_json::json!({ "cmd": "wake", "company": c, "reason": reason })
        }
        Command::Tell { company: c, body } => {
            serde_json::json!({ "cmd": "tell", "company": c, "body": body })
        }
        Command::People { company: c } => serde_json::json!({ "cmd": "people", "company": c }),
        Command::Spend { company: c } => serde_json::json!({ "cmd": "spend", "company": c }),
        Command::Receipts {
            company: c,
            capability,
            limit,
        } => serde_json::json!({
            "cmd": "receipts", "company": c, "capability": capability, "limit": limit,
        }),
        Command::Goals { company: c } => {
            serde_json::json!({ "cmd": "goals", "company": c })
        }
        Command::Commitments { company: c } => {
            serde_json::json!({ "cmd": "commitments", "company": c })
        }
        Command::Events { company: c, limit } => {
            serde_json::json!({ "cmd": "events", "company": c, "limit": limit })
        }
        Command::Inbox {
            company: c,
            as_actor,
        } => {
            serde_json::json!({ "cmd": "inbox", "company": c, "as_actor": as_actor })
        }
        Command::ClearPoison { company: c } => serde_json::json!({
            "cmd": "clear-poison",
            "company": c,
        }),
        Command::Approve {
            company: c,
            party,
            revoke,
            decline,
        } => serde_json::json!({
            "cmd": if revoke { "revoke" } else if decline { "decline" } else { "approve" },
            "company": c,
            "party": party,
        }),
        Command::Spawn {
            company: c,
            name,
            repo,
            role,
            model,
            task,
        } => serde_json::json!({
            "cmd": "spawn",
            "company": c,
            "name": name,
            "repo": repo,
            "role": role,
            "model": model,
            "body": task,
            "from": std::env::var("RESTLESS_ACTOR").ok(),
        }),
        Command::Message {
            company: c,
            from,
            to,
            body,
        } => serde_json::json!({
            "cmd": "message",
            "company": c,
            "from": from.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                .unwrap_or_else(|| "owner".to_string()),
            "to": to,
            "body": body,
        }),
        Command::Commitment {
            company: c,
            state,
            id,
            resolution,
        } => serde_json::json!({
            "cmd": "commitment-state",
            "company": c,
            "id": id,
            "state": state,
            "resolution": resolution,
        }),
        Command::Effect {
            company: c,
            capability,
            args,
            key,
        } => {
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

fn read_secret_source(source: &str) -> Result<String> {
    if source == "-" {
        let mut value = String::new();
        std::io::stdin()
            .read_to_string(&mut value)
            .context("read secret value from stdin")?;
        return Ok(value);
    }
    let path = source
        .strip_prefix('@')
        .filter(|path| !path.is_empty())
        .context("--value accepts only @<file> or - (stdin), never raw secret material")?;
    std::fs::read_to_string(path).with_context(|| format!("read secret value from {path}"))
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
        bail!("{}", render_error(&parsed["error"]))
    }
}

/// Errors are `{kind, message}` (S04-T10). The kind is shown, not swallowed:
/// an owner who sees `[authority]` knows to change what they are allowed to do,
/// and one who sees `[transport]` knows to check the daemon — the distinction
/// prose cannot carry reliably.
fn render_error(error: &serde_json::Value) -> String {
    let message = error["message"].as_str().unwrap_or("unknown error");
    match error["kind"].as_str() {
        Some(kind) if kind != "error" => format!("[{kind}] {message}"),
        _ => message.to_string(),
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
            bail!("{}", render_error(&parsed["error"]));
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
