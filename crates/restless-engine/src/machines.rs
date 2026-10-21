//! Machines the owner holds, assigned to companies (S99).
//!
//! A machine is a Docker host that can run company computers. The plane's own host is always the
//! first, [`PLANE_MACHINE`]; the owner pairs more (a PC, a Mac mini, an office server) with a
//! one-time code, and each then holds a credential stored here only as a hash. A company is
//! assigned machines in roles: exactly one `main`, which holds its computer, and later `helper`s.
//! A company with no assignment runs on the plane, as every company did before machines.
//!
//! A paired machine keeps one link open to the plane ([`serve_link`]). Over it the plane drives the
//! machine's Docker (filtered by the machine to Restless's own objects; see
//! `restless_machine::docker_filter`), connects to its containers' ports, sends it images, and
//! carries its company containers' calls to the plane's own ports.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use anyhow::{bail, Context as _, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use restless_machine::link::{Link, LinkStream};
use restless_machine::protocol::{
    AllowedImage, Control, MachineFacts, PairResponse, STREAM_CONNECT, STREAM_DOCKER, STREAM_IMAGE,
    STREAM_PLANE,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Row};
use tokio::sync::mpsc;
use uuid::Uuid;

/// The plane's own host, always present, never paired or removed.
pub const PLANE_MACHINE: &str = "plane";
pub const ROLE_MAIN: &str = "main";
pub const ROLE_HELPER: &str = "helper";
const PAIRING_MINUTES: i64 = 10;
/// Readable, unambiguous code letters: no 0/O, 1/I/L.
const CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
const MAX_LABEL: usize = 60;

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    for statement in [
        "CREATE TABLE IF NOT EXISTS restless_authority.machines (\
           id TEXT PRIMARY KEY, label TEXT NOT NULL, kind TEXT NOT NULL, \
           facts JSONB NOT NULL DEFAULT '{}'::jsonb, observed_address TEXT, \
           paused BOOLEAN NOT NULL DEFAULT FALSE, credential_hash TEXT, \
           credential_epoch BIGINT NOT NULL DEFAULT 1, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), last_seen_at TIMESTAMPTZ, removed_at TIMESTAMPTZ)",
        "CREATE UNIQUE INDEX IF NOT EXISTS machines_by_credential \
           ON restless_authority.machines (credential_hash) WHERE credential_hash IS NOT NULL",
        "CREATE TABLE IF NOT EXISTS restless_authority.machine_pairings (\
           code_hash TEXT PRIMARY KEY, label TEXT, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           expires_at TIMESTAMPTZ NOT NULL, consumed_at TIMESTAMPTZ, machine_id TEXT)",
        "CREATE TABLE IF NOT EXISTS restless_authority.company_machines (\
           company TEXT NOT NULL, machine_id TEXT NOT NULL REFERENCES restless_authority.machines(id), \
           role TEXT NOT NULL, assigned_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           PRIMARY KEY (company, machine_id))",
        "CREATE UNIQUE INDEX IF NOT EXISTS company_machines_one_main \
           ON restless_authority.company_machines (company) WHERE role = 'main'",
        "INSERT INTO restless_authority.machines (id, label, kind) VALUES ('plane', 'This plane', 'plane') \
           ON CONFLICT (id) DO NOTHING",
    ] {
        sqlx::query(statement).execute(pool).await.context("create machine tables")?;
    }
    Ok(())
}

fn hash(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A pairing code as the owner types it: case and separators do not matter.
fn normalize_code(code: &str) -> String {
    code.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn random_bytes(count: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(count + 16);
    while bytes.len() < count {
        bytes.extend_from_slice(Uuid::new_v4().as_bytes());
    }
    bytes.truncate(count);
    bytes
}

fn new_code() -> String {
    let raw: String = random_bytes(8)
        .into_iter()
        .map(|byte| CODE_ALPHABET[usize::from(byte) % CODE_ALPHABET.len()] as char)
        .collect();
    format!("{}-{}", &raw[..4], &raw[4..])
}

fn clean_label(label: &str) -> Result<String> {
    let label = label.trim();
    if label.is_empty() {
        bail!("Give the computer a name.");
    }
    if label.chars().count() > MAX_LABEL || label.chars().any(char::is_control) {
        bail!("Use a shorter name, without line breaks.");
    }
    Ok(label.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct Assignment {
    pub company: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Machine {
    pub id: String,
    pub label: String,
    /// `plane` or `own`.
    pub kind: String,
    pub facts: MachineFacts,
    pub paused: bool,
    pub online: bool,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub companies: Vec<Assignment>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Pairing {
    pub code: String,
    pub expires_at: DateTime<Utc>,
}

/// A one-time code for adding a computer, valid for ten minutes.
pub async fn create_pairing(pool: &PgPool, label: Option<&str>) -> Result<Pairing> {
    let label = label.map(clean_label).transpose()?;
    let code = new_code();
    let expires_at = Utc::now() + ChronoDuration::minutes(PAIRING_MINUTES);
    sqlx::query(
        "INSERT INTO restless_authority.machine_pairings (code_hash, label, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(hash(&normalize_code(&code)))
    .bind(label)
    .bind(expires_at)
    .execute(pool)
    .await
    .context("save pairing code")?;
    Ok(Pairing { code, expires_at })
}

/// Whether a code could still pair a machine. The install script asks before pulling an image.
pub async fn pairing_is_open(pool: &PgPool, code: &str) -> Result<bool> {
    Ok(sqlx::query(
        "SELECT 1 FROM restless_authority.machine_pairings \
         WHERE code_hash = $1 AND consumed_at IS NULL AND expires_at > now()",
    )
    .bind(hash(&normalize_code(code)))
    .fetch_optional(pool)
    .await?
    .is_some())
}

/// Exchange a pairing code for a new machine and its credential. The code works once.
pub async fn pair(
    pool: &PgPool,
    code: &str,
    facts: MachineFacts,
    address: Option<&str>,
) -> Result<PairResponse> {
    let mut transaction = pool.begin().await?;
    let row = sqlx::query(
        "UPDATE restless_authority.machine_pairings SET consumed_at = now() \
         WHERE code_hash = $1 AND consumed_at IS NULL AND expires_at > now() RETURNING label",
    )
    .bind(hash(&normalize_code(code)))
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(row) = row else {
        bail!("That code has expired or was already used. Make a new one in Settings › Computers.");
    };
    let named: Option<String> = row.try_get("label")?;
    let fallback = if facts.hostname.trim().is_empty() {
        "My computer".to_string()
    } else {
        facts.hostname.trim().to_string()
    };
    let label =
        clean_label(named.as_deref().unwrap_or(&fallback)).unwrap_or_else(|_| "My computer".into());
    let id = format!("machine_{}", Uuid::new_v4().simple());
    let credential = URL_SAFE_NO_PAD.encode(random_bytes(32));
    sqlx::query(
        "INSERT INTO restless_authority.machines \
           (id, label, kind, facts, observed_address, credential_hash, last_seen_at) \
         VALUES ($1, $2, 'own', $3, $4, $5, now())",
    )
    .bind(&id)
    .bind(&label)
    .bind(serde_json::to_value(&facts)?)
    .bind(address)
    .bind(hash(&credential))
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE restless_authority.machine_pairings SET machine_id = $2 WHERE code_hash = $1",
    )
    .bind(hash(&normalize_code(code)))
    .bind(&id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(PairResponse {
        machine_id: id,
        label,
        credential,
    })
}

/// The machine a credential belongs to, and its epoch, unless it was removed or paused.
pub async fn authenticate(pool: &PgPool, credential: &str) -> Result<Option<(String, i64)>> {
    if credential.len() < 32 || credential.len() > 128 {
        return Ok(None);
    }
    let row = sqlx::query(
        "SELECT id, credential_epoch FROM restless_authority.machines \
         WHERE credential_hash = $1 AND removed_at IS NULL AND NOT paused",
    )
    .bind(hash(credential))
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| (row.get("id"), row.get("credential_epoch"))))
}

pub async fn heartbeat(
    pool: &PgPool,
    id: &str,
    facts: &MachineFacts,
    address: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "UPDATE restless_authority.machines SET facts = $2, observed_address = COALESCE($3, observed_address), \
           last_seen_at = now() WHERE id = $1 AND removed_at IS NULL",
    )
    .bind(id)
    .bind(serde_json::to_value(facts)?)
    .bind(address)
    .execute(pool)
    .await?;
    Ok(())
}

/// Every machine the owner holds, the plane first.
pub async fn list(pool: &PgPool) -> Result<Vec<Machine>> {
    let rows = sqlx::query(
        "SELECT id, label, kind, facts, paused, created_at, last_seen_at FROM restless_authority.machines \
         WHERE removed_at IS NULL ORDER BY (kind = 'plane') DESC, created_at",
    )
    .fetch_all(pool)
    .await?;
    let assignments = sqlx::query("SELECT company, machine_id, role FROM restless_authority.company_machines ORDER BY company")
        .fetch_all(pool)
        .await?;
    let mut by_machine: HashMap<String, Vec<Assignment>> = HashMap::new();
    for row in assignments {
        by_machine
            .entry(row.get("machine_id"))
            .or_default()
            .push(Assignment {
                company: row.get("company"),
                role: row.get("role"),
            });
    }
    Ok(rows
        .into_iter()
        .map(|row| {
            let id: String = row.get("id");
            let kind: String = row.get("kind");
            let facts = serde_json::from_value(row.get("facts")).unwrap_or_default();
            Machine {
                online: kind == "plane" || is_online(&id),
                companies: by_machine.remove(&id).unwrap_or_default(),
                label: row.get("label"),
                facts,
                paused: row.get("paused"),
                created_at: row.get("created_at"),
                last_seen_at: row.get("last_seen_at"),
                kind,
                id,
            }
        })
        .collect())
}

pub async fn get(pool: &PgPool, id: &str) -> Result<Option<Machine>> {
    Ok(list(pool)
        .await?
        .into_iter()
        .find(|machine| machine.id == id))
}

pub async fn rename(pool: &PgPool, id: &str, label: &str) -> Result<()> {
    let label = clean_label(label)?;
    let changed = sqlx::query(
        "UPDATE restless_authority.machines SET label = $2 WHERE id = $1 AND removed_at IS NULL",
    )
    .bind(id)
    .bind(label)
    .execute(pool)
    .await?
    .rows_affected();
    if changed == 0 {
        bail!("That computer no longer exists.");
    }
    Ok(())
}

/// Pause or resume a machine. Pausing cuts its link at once; nothing new runs there until resumed.
pub async fn set_paused(pool: &PgPool, id: &str, paused: bool) -> Result<()> {
    if id == PLANE_MACHINE {
        bail!("The plane itself cannot be paused.");
    }
    let changed = sqlx::query(
        "UPDATE restless_authority.machines SET paused = $2, \
           credential_epoch = credential_epoch + CASE WHEN $2 THEN 1 ELSE 0 END \
         WHERE id = $1 AND removed_at IS NULL",
    )
    .bind(id)
    .bind(paused)
    .execute(pool)
    .await?
    .rows_affected();
    if changed == 0 {
        bail!("That computer no longer exists.");
    }
    if paused {
        disconnect(id, false);
    }
    Ok(())
}

/// Remove a machine: its credential stops working at once and its link closes. A machine that is
/// still some company's main computer cannot be removed until that company moves.
pub async fn remove(pool: &PgPool, id: &str) -> Result<()> {
    if id == PLANE_MACHINE {
        bail!("The plane itself cannot be removed.");
    }
    let mains: Vec<String> = sqlx::query(
        "SELECT company FROM restless_authority.company_machines WHERE machine_id = $1 AND role = 'main'",
    )
    .bind(id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.get("company"))
    .collect();
    if !mains.is_empty() {
        bail!("Move {} back to another computer first.", mains.join(", "));
    }
    let mut transaction = pool.begin().await?;
    sqlx::query("DELETE FROM restless_authority.company_machines WHERE machine_id = $1")
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "UPDATE restless_authority.machines SET removed_at = now(), credential_hash = NULL, \
           credential_epoch = credential_epoch + 1 WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    disconnect(id, true);
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct CompanyMachine {
    pub machine_id: String,
    pub role: String,
}

/// The machines a company uses, its main computer first. A company never assigned runs on the
/// plane.
pub async fn company_assignments(pool: &PgPool, company: &str) -> Result<Vec<CompanyMachine>> {
    let rows = sqlx::query(
        "SELECT machine_id, role FROM restless_authority.company_machines WHERE company = $1 ORDER BY (role = 'main') DESC, assigned_at",
    )
    .bind(company)
    .fetch_all(pool)
    .await?;
    let mut assignments: Vec<CompanyMachine> = rows
        .into_iter()
        .map(|row| CompanyMachine {
            machine_id: row.get("machine_id"),
            role: row.get("role"),
        })
        .collect();
    if !assignments
        .iter()
        .any(|assignment| assignment.role == ROLE_MAIN)
    {
        assignments.insert(
            0,
            CompanyMachine {
                machine_id: PLANE_MACHINE.into(),
                role: ROLE_MAIN.into(),
            },
        );
    }
    Ok(assignments)
}

/// The machine holding a company's computer.
pub async fn main_machine(pool: &PgPool, company: &str) -> Result<String> {
    Ok(sqlx::query(
        "SELECT machine_id FROM restless_authority.company_machines WHERE company = $1 AND role = 'main'",
    )
    .bind(company)
    .fetch_optional(pool)
    .await?
    .map(|row| row.get("machine_id"))
    .unwrap_or_else(|| PLANE_MACHINE.to_string()))
}

/// Give a company a helper machine, or (from the move flow only) a new main machine.
pub async fn assign(pool: &PgPool, company: &str, machine: &str, role: &str) -> Result<()> {
    if !matches!(role, ROLE_MAIN | ROLE_HELPER) {
        bail!("A computer is a company's main computer or a helper.");
    }
    let exists = sqlx::query(
        "SELECT 1 FROM restless_authority.machines WHERE id = $1 AND removed_at IS NULL",
    )
    .bind(machine)
    .fetch_optional(pool)
    .await?
    .is_some();
    if !exists {
        bail!("That computer no longer exists.");
    }
    let mut transaction = pool.begin().await?;
    if role == ROLE_MAIN {
        sqlx::query(
            "DELETE FROM restless_authority.company_machines WHERE company = $1 AND role = 'main'",
        )
        .bind(company)
        .execute(&mut *transaction)
        .await?;
    }
    sqlx::query(
        "INSERT INTO restless_authority.company_machines (company, machine_id, role) VALUES ($1, $2, $3) \
         ON CONFLICT (company, machine_id) DO UPDATE SET role = EXCLUDED.role, assigned_at = now()",
    )
    .bind(company)
    .bind(machine)
    .bind(role)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn unassign(pool: &PgPool, company: &str, machine: &str) -> Result<()> {
    let removed = sqlx::query(
        "DELETE FROM restless_authority.company_machines WHERE company = $1 AND machine_id = $2 AND role = 'helper'",
    )
    .bind(company)
    .bind(machine)
    .execute(pool)
    .await?
    .rows_affected();
    if removed == 0 {
        bail!("Only a helper can be taken off a company; move its main computer instead.");
    }
    Ok(())
}

/// A company's assignments forgotten with the company.
pub async fn forget_company(pool: &PgPool, company: &str) -> Result<()> {
    sqlx::query("DELETE FROM restless_authority.company_machines WHERE company = $1")
        .bind(company)
        .execute(pool)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Links: the machines connected right now.

struct Linked {
    link: Link,
    control: mpsc::UnboundedSender<String>,
    epoch: i64,
    since: Instant,
    /// Ends the link's websocket from the plane's side, whatever the machine does.
    cut: std::sync::Arc<tokio::sync::Notify>,
}

static LINKS: LazyLock<Mutex<HashMap<String, Linked>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn is_online(id: &str) -> bool {
    LINKS
        .lock()
        .unwrap()
        .get(id)
        .is_some_and(|linked| !linked.link.is_closed())
}

fn linked(id: &str) -> Option<Link> {
    LINKS
        .lock()
        .unwrap()
        .get(id)
        .map(|linked| linked.link.clone())
        .filter(|link| !link.is_closed())
}

/// Close a machine's link, telling it first when it was removed.
fn disconnect(id: &str, removed: bool) {
    let linked = LINKS.lock().unwrap().remove(id);
    if let Some(linked) = linked {
        if removed {
            let _ = linked
                .control
                .send(serde_json::to_string(&Control::Removed).unwrap());
        }
        linked.link.close();
        // Give the Removed message a moment to leave, then close the websocket regardless.
        let cut = linked.cut;
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            cut.notify_one();
        });
    }
}

/// The plane ports a company container on a machine may reach: coordination, the model and tool
/// gateways, and native documents.
pub fn plane_ports() -> Vec<u16> {
    [
        crate::daemon::COORD_TCP_PORT,
        crate::model_gateway::RUNTIME_RELAY_PORT,
        crate::documents_service::NATIVE_DOCUMENTS_SERVICE_PORT,
    ]
    .into_iter()
    .filter_map(|port| crate::port_with_offset(port).ok())
    .collect()
}

/// The images a machine may run: the plane's company image, by its local id.
pub async fn allowed_images() -> Vec<AllowedImage> {
    let reference = crate::runtime::company_image();
    let output = tokio::process::Command::new("docker")
        .args(["image", "inspect", "--format", "{{.Id}}", &reference])
        .output()
        .await;
    match output {
        Ok(output) if output.status.success() => {
            let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
            vec![AllowedImage { reference, id }]
        }
        _ => Vec::new(),
    }
}

fn settings(machine_id: &str, allowed_images: Vec<AllowedImage>) -> String {
    serde_json::to_string(&Control::Settings {
        machine_id: machine_id.into(),
        allowed_images,
        plane_ports: plane_ports(),
    })
    .unwrap()
}

/// Where the plane's own Docker client reaches a linked machine.
pub fn docker_socket(id: &str) -> PathBuf {
    // Unix socket paths are limited to about 100 bytes; a short digest of the id keeps it inside.
    let short: String = hash(id).chars().take(16).collect();
    crate::runtime::state_root()
        .join("machines")
        .join(format!("{short}.sock"))
}

/// A TCP connection to `port` of the Restless container `container` on machine `id`.
pub async fn connect(id: &str, container: &str, port: u16) -> std::io::Result<LinkStream> {
    let link = linked(id).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotConnected, "that computer is offline")
    })?;
    link.open(STREAM_CONNECT, &format!("{container}:{port}"))
        .await
}

/// Answer the streams a machine opens: images it needs, and its company containers' calls to the
/// plane's own ports.
fn answer(incoming: restless_machine::link::Incoming, allowed: Vec<AllowedImage>) {
    tokio::spawn(async move {
        match incoming.kind.as_str() {
            STREAM_PLANE => {
                let port: Option<u16> = incoming.target.parse().ok();
                let Some(port) = port.filter(|port| plane_ports().contains(port)) else {
                    incoming.refuse("not a plane port");
                    return;
                };
                match tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
                    Ok(mut tcp) => {
                        let mut stream = incoming.accept();
                        let _ = tokio::io::copy_bidirectional(&mut stream, &mut tcp).await;
                    }
                    Err(error) => incoming.refuse(&format!("the plane is not answering: {error}")),
                }
            }
            STREAM_IMAGE => {
                if !allowed.iter().any(|image| image.id == incoming.target) {
                    incoming.refuse("not an image this machine may run");
                    return;
                }
                let child = tokio::process::Command::new("docker")
                    .args(["save", &incoming.target])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .kill_on_drop(true)
                    .spawn();
                let Ok(mut child) = child else {
                    incoming.refuse("the plane could not read the image");
                    return;
                };
                let mut stdout = child.stdout.take().unwrap();
                let mut stream = incoming.accept();
                if tokio::io::copy(&mut stdout, &mut stream).await.is_ok() {
                    let _ = tokio::io::AsyncWriteExt::shutdown(&mut stream).await;
                }
                let _ = child.wait().await;
            }
            _ => incoming.refuse("the plane does not offer that"),
        }
    });
}

/// The plane's Docker client reaches the machine through a local socket; each connection to it is
/// one `docker` stream on the link.
async fn serve_docker_socket(id: String, link: Link) {
    let path = docker_socket(&id);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::remove_file(&path);
    let listener = match tokio::net::UnixListener::bind(&path) {
        Ok(listener) => listener,
        Err(error) => {
            tracing::warn!(machine = %id, %error, "cannot open the machine's Docker socket");
            return;
        }
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    while !link.is_closed() {
        let accepted = tokio::select! {
            accepted = listener.accept() => accepted,
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => continue,
        };
        let Ok((mut local, _)) = accepted else { break };
        let link = link.clone();
        tokio::spawn(async move {
            if let Ok(mut stream) = link.open(STREAM_DOCKER, "").await {
                let _ = tokio::io::copy_bidirectional(&mut local, &mut stream).await;
            }
        });
    }
    let _ = std::fs::remove_file(&path);
}

/// One frame or message to and from a machine's websocket, independent of the web framework.
pub enum WireMessage {
    Binary(Vec<u8>),
    Text(String),
}

/// Run one machine's link until it drops: the caller pumps the websocket through `inbound` and
/// `outbound`. Only one link per machine: a newer one replaces an older.
pub async fn serve_link(
    pool: PgPool,
    machine_id: String,
    epoch: i64,
    address: Option<String>,
    mut inbound: mpsc::Receiver<WireMessage>,
    outbound: mpsc::Sender<WireMessage>,
) {
    let (link, mut frames) = Link::new(restless_machine::link::Side::Plane);
    let cut = std::sync::Arc::new(tokio::sync::Notify::new());
    let (control, mut controls) = mpsc::unbounded_channel::<String>();
    let allowed = allowed_images().await;
    let _ = control.send(settings(&machine_id, allowed.clone()));
    let previous = LINKS.lock().unwrap().insert(
        machine_id.clone(),
        Linked {
            link: link.clone(),
            control: control.clone(),
            epoch,
            since: Instant::now(),
            cut: std::sync::Arc::clone(&cut),
        },
    );
    if let Some(previous) = previous {
        previous.link.close();
    }
    tracing::info!(machine = %machine_id, "machine linked");
    let docker = tokio::spawn(serve_docker_socket(machine_id.clone(), link.clone()));
    let writer = tokio::spawn(async move {
        loop {
            let message = tokio::select! {
                frame = frames.recv() => match frame { Some(frame) => WireMessage::Binary(frame), None => break },
                text = controls.recv() => match text { Some(text) => WireMessage::Text(text), None => break },
            };
            if outbound.send(message).await.is_err() {
                break;
            }
        }
    });
    loop {
        let message = tokio::select! {
            message = inbound.recv() => match message { Some(message) => message, None => break },
            _ = cut.notified() => break,
        };
        if link.is_closed() {
            break;
        }
        match message {
            WireMessage::Binary(frame) => {
                if let Some(incoming) = link.receive(&frame) {
                    answer(incoming, allowed.clone());
                }
            }
            WireMessage::Text(text) => {
                if let Ok(Control::Heartbeat { facts }) = serde_json::from_str::<Control>(&text) {
                    if let Err(error) =
                        heartbeat(&pool, &machine_id, &facts, address.as_deref()).await
                    {
                        tracing::warn!(machine = %machine_id, %error, "machine heartbeat not saved");
                    }
                    // A removed or paused machine's credential no longer authenticates.
                    let current = sqlx::query(
                        "SELECT credential_epoch FROM restless_authority.machines \
                         WHERE id = $1 AND removed_at IS NULL AND NOT paused",
                    )
                    .bind(&machine_id)
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten()
                    .map(|row| row.get::<i64, _>("credential_epoch"));
                    if current != Some(epoch) {
                        break;
                    }
                }
            }
        }
    }
    link.close();
    {
        let mut links = LINKS.lock().unwrap();
        if links
            .get(&machine_id)
            .is_some_and(|linked| linked.epoch == epoch && linked.link.is_closed())
        {
            links.remove(&machine_id);
        }
    }
    writer.abort();
    let _ = docker.await;
    tracing::info!(machine = %machine_id, "machine link closed");
}

/// How long a linked machine has been connected, for the owner.
pub fn linked_for(id: &str) -> Option<std::time::Duration> {
    LINKS
        .lock()
        .unwrap()
        .get(id)
        .map(|linked| linked.since.elapsed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_read_easily_and_normalize() {
        let code = new_code();
        assert_eq!(code.len(), 9);
        assert!(code
            .chars()
            .all(|c| c == '-' || CODE_ALPHABET.contains(&(c as u8))));
        assert_eq!(normalize_code(" k7qf-3mxp "), "K7QF3MXP");
        assert_ne!(new_code(), new_code());
    }

    /// Every company's Computer page reads this; an unquoted role once made it a 503 everywhere.
    #[tokio::test]
    async fn a_company_reads_its_machines() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            return;
        };
        let pool = PgPool::connect(&url).await.unwrap();
        sqlx::query("CREATE SCHEMA IF NOT EXISTS restless_authority")
            .execute(&pool)
            .await
            .unwrap();
        ensure_schema(&pool).await.unwrap();
        let company = format!("machines_{}_test", uuid::Uuid::new_v4().simple());
        let assignments = company_assignments(&pool, &company).await.unwrap();
        assert_eq!(
            assignments.len(),
            1,
            "a company never assigned runs on the plane"
        );
        assert_eq!(
            (
                assignments[0].machine_id.as_str(),
                assignments[0].role.as_str()
            ),
            (PLANE_MACHINE, ROLE_MAIN)
        );
    }

    #[test]
    fn labels_are_short_and_single_line() {
        assert!(clean_label("Yail's PC").is_ok());
        assert!(clean_label("  ").is_err());
        assert!(clean_label("a\nb").is_err());
        assert!(clean_label(&"x".repeat(61)).is_err());
    }
}
