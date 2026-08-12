//! Staff spawn and supervision (sprint 01 T9). The DECISION to spawn is the
//! Exec's (it names who and why in its termination envelope); the PROCESS is
//! the daemon's. One staff member = one supervised ACP process, one actor
//! row, one open commitment, one dedicated git worktree (§5.4 rule 1).
//!
//! The two signals are kept apart per the ticket's frame-2 note:
//! - process liveness is deterministic: a dead codex-acp is a crash,
//!   detected the moment the transport closes;
//! - "stalled or just thinking?" is judgement: no timeout kills a slow
//!   staff member. The Exec sees open staff commitments on its wakes and
//!   decides — resume, reassign, or leave be.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use anyhow::{Context as _, Result, bail};
use serde::Serialize;

use crate::acp::{self, AgentAuth};
use crate::exec::{self, Termination};
use crate::gateway::GatewayHandle;
use crate::runtime::{self, CompanyConfig};

/// Enough to produce handoff and crash friction without tripling token burn
/// across three companies (T9).
const STAFF_CAP_PER_COMPANY: usize = 2;
/// One staff task may continue in-session but is bounded like an exec turn.
const STAFF_TURN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20 * 60);

/// A spawn request from the Exec's termination envelope.
#[derive(Debug, Clone, serde::Deserialize, Serialize)]
pub struct SpawnRequest {
    pub name: String,
    pub task: String,
    /// Repository under /company/repos to give this staff a worktree of.
    pub repo: Option<String>,
}

/// (company, staff) pairs with a live supervised process. The registry is
/// the cap and the liveness book; the tasks remove themselves on exit.
#[derive(Clone, Default)]
pub struct StaffRegistry {
    running: Arc<Mutex<HashSet<(String, String)>>>,
}

impl StaffRegistry {
    fn try_claim(&self, company: &str, name: &str) -> Result<()> {
        let mut running = self.running.lock().map_err(|_| anyhow::anyhow!("staff registry"))?;
        if running.contains(&(company.to_string(), name.to_string())) {
            bail!("staff {name} is already running");
        }
        let active = running.iter().filter(|(c, _)| c == company).count();
        if active >= STAFF_CAP_PER_COMPANY {
            bail!("staff cap ({STAFF_CAP_PER_COMPANY}) reached for {company}");
        }
        running.insert((company.to_string(), name.to_string()));
        Ok(())
    }

    fn release(&self, company: &str, name: &str) {
        if let Ok(mut running) = self.running.lock() {
            running.remove(&(company.to_string(), name.to_string()));
        }
    }
}

/// A staff name (or repo name) becomes an actor id, a path, a branch name —
/// restrict it to what is safe to interpolate into all three.
fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 32
        && slug.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The daemon is a real correspondent in the company (refusals, crash
/// notices): its actor row must exist before its mail can land (the FK is
/// enforced), and a failed send is always logged, never dropped silently.
async fn mail_exec(org: &restless_orgintel::OrgIntel, body: &str) {
    if let Err(error) = org.add_actor("daemon", "daemon", "The daemon").await {
        tracing::warn!("failed to ensure daemon actor: {error:#}");
        return;
    }
    if let Err(error) = org.send_message("daemon", Some("exec"), body).await {
        tracing::warn!(%error, body, "failed to mail exec");
    }
}

/// Process the Exec's spawn requests at the end of a wake. Refusals and
/// crash notices reach the Exec through its inbox (which the T6 message
/// trigger turns into a wake); completions wake it via the commitment
/// trigger. The daemon never decides WHAT staff do — it runs what the Exec
/// asked for and reports what happened.
pub async fn process_spawns(
    config: &CompanyConfig,
    gateway: &GatewayHandle,
    org: &restless_orgintel::OrgIntel,
    registry: &StaffRegistry,
    requests: &[SpawnRequest],
) {
    for request in requests {
        if let Err(error) = spawn_one(config, gateway, org, registry, request).await {
            tracing::warn!(company = %config.name, staff = %request.name, "spawn refused: {error:#}");
            let note = format!("staff spawn refused ({}): {error:#}", request.name);
            mail_exec(org, &note).await;
        }
    }
}

async fn spawn_one(
    config: &CompanyConfig,
    gateway: &GatewayHandle,
    org: &restless_orgintel::OrgIntel,
    registry: &StaffRegistry,
    request: &SpawnRequest,
) -> Result<()> {
    // Validate everything before claiming the slot: a refusal must leave the
    // registry untouched.
    if !valid_slug(&request.name) {
        bail!("invalid staff name {:?}", request.name);
    }
    if request.task.trim().is_empty() {
        bail!("staff task is empty");
    }
    if let Some(repo) = &request.repo {
        if !valid_slug(repo) {
            bail!("invalid repo name {repo:?}");
        }
    }
    registry.try_claim(&config.name, &request.name)?;
    let spawned = spawn_claimed(config, gateway, org, registry, request).await;
    if spawned.is_err() {
        registry.release(&config.name, &request.name);
    }
    spawned
}

async fn spawn_claimed(
    config: &CompanyConfig,
    gateway: &GatewayHandle,
    org: &restless_orgintel::OrgIntel,
    registry: &StaffRegistry,
    request: &SpawnRequest,
) -> Result<()> {
    let actor = format!("staff-{}", request.name);
    org.add_actor(&actor, "staff", &request.name).await?;
    let title = request.task.lines().next().unwrap_or("staff task").trim().to_string();
    let commitment = org.add_commitment(&actor, &title, &request.task, None).await?;
    org.set_commitment_state(commitment, restless_orgintel::CommitmentState::Active, "")
        .await?;

    // The worktree exists before the process does, and survives it: crash
    // recovery is "the worktree is where the work is" (§5.4). If anything
    // fails BEFORE the process launches, the commitment must not stay
    // Active — there is no process whose outcome will ever move it.
    let setup = async {
        let workdir = if let Some(repo) = &request.repo {
            ensure_worktree(config, &request.name, repo).await?
        } else {
            "/company".to_string()
        };
        let auth = crate::exec::agent_auth(config)?;
        anyhow::Ok((workdir, auth))
    }
    .await;
    let (workdir, auth) = match setup {
        Ok(pair) => pair,
        Err(error) => {
            let note = format!("spawn failed before the process started: {error:#}");
            if let Err(e) = org
                .set_commitment_state(commitment, restless_orgintel::CommitmentState::Blocked, &note)
                .await
            {
                tracing::warn!("failed to block unlaunched staff commitment: {e:#}");
            }
            return Err(error);
        }
    };

    let company = config.name.clone();
    let name = request.name.clone();
    let task = request.task.clone();
    let container = runtime::container_name(&config.name);
    let org = org.clone();
    let registry = registry.clone();
    let meter = gateway.meter();
    let model = auth.model.clone();
    tokio::spawn(async move {
        let outcome = run_staff(&container, &auth, &workdir, &company, &actor, &name, &task).await;
        // Meter before recording the outcome: staff spend counts against the
        // company ceiling whether the task succeeded or not.
        if let Ok((_, _, spent)) = &outcome {
            for usage in spent {
                meter.record(&company, &model, usage.used, usage.cost_usd);
            }
        }
        let outcome = outcome.map(|(termination, reason, _)| (termination, reason));
        record_staff_outcome(&org, &actor, &name, commitment, &workdir, outcome).await;
        registry.release(&company, &name);
    });
    Ok(())
}

/// The staff turn: work the task, then the same judgement envelope as the
/// Exec; `continue` re-prompts inside the same session (one process per
/// task), bounded overall. Termination wording is the model's decision; the
/// envelope is the daemon's deterministic read of it.
async fn run_staff(
    container: &str,
    auth: &AgentAuth,
    workdir: &str,
    company: &str,
    actor: &str,
    name: &str,
    task: &str,
) -> Result<(Termination, String, Vec<acp::TurnUsage>)> {
    let prompt = format!(
        "You are {name}, a staff engineer of {company}, spawned for one task.\n\
         Your working directory is {workdir} — it is YOURS: a dedicated git worktree. \
         Commit meaningful checkpoints there with clear messages; do not touch other \
         worktrees or the main checkout.\n\n\
         # Task\n{task}\n\n\
         Work until the task is done or you are stuck. The session ends when you stop \
         writing; you will then be asked for a decision envelope."
    );
    const CONTINUE_PROMPT: &str =
        "Continue the task. If it is done or you are stuck, stop writing.";
    acp::with_agent(container, auth, workdir, actor, move |session| {
        Box::pin(async move {
            let deadline = tokio::time::Instant::now() + STAFF_TURN_TIMEOUT;
            let mut next = prompt;
            let mut spent: Vec<acp::TurnUsage> = Vec::new();
            loop {
                let prompted =
                    tokio::time::timeout_at(deadline, session.prompt(&next)).await;
                match prompted {
                    Err(_) => {
                        let _ = session.cancel().await;
                        bail!("staff turn exceeded {}s", STAFF_TURN_TIMEOUT.as_secs());
                    }
                    Ok(inner) => inner?,
                }
                // The work text is observability; the envelope is the record.
                // The usage is neither — it is the fuse's input, so it is the
                // one part of the transcript that must not be dropped. Staff
                // spend is real spend (two per company, T9).
                let worked = session.take_transcript();
                if let Some(usage) = worked.usage {
                    spent.push(usage);
                }
                let asked =
                    tokio::time::timeout_at(deadline, session.prompt(exec::TERMINATION_PROMPT)).await;
                match asked {
                    Err(_) => {
                        let _ = session.cancel().await;
                        bail!("staff turn exceeded {}s", STAFF_TURN_TIMEOUT.as_secs());
                    }
                    Ok(inner) => inner?,
                }
                let said = session.take_transcript().text;
                match exec::parse_termination(&said) {
                    Some(decision) if matches!(decision.termination, Termination::Continue) => {
                        next = CONTINUE_PROMPT.to_string();
                    }
                    Some(decision) => return Ok((decision.termination, decision.reason, spent)),
                    None => {
                        tracing::warn!(
                            said = %said.chars().take(600).collect::<String>(),
                            "staff termination unparseable"
                        );
                        return Ok((
                            Termination::Blocked,
                            "staff produced no parseable termination decision".to_string(),
                            spent,
                        ));
                    }
                }
            }
        })
    })
    .await
}

/// Record how a staff process ended. Done completes the commitment (the T6
/// trigger wakes the Exec with the result). Anything else marks it blocked
/// and mails the Exec. A dead process is a crash: detected by liveness,
/// commitment marked, worktree untouched, Exec mailed — resume or reassign
/// is the Exec's judgement, never the supervisor's.
async fn record_staff_outcome(
    org: &restless_orgintel::OrgIntel,
    actor: &str,
    name: &str,
    commitment: uuid::Uuid,
    workdir: &str,
    outcome: Result<(Termination, String)>,
) {
    let record = async {
        match outcome {
            Ok((Termination::Done, summary)) => {
                org.set_commitment_state(
                    commitment,
                    restless_orgintel::CommitmentState::Completed,
                    &summary,
                )
                .await?;
            }
            Ok((decision, summary)) => {
                org.set_commitment_state(
                    commitment,
                    restless_orgintel::CommitmentState::Blocked,
                    &format!("{decision:?}: {summary}"),
                )
                .await?;
                org.send_message(
                    actor,
                    Some("exec"),
                    &format!("staff {name} ended {:?}: {summary}", decision),
                )
                .await?;
            }
            Err(error) => {
                org.set_commitment_state(
                    commitment,
                    restless_orgintel::CommitmentState::Blocked,
                    &format!("crashed mid-turn: {error:#}"),
                )
                .await?;
                org.emit_event(
                    "staff_crash",
                    Some(actor),
                    serde_json::json!({ "error": format!("{error:#}"), "worktree": workdir }),
                )
                .await?;
                org.send_message(
                    actor,
                    Some("exec"),
                    &format!(
                        "staff {name} crashed mid-turn; worktree preserved at {workdir} with all \
                         commits and files intact. Resume it there or reassign — your call."
                    ),
                )
                .await?;
            }
        }
        anyhow::Ok(())
    };
    if let Err(error) = record.await {
        tracing::error!(staff = name, "failed to record staff outcome: {error:#}");
    }
}

/// Create (or reuse, after a crash) the staff worktree: a branch
/// `staff/<name>` of the given repo checked out at
/// /company/worktrees/<name>. Reuse is the crash-recovery path — files and
/// commits are never discarded.
async fn ensure_worktree(config: &CompanyConfig, name: &str, repo: &str) -> Result<String> {
    let container = runtime::container_name(&config.name);
    let path = format!("/company/worktrees/{name}");
    let script = format!(
        "if [ -f {path}/.git ]; then echo reused; \
         else mkdir -p /company/worktrees && \
              git -C /company/repos/{repo} worktree add {path} -b staff/{name} 2>/dev/null || \
              git -C /company/repos/{repo} worktree add {path} staff/{name}; fi"
    );
    let output = tokio::process::Command::new("docker")
        .args(["exec", "-u", "company", &container, "sh", "-c", &script])
        .output()
        .await
        .context("docker exec worktree")?;
    if !output.status.success() {
        bail!("worktree setup failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(path)
}

/// Boot sweep: any codex-acp still running in a company container when the
/// daemon starts is an orphan of the last daemon's lifetime — unsupervised,
/// its transcript unreachable. Kill it (deterministic liveness), mark open
/// staff commitments blocked with the worktree named, and mail the Exec.
/// Exec sessions orphaned the same way leave their milestone open; the next
/// wake continues it (T4).
pub async fn sweep_orphans(root: &std::path::Path, orgintel: &crate::OrgIntelRegistry) {
    let Ok(entries) = std::fs::read_dir(root.join("companies")) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else { continue };
        if !matches!(runtime::status(name).await, Ok(runtime::ContainerStatus::Running)) {
            continue;
        }
        let container = runtime::container_name(name);
        // Bracket pattern: a bare "codex-acp" would match this very wrapper
        // shell's own cmdline (sh -c '... codex-acp ...'), so detection
        // would always "find orphans" — observed: 28 false warns in one day.
        let found = tokio::process::Command::new("docker")
            .args(["exec", &container, "sh", "-c", "pgrep -f codex-ac[p]"])
            .output()
            .await;
        let Ok(found) = found else { continue };
        if !found.status.success() {
            continue; // no orphans
        }
        tracing::warn!(company = name, "killing orphaned agent processes from before restart");
        let _ = tokio::process::Command::new("docker")
            .args(["exec", &container, "sh", "-c", "pkill -9 -f codex-ac[p]"])
            .output()
            .await;
        let Ok(org) = orgintel.get(name).await else { continue };
        let Ok(commitments) = org.list_commitments().await else { continue };
        for c in commitments.iter().filter(|c| {
            c.owner_id.starts_with("staff-")
                && matches!(
                    c.state,
                    restless_orgintel::CommitmentState::Proposed
                        | restless_orgintel::CommitmentState::Active
                )
        }) {
            let note = "supervisor restarted; process lost, worktree preserved";
            if let Err(error) = org
                .set_commitment_state(c.id, restless_orgintel::CommitmentState::Blocked, note)
                .await
            {
                tracing::warn!("failed to mark orphaned staff commitment: {error:#}");
                continue;
            }
            let mail = format!(
                "staff {} was mid-task when the supervisor restarted; its worktree is intact. \
                 Resume or reassign as you see fit.",
                c.owner_id
            );
            mail_exec(&org, &mail).await;
        }
    }
}
