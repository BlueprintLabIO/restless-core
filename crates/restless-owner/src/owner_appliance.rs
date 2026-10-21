//! The owner's hand on the appliance this computer runs: its release, its backups, and the
//! repairs its banners name. Nothing here reimplements an appliance operation. Each action runs
//! the same `restless appliance` command the terminal does, as its own detached job, because a
//! backup, rollback or update stops and restarts this very plane; the cockpit reads the job's
//! outcome when the plane is back. Resume is the one in-process act: it is the daemon's own
//! drain barrier, the exact handler `restless appliance resume` asks the daemon to run.
//!
//! Only the installed (stable) appliance on this computer, and only its owner: the membership
//! boundary refuses everyone else on these non-company routes, and a hosted, development or
//! test plane refuses the operations outright.

use super::*;

const JOB: &str = "machine/appliance-job.json";
/// Written by `restless appliance backup` on success, wherever the archive went.
const LAST_BACKUP: &str = "machine/last-backup.json";
/// Where the cockpit's own backups go. Inside `machine/`, which backups never copy.
const BACKUPS: &str = "machine/backups";
/// A job with no outcome after this long is treated as gone (its machine restarted).
const JOB_HORIZON_SECONDS: u64 = 6 * 60 * 60;

#[derive(Debug, Serialize, Deserialize)]
struct ApplianceJob {
    op: String,
    started_at_unix_seconds: u64,
    log: PathBuf,
    exit: PathBuf,
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

fn refuse(message: impl Into<String>) -> Response<Body> {
    api_error(StatusCode::CONFLICT, "appliance", message)
}

/// The installed appliance on this computer, or why its lifecycle is not the cockpit's to run.
pub(super) fn local_appliance(state: &OwnerState) -> std::result::Result<(), Response<Body>> {
    if state.entry.network().is_some() {
        return Err(refuse("A hosted plane is run by its host, not from here."));
    }
    match restless_contracts::appliance::MachineProfile::from_env() {
        Ok(profile) if profile.kind == restless_contracts::appliance::ProfileKind::Stable => Ok(()),
        Ok(profile) => Err(refuse(format!(
            "This is a {} profile; only the installed appliance is managed from here.",
            profile.kind.as_str()
        ))),
        Err(error) => Err(refuse(format!("The machine profile is invalid: {error:#}"))),
    }
}

/// The `restless` command beside this plane's own binary, as every release installs it.
pub(super) fn cli_beside_plane() -> Result<PathBuf> {
    let plane = std::env::current_exe().context("locate the running plane")?;
    let cli = plane.with_file_name("restless");
    if !cli.is_file() {
        bail!(
            "the restless command is not installed beside this plane at {}",
            cli.display()
        );
    }
    Ok(cli)
}

/// The job running now, or the last one with its outcome.
pub(super) fn read_job(root: &std::path::Path) -> Option<serde_json::Value> {
    let job: ApplianceJob = serde_json::from_slice(&std::fs::read(root.join(JOB)).ok()?).ok()?;
    let code = std::fs::read_to_string(&job.exit)
        .ok()
        .and_then(|text| text.trim().parse::<i32>().ok());
    let stale = code.is_none()
        && unix_now().saturating_sub(job.started_at_unix_seconds) > JOB_HORIZON_SECONDS;
    let state = match code {
        Some(0) => "succeeded",
        Some(_) => "failed",
        None if stale => "unknown",
        None => "running",
    };
    // A failure says why in the command's own last words.
    let detail = (state == "failed").then(|| {
        std::fs::read_to_string(&job.log)
            .map(|text| {
                let lines: Vec<&str> = text
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .collect();
                lines[lines.len().saturating_sub(12)..].join("\n")
            })
            .unwrap_or_default()
    });
    Some(serde_json::json!({
        "op": job.op,
        "state": state,
        "started_at_unix_seconds": job.started_at_unix_seconds,
        "detail": detail,
    }))
}

pub(super) fn job_running(root: &std::path::Path) -> bool {
    read_job(root).is_some_and(|job| job["state"] == "running")
}

/// Start `restless <args>` as a job that outlives this plane's restart. Under systemd it is a
/// transient user unit, outside the plane's own cgroup; under launchd a new process group.
pub(super) fn spawn_job(
    root: &std::path::Path,
    op: &str,
    cli: &std::path::Path,
    args: &[String],
) -> Result<()> {
    let started = unix_now();
    let logs = root.join("logs");
    std::fs::create_dir_all(&logs)?;
    std::fs::create_dir_all(root.join("machine"))?;
    let log = logs.join(format!("appliance-{op}-{started}.log"));
    let exit = root.join(format!("machine/appliance-job-{started}.exit"));
    const SCRIPT: &str = r#"cli="$1"; log="$2"; exit_file="$3"; shift 3; "$cli" "$@" >"$log" 2>&1; echo $? >"$exit_file""#;
    let mut wrapped: Vec<String> = vec![
        "sh".into(),
        "-c".into(),
        SCRIPT.into(),
        "restless-appliance-job".into(),
        cli.display().to_string(),
        log.display().to_string(),
        exit.display().to_string(),
    ];
    wrapped.extend(args.iter().cloned());
    let job = ApplianceJob {
        op: op.to_string(),
        started_at_unix_seconds: started,
        log: log.clone(),
        exit: exit.clone(),
    };
    std::fs::write(root.join(JOB), serde_json::to_vec_pretty(&job)?)?;
    if cfg!(target_os = "linux") {
        let mut command = std::process::Command::new("systemd-run");
        command.args([
            "--user",
            "--collect",
            "--quiet",
            &format!("--unit=restless-appliance-{op}-{started}"),
        ]);
        if let Some(path) = std::env::var_os("PATH") {
            command.arg(format!("--setenv=PATH={}", path.to_string_lossy()));
        }
        let status = command
            .arg("--")
            .args(&wrapped)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .context("start the appliance job under systemd")?;
        if !status.success() {
            let _ = std::fs::remove_file(root.join(JOB));
            bail!("systemd did not start the appliance job ({status})");
        }
    } else {
        use std::os::unix::process::CommandExt as _;
        let mut child = std::process::Command::new(&wrapped[0])
            .args(&wrapped[1..])
            .process_group(0)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("start the appliance job")?;
        // Reap it if this plane is still here when it ends.
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
    Ok(())
}

/// The release this computer runs and the one it can roll back to, read through
/// `restless appliance status`; the last backup; and any appliance job.
pub(super) async fn operations(State(state): State<OwnerState>) -> Response<Body> {
    if let Err(refusal) = local_appliance(&state) {
        return refusal;
    }
    let root = state.daemon.root.clone();
    let status = match cli_beside_plane() {
        Ok(cli) => {
            let output = tokio::time::timeout(
                Duration::from_secs(20),
                tokio::process::Command::new(cli)
                    .args(["appliance", "status"])
                    .stdin(std::process::Stdio::null())
                    .output(),
            )
            .await;
            match output {
                Ok(Ok(output)) if output.status.success() => {
                    serde_json::from_slice::<serde_json::Value>(&output.stdout).ok()
                }
                _ => None,
            }
        }
        Err(_) => None,
    };
    let field = |name: &str| status.as_ref().and_then(|status| status.get(name)).cloned();
    let recovery = std::fs::read(root.join("machine/appliance-recovery.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let last_backup = std::fs::read(root.join(LAST_BACKUP))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .map(|mut backup| {
            let downloadable = backup["archive"]
                .as_str()
                .map(PathBuf::from)
                .is_some_and(|path| path.starts_with(root.join(BACKUPS)) && path.is_file());
            backup["downloadable"] = serde_json::Value::Bool(downloadable);
            backup
        });
    Json(serde_json::json!({
        "release_readable": status.is_some(),
        "installed_release": field("installed_release"),
        "previous_release": field("previous_release"),
        "source_commit": field("source_commit"),
        "upgrade_blocked": recovery.as_ref().map(|recovery| serde_json::json!({
            "failed_release": recovery.get("failed_release"),
            "rollback_refused": recovery.get("rollback_refused").and_then(serde_json::Value::as_bool).unwrap_or(false),
            "detail": recovery.get("detail"),
        })),
        "last_backup": last_backup,
        "job": read_job(&root),
    }))
    .into_response()
}

/// Run one appliance operation: `backup`, `rollback`, `retry-upgrade` (activate again the
/// release whose upgrade was blocked) or `start` (restore the plane's OS services, such as
/// schedule wake delivery). The cockpit asks the owner to confirm before it calls.
pub(super) async fn run(
    State(state): State<OwnerState>,
    AxumPath(op): AxumPath<String>,
) -> Response<Body> {
    if let Err(refusal) = local_appliance(&state) {
        return refusal;
    }
    let root = state.daemon.root.clone();
    if job_running(&root) {
        return refuse("Another appliance operation is still running. Wait for it to finish.");
    }
    let cli = match cli_beside_plane() {
        Ok(cli) => cli,
        Err(error) => return refuse(format!("{error:#}")),
    };
    let (cli, args): (PathBuf, Vec<String>) = match op.as_str() {
        "backup" => {
            let folder = root.join(BACKUPS);
            if let Err(error) = std::fs::create_dir_all(&folder) {
                return refuse(format!("Could not prepare the backup folder: {error}"));
            }
            prune_backups(&folder, 2);
            let name = format!("restless-backup-{}.tar", Utc::now().format("%Y%m%d-%H%M%S"));
            (
                cli,
                vec![
                    "appliance".into(),
                    "backup".into(),
                    folder.join(name).display().to_string(),
                ],
            )
        }
        "rollback" => (cli, vec!["appliance".into(), "rollback".into()]),
        "start" => (cli, vec!["appliance".into(), "start".into()]),
        "retry-upgrade" => match blocked_release(&root) {
            Ok(release) => {
                let mut args = vec![
                    "appliance".into(),
                    "upgrade".into(),
                    "--daemon".into(),
                    release.join("bin/restlessd").display().to_string(),
                    "--cockpit".into(),
                    release.join("web").display().to_string(),
                ];
                let environment =
                    release.join(restless_contracts::appliance::RELEASE_ENVIRONMENT_FILE);
                if environment.is_file() {
                    args.push("--release-environment".into());
                    args.push(environment.display().to_string());
                }
                // The release's own command stages it, as the terminal upgrade does.
                (release.join("bin/restless"), args)
            }
            Err(error) => return refuse(format!("{error:#}")),
        },
        _ => {
            return api_error(
                StatusCode::NOT_FOUND,
                "appliance",
                "No such appliance operation.",
            )
        }
    };
    match spawn_job(&root, &op, &cli, &args) {
        Ok(()) => {
            Json(serde_json::json!({ "started": op, "job": read_job(&root) })).into_response()
        }
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "appliance",
            format!("{error:#}"),
        ),
    }
}

/// The installed release whose upgrade was blocked, from the recovery record the upgrade wrote.
fn blocked_release(root: &std::path::Path) -> Result<PathBuf> {
    let recovery: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("machine/appliance-recovery.json"))
            .context("no appliance upgrade is blocked")?,
    )?;
    let name = recovery["failed_release"]
        .as_str()
        .filter(|name| !name.is_empty() && !name.contains('/') && !name.starts_with('.'))
        .context("the blocked upgrade names no release")?;
    // This plane runs from <install>/releases/<release>/bin/restlessd.
    let plane = std::env::current_exe()?.canonicalize()?;
    let releases = plane
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .context("this plane is not running from an installed release")?;
    let release = releases.join(name);
    if !release.join("bin/restless").is_file() || !release.join("bin/restlessd").is_file() {
        bail!("the release {name} is no longer installed; upgrade again from a new build");
    }
    Ok(release)
}

/// Keep the newest `keep` cockpit backups before writing another, so the folder cannot fill the
/// disk. A backup made elsewhere with the terminal is never touched.
fn prune_backups(folder: &std::path::Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let mut archives: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("restless-backup-") && name.ends_with(".tar"))
        })
        .collect();
    archives.sort();
    let excess = archives.len().saturating_sub(keep);
    for path in archives.into_iter().take(excess) {
        let _ = std::fs::remove_file(path);
    }
}

/// Download the last backup the cockpit made. It holds the company's secrets, so it goes only to
/// the owner, and only from the cockpit's own backup folder.
pub(super) async fn download_backup(State(state): State<OwnerState>) -> Response<Body> {
    if let Err(refusal) = local_appliance(&state) {
        return refusal;
    }
    let root = state.daemon.root.clone();
    let archive = std::fs::read(root.join(LAST_BACKUP))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|backup| backup["archive"].as_str().map(PathBuf::from));
    let Some(archive) =
        archive.filter(|path| path.starts_with(root.join(BACKUPS)) && path.is_file())
    else {
        return api_error(
            StatusCode::NOT_FOUND,
            "appliance",
            "There is no backup to download.",
        );
    };
    let name = archive
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("restless-backup.tar")
        .to_string();
    use tower::ServiceExt as _;
    let served = match ServeFile::new(&archive)
        .oneshot(Request::new(Body::empty()))
        .await
    {
        Ok(served) => served,
        Err(error) => return api_error(StatusCode::NOT_FOUND, "appliance", format!("{error}")),
    };
    let mut response = served.map(Body::new);
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/x-tar"));
    if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{name}\"")) {
        headers.insert(CONTENT_DISPOSITION, value);
    }
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// Reopen work admission after a lifecycle operation that left the drain barrier closed.
pub(super) async fn resume(State(state): State<OwnerState>) -> Response<Body> {
    if let Err(refusal) = local_appliance(&state) {
        return refusal;
    }
    match crate::coordination::resume_appliance(&state.daemon) {
        Ok(()) => Json(serde_json::json!({ "resumed": true })).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "appliance",
            format!("{error:#}"),
        ),
    }
}
