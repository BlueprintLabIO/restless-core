//! Installer and lifecycle for the per-user local Restless appliance.
//!
//! Release activation is a filesystem pointer plus OS supervision. Company
//! data never moves into the release directory, which makes rollback a binary
//! decision rather than a data-copy ritual.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use restless_contracts::appliance::{
    self as contract, MachineProfile, ServicePaths, MACOS_PLANE_LABEL, MACOS_WAKE_LABEL,
    RELEASE_ENVIRONMENT_FILE, SYSTEMD_PLANE_UNIT, SYSTEMD_WAKE_SERVICE, SYSTEMD_WAKE_TIMER,
};

/// The per-user OS supervisor that keeps the plane running and fires the
/// durable schedule wake. Everything else about the appliance is shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Supervisor {
    Launchd,
    Systemd,
}

impl Supervisor {
    fn detect() -> Result<Self> {
        if cfg!(target_os = "macos") {
            Ok(Self::Launchd)
        } else if cfg!(target_os = "linux") {
            Ok(Self::Systemd)
        } else {
            bail!("the local appliance supports macOS (launchd) and Linux (systemd --user)")
        }
    }

    fn plane_name(self) -> &'static str {
        match self {
            Self::Launchd => MACOS_PLANE_LABEL,
            Self::Systemd => SYSTEMD_PLANE_UNIT,
        }
    }

    fn wake_name(self) -> &'static str {
        match self {
            Self::Launchd => MACOS_WAKE_LABEL,
            Self::Systemd => SYSTEMD_WAKE_TIMER,
        }
    }
}
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[path = "appliance_tailscale.rs"]
pub mod tailscale;

#[path = "appliance_backup.rs"]
pub mod backup;

#[derive(Debug, Clone)]
struct Layout {
    home: PathBuf,
    install_root: PathBuf,
    releases: PathBuf,
    current: PathBuf,
    previous: PathBuf,
    bin_link: PathBuf,
    supervisor: Supervisor,
    service_dir: PathBuf,
    state_root: PathBuf,
}

impl Layout {
    fn discover() -> Result<Self> {
        let home = PathBuf::from(std::env::var("HOME").context("HOME is not set")?);
        if !home.is_absolute() {
            bail!("HOME must be absolute");
        }
        let install_root = home.join(".local/lib/restless");
        let supervisor = Supervisor::detect()?;
        Ok(Self {
            releases: install_root.join("releases"),
            current: install_root.join("current"),
            previous: install_root.join("previous"),
            bin_link: home.join(".local/bin/restless"),
            service_dir: match supervisor {
                Supervisor::Launchd => home.join("Library/LaunchAgents"),
                Supervisor::Systemd => home.join(".config/systemd/user"),
            },
            supervisor,
            state_root: home.join(".restless"),
            home,
            install_root,
        })
    }

    fn plane_definition(&self) -> PathBuf {
        match self.supervisor {
            Supervisor::Launchd => self.service_dir.join(format!("{MACOS_PLANE_LABEL}.plist")),
            Supervisor::Systemd => self.service_dir.join(SYSTEMD_PLANE_UNIT),
        }
    }

    fn wake_definition(&self) -> PathBuf {
        match self.supervisor {
            Supervisor::Launchd => self.service_dir.join(format!("{MACOS_WAKE_LABEL}.plist")),
            Supervisor::Systemd => self.service_dir.join(SYSTEMD_WAKE_TIMER),
        }
    }

    /// Every OS service definition for `release`, validated before any is written.
    fn definitions(&self, release: &Path) -> Result<Vec<(PathBuf, String)>> {
        let paths = self.release_paths(release);
        let definitions = match self.supervisor {
            Supervisor::Launchd => vec![
                (
                    self.plane_definition(),
                    contract::launchd_plane_plist(&paths)?,
                ),
                (
                    self.wake_definition(),
                    contract::launchd_wake_plist(&paths)?,
                ),
            ],
            Supervisor::Systemd => vec![
                (
                    self.plane_definition(),
                    contract::systemd_plane_unit(&paths)?,
                ),
                (
                    self.service_dir.join(SYSTEMD_WAKE_SERVICE),
                    contract::systemd_wake_service(&paths)?,
                ),
                (
                    self.wake_definition(),
                    contract::systemd_wake_timer().to_string(),
                ),
            ],
        };
        for (_, text) in &definitions {
            contract::validate_service_definition(text)?;
        }
        Ok(definitions)
    }

    fn service_loaded(&self, name: &str) -> bool {
        match self.supervisor {
            Supervisor::Launchd => service_loaded(name),
            Supervisor::Systemd => systemctl(&["is-active", "--quiet", name]).is_ok(),
        }
    }

    fn stop_services(&self) {
        match self.supervisor {
            Supervisor::Launchd => {
                bootout(MACOS_WAKE_LABEL);
                bootout(MACOS_PLANE_LABEL);
            }
            Supervisor::Systemd => {
                let _ = systemctl(&["stop", SYSTEMD_WAKE_TIMER, SYSTEMD_PLANE_UNIT]);
            }
        }
    }

    fn start_services(&self) -> Result<()> {
        match self.supervisor {
            Supervisor::Launchd => {
                bootstrap(&self.plane_definition())?;
                bootstrap(&self.wake_definition())?;
            }
            Supervisor::Systemd => {
                systemctl(&["daemon-reload"])?;
                systemctl(&["enable", SYSTEMD_PLANE_UNIT, SYSTEMD_WAKE_TIMER])?;
                systemctl(&["start", SYSTEMD_PLANE_UNIT, SYSTEMD_WAKE_TIMER])?;
            }
        }
        Ok(())
    }

    fn remove_definitions(&self) -> Result<()> {
        match self.supervisor {
            Supervisor::Launchd => {
                for (path, label) in [
                    (self.wake_definition(), MACOS_WAKE_LABEL),
                    (self.plane_definition(), MACOS_PLANE_LABEL),
                ] {
                    remove_if_owned_definition(&path, &format!("<string>{label}</string>"))?;
                }
            }
            Supervisor::Systemd => {
                let _ = systemctl(&["disable", SYSTEMD_PLANE_UNIT, SYSTEMD_WAKE_TIMER]);
                for unit in [SYSTEMD_WAKE_TIMER, SYSTEMD_WAKE_SERVICE, SYSTEMD_PLANE_UNIT] {
                    remove_if_owned_definition(
                        &self.service_dir.join(unit),
                        "Description=Restless",
                    )?;
                }
                systemctl(&["daemon-reload"])?;
            }
        }
        Ok(())
    }

    fn release_paths(&self, release: &Path) -> ServicePaths {
        ServicePaths {
            restlessd: release.join("bin/restlessd"),
            restless: release.join("bin/restless"),
            cockpit_dir: release.join("web"),
            state_root: self.state_root.clone(),
        }
    }
}

#[derive(Debug)]
struct Candidate {
    cli: PathBuf,
    daemon: PathBuf,
    cockpit: PathBuf,
    /// Validated non-secret `release.env` text: artifacts and host tools.
    release_environment: Option<String>,
}

impl Candidate {
    fn discover(
        daemon: Option<PathBuf>,
        cockpit: Option<PathBuf>,
        release_environment: Option<PathBuf>,
    ) -> Result<Self> {
        let cli = std::env::current_exe().context("locate the running restless CLI")?;
        let daemon = daemon.unwrap_or_else(|| {
            cli.parent()
                .unwrap_or_else(|| Path::new("."))
                .join("restlessd")
        });
        let cockpit = cockpit
            .or_else(|| std::env::var_os("RESTLESS_COCKPIT_DIR").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("web/build"));
        let release_environment = release_environment
            .map(|path| {
                let text = std::fs::read_to_string(&path)
                    .with_context(|| format!("read release environment {}", path.display()))?;
                contract::parse_release_environment(&text)?;
                Ok::<_, anyhow::Error>(text)
            })
            .transpose()?;
        let candidate = Self {
            cli: absolute(&cli)?,
            daemon: absolute(&daemon)?,
            cockpit: absolute(&cockpit)?,
            release_environment,
        };
        candidate.validate()?;
        Ok(candidate)
    }

    fn validate(&self) -> Result<()> {
        for (name, path) in [("restless", &self.cli), ("restlessd", &self.daemon)] {
            let metadata = std::fs::metadata(path)
                .with_context(|| format!("inspect {name} candidate {}", path.display()))?;
            if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
                bail!("{name} candidate is not executable: {}", path.display());
            }
        }
        if !self.cockpit.join("index.html").is_file() {
            bail!(
                "Cockpit candidate has no index.html: {}",
                self.cockpit.display()
            );
        }
        Ok(())
    }

    fn release_id(&self) -> Result<String> {
        let mut digest = Sha256::new();
        for path in [&self.cli, &self.daemon] {
            digest.update(path.to_string_lossy().as_bytes());
            hash_file(path, &mut digest)?;
        }
        hash_tree(&self.cockpit, &self.cockpit, &mut digest)?;
        if let Some(environment) = &self.release_environment {
            digest.update(environment.as_bytes());
        }
        Ok(format!("{:x}", digest.finalize())[..20].to_string())
    }
}

#[derive(Debug, Serialize)]
pub struct InstallReport {
    pub release: String,
    pub previous_release: Option<String>,
    pub state_root: String,
    pub plane_service: String,
    pub wake_service: String,
    pub ready: bool,
    pub rolled_back: bool,
}

#[derive(Debug, Serialize)]
pub struct StatusReport {
    pub profile: &'static str,
    pub supervisor: &'static str,
    pub state_root: String,
    /// The source commit the active release was built from, when recorded.
    pub source_commit: Option<String>,
    pub installed_release: Option<String>,
    pub previous_release: Option<String>,
    pub plane_definition: bool,
    pub wake_definition: bool,
    pub plane_loaded: bool,
    pub wake_loaded: bool,
    pub lock_pid: Option<u32>,
    pub lock_pid_alive: bool,
    pub lock_held: bool,
    pub active_binary: bool,
    pub socket_present: bool,
    pub owner_health: Option<u16>,
    pub draining: bool,
    pub state: &'static str,
    pub repair: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RecoveryState {
    state: String,
    failed_release: String,
    detail: String,
    recorded_at_unix_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct ControlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct ControlResponse {
    ok: bool,
    data: Option<serde_json::Value>,
    error: Option<ControlError>,
}

#[derive(Debug, Deserialize)]
struct DrainStatus {
    idle: bool,
    active_requests: usize,
    exec_wakes: Vec<String>,
    staff: Vec<String>,
    native_clients: Vec<String>,
}

struct DrainGuard {
    state_root: PathBuf,
    armed: bool,
}

impl DrainGuard {
    fn new(state_root: &Path) -> Self {
        Self {
            state_root: state_root.to_path_buf(),
            armed: true,
        }
    }

    fn resume(mut self) -> Result<()> {
        self.armed = false;
        resume_appliance(&self.state_root)
    }

    fn clear_without_daemon(mut self) -> Result<()> {
        self.armed = false;
        contract::clear_drain_marker(&self.state_root)
    }
}

impl Drop for DrainGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = resume_appliance(&self.state_root);
        }
    }
}

pub fn install(
    daemon: Option<PathBuf>,
    cockpit: Option<PathBuf>,
    release_environment: Option<PathBuf>,
    environment: Option<PathBuf>,
    upgrading: bool,
    force: bool,
) -> Result<InstallReport> {
    ensure_stable_profile()?;
    let layout = Layout::discover()?;
    let candidate = Candidate::discover(daemon, cockpit, release_environment)?;
    let release = candidate.release_id()?;
    let release_dir = stage_candidate(&layout, &candidate, &release)?;
    let previous_target = read_link_name(&layout.current);

    std::fs::create_dir_all(layout.state_root.join("logs"))?;
    std::fs::create_dir_all(&layout.service_dir)?;
    std::fs::create_dir_all(layout.bin_link.parent().expect("bin link has parent"))?;

    if let Some(source) = environment {
        import_environment(&layout, &source)?;
    }

    // Validate every generated definition before activation. A candidate that
    // leaks secret-shaped material never reaches the OS supervisor.
    let definitions = layout.definitions(&release_dir)?;

    // Run the exact staged daemon's read-only preflight before changing the
    // activation pointer. A bad candidate cannot take the active service down.
    if let Err(error) = run_candidate_preflight(&release_dir, &layout.state_root) {
        write_recovery_state(&layout, "upgrade_blocked", &release, &format!("{error:#}"))?;
        return Err(error);
    }

    // Close work admission only after every read-only candidate check passes.
    // The persistent marker makes the closed gate survive the daemon handoff;
    // a failed activation resumes whichever known-good process is reachable.
    let drain = begin_appliance_drain(&layout, force)?;

    if let Some(ref current) = previous_target {
        atomic_symlink(current, &layout.previous)?;
    }
    atomic_symlink(&release_dir, &layout.current)?;
    atomic_symlink(&layout.current.join("bin/restless"), &layout.bin_link)?;
    for (path, text) in &definitions {
        atomic_write(path, text.as_bytes(), 0o644)?;
    }

    restart_services(&layout)?;
    let ready = wait_ready(&layout, Duration::from_secs(30));
    if ready {
        clear_recovery_state(&layout)?;
        prune_releases(&layout)?;
        let report = InstallReport {
            release,
            previous_release: previous_target
                .and_then(|path| path.file_name().map(|v| v.to_string_lossy().into_owned())),
            state_root: layout.state_root.display().to_string(),
            plane_service: layout.supervisor.plane_name().into(),
            wake_service: layout.supervisor.wake_name().into(),
            ready: true,
            rolled_back: false,
        };
        drain.resume()?;
        return Ok(report);
    }

    if upgrading {
        if let Some(previous) = previous_target {
            atomic_symlink(&previous, &layout.current)?;
            write_service_definitions(&layout, &previous)?;
            restart_services(&layout)?;
            if !wait_ready(&layout, Duration::from_secs(30)) {
                write_recovery_state(
                    &layout,
                    "crash_loop",
                    &release,
                    "candidate and last-known-good release both failed readiness",
                )?;
                let _ = drain.resume();
                bail!("new release failed readiness and the previous release did not recover; inspect ~/.restless/logs/restlessd.log");
            }
            write_recovery_state(
                &layout,
                "upgrade_blocked",
                &release,
                "candidate failed readiness; last-known-good release was restored",
            )?;
            let report = InstallReport {
                release,
                previous_release: previous
                    .file_name()
                    .map(|v| v.to_string_lossy().into_owned()),
                state_root: layout.state_root.display().to_string(),
                plane_service: layout.supervisor.plane_name().into(),
                wake_service: layout.supervisor.wake_name().into(),
                ready: false,
                rolled_back: true,
            };
            drain.resume()?;
            return Ok(report);
        }
    }
    write_recovery_state(
        &layout,
        "crash_loop",
        &release,
        "installed service did not become ready and no last-known-good release was available",
    )?;
    let _ = drain.resume();
    bail!("installed service did not become ready within 30 seconds; inspect ~/.restless/logs/restlessd.log")
}

fn begin_appliance_drain(layout: &Layout, force: bool) -> Result<DrainGuard> {
    let guard = DrainGuard::new(&layout.state_root);
    let started = Instant::now();
    let mut last = match request_appliance_control(&layout.state_root, "appliance-drain") {
        Ok(status) => Some(status),
        Err(error) => {
            let live = stable_singleton_candidates(layout)?;
            if !live.is_empty() && !force {
                bail!(
                    "the running stable appliance cannot prove a drain-safe handoff: {error:#}; wait for it to become reachable or retry with --force only after verifying no useful work is active"
                );
            }
            contract::write_drain_marker(&layout.state_root)?;
            if !live.is_empty() {
                eprintln!(
                    "warning: forcing replacement of a stable daemon that could not report active work"
                );
            }
            None
        }
    };

    while let Some(status) = last {
        if status.idle {
            return Ok(guard);
        }
        if started.elapsed() >= Duration::from_secs(30) {
            if force {
                eprintln!(
                    "warning: forcing replacement with active work: {}",
                    render_drain_activity(&status)
                );
                return Ok(guard);
            }
            bail!(
                "stable appliance still has active work after 30 seconds: {}; no process was interrupted (retry later, or use --force only to accept interruption)",
                render_drain_activity(&status)
            );
        }
        std::thread::sleep(Duration::from_millis(100));
        last = match request_appliance_control(&layout.state_root, "appliance-drain") {
            Ok(status) => Some(status),
            Err(error) if force => {
                eprintln!("warning: lost drain status before forced replacement: {error:#}");
                return Ok(guard);
            }
            Err(error) => return Err(error).context("observe stable appliance drain"),
        };
    }
    Ok(guard)
}

fn request_appliance_control(state_root: &Path, command: &str) -> Result<DrainStatus> {
    let socket = state_root.join("restlessd.sock");
    let mut stream = UnixStream::connect(&socket)
        .with_context(|| format!("connect stable appliance socket {}", socket.display()))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    writeln!(stream, "{}", serde_json::json!({ "cmd": command }))?;
    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response)?;
    let response: ControlResponse = serde_json::from_str(response.trim())
        .with_context(|| format!("decode {command} response"))?;
    if !response.ok {
        bail!(
            "{}",
            response
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| format!("{command} was refused"))
        );
    }
    serde_json::from_value(
        response
            .data
            .context("appliance control returned no status")?,
    )
    .with_context(|| format!("decode {command} status"))
}

fn render_drain_activity(status: &DrainStatus) -> String {
    format!(
        "{} request(s), exec={:?}, staff={:?}, native={:?}",
        status.active_requests, status.exec_wakes, status.staff, status.native_clients
    )
}

fn resume_appliance(state_root: &Path) -> Result<()> {
    let started = Instant::now();
    loop {
        match request_appliance_control(state_root, "appliance-resume") {
            Ok(_) => return Ok(()),
            Err(error) => {
                let lock = state_root.join("machine/plane.lock");
                let supervisor = Supervisor::detect()?;
                let daemon_expected = contract::singleton_lock_is_held(&lock).unwrap_or(false)
                    || match supervisor {
                        Supervisor::Launchd => service_loaded(MACOS_PLANE_LABEL),
                        Supervisor::Systemd => {
                            systemctl(&["is-active", "--quiet", SYSTEMD_PLANE_UNIT]).is_ok()
                        }
                    };
                if !daemon_expected {
                    contract::clear_drain_marker(state_root)?;
                    return Ok(());
                }
                if started.elapsed() >= Duration::from_secs(5) {
                    return Err(error).context(
                        "the appliance remains safely drained; run `restless appliance resume` after repairing its service",
                    );
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn import_environment(layout: &Layout, source: &Path) -> Result<()> {
    let source = absolute(source)?;
    let supplied = dotenvy::from_path_iter(&source)
        .with_context(|| format!("read environment source {}", source.display()))?
        .collect::<std::result::Result<BTreeMap<_, _>, _>>()
        .with_context(|| format!("parse environment source {}", source.display()))?;
    let (selected, uses_infisical) = required_environment(&layout.state_root)?;
    let path = layout
        .state_root
        .join(contract::PROFILE_ENVIRONMENT_RELATIVE);
    // Credential refresh must not silently return a shared plane to local-owner entry.
    let mut retained = existing_environment(&path)?;
    retained.retain(|name, _| SHARING_ENVIRONMENT.contains(&name.as_str()));
    for name in &selected {
        if let Some(value) = supplied.get(name) {
            retained.insert(name.clone(), value.clone());
        }
    }
    if uses_infisical && !layout.state_root.join("infisical/authority.env").is_file() {
        for name in [
            "INFISICAL_API_URL",
            "INFISICAL_ENVIRONMENT",
            "INFISICAL_ORGANIZATION_SLUG",
            "INFISICAL_PROJECT_ID",
            "INFISICAL_UNIVERSAL_AUTH_CLIENT_ID",
            "INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET",
        ] {
            if let Some(value) = supplied.get(name) {
                retained.insert(name.into(), value.clone());
            }
        }
    }
    atomic_write(&path, &serde_json::to_vec_pretty(&retained)?, 0o600)?;
    Ok(())
}

const SHARING_ENVIRONMENT: &[&str] = &[
    "RESTLESS_ENTRY_MODE",
    "RESTLESS_RUNTIME_MODE",
    "RESTLESS_ENTRY_ISSUER",
    "RESTLESS_ENTRY_JWKS_URL",
    "RESTLESS_ENTRY_OWNER_ID",
    "RESTLESS_ENTRY_PLANE_ID",
    "RESTLESS_ENTRY_HOST",
    "RESTLESS_COMPANY_IMAGE",
    "RESTLESS_ENTRY_ALLOW_INSECURE_HTTP",
    "RESTLESS_ENTRY_JWKS_CA_FILE",
];

fn existing_environment(path: &Path) -> Result<BTreeMap<String, String>> {
    if path.is_file() {
        Ok(serde_json::from_slice(&std::fs::read(path)?)?)
    } else {
        Ok(BTreeMap::new())
    }
}

fn sharing_environment(source: &Path) -> Result<BTreeMap<String, String>> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(source)?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.mode() & 0o077 != 0
        || metadata.len() > 32768
    {
        bail!("sharing configuration must be one private regular file (mode 0600)");
    }
    let values =
        dotenvy::from_path_iter(source)?.collect::<std::result::Result<BTreeMap<_, _>, _>>()?;
    if values
        .keys()
        .any(|name| !SHARING_ENVIRONMENT.contains(&name.as_str()))
    {
        bail!("sharing configuration contains an unsupported setting");
    }
    if values.get("RESTLESS_ENTRY_MODE").map(String::as_str) != Some("network")
        || values.get("RESTLESS_RUNTIME_MODE").map(String::as_str) != Some("local")
    {
        bail!("sharing requires authenticated network entry and local company computers");
    }
    for name in ["RESTLESS_ENTRY_OWNER_ID", "RESTLESS_ENTRY_PLANE_ID"] {
        let value = values
            .get(name)
            .context("sharing configuration is incomplete")?;
        let uuid: uuid::Uuid = value.parse()?;
        if uuid.is_nil() {
            bail!("sharing identity must not be nil");
        }
    }
    let issuer = url::Url::parse(
        values
            .get("RESTLESS_ENTRY_ISSUER")
            .context("account address is missing")?,
    )?;
    let local = issuer.host_str().is_some_and(|host| {
        host == "localhost" || host.ends_with(".localhost") || host == "127.0.0.1"
    });
    if issuer.origin().ascii_serialization() != issuer.as_str().trim_end_matches('/')
        || !issuer.username().is_empty()
        || issuer.password().is_some()
        || !(issuer.scheme() == "https"
            || (local
                && issuer.scheme() == "http"
                && values
                    .get("RESTLESS_ENTRY_ALLOW_INSECURE_HTTP")
                    .map(String::as_str)
                    == Some("1")))
    {
        bail!("account entry requires HTTPS; insecure entry is only for loopback qualification");
    }
    if values.get("RESTLESS_ENTRY_JWKS_URL")
        != Some(&format!(
            "{}/.well-known/jwks.json",
            issuer.origin().ascii_serialization()
        ))
    {
        bail!("signing keys must come from the configured account issuer");
    }
    let host = values
        .get("RESTLESS_ENTRY_HOST")
        .context("company hostname is missing")?;
    if !host.contains('.')
        || host
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-'))
    {
        bail!("company entry needs its exact DNS hostname");
    }
    let image = values
        .get("RESTLESS_COMPANY_IMAGE")
        .context("company image is missing")?;
    if !image.split_once("@sha256:").is_some_and(|(name, digest)| {
        !name.is_empty()
            && digest.len() == 64
            && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        bail!("the company image must be pinned by OCI digest");
    }
    Ok(values)
}

/// The installed appliance owns this last step. It preserves existing credentials,
/// checks the candidate, drains useful work and restores entry settings on failure.
pub fn enable_sharing(environment: PathBuf) -> Result<serde_json::Value> {
    ensure_stable_profile()?;
    let layout = Layout::discover()?;
    let release = read_link_name(&layout.current)
        .context("install the local appliance before enabling sharing")?;
    let environment = absolute(&environment)?;
    let supplied = sharing_environment(&environment)?;
    let issuer = supplied.get("RESTLESS_ENTRY_ISSUER").unwrap();
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none());
    if let Some(path) = supplied.get("RESTLESS_ENTRY_JWKS_CA_FILE") {
        builder =
            builder.add_root_certificate(reqwest::Certificate::from_pem(&std::fs::read(path)?)?);
    }
    let client = builder.build()?;
    let metadata = sharing_response(
        client
            .get(format!("{issuer}/.well-known/restless-issuer"))
            .send()?,
        16384,
    )?;
    if metadata["contract_version"] != 1 || metadata["issuer"] != issuer.as_str() {
        bail!("the account service does not match the configured issuer contract");
    }
    let keys = sharing_response(
        client
            .get(supplied.get("RESTLESS_ENTRY_JWKS_URL").unwrap())
            .send()?,
        16384,
    )?;
    if !keys["keys"].as_array().is_some_and(|keys| {
        !keys.is_empty()
            && keys.len() <= 16
            && keys
                .iter()
                .all(|key| key["kty"] == "OKP" && key["crv"] == "Ed25519" && key["d"].is_null())
    }) {
        bail!("the account service did not provide supported public signing keys");
    }
    // Read the same private configuration the account service uses. Before a
    // plane-wide switch, prove every current company is still represented.
    let configuration = environment.parent().unwrap().join("identity.json");
    use std::os::unix::fs::MetadataExt;
    let private = std::fs::symlink_metadata(&configuration)?;
    if !private.is_file()
        || private.nlink() != 1
        || private.mode() & 0o077 != 0
        || private.len() > 32768
    {
        bail!("account configuration must be one private regular file");
    }
    let configuration: serde_json::Value = serde_json::from_slice(&std::fs::read(configuration)?)?;
    if configuration["origin"] != issuer.as_str()
        || configuration["ownerId"].as_str()
            != supplied.get("RESTLESS_ENTRY_OWNER_ID").map(String::as_str)
        || configuration["planeId"].as_str()
            != supplied.get("RESTLESS_ENTRY_PLANE_ID").map(String::as_str)
    {
        bail!("account configuration and prepared entry do not match");
    }
    let companies = configuration["companies"]
        .as_array()
        .context("prepare a current sharing setup that includes every company")?;
    let selected = companies
        .iter()
        .find(|company| company["companyId"] == configuration["companyId"])
        .context("default company is missing from sharing setup")?;
    let handle = selected["companyHandle"]
        .as_str()
        .context("company handle is missing")?;
    if !handle
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        bail!("invalid company handle in sharing configuration");
    }
    let current = sharing_response(client.post(format!("http://127.0.0.1:7788/api/companies/{handle}/sharing/setup"))
        .header("Origin", "http://127.0.0.1:7788")
        .json(&serde_json::json!({ "account_origin": issuer, "core_origin": configuration["coreOrigin"], "owner_email": configuration["ownerEmail"], "access": "private" }))
        .send()?, 65536)?;
    let current_companies = current["companies"]
        .as_array()
        .context("local company coordinates could not be checked")?;
    if current_companies.len() != companies.len()
        || !current_companies.iter().all(|current| {
            companies.iter().any(|company| {
                company["companyHandle"] == current["id"]
                    && company["companyId"] == current["company_id"]
                    && company["cellId"] == current["cell_id"]
            })
        })
    {
        bail!("the companies on this host changed; prepare sharing setup again from Members");
    }
    if !keys["keys"].as_array().unwrap().iter().any(|key| {
        key["kid"] == configuration["signingKey"]["kid"]
            && key["x"] == configuration["signingKey"]["x"]
    }) {
        bail!("the running account service uses a different signing key");
    }
    let path = layout
        .state_root
        .join(contract::PROFILE_ENVIRONMENT_RELATIVE);
    let previous = std::fs::read(&path).ok();
    let mut merged = existing_environment(&path)?;
    if merged.get("RESTLESS_ENTRY_MODE").map(String::as_str) == Some("network") {
        bail!("individual sign-in is already configured; use Members to manage access");
    }
    merged.extend(supplied);
    atomic_write(&path, &serde_json::to_vec_pretty(&merged)?, 0o600)?;
    let restore = || -> Result<()> {
        match &previous {
            Some(bytes) => atomic_write(&path, bytes, 0o600),
            None => {
                std::fs::remove_file(&path)?;
                Ok(())
            }
        }
    };
    if let Err(error) =
        run_candidate_preflight_with_environment(&release, &layout.state_root, &merged)
    {
        restore()?;
        return Err(error);
    }
    let drain = match begin_appliance_drain(&layout, false) {
        Ok(drain) => drain,
        Err(error) => {
            restore()?;
            return Err(error);
        }
    };
    let activation = restart_services(&layout).and_then(|_| {
        if wait_ready(&layout, Duration::from_secs(30)) {
            Ok(())
        } else {
            bail!("authenticated company entry did not become ready");
        }
    });
    if let Err(error) = activation {
        restore()?;
        restart_services(&layout).context("restore the previous entry service")?;
        if !wait_ready(&layout, Duration::from_secs(30)) {
            bail!("sharing activation failed and the previous entry did not recover; company data is preserved: {error:#}");
        }
        drain.resume()?;
        return Err(error)
            .context("sharing was not enabled; previous entry settings were restored");
    }
    drain.resume()?;
    Ok(
        serde_json::json!({ "ready": true, "mode": "network", "account_url": merged.get("RESTLESS_ENTRY_ISSUER") }),
    )
}

fn sharing_response(
    response: reqwest::blocking::Response,
    limit: u64,
) -> Result<serde_json::Value> {
    let mut bytes = Vec::new();
    response
        .error_for_status()?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("sharing verification response exceeded its size limit");
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn required_environment(state_root: &Path) -> Result<(BTreeSet<String>, bool)> {
    let mut required = BTreeSet::new();
    let mut uses_infisical = false;
    let companies = state_root.join("companies");
    if !companies.is_dir() {
        return Ok((required, false));
    }
    for entry in std::fs::read_dir(&companies)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let document: toml::Value = toml::from_str(&std::fs::read_to_string(&path)?)
            .with_context(|| format!("parse {}", path.display()))?;
        collect_environment_references(&document, &mut required, &mut uses_infisical);
        let mut models = Vec::new();
        if let Some(model) = document.get("model").and_then(toml::Value::as_str) {
            models.push(model);
        }
        if let Some(failover) = document
            .get("model_failover")
            .and_then(toml::Value::as_array)
        {
            models.extend(failover.iter().filter_map(toml::Value::as_str));
        }
        if models.iter().any(|model| model.starts_with("litellm/")) {
            required.insert("GPT_BASE_URL".into());
        }
    }
    Ok((required, uses_infisical))
}

fn collect_environment_references(
    value: &toml::Value,
    required: &mut BTreeSet<String>,
    uses_infisical: &mut bool,
) {
    match value {
        toml::Value::String(value) => {
            if let Some(name) = value.strip_prefix("env:") {
                if !name.is_empty()
                    && name.bytes().enumerate().all(|(index, byte)| {
                        byte == b'_'
                            || byte.is_ascii_uppercase()
                            || (index > 0 && byte.is_ascii_digit())
                    })
                {
                    required.insert(name.into());
                }
            } else if value.starts_with("infisical:") {
                *uses_infisical = true;
            }
        }
        toml::Value::Array(values) => {
            for value in values {
                collect_environment_references(value, required, uses_infisical);
            }
        }
        toml::Value::Table(values) => {
            for value in values.values() {
                collect_environment_references(value, required, uses_infisical);
            }
        }
        _ => {}
    }
}

pub fn rollback(force: bool) -> Result<InstallReport> {
    ensure_stable_profile()?;
    let layout = Layout::discover()?;
    let previous = std::fs::read_link(&layout.previous)
        .context("no previous Restless release is available")?;
    let current = read_link_name(&layout.current);
    layout.definitions(&previous)?;
    let drain = begin_appliance_drain(&layout, force)?;
    atomic_symlink(&previous, &layout.current)?;
    if let Some(ref current) = current {
        // Rollback is a two-way release swap. Keeping `previous` pointed at
        // the release we just activated makes status lie and prevents an
        // immediate switch-forward if the owner changes course.
        atomic_symlink(current, &layout.previous)?;
    }
    write_service_definitions(&layout, &previous)?;
    restart_services(&layout)?;
    if !wait_ready(&layout, Duration::from_secs(30)) {
        if let Some(current) = current {
            atomic_symlink(&current, &layout.current)?;
            atomic_symlink(&previous, &layout.previous)?;
            write_service_definitions(&layout, &current)?;
            restart_services(&layout)?;
        }
        let _ = drain.resume();
        bail!("previous release failed readiness; restored the prior activation pointer")
    }
    clear_recovery_state(&layout)?;
    let report = InstallReport {
        release: previous
            .file_name()
            .map(|v| v.to_string_lossy().into_owned())
            .unwrap_or_default(),
        previous_release: current
            .and_then(|v| v.file_name().map(|n| n.to_string_lossy().into_owned())),
        state_root: layout.state_root.display().to_string(),
        plane_service: layout.supervisor.plane_name().into(),
        wake_service: layout.supervisor.wake_name().into(),
        ready: true,
        rolled_back: true,
    };
    drain.resume()?;
    Ok(report)
}

fn write_service_definitions(layout: &Layout, release: &Path) -> Result<()> {
    for (path, text) in layout.definitions(release)? {
        atomic_write(&path, text.as_bytes(), 0o644)?;
    }
    Ok(())
}

/// Keep only the active and previous releases. Older ones are unreachable by
/// rollback and would otherwise accumulate one full build per upgrade.
fn prune_releases(layout: &Layout) -> Result<()> {
    let keep = [&layout.current, &layout.previous]
        .into_iter()
        .filter_map(|link| std::fs::canonicalize(link).ok())
        .collect::<Vec<_>>();
    let Ok(entries) = std::fs::read_dir(&layout.releases) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(resolved) = std::fs::canonicalize(&path) else {
            continue;
        };
        if path.is_dir() && !keep.contains(&resolved) {
            ensure_exact_child(&layout.releases, &path, None)?;
            std::fs::remove_dir_all(&path)
                .with_context(|| format!("remove superseded release {}", path.display()))?;
        }
    }
    Ok(())
}

pub fn status() -> Result<StatusReport> {
    ensure_stable_profile()?;
    let layout = Layout::discover()?;
    let plane_loaded = layout.service_loaded(layout.supervisor.plane_name());
    let wake_loaded = layout.service_loaded(layout.supervisor.wake_name());
    let lock_pid = read_lock_pid(&layout.state_root.join("machine/plane.lock"));
    let lock_pid_alive = lock_pid.is_some_and(process_alive);
    let lock_held = contract::singleton_lock_is_held(&layout.state_root.join("machine/plane.lock"))
        .unwrap_or(false);
    let active_binary = lock_pid.is_some_and(|pid| process_is_active_release(&layout, pid));
    let owner_health = owner_health_status();
    let owner_state = owner_appliance_state();
    let plane_definition = layout.plane_definition().is_file();
    let wake_definition = layout.wake_definition().is_file();
    let draining = contract::drain_marker_exists(&layout.state_root);
    let recovery = read_recovery_state(&layout);
    let (state, repair) = if let Some(recovery) = recovery {
        let state = if recovery.state == "crash_loop" {
            "crash_loop"
        } else {
            "upgrade_blocked"
        };
        (
            state,
            Some(format!(
                "{}; inspect the service log, then retry or run `restless appliance rollback`{}",
                recovery.detail,
                if draining {
                    "; the work gate is still closed, so run `restless appliance resume` after recovery"
                } else {
                    ""
                }
            )),
        )
    } else if draining {
        (
            "draining",
            Some(
                "finish or abandon the lifecycle operation, then run `restless appliance resume`"
                    .into(),
            ),
        )
    } else if owner_state.as_deref() == Some("recovering") {
        (
            "recovering",
            Some(
                "Runtime safety recovery is retrying; owner reads remain available and work admission opens automatically"
                    .into(),
            ),
        )
    } else if owner_health == Some(200)
        && lock_pid_alive
        && lock_held
        && active_binary
        && plane_loaded
        && wake_loaded
    {
        ("ready", None)
    } else if !plane_definition && !wake_definition && !plane_loaded && !wake_loaded {
        (
            "uninstalled",
            Some("run `restless appliance install`".into()),
        )
    } else if !plane_loaded || !wake_loaded {
        ("degraded", Some("run `restless appliance start`".into()))
    } else if !lock_pid_alive {
        (
            "booting_or_crash_loop",
            Some("inspect ~/.restless/logs/restlessd.log".into()),
        )
    } else {
        (
            "degraded",
            Some("run `restless doctor -c <company>` and inspect the service log".into()),
        )
    };
    Ok(StatusReport {
        profile: "stable",
        supervisor: layout.supervisor.plane_name(),
        state_root: layout.state_root.display().to_string(),
        source_commit: release_setting(&layout.current, "RESTLESS_SOURCE_COMMIT"),
        installed_release: read_link_name(&layout.current).and_then(file_name),
        previous_release: read_link_name(&layout.previous).and_then(file_name),
        plane_definition,
        wake_definition,
        plane_loaded,
        wake_loaded,
        lock_pid,
        lock_pid_alive,
        lock_held,
        active_binary,
        socket_present: layout.state_root.join("restlessd.sock").exists(),
        owner_health,
        draining,
        state,
        repair,
    })
}

pub fn start(force: bool) -> Result<StatusReport> {
    ensure_stable_profile()?;
    let layout = Layout::discover()?;
    if !layout.plane_definition().is_file() || !layout.wake_definition().is_file() {
        bail!("Restless is not installed; run `restless appliance install`");
    }
    let current = status()?;
    if current.state == "ready" {
        return Ok(current);
    }
    let drain = begin_appliance_drain(&layout, force)?;
    restart_services(&layout)?;
    if !wait_ready(&layout, Duration::from_secs(30)) {
        let _ = drain.resume();
        bail!("stable appliance did not become ready within 30 seconds; inspect ~/.restless/logs/restlessd.log");
    }
    drain.resume()?;
    status()
}

pub fn stop(force: bool) -> Result<StatusReport> {
    ensure_stable_profile()?;
    let layout = Layout::discover()?;
    let drain = begin_appliance_drain(&layout, force)?;
    layout.stop_services();
    release_previous_singleton(&layout)?;
    drain.clear_without_daemon()?;
    status()
}

pub fn uninstall(force: bool) -> Result<StatusReport> {
    ensure_stable_profile()?;
    let layout = Layout::discover()?;
    let drain = begin_appliance_drain(&layout, force)?;
    layout.stop_services();
    release_previous_singleton(&layout)?;
    drain.clear_without_daemon()?;
    layout.remove_definitions()?;
    remove_if_owned_symlink(&layout.bin_link, &layout.install_root)?;
    if layout.install_root.is_dir() {
        ensure_exact_child(
            &layout.home.join(".local/lib"),
            &layout.install_root,
            Some("restless"),
        )?;
        std::fs::remove_dir_all(&layout.install_root)?;
    }
    for owned in [
        layout.state_root.join("restlessd.sock"),
        layout.state_root.join("machine"),
        layout.state_root.join("launch-cache"),
    ] {
        if owned.is_dir() {
            std::fs::remove_dir_all(&owned)?;
        } else if owned.exists() {
            std::fs::remove_file(&owned)?;
        }
    }
    // Company configs, cells, Authority and OrgIntel are intentionally retained.
    status()
}

pub fn resume() -> Result<StatusReport> {
    ensure_stable_profile()?;
    let layout = Layout::discover()?;
    resume_appliance(&layout.state_root)?;
    status()
}

pub fn open_owner() -> Result<()> {
    let profile = MachineProfile::from_env()?;
    let port = 7_788u16
        .checked_add(profile.port_offset)
        .context("owner port exceeds TCP range")?;
    let url = format!("http://127.0.0.1:{port}");
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = Command::new(program)
        .arg(&url)
        .status()
        .with_context(|| format!("open {url}"))?;
    if !status.success() {
        bail!("could not open {url}");
    }
    Ok(())
}

fn stage_candidate(layout: &Layout, candidate: &Candidate, release: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(&layout.releases)?;
    let destination = layout.releases.join(release);
    if destination.is_dir() {
        return Ok(destination);
    }
    let staging = layout
        .releases
        .join(format!(".{release}.staging-{}", std::process::id()));
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(staging.join("bin"))?;
    copy_executable(&candidate.cli, &staging.join("bin/restless"))?;
    copy_executable(&candidate.daemon, &staging.join("bin/restlessd"))?;
    copy_tree(&candidate.cockpit, &staging.join("web"))?;
    if let Some(environment) = &candidate.release_environment {
        atomic_write(
            &staging.join(RELEASE_ENVIRONMENT_FILE),
            environment.as_bytes(),
            0o644,
        )?;
    }
    if !staging.join("web/index.html").is_file() {
        bail!("staged Cockpit is incomplete");
    }
    std::fs::rename(&staging, &destination)?;
    Ok(destination)
}

fn run_candidate_preflight(release: &Path, state_root: &Path) -> Result<()> {
    run_candidate_preflight_with_environment(release, state_root, &BTreeMap::new())
}

fn run_candidate_preflight_with_environment(
    release: &Path,
    state_root: &Path,
    environment: &BTreeMap<String, String>,
) -> Result<()> {
    let daemon = release.join("bin/restlessd");
    let cockpit = release.join("web");
    let output = Command::new(&daemon)
        .arg("appliance-preflight")
        .env("RESTLESS_PROFILE", "stable")
        .env("RESTLESS_HOME", state_root)
        .env("RESTLESS_PORT_OFFSET", "0")
        .env("RESTLESS_COCKPIT_DIR", &cockpit)
        .envs(environment)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("run staged appliance preflight {}", daemon.display()))?;
    if !output.status.success() {
        bail!(
            "staged release failed read-only preflight; active release was not changed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn restart_services(layout: &Layout) -> Result<()> {
    layout.stop_services();
    release_previous_singleton(layout)?;
    layout.start_services()
}

fn systemctl(args: &[&str]) -> Result<()> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("run systemctl --user {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "systemctl --user {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// One non-secret setting recorded in a release's `release.env`.
fn release_setting(release: &Path, name: &str) -> Option<String> {
    let text = std::fs::read_to_string(release.join(RELEASE_ENVIRONMENT_FILE)).ok()?;
    contract::parse_release_environment(&text)
        .ok()?
        .into_iter()
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn release_previous_singleton(layout: &Layout) -> Result<()> {
    let mut candidates = stable_singleton_candidates(layout)?;
    candidates.retain(|pid| process_alive(*pid));
    if candidates.is_empty() {
        return Ok(());
    }
    if candidates.len() != 1 {
        bail!(
            "stable profile has multiple possible owners {:?}; refusing to signal any process",
            candidates
        );
    }
    let pid = *candidates.iter().next().expect("one candidate");
    let command = process_command(pid)
        .with_context(|| format!("identify previous stable listener pid {pid}"))?;
    if command.file_name().and_then(|name| name.to_str()) != Some("restlessd") {
        bail!("stable port or lock is owned by unrecognised pid {pid}; refusing to signal it");
    }
    terminate_and_wait(pid)
}

fn stable_singleton_candidates(layout: &Layout) -> Result<BTreeSet<u32>> {
    let lock = layout.state_root.join("machine/plane.lock");
    let mut candidates = BTreeSet::new();
    if contract::singleton_lock_is_held(&lock)? {
        let pid = read_lock_pid(&lock).context("live singleton lock has no diagnostic pid")?;
        candidates.insert(pid);
    }
    candidates.extend(stable_listener_pids()?);
    candidates.retain(|pid| process_alive(*pid));
    Ok(candidates)
}

fn stable_listener_pids() -> Result<BTreeSet<u32>> {
    let output = match Command::new("lsof")
        .args(["-nP", "-t", "-iTCP:7788", "-sTCP:LISTEN"])
        .output()
    {
        Ok(output) => output,
        // A fresh Ubuntu 24.04 has no lsof (observed on a clean install, 7 October); ss is in
        // iproute2 on every Linux this runs on.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ss_listener_pids();
        }
        Err(error) => return Err(error).context("inspect the stable owner port"),
    };
    if !output.status.success() && output.stdout.is_empty() {
        return Ok(BTreeSet::new());
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<u32>()
                .with_context(|| format!("invalid listener pid {line:?}"))
        })
        .collect()
}

/// The listeners on the stable port from ss, as `users:(("restlessd",pid=123,fd=9))`. Without ss
/// either, the plane's singleton lock (checked beside this) still finds a running plane.
fn ss_listener_pids() -> Result<BTreeSet<u32>> {
    let output = match Command::new("ss").args(["-Hltnp", "sport = :7788"]).output() {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(error) => return Err(error).context("inspect the stable owner port"),
    };
    Ok(String::from_utf8_lossy(&output.stdout)
        .split("pid=")
        .skip(1)
        .filter_map(|rest| {
            rest.split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|digits| digits.parse().ok())
        })
        .collect())
}

fn terminate_and_wait(pid: u32) -> Result<()> {
    if !process_alive(pid) {
        return Ok(());
    }
    let status = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .context("stop previous stable singleton")?;
    if !status.success() && process_alive(pid) {
        bail!("could not stop previous stable singleton pid {pid}");
    }
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        if !process_alive(pid) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let status = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .status()
        .context("force-stop unresponsive previous stable singleton")?;
    if !status.success() {
        bail!("previous stable singleton pid {pid} ignored TERM and could not be force-stopped");
    }
    let forced = Instant::now();
    while forced.elapsed() < Duration::from_secs(5) {
        if !process_alive(pid) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    bail!("previous stable singleton pid {pid} remained alive after SIGKILL")
}

fn process_command(pid: u32) -> Result<PathBuf> {
    // Linux `ps -o comm=` truncates to the bare name; the kernel knows the path.
    if cfg!(target_os = "linux") {
        return std::fs::read_link(format!("/proc/{pid}/exe"))
            .with_context(|| format!("process {pid} is not alive or not ours"));
    }
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .context("inspect process executable")?;
    if !output.status.success() {
        bail!("process {pid} is not alive");
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        bail!("process {pid} has no executable path");
    }
    Ok(PathBuf::from(value))
}

fn process_is_active_release(layout: &Layout, pid: u32) -> bool {
    let Ok(actual) = process_command(pid).and_then(|path| {
        std::fs::canonicalize(&path).with_context(|| format!("resolve {}", path.display()))
    }) else {
        return false;
    };
    let Ok(expected) = std::fs::canonicalize(layout.current.join("bin/restlessd")) else {
        return false;
    };
    actual == expected
}

fn recovery_path(layout: &Layout) -> PathBuf {
    layout.state_root.join("machine/appliance-recovery.json")
}

fn write_recovery_state(
    layout: &Layout,
    state: &str,
    failed_release: &str,
    detail: &str,
) -> Result<()> {
    let recovery = RecoveryState {
        state: state.to_string(),
        failed_release: failed_release.to_string(),
        detail: detail.to_string(),
        recorded_at_unix_seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    atomic_write(
        &recovery_path(layout),
        &serde_json::to_vec_pretty(&recovery)?,
        0o600,
    )
}

fn read_recovery_state(layout: &Layout) -> Option<RecoveryState> {
    serde_json::from_slice(&std::fs::read(recovery_path(layout)).ok()?).ok()
}

fn clear_recovery_state(layout: &Layout) -> Result<()> {
    let path = recovery_path(layout);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn bootstrap(path: &Path) -> Result<()> {
    let output = Command::new("launchctl")
        .args(["bootstrap", &format!("gui/{}", unsafe { libc_getuid() })])
        .arg(path)
        .output()
        .with_context(|| format!("bootstrap {}", path.display()))?;
    if !output.status.success() {
        bail!(
            "launchctl bootstrap {} failed: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn bootout(label: &str) {
    let _ = Command::new("launchctl")
        .args([
            "bootout",
            &format!("gui/{}/{}", unsafe { libc_getuid() }, label),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

// getuid is stable on every Unix target this crate supports. Keeping this tiny
// declaration local avoids pulling an OS-management crate into the owner CLI.
extern "C" {
    fn getuid() -> u32;
}

unsafe fn libc_getuid() -> u32 {
    getuid()
}

fn service_loaded(label: &str) -> bool {
    if !cfg!(target_os = "macos") {
        return false;
    }
    Command::new("launchctl")
        .args([
            "print",
            &format!("gui/{}/{}", unsafe { libc_getuid() }, label),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn owner_health_status() -> Option<u16> {
    let layout = Layout::discover().ok()?;
    let values = existing_environment(
        &layout
            .state_root
            .join(contract::PROFILE_ENVIRONMENT_RELATIVE),
    )
    .ok()?;
    let host = values
        .get("RESTLESS_ENTRY_HOST")
        .filter(|_| values.get("RESTLESS_ENTRY_MODE").map(String::as_str) == Some("network"))
        .map(String::as_str)
        .unwrap_or("127.0.0.1:7788");
    if host
        .bytes()
        .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b':')))
    {
        return None;
    }
    let mut stream = std::net::TcpStream::connect_timeout(
        &"127.0.0.1:7788".parse().expect("literal socket"),
        Duration::from_secs(1),
    )
    .ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(1))).ok()?;
    stream
        .write_all(
            format!("GET /health HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    response
        .lines()
        .next()?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

fn owner_appliance_state() -> Option<String> {
    let mut stream = std::net::TcpStream::connect_timeout(
        &"127.0.0.1:7788".parse().expect("literal socket"),
        Duration::from_secs(1),
    )
    .ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(1))).ok()?;
    stream
        .write_all(
            b"GET /api/appliance HTTP/1.1\r\nHost: 127.0.0.1:7788\r\nConnection: close\r\n\r\n",
        )
        .ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    let body = response.split_once("\r\n\r\n")?.1;
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get("state")?
        .as_str()
        .map(str::to_string)
}

fn wait_ready(layout: &Layout, bound: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < bound {
        let lock = layout.state_root.join("machine/plane.lock");
        let pid = read_lock_pid(&lock);
        if owner_health_status() == Some(200)
            && contract::singleton_lock_is_held(&lock).unwrap_or(false)
            && pid.is_some_and(|pid| process_alive(pid) && process_is_active_release(layout, pid))
            // The owner HTTP surface intentionally becomes available before
            // every company cell has reconciled. Lifecycle activation is not
            // complete until the local control socket is accepting too;
            // otherwise install can declare readiness and then fail to lift
            // its own durable drain barrier during a slow boot.
            && UnixStream::connect(layout.state_root.join("restlessd.sock")).is_ok()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    false
}

fn process_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn read_lock_pid(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_link_name(path: &Path) -> Option<PathBuf> {
    std::fs::read_link(path).ok()
}

fn file_name(path: PathBuf) -> Option<String> {
    path.file_name()
        .map(|value| value.to_string_lossy().into_owned())
}

fn atomic_symlink(target: &Path, link: &Path) -> Result<()> {
    let parent = link.parent().context("activation link has no parent")?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{}.next-{}",
        link.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    if temporary.exists() || temporary.is_symlink() {
        std::fs::remove_file(&temporary)?;
    }
    symlink(target, &temporary)?;
    std::fs::rename(&temporary, link)?;
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    let parent = path.parent().context("file has no parent")?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{}.next-{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(mode))?;
    std::fs::rename(&temporary, path)?;
    Ok(())
}

fn copy_executable(source: &Path, destination: &Path) -> Result<()> {
    std::fs::copy(source, destination)?;
    let mut permissions = std::fs::metadata(destination)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(destination, permissions)?;
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            bail!(
                "release payload may not contain symbolic links: {}",
                source_path.display()
            );
        } else if file_type.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&source_path, &destination_path)?;
        } else {
            bail!(
                "release payload contains a non-regular file: {}",
                source_path.display()
            );
        }
    }
    Ok(())
}

fn hash_file(path: &Path, digest: &mut Sha256) -> Result<()> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(())
}

fn hash_tree(root: &Path, directory: &Path, digest: &mut Sha256) -> Result<()> {
    let mut entries = std::fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            bail!(
                "release payload may not contain symbolic links: {}",
                path.display()
            );
        }
        let relative = path
            .strip_prefix(root)
            .context("release tree escaped its root")?;
        digest.update(relative.to_string_lossy().as_bytes());
        digest.update([0]);
        if file_type.is_dir() {
            digest.update(b"directory\0");
            hash_tree(root, &path, digest)?;
        } else if file_type.is_file() {
            digest.update(b"file\0");
            hash_file(&path, digest)?;
        } else {
            bail!(
                "release payload contains a non-regular file: {}",
                path.display()
            );
        }
    }
    Ok(())
}

fn remove_if_owned_definition(path: &Path, marker: &str) -> Result<()> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Ok(());
    };
    if !text.contains(marker) {
        bail!(
            "refusing to remove modified service definition {}",
            path.display()
        );
    }
    std::fs::remove_file(path)?;
    Ok(())
}

fn remove_if_owned_symlink(path: &Path, root: &Path) -> Result<()> {
    let Ok(target) = std::fs::read_link(path) else {
        return Ok(());
    };
    let resolved = if target.is_absolute() {
        target
    } else {
        path.parent().unwrap_or_else(|| Path::new("/")).join(target)
    };
    if !resolved.starts_with(root) {
        bail!("refusing to remove non-Restless symlink {}", path.display());
    }
    std::fs::remove_file(path)?;
    Ok(())
}

fn ensure_exact_child(parent: &Path, child: &Path, name: Option<&str>) -> Result<()> {
    let file_name = child.file_name().and_then(|v| v.to_str());
    if child.parent() != Some(parent)
        || file_name.is_none_or(|value| value.is_empty() || value.starts_with('.'))
        || name.is_some_and(|name| file_name != Some(name))
    {
        bail!("refusing broad cleanup target {}", child.display());
    }
    Ok(())
}

fn absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn ensure_stable_profile() -> Result<()> {
    let profile = MachineProfile::from_env()?;
    if profile.kind != contract::ProfileKind::Stable {
        bail!(
            "appliance lifecycle commands require RESTLESS_PROFILE=stable; {} resources are isolated and must be managed by their own development/test runner",
            profile.kind.as_str()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharing_import_refuses_public_files_untrusted_keys_and_mutable_images() {
        let root =
            std::env::temp_dir().join(format!("restless-sharing-{}_test", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("entry.env");
        let valid = format!(
            "RESTLESS_ENTRY_MODE=network\nRESTLESS_RUNTIME_MODE=local\nRESTLESS_ENTRY_ISSUER=https://accounts.example.com\nRESTLESS_ENTRY_JWKS_URL=https://accounts.example.com/.well-known/jwks.json\nRESTLESS_ENTRY_OWNER_ID={}\nRESTLESS_ENTRY_PLANE_ID={}\nRESTLESS_ENTRY_HOST=work.example.com\nRESTLESS_COMPANY_IMAGE=registry.example.com/company@sha256:{}\n",
            uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), "a".repeat(64)
        );
        atomic_write(&path, valid.as_bytes(), 0o600).unwrap();
        assert!(sharing_environment(&path).is_ok());
        let link = root.join("link.env");
        symlink(&path, &link).unwrap();
        assert!(sharing_environment(&link).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(sharing_environment(&path).is_err());
        for invalid in [
            valid.replace(
                "https://accounts.example.com/.well-known/jwks.json",
                "https://other.example.com/.well-known/jwks.json",
            ),
            valid.replace(&format!("@sha256:{}", "a".repeat(64)), ":latest"),
            format!("{valid}GPT_API_KEY=must-not-import\n"),
            valid.replace("RESTLESS_ENTRY_MODE=network", "RESTLESS_ENTRY_MODE=local"),
        ] {
            atomic_write(&path, invalid.as_bytes(), 0o600).unwrap();
            assert!(sharing_environment(&path).is_err());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exact_cleanup_guard_rejects_a_parent_or_sibling() {
        let exact = Some("restless");
        assert!(
            ensure_exact_child(Path::new("/tmp/lib"), Path::new("/tmp/lib/restless"), exact)
                .is_ok()
        );
        assert!(ensure_exact_child(Path::new("/tmp/lib"), Path::new("/tmp/lib"), exact).is_err());
        assert!(
            ensure_exact_child(Path::new("/tmp/lib"), Path::new("/tmp/lib/other"), exact).is_err()
        );
    }

    #[test]
    fn candidate_release_identity_changes_with_bytes() {
        let root =
            std::env::temp_dir().join(format!("restless-release-test-{}", std::process::id()));
        std::fs::create_dir_all(root.join("web")).unwrap();
        for name in ["restless", "restlessd"] {
            std::fs::write(root.join(name), name).unwrap();
            std::fs::set_permissions(root.join(name), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        std::fs::write(root.join("web/index.html"), "one").unwrap();
        let candidate = Candidate {
            cli: root.join("restless"),
            daemon: root.join("restlessd"),
            cockpit: root.join("web"),
            release_environment: None,
        };
        let one = candidate.release_id().unwrap();
        std::fs::write(root.join("web/index.html"), "two").unwrap();
        let two = candidate.release_id().unwrap();
        assert_ne!(one, two);
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn environment_import_scope_comes_only_from_company_contracts() {
        let root = std::env::temp_dir().join(format!(
            "restless-environment-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        std::fs::write(
            root.join("companies/alpha_test.toml"),
            r#"
name = "alpha_test"
model = "litellm/gpt-5.6"
model_failover = ["zai/glm-5.3"]
[credentials]
"model.inference.litellm" = "env:GPT_API_KEY"
"model.inference.zai" = "infisical:/providers/zai/ZAI_API_KEY"
"#,
        )
        .unwrap();
        let (required, uses_infisical) = required_environment(&root).unwrap();
        assert_eq!(
            required,
            BTreeSet::from(["GPT_API_KEY".into(), "GPT_BASE_URL".into()])
        );
        assert!(uses_infisical);
        assert!(!required.contains("COOLIFY_API_KEY"));
        let profile = root.join(contract::PROFILE_ENVIRONMENT_RELATIVE);
        atomic_write(&profile, br#"{"RESTLESS_ENTRY_MODE":"network","RESTLESS_COMPANY_IMAGE":"company@sha256:pinned","GPT_API_KEY":"previous","UNUSED_SECRET":"remove"}"#, 0o600).unwrap();
        let credentials = root.join("credentials.env");
        atomic_write(
            &credentials,
            b"GPT_API_KEY=current\nRESTLESS_ENTRY_MODE=local\nUNUSED_SECRET=unrequested\n",
            0o600,
        )
        .unwrap();
        let layout = Layout {
            home: root.clone(),
            install_root: root.clone(),
            releases: root.clone(),
            current: root.clone(),
            previous: root.clone(),
            bin_link: root.clone(),
            supervisor: Supervisor::Systemd,
            service_dir: root.clone(),
            state_root: root.clone(),
        };
        import_environment(&layout, &credentials).unwrap();
        let imported = existing_environment(&profile).unwrap();
        assert_eq!(imported.get("RESTLESS_ENTRY_MODE").unwrap(), "network");
        assert_eq!(
            imported.get("RESTLESS_COMPANY_IMAGE").unwrap(),
            "company@sha256:pinned"
        );
        assert_eq!(imported.get("GPT_API_KEY").unwrap(), "current");
        assert!(!imported.contains_key("UNUSED_SECRET"));
        std::fs::remove_dir_all(root).ok();
    }
}
