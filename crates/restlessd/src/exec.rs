//! The persistent Exec (sprint 01 T4). Identity is durable — an actor row
//! plus `/company/org/exec/` (current-plan.md, journal/NNNN.md) — and the
//! ACP session is disposable: each wake starts a fresh one and rehydrates
//! from files + OrgIntel. A kill mid-turn loses at most the in-flight turn;
//! the next wake continues the milestone rather than restarting it.
//!
//! Termination is a model decision (judgement over an open-ended turn,
//! enumerable output — LLM_CURE.md frame 2), never a turn-count or timer.

use anyhow::{Context, Result};
use restless_orgintel::OrgIntel;
use serde::Serialize;

use crate::acp::{self, AgentSession};
use crate::context::{self, ContextSnapshot};
use crate::health;
use crate::runtime::{self, CompanyConfig};
use crate::spend::SpendLedger;

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
    ChangesRequested,
    OutcomeMet,
    Abandon,
}

#[derive(Debug, Serialize)]
pub struct WakeReport {
    pub company: String,
    /// The model that produced the final wake outcome.
    pub model: String,
    /// Ordered provider transitions attempted inside this wake.
    pub failovers: Vec<ModelFailoverReport>,
    pub termination: Termination,
    pub reason: String,
    /// Deterministic substrate retry delay. Model prose cannot set this.
    pub retry_after_seconds: Option<u32>,
    /// Tool calls the Exec made this turn (observability).
    pub tool_calls: Vec<String>,
    /// The Exec's closing text this turn, truncated.
    pub said: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelFailoverReport {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub reason: String,
}

#[derive(Debug, serde::Deserialize)]
struct TerminationOutput {
    decision: String,
    reason: String,
}

/// The parsed end-of-turn decision. Delegation is ordinary Work graph data,
/// never a second command hidden in an LLM envelope.
pub(crate) struct TerminationDecision {
    pub termination: Termination,
    pub reason: String,
}

/// One Exec wake: rehydrate → work turn → termination decision → record.
pub async fn wake(
    config: &CompanyConfig,
    spend: &SpendLedger,
    authority: &crate::authority::AuthorityStore,
    org: &OrgIntel,
    reason: &str,
) -> Result<WakeReport> {
    let container = runtime::container_name(&config.name);
    // Exec conversation is free-form. Machine work is created and claimed
    // through OrgIntel's Work graph, never inferred from this wake.
    org.add_actor_with_model("exec", "exec", "The Exec", Some(&config.model))
        .await?;
    org.add_actor("owner", "owner", "The Owner").await?;

    // Preflight: a company whose computer is stopped or whose disk is full
    // must not be woken. Nothing below this line is free — context assembly
    // reads the volume and the turn spends money — so the cheap deterministic
    // checks come first (F2, F3, F12).
    if let Some(blocked) = health::preflight(&config.name).await? {
        return blocked_wake(org, config, &blocked.message()).await;
    }
    if let Some((spent, ceiling)) = spend.over_ceiling(config) {
        let blocked = health::Blocked::budget(format!(
            "{} has spent ${spent:.2} of its ${ceiling:.2} ceiling; \
             the owner must raise it before work continues",
            config.name
        ));
        return blocked_wake(org, config, &blocked.message()).await;
    }

    // T7: gather the snapshot (IO), then assemble (pure, digested). The
    // digest lands in the wake event so the Exec's worldview is auditable.
    let snapshot = gather_snapshot(
        &container,
        org,
        authority,
        config,
        reason,
        spend.spent_usd(&config.name),
    )
    .await?;
    let package = context::assemble(&snapshot);
    let candidates = crate::model_gateway::available_candidates(config, None, authority).await?;
    org.emit_event(
        "wake",
        Some("exec"),
        serde_json::json!({
            "reason": reason,
            "context_digest": package.digest,
            "model_candidates": &candidates,
        }),
    )
    .await?;

    let mut failovers = Vec::new();
    let mut prior_tool_calls = Vec::new();
    let mut continuity_note: Option<String> = None;

    for (index, model) in candidates.iter().enumerate() {
        org.add_actor_with_model("exec", "exec", "The Exec", Some(model))
            .await?;
        org.emit_event(
            "model_attempt",
            Some("exec"),
            serde_json::json!({ "model": model, "attempt": index + 1 }),
        )
        .await?;

        let auth = match agent_auth_for_model(model).await {
            Ok(auth) => auth,
            Err(error) => {
                let text = format!("{error:#}");
                let blocked = health::classify_provider_error(&text)
                    .unwrap_or_else(|| health::Blocked::transport(&text));
                crate::model_gateway::record_cooldown(
                    authority,
                    &config.name,
                    model,
                    blocked.kind,
                    &blocked.message(),
                )
                .await?;
                if let Some(next) = candidates.get(index + 1) {
                    let transition = failover_report(model, next, blocked.kind, &blocked.message());
                    record_failover(org, &transition).await?;
                    continuity_note = Some(transition.reason.clone());
                    failovers.push(transition);
                    continue;
                }
                let report = blocked_report(config, model, &blocked.message(), failovers);
                record_outcome(org, &report).await?;
                return Ok(report);
            }
        };

        let turn_context = continuity_note.as_ref().map_or_else(
            || package.text.clone(),
            |failure| {
                format!(
                    "PROVIDER CONTINUITY NOTE\nA previously configured model failed: {failure}\n\
                     Rehydrate from the durable company files and organisational state below. Before \
                     repeating any material external effect, reconcile its existing Authority receipt \
                     and idempotency key.\n\n{}",
                    package.text
                )
            },
        );
        let remaining = (config.spend_ceiling_usd - spend.spent_usd(&config.name)).max(0.0);
        let metered = auth.billing == crate::model_gateway::ModelBilling::MeteredApi;
        let outcome = acp::with_agent(&container, &auth, "/company", "exec", {
            let company = config.name.clone();
            let model = model.clone();
            move |session| {
                Box::pin(async move {
                    run_turn(session, &turn_context, &company, &model, remaining, metered).await
                })
            }
        })
        .await;

        // The turn itself was already classified inside `run_turn`, once, by
        // the one function entitled to do it. Failures around session opening
        // have no `TurnEnd` and no usage, but may still be provider-specific.
        let (mut report, usage) = match outcome {
            Ok(result) => result,
            Err(error) => {
                let text = format!("{error:#}");
                let blocked = health::classify_provider_error(&text)
                    .unwrap_or_else(|| health::Blocked::transport(&text));
                let report = blocked_report(config, model, &blocked.message(), failovers.clone());
                (report, None)
            }
        };

        let failover_kind = (report.termination == Termination::Blocked)
            .then(|| health::block_kind_from_message(&report.reason))
            .flatten()
            .filter(|kind| health::is_provider_failover_kind(*kind));
        if let Some(usage) = usage {
            record_usage(spend, org, config, &auth, usage, failover_kind).await?;
        }
        if let Some(kind) = failover_kind {
            crate::model_gateway::record_cooldown(
                authority,
                &config.name,
                model,
                kind,
                &report.reason,
            )
            .await?;
        } else if report.termination != Termination::Blocked {
            authority.clear_model_cooldown(&config.name, model).await?;
        }
        if let (Some(kind), Some(next)) = (failover_kind, candidates.get(index + 1)) {
            prior_tool_calls.append(&mut report.tool_calls);
            let transition = failover_report(model, next, kind, &report.reason);
            record_failover(org, &transition).await?;
            continuity_note = Some(transition.reason.clone());
            failovers.push(transition);

            if let Some((spent, ceiling)) = spend.over_ceiling(config) {
                let reason = format!(
                    "[budget] {} has spent ${spent:.2} of its ${ceiling:.2} ceiling; the owner must raise it before provider failover continues",
                    config.name
                );
                let mut budget_report = blocked_report(config, model, &reason, failovers);
                budget_report.tool_calls = prior_tool_calls;
                record_outcome(org, &budget_report).await?;
                return Ok(budget_report);
            }
            continue;
        }

        prior_tool_calls.append(&mut report.tool_calls);
        report.tool_calls = prior_tool_calls;
        report.failovers = failovers;
        record_outcome(org, &report).await?;
        return Ok(report);
    }
    unreachable!("validated company model policy always has a primary candidate")
}

/// Context utilisation, rounded. Sprint 01 burned 95% of its dollars on
/// replayed context without anyone able to see it happening.
fn percent(used: u64, size: u64) -> u64 {
    if size == 0 {
        0
    } else {
        used.saturating_mul(100) / size
    }
}

/// The substrate failed before an actor could run. This is health telemetry,
/// not a synthetic Work transition or an implicit owner handoff.
async fn blocked_wake(org: &OrgIntel, config: &CompanyConfig, reason: &str) -> Result<WakeReport> {
    tracing::warn!(company = %config.name, reason, "wake blocked by health gate");
    let report = WakeReport {
        company: config.name.clone(),
        model: config.model.clone(),
        failovers: Vec::new(),
        termination: Termination::Blocked,
        reason: reason.to_string(),
        retry_after_seconds: None,
        tool_calls: Vec::new(),
        said: String::new(),
    };
    record_outcome(org, &report).await?;
    Ok(report)
}

pub(crate) async fn agent_auth_for_model(model: &str) -> Result<acp::AgentAuth> {
    let access = crate::model_gateway::client()?.auth_for(model)?;
    Ok(acp::AgentAuth {
        model: model.to_string(),
        provider: access.provider,
        gateway_token_env: access.token_env,
        gateway_token: access.token,
        gateway_url: access.runtime_url,
        billing: access.billing,
    })
}

fn blocked_report(
    config: &CompanyConfig,
    model: &str,
    reason: &str,
    failovers: Vec<ModelFailoverReport>,
) -> WakeReport {
    WakeReport {
        company: config.name.clone(),
        model: model.to_string(),
        failovers,
        termination: Termination::Blocked,
        reason: reason.to_string(),
        retry_after_seconds: None,
        tool_calls: Vec::new(),
        said: String::new(),
    }
}

fn failover_report(
    from: &str,
    to: &str,
    kind: health::BlockKind,
    reason: &str,
) -> ModelFailoverReport {
    ModelFailoverReport {
        from: from.to_string(),
        to: to.to_string(),
        kind: kind.as_str().to_string(),
        reason: reason.chars().take(300).collect(),
    }
}

async fn record_failover(org: &OrgIntel, transition: &ModelFailoverReport) -> Result<()> {
    org.emit_event(
        "model_failover",
        Some("exec"),
        serde_json::json!({
            "from": transition.from,
            "to": transition.to,
            "kind": transition.kind,
            "reason": transition.reason,
        }),
    )
    .await?;
    Ok(())
}

async fn record_usage(
    spend: &SpendLedger,
    org: &OrgIntel,
    config: &CompanyConfig,
    auth: &acp::AgentAuth,
    usage: acp::TurnUsage,
    failure_kind: Option<health::BlockKind>,
) -> Result<()> {
    let charged_cost_usd = match auth.billing {
        crate::model_gateway::ModelBilling::MeteredApi => usage.cost_usd,
        crate::model_gateway::ModelBilling::Subscription => Some(0.0),
    };
    // OMP can report the assembled context tokens even when a provider rejects
    // before inference, with no price. A classified auth/quota/model refusal is
    // kept as unpriced telemetry and may fail over. An unpriced, unclassified
    // mid-stream failure after token use remains fail-closed.
    if should_record_spend(auth.billing, usage, failure_kind) {
        spend.record_turn(
            &config.name,
            "exec",
            &auth.model,
            usage.used,
            charged_cost_usd,
        );
    }
    org.emit_event(
        "turn_usage",
        Some("exec"),
        serde_json::json!({
            "model": auth.model,
            "billing": auth.billing.as_str(),
            "tokens": usage.used,
            "context_size": usage.size,
            "context_used_pct": percent(usage.used, usage.size),
            "cost_usd": charged_cost_usd,
            "estimated_list_cost_usd": (auth.billing == crate::model_gateway::ModelBilling::Subscription)
                .then_some(usage.cost_usd)
                .flatten(),
            "unpriced_provider_refusal": (auth.billing == crate::model_gateway::ModelBilling::MeteredApi
                && usage.cost_usd.is_none()
                && failure_kind.is_some_and(|kind| matches!(kind,
                    health::BlockKind::Credential | health::BlockKind::Quota | health::BlockKind::Model | health::BlockKind::NoOp))),
        }),
    )
    .await?;
    Ok(())
}

pub(crate) fn should_record_spend(
    billing: crate::model_gateway::ModelBilling,
    usage: acp::TurnUsage,
    failure_kind: Option<health::BlockKind>,
) -> bool {
    if billing == crate::model_gateway::ModelBilling::Subscription || usage.cost_usd.is_some() {
        return true;
    }
    if failure_kind.is_some_and(|kind| {
        matches!(
            kind,
            health::BlockKind::Credential
                | health::BlockKind::Quota
                | health::BlockKind::Model
                | health::BlockKind::NoOp
        )
    }) {
        return false;
    }
    usage.used > 0
}

/// The full turn inside one ACP session: work prompt, then the termination
/// decision as a second prompt on the same session (it has full context).
async fn run_turn(
    session: &AgentSession,
    context: &str,
    company: &str,
    model: &str,
    remaining_budget_usd: f64,
    enforce_spend_budget: bool,
) -> Result<(WakeReport, Option<acp::TurnUsage>)> {
    // Run for as long as the agent is alive, not for a fixed wall-clock
    // budget. How the turn ends is a deterministic observation — the agent
    // finished, went silent, hit the ceiling, or the transport broke — never a
    // guess about whether the model is "stuck or just thinking", which is
    // judgement and not the daemon's.
    let end = session
        .prompt_live(context, move |usage| {
            enforce_spend_budget
                && usage
                    .cost_usd
                    .is_some_and(|cost| cost >= remaining_budget_usd)
        })
        .await;
    // The work turn's usage is what the fuse and the ledger read, whichever
    // way it ended. The termination ask that follows is a second, tiny turn on
    // the same session — it is not what we are measuring.
    let usage = end.usage();

    // The one and only place a turn is classified. Everything below reads the
    // verdict; nothing below re-derives it from the transcript.
    let verdict = health::classify(&end);
    let transcript = end.into_transcript();
    let report = |termination, reason, retry_after_seconds| WakeReport {
        company: company.to_string(),
        model: model.to_string(),
        failovers: Vec::new(),
        termination,
        reason,
        retry_after_seconds,
        tool_calls: transcript.tool_calls.clone(),
        said: transcript.text.chars().take(1_000).collect(),
    };

    match verdict {
        // Recoverable: the work is on the volume and a fresh session
        // rehydrates from it, so this costs the owner nothing.
        health::Verdict::Resume(reason) => {
            tracing::warn!(company, %reason, "turn stopped early; resuming next wake");
            Ok((report(Termination::Continue, reason, Some(60)), usage))
        }
        // Only the owner can clear this. `record_outcome` latches the
        // milestone and mails once — never a re-wake loop (F1).
        health::Verdict::Blocked(blocked) => {
            tracing::warn!(company, reason = %blocked.message(), "turn blocked the company");
            Ok((report(Termination::Blocked, blocked.message(), None), usage))
        }
        // The turn ran. Only now is the agent's own judgement worth asking
        // for — asking a wedged or unpaid session how the work stands gets
        // prose the parser then fails on, which is how a substrate failure
        // used to arrive dressed as an agent decision.
        health::Verdict::Ran => {
            let decision = termination_decision(session).await?;
            Ok((report(decision.termination, decision.reason, None), usage))
        }
    }
}

/// The end-of-turn ask, shared by the Exec and staff: the decision itself is
/// the model's judgement; the envelope is the daemon's deterministic read of
/// it. Work kickoff is absent from this envelope because graph facts own it.
pub(crate) const TERMINATION_PROMPT: &str =
    "The turn is ending now. Based on everything above, decide how the \
    work stands and answer with JSON only, no prose:\n\
    {\"decision\": \"continue\" | \"blocked\" | \"outcome_met\" | \"abandon\", \
     \"reason\": \"<one line>\"}\n\
    - continue: more machine-doable work remains\n\
    - blocked: the company cannot advance the active outcome until a human or external event acts; \
      use this even when this wake's narrower instruction is finished, and say exactly what is needed\n\
    - outcome_met: the active company milestone itself is fully achieved, with no remaining owner or \
      external gate required by its outcome contract; this closes the milestone, so never use it merely \
      because the current wake's checklist is finished\n\
    - abandon: the work is not worth continuing — say why";

/// Ask the Exec to end the turn explicitly. One retry on an unparseable
/// envelope; a second failure is blocked-on-owner (surface, never spin). A
/// timeout is neither: the work already happened and is on disk, so record
/// Continue with no model-selected timer. Substrate recovery may set its own
/// bounded retry; graph facts and real events drive ordinary continuation.
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
            });
        };
        prompted?;
        let transcript = session.take_transcript();
        match parse_termination(&transcript.text) {
            Some(parsed) => return Ok(parsed),
            None => {
                // Before calling this the model's failure, check whether the
                // model spoke at all. A provider error can arrive as message
                // *content* rather than as a transport error — omp streams the
                // upstream body through — so the turn "succeeds", tokens are
                // consumed, and the health gate sees nothing wrong. Observed
                // live: three companies blocked with "no parseable termination
                // decision" when the actual cause was
                // `429 [1113] Insufficient balance ... Please recharge`.
                //
                // This is F1 wearing a new costume. `classify` reads how a turn
                // ended; this reads what the agent said. Same deterministic
                // status-class parser, and it is named for the text it reads so
                // that the difference cannot be mistaken for the same check.
                if let Some(blocked) = health::classify_provider_error(&transcript.text) {
                    return Ok(TerminationDecision {
                        termination: Termination::Blocked,
                        reason: blocked.message(),
                    });
                }
                if attempt == 0 {
                    // The transcript carries the model's actual words — without
                    // them an unparseable decision is undebuggable (Sprint 01
                    // friction: the first silent failure cost a full probe cycle).
                    tracing::warn!(
                        said = %transcript.text.chars().take(600).collect::<String>(),
                        "termination decision unparseable; retrying once"
                    );
                    continue;
                }
                return Ok(TerminationDecision {
                    termination: Termination::Blocked,
                    reason: "exec produced no parseable termination decision twice".to_string(),
                });
            }
        }
    }
    unreachable!()
}

/// Parse the termination envelope. Delegation is absent on purpose: the
/// actor writes Work graph rows with the CLI while it has full context.
pub(crate) fn parse_termination(text: &str) -> Option<TerminationDecision> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    let output: TerminationOutput = serde_json::from_str(&text[start..=end]).ok()?;
    let termination = match output.decision.as_str() {
        "continue" => Termination::Continue,
        "blocked" | "blocked_on_owner" => Termination::Blocked,
        "changes_requested" => Termination::ChangesRequested,
        "outcome_met" => Termination::OutcomeMet,
        "abandon" => Termination::Abandon,
        _ => return None,
    };
    Some(TerminationDecision {
        termination,
        reason: output.reason,
    })
}

/// Gather the wake's read-only snapshot (the only IO in context assembly;
/// `context::assemble` is pure). Files win over memory — this is all the
/// continuity the Exec gets, so it is all here.
async fn gather_snapshot(
    container: &str,
    org: &OrgIntel,
    authority: &crate::authority::AuthorityStore,
    config: &CompanyConfig,
    reason: &str,
    spent_usd: f64,
) -> Result<ContextSnapshot> {
    let current_plan = read_company_file(container, "/company/org/exec/current-plan.md").await?;
    let latest_journal = latest_journal_entry(container).await?;
    let work = org.list_work().await?;
    let open: Vec<_> = work
        .into_iter()
        .filter(|item| {
            matches!(
                item.status,
                restless_orgintel::WorkStatus::Proposed
                    | restless_orgintel::WorkStatus::Active
                    | restless_orgintel::WorkStatus::Blocked
            )
        })
        .collect();
    let inbox = org.inbox(Some("exec")).await?;
    let owed_judgements = org.handoffs_assigned_to("exec").await?;
    let root = runtime::state_root();
    Ok(ContextSnapshot {
        company: config.name.clone(),
        constitution: load_operating_rules(&root),
        mission: config.mission.clone(),
        current_plan,
        latest_journal,
        open_work: open,
        inbox,
        owed_judgements,
        wake_reason: reason.to_string(),
        budget_remaining_usd: (config.spend_ceiling_usd - spent_usd).max(0.0),
        budget_ceiling_usd: config.spend_ceiling_usd,
        effect_ledger: crate::reconcile::effect_ledger(authority, &config.name)
            .await?
            .summary(),
        org_signals: health::organisational(org, authority, &config.name, spent_usd)
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

/// Record the conversation wake. Work status changes only through an Attempt;
/// this free-form Exec turn cannot secretly complete or block a graph node.
async fn record_outcome(org: &OrgIntel, report: &WakeReport) -> Result<()> {
    org.emit_event(
        "wake_end",
        Some("exec"),
        serde_json::json!({
            "termination": report.termination,
            "reason": report.reason,
            "tool_calls": report.tool_calls.len(),
            "model": report.model,
            "failovers": report.failovers,
        }),
    )
    .await?;
    if let Some(seconds) = report.retry_after_seconds {
        let fire_at = chrono::Utc::now() + chrono::Duration::seconds(i64::from(seconds));
        org.add_schedule("exec", None, "recover interrupted Exec substrate", fire_at)
            .await?;
    }
    Ok(())
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
    Ok(if output.trim().is_empty() {
        None
    } else {
        Some(output)
    })
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

    /// Observed in Aris wake 0018: the model said the wake directive was
    /// "done" while also naming owner-gated commercial work, and the daemon
    /// completed the whole company milestone. The ambiguous old word is no
    /// longer a valid envelope; only the explicitly milestone-scoped decision
    /// can close the company outcome.
    #[test]
    fn a_finished_wake_is_not_a_completed_company_outcome() {
        assert!(parse_termination(r#"{"decision":"done","reason":"this wake is done"}"#).is_none());
        let decision = parse_termination(
            r#"{"decision":"outcome_met","reason":"the active milestone is achieved"}"#,
        )
        .unwrap();
        assert_eq!(decision.termination, Termination::OutcomeMet);
        assert!(TERMINATION_PROMPT.contains("current wake's checklist"));
    }

    #[test]
    fn an_empty_provider_refusal_is_not_unaccounted_spend() {
        let empty = acp::TurnUsage {
            used: 0,
            size: 0,
            cost_usd: None,
        };
        assert!(!should_record_spend(
            crate::model_gateway::ModelBilling::MeteredApi,
            empty,
            Some(health::BlockKind::Quota)
        ));
        assert!(should_record_spend(
            crate::model_gateway::ModelBilling::Subscription,
            empty,
            None
        ));
        assert!(should_record_spend(
            crate::model_gateway::ModelBilling::MeteredApi,
            acp::TurnUsage { used: 1, ..empty },
            Some(health::BlockKind::Transport)
        ));
        assert!(!should_record_spend(
            crate::model_gateway::ModelBilling::MeteredApi,
            acp::TurnUsage {
                used: 15_547,
                ..empty
            },
            Some(health::BlockKind::Quota)
        ));
    }
}
