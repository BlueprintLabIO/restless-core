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
use uuid::Uuid;

use crate::acp::{self, AgentSession};
use crate::context::{self, ContextSnapshot};
use crate::spend::SpendLedger;
use crate::health;
use crate::runtime::{self, CompanyConfig};
use crate::staff::SpawnRequest;

/// Bound on the end-of-turn ask: the answer is one line of JSON, so a
/// timeout means the agent wedged (e.g. launched a hanging tool) rather
/// than that the decision needs more thought. Without a bound, a wedged
/// termination turn holds the wake guard forever and silently stops the
/// company's scheduling — the same failure family as the work-turn timeout
/// above exists to bound.
const TERMINATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2 * 60);

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
    /// Staff the Exec asked to spawn this turn (T9); the daemon processes
    /// these after the outcome is recorded.
    pub spawn_requests: Vec<SpawnRequest>,
}

/// Raw model output for the termination decision. `spawn` is deliberately
/// untyped here: a malformed spawn entry must never kill the whole decision
/// (envelope handling is deterministic; the judgement was the model's).
#[derive(Debug, serde::Deserialize)]
struct TerminationOutput {
    decision: String,
    reason: String,
    next_wake_minutes: Option<u32>,
    spawn: Option<serde_json::Value>,
}

/// The parsed end-of-turn decision (T4) plus any staff spawn requests (T9).
pub(crate) struct TerminationDecision {
    pub termination: Termination,
    pub reason: String,
    pub next_wake_minutes: Option<u32>,
    pub spawn: Vec<SpawnRequest>,
}

/// One Exec wake: rehydrate → work turn → termination decision → record.
pub async fn wake(
    config: &CompanyConfig,
    spend: &SpendLedger,
    org: &OrgIntel,
    reason: &str,
) -> Result<WakeReport> {
    let container = runtime::container_name(&config.name);
    org.add_actor("exec", "exec", "The Exec").await?;
    org.add_actor("owner", "owner", "The Owner").await?;
    let milestone = ensure_milestone(org, config).await?;

    // Preflight: a company whose computer is stopped or whose disk is full
    // must not be woken. Nothing below this line is free — context assembly
    // reads the volume and the turn spends money — so the cheap deterministic
    // checks come first (F2, F3, F12).
    if let Some(blocked) = health::preflight(&config.name).await? {
        return blocked_wake(org, milestone, config, &blocked.message()).await;
    }
    if let Some((spent, ceiling)) = spend.over_ceiling(config) {
        return blocked_wake(
            org,
            milestone,
            config,
            &format!(
                "[budget] {} has spent ${spent:.2} of its ${ceiling:.2} ceiling; \
                 the owner must raise it before work continues",
                config.name
            ),
        )
        .await;
    }

    let auth = agent_auth(config)?;
    // T7: gather the snapshot (IO), then assemble (pure, digested). The
    // digest lands in the wake event so the Exec's worldview is auditable.
    let snapshot =
        gather_snapshot(&container, org, config, reason, spend.spent_usd(&config.name)).await?;
    let package = context::assemble(&snapshot);
    org.emit_event(
        "wake",
        Some("exec"),
        serde_json::json!({ "reason": reason, "context_digest": package.digest }),
    )
    .await?;

    let outcome = acp::with_agent(&container, &auth, "/company", "exec", {
        let company = config.name.clone();
        let remaining = (config.spend_ceiling_usd - spend.spent_usd(&config.name)).max(0.0);
        move |session| {
            Box::pin(async move { run_turn(session, &package.text, &company, remaining).await })
        }
    })
    .await;

    // Postflight. A transport failure and a turn that consumed nothing are
    // both substrate failures, not the Exec's judgement — classify them
    // deterministically rather than letting them arrive as prose the
    // termination parser then fails on (F1).
    let (report, usage) = match outcome {
        Ok((report, usage)) => (report, usage),
        Err(error) => {
            let text = format!("{error:#}");
            let blocked = health::classify_turn(None, Some(&text))
                .unwrap_or_else(|| health::Blocked::transport(&text));
            return blocked_wake(org, milestone, config, &blocked.message()).await;
        }
    };

    if let Some(usage) = usage {
        spend.record_turn(&config.name, &auth.model, usage.used, usage.cost_usd);
        // Cost per outcome is the sprint's headline number and was missing
        // from every run report: emit it where the run can read it back.
        org.emit_event(
            "turn_usage",
            Some("exec"),
            serde_json::json!({
                "model": auth.model,
                "tokens": usage.used,
                "context_size": usage.size,
                "context_used_pct": percent(usage.used, usage.size),
                "cost_usd": usage.cost_usd,
            }),
        )
        .await?;
    }
    if let Some(blocked) = health::classify_turn(usage.map(|usage| usage.used), None) {
        return blocked_wake(org, milestone, config, &blocked.message()).await;
    }

    record_outcome(org, milestone, &report).await?;
    Ok(report)
}

/// Context utilisation, rounded. Sprint 01 burned 95% of its dollars on
/// replayed context without anyone able to see it happening.
fn percent(used: u64, size: u64) -> u64 {
    if size == 0 { 0 } else { used.saturating_mul(100) / size }
}

/// The substrate failed, so the company is blocked on something only the
/// owner can change. One latched milestone, one mail — never a re-wake loop
/// (F1: 20 identical mails in 3h before the latch existed).
async fn blocked_wake(
    org: &OrgIntel,
    milestone: Uuid,
    config: &CompanyConfig,
    reason: &str,
) -> Result<WakeReport> {
    tracing::warn!(company = %config.name, reason, "wake blocked by health gate");
    let report = WakeReport {
        company: config.name.clone(),
        termination: Termination::Blocked,
        reason: reason.to_string(),
        next_wake_minutes: None,
        tool_calls: Vec::new(),
        said: String::new(),
        spawn_requests: Vec::new(),
    };
    record_outcome(org, milestone, &report).await?;
    Ok(report)
}

/// Resolve the provider credential for this company's model. The value is
/// read from the daemon's own environment and handed to the agent process
/// through `docker exec -e`; it is never written to the image, the
/// container's persistent environment, or the volume.
pub(crate) fn agent_auth(config: &CompanyConfig) -> Result<acp::AgentAuth> {
    let (provider, _) = config
        .model
        .split_once('/')
        .with_context(|| format!("model {} must be provider-qualified, e.g. zai/glm-5.2", config.model))?;
    let key_env = match provider {
        "zai" => "ZAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "moonshot" => "KIMI_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        other => bail!("no credential mapping for provider {other}"),
    };
    let provider_key = std::env::var(key_env)
        .with_context(|| format!("{key_env} must be set for model {}", config.model))?;
    Ok(acp::AgentAuth {
        model: config.model.clone(),
        provider_key_env: key_env.to_string(),
        provider_key,
    })
}

/// The full turn inside one ACP session: work prompt, then the termination
/// decision as a second prompt on the same session (it has full context).
async fn run_turn(
    session: &AgentSession,
    context: &str,
    company: &str,
    remaining_budget_usd: f64,
) -> Result<(WakeReport, Option<acp::TurnUsage>)> {
    // Run for as long as the agent is alive, not for a fixed wall-clock
    // budget. A halt here is a deterministic observation — silence, or the
    // ceiling reached mid-turn — never a guess about whether the model is
    // "stuck or just thinking", which is judgement and not the daemon's.
    let halt = session.prompt_live(context, |usage| {
        usage.cost_usd.is_some_and(|cost| cost >= remaining_budget_usd)
    })
    .await?;
    if let Some(halt) = halt {
        let transcript = session.take_transcript();
        let reason = match halt {
            acp::TurnHalt::Wedged => "the agent stopped producing output entirely; \
                 its work so far is on disk and the next wake continues it"
                .to_string(),
            acp::TurnHalt::OverBudget => format!(
                "this turn reached the remaining ${remaining_budget_usd:.2} budget; \
                 work so far is on disk and the owner must raise the ceiling to continue"
            ),
        };
        return Ok((
            WakeReport {
                company: company.to_string(),
                termination: match halt {
                    // Wedged is recoverable by rehydrating; over-budget needs
                    // the owner, so it is a real blockage.
                    acp::TurnHalt::Wedged => Termination::Continue,
                    acp::TurnHalt::OverBudget => Termination::Blocked,
                },
                reason,
                next_wake_minutes: matches!(halt, acp::TurnHalt::Wedged).then_some(1),
                tool_calls: transcript.tool_calls,
                said: transcript.text.chars().take(1_000).collect(),
                spawn_requests: Vec::new(),
            },
            transcript.usage,
        ));
    }
    let work_transcript = session.take_transcript();
    // The work turn's usage is what the fuse and the health gate read. The
    // termination ask that follows is a second, tiny turn on the same session
    // — it is not what we are measuring.
    let usage = work_transcript.usage;

    let decision = termination_decision(session).await?;
    let said: String = work_transcript.text.chars().take(1_000).collect();
    Ok((
        WakeReport {
            company: company.to_string(),
            termination: decision.termination,
            reason: decision.reason,
            next_wake_minutes: decision.next_wake_minutes,
            tool_calls: work_transcript.tool_calls,
            said,
            spawn_requests: decision.spawn,
        },
        usage,
    ))
}

/// The end-of-turn ask, shared by the Exec and staff: the decision itself is
/// the model's judgement; the envelope is the daemon's deterministic read of
/// it. The Exec's staff-spawn capability is documented in its wake briefing
/// (context.rs), not here — staff have no use for it.
pub(crate) const TERMINATION_PROMPT: &str = "The turn is ending now. Based on everything above, decide how the \
    work stands and answer with JSON only, no prose:\n\
    {\"decision\": \"continue\" | \"blocked\" | \"done\" | \"abandon\", \
     \"reason\": \"<one line>\", \
     \"next_wake_minutes\": <integer, only when continue>}\n\
    - continue: more machine-doable work remains\n\
    - blocked: you cannot proceed — say exactly what you need and from whom\n\
    - done: the stated outcome is met\n\
    - abandon: the work is not worth continuing — say why";

/// Ask the Exec to end the turn explicitly. One retry on an unparseable
/// envelope; a second failure is blocked-on-owner (surface, never spin). A
/// timeout is neither: the work already happened and is on disk, so record
/// Continue with no schedule and let the periodic tick re-wake — the next
/// wake is a fresh session, and a machinery stall must not consume owner
/// attention as a fake blockage.
async fn termination_decision(session: &AgentSession) -> Result<TerminationDecision> {
    for attempt in 0..2 {
        let prompted =
            tokio::time::timeout(TERMINATION_TIMEOUT, session.prompt(TERMINATION_PROMPT)).await;
        let Ok(prompted) = prompted else {
            let _ = session.cancel().await;
            tracing::warn!(
                timeout_s = TERMINATION_TIMEOUT.as_secs(),
                "termination decision timed out; continuing on the tick"
            );
            return Ok(TerminationDecision {
                termination: Termination::Continue,
                reason: format!(
                    "termination decision timed out after {}s",
                    TERMINATION_TIMEOUT.as_secs()
                ),
                next_wake_minutes: None,
                spawn: vec![],
            });
        };
        prompted?;
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
                return Ok(TerminationDecision {
                    termination: Termination::Blocked,
                    reason: "exec produced no parseable termination decision twice".to_string(),
                    next_wake_minutes: None,
                    spawn: vec![],
                });
            }
        }
    }
    unreachable!()
}

/// Parse the termination envelope. The decision itself was the model's; this
/// is deterministic envelope handling — find the JSON object, no prose
/// interpretation (LLM_CURE.md frame 2). Malformed spawn entries are dropped
/// with a warning, never allowed to sink the decision they rode in with.
pub(crate) fn parse_termination(text: &str) -> Option<TerminationDecision> {
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
    let entries = match output.spawn {
        None => Vec::new(),
        Some(serde_json::Value::Array(entries)) => entries,
        Some(other) => {
            tracing::warn!(spawn = %other, "spawn field is not a list; ignoring it");
            Vec::new()
        }
    };
    let spawn = entries
        .into_iter()
        .filter_map(|entry| match serde_json::from_value::<SpawnRequest>(entry) {
            Ok(request) => Some(request),
            Err(error) => {
                tracing::warn!(%error, "dropping malformed spawn request");
                None
            }
        })
        .collect();
    Some(TerminationDecision {
        termination,
        reason: output.reason,
        next_wake_minutes: output.next_wake_minutes,
        spawn,
    })
}

/// Gather the wake's read-only snapshot (the only IO in context assembly;
/// `context::assemble` is pure). Files win over memory — this is all the
/// continuity the Exec gets, so it is all here.
async fn gather_snapshot(
    container: &str,
    org: &OrgIntel,
    config: &CompanyConfig,
    reason: &str,
    spent_usd: f64,
) -> Result<ContextSnapshot> {
    let current_plan = read_company_file(container, "/company/org/exec/current-plan.md").await?;
    let latest_journal = latest_journal_entry(container).await?;
    let commitments = org.list_commitments().await?;
    let open: Vec<_> = commitments
        .into_iter()
        .filter(|c| {
            matches!(
                c.state,
                CommitmentState::Proposed | CommitmentState::Active | CommitmentState::Blocked
            )
        })
        .collect();
    let inbox = org.inbox(Some("exec")).await?;
    let root = runtime::state_root();
    Ok(ContextSnapshot {
        company: config.name.clone(),
        constitution: load_operating_rules(&root),
        mission: config.mission.clone(),
        current_plan,
        latest_journal,
        open_commitments: open,
        inbox,
        wake_reason: reason.to_string(),
        budget_remaining_usd: (config.spend_ceiling_usd - spent_usd).max(0.0),
        budget_ceiling_usd: config.spend_ceiling_usd,
        capabilities: crate::effect::available_capabilities(&root, &config.name),
        effect_ledger: crate::reconcile::effect_ledger(org).await?.summary(),
        org_signals: health::organisational(org, spent_usd)
            .await?
            .into_iter()
            .map(|signal| format!("[{}] {}", signal.kind, signal.detail))
            .collect(),
    })
}

/// The installation's standing rules for agents, from
/// `docs/COMPANY_OPERATING_RULES.md`. NOT the product constitution — that is a
/// document about what Restless is, read by its builders, never injected into
/// a prompt. Absent
/// is not fatal — a company with no constitution still runs, it just runs
/// without the rules that stop it lying about verification.
fn load_operating_rules(root: &std::path::Path) -> String {
    let path = root.join("operating-rules.md");
    match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => {
            tracing::warn!(
                path = %path.display(),
                "no operating rules installed; agents run without standing rules"
            );
            String::new()
        }
    }
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
            // Latch the milestone Blocked: the tick skips blocked milestones
            // ("blocked waits on the owner, not on time"), so without this the
            // company re-wakes every idle window and re-mails the owner the
            // identical block — observed live: 20 duplicate mails in 3h (F1).
            org.set_commitment_state(milestone, CommitmentState::Blocked, &report.reason)
                .await?;
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
        Termination::Continue => {
            // Unlatch: a blocked milestone the Exec has resumed (e.g. after
            // the owner's reply woke it) rejoins the tick's drive set.
            org.set_commitment_state(milestone, CommitmentState::Active, "").await?;
            // The Exec's own time-driven trigger (T6): durable in OrgIntel,
            // so the schedule survives a restlessd restart. A continue with
            // no minutes leaves nothing; the periodic tick is the net.
            if let Some(minutes) = report.next_wake_minutes {
                let fire_at = chrono::Utc::now() + chrono::Duration::minutes(i64::from(minutes));
                org.emit_event(
                    "wake_scheduled",
                    Some("exec"),
                    serde_json::json!({ "fire_at": fire_at.to_rfc3339(), "minutes": minutes }),
                )
                .await?;
            }
        }
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

async fn read_company_file(container: &str, path: &str) -> Result<String> {
    let output = exec_output(container, &format!("cat {path} 2>/dev/null || true")).await?;
    Ok(output)
}

/// The most recent journal entry's filename and content, for rehydration.
async fn latest_journal_entry(container: &str) -> Result<Option<String>> {
    let output = exec_output(
        container,
        "cd /company/org/exec/journal 2>/dev/null && ls | sort | tail -1 | xargs -r sh -c 'echo \"== $0 ==\"; cat \"$0\"' || true",
    )
    .await?;
    Ok(if output.trim().is_empty() { None } else { Some(output) })
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The spawn field rides along with the decision; a malformed sibling
    /// entry is dropped, the valid one survives.
    #[test]
    fn spawn_requests_survive_malformed_siblings() {
        let text = r#"{"decision":"continue","reason":"need hands","next_wake_minutes":10,
            "spawn":[{"name":"builder","task":"write the parser","repo":"game"},"oops"]}"#;
        let decision = parse_termination(text).expect("decision parses");
        assert!(matches!(decision.termination, Termination::Continue));
        assert_eq!(decision.spawn.len(), 1);
        assert_eq!(decision.spawn[0].name, "builder");
        assert_eq!(decision.spawn[0].repo.as_deref(), Some("game"));
    }

    /// A spawn field of the wrong SHAPE must not sink the decision it came
    /// with — the exec's "done" still lands even if it fumbled the syntax.
    #[test]
    fn malformed_spawn_never_kills_the_decision() {
        let text = r#"{"decision":"done","reason":"shipped","spawn":{"not":"a list"}}"#;
        let decision = parse_termination(text).expect("decision parses despite bad spawn");
        assert!(matches!(decision.termination, Termination::Done));
        assert!(decision.spawn.is_empty());
    }
}
