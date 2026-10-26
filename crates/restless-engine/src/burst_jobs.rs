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

use anyhow::{bail, Context as _, Result};
use serde::Deserialize;
use sqlx::PgPool;

/// The longest a job may run.
const MAX_TIMEOUT_SECS: u64 = 6 * 60 * 60;
const DEFAULT_TIMEOUT_SECS: u64 = 60 * 60;

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
        bail!("a job on another computer runs in its own checkout, not under /company; drop --workdir");
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
    let id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
    let name = job_container(company, &id);
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
    let image = crate::runtime::company_image();
    let mut args: Vec<&str> = vec![
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
    args.extend([image.as_str(), "-c", script.as_str()]);
    let output = crate::runtime::docker_bounded(&args, std::time::Duration::from_secs(60)).await?;
    if !output.status.success() {
        bail!(
            "the job could not start: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(serde_json::json!({
        "job": id,
        "status": "running",
        "log": log,
        "cpus": cpus,
        "memory": memory,
        "timeout_secs": timeout,
        "check": format!("restless job status {id}"),
    }))
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
            "{{.State.Status}} {{.State.ExitCode}} {{.State.StartedAt}} {{.State.FinishedAt}}",
            &name,
        ],
        std::time::Duration::from_secs(10),
    )
    .await?;
    let log = format!("/company/run/jobs/{id}.log");
    if !output.status.success() {
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
        assert!(super::request(Some(
            &serde_json::json!({"command": "x", "privileged": true})
        ))
        .is_err());
        assert!(super::request(None).is_err());
        let on = super::request(Some(&serde_json::json!({
            "command": "cargo test", "on": "Yail's PC", "repo": "a/b", "ref": "main"
        })))
        .unwrap();
        assert_eq!(on.git_ref.as_deref(), Some("main"));
        assert_eq!(on.on.as_deref(), Some("Yail's PC"));
    }
}
