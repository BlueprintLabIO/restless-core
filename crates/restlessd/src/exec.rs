//! The persistent Exec (sprint 01 T4). Identity is durable — an actor row
//! plus `/company/org/exec/` (current-plan.md, journal/NNNN.md) — and the
//! ACP session is disposable: each wake spawns a fresh one and rehydrates
//! from files + OrgIntel. A kill mid-turn loses at most the in-flight turn;
//! the next wake continues the milestone rather than restarting it.
//!
//! Termination is a model decision (judgement over an open-ended turn,
//! enumerable output — LLM_CURE.md frame 2), never a turn-count or timer.

use anyhow::{Context, Result, bail};
use restless_orgintel::{CommitmentState, OrgIntel};
use serde::Serialize;
use tokio::io::AsyncWriteExt as _;
use uuid::Uuid;

use crate::acp::{self, AgentSession, GatewayAuth};
use crate::gateway::GatewayHandle;
use crate::runtime::{self, CompanyConfig};

/// Longest single work turn before the wake boundary is enforced. A timeout
/// is not a termination decision: the next wake rehydrates and continues.
const WORK_TURN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Termination {
    Continue,
    Blocked,
    Done,
    Abandon,
}

#[derive(Debug, Serialize)]
pub struct WakeReport {
    pub company: String,
    pub termination: Termination,
    pub reason: String,
    /// Model-suggested delay before the next wake (continue only).
    pub next_wake_minutes: Option<u32>,
    /// Tool calls the Exec made this turn (observability).
    pub tool_calls: Vec<String>,
    /// The Exec's closing text this turn, truncated.
    pub said: String,
}

/// Raw model output for the termination decision.
#[derive(Debug, serde::Deserialize)]
struct TerminationOutput {
    decision: String,
    reason: String,
    next_wake_minutes: Option<u32>,
}

/// One Exec wake: rehydrate → work turn → termination decision → record.
pub async fn wake(
    config: &CompanyConfig,
    gateway: &GatewayHandle,
    org: &OrgIntel,
    reason: &str,
) -> Result<WakeReport> {
    let container = runtime::container_name(&config.name);
    org.add_actor("exec", "exec", "The Exec").await?;
    org.add_actor("owner", "owner", "The Owner").await?;
    seed_codex_config(&container, &config.model).await?;
    let milestone = ensure_milestone(org, config).await?;

    let minted = gateway.mint_token(config, "exec")?;
    let auth = GatewayAuth { base_url: minted.base_url_container, token: minted.token };
    let context = assemble_context(&container, org, config, reason).await?;
    org.emit_event("wake", Some("exec"), serde_json::json!({ "reason": reason })).await?;

    let report = acp::with_agent(&container, &auth, "/company", {
        let company = config.name.clone();
        move |session| {
            Box::pin(async move { run_turn(session, &context, &company).await })
        }
    })
    .await?;

    record_outcome(org, milestone, &report).await?;
    Ok(report)
}

/// The full turn inside one ACP session: work prompt, then the termination
/// decision as a second prompt on the same session (it has full context).
async fn run_turn(
    session: &AgentSession,
    context: &str,
    company: &str,
) -> Result<WakeReport> {
    let worked = tokio::time::timeout(WORK_TURN_TIMEOUT, session.prompt(context)).await;
    if worked.is_err() {
        // The wake boundary is system-imposed (§6), not a termination: the
        // process dies here, the next wake continues from files + OrgIntel.
        let _ = session.cancel().await;
        bail!("work turn exceeded {}s", WORK_TURN_TIMEOUT.as_secs());
    }
    worked??;
    let work_transcript = session.take_transcript();

    let (termination, reason, next_wake_minutes) = termination_decision(session).await?;
    let said: String = work_transcript.text.chars().take(1_000).collect();
    Ok(WakeReport {
        company: company.to_string(),
        termination,
        reason,
        next_wake_minutes,
        tool_calls: work_transcript.tool_calls,
        said,
    })
}

/// Ask the Exec to end the turn explicitly. One retry on an unparseable
/// envelope; a second failure is blocked-on-owner (surface, never spin).
async fn termination_decision(
    session: &AgentSession,
) -> Result<(Termination, String, Option<u32>)> {
    const PROMPT: &str = "The turn is ending now. Based on everything above, decide how this \
        milestone stands and answer with JSON only, no prose:\n\
        {\"decision\": \"continue\" | \"blocked\" | \"done\" | \"abandon\", \
         \"reason\": \"<one line>\", \
         \"next_wake_minutes\": <integer, only when continue>}\n\
        - continue: more machine-doable work remains\n\
        - blocked: you need the owner's judgement, authority, identity, or a decision only a \
          human can make — say exactly what you need in reason\n\
        - done: the milestone's stated outcome is met\n\
        - abandon: the milestone is not worth continuing — say why";
    for attempt in 0..2 {
        session.prompt(PROMPT).await?;
        let transcript = session.take_transcript();
        match parse_termination(&transcript.text) {
            Some(parsed) => return Ok(parsed),
            None if attempt == 0 => {
                // The transcript carries the model's actual words — without
                // them an unparseable decision is undebuggable (Sprint 01
                // friction: the first silent failure cost a full probe cycle).
                tracing::warn!(
                    said = %transcript.text.chars().take(600).collect::<String>(),
                    "termination decision unparseable; retrying once"
                );
                continue;
            }
            None => {
                return Ok((
                    Termination::Blocked,
                    "exec produced no parseable termination decision twice".to_string(),
                    None,
                ));
            }
        }
    }
    unreachable!()
}

/// Parse the termination envelope. The decision itself was the model's; this
/// is deterministic envelope handling — find the JSON object, no prose
/// interpretation (LLM_CURE.md frame 2).
fn parse_termination(text: &str) -> Option<(Termination, String, Option<u32>)> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    let output: TerminationOutput = serde_json::from_str(&text[start..=end]).ok()?;
    let termination = match output.decision.as_str() {
        "continue" => Termination::Continue,
        "blocked" | "blocked_on_owner" => Termination::Blocked,
        "done" => Termination::Done,
        "abandon" => Termination::Abandon,
        _ => return None,
    };
    Some((termination, output.reason, output.next_wake_minutes))
}

/// Rehydration bundle: mission, current plan, latest journal entry, open
/// commitments, unread inbox, and the wake reason. Files win over memory —
/// this is all the continuity the Exec gets, so it is all here.
async fn assemble_context(
    container: &str,
    org: &OrgIntel,
    config: &CompanyConfig,
    reason: &str,
) -> Result<String> {
    let plan = read_company_file(container, "/company/org/exec/current-plan.md").await?;
    let journal = latest_journal_entry(container).await?;
    let commitments = org.list_commitments().await?;
    let open: Vec<_> = commitments
        .iter()
        .filter(|c| matches!(c.state, CommitmentState::Proposed | CommitmentState::Active | CommitmentState::Blocked))
        .collect();
    let inbox = org.inbox(Some("exec")).await?;

    let mut open_listing = String::new();
    for c in &open {
        open_listing.push_str(&format!(
            "- [{}] {} ({}): {}\n",
            format!("{:?}", c.state).to_lowercase(),
            c.title,
            c.owner_id,
            c.body
        ));
    }
    let mut inbox_listing = String::new();
    for message in &inbox {
        inbox_listing.push_str(&format!("- from {}: {}\n", message.from_actor, message.body));
    }

    Ok(format!(
        "You are the Exec of {name} — the singleton chief executive of this autonomous company.\n\
         You run in wakes. You persist ONLY through files and the coordination store, never \
         through memory: anything you do not write down is lost.\n\n\
         # Mission (owner-set, /company/mission.md)\n{mission}\n\n\
         # Your continuity\n\
         - /company/org/exec/current-plan.md — your ONE current plan. It exists: {plan_exists}. \
           Read it first; update it in place as work progresses; never start a second plan for \
           the same milestone.\n\
         - /company/org/exec/journal/NNNN.md — one entry per wake, next sequential number. \
           Record what you did, learned, and what is next.\n\
         - /company/repos — project repositories; commit meaningful checkpoints with git.\n\
         - /company/outputs — finished artifacts for the owner.\n\n\
         # Current plan\n{plan}\n\n\
         # Latest journal entry\n{journal}\n\n\
         # Open commitments (coordination store)\n{open_listing}\n\
         # Inbox\n{inbox_listing}\n\
         # This wake\n{reason}\n\n\
         Work this turn. Use the tools. Write files. Stop when the turn's work is done.",
        name = config.name,
        mission = config.mission,
        plan_exists = if plan.trim().is_empty() { "no — first wake, create it" } else { "yes" },
        plan = if plan.trim().is_empty() { "(none yet)" } else { plan.trim() },
        journal = journal.trim(),
        open_listing = if open_listing.is_empty() { "(none)\n".to_string() } else { open_listing },
        inbox_listing = if inbox_listing.is_empty() { "(empty)\n".to_string() } else { inbox_listing },
    ))
}

/// Record the wake's outcome: always an event; blocked also messages the
/// owner inbox; done/abandon resolve the milestone commitment.
async fn record_outcome(org: &OrgIntel, milestone: Uuid, report: &WakeReport) -> Result<()> {
    org.emit_event(
        "wake_end",
        Some("exec"),
        serde_json::json!({
            "termination": report.termination,
            "reason": report.reason,
            "tool_calls": report.tool_calls.len(),
        }),
    )
    .await?;
    match report.termination {
        Termination::Blocked => {
            org.send_message("exec", None, &format!("blocked: {}", report.reason)).await?;
        }
        Termination::Done => {
            org.set_commitment_state(milestone, CommitmentState::Completed, &report.reason)
                .await?;
        }
        Termination::Abandon => {
            org.set_commitment_state(milestone, CommitmentState::Abandoned, &report.reason)
                .await?;
        }
        Termination::Continue => {}
    }
    Ok(())
}

/// The milestone commitment: one open commitment owned by the Exec, created
/// on first wake and reused thereafter — a kill never resets it (T4
/// acceptance).
async fn ensure_milestone(org: &OrgIntel, config: &CompanyConfig) -> Result<Uuid> {
    for commitment in org.list_commitments().await? {
        if commitment.owner_id == "exec"
            && matches!(
                commitment.state,
                CommitmentState::Proposed | CommitmentState::Active | CommitmentState::Blocked
            )
        {
            return Ok(commitment.id);
        }
    }
    let title = config.mission.lines().find(|line| !line.trim().is_empty()).unwrap_or("milestone").trim_start_matches('#').trim().to_string();
    let id = org
        .add_commitment("exec", &format!("milestone: {title}"), &config.mission, None)
        .await?;
    org.set_commitment_state(id, CommitmentState::Active, "").await?;
    Ok(id)
}

/// codex inside the container asks for the gateway-routed adapter model; the
/// provider selection ("custom-gateway") and its full definition (base URL,
/// headers carrying the purpose token) are injected per wake by the ACP
/// gateway auth. config.toml must NOT name model_provider: codex validates
/// it at load, before the ACP merge lands. No key or token ever touches
/// this file.
async fn seed_codex_config(container: &str, model: &str) -> Result<()> {
    let config = format!("model = \"{model}\"\n");
    exec_stdin(
        container,
        "mkdir -p /company/home/.codex && cat > /company/home/.codex/config.toml",
        &config,
    )
    .await
}

async fn read_company_file(container: &str, path: &str) -> Result<String> {
    let output = exec_output(container, &format!("cat {path} 2>/dev/null || true")).await?;
    Ok(output)
}

/// The most recent journal entry's filename and content, for rehydration.
async fn latest_journal_entry(container: &str) -> Result<String> {
    let output = exec_output(
        container,
        "cd /company/org/exec/journal 2>/dev/null && ls | sort | tail -1 | xargs -r sh -c 'echo \"== $0 ==\"; cat \"$0\"' || true",
    )
    .await?;
    Ok(if output.trim().is_empty() { "(none yet — first wake)".to_string() } else { output })
}

async fn exec_output(container: &str, shell: &str) -> Result<String> {
    let output = tokio::process::Command::new("docker")
        .args(["exec", "-u", "company", container, "sh", "-c", shell])
        .output()
        .await
        .context("docker exec")?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run a shell command with piped stdin (file contents never touch the
/// command line, so no escaping games).
async fn exec_stdin(container: &str, shell: &str, input: &str) -> Result<()> {
    let mut child = tokio::process::Command::new("docker")
        .args(["exec", "-i", "-u", "company", container, "sh", "-c", shell])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("docker exec stdin")?;
    let mut stdin = child.stdin.take().expect("piped");
    stdin.write_all(input.as_bytes()).await?;
    drop(stdin);
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        bail!("exec in {container} failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}
