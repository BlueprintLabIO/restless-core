//! One reviewed local stdio adapter. The filesystem provider runs under
//! bubblewrap, outside Core's process and mount/network namespaces. Its
//! package and Node binary are owner-staged in a read-only bundle.

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use rmcp::{
    model::{CallToolRequestParams, Tool},
    transport::TokioChildProcess,
    ServiceExt as _,
};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};

use crate::mcp_gateway::McpProbe;

pub(crate) const READ_TOOL: &str = "read_text_file";
const PROVIDER_PACKAGE: &str = "@modelcontextprotocol/server-filesystem";
const REVIEWED_PACKAGE_VERSION: &str = "2026.8.31";
const PROVIDER_ENTRY: &str = "node_modules/@modelcontextprotocol/server-filesystem/dist/index.js";
const PROBE_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_FILE_BYTES: u64 = 256 * 1024;

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
    path.canonicalize()
        .with_context(|| format!("canonicalize {label}"))
}

/// rmcp owns JSON-RPC framing. Each discovery or call starts a fresh
/// disposable provider; dropping the transport terminates its sandbox.
pub(crate) fn transport(bundle: &str, read_root: &str) -> Result<TokioChildProcess> {
    let (bundle, read_root) = validate_profile(bundle, read_root)?;
    let mut command = tokio::process::Command::new("/usr/bin/bwrap");
    command.env_clear();
    command.args([
        "--unshare-all",
        "--die-with-parent",
        "--new-session",
        "--clearenv",
        "--ro-bind",
        "/usr",
        "/usr",
        "--ro-bind",
        "/lib",
        "/lib",
        "--ro-bind",
        "/lib64",
        "/lib64",
        "--ro-bind",
        "/bin",
        "/bin",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--setenv",
        "HOME",
        "/tmp",
        "--setenv",
        "TMPDIR",
        "/tmp",
        "--setenv",
        "PATH",
        "/usr/bin:/bin",
        "--ro-bind",
    ]);
    command.arg(&bundle).arg("/provider").arg("--ro-bind");
    command.arg(&read_root).arg("/data");
    command.args([
        "--",
        "/provider/node",
        "/provider/node_modules/@modelcontextprotocol/server-filesystem/dist/index.js",
        "/data",
    ]);
    Ok(TokioChildProcess::builder(command)
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("start isolated filesystem MCP provider")?
        .0)
}

pub(crate) async fn probe(bundle: &str, read_root: &str) -> Result<McpProbe> {
    let child = transport(bundle, read_root)?;
    let client = tokio::time::timeout(PROBE_TIMEOUT, ().serve(child))
        .await
        .context("stdio MCP handshake timed out")??;
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
