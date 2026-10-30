//! Burst compute jobs (S104 G4): heavy work an agent hands off instead of running it in its own
//! session, such as a simulation or a large build. Agents call `restless job run`, which returns
//! at once with a job id, and `restless job status <id>` reports how it went.
//!
//! This is the job interface with its local provider: the job runs on this machine in its own
//! container of the company image, on the company's volume, with its own CPU and memory bounds
//! and a time limit. Output goes to /company/run/jobs/<id>.log. A Cloud provider (larger servers
//! billed by the hour) can implement the same calls later without agents noticing.
//!
//! The machine provider (S108): `restless job run --on <computer> --repo owner/name` runs the job
//! on one of the owner's own computers assigned to the company, so heavy builds never land on the
//! plane and no files move. The job is a throwaway container of the company image, created through
//! that computer's filtered Docker link (Restless's own objects only, no host access), with a fresh
//! clone of the repository made with a read-only token for that one repository, and a per-company
//! cache volume as its home so toolchains and build output survive between jobs.

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use sqlx::PgPool;

/// The longest a job may run.
const MAX_TIMEOUT_SECS: u64 = 6 * 60 * 60;
const DEFAULT_TIMEOUT_SECS: u64 = 60 * 60;
/// Bound durable pending containers per account; running jobs are capacity-limited separately.
const MAX_WAITING_LOCAL_JOBS: usize = 64;
// Avoid holding one authority DB connection per concurrent submitter while
// the cross-generation advisory lock and Docker creation are in progress.
static LOCAL_JOB_SUBMISSION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobRequest {
    /// The shell command to run.
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub cpus: Option<f64>,
    /// Docker memory notation, such as 2g.
    #[serde(default)]
    pub memory: Option<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// Where it runs, under /company (default /company).
    #[serde(default)]
    pub workdir: Option<String>,
    /// For status: the job id run returned.
    #[serde(default)]
    pub id: Option<String>,
    /// One of the owner's computers assigned to this company, by name or id: the job runs there.
    #[serde(default)]
    pub on: Option<String>,
    /// owner/name, cloned fresh for the job before its command runs (needs `on`).
    #[serde(default)]
    pub repo: Option<String>,
    /// The branch, tag or commit to check out; the repository's default branch without one.
    #[serde(default, rename = "ref")]
    pub git_ref: Option<String>,
}

/// The longest log tail a status returns.
const LOG_TAIL_LINES: &str = "300";
const LOG_TAIL_BYTES: usize = 64 * 1024;

/// Run inside the job container: clone, check out, drop the token, run the command, clean up.
/// Everything is passed as environment so nothing the agent wrote is spliced into the script.
const MACHINE_SCRIPT: &str = r#"set -eu
export HOME=/company/home CARGO_HOME=/company/cargo RUSTUP_HOME=/company/rustup npm_config_cache=/company/npm-cache
mkdir -p "$HOME" /company/work
work="/company/work/$RESTLESS_JOB_ID"
rm -rf "$work"
mkdir -p "$work"
# The checkout goes however the job ends; the cache beside it stays.
trap 'cd / && rm -rf "$work"' EXIT
cd "$work"
if [ -n "${RESTLESS_JOB_REPO:-}" ]; then
  remote="https://github.com/${RESTLESS_JOB_REPO}.git"
  if [ -n "${RESTLESS_JOB_TOKEN:-}" ]; then
    remote="https://x-access-token:${RESTLESS_JOB_TOKEN}@github.com/${RESTLESS_JOB_REPO}.git"
  fi
  git -c credential.helper= clone --quiet --filter=blob:none "$remote" src
  cd src
  git remote set-url origin "https://github.com/${RESTLESS_JOB_REPO}.git"
  if [ -n "${RESTLESS_JOB_REF:-}" ]; then
    git checkout --quiet "$RESTLESS_JOB_REF" 2>/dev/null || {
      git -c credential.helper= fetch --quiet "$remote" "$RESTLESS_JOB_REF"
      git checkout --quiet FETCH_HEAD
    }
  fi
  export CARGO_TARGET_DIR="/company/target/$(printf %s "$RESTLESS_JOB_REPO" | tr / _)"
fi
unset RESTLESS_JOB_TOKEN remote
set +e
timeout "$RESTLESS_JOB_TIMEOUT" sh -c "$RESTLESS_JOB_COMMAND" 2>&1
exit $?
"#;

/// owner/name, plain GitHub characters only.
fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let plain = |part: &str| {
        !part.is_empty()
            && part.len() <= 100
            && !part.starts_with('.')
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    };
    plain(owner) && plain(name)
}

/// A branch, tag or commit: no option-looking or path-escaping names.
fn valid_ref(git_ref: &str) -> bool {
    !git_ref.is_empty()
        && git_ref.len() <= 200
        && !git_ref.starts_with('-')
        && !git_ref.contains("..")
        && git_ref
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/'))
}

fn machine_job_container(id: &str) -> String {
    format!("{}job-{id}", restless_machine::protocol::NAME_PREFIX)
}

/// The company's build cache on a computer: its home for jobs there.
fn cache_volume(company: &str) -> String {
    format!(
        "{}jobcache-{company}",
        restless_machine::protocol::NAME_PREFIX
    )
}

fn docker_on(machine: &str) -> tokio::process::Command {
    let mut command = tokio::process::Command::new("docker");
    command
        .arg("-H")
        .arg(format!(
            "unix://{}",
            crate::machines::docker_socket(machine).display()
        ))
        .env_remove("DOCKER_HOST")
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    command
}

async fn docker_on_output(
    mut command: tokio::process::Command,
    bound: std::time::Duration,
) -> Result<std::process::Output> {
    tokio::time::timeout(bound, command.output())
        .await
        .context("the computer did not answer in time")?
        .context("run docker on the computer")
}

/// One of the owner's own computers assigned to this company, by name or id, online and not
/// paused. Never the plane: that is what the local provider is for.
async fn job_machine(
    pool: &PgPool,
    company: &str,
    wanted: &str,
) -> Result<crate::machines::Machine> {
    let mut assigned = Vec::new();
    for assignment in crate::machines::company_assignments(pool, company).await? {
        if assignment.machine_id == crate::machines::PLANE_MACHINE {
            continue;
        }
        if let Some(machine) = crate::machines::get(pool, &assignment.machine_id).await? {
            assigned.push(machine);
        }
    }
    let wanted_lower = wanted.to_lowercase();
    let Some(machine) = assigned
        .into_iter()
        .find(|machine| machine.id == wanted || machine.label.to_lowercase() == wanted_lower)
    else {
        bail!(
            "\"{wanted}\" is not one of this company's computers. The owner lends one in Company › Computer › Heavy jobs; `restless job run` without --on runs beside your own computer."
        );
    };
    if machine.paused {
        bail!("{} is paused by the owner.", machine.label);
    }
    if !crate::machines::is_online(&machine.id) {
        bail!(
            "{} is offline right now (it has to be on and connected). Try again later.",
            machine.label
        );
    }
    Ok(machine)
}

async fn run_on_machine(
    pool: &PgPool,
    company: &str,
    actor: &str,
    request: &JobRequest,
) -> Result<serde_json::Value> {
    let wanted = request
        .on
        .as_deref()
        .map(str::trim)
        .filter(|on| !on.is_empty())
        .context(
            "a repository checkout runs on one of the owner's computers: add --on <computer>",
        )?;
    let command = request.command.trim();
    if command.is_empty() || command.len() > 8000 {
        bail!("give the job a command (up to 8,000 characters)");
    }
    if request.workdir.is_some() {
        bail!(
            "a job on another computer runs in its own checkout, not under /company; drop --workdir"
        );
    }
    let repo = request
        .repo
        .as_deref()
        .map(str::trim)
        .filter(|repo| !repo.is_empty());
    if let Some(repo) = repo {
        if !valid_repo(repo) {
            bail!("give the repository as owner/name, such as BlueprintLabIO/restless-core");
        }
    }
    let git_ref = request
        .git_ref
        .as_deref()
        .map(str::trim)
        .filter(|git_ref| !git_ref.is_empty());
    if let Some(git_ref) = git_ref {
        if repo.is_none() {
            bail!("--ref needs --repo");
        }
        if !valid_ref(git_ref) {
            bail!("that is not a branch, tag or commit");
        }
    }
    let machine = job_machine(pool, company, wanted).await?;
    // No token, no refusal: a public repository needs none, and an anonymous clone of a private
    // one simply fails in the job's log. The job says why it went without one.
    let (token, note) = match repo {
        Some(repo) => {
            match crate::account_broker::job_clone_token(pool, company, actor, repo).await {
                Ok(token) => (Some(token), None),
                Err(reason) => (
                    None,
                    Some(format!(
                        "{reason:#} Cloning without a token, which works only for a public repository."
                    )),
                ),
            }
        }
        None => (None, None),
    };
    let image = crate::machines::allowed_images()
        .await
        .into_iter()
        .next()
        .context("this plane has no company image to run")?
        .id;
    let timeout = request
        .timeout_secs
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .clamp(1, MAX_TIMEOUT_SECS);
    // Leave the owner one core of their own computer.
    let machine_cpus = f64::from(machine.facts.cpus.max(2)) - 1.0;
    let cpus = request
        .cpus
        .unwrap_or(machine_cpus)
        .clamp(0.5, machine_cpus);
    let memory = request
        .memory
        .as_deref()
        .map(str::trim)
        .filter(|memory| !memory.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            let half_gib = (machine.facts.memory_bytes / 2 / (1 << 30)).max(2);
            format!("{half_gib}g")
        });
    if !memory
        .trim_end_matches(['g', 'm', 'G', 'M'])
        .bytes()
        .all(|byte| byte.is_ascii_digit())
    {
        bail!("give memory like 8g or 512m");
    }
    let id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
    let name = machine_job_container(&id);
    let cache = cache_volume(company);
    let company_label = format!("restless.company={company}");
    let actor_label = format!("restless.job.actor={actor}");
    let mut volume = docker_on(&machine.id);
    volume.args(["volume", "create", "--label", &company_label, &cache]);
    let created = docker_on_output(volume, std::time::Duration::from_secs(30)).await?;
    if !created.status.success() {
        bail!(
            "the job's cache could not be made on {}: {}",
            machine.label,
            String::from_utf8_lossy(&created.stderr).trim()
        );
    }
    let cpus_text = format!("{cpus:.2}");
    let mount = format!("{cache}:/company");
    let mut run = docker_on(&machine.id);
    run.args([
        "run",
        "-d",
        "--name",
        &name,
        "--label",
        "restless.job=1",
        "--label",
        &company_label,
        "--label",
        &actor_label,
        "--cpus",
        &cpus_text,
        "--memory",
        &memory,
        "--memory-swap",
        &memory,
        "--pids-limit",
        "4096",
        "--log-driver",
        "local",
        "-u",
        "company",
        "-v",
        &mount,
        // Values come from this process's environment, so they never appear in its arguments.
        "-e",
        "RESTLESS_JOB_ID",
        "-e",
        "RESTLESS_JOB_COMMAND",
        "-e",
        "RESTLESS_JOB_TIMEOUT",
        "-e",
        "RESTLESS_JOB_REPO",
        "-e",
        "RESTLESS_JOB_REF",
        "-e",
        "RESTLESS_JOB_TOKEN",
        "--entrypoint",
        "sh",
        &image,
        "-c",
        MACHINE_SCRIPT,
    ])
    .env("RESTLESS_JOB_ID", &id)
    .env("RESTLESS_JOB_COMMAND", command)
    .env("RESTLESS_JOB_TIMEOUT", timeout.to_string())
    .env("RESTLESS_JOB_REPO", repo.unwrap_or_default())
    .env("RESTLESS_JOB_REF", git_ref.unwrap_or_default())
    .env("RESTLESS_JOB_TOKEN", token.unwrap_or_default());
    let started = docker_on_output(run, std::time::Duration::from_secs(120)).await?;
    if !started.status.success() {
        bail!(
            "the job could not start on {}: {}",
            machine.label,
            String::from_utf8_lossy(&started.stderr).trim()
        );
    }
    tracing::info!(company, actor, machine = %machine.id, job = %id, repo = ?repo, "job started on an owner's computer");
    Ok(serde_json::json!({
        "job": id,
        "status": "running",
        "on": machine.label,
        "repo": repo,
        "ref": git_ref,
        "cpus": cpus_text,
        "memory": memory,
        "timeout_secs": timeout,
        "note": note,
        "check": format!("restless job status {id}"),
    }))
}

/// A job on one of this company's computers: where it stands, and once finished, its log tail.
/// Only a container labelled with this company answers, so one company cannot read another's.
async fn machine_status(
    pool: &PgPool,
    company: &str,
    id: &str,
) -> Result<Option<serde_json::Value>> {
    let name = machine_job_container(id);
    for assignment in crate::machines::company_assignments(pool, company).await? {
        let machine_id = assignment.machine_id;
        if machine_id == crate::machines::PLANE_MACHINE || !crate::machines::is_online(&machine_id)
        {
            continue;
        }
        let mut inspect = docker_on(&machine_id);
        inspect.args([
            "inspect",
            "-f",
            "{{.State.Status}} {{.State.ExitCode}} {{.State.StartedAt}} {{.State.FinishedAt}} {{index .Config.Labels \"restless.company\"}}",
            &name,
        ]);
        let Ok(output) = docker_on_output(inspect, std::time::Duration::from_secs(20)).await else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        let fields: Vec<&str> = text.split_whitespace().collect();
        if fields.get(4).copied() != Some(company) {
            continue;
        }
        let state = fields.first().copied().unwrap_or_default();
        let exit_code: i64 = fields
            .get(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or(-1);
        let started = fields.get(2).copied().unwrap_or_default();
        let finished = fields.get(3).copied().unwrap_or_default();
        let label = crate::machines::get(pool, &machine_id)
            .await
            .ok()
            .flatten()
            .map(|machine| machine.label)
            .unwrap_or_else(|| machine_id.clone());
        let mut logs = docker_on(&machine_id);
        logs.args(["logs", "--tail", LOG_TAIL_LINES, &name]);
        let log = docker_on_output(logs, std::time::Duration::from_secs(30))
            .await
            .map(|output| {
                let mut text = String::from_utf8_lossy(&output.stdout).to_string();
                text.push_str(&String::from_utf8_lossy(&output.stderr));
                if text.len() > LOG_TAIL_BYTES {
                    let mut cut = text.len() - LOG_TAIL_BYTES;
                    while !text.is_char_boundary(cut) {
                        cut += 1;
                    }
                    text = text[cut..].to_string();
                }
                text
            })
            .unwrap_or_default();
        if state == "running" || state == "created" {
            return Ok(Some(serde_json::json!({
                "job": id, "status": "running", "on": label, "started_at": started, "log_tail": log,
            })));
        }
        let mut remove = docker_on(&machine_id);
        remove.args(["rm", &name]);
        let _ = docker_on_output(remove, std::time::Duration::from_secs(30)).await;
        return Ok(Some(serde_json::json!({
            "job": id,
            "status": if exit_code == 0 { "succeeded" } else if exit_code == 124 { "timed_out" } else { "failed" },
            "exit_code": exit_code,
            "on": label,
            "started_at": started,
            "finished_at": finished,
            "log_tail": log,
        })));
    }
    Ok(None)
}

/// Parse Docker's byte notation without accepting overflow or non-finite values.
fn job_memory_bytes(value: &str) -> Option<u64> {
    let value = value.trim().to_ascii_lowercase();
    let split = value
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(value.len());
    let (number, suffix) = value.split_at(split);
    let scale: u64 = match suffix.trim_end_matches('b') {
        "" => 1,
        "k" => 1024,
        "m" => 1024 * 1024,
        "g" => 1024 * 1024 * 1024,
        _ => return None,
    };
    let mut parts = number.split('.');
    let whole: u64 = parts.next()?.parse().ok()?;
    let mut result = whole.checked_mul(scale)?;
    if let Some(fraction) = parts.next() {
        if fraction.is_empty()
            || fraction.len() > 9
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let fraction: u64 = fraction.parse().ok()?;
        let denominator = 10u64.checked_pow(number.split('.').nth(1)?.len() as u32)?;
        let scaled = fraction.checked_mul(scale)?;
        result = result.checked_add(scaled.div_ceil(denominator))?;
    }
    if parts.next().is_some() || result == 0 {
        return None;
    }
    Some(result)
}

fn job_container(company: &str, id: &str) -> String {
    format!("{}-job-{id}", crate::runtime::container_name(company))
}

fn valid_id(id: &str) -> bool {
    id.len() == 12 && id.chars().all(|c| c.is_ascii_hexdigit())
}

/// Start a job and return its id at once.
pub async fn run(
    pool: &PgPool,
    company: &str,
    actor: &str,
    request: &JobRequest,
    wake: &tokio::sync::Notify,
) -> Result<serde_json::Value> {
    if request.on.is_some() || request.repo.is_some() {
        return run_on_machine(pool, company, actor, request).await;
    }
    let command = request.command.trim();
    if command.is_empty() || command.len() > 8000 {
        bail!("give the job a command (up to 8,000 characters)");
    }
    let workdir = request.workdir.as_deref().unwrap_or("/company").trim();
    if !(workdir == "/company" || workdir.starts_with("/company/")) || workdir.contains("..") {
        bail!("the job's workdir must be under /company");
    }
    let timeout = request
        .timeout_secs
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .clamp(1, MAX_TIMEOUT_SECS);
    let host_cpus = std::thread::available_parallelism().map_or(1, usize::from) as f64;
    let cpus = request.cpus.unwrap_or(host_cpus).clamp(0.1, host_cpus);
    let memory = crate::runtime::fit_job_memory(request.memory.as_deref());
    let memory_bytes = memory
        .as_deref()
        .and_then(job_memory_bytes)
        .context("give the job a finite memory limit, such as 512m or 2g")?;
    if let Some(ceiling) = crate::runtime::shared_job_hard_ceiling()? {
        if memory_bytes > ceiling {
            bail!(
                "This job requests more memory than the account can admit. Request at most {} MiB or increase the account limit; no job was started.",
                ceiling / (1024 * 1024)
            );
        }
    }
    let _submission = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        LOCAL_JOB_SUBMISSION.lock(),
    )
    .await
    .context("The account job queue is busy. Try again shortly.")?;
    // Do not create a saved job after this company has been destroyed. The
    // lifecycle lock also makes an in-flight create finish before destruction
    // removes the company's job containers and volume.
    let _company_guard = crate::runtime::company_start_guard(company).await;
    crate::runtime::CompanyConfig::load(&crate::runtime::state_root(), company)
        .context("The company is no longer available; no job was started")?;
    // Multiple daemon generations may overlap during an upgrade. Serialize
    // the inventory-and-create boundary in the existing shared authority DB.
    let scope = format!(
        "namespace:{}",
        std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default()
    );
    let mut queue_lock = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1), hashtext($2))")
        .bind("restless.local-job-queue")
        .bind(&scope)
        .execute(&mut *queue_lock)
        .await?;
    let scope_filter = format!("label=restless.job.scope={scope}");
    let waiting = crate::runtime::docker_bounded(
        &[
            "ps",
            "-a",
            "--filter",
            "label=restless.job.queue=1",
            "--filter",
            &scope_filter,
            "--filter",
            "status=created",
            "--format",
            "{{.Names}}",
        ],
        std::time::Duration::from_secs(10),
    )
    .await?;
    anyhow::ensure!(
        waiting.status.success(),
        "the account job queue is temporarily unavailable"
    );
    anyhow::ensure!(
        String::from_utf8_lossy(&waiting.stdout)
            .lines()
            .filter(|name| name
                .rsplit_once("-job-")
                .is_some_and(|(_, id)| valid_id(id.strip_suffix("-preparing").unwrap_or(id))))
            .count()
            < MAX_WAITING_LOCAL_JOBS,
        "The account job queue is full. Wait for a queued job to start before submitting another."
    );
    let id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
    let name = job_container(company, &id);
    // A preparation is not executable by the scheduler until a successful
    // rename marks it accepted. If Docker accepted create but its response
    // times out, an orphan cannot later run behind a caller's retry.
    let preparing = format!("{name}-preparing");
    let log = format!("/company/run/jobs/{id}.log");
    // The command runs under `timeout`, its output to the log, so an agent reads it with ordinary
    // file tools and a runaway job ends by itself.
    let script = format!(
        "mkdir -p /company/run/jobs && cd {workdir} && timeout {timeout} sh -c \"$RESTLESS_JOB_COMMAND\" > {log} 2>&1"
    );
    let cpus = format!("{cpus:.2}");
    let volume = format!("{}:/company", crate::runtime::volume_name(company));
    let command_env = format!("RESTLESS_JOB_COMMAND={command}");
    let company_label = format!("restless.company={company}");
    let actor_label = format!("restless.job.actor={actor}");
    let scope_label = format!("restless.job.scope={scope}");
    let image = crate::runtime::company_image();
    let mut args: Vec<&str> = vec![
        "create",
        "--name",
        &preparing,
        "--label",
        "restless.job=1",
        "--label",
        "restless.job.queue=1",
        "--label",
        &scope_label,
        "--label",
        &company_label,
        "--label",
        &actor_label,
        "--cpus",
        &cpus,
        "--cpu-shares",
        "256",
        "--pids-limit",
        "2048",
        "--log-driver",
        "local",
        "-u",
        "company",
        "-v",
        &volume,
        "-e",
        &command_env,
        "--entrypoint",
        "sh",
    ];
    if let Some(memory) = memory.as_deref() {
        args.extend(["--memory", memory, "--memory-swap", memory]);
    }
    let workload_parent = crate::runtime::workload_cgroup_parent()?;
    if let Some(parent) = workload_parent.as_deref() {
        args.extend(["--cgroup-parent", parent]);
    }
    args.extend([image.as_str(), "-c", script.as_str()]);
    let output = match crate::runtime::docker_bounded(&args, std::time::Duration::from_secs(60))
        .await
    {
        Ok(output) => output,
        Err(error) => {
            wake.notify_one();
            return Err(error.context("Job preparation was not confirmed; the job was not started"));
        }
    };
    if !output.status.success() {
        wake.notify_one();
        bail!(
            "Job preparation failed; the job was not started: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let renamed = crate::runtime::docker_bounded(
        &["rename", &preparing, &name],
        std::time::Duration::from_secs(15),
    )
    .await;
    let confirmed = renamed.is_ok_and(|output| output.status.success());
    wake.notify_one();
    // The transaction holds only an advisory lock. Once Docker has accepted
    // the durable intent, a lost lock-release acknowledgement must not turn
    // the successful submission into an error and invite a duplicate.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), queue_lock.rollback()).await;
    Ok(serde_json::json!({
        "job": id,
        "status": if confirmed { "waiting_for_capacity" } else { "confirmation_unknown" },
        "reason": if confirmed { "Waiting for account capacity and a running company computer; starts when both are available." } else { "Job confirmation is uncertain. Check its status before submitting another." },
        "log": log,
        "cpus": cpus,
        "memory": memory,
        "timeout_secs": timeout,
        "check": format!("restless job status {id}"),
    }))
}

/// Reconcile the Docker-owned pending jobs from the existing schedule loop.
/// Created containers retain the exact job across a daemon restart without
/// starting its command; only this loop starts them. Unknown resource readings
/// leave the intent pending. Returns whether another retry is needed.
pub async fn reconcile_pending(daemon: &crate::Daemon, companies: &[String]) -> Result<bool> {
    let output = crate::runtime::docker_bounded(
        &[
            "ps",
            "-a",
            "--filter",
            "label=restless.job=1",
            "--filter",
            "status=created",
            "--filter",
            "status=running",
            "--filter",
            "status=paused",
            "--filter",
            "status=restarting",
            "--format",
            "{{.Names}}",
        ],
        std::time::Duration::from_secs(10),
    )
    .await?;
    anyhow::ensure!(output.status.success(), "job inventory unavailable");
    let names: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|name| {
            companies.iter().any(|company| {
                name.strip_prefix(&job_container(company, ""))
                    .is_some_and(|suffix| {
                        valid_id(suffix.strip_suffix("-preparing").unwrap_or(suffix))
                    })
            })
        })
        .map(str::to_owned)
        .collect();
    if names.is_empty() {
        return Ok(false);
    }
    // One bounded Docker inspection for the whole pool avoids one process per
    // queued job during each pressure retry.
    let mut args = vec![
        "inspect",
        "-f",
        r#"{"name":{{json .Name}},"state":{{json .State.Status}},"memory":{{.HostConfig.Memory}},"created":{{json .Created}},"queued":{{json (index .Config.Labels "restless.job.queue")}},"actor":{{json (index .Config.Labels "restless.job.actor")}}}"#,
    ];
    args.extend(names.iter().map(String::as_str));
    let info = crate::runtime::docker_bounded(&args, std::time::Duration::from_secs(10)).await?;
    anyhow::ensure!(info.status.success(), "job resource inventory unavailable");
    let text = String::from_utf8_lossy(&info.stdout);
    let mut jobs = Vec::new();
    for line in text.lines() {
        let row: serde_json::Value = serde_json::from_str(line)?;
        let name = row["name"]
            .as_str()
            .context("missing job name")?
            .trim_start_matches('/');
        let company = companies
            .iter()
            .find(|company| {
                name.strip_prefix(&job_container(company, ""))
                    .is_some_and(|suffix| {
                        valid_id(suffix.strip_suffix("-preparing").unwrap_or(suffix))
                    })
            })
            .context("job inventory did not match its company")?;
        let state = row["state"]
            .as_str()
            .context("missing job state")?
            .to_string();
        let memory = row["memory"].as_u64().context("missing job memory")?;
        let created = row["created"]
            .as_str()
            .context("missing job creation time")?
            .to_string();
        let queued = row["queued"].as_str() == Some("1");
        let actor = row["actor"].as_str().map(str::to_owned);
        jobs.push((
            created,
            name.to_string(),
            company.clone(),
            state,
            memory,
            queued,
            actor,
        ));
    }
    anyhow::ensure!(
        jobs.len() == names.len(),
        "incomplete job resource inventory"
    );
    jobs.sort_by(|a, b| a.0.cmp(&b.0));
    let pending = jobs.iter().any(|job| job.3 == "created" && job.5);
    if !pending {
        return Ok(false);
    }
    // Reap old inert preparations only while no submission holds this same
    // account lock. A late rename and removal cannot both win Docker's name
    // transition; preparations are never started directly.
    if jobs.iter().any(|job| {
        job.3 == "created"
            && job.5
            && job.1.ends_with("-preparing")
            && chrono::DateTime::parse_from_rfc3339(&job.0).is_ok_and(|created| {
                chrono::Utc::now()
                    .signed_duration_since(created)
                    .num_seconds()
                    >= 120
            })
    }) {
        let scope = format!(
            "namespace:{}",
            std::env::var("RESTLESS_RESOURCE_NAMESPACE").unwrap_or_default()
        );
        let mut lock = daemon.authority.pool().begin().await?;
        let acquired: bool =
            sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtext($1), hashtext($2))")
                .bind("restless.local-job-queue")
                .bind(&scope)
                .fetch_one(&mut *lock)
                .await?;
        if acquired {
            for job in jobs
                .iter()
                .filter(|job| job.3 == "created" && job.5 && job.1.ends_with("-preparing"))
            {
                let old_enough =
                    chrono::DateTime::parse_from_rfc3339(&job.0).is_ok_and(|created| {
                        chrono::Utc::now()
                            .signed_duration_since(created)
                            .num_seconds()
                            >= 120
                    });
                if old_enough {
                    let _ = crate::runtime::docker_bounded(
                        &["rm", &job.1],
                        std::time::Duration::from_secs(15),
                    )
                    .await;
                }
            }
        }
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), lock.rollback()).await;
    }
    let shared = crate::runtime::shared_workload_admission()?;
    let reserved = jobs
        .iter()
        .filter(|job| matches!(job.3.as_str(), "running" | "paused" | "restarting"))
        .fold(0u64, |sum, job| {
            if job.4 == 0 {
                u64::MAX
            } else {
                sum.saturating_add(job.4)
            }
        });
    for (_, name, company, _state, memory, _queued, actor) in jobs
        .iter()
        .filter(|job| job.3 == "created" && job.5 && !job.1.ends_with("-preparing"))
    {
        if let Some(admission) = &shared {
            if admission.waiting()
                || memory == &0
                || admission
                    .working_set_bytes
                    .saturating_add(reserved)
                    .saturating_add(*memory)
                    > admission.resume_below_bytes
            {
                continue;
            }
        }
        // Serialize with owner stop, sleep and destruction through the same
        // per-company lifecycle lock used by those operations.
        let _company_guard = crate::runtime::company_start_guard(company).await;
        // A stopped or removed Runtime must not be woken by a queued job.
        if !matches!(
            crate::runtime::status(company).await,
            Ok(crate::runtime::ContainerStatus::Running)
        ) {
            continue;
        }
        match crate::runtime::workload_admission(company).await {
            Ok(Some(admission)) if admission.waiting() => continue,
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(company, %error, "waiting job could not read company capacity");
                continue;
            }
        }
        // A queued command is still attributed to its requester. Serialize
        // the launch boundary with actor retirement in the source cell.
        let org = match daemon.orgintel.get(company).await {
            Ok(org) => org,
            Err(error) => {
                tracing::warn!(company, %error, "waiting job cannot read its actor");
                continue;
            }
        };
        let lease = match actor {
            Some(actor) => match org.lock_active_actor_for_job(actor).await {
                Ok(lease) => lease,
                Err(error) => {
                    tracing::warn!(company, %error, "waiting job cannot validate its actor");
                    continue;
                }
            },
            None => None,
        };
        let Some(lease) = lease else {
            // Keep a durable, inspectable refusal; never run the old command.
            let refused = format!("{name}-refused");
            let output = crate::runtime::docker_bounded(
                &["rename", name, &refused],
                std::time::Duration::from_secs(15),
            )
            .await?;
            anyhow::ensure!(
                output.status.success(),
                "could not record a refused waiting job"
            );
            continue;
        };
        let output =
            crate::runtime::docker_bounded(&["start", name], std::time::Duration::from_secs(30))
                .await?;
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), lease.rollback()).await;
        anyhow::ensure!(output.status.success(), "could not start a waiting job");
        // One start per pass lets live counters settle before admitting another.
        break;
    }
    Ok(true)
}

/// Remove this company's local jobs before its volume is destroyed. Both the
/// label and exact namespace-derived name are required to protect other planes.
pub async fn remove_company_jobs(company: &str) -> Result<()> {
    let label = format!("label=restless.company={company}");
    let output = crate::runtime::docker_bounded(
        &[
            "ps",
            "-a",
            "--filter",
            "label=restless.job=1",
            "--filter",
            &label,
            "--format",
            "{{.Names}}",
        ],
        std::time::Duration::from_secs(10),
    )
    .await?;
    anyhow::ensure!(output.status.success(), "company job inventory unavailable");
    let prefix = job_container(company, "");
    for name in String::from_utf8_lossy(&output.stdout).lines() {
        if !name.strip_prefix(&prefix).is_some_and(|suffix| {
            valid_id(
                suffix
                    .strip_suffix("-refused")
                    .or_else(|| suffix.strip_suffix("-preparing"))
                    .unwrap_or(suffix),
            )
        }) {
            continue;
        }
        let removed =
            crate::runtime::docker_bounded(&["rm", "-f", name], std::time::Duration::from_secs(30))
                .await?;
        anyhow::ensure!(removed.status.success(), "company job could not be removed");
    }
    Ok(())
}

/// How a job is doing: running, or finished with its exit code (124 means it ran out of time).
/// A finished job's container is removed on the first status that sees it finished.
pub async fn status(pool: &PgPool, company: &str, id: &str) -> Result<serde_json::Value> {
    if !valid_id(id) {
        bail!("that is not a job id");
    }
    if let Some(found) = machine_status(pool, company, id).await? {
        return Ok(found);
    }
    let name = job_container(company, id);
    let output = crate::runtime::docker_bounded(
        &[
            "inspect",
            "-f",
            r#"{{.State.Status}} {{.State.ExitCode}} {{.State.StartedAt}} {{.State.FinishedAt}} {{json (index .Config.Labels "restless.job.queue")}}"#,
            &name,
        ],
        std::time::Duration::from_secs(10),
    )
    .await?;
    let log = format!("/company/run/jobs/{id}.log");
    if !output.status.success() {
        let refused = format!("{name}-refused");
        let found = crate::runtime::docker_bounded(
            &["inspect", "-f", "{{.State.Status}}", &refused],
            std::time::Duration::from_secs(10),
        )
        .await?;
        if found.status.success() {
            let _ = crate::runtime::docker_bounded(
                &["rm", &refused],
                std::time::Duration::from_secs(20),
            )
            .await;
            return Ok(
                serde_json::json!({ "job": id, "status": "failed", "reason": "The requesting agent is no longer active; this job did not run.", "log": log }),
            );
        }
        let preparing = format!("{name}-preparing");
        let preparation = crate::runtime::docker_bounded(
            &["inspect", "-f", "{{.State.Status}}", &preparing],
            std::time::Duration::from_secs(10),
        )
        .await?;
        if preparation.status.success() {
            return Ok(
                serde_json::json!({ "job": id, "status": "confirmation_unknown", "reason": "Job confirmation is uncertain. Check its status before submitting another.", "log": log }),
            );
        }
        // A failed inspect can be a Docker outage or a transient read error,
        // not evidence the job disappeared. Confirm absence from a successful
        // inventory before telling a caller it is gone.
        let filter = format!("name={name}");
        let inventory = crate::runtime::docker_bounded(
            &["ps", "-a", "--filter", &filter, "--format", "{{.Names}}"],
            std::time::Duration::from_secs(10),
        )
        .await?;
        anyhow::ensure!(
            inventory.status.success(),
            "Job status is temporarily unavailable. Try again."
        );
        let names = String::from_utf8_lossy(&inventory.stdout);
        anyhow::ensure!(
            !names
                .lines()
                .any(|item| item == name || item == refused || item == preparing),
            "Job status is temporarily unavailable. Try again."
        );
        return Ok(serde_json::json!({ "job": id, "status": "gone", "log": log,
            "detail": "No such job is running; a finished job is reported once, then its log remains." }));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut fields = text.split_whitespace();
    let state = fields.next().unwrap_or_default().to_string();
    let exit_code: i64 = fields
        .next()
        .and_then(|code| code.parse().ok())
        .unwrap_or(-1);
    let started = fields.next().unwrap_or_default().to_string();
    let finished = fields.next().unwrap_or_default().to_string();
    let queued = fields.next() == Some(r#""1""#);
    if state == "created" && queued {
        return Ok(
            serde_json::json!({ "job": id, "status": "waiting_for_capacity", "reason": "Waiting for account capacity and a running company computer; starts when both are available.", "log": log }),
        );
    }
    if state == "running" || state == "created" {
        return Ok(
            serde_json::json!({ "job": id, "status": "running", "started_at": started, "log": log }),
        );
    }
    let _ =
        crate::runtime::docker_bounded(&["rm", &name], std::time::Duration::from_secs(20)).await;
    Ok(serde_json::json!({
        "job": id,
        "status": if exit_code == 0 { "succeeded" } else if exit_code == 124 { "timed_out" } else { "failed" },
        "exit_code": exit_code,
        "started_at": started,
        "finished_at": finished,
        "log": log,
    }))
}

/// Read the job fields a coordination request carries.
pub fn request(value: Option<&serde_json::Value>) -> Result<JobRequest> {
    serde_json::from_value(value.cloned().context("missing job")?).context("invalid job")
}

#[cfg(test)]
mod tests {
    #[test]
    fn job_memory_parsing_is_finite_and_checked() {
        assert_eq!(super::job_memory_bytes("512m"), Some(512 * 1024 * 1024));
        assert_eq!(super::job_memory_bytes("1.5g"), Some(1536 * 1024 * 1024));
        assert_eq!(super::job_memory_bytes("0.1k"), Some(103));
        assert_eq!(super::job_memory_bytes("0"), None);
        assert_eq!(super::job_memory_bytes("NaN"), None);
        assert_eq!(super::job_memory_bytes("18446744073709551615g"), None);
        // The host fitting step must not panic before job validation rejects it.
        assert_eq!(
            crate::runtime::fit_job_memory(Some("18446744073709551615g")).as_deref(),
            Some("18446744073709551615g")
        );
        assert_eq!(super::job_memory_bytes("1.2.3g"), None);
    }

    #[test]
    fn job_ids_are_twelve_hex_characters() {
        assert!(super::valid_id("0123456789ab"));
        assert!(!super::valid_id("../../etc"));
        assert!(!super::valid_id("0123456789abc"));
    }

    #[test]
    fn repositories_and_refs_are_plain_names() {
        assert!(super::valid_repo("BlueprintLabIO/restless-core"));
        assert!(super::valid_repo("yail259/how-to-be-human"));
        assert!(!super::valid_repo("BlueprintLabIO"));
        assert!(!super::valid_repo("a/b/c"));
        assert!(!super::valid_repo("../etc"));
        assert!(!super::valid_repo("a/b;rm -rf /"));
        assert!(!super::valid_repo("a/.git"));
        assert!(super::valid_ref("main"));
        assert!(super::valid_ref("feat/s108-machine-jobs"));
        assert!(super::valid_ref("0123456789abcdef0123456789abcdef01234567"));
        assert!(!super::valid_ref("--upload-pack=x"));
        assert!(!super::valid_ref("a..b"));
        assert!(!super::valid_ref("main; curl x"));
    }

    #[test]
    fn a_machine_job_is_named_and_cached_as_restless_owns_it() {
        assert_eq!(
            super::machine_job_container("0123456789ab"),
            "restless-job-0123456789ab"
        );
        assert_eq!(
            super::cache_volume("company_01a1"),
            "restless-jobcache-company_01a1"
        );
    }

    #[test]
    fn the_job_script_never_keeps_the_token() {
        let script = super::MACHINE_SCRIPT;
        let unset = script.find("unset RESTLESS_JOB_TOKEN").unwrap();
        let command = script.find("sh -c \"$RESTLESS_JOB_COMMAND\"").unwrap();
        assert!(unset < command, "the token is gone before the command runs");
        assert!(script.contains("git remote set-url origin \"https://github.com/"));
    }

    #[test]
    fn a_job_request_names_only_known_fields() {
        let ok =
            super::request(Some(&serde_json::json!({"command": "make", "cpus": 2.0}))).unwrap();
        assert_eq!(ok.command, "make");
        assert!(
            super::request(Some(
                &serde_json::json!({"command": "x", "privileged": true})
            ))
            .is_err()
        );
        assert!(super::request(None).is_err());
        let on = super::request(Some(&serde_json::json!({
            "command": "cargo test", "on": "Yail's PC", "repo": "a/b", "ref": "main"
        })))
        .unwrap();
        assert_eq!(on.git_ref.as_deref(), Some("main"));
        assert_eq!(on.on.as_deref(), Some("Yail's PC"));
    }
}
