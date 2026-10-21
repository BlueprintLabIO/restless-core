//! Burst compute jobs (S104 G4): heavy work an agent hands off instead of running it in its own
//! session, such as a simulation or a large build. Agents call `restless job run`, which returns
//! at once with a job id, and `restless job status <id>` reports how it went.
//!
//! This is the job interface with its local provider: the job runs on this machine in its own
//! container of the company image, on the company's volume, with its own CPU and memory bounds
//! and a time limit. Output goes to /company/run/jobs/<id>.log. A Cloud provider (larger servers
//! billed by the hour) can implement the same calls later without agents noticing.

use anyhow::{bail, Context as _, Result};
use serde::Deserialize;

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
}

fn job_container(company: &str, id: &str) -> String {
    format!("{}-job-{id}", crate::runtime::container_name(company))
}

fn valid_id(id: &str) -> bool {
    id.len() == 12 && id.chars().all(|c| c.is_ascii_hexdigit())
}

/// Start a job and return its id at once.
pub async fn run(company: &str, actor: &str, request: &JobRequest) -> Result<serde_json::Value> {
    let command = request.command.trim();
    if command.is_empty() || command.len() > 8000 {
        bail!("give the job a command (up to 8,000 characters)");
    }
    let workdir = request.workdir.as_deref().unwrap_or("/company").trim();
    if !(workdir == "/company" || workdir.starts_with("/company/")) || workdir.contains("..") {
        bail!("the job's workdir must be under /company");
    }
    let timeout = request.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS).clamp(1, MAX_TIMEOUT_SECS);
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
        "run", "-d", "--name", &name, "--label", "restless.job=1", "--label", &company_label, "--label",
        &actor_label, "--cpus", &cpus, "--cpu-shares", "256", "--pids-limit", "2048", "--log-driver",
        "local", "-u", "company", "-v", &volume, "-e", &command_env, "--entrypoint", "sh",
    ];
    if let Some(memory) = memory.as_deref() {
        args.extend(["--memory", memory, "--memory-swap", memory]);
    }
    args.extend([image.as_str(), "-c", script.as_str()]);
    let output = crate::runtime::docker_bounded(&args, std::time::Duration::from_secs(60)).await?;
    if !output.status.success() {
        bail!("the job could not start: {}", String::from_utf8_lossy(&output.stderr).trim());
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
pub async fn status(company: &str, id: &str) -> Result<serde_json::Value> {
    if !valid_id(id) {
        bail!("that is not a job id");
    }
    let name = job_container(company, id);
    let output = crate::runtime::docker_bounded(
        &["inspect", "-f", "{{.State.Status}} {{.State.ExitCode}} {{.State.StartedAt}} {{.State.FinishedAt}}", &name],
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
    let exit_code: i64 = fields.next().and_then(|code| code.parse().ok()).unwrap_or(-1);
    let started = fields.next().unwrap_or_default().to_string();
    let finished = fields.next().unwrap_or_default().to_string();
    if state == "running" || state == "created" {
        return Ok(serde_json::json!({ "job": id, "status": "running", "started_at": started, "log": log }));
    }
    let _ = crate::runtime::docker_bounded(&["rm", &name], std::time::Duration::from_secs(20)).await;
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
    fn a_job_request_names_only_known_fields() {
        let ok = super::request(Some(&serde_json::json!({"command": "make", "cpus": 2.0}))).unwrap();
        assert_eq!(ok.command, "make");
        assert!(super::request(Some(&serde_json::json!({"command": "x", "privileged": true}))).is_err());
        assert!(super::request(None).is_err());
    }
}
