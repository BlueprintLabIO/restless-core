//! Runtime layer: the company computer's lifecycle, driven through the docker
//! CLI (mature infrastructure over bespoke machinery, §2.6). One persistent
//! container + one named volume per company; the volume is the company home.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::task::{Context as TaskContext, Poll};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use http_body_util::Empty;
use hyper::body::{Bytes, Incoming};
use hyper::client::conn::http1;
use hyper::{HeaderMap, Method, Request, Response, Uri};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::process::{Child, ChildStdin, ChildStdout};
use uuid::Uuid;

pub const COMPANY_IMAGE: &str = "restless-company-image:latest";
const SOURCE_DIGEST_LABEL: &str = "io.restless.source-digest";

/// One company's identity and configuration, as a file — not a table (sprint
/// spec, kernel slice). Lives at `$RESTLESS_HOME/companies/<name>.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyConfig {
    /// Company name; also the container/volume suffix and schema name.
    pub name: String,
    /// Owner-set mission, seeded to /company/mission.md on `up`.
    #[serde(default)]
    pub mission: String,
    /// Per-company model spend ceiling in USD (T2). The fuse, not governance.
    #[serde(default = "default_ceiling")]
    pub spend_ceiling_usd: f64,
    /// Provider-qualified model the agent runs on, e.g. `zai/glm-5.2`.
    /// Required: there is no sensible default provider, and the adapter-model
    /// indirection this replaced (`company-general-v1` → a gateway route)
    /// was vestigial once agents named providers directly.
    pub model: String,
    /// Ordered provider-qualified fallbacks for the singleton Exec. Empty is
    /// an explicit no-fallback policy; providers are never inferred from
    /// ambient credentials or broker history.
    #[serde(default)]
    pub model_failover: Vec<String>,
    /// Named binding → `credential_reference`, e.g.
    /// `resend.production = "infisical:/companies/aris/RESEND_API_KEY"`.
    /// Only a governed child process that names the binding receives it.
    #[serde(default)]
    pub credentials: std::collections::BTreeMap<String, String>,
    /// Legacy S03 approval input. At daemon boot these values migrate into the
    /// Authority-owned governance store and this list is purged. It remains in
    /// the parser only so upgrading cannot silently discard an existing grant.
    #[serde(default)]
    pub approved_parties: Vec<String>,
}

fn default_ceiling() -> f64 {
    10.0
}

impl CompanyConfig {
    pub fn load(root: &Path, name: &str) -> Result<Self> {
        Self::load_from(root.join("companies").join(format!("{name}.toml")), name)
    }

    pub fn load_archived(root: &Path, name: &str) -> Result<Self> {
        Self::load_from(
            root.join("archived-companies").join(format!("{name}.toml")),
            name,
        )
    }

    fn load_from(path: PathBuf, name: &str) -> Result<Self> {
        validate_company_name(name)?;
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("no company config at {}", path.display()))?;
        let config: Self =
            toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))?;
        if config.name != name {
            bail!(
                "company config name mismatch: file {name}.toml says {}",
                config.name
            );
        }
        config.model_candidates()?;
        Ok(config)
    }

    /// Write the config back. Used by the bounded company/credential CLI and
    /// by one-time approval migration cleanup.
    ///
    /// Writes to a temporary file and renames, because the alternative — a
    /// truncating write interrupted midway — leaves the company with no config
    /// at all, and a company that cannot load its config cannot be woken to be
    /// told why. Rename within a directory is atomic on every filesystem we run
    /// on.
    pub fn save(root: &Path, config: &Self) -> Result<()> {
        validate_company_name(&config.name)?;
        config.model_candidates()?;
        let dir = root.join("companies");
        let path = dir.join(format!("{}.toml", config.name));
        let archived = root
            .join("archived-companies")
            .join(format!("{}.toml", config.name));
        if archived.exists() && !path.exists() {
            bail!(
                "company {} is archived; restore it instead of creating a second company with the same identity",
                config.name
            );
        }
        let temporary = dir.join(format!(".{}.toml.tmp", config.name));
        let rendered = toml::to_string_pretty(config).context("render company config")?;
        std::fs::write(&temporary, rendered)
            .with_context(|| format!("write {}", temporary.display()))?;
        std::fs::rename(&temporary, &path)
            .with_context(|| format!("replace {}", path.display()))?;
        Ok(())
    }

    /// Primary followed by the exact owner-configured fallback order. The
    /// closed validation here protects both TOML and CLI writes.
    pub fn model_candidates(&self) -> Result<Vec<&str>> {
        let mut seen = std::collections::BTreeSet::new();
        let mut candidates = Vec::with_capacity(1 + self.model_failover.len());
        for model in std::iter::once(self.model.as_str())
            .chain(self.model_failover.iter().map(String::as_str))
        {
            let Some((provider, id)) = model.split_once('/') else {
                bail!("model {model:?} must be provider-qualified, e.g. moonshot/kimi-k3");
            };
            if provider.is_empty()
                || id.is_empty()
                || !provider
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            {
                bail!("invalid provider-qualified model {model:?}");
            }
            if !seen.insert(model) {
                bail!("duplicate model candidate {model:?}");
            }
            candidates.push(model);
        }
        Ok(candidates)
    }
}

/// Names preserved outside the active config directory. Archived companies
/// remain inspectable and recoverable, but normal daemon scans cannot wake
/// them merely because their config still exists.
pub fn archived_company_names(root: &Path) -> Result<Vec<String>> {
    let directory = root.join("archived-companies");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut companies = Vec::new();
    for entry in
        std::fs::read_dir(&directory).with_context(|| format!("read {}", directory.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("toml") {
            if let Some(name) = path.file_stem().and_then(|value| value.to_str()) {
                validate_company_name(name)?;
                companies.push(name.to_string());
            }
        }
    }
    companies.sort();
    Ok(companies)
}

/// Archive is the owner-facing removal path. Stop execution, then atomically
/// move only the authority-owned identity/config marker. Runtime files,
/// OrgIntel, Authority records and spend remain in place for recovery.
pub async fn archive(root: &Path, company: &str) -> Result<String> {
    CompanyConfig::load(root, company)?;
    down(company).await?;
    move_active_config_to_archive(root, company)?;
    Ok(format!(
        "{company}: archived (runtime stopped; files and history preserved)"
    ))
}

fn move_active_config_to_archive(root: &Path, company: &str) -> Result<()> {
    CompanyConfig::load(root, company)?;
    let source = root.join("companies").join(format!("{company}.toml"));
    let directory = root.join("archived-companies");
    let destination = directory.join(format!("{company}.toml"));
    if destination.exists() {
        bail!("company {company} already has an archived config");
    }
    std::fs::create_dir_all(&directory)
        .with_context(|| format!("create archive directory {}", directory.display()))?;
    std::fs::rename(&source, &destination).with_context(|| {
        format!(
            "archive company config {} as {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

/// Return an archived identity to the active portfolio. Restoring does not
/// start the runtime: the owner can inspect it first and resume deliberately.
pub fn restore(root: &Path, company: &str) -> Result<String> {
    move_archived_config_to_active(root, company)?;
    Ok(format!(
        "{company}: restored to the portfolio (runtime remains stopped)"
    ))
}

fn move_archived_config_to_active(root: &Path, company: &str) -> Result<()> {
    CompanyConfig::load_archived(root, company)?;
    let source = root
        .join("archived-companies")
        .join(format!("{company}.toml"));
    let destination = root.join("companies").join(format!("{company}.toml"));
    if destination.exists() {
        bail!("company {company} already has an active config");
    }
    std::fs::rename(&source, &destination).with_context(|| {
        format!(
            "restore company config {} as {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn validate_company_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 63
        || !name.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                byte.is_ascii_lowercase()
            } else {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
            }
        })
    {
        bail!("invalid company name {name:?}: use lowercase letters, digits or underscores, starting with a letter");
    }
    Ok(())
}

pub fn container_name(company: &str) -> String {
    format!("restless-co-{company}")
}

pub fn volume_name(company: &str) -> String {
    format!("restless-vol-{company}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ContainerStatus {
    Running,
    Stopped,
    Absent,
}

/// Whether the running company computer is built from the current Restless
/// source. `Unknown` is not collapsed into `Current`: a missing source tree or
/// an unlabelled old image is precisely the version-skew case `doctor` exists
/// to make visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationStatus {
    Current,
    Required,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct RuntimeDoctor {
    pub company: String,
    pub container: ContainerStatus,
    pub image: String,
    pub container_image_id: Option<String>,
    pub target_image_id: Option<String>,
    pub source_digest: Option<String>,
    pub image_source_digest: Option<String>,
    pub reconciliation: ReconciliationStatus,
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser: Option<BrowserDoctor>,
}

#[derive(Debug, Serialize)]
pub struct BrowserDoctor {
    pub status: String,
    pub desktop: String,
    pub chromium: String,
    pub automation: String,
    pub web_transport: String,
    pub controller: String,
}

async fn docker(args: &[&str]) -> Result<std::process::Output> {
    tokio::process::Command::new("docker")
        .args(args)
        .output()
        .await
        .context("spawn docker")
}

pub async fn status(company: &str) -> Result<ContainerStatus> {
    let name = container_name(company);
    let out = docker(&["inspect", "-f", "{{.State.Status}}", &name]).await?;
    if !out.status.success() {
        return Ok(ContainerStatus::Absent);
    }
    let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(match state.as_str() {
        "running" => ContainerStatus::Running,
        _ => ContainerStatus::Stopped,
    })
}

/// Create if absent, start if stopped, no-op if running. With `reconcile`,
/// rebuild the versioned company image from the current Restless source and
/// replace an outdated container while keeping its named volume.
pub async fn up(config: &CompanyConfig, reconcile: bool) -> Result<String> {
    let company = &config.name;
    let mut rebuilt = false;
    let mut replaced = false;
    if reconcile {
        rebuilt = ensure_current_image().await?;
        if status(company).await? != ContainerStatus::Absent
            && container_uses_old_image(company).await?
        {
            let name = container_name(company);
            // Give supervised Chromium long enough to flush its persistent
            // profile before replacing the shell. Docker's ten-second default
            // is shorter than Chromium's configured 20-second stop window and
            // was observed dropping a persistent cookie during reconciliation.
            if status(company).await? == ContainerStatus::Running {
                run_ok(&["stop", "--time", "30", &name]).await?;
            }
            // Only the replaceable container is removed; the named company
            // volume remains the durable computer (§13.4).
            run_ok(&["rm", &name]).await?;
            replaced = true;
        }
    }
    match status(company).await? {
        ContainerStatus::Running => {}
        ContainerStatus::Stopped => {
            let name = container_name(company);
            run_ok(&["start", &name]).await?;
        }
        ContainerStatus::Absent => {
            let volume = volume_name(company);
            run_ok(&["volume", "create", &volume]).await?;
            let name = container_name(company);
            run_ok(&[
                "run",
                "-d",
                "--name",
                &name,
                "--hostname",
                company,
                "-e",
                &format!("RESTLESS_COMPANY={company}"),
                "-v",
                &format!("{volume}:/company"),
                COMPANY_IMAGE,
            ])
            .await?;
        }
    }
    seed_mission(config).await?;
    let suffix = match (rebuilt, replaced) {
        (true, true) => " (image rebuilt; container replaced; volume kept)",
        (true, false) => " (image rebuilt)",
        (false, true) => " (container replaced; volume kept)",
        (false, false) if reconcile => " (runtime already current)",
        (false, false) => "",
    };
    Ok(format!("{}: running{suffix}", config.name))
}

/// Check the replaceable runtime image independently of an agent report.
pub async fn doctor(company: &str) -> Result<RuntimeDoctor> {
    let container = status(company).await?;
    let container_image_id = if container == ContainerStatus::Absent {
        None
    } else {
        inspect_value(&["inspect", "-f", "{{.Image}}", &container_name(company)]).await?
    };
    let target_image_id =
        inspect_value(&["image", "inspect", "-f", "{{.Id}}", COMPANY_IMAGE]).await?;
    let image_source_digest = inspect_value(&[
        "image",
        "inspect",
        "-f",
        &format!("{{{{index .Config.Labels \"{SOURCE_DIGEST_LABEL}\"}}}}"),
        COMPANY_IMAGE,
    ])
    .await?
    .filter(|value| value != "<no value>");
    let source_digest = source_root()
        .ok()
        .and_then(|root| digest_source(&root).ok());

    let runtime_missing_or_stale = container == ContainerStatus::Absent
        || matches!(
            (&container_image_id, &target_image_id),
            (Some(container_id), Some(target_id)) if container_id != target_id
        )
        || matches!(
            (&source_digest, &image_source_digest),
            (Some(source), Some(image)) if source != image
        );
    let reconciliation = if runtime_missing_or_stale {
        ReconciliationStatus::Required
    } else if container_image_id.is_some()
        && target_image_id.is_some()
        && source_digest.is_some()
        && image_source_digest.is_some()
    {
        ReconciliationStatus::Current
    } else {
        ReconciliationStatus::Unknown
    };

    let browser = if container == ContainerStatus::Running {
        Some(browser_doctor(company).await)
    } else {
        None
    };

    Ok(RuntimeDoctor {
        company: company.to_string(),
        container,
        image: COMPANY_IMAGE.to_string(),
        container_image_id,
        target_image_id,
        source_digest,
        image_source_digest,
        reconciliation,
        action: (reconciliation != ReconciliationStatus::Current)
            .then(|| format!("restless up -c {company} --reconcile")),
        browser,
    })
}

/// The container id is the V0 Runtime generation: it changes when the
/// replaceable shell changes and stays stable across ordinary process restarts.
pub async fn generation(company: &str) -> Result<Option<String>> {
    inspect_value(&["inspect", "-f", "{{.Id}}", &container_name(company)]).await
}

async fn browser_doctor(company: &str) -> BrowserDoctor {
    let name = container_name(company);
    let process = async |program: &str| -> String {
        let output = docker(&[
            "exec",
            &name,
            "supervisorctl",
            "-c",
            "/etc/supervisor/conf.d/restless.conf",
            "status",
            program,
        ])
        .await;
        match output {
            Ok(output) if output.status.success() => {
                let line = String::from_utf8_lossy(&output.stdout);
                if line.contains("RUNNING") {
                    "available".into()
                } else {
                    "degraded".into()
                }
            }
            _ => "unavailable".into(),
        }
    };
    let desktop = process("desktop").await;
    let chromium = process("chromium").await;
    let web_transport = process("desktop-web").await;
    let automation: String = match docker(&[
        "exec",
        &name,
        "curl",
        // Preserve the broker's structured 423 body. `--fail` discarded it,
        // so an intentional owner pause was misreported as a broken browser.
        "--fail-with-body",
        "--silent",
        "--max-time",
        "2",
        "http://127.0.0.1:9223/json/version",
    ])
    .await
    {
        Ok(output) if output.status.success() => "available".into(),
        Ok(output) if String::from_utf8_lossy(&output.stdout).contains("owner_controls") => {
            "owner_paused".into()
        }
        _ => "unavailable".into(),
    };
    let controller = read_browser_control(company)
        .await
        .ok()
        .flatten()
        .and_then(|value| value["controller"].as_str().map(str::to_string))
        .unwrap_or_else(|| "unclaimed".into());
    let status = if [&desktop, &chromium, &web_transport]
        .iter()
        .all(|part| part.as_str() == "available")
        && matches!(automation.as_str(), "available" | "owner_paused")
    {
        "available"
    } else {
        "degraded"
    }
    .into();
    BrowserDoctor {
        status,
        desktop,
        chromium,
        automation,
        web_transport,
        controller,
    }
}

/// Fetch one noVNC asset through an ephemeral Runtime Bridge process. The
/// desktop service remains bound inside the company computer; there is no host
/// port that can bypass owner authentication.
pub async fn desktop_asset(company: &str, asset: &str) -> Result<Vec<u8>> {
    if asset.is_empty() || asset.contains("..") || asset.contains('\0') {
        bail!("invalid desktop asset path");
    }
    let url = format!("http://127.0.0.1:6080/{asset}");
    let output = docker(&[
        "exec",
        &container_name(company),
        "curl",
        "--fail",
        "--silent",
        "--show-error",
        "--max-time",
        "5",
        &url,
    ])
    .await?;
    if !output.status.success() {
        bail!(
            "desktop asset unavailable: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

/// A full-duplex byte stream backed by `docker exec socat`. This is the V0
/// Runtime Bridge for the private desktop transport: mature process tooling,
/// not a published port or a browser-action protocol.
pub struct DesktopStream {
    _child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

/// A reviewed web outcome is an ordinary HTTP project service inside the
/// company computer. Only loopback HTTP services are eligible, and the
/// desktop/browser control ports are deliberately excluded: review is a
/// read-only outcome surface, never a second way around the browser handoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHttpTarget {
    pub port: u16,
    pub path_and_query: String,
}

pub fn runtime_http_target(value: &str) -> Result<RuntimeHttpTarget> {
    let url = url::Url::parse(value).context("parse runtime review URL")?;
    if url.scheme() != "http"
        || !matches!(url.host_str(), Some("127.0.0.1" | "localhost"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        bail!("review target must be an uncredentialed loopback http URL");
    }
    let port = url.port_or_known_default().unwrap_or(80);
    if matches!(port, 5901 | 6080 | 9222 | 9223) {
        bail!("review target names a reserved browser/desktop port");
    }
    let mut path_and_query = url.path().to_string();
    if path_and_query.is_empty() {
        path_and_query.push('/');
    }
    if let Some(query) = url.query() {
        path_and_query.push('?');
        path_and_query.push_str(query);
    }
    Ok(RuntimeHttpTarget {
        port,
        path_and_query,
    })
}

/// Issue one GET/HEAD over a private `docker exec socat` stream. The project
/// service remains unpublished; the owner gateway is the only host-side
/// transport and decides which headers and methods may cross it.
pub async fn runtime_http_request(
    company: &str,
    port: u16,
    method: Method,
    path_and_query: &str,
    headers: &HeaderMap,
) -> Result<Response<Incoming>> {
    if !matches!(method, Method::GET | Method::HEAD) {
        bail!("runtime review transport is read-only");
    }
    let uri: Uri = path_and_query
        .parse()
        .context("parse runtime request path")?;
    if uri.scheme().is_some() || uri.authority().is_some() {
        bail!("runtime request must use an origin-relative path");
    }
    let stream = private_tcp_stream(company, port).await?;
    let (mut sender, connection) = http1::handshake(TokioIo::new(stream))
        .await
        .context("open runtime HTTP connection")?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            tracing::debug!("runtime review HTTP connection ended: {error}");
        }
    });

    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("host", format!("127.0.0.1:{port}"))
        .header("connection", "close");
    for name in [
        hyper::header::ACCEPT,
        hyper::header::ACCEPT_LANGUAGE,
        hyper::header::IF_MODIFIED_SINCE,
        hyper::header::IF_NONE_MATCH,
        hyper::header::RANGE,
    ] {
        if let Some(value) = headers.get(&name) {
            request = request.header(name, value);
        }
    }
    sender
        .send_request(request.body(Empty::<Bytes>::new())?)
        .await
        .context("request runtime web outcome")
}

pub async fn probe_runtime_http(company: &str, value: &str) -> Result<()> {
    let target = runtime_http_target(value)?;
    let response = runtime_http_request(
        company,
        target.port,
        Method::HEAD,
        &target.path_and_query,
        &HeaderMap::new(),
    )
    .await?;
    if response.status().is_success() || response.status().is_redirection() {
        Ok(())
    } else {
        bail!("runtime review target returned {}", response.status())
    }
}

impl AsyncRead for DesktopStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdout).poll_read(cx, buffer)
    }
}

impl AsyncWrite for DesktopStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.stdin).poll_write(cx, buffer)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdin).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdin).poll_shutdown(cx)
    }
}

pub async fn desktop_stream(company: &str) -> Result<DesktopStream> {
    private_tcp_stream(company, 6080).await
}

async fn private_tcp_stream(company: &str, port: u16) -> Result<DesktopStream> {
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            &container_name(company),
            "socat",
            "STDIO",
            &format!("TCP:127.0.0.1:{port}"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("open private desktop bridge")?;
    let stdin = child.stdin.take().context("desktop bridge stdin")?;
    let stdout = child.stdout.take().context("desktop bridge stdout")?;
    Ok(DesktopStream {
        _child: child,
        stdin,
        stdout,
    })
}

pub async fn read_browser_control(company: &str) -> Result<Option<serde_json::Value>> {
    let name = container_name(company);
    let output = docker(&[
        "exec",
        &name,
        "sh",
        "-c",
        "test -f /company/run/browser-control.json && cat /company/run/browser-control.json",
    ])
    .await?;
    if !output.status.success() || output.stdout.is_empty() {
        return Ok(None);
    }
    let state = serde_json::from_slice(&output.stdout).context("parse browser controller state")?;
    Ok(Some(normalize_expired_browser_control(state)))
}

/// An owner lease is bounded even if the SPA vanishes without hand-back.
/// The Runtime file is reconstructable coordination, not durable truth, so a
/// reader projects an expired owner back to the requesting agent (or
/// unclaimed rescue state). The browser broker independently uses the same
/// expiry to reopen CDP; keeping the health projection stale at `owner` while
/// automation had resumed was precisely the split-brain this lease prevents.
fn normalize_expired_browser_control(mut state: serde_json::Value) -> serde_json::Value {
    if state["controller"] != "owner" {
        return state;
    }
    let Some(expires_at) = state["expires_at"].as_str().map(str::to_string) else {
        return state;
    };
    let Ok(expires) = DateTime::parse_from_rfc3339(&expires_at) else {
        return state;
    };
    if expires.with_timezone(&Utc) > Utc::now() {
        return state;
    }

    let requester = state["requester"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    state = match requester {
        Some(session_id) => serde_json::json!({
            "controller": "agent",
            "session_id": session_id,
            "reason": "owner_lease_expired",
            "expired_at": expires_at,
        }),
        None => serde_json::json!({
            "controller": "unclaimed",
            "reason": "owner_lease_expired",
            "expired_at": expires_at,
        }),
    };
    state
}

/// Replace the reconstructable lease atomically inside the persistent Runtime.
/// This is deterministic process coordination, not an Authority effect.
pub async fn write_browser_control(company: &str, state: &serde_json::Value) -> Result<()> {
    let name = container_name(company);
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            "-u",
            "company",
            &name,
            "sh",
            "-c",
            "umask 077; cat > /company/run/browser-control.json.tmp && mv /company/run/browser-control.json.tmp /company/run/browser-control.json",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .context("write browser controller state")?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin
        .write_all(serde_json::to_string(state)?.as_bytes())
        .await?;
    drop(stdin);
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        bail!(
            "write browser controller state failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Build only when the image's source label differs. Docker remains the
/// mature V0 lifecycle mechanism; the owner asks Restless to reconcile and
/// never has to know the build invocation.
async fn ensure_current_image() -> Result<bool> {
    let root = source_root()?;
    let digest = digest_source(&root)?;
    let labelled = inspect_value(&[
        "image",
        "inspect",
        "-f",
        &format!("{{{{index .Config.Labels \"{SOURCE_DIGEST_LABEL}\"}}}}"),
        COMPANY_IMAGE,
    ])
    .await?
    .filter(|value| value != "<no value>");
    if labelled.as_deref() == Some(digest.as_str()) {
        return Ok(false);
    }

    let dockerfile = root.join("infra/company-image/Dockerfile");
    let output = tokio::process::Command::new("docker")
        .arg("build")
        .arg("--quiet")
        .arg("--file")
        .arg(&dockerfile)
        .arg("--tag")
        .arg(COMPANY_IMAGE)
        .arg("--label")
        .arg(format!("{SOURCE_DIGEST_LABEL}={digest}"))
        .arg(&root)
        .output()
        .await
        .with_context(|| format!("build company image from {}", root.display()))?;
    if !output.status.success() {
        bail!(
            "company image build failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(true)
}

async fn container_uses_old_image(company: &str) -> Result<bool> {
    let name = container_name(company);
    let container_id = inspect_value(&["inspect", "-f", "{{.Image}}", &name]).await?;
    let target_id = inspect_value(&["image", "inspect", "-f", "{{.Id}}", COMPANY_IMAGE]).await?;
    Ok(match (container_id, target_id) {
        (Some(container_id), Some(target_id)) => container_id != target_id,
        // If the target disappeared after a successful build, state is
        // unknowable and replacement would be a guess.
        (_, None) => bail!("company image {COMPANY_IMAGE} is unavailable after reconciliation"),
        (None, Some(_)) => true,
    })
}

async fn inspect_value(args: &[&str]) -> Result<Option<String>> {
    let output = docker(args).await?;
    if !output.status.success() {
        return Ok(None);
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!value.is_empty()).then_some(value))
}

/// The local source tree is the V0 image source. An explicit environment
/// override supports a daemon launched outside the repository; otherwise walk
/// upward from its working directory. We do not silently build from an
/// arbitrary directory.
pub(crate) fn source_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("RESTLESS_SOURCE_ROOT") {
        let root = PathBuf::from(root);
        validate_source_root(&root)?;
        return Ok(root);
    }
    let mut cursor = std::env::current_dir().context("read daemon working directory")?;
    loop {
        if validate_source_root(&cursor).is_ok() {
            return Ok(cursor);
        }
        if !cursor.pop() {
            break;
        }
    }
    let compiled = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("resolve compiled Restless source root")?;
    validate_source_root(&compiled)?;
    Ok(compiled)
}

fn validate_source_root(root: &Path) -> Result<()> {
    if !root.join("Cargo.toml").is_file() || !root.join("infra/company-image/Dockerfile").is_file()
    {
        bail!(
            "{} is not a Restless source tree (set RESTLESS_SOURCE_ROOT)",
            root.display()
        );
    }
    Ok(())
}

fn digest_source(root: &Path) -> Result<String> {
    let mut files = Vec::new();
    for relative in ["Cargo.toml", "Cargo.lock", "crates", "infra/company-image"] {
        collect_files(&root.join(relative), &mut files)?;
    }
    files.sort();
    let mut digest = Sha256::new();
    for path in files {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        digest.update(relative.to_string_lossy().as_bytes());
        digest.update([0]);
        digest.update(
            std::fs::read(&path).with_context(|| format!("read image input {}", path.display()))?,
        );
        digest.update([0]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("inspect image input {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("image input may not be a symlink: {}", path.display());
    }
    if metadata.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }
    let mut children: Vec<PathBuf> = std::fs::read_dir(path)
        .with_context(|| format!("read image input directory {}", path.display()))?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|entry| entry.path())
        .collect();
    children.sort();
    for child in children {
        collect_files(&child, files)?;
    }
    Ok(())
}

/// Stop the container. The volume — files, Git history, browser profile —
/// survives (§5, §17 step 2: the persistent company computer).
pub async fn down(company: &str) -> Result<String> {
    match status(company).await? {
        ContainerStatus::Running => {
            let name = container_name(company);
            // Chromium's supervisor stop window is 20 seconds. Use a longer
            // container deadline so cookies and profile state reach disk
            // before Docker escalates to SIGKILL.
            run_ok(&["stop", "--time", "30", &name]).await?;
            Ok(format!("{company}: stopped (volume kept)"))
        }
        ContainerStatus::Stopped => Ok(format!("{company}: already stopped")),
        ContainerStatus::Absent => Ok(format!("{company}: no container")),
    }
}

/// S04-T1. Remove a throwaway company entirely: container, volume, OrgIntel
/// schema **and spend spool**.
///
/// The spend spool is the part that looks optional and is not. The sprint-02
/// comparison harness reset container, volume and schema between its three
/// arms and never the spool, so the arms ran with $2.45 / $10.51 / $12.85 of
/// headroom against a nominal $15 ceiling — three "identical" runs that were
/// not comparable, and nobody could see it. Destroying three of four states is
/// how you get a clean-looking run with a hidden variable in it.
pub async fn destroy(
    root: &Path,
    company: &str,
    org: &restless_orgintel::OrgIntel,
    spend: &crate::spend::SpendLedger,
) -> Result<String> {
    let mut removed = Vec::new();

    if status(company).await? != ContainerStatus::Absent {
        let name = container_name(company);
        // `rm -f` covers running and stopped in one call; stopping first would
        // leave a window where a crash strands the container.
        run_ok(&["rm", "-f", &name]).await?;
        removed.push("container");
    }

    let volume = volume_name(company);
    if docker(&["volume", "inspect", &volume])
        .await?
        .status
        .success()
    {
        run_ok(&["volume", "rm", &volume]).await?;
        removed.push("volume");
    }

    // `drop_schema` has existed unused since sprint 01 and was nearly deleted
    // in the sprint-02 purge. Ephemeral companies are why it exists.
    org.drop_schema().await.context("drop orgintel schema")?;
    removed.push("schema");

    // The spool is ONE shared file, not one per company (`spend.jsonl`), so
    // this cannot be a file deletion — an earlier version of this function
    // removed `spend/<company>.jsonl`, a path that has never existed, and so
    // silently left every destroyed company's spend accounted. That is the
    // sprint-02 defect verbatim.
    spend
        .forget(company)
        .context("clearing the destroyed company's spend")?;
    removed.push("spend");

    // The config last: while it exists, the company is nameable and the earlier
    // steps are re-runnable if one of them failed.
    let config = root.join("companies").join(format!("{company}.toml"));
    if config.exists() {
        std::fs::remove_file(&config)
            .with_context(|| format!("remove config {}", config.display()))?;
        removed.push("config");
    }

    Ok(format!("{company}: destroyed ({})", removed.join(", ")))
}

/// Is this a throwaway company? The name is the marker, so the property is
/// visible in every log line, schema name and container name without anyone
/// looking up config (`S03-T7`: underscores, because a company name becomes a
/// Postgres schema name and `aris-test` is rejected at creation).
pub fn is_test_company(company: &str) -> bool {
    company.ends_with("_test")
}

/// S04-T1. Clone a live company's mission and configuration under a new name,
/// with every live credential, approval and external-effect identity stripped.
///
/// The guarantee is structural, not a rule someone remembers: a `_test`
/// company receives no live secret bindings, so a scenario must install fake
/// CLIs. We contaminated a live company's beliefs with
/// a synthetic webhook once already — "the strongest single demand signal so
/// far" turned out to be us — and that happened because the live company was
/// the only one available to try things on.
pub fn clone_config(root: &Path, from: &str, to: &str) -> Result<CompanyConfig> {
    if !is_test_company(to) {
        bail!(
            "refusing to clone into {to:?}: a cloned company must be a throwaway, \
             and a throwaway's name must end in `_test` so it is visible everywhere"
        );
    }
    let source =
        CompanyConfig::load(root, from).with_context(|| format!("load source company {from}"))?;
    let config = CompanyConfig {
        name: to.to_string(),
        // A scenario gets no live secret bindings.
        credentials: std::collections::BTreeMap::new(),
        model_failover: Vec::new(),
        // Standing approvals are the owner's blessing of a *live* company's
        // counterparties. They do not travel to a throwaway.
        approved_parties: Vec::new(),
        ..source
    };
    CompanyConfig::save(root, &config)?;
    Ok(config)
}

async fn seed_mission(config: &CompanyConfig) -> Result<()> {
    let name = container_name(&config.name);
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            &name,
            "sh",
            "-c",
            "cat > /company/mission.md && chown company:company /company/mission.md",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .context("spawn docker exec for mission seed")?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin.write_all(config.mission.as_bytes()).await?;
    drop(stdin);
    let out = child.wait_with_output().await?;
    if !out.status.success() {
        bail!(
            "mission seed failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

/// Put an owner-supplied attachment on the persistent company computer as an
/// ordinary file. The UUID is generated by the daemon; user filenames never
/// become path components. Metadata is a sidecar so the authenticated owner
/// download can return the original name and media type without inventing an
/// asset-custody service.
pub async fn store_owner_attachment(
    company: &str,
    attachment_id: Uuid,
    bytes: &[u8],
    metadata: &[u8],
) -> Result<String> {
    validate_company_name(company)?;
    if status(company).await? != ContainerStatus::Running {
        bail!("the company computer is not running; attachments need its persistent filesystem");
    }
    let name = container_name(company);
    let directory = format!("/company/inbox/owner-attachments/{attachment_id}");
    let content_path = format!("{directory}/content");
    let metadata_path = format!("{directory}/metadata.json");
    write_container_file(&name, &directory, &content_path, bytes).await?;
    if let Err(error) = write_container_file(&name, &directory, &metadata_path, metadata).await {
        let _ = remove_owner_attachment(company, attachment_id).await;
        return Err(error);
    }
    Ok(content_path)
}

async fn write_container_file(
    container: &str,
    directory: &str,
    path: &str,
    bytes: &[u8],
) -> Result<()> {
    let command = format!(
        "umask 077; mkdir -p {directory}; cat > {path}; chown -R company:company {directory}"
    );
    let mut child = tokio::process::Command::new("docker")
        .args(["exec", "-i", container, "sh", "-c", &command])
        .stdin(Stdio::piped())
        .spawn()
        .context("spawn docker exec for owner attachment")?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin.write_all(bytes).await?;
    drop(stdin);
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        bail!(
            "store owner attachment failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Read one attachment and its small sidecar. Only a parsed UUID reaches the
/// fixed path template, so this cannot become an arbitrary company-file read.
pub async fn read_owner_attachment(
    company: &str,
    attachment_id: Uuid,
) -> Result<(Vec<u8>, Vec<u8>)> {
    validate_company_name(company)?;
    let name = container_name(company);
    let directory = format!("/company/inbox/owner-attachments/{attachment_id}");
    let content = docker(&["exec", &name, "cat", &format!("{directory}/content")]).await?;
    if !content.status.success() {
        bail!("attachment is absent from the company computer");
    }
    let metadata = docker(&["exec", &name, "cat", &format!("{directory}/metadata.json")]).await?;
    if !metadata.status.success() {
        bail!("attachment metadata is absent from the company computer");
    }
    Ok((content.stdout, metadata.stdout))
}

/// Roll back files whose message could not be recorded. The target is one
/// daemon-generated UUID directory beneath the fixed attachment root.
pub async fn remove_owner_attachment(company: &str, attachment_id: Uuid) -> Result<()> {
    validate_company_name(company)?;
    let name = container_name(company);
    let path = format!("/company/inbox/owner-attachments/{attachment_id}");
    let output = docker(&["exec", &name, "rm", "-r", "--", &path]).await?;
    if !output.status.success() {
        bail!("roll back owner attachment failed");
    }
    Ok(())
}

async fn run_ok(args: &[&str]) -> Result<()> {
    let out = docker(args).await?;
    if !out.status.success() {
        bail!(
            "docker {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// The state root: $RESTLESS_HOME or ~/.restless.
pub fn state_root() -> PathBuf {
    if let Ok(root) = std::env::var("RESTLESS_HOME") {
        return PathBuf::from(root);
    }
    let home = std::env::var("HOME").expect("HOME");
    PathBuf::from(home).join(".restless")
}

#[cfg(test)]
mod tests {
    use super::{
        move_active_config_to_archive, move_archived_config_to_active,
        normalize_expired_browser_control, runtime_http_target, CompanyConfig,
    };

    #[test]
    fn an_expired_owner_lease_returns_to_its_requester() {
        let state = serde_json::json!({
            "controller": "owner",
            "client_id": "owner-tab",
            "requester": "exec/session-7",
            "expires_at": "2000-01-01T00:00:00Z",
        });
        let normalized = normalize_expired_browser_control(state);
        assert_eq!(normalized["controller"], "agent");
        assert_eq!(normalized["session_id"], "exec/session-7");
        assert_eq!(normalized["reason"], "owner_lease_expired");
    }

    #[test]
    fn a_live_owner_lease_stays_exclusive() {
        let state = serde_json::json!({
            "controller": "owner",
            "client_id": "owner-tab",
            "requester": "exec/session-7",
            "expires_at": "2999-01-01T00:00:00Z",
        });
        assert_eq!(normalize_expired_browser_control(state.clone()), state);
    }

    #[test]
    fn model_policy_preserves_order_and_rejects_duplicates() {
        let mut config: CompanyConfig = toml::from_str(
            r#"name = "policy_test"
mission = "test"
model = "moonshot/kimi-k3"
model_failover = ["anthropic/claude-haiku-4-5", "zai/glm-5"]
"#,
        )
        .unwrap();
        assert_eq!(
            config.model_candidates().unwrap(),
            vec![
                "moonshot/kimi-k3",
                "anthropic/claude-haiku-4-5",
                "zai/glm-5"
            ]
        );
        config.model_failover.push("moonshot/kimi-k3".into());
        assert!(config.model_candidates().is_err());
    }

    #[test]
    fn review_targets_are_loopback_http_but_never_browser_control() {
        let target =
            runtime_http_target("http://127.0.0.1:4173/for-tutoring-centres?language=en").unwrap();
        assert_eq!(target.port, 4173);
        assert_eq!(target.path_and_query, "/for-tutoring-centres?language=en");
        for refused in [
            "https://127.0.0.1:4173/",
            "http://example.com:4173/",
            "http://localhost:6080/vnc.html",
            "http://127.0.0.1:9223/json/version",
            "http://user:secret@localhost:4173/",
        ] {
            assert!(runtime_http_target(refused).is_err(), "accepted {refused}");
        }
    }

    #[test]
    fn archived_identity_moves_out_of_active_scans_and_restores_without_data_rewrite() {
        let root = std::env::temp_dir().join(format!(
            "restless-archive-contract-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        let config = CompanyConfig {
            name: "archive_contract_test".into(),
            mission: "Preserve me".into(),
            spend_ceiling_usd: 5.0,
            model: "moonshot/kimi-k3".into(),
            model_failover: Vec::new(),
            credentials: std::collections::BTreeMap::new(),
            approved_parties: Vec::new(),
        };
        CompanyConfig::save(&root, &config).unwrap();

        move_active_config_to_archive(&root, &config.name).unwrap();
        assert!(CompanyConfig::load(&root, &config.name).is_err());
        let archived = CompanyConfig::load_archived(&root, &config.name).unwrap();
        assert_eq!(archived.mission, "Preserve me");
        assert!(CompanyConfig::save(&root, &config).is_err());

        move_archived_config_to_active(&root, &config.name).unwrap();
        assert!(CompanyConfig::load_archived(&root, &config.name).is_err());
        assert_eq!(
            CompanyConfig::load(&root, &config.name).unwrap().mission,
            "Preserve me"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }
}
