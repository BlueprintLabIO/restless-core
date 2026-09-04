//! Self-hosted implementation of the company-computer transport.
//!
//! The local appliance still uses Docker as its mature lifecycle and process
//! boundary.  All product code above this module speaks [`RuntimeTransport`],
//! while every Docker process below it is guarded so a network account plane
//! can never silently fall back to the host Docker daemon.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io,
    pin::Pin,
    process::{ExitStatus, Output, Stdio},
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf},
    process::{Child, Command},
    sync::Notify,
};
use uuid::Uuid;

use crate::{
    hosted_runtime,
    runtime_transport::{
        validate_company, CompanyPath, RuntimeActiveProcess, RuntimeActivity,
        RuntimeComponentCheck, RuntimeComponentStatus, RuntimeDirectoryEntry, RuntimeDuplex,
        RuntimeFileKind, RuntimeFileMetadata, RuntimeProcess, RuntimeProcessAuthority,
        RuntimeProcessControl, RuntimeProcessExit, RuntimeProcessSpec, RuntimeService,
        RuntimeSignal, RuntimeTransport, RuntimeTransportError,
    },
};

const RESOURCE_NAMESPACE_ENV: &str = "RESTLESS_RESOURCE_NAMESPACE";
const FILE_INPUT_LIMIT: usize = 16 * 1024 * 1024;
const FILE_OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
const LIST_ENTRY_LIMIT: usize = 4_096;
const LIST_OUTPUT_LIMIT: usize = 2 * 1024 * 1024;
const PROCESS_HANDSHAKE_ATTEMPTS: usize = 80;
const PROCESS_HANDSHAKE_INTERVAL: Duration = Duration::from_millis(25);
const MAX_SERVICE_IDLE: Duration = Duration::from_secs(60 * 60);

const EXIT_NOT_FOUND: i32 = 40;
const EXIT_CONFLICT: i32 = 41;
const EXIT_LIMIT: i32 = 42;
const EXIT_INVALID: i32 = 43;

const PROCESS_ENVIRONMENT_WRITER: &str = r#"
import os
import sys

path = sys.argv[1]
parent = os.path.dirname(path)
os.makedirs(parent, mode=0o700, exist_ok=True)
os.chmod(parent, 0o700)
temporary = path + ".incoming"
try:
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "wb") as output:
        while True:
            block = sys.stdin.buffer.read(65536)
            if not block:
                break
            output.write(block)
        output.flush()
        os.fsync(output.fileno())
    os.replace(temporary, path)
finally:
    try:
        os.unlink(temporary)
    except FileNotFoundError:
        pass
"#;

const PROCESS_WRAPPER: &str = r#"
import json
import os
import sys

environment_path, pid_path, executable = sys.argv[1:4]
with open(environment_path, "rb") as source:
    supplied = json.load(source)
os.unlink(environment_path)
if not isinstance(supplied, dict) or not all(isinstance(key, str) and isinstance(value, str) for key, value in supplied.items()):
    raise SystemExit(43)
environment = os.environ.copy()
environment.update(supplied)
temporary = pid_path + ".incoming"
descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
with os.fdopen(descriptor, "w", encoding="ascii") as output:
    output.write(str(os.getpid()))
    output.write("\n")
    output.flush()
    os.fsync(output.fileno())
os.replace(temporary, pid_path)
os.execvpe(executable, [executable, *sys.argv[4:]], environment)
"#;

const PROCESS_SIGNAL: &str = r#"
import os
import signal
import sys

with open(sys.argv[1], "r", encoding="ascii") as source:
    raw = source.read(32).strip()
if not raw.isascii() or not raw.isdecimal():
    raise SystemExit(43)
pid = int(raw)
if pid < 2:
    raise SystemExit(43)
try:
    os.killpg(pid, int(sys.argv[2]))
except ProcessLookupError:
    raise SystemExit(40)
except PermissionError:
    raise SystemExit(43)
"#;

const FILE_STAT: &str = r#"
import json
import os
import stat
import sys

root = os.path.realpath("/company")
path = sys.argv[1]
try:
    resolved = os.path.realpath(path, strict=True)
except FileNotFoundError:
    raise SystemExit(40)
if os.path.commonpath((root, resolved)) != root:
    raise SystemExit(43)
value = os.stat(resolved, follow_symlinks=True)
if stat.S_ISREG(value.st_mode):
    kind = "file"
elif stat.S_ISDIR(value.st_mode):
    kind = "directory"
else:
    raise SystemExit(43)
print(json.dumps({
    "kind": kind,
    "size": value.st_size,
    "modified_ns": value.st_mtime_ns,
    "mode": stat.S_IMODE(value.st_mode),
}, separators=(",", ":")))
"#;

const FILE_LIST: &str = r#"
import json
import os
import stat
import sys

root = os.path.realpath("/company")
path = sys.argv[1]
limit = int(sys.argv[2])
try:
    resolved = os.path.realpath(path, strict=True)
except FileNotFoundError:
    raise SystemExit(40)
if os.path.commonpath((root, resolved)) != root or not os.path.isdir(resolved):
    raise SystemExit(43)
names = os.listdir(resolved)
if len(names) > limit:
    raise SystemExit(42)
entries = []
for name in sorted(names):
    try:
        name.encode("utf-8", "strict")
        child = os.path.realpath(os.path.join(resolved, name), strict=True)
    except (UnicodeError, FileNotFoundError):
        raise SystemExit(43)
    if os.path.commonpath((root, child)) != root:
        raise SystemExit(43)
    value = os.stat(child, follow_symlinks=True)
    if stat.S_ISREG(value.st_mode):
        kind = "file"
    elif stat.S_ISDIR(value.st_mode):
        kind = "directory"
    else:
        raise SystemExit(43)
    entries.append({
        "name": name,
        "kind": kind,
        "size": value.st_size,
        "modified_ns": value.st_mtime_ns,
        "mode": stat.S_IMODE(value.st_mode),
    })
print(json.dumps(entries, separators=(",", ":"), ensure_ascii=False))
"#;

const FILE_READ: &str = r#"
import os
import stat
import sys

root = os.path.realpath("/company")
path = sys.argv[1]
limit = int(sys.argv[2])
try:
    resolved = os.path.realpath(path, strict=True)
except FileNotFoundError:
    raise SystemExit(40)
if os.path.commonpath((root, resolved)) != root:
    raise SystemExit(43)
value = os.stat(resolved, follow_symlinks=True)
if not stat.S_ISREG(value.st_mode):
    raise SystemExit(43)
with open(resolved, "rb", buffering=0) as source:
    body = source.read(limit + 1)
if len(body) > limit:
    raise SystemExit(42)
sys.stdout.buffer.write(body)
"#;

const FILE_ATOMIC_WRITE: &str = r#"
import os
import sys

root = os.path.realpath("/company")
path, operation, raw_mode, raw_limit = sys.argv[1:5]
parent = os.path.realpath(os.path.dirname(path), strict=True)
if os.path.commonpath((root, parent)) != root:
    raise SystemExit(43)
if os.path.isdir(path):
    raise SystemExit(41)
mode = int(raw_mode)
limit = int(raw_limit)
temporary = os.path.join(parent, ".restless-write-" + operation)
try:
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
except FileExistsError:
    raise SystemExit(41)
try:
    total = 0
    with os.fdopen(descriptor, "wb") as output:
        while True:
            block = sys.stdin.buffer.read(65536)
            if not block:
                break
            total += len(block)
            if total > limit:
                raise SystemExit(42)
            output.write(block)
        output.flush()
        os.fsync(output.fileno())
    os.chmod(temporary, mode, follow_symlinks=False)
    os.replace(temporary, path)
    directory = os.open(parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)
finally:
    try:
        os.unlink(temporary)
    except FileNotFoundError:
        pass
"#;

const FILE_RENAME: &str = r#"
import os
import sys

root = os.path.realpath("/company")
source, destination = sys.argv[1:3]
try:
    source_resolved = os.path.realpath(source, strict=True)
    source_parent = os.path.realpath(os.path.dirname(source), strict=True)
    destination_parent = os.path.realpath(os.path.dirname(destination), strict=True)
except FileNotFoundError:
    raise SystemExit(40)
if (os.path.commonpath((root, source_resolved)) != root
        or os.path.commonpath((root, source_parent)) != root
        or os.path.commonpath((root, destination_parent)) != root
        or os.path.islink(source)):
    raise SystemExit(43)
try:
    os.replace(source, destination)
except FileNotFoundError:
    raise SystemExit(40)
except (IsADirectoryError, NotADirectoryError):
    raise SystemExit(41)
directory = os.open(destination_parent, os.O_RDONLY | os.O_DIRECTORY)
try:
    os.fsync(directory)
finally:
    os.close(directory)
"#;

const FILE_DIGEST: &str = r#"
import hashlib
import os
import stat
import sys

root = os.path.realpath("/company")
path = sys.argv[1]
try:
    resolved = os.path.realpath(path, strict=True)
except FileNotFoundError:
    raise SystemExit(40)
if os.path.commonpath((root, resolved)) != root:
    raise SystemExit(43)
value = os.stat(resolved, follow_symlinks=True)
if not stat.S_ISREG(value.st_mode):
    raise SystemExit(43)
digest = hashlib.sha256()
with open(resolved, "rb", buffering=0) as source:
    while True:
        block = source.read(1024 * 1024)
        if not block:
            break
        digest.update(block)
sys.stdout.buffer.write(digest.digest())
"#;

const PROCESS_PROBE: &str = r#"
import os
import sys

root = os.path.realpath("/company")
if root != "/company" or not os.path.isdir(root) or not os.access(root, os.R_OK | os.W_OK | os.X_OK):
    raise SystemExit(1)
"#;

const PORT_PROBE: &str = r#"
import socket
import sys

connection = socket.create_connection(("127.0.0.1", int(sys.argv[1])), timeout=1.0)
connection.close()
"#;

const INSPECT_FORMAT: &str = concat!(
    "{{.Id}}\n",
    "{{.State.Running}}\n",
    "{{.Image}}\n",
    "{{range .Mounts}}{{if eq .Destination \"/company\"}}{{.Name}}{{end}}{{end}}"
);

/// Docker-backed transport for a single local account plane.
///
/// The namespace is captured once at composition time.  This matches the
/// appliance's resource naming while preventing a process-global environment
/// mutation from redirecting an already-running daemon to a different cell.
#[derive(Clone)]
pub struct LocalDockerRuntimeTransport {
    namespace: Option<String>,
    published_service_ports: Arc<BTreeSet<u16>>,
    operations: Arc<Mutex<BTreeSet<Uuid>>>,
    processes: Arc<Mutex<HashMap<Uuid, ActiveProcessRecord>>>,
    services: Arc<Mutex<HashMap<Uuid, ActiveServiceRecord>>>,
}

impl LocalDockerRuntimeTransport {
    pub fn from_environment() -> Result<Self, RuntimeTransportError> {
        Self::from_environment_with_published_ports([])
    }

    pub fn from_environment_with_published_ports(
        published_service_ports: impl IntoIterator<Item = u16>,
    ) -> Result<Self, RuntimeTransportError> {
        let namespace = std::env::var(RESOURCE_NAMESPACE_ENV)
            .ok()
            .filter(|value| !value.is_empty());
        Self::new(namespace, published_service_ports)
    }

    pub fn new(
        namespace: Option<String>,
        published_service_ports: impl IntoIterator<Item = u16>,
    ) -> Result<Self, RuntimeTransportError> {
        if let Some(namespace) = namespace.as_deref() {
            validate_namespace(namespace)?;
        }
        let mut allowed = BTreeSet::new();
        for port in published_service_ports {
            RuntimeService::Published(port).loopback_port()?;
            allowed.insert(port);
        }
        Ok(Self {
            namespace,
            published_service_ports: Arc::new(allowed),
            operations: Arc::new(Mutex::new(BTreeSet::new())),
            processes: Arc::new(Mutex::new(HashMap::new())),
            services: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn container_name(&self, company: &str) -> Result<String, RuntimeTransportError> {
        validate_company(company)?;
        Ok(match self.namespace.as_deref() {
            Some(namespace) => format!("restless-{namespace}-co-{company}"),
            None => format!("restless-co-{company}"),
        })
    }

    fn volume_name(&self, company: &str) -> Result<String, RuntimeTransportError> {
        validate_company(company)?;
        Ok(match self.namespace.as_deref() {
            Some(namespace) => format!("restless-{namespace}-vol-{company}"),
            None => format!("restless-vol-{company}"),
        })
    }

    fn process_launch(
        &self,
        specification: &RuntimeProcessSpec,
    ) -> Result<ProcessLaunch, RuntimeTransportError> {
        let company = specification.authority.company();
        let container = self.container_name(company)?;
        let process_id = specification.operation_id;
        let process_user = process_user(&specification.authority);
        // The staged environment may contain short-lived credentials. Keep it
        // on the replaceable container filesystem, never the persistent
        // company volume, and unlink it before the governed executable starts.
        let identity_directory = if process_user == "company" {
            "company"
        } else {
            "effect"
        };
        let directory =
            format!("/tmp/restless-runtime-processes-{identity_directory}/{process_id}");
        let environment_path = format!("{directory}.environment.json");
        let pid_path = format!("{directory}.pid");
        let environment = authoritative_environment(specification)?;
        let environment_body = serde_json::to_vec(&environment)
            .map_err(|error| RuntimeTransportError::Transport(error.to_string()))?;

        let environment_args = vec![
            "exec".into(),
            "-i".into(),
            "-u".into(),
            process_user.into(),
            container.clone(),
            "python3".into(),
            "-c".into(),
            PROCESS_ENVIRONMENT_WRITER.into(),
            environment_path.clone(),
        ];
        let mut process_args = vec![
            "exec".into(),
            "-i".into(),
            "-u".into(),
            process_user.into(),
            "-w".into(),
            specification.working_directory.as_str().into(),
            container.clone(),
            "setsid".into(),
            "--wait".into(),
            "python3".into(),
            "-c".into(),
            PROCESS_WRAPPER.into(),
            environment_path.clone(),
            pid_path.clone(),
            specification.executable.clone(),
        ];
        process_args.extend(specification.arguments.iter().cloned());

        Ok(ProcessLaunch {
            company: company.into(),
            container,
            process_id,
            process_user: process_user.into(),
            environment_path,
            pid_path,
            environment_args,
            environment_body,
            process_args,
        })
    }

    async fn stage_process_environment(
        &self,
        launch: &ProcessLaunch,
    ) -> Result<(), RuntimeTransportError> {
        let mut command = guarded_docker(&launch.environment_args)?;
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| transport_error("stage process environment", error))?;
        let mut stdin = child.stdin.take().ok_or_else(|| {
            RuntimeTransportError::Transport("Docker environment input was unavailable".into())
        })?;
        stdin
            .write_all(&launch.environment_body)
            .await
            .map_err(|error| transport_error("write process environment", error))?;
        stdin
            .shutdown()
            .await
            .map_err(|error| transport_error("finish process environment", error))?;
        drop(stdin);
        let output = child
            .wait_with_output()
            .await
            .map_err(|error| transport_error("stage process environment", error))?;
        checked_output(output, "stage process environment")?;
        Ok(())
    }

    async fn process_pid(
        &self,
        launch: &ProcessLaunch,
        child: &mut Child,
    ) -> Result<u32, RuntimeTransportError> {
        let args = vec![
            "exec".into(),
            "-u".into(),
            launch.process_user.clone(),
            launch.container.clone(),
            "cat".into(),
            launch.pid_path.clone(),
        ];
        for _ in 0..PROCESS_HANDSHAKE_ATTEMPTS {
            let output = docker_output(&args).await?;
            if output.status.success() {
                let value = String::from_utf8_lossy(&output.stdout);
                let value = value.trim();
                if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
                    if let Ok(pid) = value.parse::<u32>() {
                        if pid > 1 {
                            return Ok(pid);
                        }
                    }
                }
            }
            if let Some(status) = child
                .try_wait()
                .map_err(|error| transport_error("observe governed process start", error))?
            {
                return Err(RuntimeTransportError::Remote(format!(
                    "governed process exited before its PID handshake ({status})"
                )));
            }
            tokio::time::sleep(PROCESS_HANDSHAKE_INTERVAL).await;
        }
        Err(RuntimeTransportError::DeadlineExceeded)
    }

    fn ensure_published_service_allowed(
        &self,
        service: RuntimeService,
    ) -> Result<u16, RuntimeTransportError> {
        let port = service.loopback_port()?;
        if let RuntimeService::Published(port) = service {
            if !self.published_service_ports.contains(&port) {
                return Err(RuntimeTransportError::Unauthorized);
            }
        }
        Ok(port)
    }
}

#[async_trait]
impl RuntimeTransport for LocalDockerRuntimeTransport {
    async fn readiness(
        &self,
        company: &str,
    ) -> Result<crate::runtime_transport::RuntimeReadiness, RuntimeTransportError> {
        let container = self.container_name(company)?;
        let expected_volume = self.volume_name(company)?;
        let inspect_args = inspect_arguments(&container);
        let inspect = checked_output(
            docker_output(&inspect_args).await?,
            "inspect company Runtime",
        )?;
        let inspect = String::from_utf8(inspect.stdout).map_err(|_| {
            RuntimeTransportError::Transport("Docker inspection was not UTF-8".into())
        })?;
        let mut fields = inspect.lines();
        let container_id = fields.next().unwrap_or_default().trim().to_owned();
        let running = fields.next().unwrap_or_default().trim() == "true";
        let runtime_image = fields.next().unwrap_or_default().trim().to_owned();
        let mounted_volume = fields.next().unwrap_or_default().trim().to_owned();
        if container_id.is_empty() || runtime_image.is_empty() {
            return Err(RuntimeTransportError::Transport(
                "Docker inspection omitted Runtime identity".into(),
            ));
        }

        let source_revision_args = vec![
            "exec".into(),
            "-u".into(),
            "company".into(),
            container.clone(),
            "printenv".into(),
            "RESTLESS_SOURCE_REVISION".into(),
        ];
        let source_revision = if running {
            let output = docker_output(&source_revision_args).await?;
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).trim().to_owned()
            } else {
                "unknown".into()
            }
        } else {
            "unknown".into()
        };

        let process_execution = if running {
            docker_probe(vec![
                "exec".into(),
                "-u".into(),
                "company".into(),
                container.clone(),
                "python3".into(),
                "-c".into(),
                PROCESS_PROBE.into(),
            ])
            .await?
        } else {
            false
        };
        let mut components = vec![
            component("container", running),
            component("persistent_volume", mounted_volume == expected_volume),
            component("process_execution", process_execution),
        ];
        for (name, port) in [
            ("desktop", 6080_u16),
            ("browser_control", 9223_u16),
            ("release_health", 7789_u16),
        ] {
            let ready = if running {
                docker_probe(port_probe_arguments(&container, port)).await?
            } else {
                false
            };
            components.push(component(name, ready));
        }

        Ok(crate::runtime_transport::RuntimeReadiness {
            runtime_id: container,
            runtime_generation: local_generation(&container_id),
            runtime_image,
            source_revision,
            volume_name: mounted_volume,
            observed_at: Utc::now(),
            components,
        })
    }

    async fn start_process(
        &self,
        specification: RuntimeProcessSpec,
    ) -> Result<RuntimeProcess, RuntimeTransportError> {
        let now = Utc::now();
        specification.validate(now)?;
        let reservation = OperationReservation::acquire(
            Arc::clone(&self.operations),
            specification.operation_id,
        )?;
        let launch = self.process_launch(&specification)?;
        self.stage_process_environment(&launch).await?;

        let mut command = guarded_docker(&launch.process_args)?;
        let mut child = match command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                cleanup_process_files(
                    &launch.container,
                    &launch.process_user,
                    &launch.environment_path,
                    &launch.pid_path,
                )
                .await;
                return Err(transport_error("start governed process", error));
            }
        };
        let pid = match self.process_pid(&launch, &mut child).await {
            Ok(pid) => pid,
            Err(error) => {
                let _ = child.kill().await;
                cleanup_process_files(
                    &launch.container,
                    &launch.process_user,
                    &launch.environment_path,
                    &launch.pid_path,
                )
                .await;
                return Err(error);
            }
        };
        let stdin = child.stdin.take().ok_or_else(|| {
            RuntimeTransportError::Transport("governed process stdin was unavailable".into())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            RuntimeTransportError::Transport("governed process stdout was unavailable".into())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            RuntimeTransportError::Transport("governed process stderr was unavailable".into())
        })?;
        let started_at = Utc::now();
        let record = ActiveProcessRecord {
            company: launch.company.clone(),
            process_id: launch.process_id,
            pid,
            authority: specification.authority.clone(),
            started_at,
        };
        lock(&self.processes).insert(launch.process_id, record);

        let completion = Arc::new(ProcessCompletion::default());
        let control = Arc::new(LocalProcessControl {
            container: launch.container.clone(),
            process_user: launch.process_user.clone(),
            pid_path: launch.pid_path.clone(),
            completion: Arc::clone(&completion),
        });
        let completion_for_wait = Arc::clone(&completion);
        let operation_records = Arc::clone(&self.operations);
        let process_records = Arc::clone(&self.processes);
        let process_id = launch.process_id;
        let cleanup_container = launch.container.clone();
        let cleanup_user = launch.process_user.clone();
        let cleanup_environment = launch.environment_path.clone();
        let cleanup_pid = launch.pid_path.clone();
        tokio::spawn(async move {
            let result = child
                .wait()
                .await
                .map(|status| completion_for_wait.process_exit(status))
                .map_err(|error| transport_error("wait for governed process", error));
            completion_for_wait.finish(result);
            lock(&process_records).remove(&process_id);
            lock(&operation_records).remove(&process_id);
            cleanup_process_files(
                &cleanup_container,
                &cleanup_user,
                &cleanup_environment,
                &cleanup_pid,
            )
            .await;
        });

        let deadline_control = Arc::clone(&control);
        let until_deadline = specification
            .deadline
            .signed_duration_since(Utc::now())
            .to_std()
            .unwrap_or_default();
        tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(until_deadline) => {
                    if !deadline_control.completion.is_finished() {
                        let _ = deadline_control.send_signal(RuntimeSignal::Kill).await;
                    }
                }
                _ = deadline_control.completion.changed() => {}
            }
        });
        reservation.retain_until_completion();

        Ok(RuntimeProcess {
            process_id: launch.process_id,
            pid,
            stdin: Box::pin(stdin),
            stdout: Box::pin(stdout),
            stderr: Box::pin(stderr),
            control: Box::new(SharedProcessControl(control)),
        })
    }

    async fn stat(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<RuntimeFileMetadata, RuntimeTransportError> {
        let args = file_arguments(self.container_name(company)?, FILE_STAT, [path.as_str()]);
        let output = checked_output(docker_output(&args).await?, "stat Runtime file")?;
        let wire: FileMetadataWire = serde_json::from_slice(&output.stdout).map_err(|error| {
            RuntimeTransportError::Transport(format!("decode Runtime file metadata: {error}"))
        })?;
        wire.try_into()
    }

    async fn list(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<Vec<RuntimeDirectoryEntry>, RuntimeTransportError> {
        let limit = LIST_ENTRY_LIMIT.to_string();
        let args = file_arguments(
            self.container_name(company)?,
            FILE_LIST,
            [path.as_str(), limit.as_str()],
        );
        let output = checked_output(docker_output(&args).await?, "list Runtime directory")?;
        if output.stdout.len() > LIST_OUTPUT_LIMIT {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime directory listing exceeds the local transport limit",
            ));
        }
        let wires: Vec<DirectoryEntryWire> =
            serde_json::from_slice(&output.stdout).map_err(|error| {
                RuntimeTransportError::Transport(format!(
                    "decode Runtime directory listing: {error}"
                ))
            })?;
        wires.into_iter().map(TryInto::try_into).collect()
    }

    async fn read(
        &self,
        company: &str,
        path: &CompanyPath,
        maximum_bytes: usize,
    ) -> Result<Vec<u8>, RuntimeTransportError> {
        if maximum_bytes == 0 || maximum_bytes > FILE_OUTPUT_LIMIT {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime file read limit must be between 1 byte and 16 MiB",
            ));
        }
        let maximum = maximum_bytes.to_string();
        let args = file_arguments(
            self.container_name(company)?,
            FILE_READ,
            [path.as_str(), maximum.as_str()],
        );
        let output = checked_output(docker_output(&args).await?, "read Runtime file")?;
        if output.stdout.len() > maximum_bytes {
            return Err(RuntimeTransportError::Transport(
                "Runtime returned more file data than requested".into(),
            ));
        }
        Ok(output.stdout)
    }

    async fn atomic_write(
        &self,
        company: &str,
        operation_id: Uuid,
        path: &CompanyPath,
        contents: &[u8],
        mode: u32,
    ) -> Result<(), RuntimeTransportError> {
        validate_file_mutation(operation_id, path, mode, contents.len())?;
        let operation = operation_id.to_string();
        let mode = mode.to_string();
        let limit = FILE_INPUT_LIMIT.to_string();
        let mut args = file_arguments(
            self.container_name(company)?,
            FILE_ATOMIC_WRITE,
            [
                path.as_str(),
                operation.as_str(),
                mode.as_str(),
                limit.as_str(),
            ],
        );
        // `docker exec` does not attach its stdin unless `-i` appears before
        // the container name. Without this exact flag the atomic target was
        // successfully created with zero bytes while the caller had supplied
        // content, which is precisely the silent data loss this transport
        // boundary must prevent.
        args.insert(1, "-i".into());
        let mut command = guarded_docker(&args)?;
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| transport_error("start atomic Runtime file write", error))?;
        let mut stdin = child.stdin.take().ok_or_else(|| {
            RuntimeTransportError::Transport("Runtime file input was unavailable".into())
        })?;
        stdin
            .write_all(contents)
            .await
            .map_err(|error| transport_error("write Runtime file", error))?;
        stdin
            .shutdown()
            .await
            .map_err(|error| transport_error("finish Runtime file write", error))?;
        drop(stdin);
        let output = child
            .wait_with_output()
            .await
            .map_err(|error| transport_error("finish atomic Runtime file write", error))?;
        checked_output(output, "write Runtime file")?;
        Ok(())
    }

    async fn rename(
        &self,
        company: &str,
        operation_id: Uuid,
        source: &CompanyPath,
        destination: &CompanyPath,
    ) -> Result<(), RuntimeTransportError> {
        if operation_id.is_nil()
            || source.as_str() == "/company"
            || destination.as_str() == "/company"
        {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime rename requires a non-nil operation and two company entries",
            ));
        }
        let args = file_arguments(
            self.container_name(company)?,
            FILE_RENAME,
            [source.as_str(), destination.as_str()],
        );
        checked_output(docker_output(&args).await?, "rename Runtime file")?;
        Ok(())
    }

    async fn digest(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<[u8; 32], RuntimeTransportError> {
        let args = file_arguments(self.container_name(company)?, FILE_DIGEST, [path.as_str()]);
        let output = checked_output(docker_output(&args).await?, "digest Runtime file")?;
        output.stdout.try_into().map_err(|body: Vec<u8>| {
            RuntimeTransportError::Transport(format!(
                "Runtime returned a {}-byte SHA-256 digest",
                body.len()
            ))
        })
    }

    async fn open_service(
        &self,
        company: &str,
        operation_id: Uuid,
        service: RuntimeService,
        idle_timeout: Duration,
    ) -> Result<RuntimeDuplex, RuntimeTransportError> {
        validate_company(company)?;
        if operation_id.is_nil() {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime service operation ID must be non-nil",
            ));
        }
        if idle_timeout.is_zero() || idle_timeout > MAX_SERVICE_IDLE {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime service idle timeout must be between 1 millisecond and 1 hour",
            ));
        }
        let port = self.ensure_published_service_allowed(service)?;
        let reservation =
            OperationReservation::acquire(Arc::clone(&self.operations), operation_id)?;
        let container = self.container_name(company)?;
        let timeout_seconds = idle_timeout.as_secs().max(1).to_string();
        let destination = format!("TCP:127.0.0.1:{port},connect-timeout=3");
        let args = vec![
            "exec".into(),
            "-i".into(),
            "-u".into(),
            "company".into(),
            container,
            "socat".into(),
            "-T".into(),
            timeout_seconds,
            "STDIO".into(),
            destination,
        ];
        let mut command = guarded_docker(&args)?;
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| transport_error("open local Runtime service", error))?;
        let stdin = child.stdin.take().ok_or_else(|| {
            RuntimeTransportError::Transport("Runtime service input was unavailable".into())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            RuntimeTransportError::Transport("Runtime service output was unavailable".into())
        })?;
        lock(&self.services).insert(
            operation_id,
            ActiveServiceRecord {
                company: company.into(),
            },
        );
        let stream: RuntimeDuplex = Box::new(LocalServiceStream {
            _child: child,
            stdin,
            stdout,
            operation_id,
            services: Arc::clone(&self.services),
            operations: Arc::clone(&self.operations),
            closed: false,
        });
        reservation.retain_until_completion();
        Ok(stream)
    }

    async fn activity(&self, company: &str) -> Result<RuntimeActivity, RuntimeTransportError> {
        validate_company(company)?;
        let active_processes = lock(&self.processes)
            .values()
            .filter(|record| record.company == company)
            .map(|record| RuntimeActiveProcess {
                process_id: record.process_id,
                pid: record.pid,
                authority: record.authority.clone(),
                started_at: record.started_at,
            })
            .collect();
        let open_service_streams = lock(&self.services)
            .values()
            .filter(|record| record.company == company)
            .count();
        Ok(RuntimeActivity {
            observed_at: Utc::now(),
            active_processes,
            open_service_streams,
        })
    }
}

struct ProcessLaunch {
    company: String,
    container: String,
    process_id: Uuid,
    process_user: String,
    environment_path: String,
    pid_path: String,
    environment_args: Vec<String>,
    environment_body: Vec<u8>,
    process_args: Vec<String>,
}

#[derive(Clone)]
struct ActiveProcessRecord {
    company: String,
    process_id: Uuid,
    pid: u32,
    authority: RuntimeProcessAuthority,
    started_at: DateTime<Utc>,
}

struct ActiveServiceRecord {
    company: String,
}

struct OperationReservation {
    operations: Arc<Mutex<BTreeSet<Uuid>>>,
    operation_id: Uuid,
    retained: bool,
}

impl OperationReservation {
    fn acquire(
        operations: Arc<Mutex<BTreeSet<Uuid>>>,
        operation_id: Uuid,
    ) -> Result<Self, RuntimeTransportError> {
        if !lock(&operations).insert(operation_id) {
            return Err(RuntimeTransportError::Conflict);
        }
        Ok(Self {
            operations,
            operation_id,
            retained: false,
        })
    }

    fn retain_until_completion(mut self) {
        self.retained = true;
    }
}

impl Drop for OperationReservation {
    fn drop(&mut self) {
        if !self.retained {
            lock(&self.operations).remove(&self.operation_id);
        }
    }
}

#[derive(Default)]
struct ProcessCompletion {
    result: Mutex<Option<Result<RuntimeProcessExit, RuntimeTransportError>>>,
    requested_signal: Mutex<Option<i32>>,
    changed: Notify,
}

impl ProcessCompletion {
    fn finish(&self, result: Result<RuntimeProcessExit, RuntimeTransportError>) {
        *lock(&self.result) = Some(result);
        self.changed.notify_waiters();
    }

    fn is_finished(&self) -> bool {
        lock(&self.result).is_some()
    }

    fn process_exit(&self, status: ExitStatus) -> RuntimeProcessExit {
        process_exit(status, *lock(&self.requested_signal))
    }

    fn note_delivered_signal(&self, signal: i32) {
        *lock(&self.requested_signal) = Some(signal);
        let mut result = lock(&self.result);
        if let Some(Ok(exit)) = result.as_mut() {
            if exit.code == Some(signal) {
                exit.code = None;
                exit.signal = Some(signal);
            }
        }
    }

    async fn changed(&self) {
        loop {
            let notified = self.changed.notified();
            if self.is_finished() {
                return;
            }
            notified.await;
        }
    }

    async fn wait(&self) -> Result<RuntimeProcessExit, RuntimeTransportError> {
        self.changed().await;
        lock(&self.result)
            .as_ref()
            .expect("completion was observed")
            .clone()
    }
}

struct LocalProcessControl {
    container: String,
    process_user: String,
    pid_path: String,
    completion: Arc<ProcessCompletion>,
}

impl LocalProcessControl {
    async fn send_signal(&self, signal: RuntimeSignal) -> Result<(), RuntimeTransportError> {
        if self.completion.is_finished() {
            return Err(RuntimeTransportError::NotFound);
        }
        let signal_number = match signal {
            RuntimeSignal::Interrupt => 2,
            RuntimeSignal::Terminate => 15,
            RuntimeSignal::Kill => 9,
        };
        let args = vec![
            "exec".into(),
            "-u".into(),
            self.process_user.clone(),
            self.container.clone(),
            "python3".into(),
            "-c".into(),
            PROCESS_SIGNAL.into(),
            self.pid_path.clone(),
            signal_number.to_string(),
        ];
        let outcome = match docker_output(&args).await {
            Ok(output) => checked_output(output, "signal governed process").map(|_| ()),
            Err(error) => Err(error),
        };
        outcome?;
        self.completion.note_delivered_signal(signal_number);
        Ok(())
    }
}

struct SharedProcessControl(Arc<LocalProcessControl>);

#[async_trait]
impl RuntimeProcessControl for SharedProcessControl {
    async fn signal(&self, signal: RuntimeSignal) -> Result<(), RuntimeTransportError> {
        self.0.send_signal(signal).await
    }

    async fn wait(&self) -> Result<RuntimeProcessExit, RuntimeTransportError> {
        self.0.completion.wait().await
    }
}

struct LocalServiceStream {
    _child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    operation_id: Uuid,
    services: Arc<Mutex<HashMap<Uuid, ActiveServiceRecord>>>,
    operations: Arc<Mutex<BTreeSet<Uuid>>>,
    closed: bool,
}

impl LocalServiceStream {
    fn close_once(&mut self) {
        if !self.closed {
            self.closed = true;
            lock(&self.services).remove(&self.operation_id);
            lock(&self.operations).remove(&self.operation_id);
        }
    }
}

impl Drop for LocalServiceStream {
    fn drop(&mut self) {
        self.close_once();
    }
}

impl AsyncRead for LocalServiceStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buffer.filled().len();
        let result = Pin::new(&mut self.stdout).poll_read(context, buffer);
        if matches!(&result, Poll::Ready(Err(_)))
            || matches!(&result, Poll::Ready(Ok(())) if buffer.filled().len() == before)
        {
            self.close_once();
        }
        result
    }
}

impl AsyncWrite for LocalServiceStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        let result = Pin::new(&mut self.stdin).poll_write(context, buffer);
        if matches!(&result, Poll::Ready(Err(_))) {
            self.close_once();
        }
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.stdin).poll_flush(context);
        if matches!(&result, Poll::Ready(Err(_))) {
            self.close_once();
        }
        result
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.stdin).poll_shutdown(context);
        if matches!(&result, Poll::Ready(Err(_))) {
            self.close_once();
        }
        result
    }
}

#[derive(Deserialize)]
struct FileMetadataWire {
    kind: String,
    size: u64,
    modified_ns: i64,
    mode: u32,
}

impl TryFrom<FileMetadataWire> for RuntimeFileMetadata {
    type Error = RuntimeTransportError;

    fn try_from(value: FileMetadataWire) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: file_kind(&value.kind)?,
            size: value.size,
            modified_at: timestamp_from_nanoseconds(value.modified_ns)?,
            mode: value.mode,
        })
    }
}

#[derive(Deserialize)]
struct DirectoryEntryWire {
    name: String,
    kind: String,
    size: u64,
    modified_ns: i64,
    mode: u32,
}

impl TryFrom<DirectoryEntryWire> for RuntimeDirectoryEntry {
    type Error = RuntimeTransportError;

    fn try_from(value: DirectoryEntryWire) -> Result<Self, Self::Error> {
        if value.name.is_empty()
            || value.name == "."
            || value.name == ".."
            || value.name.contains(['/', '\0'])
        {
            return Err(RuntimeTransportError::Transport(
                "Runtime returned an invalid directory entry name".into(),
            ));
        }
        Ok(Self {
            name: value.name,
            metadata: RuntimeFileMetadata {
                kind: file_kind(&value.kind)?,
                size: value.size,
                modified_at: timestamp_from_nanoseconds(value.modified_ns)?,
                mode: value.mode,
            },
        })
    }
}

fn file_kind(value: &str) -> Result<RuntimeFileKind, RuntimeTransportError> {
    match value {
        "file" => Ok(RuntimeFileKind::File),
        "directory" => Ok(RuntimeFileKind::Directory),
        _ => Err(RuntimeTransportError::Transport(
            "Runtime returned an unsupported file kind".into(),
        )),
    }
}

fn timestamp_from_nanoseconds(value: i64) -> Result<DateTime<Utc>, RuntimeTransportError> {
    let seconds = value.div_euclid(1_000_000_000);
    let nanoseconds = value.rem_euclid(1_000_000_000) as u32;
    Utc.timestamp_opt(seconds, nanoseconds)
        .single()
        .ok_or_else(|| {
            RuntimeTransportError::Transport("Runtime returned an invalid file timestamp".into())
        })
}

fn authoritative_environment(
    specification: &RuntimeProcessSpec,
) -> Result<BTreeMap<String, String>, RuntimeTransportError> {
    let mut environment = BTreeMap::new();
    for variable in &specification.environment {
        if environment
            .insert(variable.name.clone(), variable.value.clone())
            .is_some()
        {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime process environment contains a duplicate name",
            ));
        }
    }
    environment.insert(
        "RESTLESS_RUNTIME_OPERATION_ID".into(),
        specification.operation_id.to_string(),
    );
    environment.insert(
        "RESTLESS_COMPANY".into(),
        specification.authority.company().into(),
    );
    match &specification.authority {
        RuntimeProcessAuthority::Attempt {
            actor,
            responsibility,
            work_id,
            attempt_id,
            session_id,
            ..
        } => {
            environment.insert("RESTLESS_ACTOR".into(), actor.clone());
            environment.insert("RESTLESS_RESPONSIBILITY".into(), responsibility.clone());
            environment.insert("RESTLESS_WORK_ID".into(), work_id.to_string());
            environment.insert("RESTLESS_ATTEMPT_ID".into(), attempt_id.to_string());
            environment.insert("RESTLESS_SESSION_ID".into(), session_id.clone());
        }
        RuntimeProcessAuthority::AuthorityEvent {
            actor,
            responsibility,
            event_id,
            session_id,
            ..
        } => {
            environment.insert("RESTLESS_ACTOR".into(), actor.clone());
            environment.insert("RESTLESS_RESPONSIBILITY".into(), responsibility.clone());
            environment.insert("RESTLESS_AUTHORITY_EVENT_ID".into(), event_id.to_string());
            environment.insert("RESTLESS_SESSION_ID".into(), session_id.clone());
        }
        RuntimeProcessAuthority::GovernedEffect {
            effect_class,
            authority_id,
            idempotency_key,
            execution_no,
            staging_id,
            phase,
            actor,
            ..
        } => {
            environment.insert("RESTLESS_ACTOR".into(), actor.clone());
            environment.insert("RESTLESS_EFFECT_CLASS".into(), effect_class.clone());
            environment.insert(
                "RESTLESS_EFFECT_AUTHORITY_ID".into(),
                authority_id.to_string(),
            );
            environment.insert(
                "RESTLESS_EFFECT_IDEMPOTENCY_KEY".into(),
                idempotency_key.clone(),
            );
            environment.insert(
                "RESTLESS_EFFECT_EXECUTION_NO".into(),
                execution_no.to_string(),
            );
            environment.insert("RESTLESS_EFFECT_STAGING_ID".into(), staging_id.to_string());
            environment.insert("RESTLESS_EFFECT_PHASE".into(), phase.as_str().into());
        }
        RuntimeProcessAuthority::InfrastructureProbe { probe, .. } => {
            environment.insert("RESTLESS_RUNTIME_PROBE".into(), probe.clone());
        }
    }
    Ok(environment)
}

fn process_user(authority: &RuntimeProcessAuthority) -> &'static str {
    match authority {
        RuntimeProcessAuthority::GovernedEffect { phase, .. } if phase.uses_effect_identity() => {
            "2001:2000"
        }
        _ => "company",
    }
}

fn validate_namespace(namespace: &str) -> Result<(), RuntimeTransportError> {
    if namespace.is_empty()
        || namespace.len() > 24
        || !namespace.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                byte.is_ascii_lowercase()
            } else {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
            }
        })
    {
        return Err(RuntimeTransportError::InvalidRequest(
            "Runtime resource namespace must be 1..24 lowercase letters, digits, '_' or '-', starting with a letter",
        ));
    }
    Ok(())
}

fn validate_file_mutation(
    operation_id: Uuid,
    path: &CompanyPath,
    mode: u32,
    length: usize,
) -> Result<(), RuntimeTransportError> {
    if operation_id.is_nil() {
        return Err(RuntimeTransportError::InvalidRequest(
            "Runtime file operation ID must be non-nil",
        ));
    }
    if path.as_str() == "/company" {
        return Err(RuntimeTransportError::InvalidRequest(
            "the company root cannot be replaced with a file",
        ));
    }
    if mode & !0o777 != 0 || mode & 0o600 != 0o600 {
        return Err(RuntimeTransportError::InvalidRequest(
            "Runtime file mode must be an owner-readable, owner-writable permission mode",
        ));
    }
    if length > FILE_INPUT_LIMIT {
        return Err(RuntimeTransportError::InvalidRequest(
            "Runtime atomic file write exceeds 16 MiB",
        ));
    }
    Ok(())
}

fn inspect_arguments(container: &str) -> Vec<String> {
    vec![
        "inspect".into(),
        "--format".into(),
        INSPECT_FORMAT.into(),
        container.into(),
    ]
}

fn port_probe_arguments(container: &str, port: u16) -> Vec<String> {
    vec![
        "exec".into(),
        "-u".into(),
        "company".into(),
        container.into(),
        "python3".into(),
        "-c".into(),
        PORT_PROBE.into(),
        port.to_string(),
    ]
}

fn file_arguments<'a>(
    container: String,
    program: &str,
    arguments: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    let mut result = vec![
        "exec".into(),
        "-u".into(),
        "company".into(),
        container,
        "python3".into(),
        "-c".into(),
        program.into(),
    ];
    result.extend(arguments.into_iter().map(str::to_owned));
    result
}

fn component(name: &str, ready: bool) -> RuntimeComponentCheck {
    RuntimeComponentCheck {
        name: name.into(),
        status: if ready {
            RuntimeComponentStatus::Ready
        } else {
            RuntimeComponentStatus::Degraded
        },
    }
}

fn local_generation(container_id: &str) -> i64 {
    let digest = Sha256::digest(container_id.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    (u64::from_be_bytes(bytes) & i64::MAX as u64).max(1) as i64
}

fn guarded_docker(args: &[String]) -> Result<Command, RuntimeTransportError> {
    hosted_runtime::require_local_docker_from_environment()
        .map_err(|_| RuntimeTransportError::Unavailable)?;
    let mut command = Command::new("docker");
    command.args(args).kill_on_drop(true);
    Ok(command)
}

async fn docker_output(args: &[String]) -> Result<Output, RuntimeTransportError> {
    guarded_docker(args)?
        .output()
        .await
        .map_err(|error| transport_error("spawn Docker", error))
}

async fn docker_probe(args: Vec<String>) -> Result<bool, RuntimeTransportError> {
    Ok(docker_output(&args).await?.status.success())
}

fn checked_output(output: Output, operation: &str) -> Result<Output, RuntimeTransportError> {
    if output.status.success() {
        return Ok(output);
    }
    let error = match output.status.code() {
        Some(EXIT_NOT_FOUND) => RuntimeTransportError::NotFound,
        Some(EXIT_CONFLICT) => RuntimeTransportError::Conflict,
        Some(EXIT_LIMIT) | Some(EXIT_INVALID) => RuntimeTransportError::InvalidRequest(
            "the company Runtime rejected a bounded local operation",
        ),
        _ if docker_not_found(&output.stderr) => RuntimeTransportError::NotFound,
        _ => RuntimeTransportError::Remote(format!(
            "{operation}: {}",
            bounded_stderr(&output.stderr)
        )),
    };
    Err(error)
}

fn docker_not_found(stderr: &[u8]) -> bool {
    let stderr = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    stderr.contains("no such container") || stderr.contains("no such object")
}

fn bounded_stderr(stderr: &[u8]) -> String {
    let mut value = String::from_utf8_lossy(stderr)
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(300)
        .collect::<String>();
    if value.trim().is_empty() {
        value = "operation failed".into();
    }
    value
}

fn transport_error(operation: &str, error: impl std::fmt::Display) -> RuntimeTransportError {
    RuntimeTransportError::Transport(format!("{operation}: {error}"))
}

fn process_exit(status: ExitStatus, requested_signal: Option<i32>) -> RuntimeProcessExit {
    #[cfg(unix)]
    let host_signal = {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    };
    #[cfg(not(unix))]
    let host_signal = None;

    let mut code = status.code();
    let signal = host_signal.or(match code {
        Some(130) => Some(2),
        Some(137) => Some(9),
        Some(143) => Some(15),
        Some(code) if Some(code) == requested_signal => requested_signal,
        _ => None,
    });
    if signal.is_some() {
        code = None;
    }
    RuntimeProcessExit {
        code,
        signal,
        finished_at: Utc::now(),
    }
}

async fn cleanup_process_files(
    container: &str,
    process_user: &str,
    environment_path: &str,
    pid_path: &str,
) {
    let args = vec![
        "exec".into(),
        "-u".into(),
        process_user.into(),
        container.into(),
        "rm".into(),
        "-f".into(),
        "--".into(),
        environment_path.into(),
        format!("{environment_path}.incoming"),
        pid_path.into(),
        format!("{pid_path}.incoming"),
    ];
    let _ = docker_output(&args).await;
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt as _;

    fn process_spec(secret: &str) -> RuntimeProcessSpec {
        RuntimeProcessSpec {
            operation_id: Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap(),
            authority: RuntimeProcessAuthority::Attempt {
                company: "c0123456789abcdef0123456789abcdef".into(),
                actor: "exec".into(),
                responsibility: "owner-direction".into(),
                work_id: Uuid::parse_str("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb").unwrap(),
                attempt_id: Uuid::parse_str("cccccccc-cccc-4ccc-8ccc-cccccccccccc").unwrap(),
                session_id: "session-1".into(),
            },
            executable: "omp".into(),
            arguments: vec!["acp".into(), "--mode=auto".into()],
            working_directory: CompanyPath::parse("/company/projects/restless").unwrap(),
            environment: vec![crate::runtime_transport::RuntimeEnvironment::secret(
                "RESTLESS_MODEL_TOKEN",
                secret,
            )],
            deadline: Utc::now() + chrono::Duration::minutes(10),
        }
    }

    #[test]
    fn local_names_exactly_match_the_appliance_namespace_contract() {
        let stable = LocalDockerRuntimeTransport::new(None, []).unwrap();
        assert_eq!(stable.container_name("aris").unwrap(), "restless-co-aris");
        assert_eq!(stable.volume_name("aris").unwrap(), "restless-vol-aris");

        let dev = LocalDockerRuntimeTransport::new(Some("dev123_test".into()), []).unwrap();
        assert_eq!(
            dev.container_name("aris").unwrap(),
            "restless-dev123_test-co-aris"
        );
        assert_eq!(
            dev.volume_name("aris").unwrap(),
            "restless-dev123_test-vol-aris"
        );
        assert!(LocalDockerRuntimeTransport::new(Some("--context=host".into()), []).is_err());
    }

    #[test]
    fn governed_process_keeps_secrets_off_docker_argv_and_pins_identity() {
        let secret = "provider-secret-that-must-not-enter-argv";
        let transport = LocalDockerRuntimeTransport::new(None, []).unwrap();
        let specification = process_spec(secret);
        let launch = transport.process_launch(&specification).unwrap();
        let argv = launch.process_args.join("\0") + &launch.environment_args.join("\0");
        assert!(!argv.contains(secret));
        assert!(!argv.contains("RESTLESS_MODEL_TOKEN="));
        assert_eq!(
            launch.process_args[..7],
            [
                "exec",
                "-i",
                "-u",
                "company",
                "-w",
                "/company/projects/restless",
                "restless-co-c0123456789abcdef0123456789abcdef"
            ]
        );
        assert_eq!(
            &launch.process_args[7..11],
            ["setsid", "--wait", "python3", "-c"]
        );

        let environment: BTreeMap<String, String> =
            serde_json::from_slice(&launch.environment_body).unwrap();
        assert_eq!(environment["RESTLESS_MODEL_TOKEN"], secret);
        assert_eq!(environment["RESTLESS_ACTOR"], "exec");
        assert_eq!(environment["RESTLESS_RESPONSIBILITY"], "owner-direction");
        assert_eq!(
            environment["RESTLESS_COMPANY"],
            "c0123456789abcdef0123456789abcdef"
        );
        assert_eq!(
            environment["RESTLESS_RUNTIME_OPERATION_ID"],
            specification.operation_id.to_string()
        );
    }

    #[test]
    fn governed_effect_uses_the_dedicated_uid_and_private_environment_staging() {
        let secret = "effect-secret-that-must-not-enter-docker-argv";
        let transport = LocalDockerRuntimeTransport::new(None, []).unwrap();
        let mut specification = process_spec(secret);
        specification.authority = RuntimeProcessAuthority::GovernedEffect {
            company: "c0123456789abcdef0123456789abcdef".into(),
            actor: "exec".into(),
            effect_class: "customer-contact.email".into(),
            authority_id: 44,
            idempotency_key: "welcome-1".into(),
            execution_no: 1,
            staging_id: Uuid::new_v4(),
            phase: crate::runtime_transport::RuntimeEffectPhase::Execute,
        };
        let launch = transport.process_launch(&specification).unwrap();
        let docker_argv = launch.process_args.join("\0") + &launch.environment_args.join("\0");
        assert_eq!(launch.process_user, "2001:2000");
        assert!(launch
            .environment_path
            .starts_with("/tmp/restless-runtime-processes-effect/"));
        assert!(!docker_argv.contains(secret));
        let environment: BTreeMap<String, String> =
            serde_json::from_slice(&launch.environment_body).unwrap();
        assert_eq!(environment["RESTLESS_MODEL_TOKEN"], secret);
        assert_eq!(environment["RESTLESS_EFFECT_AUTHORITY_ID"], "44");
        assert_eq!(environment["RESTLESS_EFFECT_PHASE"], "execute");
    }

    #[test]
    fn caller_cannot_replace_authoritative_process_identity() {
        let mut specification = process_spec("secret");
        specification.environment.extend([
            crate::runtime_transport::RuntimeEnvironment::public("RESTLESS_ACTOR", "intruder"),
            crate::runtime_transport::RuntimeEnvironment::public("RESTLESS_COMPANY", "other"),
        ]);
        let environment = authoritative_environment(&specification).unwrap();
        assert_eq!(environment["RESTLESS_ACTOR"], "exec");
        assert_eq!(
            environment["RESTLESS_COMPANY"],
            "c0123456789abcdef0123456789abcdef"
        );
    }

    #[test]
    fn durable_authority_events_are_materialised_without_inventing_work_identity() {
        let mut specification = process_spec("secret");
        specification.authority = RuntimeProcessAuthority::AuthorityEvent {
            company: "c0123456789abcdef0123456789abcdef".into(),
            actor: "exec".into(),
            responsibility: "portfolio-wake".into(),
            event_id: 42,
            session_id: "session-event-42".into(),
        };
        let environment = authoritative_environment(&specification).unwrap();
        assert_eq!(environment["RESTLESS_AUTHORITY_EVENT_ID"], "42");
        assert_eq!(environment["RESTLESS_ACTOR"], "exec");
        assert!(!environment.contains_key("RESTLESS_WORK_ID"));
        assert!(!environment.contains_key("RESTLESS_ATTEMPT_ID"));
    }

    #[test]
    fn file_paths_are_positional_arguments_not_shell_interpolation() {
        let path = CompanyPath::parse("/company/projects/a b/report.txt").unwrap();
        let container = "restless-co-aris".to_string();
        let arguments = file_arguments(container.clone(), FILE_READ, [path.as_str(), "1024"]);
        assert_eq!(
            &arguments[..7],
            ["exec", "-u", "company", &container, "python3", "-c", FILE_READ]
        );
        assert_eq!(arguments[7], path.as_str());
        assert_eq!(arguments[8], "1024");
        assert!(!FILE_READ.contains("report.txt"));
    }

    #[test]
    fn file_mutations_are_bounded_and_cannot_replace_the_company_root() {
        let operation = Uuid::new_v4();
        let file = CompanyPath::parse("/company/outputs/result.txt").unwrap();
        assert!(validate_file_mutation(operation, &file, 0o640, FILE_INPUT_LIMIT).is_ok());
        assert!(validate_file_mutation(operation, &file, 0o444, 1).is_err());
        assert!(validate_file_mutation(operation, &file, 0o4640, 1).is_err());
        assert!(validate_file_mutation(operation, &file, 0o640, FILE_INPUT_LIMIT + 1).is_err());
        assert!(validate_file_mutation(
            operation,
            &CompanyPath::parse("/company").unwrap(),
            0o640,
            1
        )
        .is_err());
    }

    #[test]
    fn services_are_loopback_only_and_published_ports_need_an_explicit_allow_list() {
        let transport = LocalDockerRuntimeTransport::new(None, [4173]).unwrap();
        assert_eq!(
            transport
                .ensure_published_service_allowed(RuntimeService::Desktop)
                .unwrap(),
            6080
        );
        assert_eq!(
            transport
                .ensure_published_service_allowed(RuntimeService::Published(4173))
                .unwrap(),
            4173
        );
        assert_eq!(
            transport.ensure_published_service_allowed(RuntimeService::Published(3000)),
            Err(RuntimeTransportError::Unauthorized)
        );
        assert!(LocalDockerRuntimeTransport::new(None, [9223]).is_err());

        let arguments = port_probe_arguments("restless-co-aris", 4173);
        assert_eq!(arguments.last().unwrap(), "4173");
        assert!(arguments
            .iter()
            .all(|argument| !argument.contains("0.0.0.0")));
        assert!(PORT_PROBE.contains("127.0.0.1"));
    }

    #[test]
    fn readiness_inspection_names_only_the_exact_company_container() {
        let arguments = inspect_arguments("restless-dev1-co-aris");
        assert_eq!(
            arguments,
            [
                "inspect",
                "--format",
                INSPECT_FORMAT,
                "restless-dev1-co-aris"
            ]
        );
        assert!(!INSPECT_FORMAT.contains(".Config.Env"));
    }

    #[test]
    fn operation_identity_is_reserved_atomically_and_released_on_failure() {
        let operations = Arc::new(Mutex::new(BTreeSet::new()));
        let operation_id = Uuid::new_v4();
        let reservation =
            OperationReservation::acquire(Arc::clone(&operations), operation_id).unwrap();
        assert!(matches!(
            OperationReservation::acquire(Arc::clone(&operations), operation_id),
            Err(RuntimeTransportError::Conflict)
        ));
        drop(reservation);
        assert!(OperationReservation::acquire(operations, operation_id).is_ok());
    }

    #[test]
    fn conventional_container_signal_exit_codes_are_not_reported_as_success_codes() {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            for (code, signal) in [(130, 2), (137, 9), (143, 15)] {
                let exit = process_exit(ExitStatus::from_raw(code << 8), None);
                assert_eq!(exit.code, None);
                assert_eq!(exit.signal, Some(signal));
            }
        }
    }

    struct DockerFixture {
        container: String,
        volume: String,
    }

    impl Drop for DockerFixture {
        fn drop(&mut self) {
            let _ = docker_test_output(["rm", "--force", self.container.as_str()]);
            let _ = docker_test_output(["volume", "rm", self.volume.as_str()]);
        }
    }

    fn docker_test_output<'a>(arguments: impl IntoIterator<Item = &'a str>) -> Output {
        hosted_runtime::require_local_docker_from_environment()
            .expect("the explicit local Docker integration cannot run in network mode");
        std::process::Command::new("docker")
            .args(arguments)
            .output()
            .expect("run Docker for the explicit local transport integration")
    }

    fn require_docker_success(output: Output, operation: &str) {
        assert!(
            output.status.success(),
            "{operation}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// This deliberately creates only an exact, uniquely namespaced `_test`
    /// company and tears its container and volume down even if an assertion
    /// unwinds. It is ignored because ordinary unit verification must not
    /// require Docker or mutate runtime state.
    #[tokio::test]
    #[ignore = "explicit local Docker integration; creates and removes one *_test Runtime"]
    async fn local_docker_transport_carries_real_process_files_service_and_activity() {
        let suffix = Uuid::new_v4().simple().to_string();
        let namespace = format!("lrt{}_test", &suffix[..8]);
        let company = "runtime_transport_test";
        let transport = LocalDockerRuntimeTransport::new(Some(namespace.clone()), [4173]).unwrap();
        let container = transport.container_name(company).unwrap();
        let volume = transport.volume_name(company).unwrap();
        let fixture = DockerFixture {
            container: container.clone(),
            volume: volume.clone(),
        };

        require_docker_success(
            docker_test_output([
                "volume",
                "create",
                "--label",
                "io.restless.profile=test",
                fixture.volume.as_str(),
            ]),
            "create integration volume",
        );
        let volume_mount = format!("{}:/company", fixture.volume);
        let company_environment = format!("RESTLESS_COMPANY={company}");
        require_docker_success(
            docker_test_output([
                "run",
                "--detach",
                "--name",
                fixture.container.as_str(),
                "--hostname",
                company,
                "--cpus",
                "1",
                "--memory",
                "1g",
                "--memory-swap",
                "1g",
                "--pids-limit",
                "512",
                "--env",
                company_environment.as_str(),
                "--volume",
                volume_mount.as_str(),
                "restless-company-image:latest",
            ]),
            "start integration Runtime",
        );

        let readiness = tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                if let Ok(readiness) = transport.readiness(company).await {
                    if readiness
                        .components
                        .iter()
                        .all(|check| check.status == RuntimeComponentStatus::Ready)
                    {
                        break readiness;
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("the local Runtime became ready");
        assert_eq!(readiness.runtime_id, fixture.container);
        assert_eq!(readiness.volume_name, fixture.volume);
        assert!(readiness.runtime_generation > 0);

        let process_id = Uuid::new_v4();
        let specification = RuntimeProcessSpec {
            operation_id: process_id,
            authority: RuntimeProcessAuthority::Attempt {
                company: company.into(),
                actor: "exec".into(),
                responsibility: "transport-proof".into(),
                work_id: Uuid::new_v4(),
                attempt_id: Uuid::new_v4(),
                session_id: "local-runtime-transport-test".into(),
            },
            executable: "python3".into(),
            arguments: vec![
                "-c".into(),
                "import sys; body=sys.stdin.buffer.read(); sys.stdout.buffer.write(body.upper()); sys.stdout.buffer.flush(); sys.stderr.write('observed-stderr\\n')".into(),
            ],
            working_directory: CompanyPath::parse("/company").unwrap(),
            environment: vec![crate::runtime_transport::RuntimeEnvironment::secret(
                "TEST_SECRET",
                "not-in-docker-argv",
            )],
            deadline: Utc::now() + chrono::Duration::seconds(20),
        };
        let RuntimeProcess {
            mut stdin,
            mut stdout,
            mut stderr,
            control,
            ..
        } = transport.start_process(specification).await.unwrap();
        let activity = transport.activity(company).await.unwrap();
        assert_eq!(activity.active_processes.len(), 1);
        assert_eq!(activity.active_processes[0].process_id, process_id);
        stdin.write_all(b"restless runtime\n").await.unwrap();
        stdin.shutdown().await.unwrap();
        drop(stdin);
        let (stdout_result, stderr_result) = tokio::join!(
            async {
                let mut body = Vec::new();
                stdout.read_to_end(&mut body).await.map(|_| body)
            },
            async {
                let mut body = Vec::new();
                stderr.read_to_end(&mut body).await.map(|_| body)
            }
        );
        assert_eq!(stdout_result.unwrap(), b"RESTLESS RUNTIME\n");
        assert_eq!(stderr_result.unwrap(), b"observed-stderr\n");
        let exit = control.wait().await.unwrap();
        assert_eq!(exit.code, Some(0));
        assert_eq!(exit.signal, None);
        assert!(transport
            .activity(company)
            .await
            .unwrap()
            .active_processes
            .is_empty());

        let signalled_process = transport
            .start_process(RuntimeProcessSpec {
                operation_id: Uuid::new_v4(),
                authority: RuntimeProcessAuthority::InfrastructureProbe {
                    company: company.into(),
                    probe: "signal-proof".into(),
                },
                executable: "python3".into(),
                arguments: vec!["-c".into(), "import time; time.sleep(60)".into()],
                working_directory: CompanyPath::parse("/company").unwrap(),
                environment: Vec::new(),
                deadline: Utc::now() + chrono::Duration::seconds(20),
            })
            .await
            .unwrap();
        signalled_process
            .control
            .signal(RuntimeSignal::Terminate)
            .await
            .unwrap();
        let signalled_exit =
            tokio::time::timeout(Duration::from_secs(5), signalled_process.control.wait())
                .await
                .expect("the governed process observed its signal")
                .unwrap();
        assert_eq!(signalled_exit.code, None);
        assert_eq!(signalled_exit.signal, Some(15));

        let original = CompanyPath::parse("/company/outputs/transport-proof.txt").unwrap();
        let renamed = CompanyPath::parse("/company/outputs/transport-proof-renamed.txt").unwrap();
        let contents = b"real local transport file\n";
        transport
            .atomic_write(company, Uuid::new_v4(), &original, contents, 0o640)
            .await
            .unwrap();
        let metadata = transport.stat(company, &original).await.unwrap();
        assert_eq!(metadata.kind, RuntimeFileKind::File);
        assert_eq!(metadata.size, contents.len() as u64);
        assert_eq!(
            transport.read(company, &original, 1024).await.unwrap(),
            contents
        );
        assert_eq!(
            transport.digest(company, &original).await.unwrap(),
            Sha256::digest(contents).as_slice()
        );
        assert!(transport
            .list(company, &CompanyPath::parse("/company/outputs").unwrap())
            .await
            .unwrap()
            .iter()
            .any(|entry| entry.name == "transport-proof.txt"));
        transport
            .rename(company, Uuid::new_v4(), &original, &renamed)
            .await
            .unwrap();
        assert_eq!(
            transport.read(company, &renamed, 1024).await.unwrap(),
            contents
        );

        let service_id = Uuid::new_v4();
        let mut service = transport
            .open_service(
                company,
                service_id,
                RuntimeService::ReleaseHealth,
                Duration::from_secs(10),
            )
            .await
            .unwrap();
        assert_eq!(
            transport
                .activity(company)
                .await
                .unwrap()
                .open_service_streams,
            1
        );
        service
            .write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        service.flush().await.unwrap();
        let mut response = Vec::new();
        service.read_to_end(&mut response).await.unwrap();
        assert!(String::from_utf8_lossy(&response).contains("200 OK"));
        drop(service);
        assert_eq!(
            transport
                .activity(company)
                .await
                .unwrap()
                .open_service_streams,
            0
        );

        drop(fixture);
    }
}
