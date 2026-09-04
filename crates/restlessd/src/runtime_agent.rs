//! Cell-local implementation of the public Runtime Agent protocol.
//!
//! This module has no listener. The binary drives it exclusively from the
//! outbound account-plane WebSocket. Filesystem operations use directory
//! capabilities rooted at explicitly approved `/company` subtrees, and child
//! processes run as the unprivileged `company` user, separate from the
//! credential-custody identity used by this agent.

use std::collections::{BTreeSet, HashMap};
use std::fs::{self, OpenOptions as StdOpenOptions};
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd as _;
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt as _;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use cap_std::ambient_authority;
use cap_std::fs::{
    Dir, Metadata, MetadataExt as _, OpenOptions, OpenOptionsExt as _, Permissions,
    PermissionsExt as _,
};
use chrono::{DateTime, Utc};
use nix::sys::signal::{killpg, Signal};
#[cfg(target_os = "linux")]
use nix::unistd::getegid;
use nix::unistd::{geteuid, Pid};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio::process::{ChildStdin, Command};
use tokio::sync::{mpsc, Mutex};
use url::Url;
use uuid::Uuid;

use crate::hosted_runtime::HostedRuntimeIdentity;
use crate::runtime_agent_protocol::*;

#[cfg(target_os = "linux")]
const CAP_KILL_NUMBER: u32 = 5;
#[cfg(target_os = "linux")]
const CAP_SETGID_NUMBER: u32 = 6;
#[cfg(target_os = "linux")]
const CAP_SETUID_NUMBER: u32 = 7;
#[cfg(target_os = "linux")]
const CAP_SETPCAP_NUMBER: u32 = 8;
#[cfg(target_os = "linux")]
const AGENT_CAPABILITIES: [u32; 4] = [
    CAP_KILL_NUMBER,
    CAP_SETGID_NUMBER,
    CAP_SETUID_NUMBER,
    CAP_SETPCAP_NUMBER,
];

const MAX_ARGUMENTS: usize = 256;
const MAX_ARGUMENT_BYTES: usize = 128 * 1024;
const MAX_ENVIRONMENT: usize = 128;
const MAX_DIRECTORY_ENTRIES: u16 = 256;
const MAX_ACTIVE_RESOURCES: usize = 1_024;
const MAX_OPERATION_RECEIPTS: usize = 2_048;
const MAX_REQUEST_FUTURE_SECONDS: i64 = 24 * 60 * 60;
const DEFAULT_SERVICE_IDLE_MS: u32 = 30_000;
const MAX_SERVICE_IDLE_MS: u32 = 24 * 60 * 60 * 1_000;
const PROCESS_OUTPUT_CHUNK: usize = 16 * 1024;
const EXITED_PROCESS_RETENTION: Duration = Duration::from_secs(5 * 60);
const DEFAULT_PUBLISHED_SERVICE_PORTS: &[u16] = &[4173];

pub const COMPANY_UID: u32 = 2_000;
pub const COMPANY_GID: u32 = 2_000;
pub const EFFECT_UID: u32 = 2_001;
pub const RUNTIME_AGENT_UID: u32 = 2_002;
pub const RUNTIME_AGENT_GID: u32 = 2_002;

/// Select the Runtime Agent's TLS implementation explicitly. The workspace
/// enables both Rustls crypto backends through independent clients, so feature
/// unification cannot safely choose a process-wide default for us.
pub fn install_runtime_agent_tls_crypto_provider() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

/// Enter the Linux credential-custody identity before constructing a Tokio
/// runtime (Linux credentials and capabilities are per-thread kernel state).
/// The container grants root only the small bounding set needed by init; this
/// transition retains exactly KILL/SETGID/SETUID/SETPCAP. SETPCAP is required
/// solely so each pre-exec child can empty its own capability bounding set.
#[cfg(target_os = "linux")]
pub fn enter_runtime_agent_security_context() -> Result<(), RuntimeAgentError> {
    if geteuid().as_raw() != 0 || getegid().as_raw() != 0 {
        return Err(RuntimeAgentError::InvalidConfiguration(
            "Runtime Agent must begin its privilege transition as root",
        ));
    }
    linux_prune_bounding_set(&AGENT_CAPABILITIES).map_err(RuntimeAgentError::file)?;
    unsafe {
        if nix::libc::prctl(nix::libc::PR_SET_KEEPCAPS, 1, 0, 0, 0) != 0
            || nix::libc::setgroups(0, std::ptr::null()) != 0
            || nix::libc::setresgid(RUNTIME_AGENT_GID, RUNTIME_AGENT_GID, RUNTIME_AGENT_GID) != 0
            || nix::libc::setresuid(RUNTIME_AGENT_UID, RUNTIME_AGENT_UID, RUNTIME_AGENT_UID) != 0
        {
            return Err(RuntimeAgentError::file(io::Error::last_os_error()));
        }
    }
    linux_set_capabilities(&AGENT_CAPABILITIES).map_err(RuntimeAgentError::file)?;
    unsafe {
        if nix::libc::prctl(nix::libc::PR_SET_KEEPCAPS, 0, 0, 0, 0) != 0
            || nix::libc::prctl(
                nix::libc::PR_CAP_AMBIENT,
                nix::libc::PR_CAP_AMBIENT_CLEAR_ALL,
                0,
                0,
                0,
            ) != 0
            || nix::libc::prctl(nix::libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
            || nix::libc::prctl(nix::libc::PR_SET_DUMPABLE, 0, 0, 0, 0) != 0
        {
            return Err(RuntimeAgentError::file(io::Error::last_os_error()));
        }
    }
    verify_runtime_agent_security_context()
}

#[cfg(not(target_os = "linux"))]
pub fn enter_runtime_agent_security_context() -> Result<(), RuntimeAgentError> {
    Err(RuntimeAgentError::InvalidConfiguration(
        "Runtime Agent credential custody is supported only on Linux",
    ))
}

#[cfg(target_os = "linux")]
pub fn verify_runtime_agent_security_context() -> Result<(), RuntimeAgentError> {
    let (effective, permitted, inheritable) =
        linux_get_capabilities().map_err(RuntimeAgentError::file)?;
    let expected = linux_capability_mask(&AGENT_CAPABILITIES);
    if geteuid().as_raw() != RUNTIME_AGENT_UID
        || getegid().as_raw() != RUNTIME_AGENT_GID
        || effective != expected
        || permitted != expected
        || inheritable != 0
        || !linux_bounding_set_is_exact(&AGENT_CAPABILITIES).map_err(RuntimeAgentError::file)?
        || !linux_no_new_privileges().map_err(RuntimeAgentError::file)?
        || !linux_ambient_set_is_empty().map_err(RuntimeAgentError::file)?
    {
        return Err(RuntimeAgentError::InvalidConfiguration(
            "Runtime Agent Linux privilege boundary is not exact",
        ));
    }
    Ok(())
}

/// Executed inside the same company-UID boundary used for every governed
/// child. Readiness invokes this exact probe rather than testing execution as
/// the credential-custody identity.
#[cfg(target_os = "linux")]
pub fn run_company_security_probe() -> Result<(), RuntimeAgentError> {
    verify_company_worker_security()?;
    verify_private_state_is_inaccessible_to_company()
}

#[cfg(not(target_os = "linux"))]
pub fn run_company_security_probe() -> Result<(), RuntimeAgentError> {
    Err(RuntimeAgentError::InvalidConfiguration(
        "Runtime company security probe is supported only on Linux",
    ))
}

#[cfg(target_os = "linux")]
fn verify_private_state_is_inaccessible_to_company() -> Result<(), RuntimeAgentError> {
    let root = Path::new(EXPECTED_PRIVATE_STATE_ROOT);
    let probe = root.join(PRIVATE_CUSTODY_PROBE_FILE);
    let renamed = root.join(".company-custody-probe-renamed");
    expect_company_permission_denied(
        fs::read_dir(root).map(|_| ()),
        "company UID can list Runtime Agent control state",
    )?;
    expect_company_permission_denied(
        fs::read(&probe).map(|_| ()),
        "company UID can read Runtime Agent control state",
    )?;
    expect_company_permission_denied(
        StdOpenOptions::new().write(true).open(&probe).map(|_| ()),
        "company UID can write Runtime Agent control state",
    )?;
    expect_company_permission_denied(
        fs::set_permissions(&probe, fs::Permissions::from_mode(0o644)),
        "company UID can chmod Runtime Agent control state",
    )?;
    expect_company_permission_denied(
        fs::rename(&probe, &renamed),
        "company UID can rename Runtime Agent control state",
    )?;
    expect_company_permission_denied(
        fs::remove_file(&probe),
        "company UID can unlink Runtime Agent control state",
    )?;
    expect_company_permission_denied(
        StdOpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(root.join(".company-custody-create-probe"))
            .map(|_| ()),
        "company UID can create Runtime Agent control state",
    )
}

#[cfg(target_os = "linux")]
fn expect_company_permission_denied(
    result: io::Result<()>,
    escaped_boundary: &'static str,
) -> Result<(), RuntimeAgentError> {
    match result {
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => Ok(()),
        Ok(()) => Err(RuntimeAgentError::InvalidConfiguration(escaped_boundary)),
        Err(error) => Err(RuntimeAgentError::file(error)),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn verify_runtime_agent_security_context() -> Result<(), RuntimeAgentError> {
    Err(RuntimeAgentError::InvalidConfiguration(
        "Runtime Agent credential custody is supported only on Linux",
    ))
}

#[cfg(target_os = "linux")]
fn verify_company_worker_security() -> Result<(), RuntimeAgentError> {
    verify_unprivileged_worker_security(COMPANY_UID)
}

#[cfg(target_os = "linux")]
fn verify_effect_worker_security() -> Result<(), RuntimeAgentError> {
    verify_unprivileged_worker_security(EFFECT_UID)
}

#[cfg(target_os = "linux")]
fn verify_unprivileged_worker_security(expected_uid: u32) -> Result<(), RuntimeAgentError> {
    let (effective, permitted, inheritable) =
        linux_get_capabilities().map_err(RuntimeAgentError::file)?;
    if geteuid().as_raw() != expected_uid
        || getegid().as_raw() != COMPANY_GID
        || effective != 0
        || permitted != 0
        || inheritable != 0
        || !linux_bounding_set_is_exact(&[]).map_err(RuntimeAgentError::file)?
        || !linux_no_new_privileges().map_err(RuntimeAgentError::file)?
        || !linux_ambient_set_is_empty().map_err(RuntimeAgentError::file)?
    {
        return Err(RuntimeAgentError::InvalidConfiguration(
            "governed Runtime worker retained privilege",
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn configure_company_child(command: &mut Command) {
    // SAFETY: the closure performs only direct, async-signal-safe Linux
    // syscalls between fork and exec. It allocates nothing and captures no
    // mutable process state.
    unsafe {
        command.pre_exec(company_child_pre_exec);
    }
}

#[cfg(target_os = "linux")]
fn configure_effect_child(command: &mut Command) {
    // SAFETY: the closure performs only direct, async-signal-safe Linux
    // syscalls between fork and exec.
    unsafe {
        command.pre_exec(effect_child_pre_exec);
    }
}

#[cfg(target_os = "linux")]
fn configure_company_child_sync(command: &mut std::process::Command) {
    // SAFETY: identical boundary to `configure_company_child`, used by the
    // immutable-image verification entry point before any Tokio threads exist.
    unsafe {
        command.pre_exec(company_child_pre_exec);
    }
}

/// Exercise the complete agent-to-company Linux boundary without a network
/// connection. Release verification runs this inside the actual company image
/// with the same minimal container capability bounding set used in production.
#[cfg(target_os = "linux")]
pub fn run_runtime_agent_security_self_test() -> Result<(), RuntimeAgentError> {
    verify_runtime_agent_security_context()?;
    let root = Path::new(EXPECTED_PRIVATE_STATE_ROOT);
    validate_private_directory(root, 0o700)?;
    ensure_private_custody_probe(root)?;

    let executable = std::env::current_exe().map_err(RuntimeAgentError::file)?;
    let mut command = std::process::Command::new(executable);
    command
        .arg("--security-probe-worker")
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_company_child_sync(&mut command);
    let status = command.status().map_err(RuntimeAgentError::process)?;
    if !status.success() {
        return Err(RuntimeAgentError::InvalidConfiguration(
            "governed Runtime worker failed its Linux privilege proof",
        ));
    }
    validate_private_file(&root.join(PRIVATE_CUSTODY_PROBE_FILE), 0o600, 128)?;
    if root.join(".company-custody-probe-renamed").exists()
        || root.join(".company-custody-create-probe").exists()
    {
        return Err(RuntimeAgentError::InvalidConfiguration(
            "company UID mutated Runtime Agent control state",
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn run_runtime_agent_security_self_test() -> Result<(), RuntimeAgentError> {
    Err(RuntimeAgentError::InvalidConfiguration(
        "Runtime Agent credential custody is supported only on Linux",
    ))
}

#[cfg(target_os = "linux")]
fn company_child_pre_exec() -> io::Result<()> {
    unprivileged_child_pre_exec(COMPANY_UID)
}

#[cfg(target_os = "linux")]
fn effect_child_pre_exec() -> io::Result<()> {
    unprivileged_child_pre_exec(EFFECT_UID)
}

#[cfg(target_os = "linux")]
fn unprivileged_child_pre_exec(uid: u32) -> io::Result<()> {
    linux_prune_bounding_set(&[])?;
    unsafe {
        if nix::libc::setgroups(0, std::ptr::null()) != 0
            || nix::libc::setresgid(COMPANY_GID, COMPANY_GID, COMPANY_GID) != 0
            || nix::libc::setresuid(uid, uid, uid) != 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    linux_set_capabilities(&[])?;
    unsafe {
        if nix::libc::prctl(
            nix::libc::PR_CAP_AMBIENT,
            nix::libc::PR_CAP_AMBIENT_CLEAR_ALL,
            0,
            0,
            0,
        ) != 0
            || nix::libc::prctl(nix::libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
            || nix::libc::prctl(nix::libc::PR_SET_DUMPABLE, 0, 0, 0, 0) != 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn linux_prune_bounding_set(allowed: &[u32]) -> io::Result<()> {
    for capability in 0..=63_u32 {
        let present = unsafe {
            nix::libc::prctl(
                nix::libc::PR_CAPBSET_READ,
                capability as nix::libc::c_ulong,
                0,
                0,
                0,
            )
        };
        if present < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(nix::libc::EINVAL) {
                break;
            }
            return Err(error);
        }
        if present == 1 && !allowed.contains(&capability) {
            let dropped = unsafe {
                nix::libc::prctl(
                    nix::libc::PR_CAPBSET_DROP,
                    capability as nix::libc::c_ulong,
                    0,
                    0,
                    0,
                )
            };
            if dropped != 0 {
                return Err(io::Error::last_os_error());
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn linux_bounding_set_is_exact(expected: &[u32]) -> io::Result<bool> {
    for capability in 0..=63_u32 {
        let present = unsafe {
            nix::libc::prctl(
                nix::libc::PR_CAPBSET_READ,
                capability as nix::libc::c_ulong,
                0,
                0,
                0,
            )
        };
        if present < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(nix::libc::EINVAL) {
                return Ok(true);
            }
            return Err(error);
        }
        if (present == 1) != expected.contains(&capability) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(target_os = "linux")]
fn linux_capability_mask(capabilities: &[u32]) -> u64 {
    capabilities
        .iter()
        .fold(0_u64, |mask, capability| mask | (1_u64 << capability))
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct LinuxCapabilityHeader {
    version: u32,
    pid: i32,
}

#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LinuxCapabilityData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}

#[cfg(target_os = "linux")]
fn linux_set_capabilities(capabilities: &[u32]) -> io::Result<()> {
    let mask = linux_capability_mask(capabilities);
    let mut header = LinuxCapabilityHeader {
        version: 0x2008_0522,
        pid: 0,
    };
    let data = [
        LinuxCapabilityData {
            effective: mask as u32,
            permitted: mask as u32,
            inheritable: 0,
        },
        LinuxCapabilityData {
            effective: (mask >> 32) as u32,
            permitted: (mask >> 32) as u32,
            inheritable: 0,
        },
    ];
    let result = unsafe {
        nix::libc::syscall(
            nix::libc::SYS_capset,
            &mut header as *mut LinuxCapabilityHeader,
            data.as_ptr(),
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
fn linux_get_capabilities() -> io::Result<(u64, u64, u64)> {
    let mut header = LinuxCapabilityHeader {
        version: 0x2008_0522,
        pid: 0,
    };
    let mut data = [LinuxCapabilityData::default(); 2];
    let result = unsafe {
        nix::libc::syscall(
            nix::libc::SYS_capget,
            &mut header as *mut LinuxCapabilityHeader,
            data.as_mut_ptr(),
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((
        u64::from(data[0].effective) | (u64::from(data[1].effective) << 32),
        u64::from(data[0].permitted) | (u64::from(data[1].permitted) << 32),
        u64::from(data[0].inheritable) | (u64::from(data[1].inheritable) << 32),
    ))
}

#[cfg(target_os = "linux")]
fn linux_no_new_privileges() -> io::Result<bool> {
    let value = unsafe { nix::libc::prctl(nix::libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) };
    if value < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(value == 1)
    }
}

#[cfg(target_os = "linux")]
fn linux_ambient_set_is_empty() -> io::Result<bool> {
    for capability in 0..=63_u32 {
        let value = unsafe {
            nix::libc::prctl(
                nix::libc::PR_CAP_AMBIENT,
                nix::libc::PR_CAP_AMBIENT_IS_SET,
                capability as nix::libc::c_ulong,
                0,
                0,
            )
        };
        if value < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(nix::libc::EINVAL) {
                return Ok(true);
            }
            return Err(error);
        }
        if value != 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

const BRIDGE_URL_ENV: &str = "RESTLESS_RUNTIME_BRIDGE_URL";
const OWNER_ID_ENV: &str = "RESTLESS_RUNTIME_OWNER_ID";
const PLANE_ID_ENV: &str = "RESTLESS_RUNTIME_PLANE_ID";
const COMPANY_ID_ENV: &str = "RESTLESS_RUNTIME_COMPANY_ID";
const CELL_ID_ENV: &str = "RESTLESS_RUNTIME_CELL_ID";
const COMPANY_ENV: &str = "RESTLESS_COMPANY";
const RUNTIME_ID_ENV: &str = "RESTLESS_RUNTIME_ID";
const GENERATION_ENV: &str = "RESTLESS_RUNTIME_GENERATION";
const DESIRED_REVISION_ENV: &str = "RESTLESS_RUNTIME_DESIRED_REVISION";
const RUNTIME_IMAGE_ENV: &str = "RESTLESS_RUNTIME_IMAGE";
const VOLUME_NAME_ENV: &str = "RESTLESS_RUNTIME_VOLUME_NAME";
const SOURCE_REVISION_ENV: &str = "RESTLESS_SOURCE_REVISION";
const CAPABILITY_FILE_ENV: &str = "RESTLESS_RUNTIME_BRIDGE_CAPABILITY_FILE";
const CAPABILITY_STATE_FILE_ENV: &str = "RESTLESS_RUNTIME_BRIDGE_CAPABILITY_STATE_FILE";
const EXPECTED_BOOTSTRAP_PATH: &str = "/run/restless-agent/runtime-bridge-bootstrap";
const EXPECTED_STATE_PATH: &str = "/var/lib/restless-runtime-agent/runtime-bridge-capability";
const EXPECTED_PRIVATE_STATE_ROOT: &str = "/var/lib/restless-runtime-agent";
const PRIVATE_CUSTODY_PROBE_FILE: &str = ".company-custody-probe";

#[derive(Debug, Clone)]
pub struct RuntimeAgentConfig {
    pub bridge_url: Url,
    pub identity: HostedRuntimeIdentity,
    pub desired_revision: i64,
    pub core_company: String,
    pub company_root: PathBuf,
    pub capability_file: PathBuf,
    pub capability_state_file: PathBuf,
    pub private_state_root: PathBuf,
    pub operation_journal_file: PathBuf,
    pub published_service_ports: BTreeSet<u16>,
    pub release: RuntimeReleaseIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeReleaseIdentity {
    pub core_version: String,
    pub api_contract_version: String,
    pub assertion_contract_version: String,
    pub schema_version: String,
}

impl RuntimeAgentConfig {
    pub fn from_environment() -> Result<Self, RuntimeAgentError> {
        let values = RuntimeAgentConfigValues {
            bridge_url: required_env(BRIDGE_URL_ENV)?,
            owner_id: required_env(OWNER_ID_ENV)?,
            plane_id: required_env(PLANE_ID_ENV)?,
            company_id: required_env(COMPANY_ID_ENV)?,
            cell_id: required_env(CELL_ID_ENV)?,
            core_company: required_env(COMPANY_ENV)?,
            runtime_id: required_env(RUNTIME_ID_ENV)?,
            runtime_generation: required_env(GENERATION_ENV)?,
            desired_revision: required_env(DESIRED_REVISION_ENV)?,
            runtime_image: required_env(RUNTIME_IMAGE_ENV)?,
            volume_name: required_env(VOLUME_NAME_ENV)?,
            source_revision: required_env(SOURCE_REVISION_ENV)?,
            capability_file: PathBuf::from(required_env(CAPABILITY_FILE_ENV)?),
            capability_state_file: PathBuf::from(required_env(CAPABILITY_STATE_FILE_ENV)?),
            company_root: PathBuf::from("/company"),
            release: RuntimeReleaseIdentity {
                core_version: required_env("RESTLESS_CORE_VERSION")?,
                api_contract_version: required_env("RESTLESS_API_CONTRACT_VERSION")?,
                assertion_contract_version: required_env("RESTLESS_ASSERTION_CONTRACT_VERSION")?,
                schema_version: required_env("RESTLESS_SCHEMA_VERSION")?,
            },
            published_service_ports: DEFAULT_PUBLISHED_SERVICE_PORTS.iter().copied().collect(),
        };
        let config = Self::from_values(values)?;
        if config.capability_file != Path::new(EXPECTED_BOOTSTRAP_PATH)
            || config.capability_state_file != Path::new(EXPECTED_STATE_PATH)
            || config.private_state_root != Path::new(EXPECTED_PRIVATE_STATE_ROOT)
        {
            return Err(RuntimeAgentError::InvalidConfiguration(
                "Runtime capability paths do not match the released cell contract",
            ));
        }
        Ok(config)
    }

    pub fn from_values(values: RuntimeAgentConfigValues) -> Result<Self, RuntimeAgentError> {
        let bridge_url = validate_bridge_url(&values.bridge_url)?;
        let owner_id = parse_uuid(OWNER_ID_ENV, &values.owner_id)?;
        let plane_id = parse_uuid(PLANE_ID_ENV, &values.plane_id)?;
        let company_id = parse_uuid(COMPANY_ID_ENV, &values.company_id)?;
        let cell_id = parse_uuid(CELL_ID_ENV, &values.cell_id)?;
        let runtime_generation = parse_positive_i64(GENERATION_ENV, &values.runtime_generation)?;
        let desired_revision = parse_positive_i64(DESIRED_REVISION_ENV, &values.desired_revision)?;
        let expected_runtime_id = format!("restless-cell-{cell_id}");
        let expected_volume_name = format!("{expected_runtime_id}-data");
        let expected_company = format!("c{}", company_id.simple());
        if values.runtime_id != expected_runtime_id
            || values.volume_name != expected_volume_name
            || values.core_company != expected_company
        {
            return Err(RuntimeAgentError::InvalidConfiguration(
                "Runtime, volume, or Core company identity does not match the Fleet UUIDs",
            ));
        }
        if !immutable_image(&values.runtime_image) {
            return Err(RuntimeAgentError::InvalidConfiguration(
                "RESTLESS_RUNTIME_IMAGE must be an immutable sha256 OCI reference",
            ));
        }
        if !exact_revision(&values.source_revision) {
            return Err(RuntimeAgentError::InvalidConfiguration(
                "RESTLESS_SOURCE_REVISION must be an exact lowercase Git revision",
            ));
        }
        let private_state_root = values.capability_state_file.parent().ok_or(
            RuntimeAgentError::InvalidConfiguration(
                "Runtime capability state has no private parent",
            ),
        )?;
        if !values.company_root.is_absolute()
            || !values.capability_file.is_absolute()
            || !values.capability_state_file.is_absolute()
            || values.capability_file.starts_with(&values.company_root)
            || private_state_root.starts_with(&values.company_root)
            || values
                .published_service_ports
                .iter()
                .any(|port| !valid_published_port(*port))
        {
            return Err(RuntimeAgentError::InvalidConfiguration(
                "Runtime filesystem or service allow-list configuration is invalid",
            ));
        }
        validate_release(&values.release)?;
        let private_state_root = private_state_root.to_path_buf();
        let operation_journal_file = private_state_root.join("runtime-operations.json");
        Ok(Self {
            bridge_url,
            identity: HostedRuntimeIdentity {
                owner_id,
                plane_id,
                company_id,
                cell_id,
                runtime_id: values.runtime_id,
                runtime_generation,
                runtime_image: values.runtime_image,
                volume_name: values.volume_name,
                source_revision: values.source_revision,
            },
            desired_revision,
            core_company: values.core_company,
            company_root: values.company_root,
            capability_file: values.capability_file,
            capability_state_file: values.capability_state_file,
            private_state_root,
            operation_journal_file,
            published_service_ports: values.published_service_ports,
            release: values.release,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeAgentConfigValues {
    pub bridge_url: String,
    pub owner_id: String,
    pub plane_id: String,
    pub company_id: String,
    pub cell_id: String,
    pub core_company: String,
    pub runtime_id: String,
    pub runtime_generation: String,
    pub desired_revision: String,
    pub runtime_image: String,
    pub volume_name: String,
    pub source_revision: String,
    pub capability_file: PathBuf,
    pub capability_state_file: PathBuf,
    pub company_root: PathBuf,
    pub release: RuntimeReleaseIdentity,
    pub published_service_ports: BTreeSet<u16>,
}

#[derive(Debug, Clone)]
pub struct RuntimeRequestSequence {
    next: u64,
}

impl RuntimeRequestSequence {
    pub fn new(next: u64) -> Result<Self, RuntimeAgentError> {
        if next == 0 {
            return Err(RuntimeAgentError::InvalidConfiguration(
                "Runtime request sequence must begin above zero",
            ));
        }
        Ok(Self { next })
    }

    fn accept(&mut self, value: u64) -> Result<(), RuntimeAgentError> {
        if value != self.next {
            return Err(RuntimeAgentError::SequenceViolation);
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or(RuntimeAgentError::SequenceViolation)?;
        Ok(())
    }
}

pub struct RuntimeCapabilityStore {
    bootstrap_path: PathBuf,
    state_path: PathBuf,
}

impl RuntimeCapabilityStore {
    pub fn new(bootstrap_path: PathBuf, state_path: PathBuf) -> Self {
        Self {
            bootstrap_path,
            state_path,
        }
    }

    /// A newly staged one-use bootstrap belongs to the exact replacement
    /// generation and therefore precedes an older persisted reconnect grant.
    /// On an ordinary restart a consumed bootstrap is rejected and the still
    /// valid persisted grant is the bounded fallback.
    pub fn candidates(&self) -> Result<Vec<RuntimeBridgeCapability>, RuntimeAgentError> {
        let mut values = Vec::new();
        if self.bootstrap_path.exists() {
            let bootstrap = read_capability(&self.bootstrap_path, 0o400)?;
            values.push(bootstrap);
        }
        if self.state_path.exists() {
            let persisted = read_capability(&self.state_path, 0o600)?;
            if values.iter().all(|value| value != &persisted) {
                values.push(persisted);
            }
        }
        if values.is_empty() {
            return Err(RuntimeAgentError::CapabilityUnavailable);
        }
        Ok(values)
    }

    pub fn persist_rotation(
        &self,
        capability: &RuntimeBridgeCapability,
    ) -> Result<(), RuntimeAgentError> {
        let parent = self
            .state_path
            .parent()
            .ok_or(RuntimeAgentError::Internal)?;
        fs::create_dir_all(parent).map_err(RuntimeAgentError::file)?;
        let temporary = parent.join(format!(".runtime-capability-{}.tmp", Uuid::new_v4()));
        let mut file = StdOpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(RuntimeAgentError::file)?;
        let result = file
            .write_all(capability.expose().as_bytes())
            .and_then(|()| file.sync_all())
            .and_then(|()| fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600)))
            .and_then(|()| fs::rename(&temporary, &self.state_path));
        if let Err(error) = result {
            let _ = fs::remove_file(&temporary);
            return Err(RuntimeAgentError::file(error));
        }
        sync_parent(parent)?;
        validate_private_file(&self.state_path, 0o600, 16_384)?;
        Ok(())
    }

    pub fn discard_bootstrap(&self) -> Result<(), RuntimeAgentError> {
        match fs::remove_file(&self.bootstrap_path) {
            Ok(()) => {
                let parent = self
                    .bootstrap_path
                    .parent()
                    .ok_or(RuntimeAgentError::Internal)?;
                sync_parent(parent)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(RuntimeAgentError::file(error)),
        }
    }
}

fn read_capability(
    path: &Path,
    exact_mode: u32,
) -> Result<RuntimeBridgeCapability, RuntimeAgentError> {
    validate_private_file(path, exact_mode, 16_384)?;
    let value = fs::read_to_string(path).map_err(RuntimeAgentError::file)?;
    RuntimeBridgeCapability::new(value).map_err(|_| RuntimeAgentError::CapabilityUnavailable)
}

fn validate_private_file(
    path: &Path,
    exact_mode: u32,
    maximum: u64,
) -> Result<(), RuntimeAgentError> {
    let metadata = fs::symlink_metadata(path).map_err(RuntimeAgentError::file)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > maximum
        || metadata.mode() & 0o777 != exact_mode
        || metadata.uid() != geteuid().as_raw()
    {
        return Err(RuntimeAgentError::CapabilityUnavailable);
    }
    Ok(())
}

fn validate_private_directory(path: &Path, exact_mode: u32) -> Result<(), RuntimeAgentError> {
    let metadata = fs::symlink_metadata(path).map_err(RuntimeAgentError::file)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.mode() & 0o777 != exact_mode
        || metadata.uid() != geteuid().as_raw()
    {
        return Err(RuntimeAgentError::InvalidConfiguration(
            "Runtime Agent control state is not private to its Linux identity",
        ));
    }
    Ok(())
}

fn validate_envelope(
    sequence: &mut RuntimeRequestSequence,
    envelope: &RuntimeRequestEnvelope,
    now: DateTime<Utc>,
) -> Result<(), RuntimeProtocolError> {
    if envelope.operation_id.is_nil() {
        return Err(protocol_error(
            RuntimeProtocolErrorCode::InvalidRequest,
            "operation ID must be non-nil",
            false,
        ));
    }
    sequence
        .accept(envelope.session_sequence)
        .map_err(|error| error.protocol())?;
    if envelope.deadline <= now {
        return Err(protocol_error(
            RuntimeProtocolErrorCode::Expired,
            "Runtime request deadline elapsed",
            false,
        ));
    }
    if envelope.deadline > now + chrono::Duration::seconds(MAX_REQUEST_FUTURE_SECONDS) {
        return Err(protocol_error(
            RuntimeProtocolErrorCode::InvalidRequest,
            "Runtime request deadline is too far in the future",
            false,
        ));
    }
    Ok(())
}

fn validate_process_request(
    request: &ProcessStartRequest,
    company: &str,
) -> Result<(), RuntimeAgentError> {
    if request.process_id.is_nil()
        || request.executable.is_empty()
        || request.executable.len() > 1024
        || request.executable.contains(['\0', '\r', '\n'])
        || request.arguments.len() > MAX_ARGUMENTS
        || request.arguments.iter().map(String::len).sum::<usize>() > MAX_ARGUMENT_BYTES
        || request
            .arguments
            .iter()
            .any(|argument| argument.len() > 16_384 || argument.contains('\0'))
        || request.environment.len() > MAX_ENVIRONMENT
    {
        return Err(RuntimeAgentError::InvalidRequest(
            "process request is invalid or too large",
        ));
    }
    let authority_company = match &request.authority {
        ProcessAuthority::Attempt {
            company,
            actor,
            responsibility,
            work_id,
            attempt_id,
            session_id,
        } => {
            if work_id.is_nil()
                || attempt_id.is_nil()
                || [actor, responsibility, session_id]
                    .iter()
                    .any(|value| !valid_label(value))
            {
                return Err(RuntimeAgentError::InvalidRequest(
                    "productive process authority is incomplete",
                ));
            }
            company
        }
        ProcessAuthority::AuthorityEvent {
            company,
            actor,
            responsibility,
            event_id,
            session_id,
        } => {
            if *event_id < 1
                || [actor, responsibility, session_id]
                    .iter()
                    .any(|value| !valid_label(value))
            {
                return Err(RuntimeAgentError::InvalidRequest(
                    "productive event authority is incomplete",
                ));
            }
            company
        }
        ProcessAuthority::GovernedEffect {
            company,
            actor,
            effect_class,
            authority_id,
            idempotency_key,
            execution_no,
            staging_id,
            ..
        } => {
            if *authority_id < 1
                || *execution_no < 1
                || staging_id.is_nil()
                || [actor, effect_class, idempotency_key]
                    .iter()
                    .any(|value| !valid_label(value))
            {
                return Err(RuntimeAgentError::InvalidRequest(
                    "governed effect authority is incomplete",
                ));
            }
            company
        }
        ProcessAuthority::InfrastructureProbe { company, probe } => {
            if !valid_label(probe) {
                return Err(RuntimeAgentError::InvalidRequest(
                    "infrastructure probe authority is invalid",
                ));
            }
            company
        }
    };
    if authority_company != company {
        return Err(RuntimeAgentError::PermissionDenied);
    }
    for (name, value) in &request.environment {
        if name.is_empty()
            || name.len() > 128
            || !name.bytes().enumerate().all(|(index, byte)| {
                byte == b'_' || byte.is_ascii_uppercase() || (index > 0 && byte.is_ascii_digit())
            })
            || value.expose().len() > 32 * 1024
            || value.expose().contains('\0')
            || name.starts_with("RESTLESS_RUNTIME_BRIDGE_")
        {
            return Err(RuntimeAgentError::InvalidRequest(
                "process environment is invalid or contains a reserved name",
            ));
        }
    }
    validate_working_directory(&request.working_directory)?;
    Ok(())
}

fn effect_process_identity(authority: &ProcessAuthority) -> bool {
    matches!(
        authority,
        ProcessAuthority::GovernedEffect { phase, .. }
            if !matches!(
                phase,
                EffectProcessPhase::WorkspaceAccess
                    | EffectProcessPhase::RecoverCompanyProcess
            )
    )
}

fn apply_authoritative_process_environment(command: &mut Command, authority: &ProcessAuthority) {
    match authority {
        ProcessAuthority::Attempt {
            company,
            actor,
            responsibility,
            work_id,
            attempt_id,
            session_id,
        } => {
            command
                .env("RESTLESS_COMPANY", company)
                .env("RESTLESS_ACTOR", actor)
                .env("RESTLESS_RESPONSIBILITY", responsibility)
                .env("RESTLESS_WORK_ID", work_id.to_string())
                .env("RESTLESS_ATTEMPT_ID", attempt_id.to_string())
                .env("RESTLESS_SESSION_ID", session_id);
        }
        ProcessAuthority::AuthorityEvent {
            company,
            actor,
            responsibility,
            event_id,
            session_id,
        } => {
            command
                .env("RESTLESS_COMPANY", company)
                .env("RESTLESS_ACTOR", actor)
                .env("RESTLESS_RESPONSIBILITY", responsibility)
                .env("RESTLESS_AUTHORITY_EVENT_ID", event_id.to_string())
                .env("RESTLESS_SESSION_ID", session_id);
        }
        ProcessAuthority::GovernedEffect {
            company,
            actor,
            effect_class,
            authority_id,
            idempotency_key,
            execution_no,
            staging_id,
            phase,
        } => {
            command
                .env("RESTLESS_COMPANY", company)
                .env("RESTLESS_ACTOR", actor)
                .env("RESTLESS_EFFECT_CLASS", effect_class)
                .env("RESTLESS_EFFECT_AUTHORITY_ID", authority_id.to_string())
                .env("RESTLESS_EFFECT_IDEMPOTENCY_KEY", idempotency_key)
                .env("RESTLESS_EFFECT_EXECUTION_NO", execution_no.to_string())
                .env("RESTLESS_EFFECT_STAGING_ID", staging_id.to_string())
                .env(
                    "RESTLESS_EFFECT_PHASE",
                    match phase {
                        EffectProcessPhase::WorkspaceAccess => "workspace_access",
                        EffectProcessPhase::ArtifactStage => "artifact_stage",
                        EffectProcessPhase::Execute => "execute",
                        EffectProcessPhase::ArtifactCleanup => "artifact_cleanup",
                        EffectProcessPhase::RecoverCompanyProcess => "recover_company_process",
                        EffectProcessPhase::RecoverEffectProcess => "recover_effect_process",
                    },
                );
        }
        ProcessAuthority::InfrastructureProbe { company, probe } => {
            command
                .env("RESTLESS_COMPANY", company)
                .env("RESTLESS_RUNTIME_PROBE", probe);
        }
    }
}

fn validate_working_directory(
    value: &RuntimeWorkingDirectory,
) -> Result<PathBuf, RuntimeAgentError> {
    if value.path.len() > 4096
        || value.path.contains(['\0', '\r', '\n'])
        || (value.path != "/company" && !value.path.starts_with("/company/"))
        || value.path[1..].contains("//")
        || Path::new(&value.path)
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(RuntimeAgentError::PermissionDenied);
    }
    let relative = value.path.strip_prefix("/company").unwrap_or_default();
    Ok(if relative.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(relative.trim_start_matches('/'))
    })
}

/// Internal process trampoline. The parent first removes every capability and
/// changes all real/effective/saved IDs to company, then this function opens
/// the cwd beneath a `/company` directory capability and replaces itself with
/// the requested executable. Stdin/stdout/stderr and the already-scrubbed
/// environment pass through the exec unchanged.
#[cfg(target_os = "linux")]
pub fn run_process_worker(arguments: Vec<String>) -> Result<(), RuntimeAgentError> {
    match geteuid().as_raw() {
        COMPANY_UID => verify_company_worker_security()?,
        EFFECT_UID => verify_effect_worker_security()?,
        _ => {
            return Err(RuntimeAgentError::InvalidConfiguration(
                "Runtime process worker has an unauthorized identity",
            ));
        }
    }
    verify_private_state_is_inaccessible_to_company()?;
    if arguments.len() < 3 || arguments[2] != "--" {
        return Err(RuntimeAgentError::InvalidRequest(
            "process worker invocation is malformed",
        ));
    }
    let working_directory = RuntimeWorkingDirectory {
        path: arguments[0].clone(),
    };
    let relative = validate_working_directory(&working_directory)?;
    let executable = &arguments[1];
    let process_arguments = &arguments[3..];
    if executable.is_empty()
        || executable.len() > 1024
        || executable.contains(['\0', '\r', '\n'])
        || process_arguments.len() > MAX_ARGUMENTS
        || process_arguments.iter().map(String::len).sum::<usize>() > MAX_ARGUMENT_BYTES
        || process_arguments
            .iter()
            .any(|argument| argument.len() > 16_384 || argument.contains('\0'))
    {
        return Err(RuntimeAgentError::InvalidRequest(
            "process worker command is invalid",
        ));
    }
    let company =
        Dir::open_ambient_dir("/company", ambient_authority()).map_err(RuntimeAgentError::file)?;
    let directory = company
        .open_dir(relative)
        .map_err(RuntimeAgentError::file)?;
    let changed = unsafe { nix::libc::fchdir(directory.as_raw_fd()) };
    if changed != 0 {
        return Err(RuntimeAgentError::file(io::Error::last_os_error()));
    }
    let error = std::process::Command::new(executable)
        .args(process_arguments)
        .exec();
    Err(RuntimeAgentError::process(error))
}

#[cfg(not(target_os = "linux"))]
pub fn run_process_worker(_arguments: Vec<String>) -> Result<(), RuntimeAgentError> {
    Err(RuntimeAgentError::InvalidConfiguration(
        "Runtime process worker is supported only on Linux",
    ))
}

fn company_security_probe_command() -> Result<Command, RuntimeAgentError> {
    #[cfg(all(target_os = "linux", not(test)))]
    {
        let executable = std::env::current_exe().map_err(RuntimeAgentError::file)?;
        let mut command = Command::new(executable);
        command.arg("--security-probe-worker");
        configure_company_child(&mut command);
        Ok(command)
    }
    #[cfg(any(not(target_os = "linux"), test))]
    {
        Ok(Command::new("/bin/true"))
    }
}

fn is_mutating(request: &RuntimeAgentRequest) -> bool {
    matches!(
        request,
        RuntimeAgentRequest::ProcessStart(_)
            | RuntimeAgentRequest::ProcessStdin(_)
            | RuntimeAgentRequest::ProcessSignal(_)
            | RuntimeAgentRequest::File(
                FileRequest::AtomicWrite { .. }
                    | FileRequest::UploadBegin { .. }
                    | FileRequest::UploadChunk { .. }
                    | FileRequest::UploadCommit { .. }
                    | FileRequest::UploadAbort { .. }
                    | FileRequest::Rename { .. }
            )
            | RuntimeAgentRequest::ServiceOpen(_)
            | RuntimeAgentRequest::ServiceWrite(_)
            | RuntimeAgentRequest::ServiceClose(_)
    )
}

fn request_digest(request: &RuntimeAgentRequest) -> Result<String, RuntimeAgentError> {
    let bytes = serde_json::to_vec(request).map_err(|_| RuntimeAgentError::Internal)?;
    Ok(hex_digest(&bytes))
}

fn decode_chunk(value: &str) -> Result<Vec<u8>, RuntimeAgentError> {
    if value.len() > (RUNTIME_AGENT_MAX_CHUNK_BYTES * 4 / 3) + 4 {
        return Err(RuntimeAgentError::LimitExceeded);
    }
    let bytes = BASE64
        .decode(value)
        .map_err(|_| RuntimeAgentError::InvalidRequest("payload is not canonical base64"))?;
    if bytes.len() > RUNTIME_AGENT_MAX_CHUNK_BYTES || BASE64.encode(&bytes) != value {
        return Err(RuntimeAgentError::LimitExceeded);
    }
    Ok(bytes)
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_mode(mode: u32) -> bool {
    // Owner read/write is required so a crash after rename remains
    // reconcilable by the unprivileged company worker.
    mode & !0o755 == 0 && mode & 0o600 == 0o600
}

fn valid_label(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.contains(['\0', '\r', '\n'])
}

fn check(component: RuntimeReadinessComponent, ready: bool) -> RuntimeReadinessCheck {
    RuntimeReadinessCheck {
        component,
        status: if ready {
            RuntimeCheckStatus::Ready
        } else {
            RuntimeCheckStatus::Unavailable
        },
    }
}

async fn loopback_probe(port: u16) -> bool {
    tokio::time::timeout(
        Duration::from_millis(300),
        TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)),
    )
    .await
    .is_ok_and(|result| result.is_ok())
}

fn validate_bridge_url(value: &str) -> Result<Url, RuntimeAgentError> {
    let parsed = Url::parse(value).map_err(|_| {
        RuntimeAgentError::InvalidConfiguration("RESTLESS_RUNTIME_BRIDGE_URL must be a valid URL")
    })?;
    if parsed.scheme() != "wss"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/internal/v1/runtime-bridge"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(RuntimeAgentError::InvalidConfiguration(
            "RESTLESS_RUNTIME_BRIDGE_URL must be the exact account-plane WSS endpoint",
        ));
    }
    Ok(parsed)
}

fn parse_uuid(name: &'static str, value: &str) -> Result<Uuid, RuntimeAgentError> {
    value
        .parse::<Uuid>()
        .ok()
        .filter(|parsed| !parsed.is_nil() && parsed.to_string() == value)
        .ok_or(RuntimeAgentError::MissingOrInvalidEnvironment(name))
}

fn parse_positive_i64(name: &'static str, value: &str) -> Result<i64, RuntimeAgentError> {
    value
        .parse::<i64>()
        .ok()
        .filter(|parsed| *parsed > 0 && parsed.to_string() == value)
        .ok_or(RuntimeAgentError::MissingOrInvalidEnvironment(name))
}

fn required_env(name: &'static str) -> Result<String, RuntimeAgentError> {
    std::env::var(name)
        .ok()
        .filter(|value| {
            !value.is_empty() && value.len() <= 16_384 && !value.contains(['\0', '\r', '\n'])
        })
        .ok_or(RuntimeAgentError::MissingOrInvalidEnvironment(name))
}

fn immutable_image(value: &str) -> bool {
    let Some((repository, digest)) = value.rsplit_once("@sha256:") else {
        return false;
    };
    !repository.is_empty()
        && repository.len() <= 1_024
        && !repository.contains(char::is_whitespace)
        && valid_sha256(digest)
}

fn exact_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate_release(release: &RuntimeReleaseIdentity) -> Result<(), RuntimeAgentError> {
    let values = [
        release.core_version.as_str(),
        release.api_contract_version.as_str(),
        release.assertion_contract_version.as_str(),
        release.schema_version.as_str(),
    ];
    if values
        .iter()
        .any(|value| value.is_empty() || value.len() > 128 || value.contains(['\0', '\r', '\n']))
    {
        return Err(RuntimeAgentError::InvalidConfiguration(
            "Runtime release identity is invalid",
        ));
    }
    Ok(())
}

fn valid_published_port(port: u16) -> bool {
    (1024..=u16::MAX).contains(&port) && !matches!(port, 5901 | 6080 | 7789 | 9222 | 9223)
}

#[cfg(unix)]
fn exit_signal(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt as _;
    status.signal()
}

fn sync_parent(parent: &Path) -> Result<(), RuntimeAgentError> {
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(RuntimeAgentError::file)
}

fn protocol_error(
    code: RuntimeProtocolErrorCode,
    message: &'static str,
    retryable: bool,
) -> RuntimeProtocolError {
    RuntimeProtocolError {
        code,
        message: message.to_owned(),
        retryable,
    }
}

#[derive(Debug)]
pub enum RuntimeAgentError {
    MissingOrInvalidEnvironment(&'static str),
    InvalidConfiguration(&'static str),
    CapabilityUnavailable,
    SequenceViolation,
    InvalidRequest(&'static str),
    PermissionDenied,
    NotFound,
    Conflict,
    LimitExceeded,
    ProcessUnavailable,
    ServiceUnavailable,
    File(io::ErrorKind),
    Internal,
}

impl RuntimeAgentError {
    fn file(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::AlreadyExists => Self::Conflict,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            kind => Self::File(kind),
        }
    }

    fn process(error: io::Error) -> Self {
        if error.kind() == io::ErrorKind::NotFound {
            Self::NotFound
        } else {
            Self::ProcessUnavailable
        }
    }

    pub fn protocol(&self) -> RuntimeProtocolError {
        match self {
            Self::MissingOrInvalidEnvironment(_) | Self::InvalidConfiguration(_) => protocol_error(
                RuntimeProtocolErrorCode::InvalidIdentity,
                "the Runtime identity or release configuration is invalid",
                false,
            ),
            Self::CapabilityUnavailable => protocol_error(
                RuntimeProtocolErrorCode::InvalidCapability,
                "no safe Runtime Bridge capability is available",
                false,
            ),
            Self::SequenceViolation => protocol_error(
                RuntimeProtocolErrorCode::SequenceViolation,
                "Runtime request sequence is not the next expected value",
                false,
            ),
            Self::InvalidRequest(message) => {
                protocol_error(RuntimeProtocolErrorCode::InvalidRequest, message, false)
            }
            Self::PermissionDenied => protocol_error(
                RuntimeProtocolErrorCode::PermissionDenied,
                "the Runtime operation is outside its approved scope",
                false,
            ),
            Self::NotFound => protocol_error(
                RuntimeProtocolErrorCode::ResourceNotFound,
                "the Runtime resource does not exist",
                false,
            ),
            Self::Conflict => protocol_error(
                RuntimeProtocolErrorCode::ResourceExists,
                "the Runtime resource conflicts with existing state",
                false,
            ),
            Self::LimitExceeded => protocol_error(
                RuntimeProtocolErrorCode::LimitExceeded,
                "the Runtime operation exceeds a released bound",
                false,
            ),
            Self::ProcessUnavailable => protocol_error(
                RuntimeProtocolErrorCode::ProcessUnavailable,
                "the governed process is unavailable",
                true,
            ),
            Self::ServiceUnavailable => protocol_error(
                RuntimeProtocolErrorCode::ServiceUnavailable,
                "the allow-listed Runtime service is unavailable",
                true,
            ),
            Self::File(_) | Self::Internal => protocol_error(
                RuntimeProtocolErrorCode::Internal,
                "the Runtime Agent could not complete the operation",
                true,
            ),
        }
    }
}

impl std::fmt::Display for RuntimeAgentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingOrInvalidEnvironment(name) => {
                write!(formatter, "{name} is missing or invalid")
            }
            Self::InvalidConfiguration(reason) => formatter.write_str(reason),
            Self::CapabilityUnavailable => {
                formatter.write_str("Runtime Bridge capability is unavailable")
            }
            Self::SequenceViolation => formatter.write_str("Runtime request sequence violation"),
            Self::InvalidRequest(reason) => formatter.write_str(reason),
            Self::PermissionDenied => {
                formatter.write_str("Runtime operation is outside the approved scope")
            }
            Self::NotFound => formatter.write_str("Runtime resource was not found"),
            Self::Conflict => formatter.write_str("Runtime resource conflicted"),
            Self::LimitExceeded => {
                formatter.write_str("Runtime operation exceeds a released bound")
            }
            Self::ProcessUnavailable => formatter.write_str("governed process is unavailable"),
            Self::ServiceUnavailable => formatter.write_str("allow-listed service is unavailable"),
            Self::File(kind) => write!(formatter, "Runtime filesystem operation failed ({kind:?})"),
            Self::Internal => formatter.write_str("Runtime Agent internal failure"),
        }
    }
}

impl std::error::Error for RuntimeAgentError {}

fn ensure_private_custody_probe(root: &Path) -> Result<(), RuntimeAgentError> {
    let path = root.join(PRIVATE_CUSTODY_PROBE_FILE);
    if !path.exists() {
        let mut file = StdOpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(RuntimeAgentError::file)?;
        file.write_all(b"runtime-agent-private-state\n")
            .and_then(|()| file.sync_all())
            .map_err(RuntimeAgentError::file)?;
        sync_parent(root)?;
    }
    validate_private_file(&path, 0o600, 128)
}

#[derive(Clone)]
pub struct RuntimeAgent {
    config: Arc<RuntimeAgentConfig>,
    filesystem: Arc<RuntimeFilesystem>,
    journal: Arc<OperationJournal>,
    processes: Arc<Mutex<HashMap<Uuid, Arc<ProcessRecord>>>>,
    services: Arc<Mutex<HashMap<Uuid, ServiceRecord>>>,
    events: mpsc::Sender<RuntimeAgentEvent>,
}

impl RuntimeAgent {
    pub fn new(
        config: RuntimeAgentConfig,
    ) -> Result<(Self, mpsc::Receiver<RuntimeAgentEvent>), RuntimeAgentError> {
        validate_private_directory(&config.private_state_root, 0o700)?;
        ensure_private_custody_probe(&config.private_state_root)?;
        let filesystem = RuntimeFilesystem::new(
            config.company_root.clone(),
            config.private_state_root.join("uploads"),
        )?;
        let journal = OperationJournal::open(config.operation_journal_file.clone())?;
        let (events, receiver) = mpsc::channel(1_024);
        Ok((
            Self {
                config: Arc::new(config),
                filesystem: Arc::new(filesystem),
                journal: Arc::new(journal),
                processes: Arc::new(Mutex::new(HashMap::new())),
                services: Arc::new(Mutex::new(HashMap::new())),
                events,
            },
            receiver,
        ))
    }

    pub fn registration(&self, capability: RuntimeBridgeCapability) -> RuntimeRegistration {
        RuntimeRegistration {
            protocol: RUNTIME_AGENT_PROTOCOL.to_owned(),
            identity: self.config.identity.clone(),
            desired_revision: self.config.desired_revision,
            features: RuntimeAgentFeature::ALL.to_vec(),
            capability,
        }
    }

    pub async fn handle_request(
        &self,
        sequence: &mut RuntimeRequestSequence,
        envelope: RuntimeRequestEnvelope,
        now: DateTime<Utc>,
    ) -> RuntimeResponseEnvelope {
        let operation_id = envelope.operation_id;
        let session_sequence = envelope.session_sequence;
        let response = match validate_envelope(sequence, &envelope, now) {
            Ok(()) => self.execute_with_receipt(envelope, now).await,
            Err(error) => RuntimeAgentResponse::Error(error),
        };
        RuntimeResponseEnvelope {
            operation_id,
            session_sequence,
            response,
        }
    }

    async fn execute_with_receipt(
        &self,
        envelope: RuntimeRequestEnvelope,
        now: DateTime<Utc>,
    ) -> RuntimeAgentResponse {
        if !is_mutating(&envelope.request) {
            return self.execute(envelope.request, now).await;
        }
        let digest = match request_digest(&envelope.request) {
            Ok(value) => value,
            Err(error) => return RuntimeAgentResponse::Error(error.protocol()),
        };
        match self.journal.begin(envelope.operation_id, digest) {
            Ok(JournalBegin::New) => {}
            Ok(JournalBegin::Completed(response)) => return response,
            Ok(JournalBegin::Pending) => {
                return RuntimeAgentResponse::Error(protocol_error(
                    RuntimeProtocolErrorCode::OperationPending,
                    "the mutating Runtime operation has an outcome pending reconciliation",
                    true,
                ));
            }
            Ok(JournalBegin::Conflict) => {
                return RuntimeAgentResponse::Error(protocol_error(
                    RuntimeProtocolErrorCode::OperationConflict,
                    "the operation ID was already used for a different request",
                    false,
                ));
            }
            Err(error) => return RuntimeAgentResponse::Error(error.protocol()),
        }
        let response = self.execute(envelope.request, now).await;
        if let Err(error) = self
            .journal
            .complete(envelope.operation_id, response.clone(), now)
        {
            return RuntimeAgentResponse::Error(error.protocol());
        }
        response
    }

    async fn execute(
        &self,
        request: RuntimeAgentRequest,
        now: DateTime<Utc>,
    ) -> RuntimeAgentResponse {
        let result = match request {
            RuntimeAgentRequest::Readiness => {
                self.readiness().await.map(RuntimeAgentResponse::Readiness)
            }
            RuntimeAgentRequest::ProcessStart(request) => self
                .start_process(request, now)
                .await
                .map(RuntimeAgentResponse::ProcessStarted),
            RuntimeAgentRequest::ProcessStdin(request) => self
                .process_stdin(request)
                .await
                .map(RuntimeAgentResponse::ProcessInputAccepted),
            RuntimeAgentRequest::ProcessSignal(request) => self
                .process_signal(request)
                .await
                .map(RuntimeAgentResponse::ProcessSignalAccepted),
            RuntimeAgentRequest::ProcessObserve(request) => self
                .process_observe(request)
                .await
                .map(RuntimeAgentResponse::ProcessObserved),
            RuntimeAgentRequest::File(request) => self
                .filesystem
                .execute(request)
                .await
                .map(RuntimeAgentResponse::File),
            RuntimeAgentRequest::ServiceOpen(request) => self
                .service_open(request, now)
                .await
                .map(RuntimeAgentResponse::ServiceOpened),
            RuntimeAgentRequest::ServiceWrite(request) => self
                .service_write(request)
                .await
                .map(RuntimeAgentResponse::ServiceWriteAccepted),
            RuntimeAgentRequest::ServiceClose(request) => self
                .service_close(request, ServiceCloseReason::Requested)
                .await
                .map(RuntimeAgentResponse::ServiceClosed),
            RuntimeAgentRequest::Activity => {
                Ok(RuntimeAgentResponse::Activity(self.activity(now).await))
            }
        };
        result.unwrap_or_else(|error| RuntimeAgentResponse::Error(error.protocol()))
    }

    async fn readiness(&self) -> Result<RuntimeReadiness, RuntimeAgentError> {
        let mut checks = Vec::with_capacity(7);
        checks.push(check(RuntimeReadinessComponent::RuntimeAgent, true));
        checks.push(check(
            RuntimeReadinessComponent::PersistentVolume,
            self.config.company_root.join(".seeded").is_file(),
        ));
        checks.push(check(
            RuntimeReadinessComponent::SessionScratch,
            self.config.company_root.join("run/sessions").is_dir(),
        ));
        let mut process_probe = company_security_probe_command()?;
        let process_ready = tokio::time::timeout(Duration::from_secs(2), async move {
            let command = &mut process_probe;
            command
                .env_clear()
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            command.status().await.map(|status| status.success())
        })
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or(false);
        checks.push(check(
            RuntimeReadinessComponent::ProcessExecution,
            process_ready,
        ));
        checks.push(check(
            RuntimeReadinessComponent::Desktop,
            loopback_probe(6080).await,
        ));
        checks.push(check(
            RuntimeReadinessComponent::BrowserBroker,
            loopback_probe(9223).await,
        ));
        checks.push(check(
            RuntimeReadinessComponent::ReleaseHealth,
            loopback_probe(7789).await,
        ));
        Ok(RuntimeReadiness {
            protocol: RUNTIME_AGENT_PROTOCOL.to_owned(),
            runtime_image: self.config.identity.runtime_image.clone(),
            source_revision: self.config.identity.source_revision.clone(),
            core_version: self.config.release.core_version.clone(),
            api_contract_version: self.config.release.api_contract_version.clone(),
            assertion_contract_version: self.config.release.assertion_contract_version.clone(),
            schema_version: self.config.release.schema_version.clone(),
            volume_name: self.config.identity.volume_name.clone(),
            runtime_id: self.config.identity.runtime_id.clone(),
            runtime_generation: self.config.identity.runtime_generation,
            desired_revision: self.config.desired_revision,
            ready: checks
                .iter()
                .all(|item| item.status == RuntimeCheckStatus::Ready),
            checks,
        })
    }

    async fn start_process(
        &self,
        request: ProcessStartRequest,
        now: DateTime<Utc>,
    ) -> Result<ProcessStarted, RuntimeAgentError> {
        validate_process_request(&request, &self.config.core_company)?;
        if self.processes.lock().await.len() >= MAX_ACTIVE_RESOURCES {
            return Err(RuntimeAgentError::LimitExceeded);
        }
        let mut processes = self.processes.lock().await;
        if processes.contains_key(&request.process_id) {
            return Err(RuntimeAgentError::Conflict);
        }
        #[cfg(all(target_os = "linux", not(test)))]
        let mut command = {
            let executable = std::env::current_exe().map_err(RuntimeAgentError::file)?;
            let mut command = Command::new(executable);
            command
                .arg("--process-worker")
                .arg(&request.working_directory.path)
                .arg(&request.executable)
                .arg("--")
                .args(&request.arguments);
            if effect_process_identity(&request.authority) {
                configure_effect_child(&mut command);
            } else {
                configure_company_child(&mut command);
            }
            command
        };
        #[cfg(any(not(target_os = "linux"), test))]
        let mut command = {
            let working_directory = self
                .filesystem
                .executable_path(&request.working_directory)
                .await?;
            let mut command = Command::new(&request.executable);
            command
                .args(&request.arguments)
                .current_dir(working_directory);
            command
        };
        let effect_identity = effect_process_identity(&request.authority);
        command
            .env_clear()
            .env(
                "HOME",
                if effect_identity {
                    "/tmp/restless-effect"
                } else {
                    "/company/home"
                },
            )
            .env("USER", if effect_identity { "effect" } else { "company" })
            .env(
                "LOGNAME",
                if effect_identity { "effect" } else { "company" },
            )
            .env("LANG", "C.UTF-8")
            .env("DISPLAY", ":1")
            .env("PATH", "/usr/local/bin:/usr/bin:/bin")
            .stdin(if request.stdin {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(false);
        command.process_group(0);
        for (name, value) in &request.environment {
            command.env(name, value.expose());
        }
        apply_authoritative_process_environment(&mut command, &request.authority);
        let mut child = command.spawn().map_err(RuntimeAgentError::process)?;
        let pid = child.id().ok_or(RuntimeAgentError::ProcessUnavailable)?;
        let stdin = child.stdin.take();
        let stdout = child
            .stdout
            .take()
            .ok_or(RuntimeAgentError::ProcessUnavailable)?;
        let stderr = child
            .stderr
            .take()
            .ok_or(RuntimeAgentError::ProcessUnavailable)?;
        let status = Arc::new(Mutex::new(ProcessStatus::Running));
        let record = Arc::new(ProcessRecord {
            process_id: request.process_id,
            pid,
            authority: request.authority.clone(),
            started_at: now,
            stdin: Mutex::new(stdin),
            status: Arc::clone(&status),
        });
        processes.insert(request.process_id, Arc::clone(&record));
        drop(processes);

        spawn_output_reader(
            request.process_id,
            ProcessStream::Stdout,
            stdout,
            self.events.clone(),
        );
        spawn_output_reader(
            request.process_id,
            ProcessStream::Stderr,
            stderr,
            self.events.clone(),
        );
        let events = self.events.clone();
        let process_id = request.process_id;
        let process_records = Arc::clone(&self.processes);
        tokio::spawn(async move {
            let observed = child.wait().await;
            let finished_at = Utc::now();
            let (exit_code, signal) = match observed {
                Ok(status) => (status.code(), exit_signal(&status)),
                Err(_) => (None, None),
            };
            *status.lock().await = ProcessStatus::Exited {
                finished_at,
                exit_code,
                signal,
            };
            let _ = events
                .send(RuntimeAgentEvent::ProcessExited(ProcessExited {
                    process_id,
                    exit_code,
                    signal,
                    finished_at,
                }))
                .await;
            tokio::time::sleep(EXITED_PROCESS_RETENTION).await;
            let mut records = process_records.lock().await;
            if records
                .get(&process_id)
                .is_some_and(|current| Arc::ptr_eq(current, &record))
            {
                records.remove(&process_id);
            }
        });
        Ok(ProcessStarted {
            process_id: request.process_id,
            pid,
            started_at: now,
        })
    }

    async fn process_stdin(
        &self,
        request: ProcessStdinRequest,
    ) -> Result<ProcessInputAccepted, RuntimeAgentError> {
        let bytes = decode_chunk(&request.data_base64)?;
        let record = self
            .processes
            .lock()
            .await
            .get(&request.process_id)
            .cloned()
            .ok_or(RuntimeAgentError::NotFound)?;
        if !matches!(*record.status.lock().await, ProcessStatus::Running) {
            return Err(RuntimeAgentError::ProcessUnavailable);
        }
        let mut stdin = record.stdin.lock().await;
        let writer = stdin
            .as_mut()
            .ok_or(RuntimeAgentError::ProcessUnavailable)?;
        writer
            .write_all(&bytes)
            .await
            .map_err(RuntimeAgentError::process)?;
        writer.flush().await.map_err(RuntimeAgentError::process)?;
        if request.eof {
            writer
                .shutdown()
                .await
                .map_err(RuntimeAgentError::process)?;
            stdin.take();
        }
        Ok(ProcessInputAccepted {
            process_id: request.process_id,
            decoded_bytes: bytes.len() as u32,
            eof: request.eof,
        })
    }

    async fn process_signal(
        &self,
        request: ProcessSignalRequest,
    ) -> Result<ProcessSignalAccepted, RuntimeAgentError> {
        let record = self
            .processes
            .lock()
            .await
            .get(&request.process_id)
            .cloned()
            .ok_or(RuntimeAgentError::NotFound)?;
        if !matches!(*record.status.lock().await, ProcessStatus::Running) {
            return Err(RuntimeAgentError::ProcessUnavailable);
        }
        let signal = match request.signal {
            GovernedSignal::Interrupt => Signal::SIGINT,
            GovernedSignal::Terminate => Signal::SIGTERM,
            GovernedSignal::Kill => Signal::SIGKILL,
        };
        let pid = i32::try_from(record.pid).map_err(|_| RuntimeAgentError::ProcessUnavailable)?;
        killpg(Pid::from_raw(pid), signal).map_err(|_| RuntimeAgentError::ProcessUnavailable)?;
        Ok(ProcessSignalAccepted {
            process_id: request.process_id,
            signal: request.signal,
        })
    }

    async fn process_observe(
        &self,
        request: ProcessObserveRequest,
    ) -> Result<ProcessObserved, RuntimeAgentError> {
        let record = self
            .processes
            .lock()
            .await
            .get(&request.process_id)
            .cloned()
            .ok_or(RuntimeAgentError::NotFound)?;
        let status = record.status.lock().await.clone();
        let (state, finished_at, exit_code, signal) = match status {
            ProcessStatus::Running => (ProcessState::Running, None, None, None),
            ProcessStatus::Exited {
                finished_at,
                exit_code,
                signal,
            } => (ProcessState::Exited, Some(finished_at), exit_code, signal),
        };
        Ok(ProcessObserved {
            process_id: record.process_id,
            pid: record.pid,
            state,
            started_at: record.started_at,
            finished_at,
            exit_code,
            signal,
        })
    }

    async fn service_open(
        &self,
        request: ServiceOpenRequest,
        now: DateTime<Utc>,
    ) -> Result<ServiceOpened, RuntimeAgentError> {
        if request.stream_id.is_nil() {
            return Err(RuntimeAgentError::InvalidRequest(
                "service stream ID must be non-nil",
            ));
        }
        let timeout_ms = if request.idle_timeout_ms == 0 {
            DEFAULT_SERVICE_IDLE_MS
        } else {
            request.idle_timeout_ms
        };
        if timeout_ms > MAX_SERVICE_IDLE_MS {
            return Err(RuntimeAgentError::LimitExceeded);
        }
        let port = self.service_port(&request.service)?;
        let mut services = self.services.lock().await;
        if services.len() >= MAX_ACTIVE_RESOURCES {
            return Err(RuntimeAgentError::LimitExceeded);
        }
        if services.contains_key(&request.stream_id) {
            return Err(RuntimeAgentError::Conflict);
        }
        let stream = tokio::time::timeout(
            Duration::from_secs(3),
            TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)),
        )
        .await
        .map_err(|_| RuntimeAgentError::ServiceUnavailable)?
        .map_err(|_| RuntimeAgentError::ServiceUnavailable)?;
        let (reader, writer) = stream.into_split();
        services.insert(
            request.stream_id,
            ServiceRecord {
                service: request.service.clone(),
                opened_at: now,
                writer: Arc::new(Mutex::new(Some(writer))),
            },
        );
        drop(services);
        spawn_service_reader(
            request.stream_id,
            reader,
            Duration::from_millis(u64::from(timeout_ms)),
            Arc::clone(&self.services),
            self.events.clone(),
        );
        Ok(ServiceOpened {
            stream_id: request.stream_id,
            service: request.service,
        })
    }

    async fn service_write(
        &self,
        request: ServiceWriteRequest,
    ) -> Result<ServiceWriteAccepted, RuntimeAgentError> {
        let bytes = decode_chunk(&request.data_base64)?;
        let writer = self
            .services
            .lock()
            .await
            .get(&request.stream_id)
            .map(|record| Arc::clone(&record.writer))
            .ok_or(RuntimeAgentError::NotFound)?;
        let mut guard = writer.lock().await;
        let stream = guard
            .as_mut()
            .ok_or(RuntimeAgentError::ServiceUnavailable)?;
        stream
            .write_all(&bytes)
            .await
            .map_err(|_| RuntimeAgentError::ServiceUnavailable)?;
        stream
            .flush()
            .await
            .map_err(|_| RuntimeAgentError::ServiceUnavailable)?;
        if request.eof {
            stream
                .shutdown()
                .await
                .map_err(|_| RuntimeAgentError::ServiceUnavailable)?;
            guard.take();
        }
        Ok(ServiceWriteAccepted {
            stream_id: request.stream_id,
            decoded_bytes: bytes.len() as u32,
            eof: request.eof,
        })
    }

    async fn service_close(
        &self,
        request: ServiceCloseRequest,
        reason: ServiceCloseReason,
    ) -> Result<ServiceClosed, RuntimeAgentError> {
        let record = self
            .services
            .lock()
            .await
            .remove(&request.stream_id)
            .ok_or(RuntimeAgentError::NotFound)?;
        if let Some(mut writer) = record.writer.lock().await.take() {
            let _ = writer.shutdown().await;
        }
        Ok(ServiceClosed {
            stream_id: request.stream_id,
            reason,
        })
    }

    fn service_port(&self, service: &RuntimeService) -> Result<u16, RuntimeAgentError> {
        match service {
            RuntimeService::Desktop => Ok(6080),
            RuntimeService::BrowserControl => Ok(9223),
            RuntimeService::ReleaseHealth => Ok(7789),
            RuntimeService::Published { port }
                if self.config.published_service_ports.contains(port)
                    && valid_published_port(*port) =>
            {
                Ok(*port)
            }
            RuntimeService::Published { .. } => Err(RuntimeAgentError::PermissionDenied),
        }
    }

    async fn activity(&self, observed_at: DateTime<Utc>) -> RuntimeActivity {
        let records = self.processes.lock().await;
        let accepts_processes = records.len() < MAX_ACTIVE_RESOURCES;
        let records = records.values().cloned().collect::<Vec<_>>();
        let mut processes = Vec::new();
        for record in records {
            if matches!(*record.status.lock().await, ProcessStatus::Running) {
                processes.push(ActiveProcess {
                    process_id: record.process_id,
                    pid: record.pid,
                    authority: record.authority.clone(),
                    started_at: record.started_at,
                });
            }
        }
        let services = self.services.lock().await;
        let accepts_services = services.len() < MAX_ACTIVE_RESOURCES;
        RuntimeActivity {
            observed_at,
            processes,
            service_streams: services
                .iter()
                .map(|(stream_id, record)| ActiveServiceStream {
                    stream_id: *stream_id,
                    service: record.service.clone(),
                    opened_at: record.opened_at,
                })
                .collect(),
            accepts_new_sessions: accepts_processes && accepts_services,
        }
    }
}

struct ProcessRecord {
    process_id: Uuid,
    pid: u32,
    authority: ProcessAuthority,
    started_at: DateTime<Utc>,
    stdin: Mutex<Option<ChildStdin>>,
    status: Arc<Mutex<ProcessStatus>>,
}

#[derive(Clone)]
enum ProcessStatus {
    Running,
    Exited {
        finished_at: DateTime<Utc>,
        exit_code: Option<i32>,
        signal: Option<i32>,
    },
}

struct ServiceRecord {
    service: RuntimeService,
    opened_at: DateTime<Utc>,
    writer: Arc<Mutex<Option<OwnedWriteHalf>>>,
}

fn spawn_output_reader(
    process_id: Uuid,
    stream: ProcessStream,
    mut reader: impl AsyncRead + Unpin + Send + 'static,
    events: mpsc::Sender<RuntimeAgentEvent>,
) {
    tokio::spawn(async move {
        let mut buffer = vec![0; PROCESS_OUTPUT_CHUNK];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => {
                    let _ = events
                        .send(RuntimeAgentEvent::ProcessOutput(ProcessOutput {
                            process_id,
                            stream,
                            data_base64: String::new(),
                            eof: true,
                        }))
                        .await;
                    break;
                }
                Ok(count) => {
                    if events
                        .send(RuntimeAgentEvent::ProcessOutput(ProcessOutput {
                            process_id,
                            stream,
                            data_base64: BASE64.encode(&buffer[..count]),
                            eof: false,
                        }))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
}

fn spawn_service_reader(
    stream_id: Uuid,
    mut reader: tokio::net::tcp::OwnedReadHalf,
    idle_timeout: Duration,
    services: Arc<Mutex<HashMap<Uuid, ServiceRecord>>>,
    events: mpsc::Sender<RuntimeAgentEvent>,
) {
    tokio::spawn(async move {
        let mut buffer = vec![0; PROCESS_OUTPUT_CHUNK];
        let reason = loop {
            match tokio::time::timeout(idle_timeout, reader.read(&mut buffer)).await {
                Ok(Ok(0)) => {
                    let _ = events
                        .send(RuntimeAgentEvent::ServiceOutput(ServiceOutput {
                            stream_id,
                            data_base64: String::new(),
                            eof: true,
                        }))
                        .await;
                    break ServiceCloseReason::RemoteClosed;
                }
                Ok(Ok(count)) => {
                    if events
                        .send(RuntimeAgentEvent::ServiceOutput(ServiceOutput {
                            stream_id,
                            data_base64: BASE64.encode(&buffer[..count]),
                            eof: false,
                        }))
                        .await
                        .is_err()
                    {
                        break ServiceCloseReason::TransportError;
                    }
                }
                Ok(Err(_)) => break ServiceCloseReason::TransportError,
                Err(_) => break ServiceCloseReason::IdleTimeout,
            }
        };
        if services.lock().await.remove(&stream_id).is_some() {
            let _ = events
                .send(RuntimeAgentEvent::ServiceClosed(ServiceClosed {
                    stream_id,
                    reason,
                }))
                .await;
        }
    });
}

#[derive(Debug)]
struct RuntimeFilesystem {
    #[cfg_attr(all(target_os = "linux", not(test)), allow(dead_code))]
    company_root: PathBuf,
    upload_state_root: PathBuf,
}

impl RuntimeFilesystem {
    fn new(company_root: PathBuf, upload_state_root: PathBuf) -> Result<Self, RuntimeAgentError> {
        if !company_root.is_absolute() || !company_root.is_dir() {
            return Err(RuntimeAgentError::InvalidConfiguration(
                "the persistent company root is unavailable",
            ));
        }
        fs::create_dir_all(&upload_state_root).map_err(RuntimeAgentError::file)?;
        fs::set_permissions(&upload_state_root, fs::Permissions::from_mode(0o700))
            .map_err(RuntimeAgentError::file)?;
        validate_private_directory(&upload_state_root, 0o700)?;
        Ok(Self {
            company_root,
            upload_state_root,
        })
    }

    async fn execute(&self, request: FileRequest) -> Result<FileResponse, RuntimeAgentError> {
        match request {
            FileRequest::UploadBegin {
                write_id,
                path,
                exact_size,
                exact_sha256,
                expected_sha256,
                mode,
            } => {
                let requested = UploadState {
                    write_id,
                    path,
                    exact_size,
                    exact_sha256,
                    expected_sha256,
                    mode,
                    temporary_name: format!(".runtime-upload-{write_id}.tmp"),
                    device: 0,
                    inode: 0,
                    change_time_seconds: 0,
                    change_time_nanoseconds: 0,
                };
                validate_upload_declaration(&requested)?;
                let state_path = upload_state_path(&self.upload_state_root, write_id);
                let state = if state_path.exists() {
                    let persisted = load_upload_state(&self.upload_state_root, write_id)?;
                    if !same_upload_declaration(&persisted, &requested) {
                        return Err(RuntimeAgentError::Conflict);
                    }
                    persisted
                } else {
                    requested
                };
                let prepared = match self
                    .execute_worker(FileWorkerRequest::UploadBegin(state.clone()))
                    .await?
                {
                    FileWorkerResponse::UploadPrepared(prepared) => prepared,
                    _ => return Err(RuntimeAgentError::Internal),
                };
                if !state_path.exists() {
                    persist_json(&state_path, &prepared.state)?;
                }
                Ok(FileResponse::UploadBegun(prepared.upload))
            }
            FileRequest::UploadChunk {
                write_id,
                offset,
                data_base64,
            } => {
                let state = load_upload_state(&self.upload_state_root, write_id)?;
                let advanced = match self
                    .execute_worker(FileWorkerRequest::UploadChunk {
                        state,
                        offset,
                        data_base64,
                    })
                    .await?
                {
                    FileWorkerResponse::UploadAdvanced(advanced) => advanced,
                    _ => return Err(RuntimeAgentError::Internal),
                };
                persist_json(
                    &upload_state_path(&self.upload_state_root, write_id),
                    &advanced.state,
                )?;
                Ok(FileResponse::UploadChunkAccepted(advanced.progress))
            }
            FileRequest::UploadCommit { write_id } => {
                let state = load_upload_state(&self.upload_state_root, write_id)?;
                let response = match self
                    .execute_worker(FileWorkerRequest::UploadCommit(state))
                    .await?
                {
                    FileWorkerResponse::File(response @ FileResponse::Written(_)) => response,
                    _ => return Err(RuntimeAgentError::Internal),
                };
                remove_upload_state(&self.upload_state_root, write_id)?;
                Ok(response)
            }
            FileRequest::UploadAbort { write_id } => {
                let state = load_upload_state(&self.upload_state_root, write_id)?;
                match self
                    .execute_worker(FileWorkerRequest::UploadAbort(state))
                    .await?
                {
                    FileWorkerResponse::File(FileResponse::UploadAborted {
                        write_id: observed,
                    }) if observed == write_id => {}
                    _ => return Err(RuntimeAgentError::Internal),
                }
                remove_upload_state(&self.upload_state_root, write_id)?;
                Ok(FileResponse::UploadAborted { write_id })
            }
            request => match self
                .execute_worker(FileWorkerRequest::Standard(request))
                .await?
            {
                FileWorkerResponse::File(response) => Ok(response),
                _ => Err(RuntimeAgentError::Internal),
            },
        }
    }

    async fn execute_worker(
        &self,
        request: FileWorkerRequest,
    ) -> Result<FileWorkerResponse, RuntimeAgentError> {
        #[cfg(all(target_os = "linux", not(test)))]
        {
            execute_file_worker_process(request).await
        }
        #[cfg(any(not(target_os = "linux"), test))]
        {
            let root = self.company_root.clone();
            tokio::task::spawn_blocking(move || execute_file_worker_request(&root, request))
                .await
                .map_err(|_| RuntimeAgentError::Internal)?
        }
    }

    #[cfg_attr(all(target_os = "linux", not(test)), allow(dead_code))]
    async fn executable_path(
        &self,
        path: &RuntimeWorkingDirectory,
    ) -> Result<PathBuf, RuntimeAgentError> {
        let root = self.company_root.clone();
        let path = path.clone();
        tokio::task::spawn_blocking(move || {
            let relative = validate_working_directory(&path)?;
            let dir = Dir::open_ambient_dir(&root, ambient_authority())
                .map_err(RuntimeAgentError::file)?;
            let metadata = dir.metadata(&relative).map_err(RuntimeAgentError::file)?;
            if !metadata.is_dir() {
                return Err(RuntimeAgentError::InvalidRequest(
                    "process working directory must be a directory",
                ));
            }
            // cap-std has already proved the canonical path remains beneath
            // this approved root. Return the ambient spelling only for
            // `Command::current_dir`, which cannot accept an open directory.
            let canonical = dir
                .canonicalize(&relative)
                .map_err(RuntimeAgentError::file)?;
            Ok(root.join(canonical))
        })
        .await
        .map_err(|_| RuntimeAgentError::Internal)?
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum FileWorkerRequest {
    Standard(FileRequest),
    UploadBegin(UploadState),
    UploadChunk {
        state: UploadState,
        offset: u64,
        data_base64: String,
    },
    UploadCommit(UploadState),
    UploadAbort(UploadState),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum FileWorkerResponse {
    File(FileResponse),
    UploadPrepared(PreparedUpload),
    UploadAdvanced(AdvancedUpload),
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum FileWorkerResult {
    Ok(FileWorkerResponse),
    Error(FileWorkerError),
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FileWorkerError {
    InvalidRequest,
    PermissionDenied,
    NotFound,
    Conflict,
    LimitExceeded,
    Internal,
}

#[cfg(target_os = "linux")]
async fn execute_file_worker_process(
    request: FileWorkerRequest,
) -> Result<FileWorkerResponse, RuntimeAgentError> {
    let encoded = serde_json::to_vec(&request).map_err(|_| RuntimeAgentError::Internal)?;
    if encoded.len() > RUNTIME_AGENT_MAX_FRAME_BYTES {
        return Err(RuntimeAgentError::LimitExceeded);
    }
    let executable = std::env::current_exe().map_err(RuntimeAgentError::file)?;
    let mut command = Command::new(executable);
    command
        .arg("--file-worker")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    configure_company_child(&mut command);
    let mut child = command.spawn().map_err(RuntimeAgentError::process)?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or(RuntimeAgentError::ProcessUnavailable)?;
    stdin
        .write_all(&encoded)
        .await
        .map_err(RuntimeAgentError::process)?;
    stdin.shutdown().await.map_err(RuntimeAgentError::process)?;
    drop(stdin);
    let output = tokio::time::timeout(Duration::from_secs(30), child.wait_with_output())
        .await
        .map_err(|_| RuntimeAgentError::ProcessUnavailable)?
        .map_err(RuntimeAgentError::process)?;
    if !output.status.success() || output.stdout.len() > RUNTIME_AGENT_MAX_FRAME_BYTES {
        return Err(RuntimeAgentError::Internal);
    }
    match serde_json::from_slice::<FileWorkerResult>(&output.stdout)
        .map_err(|_| RuntimeAgentError::Internal)?
    {
        FileWorkerResult::Ok(response) => Ok(response),
        FileWorkerResult::Error(error) => Err(error.into()),
    }
}

/// Internal company-UID entry point. It accepts exactly one bounded file
/// operation on stdin and returns exactly one bounded result on stdout. The
/// private capability and operation journal never enter this process.
#[cfg(target_os = "linux")]
pub fn run_file_worker_stdio() -> Result<(), RuntimeAgentError> {
    verify_company_worker_security()?;
    verify_private_state_is_inaccessible_to_company()?;
    let mut encoded = Vec::new();
    std::io::stdin()
        .take((RUNTIME_AGENT_MAX_FRAME_BYTES + 1) as u64)
        .read_to_end(&mut encoded)
        .map_err(RuntimeAgentError::file)?;
    if encoded.len() > RUNTIME_AGENT_MAX_FRAME_BYTES {
        return Err(RuntimeAgentError::LimitExceeded);
    }
    let request: FileWorkerRequest = serde_json::from_slice(&encoded).map_err(|_| {
        RuntimeAgentError::InvalidRequest("file worker request is not exact bounded JSON")
    })?;
    let result = match execute_file_worker_request(Path::new("/company"), request) {
        Ok(response) => FileWorkerResult::Ok(response),
        Err(error) => FileWorkerResult::Error((&error).into()),
    };
    let response = serde_json::to_vec(&result).map_err(|_| RuntimeAgentError::Internal)?;
    if response.len() > RUNTIME_AGENT_MAX_FRAME_BYTES {
        return Err(RuntimeAgentError::LimitExceeded);
    }
    std::io::stdout()
        .write_all(&response)
        .map_err(RuntimeAgentError::file)?;
    std::io::stdout().flush().map_err(RuntimeAgentError::file)
}

#[cfg(not(target_os = "linux"))]
pub fn run_file_worker_stdio() -> Result<(), RuntimeAgentError> {
    Err(RuntimeAgentError::InvalidConfiguration(
        "Runtime file worker is supported only on Linux",
    ))
}

#[cfg(target_os = "linux")]
impl From<&RuntimeAgentError> for FileWorkerError {
    fn from(error: &RuntimeAgentError) -> Self {
        match error {
            RuntimeAgentError::InvalidRequest(_) => Self::InvalidRequest,
            RuntimeAgentError::PermissionDenied => Self::PermissionDenied,
            RuntimeAgentError::NotFound => Self::NotFound,
            RuntimeAgentError::Conflict => Self::Conflict,
            RuntimeAgentError::LimitExceeded => Self::LimitExceeded,
            _ => Self::Internal,
        }
    }
}

#[cfg(target_os = "linux")]
impl From<FileWorkerError> for RuntimeAgentError {
    fn from(error: FileWorkerError) -> Self {
        match error {
            FileWorkerError::InvalidRequest => {
                Self::InvalidRequest("file worker rejected the bounded request")
            }
            FileWorkerError::PermissionDenied => Self::PermissionDenied,
            FileWorkerError::NotFound => Self::NotFound,
            FileWorkerError::Conflict => Self::Conflict,
            FileWorkerError::LimitExceeded => Self::LimitExceeded,
            FileWorkerError::Internal => Self::Internal,
        }
    }
}

fn execute_file_worker_request(
    company_root: &Path,
    request: FileWorkerRequest,
) -> Result<FileWorkerResponse, RuntimeAgentError> {
    match request {
        FileWorkerRequest::Standard(request) => {
            execute_standard_file_request(company_root, request).map(FileWorkerResponse::File)
        }
        FileWorkerRequest::UploadBegin(state) => {
            prepare_upload(company_root, state).map(FileWorkerResponse::UploadPrepared)
        }
        FileWorkerRequest::UploadChunk {
            state,
            offset,
            data_base64,
        } => write_upload_chunk(company_root, state, offset, data_base64)
            .map(FileWorkerResponse::UploadAdvanced),
        FileWorkerRequest::UploadCommit(state) => commit_upload(company_root, &state)
            .map(FileResponse::Written)
            .map(FileWorkerResponse::File),
        FileWorkerRequest::UploadAbort(state) => {
            abort_upload(company_root, &state)?;
            Ok(FileWorkerResponse::File(FileResponse::UploadAborted {
                write_id: state.write_id,
            }))
        }
    }
}

fn execute_standard_file_request(
    company_root: &Path,
    request: FileRequest,
) -> Result<FileResponse, RuntimeAgentError> {
    match request {
        FileRequest::Stat { path } => stat_file(company_root, path).map(FileResponse::Stat),
        FileRequest::List {
            path,
            cursor,
            limit,
        } => list_files(company_root, path, cursor, limit).map(FileResponse::List),
        FileRequest::Read {
            path,
            offset,
            max_bytes,
        } => read_file(company_root, path, offset, max_bytes).map(FileResponse::Read),
        FileRequest::AtomicWrite {
            path,
            data_base64,
            expected_sha256,
            mode,
        } => atomic_write(company_root, path, data_base64, expected_sha256, mode)
            .map(FileResponse::Written),
        FileRequest::UploadBegin { .. }
        | FileRequest::UploadChunk { .. }
        | FileRequest::UploadCommit { .. }
        | FileRequest::UploadAbort { .. } => Err(RuntimeAgentError::InvalidRequest(
            "upload control must remain in the credential-custody process",
        )),
        FileRequest::Rename {
            from,
            to,
            no_replace,
        } => rename_file(company_root, from, to, no_replace).map(FileResponse::Renamed),
        FileRequest::Digest { path } => digest_file(company_root, path).map(FileResponse::Digest),
    }
}

fn open_runtime_path(
    company_root: &Path,
    path: &RuntimePath,
) -> Result<(Dir, PathBuf), RuntimeAgentError> {
    let relative = validate_relative(&path.relative)?;
    let subdirectory = match path.root {
        RuntimeFileRoot::Org => "org",
        RuntimeFileRoot::Projects => "projects",
        RuntimeFileRoot::Knowledge => "knowledge",
        RuntimeFileRoot::Outputs => "outputs",
        RuntimeFileRoot::Repos => "repos",
        RuntimeFileRoot::Home => "home",
        RuntimeFileRoot::Downloads => "downloads",
        RuntimeFileRoot::SessionScratch => "run/sessions",
    };
    // Open the immutable mount point first, then descend through a capability.
    // Opening `/company/<root>` ambiently would let UID 2000 replace that
    // top-level directory with a symlink before the operation begins.
    let company = Dir::open_ambient_dir(company_root, ambient_authority())
        .map_err(RuntimeAgentError::file)?;
    let dir = company
        .open_dir(subdirectory)
        .map_err(RuntimeAgentError::file)?;
    Ok((dir, relative))
}

fn validate_relative(value: &str) -> Result<PathBuf, RuntimeAgentError> {
    if value.len() > 4096 || value.contains(['\0', '\r', '\n']) || Path::new(value).is_absolute() {
        return Err(RuntimeAgentError::InvalidRequest(
            "Runtime file path must be a bounded relative path",
        ));
    }
    let path = PathBuf::from(value);
    if !value.is_empty()
        && path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(RuntimeAgentError::PermissionDenied);
    }
    Ok(if value.is_empty() {
        PathBuf::from(".")
    } else {
        path
    })
}

fn stat_file(
    company_root: &Path,
    path: RuntimePath,
) -> Result<RuntimeFileMetadata, RuntimeAgentError> {
    let (dir, relative) = open_runtime_path(company_root, &path)?;
    let metadata = dir.metadata(relative).map_err(RuntimeAgentError::file)?;
    metadata_response(path, &metadata)
}

fn list_files(
    company_root: &Path,
    path: RuntimePath,
    cursor: Option<String>,
    limit: u16,
) -> Result<RuntimeFileList, RuntimeAgentError> {
    if limit == 0 || limit > MAX_DIRECTORY_ENTRIES {
        return Err(RuntimeAgentError::LimitExceeded);
    }
    if cursor
        .as_ref()
        .is_some_and(|value| value.len() > 255 || value.contains(['\0', '/', '\r', '\n']))
    {
        return Err(RuntimeAgentError::InvalidRequest(
            "directory cursor is invalid",
        ));
    }
    let (dir, relative) = open_runtime_path(company_root, &path)?;
    let mut entries = Vec::new();
    for candidate in dir.read_dir(relative).map_err(RuntimeAgentError::file)? {
        let candidate = candidate.map_err(RuntimeAgentError::file)?;
        let name = candidate.file_name().into_string().map_err(|_| {
            RuntimeAgentError::InvalidRequest("directory contains a non-UTF-8 name")
        })?;
        if cursor.as_ref().is_some_and(|cursor| name <= *cursor) {
            continue;
        }
        let metadata = candidate.metadata().map_err(RuntimeAgentError::file)?;
        entries.push(RuntimeFileListEntry {
            name,
            kind: metadata_kind(&metadata)?,
            size: metadata.len(),
            modified_at: modified_at(&metadata)?,
            mode: metadata.mode() & 0o777,
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    let has_more = entries.len() > usize::from(limit);
    entries.truncate(usize::from(limit));
    let next_cursor = has_more.then(|| entries.last().expect("nonzero page").name.clone());
    Ok(RuntimeFileList {
        path,
        entries,
        next_cursor,
    })
}

fn read_file(
    company_root: &Path,
    path: RuntimePath,
    offset: u64,
    max_bytes: u32,
) -> Result<RuntimeFileChunk, RuntimeAgentError> {
    if max_bytes == 0
        || usize::try_from(max_bytes).unwrap_or(usize::MAX) > RUNTIME_AGENT_MAX_CHUNK_BYTES
    {
        return Err(RuntimeAgentError::LimitExceeded);
    }
    let (dir, relative) = open_runtime_path(company_root, &path)?;
    let mut file = dir.open(relative).map_err(RuntimeAgentError::file)?;
    let metadata = file.metadata().map_err(RuntimeAgentError::file)?;
    if !metadata.is_file() {
        return Err(RuntimeAgentError::InvalidRequest(
            "Runtime read target must be a file",
        ));
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(RuntimeAgentError::file)?;
    let mut data = vec![0; max_bytes as usize];
    let count = file.read(&mut data).map_err(RuntimeAgentError::file)?;
    data.truncate(count);
    let eof = offset.saturating_add(count as u64) >= metadata.len();
    Ok(RuntimeFileChunk {
        path,
        offset,
        data_base64: BASE64.encode(data),
        eof,
    })
}

fn atomic_write(
    company_root: &Path,
    path: RuntimePath,
    data_base64: String,
    expected_sha256: Option<String>,
    mode: u32,
) -> Result<RuntimeFileMutation, RuntimeAgentError> {
    let data = decode_chunk(&data_base64)?;
    if expected_sha256
        .as_ref()
        .is_some_and(|value| !valid_sha256(value))
        || !valid_mode(mode)
    {
        return Err(RuntimeAgentError::InvalidRequest(
            "file digest or mode is invalid",
        ));
    }
    let (root, relative) = open_runtime_path(company_root, &path)?;
    let (parent, file_name) = open_parent(&root, &relative)?;
    verify_expected(&parent, file_name, expected_sha256.as_deref())?;
    let temporary = format!(".runtime-agent-{}.tmp", Uuid::new_v4());
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(mode);
    let mut file = parent
        .open_with(&temporary, &options)
        .map_err(RuntimeAgentError::file)?;
    let result: Result<(), RuntimeAgentError> = (|| {
        file.write_all(&data).map_err(RuntimeAgentError::file)?;
        file.sync_all().map_err(RuntimeAgentError::file)?;
        file.set_permissions(Permissions::from_mode(mode))
            .map_err(RuntimeAgentError::file)?;
        let opened = file.metadata().map_err(RuntimeAgentError::file)?;
        let named = parent
            .symlink_metadata(&temporary)
            .map_err(RuntimeAgentError::file)?;
        if !opened.is_file()
            || opened.uid() != geteuid().as_raw()
            || opened.mode() & 0o777 != mode
            || opened.nlink() != 1
            || opened.len() != data.len() as u64
            || named.file_type().is_symlink()
            || named.dev() != opened.dev()
            || named.ino() != opened.ino()
        {
            return Err(RuntimeAgentError::Conflict);
        }
        parent
            .rename(&temporary, &parent, file_name)
            .map_err(RuntimeAgentError::file)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = parent.remove_file(&temporary);
    }
    result?;
    Ok(RuntimeFileMutation {
        path,
        size: data.len() as u64,
        sha256: hex_digest(&data),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct UploadState {
    write_id: Uuid,
    path: RuntimePath,
    exact_size: u64,
    exact_sha256: String,
    expected_sha256: Option<String>,
    mode: u32,
    temporary_name: String,
    device: u64,
    inode: u64,
    change_time_seconds: i64,
    change_time_nanoseconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedUpload {
    state: UploadState,
    upload: RuntimeFileUpload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdvancedUpload {
    state: UploadState,
    progress: RuntimeFileUploadProgress,
}

fn validate_upload_declaration(state: &UploadState) -> Result<(), RuntimeAgentError> {
    if state.write_id.is_nil()
        || state.exact_size > RUNTIME_AGENT_MAX_UPLOAD_BYTES
        || !valid_sha256(&state.exact_sha256)
        || state
            .expected_sha256
            .as_ref()
            .is_some_and(|value| !valid_sha256(value))
        || !valid_mode(state.mode)
    {
        return Err(RuntimeAgentError::InvalidRequest(
            "atomic upload declaration is invalid",
        ));
    }
    Ok(())
}

fn same_upload_declaration(left: &UploadState, right: &UploadState) -> bool {
    left.write_id == right.write_id
        && left.path == right.path
        && left.exact_size == right.exact_size
        && left.exact_sha256 == right.exact_sha256
        && left.expected_sha256 == right.expected_sha256
        && left.mode == right.mode
        && left.temporary_name == right.temporary_name
}

fn prepare_upload(
    company_root: &Path,
    mut state: UploadState,
) -> Result<PreparedUpload, RuntimeAgentError> {
    validate_upload_declaration(&state)?;
    let (root, relative) = open_runtime_path(company_root, &state.path)?;
    let (parent, file_name) = open_parent(&root, &relative)?;
    verify_expected(&parent, file_name, state.expected_sha256.as_deref())?;
    let metadata = match parent.symlink_metadata(&state.temporary_name) {
        Ok(path_metadata) => {
            if path_metadata.file_type().is_symlink() || !path_metadata.is_file() {
                return Err(RuntimeAgentError::Conflict);
            }
            let file = parent
                .open(&state.temporary_name)
                .map_err(RuntimeAgentError::file)?;
            let metadata = file.metadata().map_err(RuntimeAgentError::file)?;
            if metadata.dev() != path_metadata.dev() || metadata.ino() != path_metadata.ino() {
                return Err(RuntimeAgentError::Conflict);
            }
            metadata
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if state.device != 0 || state.inode != 0 {
                return Err(RuntimeAgentError::Conflict);
            }
            let mut options = OpenOptions::new();
            options.write(true).create_new(true).mode(0o600);
            let file = parent
                .open_with(&state.temporary_name, &options)
                .map_err(RuntimeAgentError::file)?;
            file.sync_all().map_err(RuntimeAgentError::file)?;
            file.metadata().map_err(RuntimeAgentError::file)?
        }
        Err(error) => return Err(RuntimeAgentError::file(error)),
    };
    if state.device == 0 && state.inode == 0 {
        state.device = metadata.dev();
        state.inode = metadata.ino();
        state.change_time_seconds = metadata.ctime();
        state.change_time_nanoseconds = metadata.ctime_nsec();
    }
    validate_upload_temp(&metadata, &state, true)?;
    verify_upload_path_identity(&parent, &state, &metadata)?;
    let next_offset = metadata.len();
    Ok(PreparedUpload {
        upload: upload_response(&state, next_offset),
        state,
    })
}

fn write_upload_chunk(
    company_root: &Path,
    mut state: UploadState,
    offset: u64,
    data_base64: String,
) -> Result<AdvancedUpload, RuntimeAgentError> {
    let data = decode_chunk(&data_base64)?;
    let (root, relative) = open_runtime_path(company_root, &state.path)?;
    let (parent, _) = open_parent(&root, &relative)?;
    let mut options = OpenOptions::new();
    options.read(true).append(true);
    let mut file = parent
        .open_with(&state.temporary_name, &options)
        .map_err(RuntimeAgentError::file)?;
    let metadata = file.metadata().map_err(RuntimeAgentError::file)?;
    validate_upload_temp(&metadata, &state, true)?;
    if metadata.len() != offset || offset.saturating_add(data.len() as u64) > state.exact_size {
        return Err(RuntimeAgentError::Conflict);
    }
    file.write_all(&data).map_err(RuntimeAgentError::file)?;
    file.sync_data().map_err(RuntimeAgentError::file)?;
    let after = file.metadata().map_err(RuntimeAgentError::file)?;
    validate_upload_identity(&after, &state)?;
    if after.mode() & 0o777 != 0o600 || after.len() > state.exact_size {
        return Err(RuntimeAgentError::Conflict);
    }
    verify_upload_path_identity(&parent, &state, &after)?;
    state.change_time_seconds = after.ctime();
    state.change_time_nanoseconds = after.ctime_nsec();
    Ok(AdvancedUpload {
        progress: RuntimeFileUploadProgress {
            write_id: state.write_id,
            accepted_bytes: data.len() as u32,
            next_offset: offset + data.len() as u64,
        },
        state,
    })
}

fn commit_upload(
    company_root: &Path,
    state: &UploadState,
) -> Result<RuntimeFileMutation, RuntimeAgentError> {
    let (root, relative) = open_runtime_path(company_root, &state.path)?;
    let (parent, file_name) = open_parent(&root, &relative)?;
    match parent.open(&state.temporary_name) {
        Ok(mut file) => {
            let before = file.metadata().map_err(RuntimeAgentError::file)?;
            validate_upload_temp(&before, state, true)?;
            file.set_permissions(Permissions::from_mode(state.mode))
                .map_err(RuntimeAgentError::file)?;
            file.sync_all().map_err(RuntimeAgentError::file)?;
            // Hash the still-open descriptor only after its final mode and
            // bytes are durable, then compare the pathname to that same
            // descriptor immediately before the atomic rename.
            let (size, digest) = digest_file_handle(&mut file)?;
            if size != state.exact_size || digest != state.exact_sha256 {
                return Err(RuntimeAgentError::Conflict);
            }
            let final_metadata = file.metadata().map_err(RuntimeAgentError::file)?;
            if final_metadata.dev() != state.device
                || final_metadata.ino() != state.inode
                || final_metadata.uid() != geteuid().as_raw()
                || !final_metadata.is_file()
                || final_metadata.len() != state.exact_size
                || final_metadata.mode() & 0o777 != state.mode
                || final_metadata.nlink() != 1
            {
                return Err(RuntimeAgentError::Conflict);
            }
            verify_upload_path_identity(&parent, state, &final_metadata)?;
            parent
                .rename(&state.temporary_name, &parent, file_name)
                .map_err(RuntimeAgentError::file)?;
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // A crash can occur after the atomic rename and before the state
            // marker is removed. Reconcile the exact destination instead of
            // writing it a second time.
            let metadata = parent
                .metadata(file_name)
                .map_err(RuntimeAgentError::file)?;
            if metadata.dev() != state.device
                || metadata.ino() != state.inode
                || metadata.uid() != geteuid().as_raw()
                || metadata.mode() & 0o777 != state.mode
            {
                return Err(RuntimeAgentError::Conflict);
            }
            let (size, digest) = digest_opened(&parent, Path::new(file_name))?;
            if size != state.exact_size || digest != state.exact_sha256 {
                return Err(RuntimeAgentError::Conflict);
            }
        }
        Err(error) => return Err(RuntimeAgentError::file(error)),
    }
    Ok(RuntimeFileMutation {
        path: state.path.clone(),
        size: state.exact_size,
        sha256: state.exact_sha256.clone(),
    })
}

fn abort_upload(company_root: &Path, state: &UploadState) -> Result<(), RuntimeAgentError> {
    let (root, relative) = open_runtime_path(company_root, &state.path)?;
    let (parent, _) = open_parent(&root, &relative)?;
    match parent.symlink_metadata(&state.temporary_name) {
        Ok(metadata) => {
            validate_upload_temp(&metadata, state, true)?;
            parent
                .remove_file(&state.temporary_name)
                .map_err(RuntimeAgentError::file)?;
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(RuntimeAgentError::file(error)),
    }
    Ok(())
}

fn validate_upload_temp(
    metadata: &Metadata,
    state: &UploadState,
    exact_identity: bool,
) -> Result<(), RuntimeAgentError> {
    validate_upload_identity(metadata, state)?;
    if metadata.mode() & 0o777 != 0o600
        || metadata.len() > state.exact_size
        || (exact_identity
            && (metadata.dev() != state.device
                || metadata.ino() != state.inode
                || metadata.ctime() != state.change_time_seconds
                || metadata.ctime_nsec() != state.change_time_nanoseconds))
    {
        return Err(RuntimeAgentError::Conflict);
    }
    Ok(())
}

fn validate_upload_identity(
    metadata: &Metadata,
    state: &UploadState,
) -> Result<(), RuntimeAgentError> {
    if !metadata.is_file()
        || metadata.uid() != geteuid().as_raw()
        || metadata.nlink() != 1
        || metadata.dev() != state.device
        || metadata.ino() != state.inode
    {
        return Err(RuntimeAgentError::Conflict);
    }
    Ok(())
}

fn verify_upload_path_identity(
    parent: &Dir,
    state: &UploadState,
    opened: &Metadata,
) -> Result<(), RuntimeAgentError> {
    let path = parent
        .symlink_metadata(&state.temporary_name)
        .map_err(RuntimeAgentError::file)?;
    if path.dev() != opened.dev()
        || path.ino() != opened.ino()
        || path.file_type().is_symlink()
        || !path.is_file()
    {
        return Err(RuntimeAgentError::Conflict);
    }
    Ok(())
}

fn digest_file_handle(file: &mut cap_std::fs::File) -> Result<(u64, String), RuntimeAgentError> {
    file.seek(SeekFrom::Start(0))
        .map_err(RuntimeAgentError::file)?;
    let mut hasher = Sha256::new();
    let size = io::copy(file, &mut hasher).map_err(RuntimeAgentError::file)?;
    Ok((size, format!("{:x}", hasher.finalize())))
}

fn upload_response(state: &UploadState, next_offset: u64) -> RuntimeFileUpload {
    RuntimeFileUpload {
        write_id: state.write_id,
        path: state.path.clone(),
        exact_size: state.exact_size,
        exact_sha256: state.exact_sha256.clone(),
        mode: state.mode,
        next_offset,
    }
}

fn upload_state_path(root: &Path, write_id: Uuid) -> PathBuf {
    root.join(format!("{write_id}.json"))
}

fn load_upload_state(root: &Path, write_id: Uuid) -> Result<UploadState, RuntimeAgentError> {
    if write_id.is_nil() {
        return Err(RuntimeAgentError::InvalidRequest(
            "atomic upload ID must be non-nil",
        ));
    }
    let path = upload_state_path(root, write_id);
    validate_private_file(&path, 0o600, 32 * 1024)?;
    let bytes = fs::read(path).map_err(RuntimeAgentError::file)?;
    let state: UploadState =
        serde_json::from_slice(&bytes).map_err(|_| RuntimeAgentError::Internal)?;
    if state.write_id != write_id {
        return Err(RuntimeAgentError::Conflict);
    }
    Ok(state)
}

fn remove_upload_state(root: &Path, write_id: Uuid) -> Result<(), RuntimeAgentError> {
    fs::remove_file(upload_state_path(root, write_id)).map_err(RuntimeAgentError::file)?;
    sync_parent(root)?;
    Ok(())
}

fn rename_file(
    company_root: &Path,
    from: RuntimePath,
    to: RuntimePath,
    no_replace: bool,
) -> Result<RuntimeFileMutation, RuntimeAgentError> {
    let (source_root, source) = open_runtime_path(company_root, &from)?;
    let (destination_root, destination) = open_runtime_path(company_root, &to)?;
    let (source_parent, source_name) = open_parent(&source_root, &source)?;
    let (destination_parent, destination_name) = open_parent(&destination_root, &destination)?;
    if no_replace
        && destination_parent
            .symlink_metadata(destination_name)
            .is_ok()
    {
        return Err(RuntimeAgentError::Conflict);
    }
    source_parent
        .rename(source_name, &destination_parent, destination_name)
        .map_err(RuntimeAgentError::file)?;
    let (size, sha256) = digest_opened(&destination_parent, Path::new(destination_name))?;
    Ok(RuntimeFileMutation {
        path: to,
        size,
        sha256,
    })
}

fn digest_file(
    company_root: &Path,
    path: RuntimePath,
) -> Result<RuntimeFileDigest, RuntimeAgentError> {
    let (dir, relative) = open_runtime_path(company_root, &path)?;
    let (size, sha256) = digest_opened(&dir, &relative)?;
    Ok(RuntimeFileDigest { path, size, sha256 })
}

fn digest_opened(dir: &Dir, path: &Path) -> Result<(u64, String), RuntimeAgentError> {
    let mut file = dir.open(path).map_err(RuntimeAgentError::file)?;
    if !file.metadata().map_err(RuntimeAgentError::file)?.is_file() {
        return Err(RuntimeAgentError::InvalidRequest(
            "digest target must be a file",
        ));
    }
    let mut hasher = Sha256::new();
    let size = io::copy(&mut file, &mut hasher).map_err(RuntimeAgentError::file)?;
    Ok((size, format!("{:x}", hasher.finalize())))
}

fn open_parent<'a>(
    dir: &Dir,
    path: &'a Path,
) -> Result<(Dir, &'a std::ffi::OsStr), RuntimeAgentError> {
    if path == Path::new(".") {
        return Err(RuntimeAgentError::InvalidRequest(
            "cannot mutate an approved root",
        ));
    }
    let parent_path = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or(RuntimeAgentError::InvalidRequest("file path has no name"))?;
    Ok((
        dir.open_dir(parent_path).map_err(RuntimeAgentError::file)?,
        name,
    ))
}

fn verify_expected(
    parent: &Dir,
    name: &std::ffi::OsStr,
    expected: Option<&str>,
) -> Result<(), RuntimeAgentError> {
    if let Some(expected) = expected {
        let (_, current) = digest_opened(parent, Path::new(name))?;
        if current != expected {
            return Err(RuntimeAgentError::Conflict);
        }
    }
    Ok(())
}

fn metadata_response(
    path: RuntimePath,
    metadata: &Metadata,
) -> Result<RuntimeFileMetadata, RuntimeAgentError> {
    Ok(RuntimeFileMetadata {
        path,
        kind: metadata_kind(metadata)?,
        size: metadata.len(),
        modified_at: modified_at(metadata)?,
        mode: metadata.mode() & 0o777,
    })
}

fn metadata_kind(metadata: &Metadata) -> Result<RuntimeFileKind, RuntimeAgentError> {
    if metadata.is_file() {
        Ok(RuntimeFileKind::File)
    } else if metadata.is_dir() {
        Ok(RuntimeFileKind::Directory)
    } else {
        Err(RuntimeAgentError::PermissionDenied)
    }
}

fn modified_at(metadata: &Metadata) -> Result<DateTime<Utc>, RuntimeAgentError> {
    metadata
        .modified()
        .map(|value| DateTime::<Utc>::from(value.into_std()))
        .map_err(RuntimeAgentError::file)
}

#[derive(Debug, Serialize, Deserialize)]
struct OperationJournalState {
    version: u32,
    receipts: Vec<OperationReceipt>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationReceipt {
    operation_id: Uuid,
    request_sha256: String,
    state: ReceiptState,
    completed_at: Option<DateTime<Utc>>,
    response: Option<RuntimeAgentResponse>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReceiptState {
    Pending,
    Completed,
}

struct OperationJournal {
    path: PathBuf,
    state: StdMutex<OperationJournalState>,
}

enum JournalBegin {
    New,
    Completed(RuntimeAgentResponse),
    Pending,
    Conflict,
}

impl OperationJournal {
    fn open(path: PathBuf) -> Result<Self, RuntimeAgentError> {
        let state = if path.exists() {
            validate_private_file(&path, 0o600, 4 * 1024 * 1024)?;
            let bytes = fs::read(&path).map_err(RuntimeAgentError::file)?;
            serde_json::from_slice::<OperationJournalState>(&bytes).map_err(|_| {
                RuntimeAgentError::InvalidConfiguration("operation journal is corrupt")
            })?
        } else {
            OperationJournalState {
                version: 1,
                receipts: Vec::new(),
            }
        };
        if state.version != 1 || state.receipts.len() > MAX_OPERATION_RECEIPTS {
            return Err(RuntimeAgentError::InvalidConfiguration(
                "operation journal has an unsupported shape",
            ));
        }
        Ok(Self {
            path,
            state: StdMutex::new(state),
        })
    }

    fn begin(&self, operation_id: Uuid, digest: String) -> Result<JournalBegin, RuntimeAgentError> {
        let mut state = self.state.lock().map_err(|_| RuntimeAgentError::Internal)?;
        if let Some(existing) = state
            .receipts
            .iter()
            .find(|receipt| receipt.operation_id == operation_id)
        {
            if existing.request_sha256 != digest {
                return Ok(JournalBegin::Conflict);
            }
            return Ok(match existing.state {
                ReceiptState::Pending => JournalBegin::Pending,
                ReceiptState::Completed => JournalBegin::Completed(
                    existing
                        .response
                        .clone()
                        .ok_or(RuntimeAgentError::Internal)?,
                ),
            });
        }
        if operation_id.is_nil() {
            return Err(RuntimeAgentError::InvalidRequest(
                "operation ID must be non-nil",
            ));
        }
        if state.receipts.len() >= MAX_OPERATION_RECEIPTS {
            if let Some(index) = state
                .receipts
                .iter()
                .position(|receipt| receipt.state == ReceiptState::Completed)
            {
                state.receipts.remove(index);
            } else {
                return Err(RuntimeAgentError::LimitExceeded);
            }
        }
        state.receipts.push(OperationReceipt {
            operation_id,
            request_sha256: digest,
            state: ReceiptState::Pending,
            completed_at: None,
            response: None,
        });
        persist_json(&self.path, &*state)?;
        Ok(JournalBegin::New)
    }

    fn complete(
        &self,
        operation_id: Uuid,
        response: RuntimeAgentResponse,
        completed_at: DateTime<Utc>,
    ) -> Result<(), RuntimeAgentError> {
        let mut state = self.state.lock().map_err(|_| RuntimeAgentError::Internal)?;
        let receipt = state
            .receipts
            .iter_mut()
            .find(|receipt| receipt.operation_id == operation_id)
            .ok_or(RuntimeAgentError::Internal)?;
        receipt.state = ReceiptState::Completed;
        receipt.completed_at = Some(completed_at);
        receipt.response = Some(response);
        persist_json(&self.path, &*state)
    }
}

fn persist_json(path: &Path, value: &impl Serialize) -> Result<(), RuntimeAgentError> {
    let parent = path.parent().ok_or(RuntimeAgentError::Internal)?;
    fs::create_dir_all(parent).map_err(RuntimeAgentError::file)?;
    let temporary = parent.join(format!(".runtime-agent-{}.tmp", Uuid::new_v4()));
    let bytes = serde_json::to_vec(value).map_err(|_| RuntimeAgentError::Internal)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(RuntimeAgentError::LimitExceeded);
    }
    let mut file = StdOpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(RuntimeAgentError::file)?;
    let result = file
        .write_all(&bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600)))
        .and_then(|()| fs::rename(&temporary, path));
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(RuntimeAgentError::file(error));
    }
    sync_parent(parent)?;
    validate_private_file(path, 0o600, 4 * 1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::os::unix::fs::symlink;

    #[test]
    fn runtime_agent_installs_a_process_tls_provider() {
        install_runtime_agent_tls_crypto_provider();
        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }

    struct Fixture {
        root: PathBuf,
        company: PathBuf,
        control: PathBuf,
        config: RuntimeAgentConfig,
    }

    impl Fixture {
        fn new(published_service_ports: BTreeSet<u16>) -> Self {
            let root =
                std::env::temp_dir().join(format!("restless-runtime-agent-{}", Uuid::new_v4()));
            let company = root.join("company");
            let control = root.join("control");
            for path in [
                company.join("org"),
                company.join("projects"),
                company.join("knowledge"),
                company.join("outputs"),
                company.join("repos"),
                company.join("home"),
                company.join("downloads"),
                company.join("run/sessions"),
                company.join("reviews/one"),
                control.clone(),
            ] {
                fs::create_dir_all(path).unwrap();
            }
            fs::set_permissions(&control, fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(company.join(".seeded"), b"").unwrap();
            let owner_id = Uuid::new_v4();
            let plane_id = Uuid::new_v4();
            let company_id = Uuid::new_v4();
            let cell_id = Uuid::new_v4();
            let config = RuntimeAgentConfig::from_values(RuntimeAgentConfigValues {
                bridge_url: "wss://plane.example/internal/v1/runtime-bridge".into(),
                owner_id: owner_id.to_string(),
                plane_id: plane_id.to_string(),
                company_id: company_id.to_string(),
                cell_id: cell_id.to_string(),
                core_company: format!("c{}", company_id.simple()),
                runtime_id: format!("restless-cell-{cell_id}"),
                runtime_generation: "2".into(),
                desired_revision: "3".into(),
                runtime_image: format!("registry.example/restless@sha256:{}", "a".repeat(64)),
                volume_name: format!("restless-cell-{cell_id}-data"),
                source_revision: "b".repeat(40),
                capability_file: root.join("bootstrap"),
                capability_state_file: control.join("runtime-bridge-capability"),
                company_root: company.clone(),
                release: RuntimeReleaseIdentity {
                    core_version: "1.2.3".into(),
                    api_contract_version: "1".into(),
                    assertion_contract_version: "1".into(),
                    schema_version: "33".into(),
                },
                published_service_ports,
            })
            .unwrap();
            Self {
                root,
                company,
                control,
                config,
            }
        }

        fn path(root: RuntimeFileRoot, relative: &str) -> RuntimePath {
            RuntimePath {
                root,
                relative: relative.into(),
            }
        }

        fn request(
            sequence: u64,
            operation_id: Uuid,
            request: RuntimeAgentRequest,
        ) -> RuntimeRequestEnvelope {
            RuntimeRequestEnvelope {
                operation_id,
                deadline: Utc::now() + chrono::Duration::minutes(1),
                session_sequence: sequence,
                request,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn assert_error_code(response: RuntimeAgentResponse, code: RuntimeProtocolErrorCode) {
        match response {
            RuntimeAgentResponse::Error(error) => assert_eq!(error.code, code),
            other => panic!("expected protocol error, got {other:?}"),
        }
    }

    #[test]
    fn exact_identity_and_working_directory_contracts_are_enforced() {
        let fixture = Fixture::new(BTreeSet::new());
        assert_eq!(fixture.config.identity.runtime_generation, 2);
        assert_eq!(
            validate_working_directory(&RuntimeWorkingDirectory {
                path: "/company".into(),
            })
            .unwrap(),
            PathBuf::from(".")
        );
        assert_eq!(
            validate_working_directory(&RuntimeWorkingDirectory {
                path: "/company/reviews/one".into(),
            })
            .unwrap(),
            PathBuf::from("reviews/one")
        );
        for invalid in [
            "/companyish",
            "/company/../etc",
            "/company/reviews//one",
            "/tmp",
        ] {
            assert!(validate_working_directory(&RuntimeWorkingDirectory {
                path: invalid.into(),
            })
            .is_err());
        }

        let mut values = RuntimeAgentConfigValues {
            bridge_url: fixture.config.bridge_url.to_string(),
            owner_id: fixture.config.identity.owner_id.to_string(),
            plane_id: fixture.config.identity.plane_id.to_string(),
            company_id: fixture.config.identity.company_id.to_string(),
            cell_id: fixture.config.identity.cell_id.to_string(),
            core_company: fixture.config.core_company.clone(),
            runtime_id: fixture.config.identity.runtime_id.clone(),
            runtime_generation: "2".into(),
            desired_revision: "3".into(),
            runtime_image: fixture.config.identity.runtime_image.clone(),
            volume_name: fixture.config.identity.volume_name.clone(),
            source_revision: fixture.config.identity.source_revision.clone(),
            capability_file: fixture.config.capability_file.clone(),
            capability_state_file: fixture.config.capability_state_file.clone(),
            company_root: fixture.company.clone(),
            release: fixture.config.release.clone(),
            published_service_ports: BTreeSet::new(),
        };
        values.runtime_image = "restless:latest".into();
        assert!(RuntimeAgentConfig::from_values(values).is_err());
    }

    #[test]
    fn capability_rotation_is_private_atomic_and_bootstrap_first() {
        let fixture = Fixture::new(BTreeSet::new());
        let bootstrap = RuntimeBridgeCapability::new("b".repeat(96)).unwrap();
        let persisted = RuntimeBridgeCapability::new("p".repeat(96)).unwrap();
        fs::write(&fixture.config.capability_file, bootstrap.expose()).unwrap();
        fs::set_permissions(
            &fixture.config.capability_file,
            fs::Permissions::from_mode(0o400),
        )
        .unwrap();
        fs::write(&fixture.config.capability_state_file, persisted.expose()).unwrap();
        fs::set_permissions(
            &fixture.config.capability_state_file,
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let store = RuntimeCapabilityStore::new(
            fixture.config.capability_file.clone(),
            fixture.config.capability_state_file.clone(),
        );
        let candidates = store.candidates().unwrap();
        assert_eq!(candidates[0], bootstrap);
        assert_eq!(candidates[1], persisted);

        let rotated = RuntimeBridgeCapability::new("r".repeat(96)).unwrap();
        store.persist_rotation(&rotated).unwrap();
        assert_eq!(
            fs::metadata(&fixture.config.capability_state_file)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            fs::read_to_string(&fixture.config.capability_state_file).unwrap(),
            rotated.expose()
        );
        store.discard_bootstrap().unwrap();
        assert!(!fixture.config.capability_file.exists());
        assert_eq!(store.candidates().unwrap(), vec![rotated]);
    }

    #[tokio::test]
    async fn bounded_files_report_exact_metadata_and_block_symlink_escape() {
        let fixture = Fixture::new(BTreeSet::new());
        let filesystem =
            RuntimeFilesystem::new(fixture.company.clone(), fixture.control.join("uploads"))
                .unwrap();
        let path = Fixture::path(RuntimeFileRoot::Projects, "artifact.txt");
        let data = b"runtime file evidence";
        let written = filesystem
            .execute(FileRequest::AtomicWrite {
                path: path.clone(),
                data_base64: BASE64.encode(data),
                expected_sha256: None,
                mode: 0o640,
            })
            .await
            .unwrap();
        assert!(matches!(written, FileResponse::Written(_)));

        let metadata = filesystem
            .execute(FileRequest::Stat { path: path.clone() })
            .await
            .unwrap();
        match metadata {
            FileResponse::Stat(metadata) => {
                assert_eq!(metadata.mode, 0o640);
                assert_eq!(metadata.size, data.len() as u64);
                assert!(metadata.modified_at <= Utc::now());
            }
            other => panic!("unexpected response: {other:?}"),
        }
        let read = filesystem
            .execute(FileRequest::Read {
                path: path.clone(),
                offset: 0,
                max_bytes: 1024,
            })
            .await
            .unwrap();
        match read {
            FileResponse::Read(chunk) => {
                assert_eq!(BASE64.decode(chunk.data_base64).unwrap(), data);
                assert!(chunk.eof);
            }
            other => panic!("unexpected response: {other:?}"),
        }
        let digest = filesystem
            .execute(FileRequest::Digest { path })
            .await
            .unwrap();
        match digest {
            FileResponse::Digest(digest) => assert_eq!(digest.sha256, hex_digest(data)),
            other => panic!("unexpected response: {other:?}"),
        }

        let outside = fixture.root.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("secret"), b"outside").unwrap();
        symlink(&outside, fixture.company.join("projects/escape")).unwrap();
        assert!(filesystem
            .execute(FileRequest::Read {
                path: Fixture::path(RuntimeFileRoot::Projects, "escape/secret"),
                offset: 0,
                max_bytes: 100,
            })
            .await
            .is_err());
        assert_eq!(fs::read(outside.join("secret")).unwrap(), b"outside");

        // The selected top-level root is also opened beneath the `/company`
        // capability; replacing `projects` itself cannot redirect the API.
        fs::remove_dir_all(fixture.company.join("projects")).unwrap();
        symlink(&outside, fixture.company.join("projects")).unwrap();
        assert!(filesystem
            .execute(FileRequest::Read {
                path: Fixture::path(RuntimeFileRoot::Projects, "secret"),
                offset: 0,
                max_bytes: 100,
            })
            .await
            .is_err());
        assert!(validate_relative("../outside").is_err());
    }

    #[tokio::test]
    async fn multipart_upload_is_monotonic_hash_verified_atomic_and_substitution_safe() {
        let fixture = Fixture::new(BTreeSet::new());
        let filesystem =
            RuntimeFilesystem::new(fixture.company.clone(), fixture.control.join("uploads"))
                .unwrap();
        let bytes = (0..(RUNTIME_AGENT_MAX_CHUNK_BYTES * 2 + 31))
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let path = Fixture::path(RuntimeFileRoot::Projects, "large.bin");
        let write_id = Uuid::new_v4();
        filesystem
            .execute(FileRequest::UploadBegin {
                write_id,
                path: path.clone(),
                exact_size: bytes.len() as u64,
                exact_sha256: hex_digest(&bytes),
                expected_sha256: None,
                mode: 0o600,
            })
            .await
            .unwrap();
        assert!(fixture
            .control
            .join(format!("uploads/{write_id}.json"))
            .is_file());
        assert!(matches!(
            filesystem
                .execute(FileRequest::UploadChunk {
                    write_id,
                    offset: 1,
                    data_base64: BASE64.encode(&bytes[..10]),
                })
                .await,
            Err(RuntimeAgentError::Conflict)
        ));
        let mut offset = 0_u64;
        for chunk in bytes.chunks(RUNTIME_AGENT_MAX_CHUNK_BYTES) {
            filesystem
                .execute(FileRequest::UploadChunk {
                    write_id,
                    offset,
                    data_base64: BASE64.encode(chunk),
                })
                .await
                .unwrap();
            offset += chunk.len() as u64;
        }
        filesystem
            .execute(FileRequest::UploadCommit { write_id })
            .await
            .unwrap();
        assert_eq!(
            fs::read(fixture.company.join("projects/large.bin")).unwrap(),
            bytes
        );
        assert!(!fixture
            .control
            .join(format!("uploads/{write_id}.json"))
            .exists());

        let swapped_id = Uuid::new_v4();
        let expected = b"protected";
        filesystem
            .execute(FileRequest::UploadBegin {
                write_id: swapped_id,
                path: Fixture::path(RuntimeFileRoot::Projects, "swapped.bin"),
                exact_size: expected.len() as u64,
                exact_sha256: hex_digest(expected),
                expected_sha256: None,
                mode: 0o600,
            })
            .await
            .unwrap();
        let temporary = fixture
            .company
            .join(format!("projects/.runtime-upload-{swapped_id}.tmp"));
        fs::remove_file(&temporary).unwrap();
        fs::write(&temporary, expected).unwrap();
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(matches!(
            filesystem
                .execute(FileRequest::UploadCommit {
                    write_id: swapped_id
                })
                .await,
            Err(RuntimeAgentError::Conflict)
        ));
        assert!(!fixture.company.join("projects/swapped.bin").exists());
    }

    #[tokio::test]
    async fn governed_process_streams_input_output_and_durable_receipts() {
        let fixture = Fixture::new(BTreeSet::new());
        let (agent, mut events) = RuntimeAgent::new(fixture.config.clone()).unwrap();
        let mut sequence = RuntimeRequestSequence::new(1).unwrap();
        let process_id = Uuid::new_v4();
        let start_operation = Uuid::new_v4();
        let start = RuntimeAgentRequest::ProcessStart(ProcessStartRequest {
            process_id,
            authority: ProcessAuthority::AuthorityEvent {
                company: fixture.config.core_company.clone(),
                actor: "exec".into(),
                responsibility: "portfolio".into(),
                event_id: 42,
                session_id: "session-1".into(),
            },
            executable: "/bin/sh".into(),
            arguments: vec![
                "-c".into(),
                "IFS= read -r line; printf 'out:%s' \"$line\"; printf 'err' >&2".into(),
            ],
            working_directory: RuntimeWorkingDirectory {
                path: "/company".into(),
            },
            environment: BTreeMap::new(),
            stdin: true,
        });
        let started = agent
            .handle_request(
                &mut sequence,
                Fixture::request(1, start_operation, start.clone()),
                Utc::now(),
            )
            .await;
        assert!(matches!(
            started.response,
            RuntimeAgentResponse::ProcessStarted(_)
        ));

        let replay = agent
            .handle_request(
                &mut sequence,
                Fixture::request(2, start_operation, start),
                Utc::now(),
            )
            .await;
        assert!(matches!(
            replay.response,
            RuntimeAgentResponse::ProcessStarted(_)
        ));
        let input = agent
            .handle_request(
                &mut sequence,
                Fixture::request(
                    3,
                    Uuid::new_v4(),
                    RuntimeAgentRequest::ProcessStdin(ProcessStdinRequest {
                        process_id,
                        data_base64: BASE64.encode(b"hello\n"),
                        eof: true,
                    }),
                ),
                Utc::now(),
            )
            .await;
        assert!(matches!(
            input.response,
            RuntimeAgentResponse::ProcessInputAccepted(_)
        ));

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exited = false;
        while !exited || stdout != b"out:hello" || stderr != b"err" {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            match event {
                RuntimeAgentEvent::ProcessOutput(output) if !output.eof => {
                    let decoded = BASE64.decode(output.data_base64).unwrap();
                    match output.stream {
                        ProcessStream::Stdout => stdout.extend(decoded),
                        ProcessStream::Stderr => stderr.extend(decoded),
                    }
                }
                RuntimeAgentEvent::ProcessExited(exit) if exit.process_id == process_id => {
                    assert_eq!(exit.exit_code, Some(0));
                    exited = true;
                }
                _ => {}
            }
        }
        let observed = agent
            .handle_request(
                &mut sequence,
                Fixture::request(
                    4,
                    Uuid::new_v4(),
                    RuntimeAgentRequest::ProcessObserve(ProcessObserveRequest { process_id }),
                ),
                Utc::now(),
            )
            .await;
        assert!(matches!(
            observed.response,
            RuntimeAgentResponse::ProcessObserved(ProcessObserved {
                state: ProcessState::Exited,
                exit_code: Some(0),
                ..
            })
        ));
        assert_eq!(stdout, b"out:hello");
        assert_eq!(stderr, b"err");
        assert_eq!(
            fs::metadata(&fixture.config.operation_journal_file)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[tokio::test]
    async fn published_service_is_exactly_allowlisted_and_streamed() {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let fixture = Fixture::new(BTreeSet::from([port]));
        let (agent, mut events) = RuntimeAgent::new(fixture.config.clone()).unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).await.unwrap();
            stream.write_all(&bytes).await.unwrap();
        });
        let stream_id = Uuid::new_v4();
        let mut sequence = RuntimeRequestSequence::new(1).unwrap();
        let opened = agent
            .handle_request(
                &mut sequence,
                Fixture::request(
                    1,
                    Uuid::new_v4(),
                    RuntimeAgentRequest::ServiceOpen(ServiceOpenRequest {
                        stream_id,
                        service: RuntimeService::Published { port },
                        idle_timeout_ms: 5_000,
                    }),
                ),
                Utc::now(),
            )
            .await;
        assert!(matches!(
            opened.response,
            RuntimeAgentResponse::ServiceOpened(_)
        ));
        let written = agent
            .handle_request(
                &mut sequence,
                Fixture::request(
                    2,
                    Uuid::new_v4(),
                    RuntimeAgentRequest::ServiceWrite(ServiceWriteRequest {
                        stream_id,
                        data_base64: BASE64.encode(b"loopback"),
                        eof: true,
                    }),
                ),
                Utc::now(),
            )
            .await;
        assert!(matches!(
            written.response,
            RuntimeAgentResponse::ServiceWriteAccepted(_)
        ));
        let mut observed = Vec::new();
        while observed != b"loopback" {
            let event = tokio::time::timeout(Duration::from_secs(5), events.recv())
                .await
                .unwrap()
                .unwrap();
            if let RuntimeAgentEvent::ServiceOutput(output) = event {
                if !output.eof {
                    observed.extend(BASE64.decode(output.data_base64).unwrap());
                }
            }
        }
        assert_eq!(observed, b"loopback");

        let denied = agent
            .handle_request(
                &mut sequence,
                Fixture::request(
                    3,
                    Uuid::new_v4(),
                    RuntimeAgentRequest::ServiceOpen(ServiceOpenRequest {
                        stream_id: Uuid::new_v4(),
                        service: RuntimeService::Published { port: port + 1 },
                        idle_timeout_ms: 1_000,
                    }),
                ),
                Utc::now(),
            )
            .await;
        assert_error_code(denied.response, RuntimeProtocolErrorCode::PermissionDenied);
    }
}
