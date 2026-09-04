//! Host-level contract for one dependable local Restless appliance.
//!
//! This module deliberately contains no company work and no schedule payloads.
//! The operating system may supervise the account plane and deliver a bounded
//! `wake-due` hint; durable company and schedule truth remains in Restless.

use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

pub const PROFILE_ENVIRONMENT_RELATIVE: &str = "credentials/environment.json";
pub const APPLIANCE_DRAIN_RELATIVE: &str = "machine/appliance-draining.json";

pub const MACOS_PLANE_LABEL: &str = "io.restless.plane";
pub const MACOS_WAKE_LABEL: &str = "io.restless.wake-due";
pub const SYSTEMD_PLANE_UNIT: &str = "restless-plane.service";
pub const SYSTEMD_WAKE_SERVICE: &str = "restless-wake-due.service";
pub const SYSTEMD_WAKE_TIMER: &str = "restless-wake-due.timer";

/// Admission barrier used while replacing the stable daemon. A request either
/// enters before the drain and is counted, or observes the closed gate; the
/// second admission check closes the race between those two operations.
#[derive(Clone)]
pub struct LifecycleGate {
    draining: Arc<AtomicBool>,
    recovering: Arc<AtomicBool>,
    active: Arc<AtomicUsize>,
}

impl LifecycleGate {
    pub fn new(draining: bool) -> Self {
        Self {
            draining: Arc::new(AtomicBool::new(draining)),
            recovering: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn try_enter(&self) -> Option<LifecycleLease> {
        if self.is_blocked() {
            return None;
        }
        self.active.fetch_add(1, Ordering::SeqCst);
        if self.is_blocked() {
            self.active.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(LifecycleLease { gate: self.clone() })
    }

    pub fn begin_drain(&self) {
        self.draining.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.draining.store(false, Ordering::SeqCst);
    }

    pub fn is_draining(&self) -> bool {
        self.draining.load(Ordering::SeqCst)
    }

    /// Close work admission while crash recovery is incomplete without
    /// pretending the owner initiated an appliance replacement. Read-only
    /// owner/control surfaces remain available to explain the degraded state.
    pub fn begin_recovery(&self) {
        self.recovering.store(true, Ordering::SeqCst);
    }

    pub fn finish_recovery(&self) {
        self.recovering.store(false, Ordering::SeqCst);
    }

    pub fn is_recovering(&self) -> bool {
        self.recovering.load(Ordering::SeqCst)
    }

    pub fn active(&self) -> usize {
        self.active.load(Ordering::SeqCst)
    }

    fn is_blocked(&self) -> bool {
        self.is_draining() || self.is_recovering()
    }
}

impl Default for LifecycleGate {
    fn default() -> Self {
        Self::new(false)
    }
}

pub struct LifecycleLease {
    gate: LifecycleGate,
}

impl Drop for LifecycleLease {
    fn drop(&mut self) {
        self.gate.active.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Serialize)]
struct DrainMarker {
    pid: u32,
    recorded_at_unix_seconds: u64,
}

pub fn drain_marker_exists(state_root: &Path) -> bool {
    state_root.join(APPLIANCE_DRAIN_RELATIVE).is_file()
}

pub fn write_drain_marker(state_root: &Path) -> Result<()> {
    let path = state_root.join(APPLIANCE_DRAIN_RELATIVE);
    let parent = path.parent().expect("drain marker has a parent");
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".appliance-draining-{}.json", std::process::id()));
    if temporary.exists() {
        std::fs::remove_file(&temporary)?;
    }
    let marker = DrainMarker {
        pid: std::process::id(),
        recorded_at_unix_seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    };
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(&serde_json::to_vec_pretty(&marker)?)?;
    file.sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&temporary, &path)?;
    Ok(())
}

pub fn clear_drain_marker(state_root: &Path) -> Result<()> {
    let path = state_root.join(APPLIANCE_DRAIN_RELATIVE);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove {}", path.display())),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKind {
    Stable,
    Dev,
    Test,
}

impl ProfileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Dev => "dev",
            Self::Test => "test",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineProfile {
    pub kind: ProfileKind,
    pub state_root: PathBuf,
    pub port_offset: u16,
    /// Empty for the backwards-compatible stable Docker names. Explicit for
    /// every development and test plane.
    pub resource_namespace: String,
}

impl MachineProfile {
    pub fn from_env() -> Result<Self> {
        let kind = match std::env::var("RESTLESS_PROFILE").ok().as_deref() {
            None | Some("") | Some("stable") => ProfileKind::Stable,
            Some("dev") => ProfileKind::Dev,
            Some("test") => ProfileKind::Test,
            Some(other) => bail!("unknown RESTLESS_PROFILE {other:?}; expected stable|dev|test"),
        };
        let home = std::env::var("HOME").context("HOME is not set")?;
        let default_root = absolute_clean_path(&PathBuf::from(home).join(".restless"))?;
        let state_root = match std::env::var_os("RESTLESS_HOME") {
            Some(value) => absolute_clean_path(&PathBuf::from(value))?,
            None if kind == ProfileKind::Stable => default_root.clone(),
            None => bail!(
                "RESTLESS_PROFILE={} requires an explicit RESTLESS_HOME",
                kind.as_str()
            ),
        };
        let port_offset = std::env::var("RESTLESS_PORT_OFFSET")
            .unwrap_or_else(|_| "0".into())
            .parse::<u16>()
            .context("RESTLESS_PORT_OFFSET must be an integer")?;
        let resource_namespace = std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default();
        let profile = Self {
            kind,
            state_root,
            port_offset,
            resource_namespace,
        };
        profile.validate()?;
        if profile.kind == ProfileKind::Stable && profile.state_root != default_root {
            bail!(
                "the stable profile requires the canonical per-user state root {}",
                default_root.display()
            );
        }
        Ok(profile)
    }

    pub fn stable(home: &Path) -> Result<Self> {
        let profile = Self {
            kind: ProfileKind::Stable,
            state_root: absolute_clean_path(&home.join(".restless"))?,
            port_offset: 0,
            resource_namespace: String::new(),
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<()> {
        if !self.state_root.is_absolute() {
            bail!("RESTLESS_HOME must be absolute");
        }
        if self
            .state_root
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        {
            bail!("RESTLESS_HOME must not contain '..'");
        }
        match self.kind {
            ProfileKind::Stable => {
                if self.port_offset != 0 {
                    bail!("the stable profile requires RESTLESS_PORT_OFFSET=0");
                }
                if !self.resource_namespace.is_empty() {
                    bail!("the stable profile cannot set RESTLESS_RESOURCE_NAMESPACE");
                }
            }
            ProfileKind::Dev => {
                if !(1_000..=19_999).contains(&self.port_offset) {
                    bail!("the dev profile requires RESTLESS_PORT_OFFSET in 1000..19999");
                }
                validate_namespace(&self.resource_namespace)?;
            }
            ProfileKind::Test => {
                if !(20_000..=50_000).contains(&self.port_offset) {
                    bail!("the test profile requires RESTLESS_PORT_OFFSET in 20000..50000");
                }
                validate_namespace(&self.resource_namespace)?;
                if !self.resource_namespace.ends_with("_test") {
                    bail!("the test resource namespace must end in _test");
                }
            }
        }
        Ok(())
    }

    pub fn socket_path(&self) -> PathBuf {
        self.state_root.join("restlessd.sock")
    }

    pub fn lock_path(&self) -> PathBuf {
        self.state_root.join("machine").join("plane.lock")
    }

    pub fn log_dir(&self) -> PathBuf {
        self.state_root.join("logs")
    }

    pub fn launch_cache_dir(&self) -> PathBuf {
        self.state_root.join("launch-cache")
    }

    pub fn docker_container_name(&self, company: &str) -> String {
        if self.resource_namespace.is_empty() {
            format!("restless-co-{company}")
        } else {
            format!("restless-{}-co-{company}", self.resource_namespace)
        }
    }

    pub fn docker_volume_name(&self, company: &str) -> String {
        if self.resource_namespace.is_empty() {
            format!("restless-vol-{company}")
        } else {
            format!("restless-{}-vol-{company}", self.resource_namespace)
        }
    }

    pub fn docker_image_name(&self) -> String {
        if self.resource_namespace.is_empty() {
            "restless-company-image:latest".into()
        } else {
            format!("restless-company-image:{}", self.resource_namespace)
        }
    }
}

/// Load profile-owned bootstrap credentials without putting any secret in an
/// OS service definition. Explicitly inherited variables always win. The JSON
/// file is written only by the filtered appliance importer; the legacy
/// Authority dotenv remains a supported local Infisical machine identity.
pub fn load_profile_environment(profile: &MachineProfile) -> Result<()> {
    let json_path = profile.state_root.join(PROFILE_ENVIRONMENT_RELATIVE);
    if json_path.is_file() {
        require_private_file(&json_path)?;
        let values: std::collections::BTreeMap<String, String> =
            serde_json::from_slice(&std::fs::read(&json_path)?)
                .with_context(|| format!("parse {}", json_path.display()))?;
        for (name, value) in values {
            if name.is_empty()
                || !name.bytes().enumerate().all(|(index, byte)| {
                    byte == b'_'
                        || byte.is_ascii_uppercase()
                        || (index > 0 && byte.is_ascii_digit())
                })
            {
                bail!("invalid environment name in {}", json_path.display());
            }
            if std::env::var_os(&name).is_none() {
                std::env::set_var(name, value);
            }
        }
    }
    let authority = profile.state_root.join("infisical/authority.env");
    if authority.is_file() {
        require_private_file(&authority)?;
        dotenvy::from_path(&authority).with_context(|| format!("load {}", authority.display()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn require_private_file(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path)?.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        bail!(
            "credential file {} must not be group/world accessible",
            path.display()
        );
    }
    Ok(())
}

#[cfg(not(unix))]
fn require_private_file(_path: &Path) -> Result<()> {
    Ok(())
}

fn validate_namespace(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 24
        || !value.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                byte.is_ascii_lowercase()
            } else {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
            }
        })
    {
        bail!(
            "RESTLESS_RESOURCE_NAMESPACE must be 1..24 lowercase letters, digits, '_' or '-', starting with a letter"
        );
    }
    Ok(())
}

fn absolute_clean_path(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !clean.pop() {
                    bail!("path escapes its filesystem root");
                }
            }
            other => clean.push(other.as_os_str()),
        }
    }
    Ok(clean)
}

/// An advisory lock held for the full daemon lifetime. Unlike the Unix socket,
/// another process cannot unlink this lock out from under a live daemon.
pub struct SingletonGuard {
    file: File,
    path: PathBuf,
}

impl SingletonGuard {
    pub fn acquire(profile: &MachineProfile) -> Result<Self> {
        let path = profile.lock_path();
        let parent = path.parent().expect("lock path has a parent");
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create singleton directory {}", parent.display()))?;
        let mut file = OpenOptions::new()
            .create(true)
            // Never truncate before obtaining the advisory lock: a refused
            // second singleton must not erase the live owner's PID evidence.
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .with_context(|| format!("open singleton lock {}", path.display()))?;
        file.try_lock_exclusive().with_context(|| {
            format!(
                "another Restless {} plane already owns {}",
                profile.kind.as_str(),
                profile.state_root.display()
            )
        })?;
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        writeln!(file, "{}", std::process::id())?;
        file.sync_data()?;
        Ok(Self { file, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SingletonGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// Distinguish a live advisory owner from a stale lock file. The PID written
/// in the file is diagnostic only and must never be used as kill authority by
/// itself.
pub fn singleton_lock_is_held(path: &Path) -> Result<bool> {
    let file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error).with_context(|| format!("open {}", path.display())),
    };
    match file.try_lock_exclusive() {
        Ok(()) => {
            file.unlock()?;
            Ok(false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(true),
        Err(error) => Err(error).with_context(|| format!("probe {}", path.display())),
    }
}

#[derive(Debug, Clone)]
pub struct ServicePaths {
    pub restlessd: PathBuf,
    pub restless: PathBuf,
    pub cockpit_dir: PathBuf,
    pub state_root: PathBuf,
}

impl ServicePaths {
    pub fn validate(&self) -> Result<()> {
        for (label, path) in [
            ("restlessd", &self.restlessd),
            ("restless", &self.restless),
            ("cockpit", &self.cockpit_dir),
            ("state root", &self.state_root),
        ] {
            if !path.is_absolute() {
                bail!("{label} path must be absolute: {}", path.display());
            }
        }
        Ok(())
    }
}

pub fn launchd_plane_plist(paths: &ServicePaths) -> Result<String> {
    paths.validate()?;
    let log = paths.state_root.join("logs/restlessd.log");
    let home = paths
        .state_root
        .parent()
        .context("stable state root has no home directory")?;
    let service_path = format!(
        "{}/.local/bin:{}/.bun/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
        home.display(),
        home.display()
    );
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>{}</string>
  <key>ProgramArguments</key><array><string>{}</string></array>
  <key>EnvironmentVariables</key><dict>
    <key>RESTLESS_PROFILE</key><string>stable</string>
    <key>RESTLESS_HOME</key><string>{}</string>
    <key>RESTLESS_PORT_OFFSET</key><string>0</string>
    <key>RESTLESS_COCKPIT_DIR</key><string>{}</string>
    <key>PATH</key><string>{}</string>
  </dict>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ThrottleInterval</key><integer>30</integer>
  <key>ProcessType</key><string>Interactive</string>
  <key>StandardOutPath</key><string>{}</string>
  <key>StandardErrorPath</key><string>{}</string>
</dict></plist>
"#,
        MACOS_PLANE_LABEL,
        xml(&paths.restlessd.to_string_lossy()),
        xml(&paths.state_root.to_string_lossy()),
        xml(&paths.cockpit_dir.to_string_lossy()),
        xml(&service_path),
        xml(&log.to_string_lossy()),
        xml(&log.to_string_lossy()),
    ))
}

pub fn launchd_wake_plist(paths: &ServicePaths) -> Result<String> {
    paths.validate()?;
    let log = paths.state_root.join("logs/wake-due.log");
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>{}</string>
  <key>ProgramArguments</key><array><string>{}</string><string>appliance</string><string>wake-due</string><string>--adapter</string><string>launchd</string></array>
  <key>EnvironmentVariables</key><dict>
    <key>RESTLESS_PROFILE</key><string>stable</string>
    <key>RESTLESS_HOME</key><string>{}</string>
    <key>RESTLESS_PORT_OFFSET</key><string>0</string>
  </dict>
  <key>StartInterval</key><integer>60</integer>
  <key>ProcessType</key><string>Background</string>
  <key>StandardOutPath</key><string>{}</string>
  <key>StandardErrorPath</key><string>{}</string>
</dict></plist>
"#,
        MACOS_WAKE_LABEL,
        xml(&paths.restless.to_string_lossy()),
        xml(&paths.state_root.to_string_lossy()),
        xml(&log.to_string_lossy()),
        xml(&log.to_string_lossy()),
    ))
}

pub fn systemd_plane_unit(paths: &ServicePaths) -> Result<String> {
    paths.validate()?;
    Ok(format!(
        "[Unit]\nDescription=Restless local account plane\nAfter=network.target\n\n[Service]\nType=simple\nExecStart={}\nEnvironment=RESTLESS_PROFILE=stable\nEnvironment=RESTLESS_HOME={}\nEnvironment=RESTLESS_PORT_OFFSET=0\nEnvironment=RESTLESS_COCKPIT_DIR={}\nRestart=always\nRestartSec=30\n\n[Install]\nWantedBy=default.target\n",
        systemd_arg(&paths.restlessd)?,
        systemd_arg(&paths.state_root)?,
        systemd_arg(&paths.cockpit_dir)?,
    ))
}

pub fn systemd_wake_service(paths: &ServicePaths) -> Result<String> {
    paths.validate()?;
    Ok(format!(
        "[Unit]\nDescription=Ask Restless to reconcile due schedules\n\n[Service]\nType=oneshot\nExecStart={} appliance wake-due --adapter systemd\nEnvironment=RESTLESS_PROFILE=stable\nEnvironment=RESTLESS_HOME={}\nEnvironment=RESTLESS_PORT_OFFSET=0\n",
        systemd_arg(&paths.restless)?,
        systemd_arg(&paths.state_root)?,
    ))
}

pub fn systemd_wake_timer() -> &'static str {
    "[Unit]\nDescription=Restless durable schedule wake\n\n[Timer]\nOnBootSec=30s\nOnUnitActiveSec=60s\nPersistent=true\nAccuracySec=10s\nUnit=restless-wake-due.service\n\n[Install]\nWantedBy=timers.target\n"
}

/// Reject service definitions that accidentally turn the OS scheduler into a
/// prompt/credential store. Paths and released command names are allowed.
pub fn validate_service_definition(text: &str) -> Result<()> {
    let lowered = text.to_ascii_lowercase();
    for forbidden in [
        "api_key",
        "apikey",
        "authorization:",
        "bearer ",
        "invitation",
        "prompt=",
        "task_payload",
        "zai_base_url",
        "gpt_base_url",
    ] {
        if lowered.contains(forbidden) {
            bail!("service definition contains forbidden material {forbidden:?}");
        }
    }
    Ok(())
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn systemd_arg(path: &Path) -> Result<String> {
    let value = path.to_string_lossy();
    if value.contains(['\n', '\r', '\0']) {
        bail!("systemd path contains a control character");
    }
    Ok(value.replace('%', "%%").replace(' ', "\\x20"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn paths(root: &Path) -> ServicePaths {
        ServicePaths {
            restlessd: root.join("current/restlessd"),
            restless: root.join("current/restless"),
            cockpit_dir: root.join("current/web"),
            state_root: root.join("state"),
        }
    }

    #[test]
    fn singleton_refuses_a_second_writer_and_releases_cleanly() {
        let root =
            std::env::temp_dir().join(format!("restless-singleton-{}", uuid::Uuid::new_v4()));
        let profile = MachineProfile {
            kind: ProfileKind::Test,
            state_root: root.clone(),
            port_offset: 20_001,
            resource_namespace: "singleton_test".into(),
        };
        let first = SingletonGuard::acquire(&profile).expect("first lock");
        assert!(singleton_lock_is_held(&profile.lock_path()).unwrap());
        let error = SingletonGuard::acquire(&profile)
            .err()
            .expect("second writer must fail");
        assert!(error.to_string().contains("already owns"));
        drop(first);
        assert!(!singleton_lock_is_held(&profile.lock_path()).unwrap());
        SingletonGuard::acquire(&profile).expect("released lock");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn profiles_freeze_disjoint_machine_namespaces() {
        let stable = MachineProfile::stable(Path::new("/Users/founder")).unwrap();
        let dev = MachineProfile {
            kind: ProfileKind::Dev,
            state_root: PathBuf::from("/tmp/restless-dev"),
            port_offset: 4_321,
            resource_namespace: "checkout_42".into(),
        };
        dev.validate().unwrap();
        assert_ne!(stable.socket_path(), dev.socket_path());
        assert_ne!(stable.log_dir(), dev.log_dir());
        assert_ne!(stable.launch_cache_dir(), dev.launch_cache_dir());
        assert_ne!(
            stable.docker_container_name("aris"),
            dev.docker_container_name("aris")
        );
        assert_ne!(
            stable.docker_volume_name("aris"),
            dev.docker_volume_name("aris")
        );
        assert_ne!(stable.docker_image_name(), dev.docker_image_name());
    }

    #[test]
    fn generated_service_contracts_are_wake_only_and_secret_free() {
        let paths = paths(Path::new("/Users/founder/.local/lib/restless"));
        for definition in [
            launchd_plane_plist(&paths).unwrap(),
            launchd_wake_plist(&paths).unwrap(),
            systemd_plane_unit(&paths).unwrap(),
            systemd_wake_service(&paths).unwrap(),
            systemd_wake_timer().to_string(),
        ] {
            validate_service_definition(&definition).unwrap();
            assert!(!definition.contains("company"));
            assert!(!definition.contains("reason"));
        }
        let launchd_wake = launchd_wake_plist(&paths).unwrap();
        let launchd_plane = launchd_plane_plist(&paths).unwrap();
        assert!(launchd_plane.contains("<key>ProcessType</key><string>Interactive</string>"));
        assert!(launchd_wake.contains("wake-due"));
        assert!(launchd_wake.contains("<key>ProcessType</key><string>Background</string>"));
        assert!(!launchd_wake.contains("KeepAlive"));
    }

    #[test]
    fn test_profile_is_explicit_and_self_identifying() {
        let invalid = MachineProfile {
            kind: ProfileKind::Test,
            state_root: PathBuf::from("/tmp/restless-test"),
            port_offset: 20_500,
            resource_namespace: "ambiguous".into(),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn drain_gate_counts_entered_work_and_refuses_new_work() {
        let gate = LifecycleGate::default();
        let first = gate.try_enter().expect("open gate");
        assert_eq!(gate.active(), 1);
        gate.begin_drain();
        assert!(gate.is_draining());
        assert!(gate.try_enter().is_none());
        drop(first);
        assert_eq!(gate.active(), 0);
        gate.resume();
        assert!(gate.try_enter().is_some());
    }

    #[test]
    fn recovery_and_owner_drain_are_independent_admission_barriers() {
        let gate = LifecycleGate::default();
        gate.begin_recovery();
        assert!(gate.is_recovering());
        assert!(!gate.is_draining());
        assert!(gate.try_enter().is_none());

        gate.begin_drain();
        gate.finish_recovery();
        assert!(!gate.is_recovering());
        assert!(gate.is_draining());
        assert!(gate.try_enter().is_none());

        gate.resume();
        assert!(gate.try_enter().is_some());
    }

    #[test]
    fn drain_marker_survives_replacement_and_clears_exactly() {
        let root = std::env::temp_dir().join(format!("restless-drain-{}", uuid::Uuid::new_v4()));
        write_drain_marker(&root).unwrap();
        assert!(drain_marker_exists(&root));
        let mode = std::fs::metadata(root.join(APPLIANCE_DRAIN_RELATIVE))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        clear_drain_marker(&root).unwrap();
        clear_drain_marker(&root).unwrap();
        assert!(!drain_marker_exists(&root));
        std::fs::remove_dir_all(root).ok();
    }
}
