//! One reviewed local stdio adapter. The filesystem provider runs in a
//! disposable, networkless Docker worker, outside Core's process and mounts.
//! Its package and Node binary are owner-staged in a read-only bundle.

use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use rmcp::{
    model::{CallToolRequestParams, Tool},
    service::RunningService,
    transport::async_rw::AsyncRwTransport,
    RoleClient, ServiceExt as _,
};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use tokio::io::AsyncReadExt as _;
use uuid::Uuid;

use crate::mcp_gateway::McpProbe;

pub(crate) const READ_TOOL: &str = "read_text_file";
const PROVIDER_PACKAGE: &str = "@modelcontextprotocol/server-filesystem";
const REVIEWED_PACKAGE_VERSION: &str = "2026.8.31";
const PROVIDER_ENTRY: &str = "node_modules/@modelcontextprotocol/server-filesystem/dist/index.js";
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_FILE_BYTES: u64 = 256 * 1024;
const WORKER_LIFETIME_SECONDS: u64 = 120;
const STDERR_DIAGNOSTIC_BYTES: usize = 4096;
const WORKER_IMAGE_ENV: &str = "RESTLESS_STDIO_MCP_IMAGE_ID";

/// Keep the upstream tool definition pinned, but show actors only arguments
/// that Core accepts. The published server also advertises head and tail.
pub(crate) fn expose_path_only(tool: &mut Tool) -> Result<()> {
    if tool.name.as_ref() != READ_TOOL {
        bail!("filesystem MCP exposed an unexpected read tool");
    }
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Absolute path to one existing text file under /data"
            }
        },
        "required": ["path"],
        "additionalProperties": false
    });
    tool.input_schema = Arc::new(schema.as_object().expect("static path-only schema").clone());
    tool.description = Some(
        "Read the complete contents of one existing text file under /data (maximum 256 KiB)."
            .into(),
    );
    Ok(())
}

/// Canonical paths are stored at install time so a parent symlink cannot
/// silently retarget the provider or the read mount later.
pub(crate) fn validate_profile(bundle: &str, read_root: &str) -> Result<(PathBuf, PathBuf)> {
    let bundle = canonical_directory(bundle, "provider bundle")?;
    let read_root = canonical_directory(read_root, "read root")?;
    if bundle.starts_with(&read_root) || read_root.starts_with(&bundle) {
        bail!("provider bundle and read root must be separate directories");
    }
    let node = bundle.join("node");
    let entry = bundle.join(PROVIDER_ENTRY);
    let node_meta = std::fs::metadata(&node).context("inspect bundled Node executable")?;
    if !node_meta.is_file() || node_meta.permissions().mode() & 0o111 == 0 {
        bail!("provider bundle must contain an executable node file");
    }
    for (path, label) in [
        (&node, "Node executable"),
        (&entry, "filesystem MCP entrypoint"),
    ] {
        let actual = path
            .canonicalize()
            .with_context(|| format!("inspect {label}"))?;
        if !actual.starts_with(&bundle) || !actual.is_file() {
            bail!("{label} must be a regular file inside the provider bundle");
        }
    }
    let manifest_path =
        bundle.join("node_modules/@modelcontextprotocol/server-filesystem/package.json");
    let manifest_path = manifest_path
        .canonicalize()
        .context("inspect filesystem MCP package manifest")?;
    if !manifest_path.starts_with(&bundle) || !manifest_path.is_file() {
        bail!("filesystem MCP package manifest must be inside the provider bundle");
    }
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(manifest_path)?)?;
    if manifest.get("name").and_then(|value| value.as_str()) != Some(PROVIDER_PACKAGE)
        || manifest.get("version").and_then(|value| value.as_str())
            != Some(REVIEWED_PACKAGE_VERSION)
    {
        bail!("filesystem MCP package is outside the reviewed version");
    }
    Ok((bundle, read_root))
}

fn canonical_directory(raw: &str, label: &str) -> Result<PathBuf> {
    let path = Path::new(raw);
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        bail!("{label} must be an absolute directory without traversal");
    }
    let meta = std::fs::symlink_metadata(path).with_context(|| format!("inspect {label}"))?;
    if !meta.file_type().is_dir() {
        bail!("{label} must be a directory, not a symlink");
    }
    let canonical = path.canonicalize()
        .with_context(|| format!("canonicalize {label}"))?;
    let docker_mount_path = canonical.to_str().context("Docker mount path must be UTF-8")?;
    if docker_mount_path.contains(',') || docker_mount_path.contains('\n') {
        bail!("{label} contains a Docker mount separator");
    }
    Ok(canonical)
}

fn worker_image_id() -> Result<String> {
    let image = std::env::var(WORKER_IMAGE_ENV)
        .with_context(|| format!("{WORKER_IMAGE_ENV} must pin a local Docker image ID"))?;
    let digest = image.strip_prefix("sha256:").context("stdio MCP worker image must be a sha256 image ID")?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("stdio MCP worker image must be a sha256 image ID");
    }
    Ok(image)
}

fn worker_command(bundle: &str, read_root: &str) -> Result<(tokio::process::Command, String)> {
    let (bundle, read_root) = validate_profile(bundle, read_root)?;
    let image = worker_image_id()?;
    let process = std::fs::metadata("/proc/self").context("inspect host worker identity")?;
    let user = format!("{}:{}", process.uid(), process.gid());
    let bundle_mount = format!("type=bind,src={},dst=/provider,readonly", bundle.display());
    let data_mount = format!("type=bind,src={},dst=/data,readonly", read_root.display());
    let name = format!("restless-stdio-{}", Uuid::new_v4());
    let mut command = tokio::process::Command::new("/usr/bin/docker");
    command.env_clear();
    command.env("HOME", "/tmp").env("PATH", "/usr/bin:/bin");
    command.args([
        "run", "--rm", "-i", "--pull=never", "--network", "none",
        "--read-only", "--cap-drop", "ALL", "--security-opt", "no-new-privileges",
        "--pids-limit", "64", "--memory", "512m", "--ipc", "none",
        "--name", &name, "--label", "io.restless.stdio-worker=true",
        "--user", &user, "--tmpfs", "/tmp:rw,nosuid,nodev,size=16m",
        "--env", "HOME=/tmp", "--env", "TMPDIR=/tmp",
        "--mount", &bundle_mount, "--mount", &data_mount,
        "--entrypoint", "/usr/bin/timeout", &image,
    ]);
    command.args([
        "120s",
        "/provider/node",
        "/provider/node_modules/@modelcontextprotocol/server-filesystem/dist/index.js",
        "/data",
    ]);
    Ok((command, name))
}

/// Docker's `--rm` can leave a container in Created state when the attached
/// CLI is interrupted before start. Remove only this invocation's UUID name.
/// A failed startup needs a longer watch: its create request may reach the
/// daemon after the CLI has exited and an initial removal found nothing.
async fn cleanup_worker(name: &str, watch_late_create: bool) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(
        if watch_late_create { WORKER_LIFETIME_SECONDS + 10 } else { 5 },
    );
    let observed_absent = loop {
        let mut command = tokio::process::Command::new("/usr/bin/docker");
        command.env_clear().env("HOME", "/tmp").env("PATH", "/usr/bin:/bin");
        let removal = tokio::time::timeout(
            Duration::from_secs(8), command.args(["rm", "-f", name]).output(),
        ).await;
        let absent = match removal {
            Ok(Ok(output)) if output.status.success() => return,
            Ok(Ok(output)) if String::from_utf8_lossy(&output.stderr).contains("No such container") => {
                if !watch_late_create { return; }
                true
            }
            _ => false,
        };
        if tokio::time::Instant::now() >= deadline { break absent; }
        tokio::time::sleep(Duration::from_secs(if watch_late_create { 5 } else { 1 })).await;
    };
    if !observed_absent {
        tracing::warn!(worker = name, "exact stdio worker cleanup could not be confirmed");
    }
}

struct StderrDiagnostic {
    bytes: u64,
    class: &'static str,
}

async fn read_stderr(mut stderr: tokio::process::ChildStderr) -> StderrDiagnostic {
    let mut first = Vec::with_capacity(STDERR_DIAGNOSTIC_BYTES);
    let mut bytes = 0u64;
    let mut chunk = [0u8; 1024];
    while let Ok(count) = stderr.read(&mut chunk).await {
        if count == 0 { break; }
        bytes = bytes.saturating_add(count as u64);
        let room = STDERR_DIAGNOSTIC_BYTES.saturating_sub(first.len());
        first.extend_from_slice(&chunk[..count.min(room)]);
    }
    // The worker or Docker may write arbitrary data. Expose only fixed classes,
    // never the provider's stderr text, host paths, or listing contents.
    let sample = String::from_utf8_lossy(&first);
    let class = if sample.contains("Cannot connect to the Docker daemon") {
        "docker_unavailable"
    } else if sample.contains("No such image") {
        "image_missing"
    } else if sample.contains("Error response from daemon") {
        "docker_daemon_error"
    } else if sample.contains("Secure MCP Filesystem Server running on stdio") {
        "provider_started"
    } else if first.is_empty() {
        "empty"
    } else {
        "other"
    };
    StderrDiagnostic { bytes, class }
}

/// A fresh local-only Docker worker carries the provider for at most two
/// minutes. The image ID is immutable and must already exist on this host.
/// rmcp still owns the JSON-RPC stream; Core keeps the child handle to report
/// exit status on a failed handshake and reap it after calls.
pub(crate) async fn connect(bundle: &str, read_root: &str) -> Result<RunningService<RoleClient, ()>> {
    let (mut command, name) = worker_command(bundle, read_root)?;
    let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().context("start isolated filesystem MCP worker")?;
    let stdout = child.stdout.take().context("stdio MCP worker has no stdout")?;
    let stdin = child.stdin.take().context("stdio MCP worker has no stdin")?;
    let stderr = child.stderr.take().context("stdio MCP worker has no stderr")?;
    let stderr_task = tokio::spawn(read_stderr(stderr));
    let handshake = tokio::time::timeout(
        PROBE_TIMEOUT,
        ().serve(AsyncRwTransport::<RoleClient, _, _>::new(stdout, stdin)),
    ).await;
    match handshake {
        Ok(Ok(client)) => {
            tokio::spawn(async move {
                if tokio::time::timeout(
                    Duration::from_secs(WORKER_LIFETIME_SECONDS + 10), child.wait(),
                ).await.is_err() {
                    let _ = child.kill().await;
                }
                cleanup_worker(&name, false).await;
            });
            // Keep draining stderr so Docker cannot block on a full pipe.
            drop(stderr_task);
            Ok(client)
        }
        failed => {
            let status = match tokio::time::timeout(Duration::from_millis(300), child.wait()).await {
                Ok(Ok(status)) => status.code().map_or("signalled".to_string(), |code| code.to_string()),
                _ => {
                    let _ = child.kill().await;
                    "still_running_terminated".to_string()
                }
            };
            let diagnostic = tokio::time::timeout(Duration::from_millis(300), stderr_task).await
                .ok().and_then(|result| result.ok());
            // Keep watching after returning the error: a Docker create request
            // can complete after the attached CLI has been killed.
            tokio::spawn(async move { cleanup_worker(&name, true).await; });
            let class = diagnostic.as_ref().map_or("unavailable", |value| value.class);
            let bytes = diagnostic.as_ref().map_or(0, |value| value.bytes);
            let reason = match failed {
                Ok(Err(_)) => "protocol_or_connection_error",
                Err(_) => "startup_timeout",
                Ok(Ok(_)) => unreachable!(),
            };
            bail!("stdio MCP handshake failed: {reason}; worker_exit={status}; stderr_class={class}; stderr_bytes={bytes}");
        }
    }
}

pub(crate) async fn probe(bundle: &str, read_root: &str) -> Result<McpProbe> {
    let client = connect(bundle, read_root).await?;
    let server_version = client
        .peer_info()
        .as_ref()
        .and_then(|info| {
            info.server_info
                .as_ref()
                .map(|server| server.version.clone())
        })
        .context("stdio MCP did not provide a server version")?;
    let tools = tokio::time::timeout(PROBE_TIMEOUT, client.list_all_tools())
        .await
        .context("stdio MCP discovery timed out")??;
    let _ = client.cancel().await;
    let selected = tools
        .into_iter()
        .filter(|tool| tool.name.as_ref() == READ_TOOL)
        .collect::<Vec<_>>();
    if selected.len() != 1 {
        bail!("filesystem MCP did not expose the reviewed read_text_file tool");
    }
    let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&selected)?));
    Ok(McpProbe {
        names: vec![READ_TOOL.into()],
        digest,
        server_version,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadTextFileArgs {
    path: String,
}

/// Only one existing regular file under the owner's read root may be named.
/// The sandbox makes the entire data mount read-only and disables networking.
pub(crate) fn validated_read_params(
    params: CallToolRequestParams,
    read_root: &str,
) -> Result<CallToolRequestParams> {
    if params.name.as_ref() != READ_TOOL
        || params.meta.is_some()
        || params.input_responses.is_some()
        || params.request_state.is_some()
    {
        bail!("stdio MCP operation is outside the reviewed read profile");
    }
    let args: ReadTextFileArgs = serde_json::from_value(serde_json::Value::Object(
        params
            .arguments
            .context("stdio MCP read arguments are required")?,
    ))?;
    let path = Path::new(&args.path);
    let relative = path
        .strip_prefix("/data")
        .context("stdio MCP path must be under /data")?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        bail!("stdio MCP path must name one file under /data without traversal");
    }
    let root = canonical_directory(read_root, "read root")?;
    let file = root
        .join(relative)
        .canonicalize()
        .context("resolve requested file")?;
    let meta = std::fs::metadata(&file).context("inspect requested file")?;
    if !file.starts_with(root) || !meta.is_file() || meta.len() > MAX_FILE_BYTES {
        bail!("stdio MCP read is outside the mounted root or exceeds its size bound");
    }
    let mut arguments = serde_json::Map::new();
    arguments.insert("path".into(), serde_json::Value::String(args.path));
    Ok(CallToolRequestParams::new(READ_TOOL).with_arguments(arguments))
}
