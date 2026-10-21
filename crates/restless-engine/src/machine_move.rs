//! Moving a company's computer to another machine (S99, US-347).
//!
//! The company's work stops, its workspace and kept files are copied to the target machine, the
//! target becomes its main computer, and the computer starts there. The source keeps its copy,
//! stopped and renamed aside, until the owner removes it: a move never deletes the only copy. If
//! anything fails before the switch, the company starts again where it was.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{LazyLock, Mutex};

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use tokio::process::Command;

use crate::machines::{self, PLANE_MACHINE, ROLE_MAIN};
use crate::runtime::{self, CompanyConfig, ContainerStatus};

#[derive(Debug, Clone, Serialize)]
pub struct MoveProgress {
    pub to: String,
    pub step: String,
    pub done: bool,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
}

static MOVES: LazyLock<Mutex<HashMap<String, MoveProgress>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The move in progress or last finished for a company, as long as this plane process has run.
pub fn progress(company: &str) -> Option<MoveProgress> {
    MOVES.lock().unwrap().get(company).cloned()
}

fn record(company: &str, step: &str) {
    if let Some(progress) = MOVES.lock().unwrap().get_mut(company) {
        progress.step = step.to_string();
    }
}

/// The Docker socket that reaches `machine` directly, bypassing placement.
fn socket_of(machine: &str) -> PathBuf {
    if machine == PLANE_MACHINE {
        crate::machine_router::local_socket()
    } else {
        machines::docker_socket(machine)
    }
}

fn docker_at(machine: &str) -> Command {
    let mut command = Command::new("docker");
    command
        .arg("-H")
        .arg(format!("unix://{}", socket_of(machine).display()));
    command.env_remove("DOCKER_HOST");
    command
}

async fn ok(mut command: Command, what: &str) -> Result<String> {
    let output = command
        .stdin(Stdio::null())
        .output()
        .await
        .with_context(|| format!("run docker to {what}"))?;
    if !output.status.success() {
        bail!("{what}: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn volume_labels(machine: &str, volume: &str) -> Result<Option<Vec<String>>> {
    let mut inspect = docker_at(machine);
    inspect.args([
        "volume",
        "inspect",
        "-f",
        "{{range $k, $v := .Labels}}{{$k}}={{$v}}\n{{end}}",
        volume,
    ]);
    let output = inspect.stdin(Stdio::null()).output().await?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| {
                !line.is_empty() && !line.starts_with(restless_machine::protocol::MACHINE_LABEL)
            })
            .map(str::to_string)
            .collect(),
    ))
}

/// Copy one named volume from one machine to another. The target's copy is replaced: it can only
/// be a stale one, since the company's main computer is the source.
pub(crate) async fn copy_volume(company: &str, volume: &str, from: &str, to: &str) -> Result<bool> {
    let Some(labels) = volume_labels(from, volume).await? else {
        return Ok(false);
    };
    let image = runtime::company_image();
    let mut create = docker_at(to);
    create.args(["volume", "create"]);
    for label in &labels {
        create.args(["--label", label]);
    }
    create.arg(volume);
    ok(create, "create the volume on the new computer").await?;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let mount = format!("{volume}:/v");
    let mut source = docker_at(from);
    source
        .args([
            "run",
            "--rm",
            "--name",
            &format!("restless-move-out-{}", &suffix[..12]),
        ])
        .args([
            "--network",
            "none",
            "--user",
            "0",
            "-v",
            &mount,
            "--entrypoint",
            "tar",
            &image,
        ])
        .args(["-C", "/v", "-cf", "-", "."])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .kill_on_drop(true);
    let mut target = docker_at(to);
    target
        .args([
            "run",
            "--rm",
            "-i",
            "--name",
            &format!("restless-move-in-{}", &suffix[..12]),
        ])
        .args([
            "--network",
            "none",
            "--user",
            "0",
            "-v",
            &mount,
            "--entrypoint",
            "sh",
            &image,
        ])
        .args(["-c", "find /v -mindepth 1 -delete && tar -C /v -xpf -"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut reader = source.spawn().context("start reading the volume")?;
    let mut writer = target.spawn().context("start writing the volume")?;
    let mut out = reader.stdout.take().context("volume reader output")?;
    let mut input = writer.stdin.take().context("volume writer input")?;
    let copied = tokio::io::copy(&mut out, &mut input).await;
    drop(input);
    let read = reader.wait_with_output().await?;
    let written = writer.wait_with_output().await?;
    copied.with_context(|| format!("copy {volume} for {company}"))?;
    if !read.status.success() {
        bail!(
            "read {volume}: {}",
            String::from_utf8_lossy(&read.stderr).trim()
        );
    }
    if !written.status.success() {
        bail!(
            "write {volume}: {}",
            String::from_utf8_lossy(&written.stderr).trim()
        );
    }
    Ok(true)
}

async fn set_aside(company: &str, machine: &str) -> Result<()> {
    let name = runtime::container_name(company);
    let mut exists = docker_at(machine);
    exists.args(["inspect", "-f", "{{.Id}}", &name]);
    if ok(exists, "find the old computer").await.is_err() {
        return Ok(());
    }
    let aside = format!("{name}-moved-{}", Utc::now().format("%Y%m%d%H%M%S"));
    let mut rename = docker_at(machine);
    rename.args(["rename", &name, &aside]);
    ok(rename, "set the old computer aside").await.map(|_| ())
}

async fn run_move(pool: &PgPool, config: &CompanyConfig, from: &str, to: &str) -> Result<()> {
    let company = config.name.as_str();
    record(company, "Pausing work");
    runtime::down(company)
        .await
        .context("stop the company computer")?;
    for volume in [
        runtime::volume_name(company),
        runtime::attachments_volume_name(company),
    ] {
        record(
            company,
            if volume == runtime::volume_name(company) {
                "Copying the workspace"
            } else {
                "Copying kept files"
            },
        );
        copy_volume(company, &volume, from, to).await?;
    }
    record(company, "Switching");
    set_aside(company, from).await?;
    machines::assign(pool, company, to, ROLE_MAIN).await?;
    crate::machine_router::refresh(pool).await?;
    record(company, "Starting on the new computer");
    runtime::up(config, true)
        .await
        .context("start the company on the new computer")?;
    if runtime::status(company).await? != ContainerStatus::Running {
        bail!("the company computer did not start on the new computer");
    }
    Ok(())
}

/// Undo a failed move: the company's main computer is `from` again and starts there.
async fn restore(pool: &PgPool, config: &CompanyConfig, from: &str) -> Result<()> {
    let company = config.name.as_str();
    if from == PLANE_MACHINE {
        sqlx::query(
            "DELETE FROM restless_authority.company_machines WHERE company = $1 AND role = 'main'",
        )
        .bind(company)
        .execute(pool)
        .await?;
    } else {
        machines::assign(pool, company, from, ROLE_MAIN).await?;
    }
    crate::machine_router::refresh(pool).await?;
    // Bring back a computer set aside on the source.
    let name = runtime::container_name(company);
    let mut list = docker_at(from);
    list.args([
        "ps",
        "-a",
        "--filter",
        &format!("name=^{name}-moved-"),
        "--format",
        "{{.Names}}",
    ]);
    if let Ok(names) = ok(list, "find the set-aside computer").await {
        if let Some(latest) = names.lines().max() {
            let mut rename = docker_at(from);
            rename.args(["rename", latest, &name]);
            let _ = ok(rename, "restore the old computer").await;
        }
    }
    runtime::up(config, false).await.map(|_| ())
}

/// Start moving a company's computer to `to`. Returns at once; [`progress`] reports the steps.
pub async fn start(pool: PgPool, config: CompanyConfig, to: String) -> Result<()> {
    let company = config.name.clone();
    let from = machines::main_machine(&pool, &company).await?;
    if from == to {
        bail!("The company already runs on that computer.");
    }
    for machine in [&from, &to] {
        if machine != PLANE_MACHINE && !machines::is_online(machine) {
            bail!("Both computers must be online to move a company between them.");
        }
    }
    if machines::get(&pool, &to)
        .await?
        .is_none_or(|machine| machine.paused)
    {
        bail!("That computer is paused or no longer exists.");
    }
    {
        let mut moves = MOVES.lock().unwrap();
        if moves.get(&company).is_some_and(|progress| !progress.done) {
            bail!("This company is already moving.");
        }
        moves.insert(
            company.clone(),
            MoveProgress {
                to: to.clone(),
                step: "Starting".into(),
                done: false,
                error: None,
                started_at: Utc::now(),
            },
        );
    }
    tokio::spawn(async move {
        let outcome = run_move(&pool, &config, &from, &to).await;
        let error = match outcome {
            Ok(()) => None,
            Err(error) => {
                tracing::warn!(company = %config.name, error = %format!("{error:#}"), "company move failed; restoring");
                if let Err(restore_error) = restore(&pool, &config, &from).await {
                    tracing::error!(company = %config.name, error = %format!("{restore_error:#}"), "restore after a failed move failed");
                }
                Some(format!("{error:#}"))
            }
        };
        if let Some(progress) = MOVES.lock().unwrap().get_mut(&config.name) {
            progress.done = true;
            progress.step = if error.is_none() {
                "Moved".into()
            } else {
                "Moved back".into()
            };
            progress.error = error;
        }
    });
    Ok(())
}
