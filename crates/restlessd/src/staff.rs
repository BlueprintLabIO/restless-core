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

use anyhow::{bail, Context as _, Result};
use serde::Serialize;

use crate::acp::{self, AgentAuth};
use crate::exec::{self, Termination};
use crate::health;
use crate::runtime::{self, CompanyConfig};
use crate::spend::SpendLedger;

/// Enough to produce handoff and crash friction without tripling token burn
/// across three companies (T9).
const STAFF_CAP_PER_COMPANY: usize = 2;
/// A staff turn is bounded by liveness and budget, not wall-clock — the
/// module note above ("no timeout kills a slow staff member") was contradicted
/// by a 20-minute bound that did exactly that.
/// A spawn request from the Exec's termination envelope, or from
/// `restless spawn` mid-turn.
///
/// `role` and `model` are what make a team a team rather than a bigger context
/// window. Three sprints across three companies produced **zero** organic
/// delegation, and the cause was arithmetic rather than reluctance: every staff
/// member ran the Exec's own model under the generic role `"staff"`, so
/// delegating meant handing work to a copy of yourself with less context. It
/// bought parallelism and nothing else, and a rational Exec declined every
/// time. `orgintel §6.3` (**Core contract**, and item 12 of the §10.1 V0
/// acceptance list) has always required real teamwork patterns; this is the
/// field that lets one exist.
#[derive(Debug, Clone, serde::Deserialize, Serialize)]
pub struct SpawnRequest {
    pub name: String,
    pub task: String,
    /// Repository under /company/repos to give this staff a worktree of.
    pub repo: Option<String>,
    /// What this actor IS — `copywriter`, `critic`, `engineer`. Becomes the
    /// actor's durable kind, so "who did what" is answerable from OrgIntel
    /// rather than from a log. Defaults to `staff` when the Exec does not say,
    /// which keeps every existing call site working and is honest: an
    /// unspecified role is a generalist.
    #[serde(default)]
    pub role: Option<String>,
    /// Provider-qualified model for this role, e.g. `moonshot/kimi-k3`.
    /// Absent inherits the company's. Naming a model is not a budget: the
    /// company ceiling is unchanged and remains the only fuse.
    #[serde(default)]
    pub model: Option<String>,
}

impl SpawnRequest {
    /// The durable actor kind. Never the literal `"staff"` when a role was
    /// given — that string is what made three sprints of delegation invisible.
    #[must_use]
    pub fn role_or_default(&self) -> String {
        self.role
            .as_deref()
            .map(str::trim)
            .filter(|role| !role.is_empty())
            .unwrap_or("staff")
            .to_string()
    }
}

/// (company, staff) pairs with a live supervised process. The registry is
/// the cap and the liveness book; the tasks remove themselves on exit.
#[derive(Clone, Default)]
pub struct StaffRegistry {
    running: Arc<Mutex<HashSet<(String, String)>>>,
}

impl StaffRegistry {
    fn try_claim(&self, company: &str, name: &str) -> Result<()> {
        let mut running = self
            .running
            .lock()
            .map_err(|_| anyhow::anyhow!("staff registry"))?;
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

    /// Whether this durable actor currently has a supervised process. The
    /// actor id is what OrgIntel owns; the registry's internal key remains the
    /// short staff name used for its worktree and process.
    pub fn is_actor_running(&self, company: &str, actor: &str) -> bool {
        let Some(name) = actor.strip_prefix("staff-") else {
            return false;
        };
        self.running
            .lock()
            .map(|running| running.contains(&(company.to_string(), name.to_string())))
            .unwrap_or(false)
    }

    /// Actor ids currently supervised for one company. Runtime replacement
    /// refuses while this is non-empty; the owner chooses `down` explicitly
    /// rather than an image refresh silently killing useful work.
    pub fn running_actors(&self, company: &str) -> Vec<String> {
        self.running
            .lock()
            .map(|running| {
                let mut actors: Vec<String> = running
                    .iter()
                    .filter(|(running_company, _)| running_company == company)
                    .map(|(_, name)| format!("staff-{name}"))
                    .collect();
                actors.sort();
                actors
            })
            .unwrap_or_default()
    }
}

/// A staff name (or repo name) becomes an actor id, a path, a branch name —
/// restrict it to what is safe to interpolate into all three.
fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 32
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
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
    spend: &SpendLedger,
    org: &restless_orgintel::OrgIntel,
    registry: &StaffRegistry,
    requests: &[SpawnRequest],
) {
    for request in requests {
        if let Err(error) = spawn_one(config, spend, org, registry, request).await {
            tracing::warn!(company = %config.name, staff = %request.name, "spawn refused: {error:#}");
            let note = format!("staff spawn refused ({}): {error:#}", request.name);
            mail_exec(org, &note).await;
        }
    }
}

/// Spawn one staff member and report the outcome to the caller. The Exec
/// asks for this directly (S02-T2), so a refusal belongs in the reply — not
/// mailed back later, detached from the decision that caused it.
pub async fn spawn_now(
    config: &CompanyConfig,
    spend: &SpendLedger,
    org: &restless_orgintel::OrgIntel,
    registry: &StaffRegistry,
    request: &SpawnRequest,
) -> Result<()> {
    spawn_one(config, spend, org, registry, request).await
}

async fn spawn_one(
    config: &CompanyConfig,
    spend: &SpendLedger,
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
    let spawned = spawn_claimed(config, spend, org, registry, request).await;
    if spawned.is_err() {
        registry.release(&config.name, &request.name);
    }
    spawned
}

async fn spawn_claimed(
    config: &CompanyConfig,
    spend: &SpendLedger,
    org: &restless_orgintel::OrgIntel,
    registry: &StaffRegistry,
    request: &SpawnRequest,
) -> Result<()> {
    let actor = format!("staff-{}", request.name);
    // S04-T9. The model is persisted with the actor, so "which model wrote
    // this" is answerable from OrgIntel rather than from a process that has
    // already exited. Resolved the same way `agent_auth` resolves it below —
    // the role's model when it names one, the company's otherwise.
    let model = request
        .model
        .clone()
        .unwrap_or_else(|| config.model.clone());
    org.add_actor_with_model(
        &actor,
        &request.role_or_default(),
        &request.name,
        Some(&model),
    )
    .await?;
    let title = request
        .task
        .lines()
        .next()
        .unwrap_or("staff task")
        .trim()
        .to_string();
    let commitment = org
        .add_commitment(&actor, &title, &request.task, None)
        .await?;
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
        // A role may bring its own model. This is the whole point of the
        // ticket: a critic that runs the same model as the producer, on the
        // same context, is an echo chamber with a second invoice.
        let auth = match &request.model {
            Some(model) => crate::exec::agent_auth(&CompanyConfig {
                model: model.clone(),
                ..config.clone()
            })?,
            None => crate::exec::agent_auth(config)?,
        };
        anyhow::Ok((workdir, auth))
    }
    .await;
    let (workdir, auth) = match setup {
        Ok(pair) => pair,
        Err(error) => {
            let note = format!("spawn failed before the process started: {error:#}");
            if let Err(e) = org
                .set_commitment_state(
                    commitment,
                    restless_orgintel::CommitmentState::Blocked,
                    &note,
                )
                .await
            {
                tracing::warn!("failed to block unlaunched staff commitment: {e:#}");
            }
            return Err(error);
        }
    };

    // Every worker gets the shared spine: mission, plan, open commitments, and
    // what the company already knows. This used to be the independent variable
    // of the three-mode comparison, which was retired when its arms turned out
    // not to be distinct — two of the three were byte-identical. Withholding
    // context from a worker is not a configuration anyone should be able to
    // choose; it was a measurement apparatus, and the measurement is over.
    let spine = shared_spine(config, org).await;
    let company = config.name.clone();
    let name = request.name.clone();
    let task = request.task.clone();
    let container = runtime::container_name(&config.name);
    let org = org.clone();
    let registry = registry.clone();
    let meter = spend.meter();
    let model = auth.model.clone();
    let role = request.role_or_default();
    tokio::spawn(async move {
        let outcome = run_staff(StaffBrief {
            container,
            auth,
            workdir: workdir.clone(),
            company: company.clone(),
            actor: actor.clone(),
            name: name.clone(),
            task,
            role,
            spine,
        })
        .await;
        // Meter before recording the outcome: staff spend counts against the
        // company ceiling whether the task succeeded or not.
        if let Ok((_, _, spent)) = &outcome {
            for usage in spent {
                meter.record(&company, &actor, &model, usage.used, usage.cost_usd);
            }
        }
        let outcome = outcome.map(|(termination, reason, _)| (termination, reason));
        record_staff_outcome(&org, &actor, &name, commitment, &workdir, outcome).await;
        registry.release(&company, &name);
    });
    Ok(())
}

/// What a worker needs to know about the company it works for, beyond its own
/// task: the mission, the plan the Exec is working to, and what else is open.
/// `docs/specs/orgintel.md` §5.2 — shared spine plus local depth.
async fn shared_spine(config: &CompanyConfig, org: &restless_orgintel::OrgIntel) -> String {
    let mut spine = format!("\n# The company you work for\n{}\n", config.mission.trim());
    match org.list_commitments().await {
        Ok(commitments) => {
            let open: Vec<String> = commitments
                .iter()
                .filter(|c| {
                    matches!(
                        c.state,
                        restless_orgintel::CommitmentState::Active
                            | restless_orgintel::CommitmentState::Blocked
                    )
                })
                .map(|c| {
                    format!(
                        "- [{}] {} (owner: {})",
                        format!("{:?}", c.state).to_lowercase(),
                        c.title,
                        c.owner_id
                    )
                })
                .collect();
            if !open.is_empty() {
                spine.push_str(&format!(
                    "\n# Also in flight — do not duplicate or collide with these\n{}\n",
                    open.join("\n")
                ));
            }
        }
        Err(error) => tracing::warn!(%error, "could not read commitments for the staff spine"),
    }
    spine.push_str(
        "\nYou can reach the company: `restless message --to exec \"...\"` to raise a blocker or \
         ask a question, and `restless commitment blocked <id> --resolution \"...\"` if you cannot \
         proceed. Say so early rather than guessing.\n",
    );
    spine
}

/// The staff turn: work the task, then the same judgement envelope as the
/// Exec; `continue` re-prompts inside the same session (one process per
/// task), bounded overall. Termination wording is the model's decision; the
/// envelope is the daemon's deterministic read of it.
/// Everything one supervised staff turn needs. Grouped because the parameter
/// list had grown past the point where call sites were readable — and because
/// `spine` being present or empty is the OrgIntel comparison's independent
/// variable, which deserves to be visible in a type rather than buried as the
/// eighth positional argument.
struct StaffBrief {
    container: String,
    auth: AgentAuth,
    workdir: String,
    company: String,
    actor: String,
    name: String,
    task: String,
    /// What this actor IS. Reaches the agent's own briefing, so the
    /// specialisation is something it knows about itself rather than a label
    /// only the daemon can see.
    role: String,
    /// The shared spine, or empty in `minimal_team` and `single_agent`.
    spine: String,
}

/// A staff turn that did not run to completion, as one sentence — or `None`
/// when it did. Staff go through the same total classifier as the Exec so that
/// a wedge is reported as a wedge here too; they differ only in what they can
/// do about it, which is nothing.
fn staff_halt(end: &acp::TurnEnd) -> Option<String> {
    match health::classify(end) {
        health::Verdict::Ran => None,
        health::Verdict::Resume(reason) => Some(reason),
        health::Verdict::Blocked(blocked) => Some(blocked.message()),
    }
}

async fn run_staff(brief: StaffBrief) -> Result<(Termination, String, Vec<acp::TurnUsage>)> {
    let StaffBrief {
        container,
        auth,
        workdir,
        company,
        actor,
        name,
        task,
        role,
        spine,
    } = brief;
    let (container, auth, workdir, actor) =
        (container.as_str(), &auth, workdir.as_str(), actor.as_str());
    let prompt = format!(
        "You are {name}, the {role} of {company}, spawned for one task.\n\
         You are a SPECIALIST, not a smaller Exec. Do the job your role names and \
         say so plainly when something falls outside it — a specialist who quietly \
         does everything is a generalist with a job title, and the reason you exist \
         is that one actor doing every job did one of them badly.\n\
         Your working directory is {workdir} — it is YOURS: a dedicated git worktree. \
         Commit meaningful checkpoints there with clear messages; do not touch other \
         worktrees or the main checkout.\n\
         {spine}\n\
         # Task\n{task}\n\n\
         Work until the task is done or you are stuck. The session ends when you stop \
         writing; you will then be asked for a decision envelope."
    );
    const CONTINUE_PROMPT: &str =
        "Continue the task. If it is done or you are stuck, stop writing.";
    acp::with_agent(container, auth, workdir, actor, move |session| {
        Box::pin(async move {
            let mut next = prompt;
            let mut spent: Vec<acp::TurnUsage> = Vec::new();
            loop {
                let end = session.prompt_live(&next, |_| false).await;
                // The work text is observability; the envelope is the record.
                // The usage is neither — it is the fuse's input, so it is the
                // one part of the transcript that must not be dropped, and it
                // is dropped LAST: a staff member that wedged or failed still
                // spent the company's money, and billing only the clean path
                // is how spend goes quietly missing. Staff spend is real spend
                // (two per company, T9).
                if let Some(usage) = end.usage() {
                    spent.push(usage);
                }
                // Staff report to the Exec, not to the owner, so they have no
                // Verdict of their own to act on — the whole run is abandoned
                // and the Exec reads the reason on its next wake. But the
                // reason must still be the specific one.
                if let Some(blocked) = staff_halt(&end) {
                    bail!("staff turn: {blocked}");
                }

                let end = session
                    .prompt_live(exec::TERMINATION_PROMPT, |_| false)
                    .await;
                if let Some(usage) = end.usage() {
                    spent.push(usage);
                }
                if let Some(blocked) = staff_halt(&end) {
                    bail!("staff termination ask: {blocked}");
                }
                let said = end.into_transcript().text;
                match exec::parse_termination(&said) {
                    Some(decision) if matches!(decision.termination, Termination::Continue) => {
                        next = CONTINUE_PROMPT.to_string();
                    }
                    Some(decision) => return Ok((decision.termination, decision.reason, spent)),
                    None => {
                        // Before blaming the model, check whether it spoke at
                        // all. omp streams an upstream error body through as
                        // message CONTENT, so a provider refusal arrives as
                        // assistant text: the turn "succeeds", tokens are
                        // consumed, and nothing in the transport looks wrong.
                        //
                        // This is F1 in its third costume. It was fixed for the
                        // Exec path in sprint 02 and the identical gap sat here
                        // untouched until a critic spawned on a second provider
                        // hit `429 [1113] Insufficient balance` and was reported
                        // as "staff produced no parseable termination decision"
                        // — which blames the specialist for the wallet.
                        if let Some(blocked) = health::classify_provider_error(&said) {
                            tracing::warn!(
                                kind = blocked.kind.as_str(),
                                "staff blocked by the provider, not by its own output"
                            );
                            return Ok((Termination::Blocked, blocked.message(), spent));
                        }
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
        bail!(
            "worktree setup failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(path)
}

/// Boot sweep: any marked ACP session still running in a company container
/// when the daemon starts is an orphan of the last daemon's lifetime —
/// unsupervised, its transcript unreachable. Reap only those Linux sessions,
/// mark open staff commitments blocked with the worktree named, and mail the
/// Exec.
/// Exec sessions orphaned the same way leave their milestone open; the next
/// wake continues it (T4).
pub async fn sweep_orphans(root: &std::path::Path, orgintel: &crate::OrgIntelRegistry) {
    let Ok(entries) = std::fs::read_dir(root.join("companies")) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if !matches!(
            runtime::status(name).await,
            Ok(runtime::ContainerStatus::Running)
        ) {
            continue;
        }
        let container = runtime::container_name(name);
        let reaped = crate::acp::reap_orphan_sessions(&container).await;
        if reaped == 0 {
            continue; // no orphans
        }
        tracing::warn!(
            company = name,
            reaped,
            "reaped marked agent processes from before restart"
        );
        let Ok(org) = orgintel.get(name).await else {
            continue;
        };
        let Ok(commitments) = org.list_commitments().await else {
            continue;
        };
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

#[cfg(test)]
mod role_tests {
    use super::*;

    fn request(role: Option<&str>) -> SpawnRequest {
        SpawnRequest {
            name: "critic".to_string(),
            task: "find what is wrong with the draft".to_string(),
            repo: None,
            role: role.map(str::to_string),
            model: None,
        }
    }

    /// The literal `"staff"` on every actor is what made three sprints of
    /// delegation invisible: the owner could not ask who did what, because
    /// everyone was the same thing. A named role must survive into the actor
    /// row unchanged.
    #[test]
    fn a_named_role_is_never_flattened_to_staff() {
        assert_eq!(request(Some("copywriter")).role_or_default(), "copywriter");
        assert_eq!(request(Some("  critic  ")).role_or_default(), "critic");
    }

    /// Absent is honest, not broken: an unspecified role is a generalist, and
    /// every pre-S04 call site keeps working.
    #[test]
    fn an_absent_or_blank_role_is_a_generalist() {
        assert_eq!(request(None).role_or_default(), "staff");
        assert_eq!(request(Some("   ")).role_or_default(), "staff");
    }

    /// Old envelopes must still parse. The Exec has two sprints of habit
    /// writing spawn requests without these fields, and a decision that fails
    /// to deserialise takes the whole wake with it.
    #[test]
    fn a_pre_s04_spawn_envelope_still_parses() {
        let old = serde_json::json!({ "name": "builder", "task": "write it", "repo": "study" });
        let parsed: SpawnRequest = serde_json::from_value(old).expect("old envelope must parse");
        assert_eq!(parsed.role_or_default(), "staff");
        assert!(parsed.model.is_none());
        // And the new shape carries both through.
        let new = serde_json::json!({
            "name": "critic", "task": "critique", "role": "critic",
            "model": "anthropic/claude-sonnet-4"
        });
        let parsed: SpawnRequest = serde_json::from_value(new).expect("new envelope must parse");
        assert_eq!(parsed.role_or_default(), "critic");
        assert_eq!(parsed.model.as_deref(), Some("anthropic/claude-sonnet-4"));
    }
}
