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
use restlessd::runtime_transport::{
    CompanyPath, RuntimeFileKind, RuntimeProcessAuthority, RuntimeTransport, RuntimeTransportError,
};
use serde::Serialize;
use std::collections::HashSet;
use tokio_util::sync::CancellationToken;

use crate::acp::{self, AgentSession};
use crate::context::{self, ContextSnapshot};
use crate::health;
use crate::runtime::CompanyConfig;
use crate::spend::SpendLedger;

/// Bound on the end-of-turn ask: the answer is one line of JSON, so a
/// timeout means the agent wedged (e.g. launched a hanging tool) rather
/// than that the decision needs more thought. Without a bound, a wedged
/// termination turn holds the wake guard forever and silently stops the
/// company's scheduling — the same failure family as the work-turn timeout
/// above exists to bound.
const TERMINATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2 * 60);
const CONTINUE_WAKE_DELAY_SECONDS: u32 = 60;
const EXEC_CONTINUITY_FILE_LIMIT: usize = 256 * 1024;

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
    /// Deterministic next-wake delay. Model prose cannot set this.
    pub retry_after_seconds: Option<u32>,
    /// Tool calls the Exec made this turn (observability).
    pub tool_calls: Vec<String>,
    /// The Exec's closing text this turn, truncated.
    pub said: String,
    /// The final assistant block from the work turn. Conversation scheduling
    /// uses this only as a durable-reply fallback when the Exec spoke to the
    /// owner but omitted the `restless message` tool call.
    #[serde(skip)]
    pub(crate) owner_reply: Option<String>,
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
    pub retry_after_seconds: Option<u32>,
}

/// One Exec wake: rehydrate → work turn → termination decision → record.
#[expect(
    clippy::too_many_arguments,
    reason = "the Exec wake boundary keeps company, authority, organisational state and cancellation explicit"
)]
pub async fn wake(
    config: &CompanyConfig,
    runtime_transport: &std::sync::Arc<dyn RuntimeTransport>,
    spend: &SpendLedger,
    authority: &crate::authority::AuthorityStore,
    capabilities: &crate::capability::CapabilityIssuer,
    org: &OrgIntel,
    reason: &str,
    observer: Option<acp::SessionObserver>,
    cancellation: &CancellationToken,
) -> Result<WakeReport> {
    // Exec conversation is free-form. Machine work is created and claimed
    // through OrgIntel's Work graph, never inferred from this wake.
    org.ensure_actor_with_model("exec", "exec", "exec", "The Exec", Some(&config.model))
        .await?;
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await?;
    let exec_model = org
        .active_actor("exec")
        .await?
        .and_then(|actor| actor.model);

    // Preflight: a company whose computer is stopped or whose disk is full
    // must not be woken. Nothing below this line is free — context assembly
    // reads the volume and the turn spends money — so the cheap deterministic
    // checks come first (F2, F3, F12).
    if let Some(blocked) = health::preflight(runtime_transport.as_ref(), &config.name).await? {
        return blocked_wake(org, config, &blocked.message()).await;
    }
    let initial_budget = spend.budget_state(config);

    // T7: gather the snapshot (IO), then assemble (pure, digested). The
    // digest lands in the wake event so the Exec's worldview is auditable.
    let spent_usd = spend.spent_usd(&config.name);
    let snapshot = gather_snapshot(
        runtime_transport.as_ref(),
        org,
        authority,
        config,
        reason,
        spent_usd,
        initial_budget
            .remaining_micro_usd()
            .map(|remaining| remaining as f64 / 1_000_000.0),
    )
    .await?;
    let package = context::assemble(&snapshot);
    let candidates =
        crate::model_gateway::available_candidates(config, exec_model.as_deref(), authority)
            .await?;
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
        // `actors.model` is the next-wake preference set by company config or
        // an explicit organisational change. The exact provider attempted is
        // recorded below; failover must not silently make a fallback model
        // the Exec's durable preference.
        let model_attempt_event_id = org
            .emit_event(
                "model_attempt",
                Some("exec"),
                serde_json::json!({ "model": model, "configured_effort": config.reasoning_effort, "attempt": index + 1 }),
            )
            .await?;

        let auth = match agent_auth_for_model(
            model,
            &config.reasoning_effort,
            capabilities,
            &config.name,
            "exec",
            "exec:portfolio",
            None,
            None,
        )
        .await
        {
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
            || package.user_prompt.clone(),
            |failure| {
                format!(
                    "PROVIDER CONTINUITY NOTE\nA previously configured model failed: {failure}\n\
                     Rehydrate from the durable company files and system context. Before \
                     repeating any material external effect, reconcile its existing Authority receipt \
                     and idempotency key.\n\n{}",
                    package.user_prompt
                )
            },
        );
        // Acquire after provider authentication but before opening the ACP
        // session. A waiting charged turn consumes no model session; after
        // the prior holder records usage, it sees the current envelope.
        // Subscription sessions do not take this lane because their charged
        // cost is authoritatively zero.
        let metered_turn = spend
            .acquire_metered_turn(&config.name, auth.billing, config.spend_ceiling_usd)
            .await;
        let budget = spend.budget_state(config);
        let reserved_budget_available = metered_turn
            .as_ref()
            .is_none_or(|turn| turn.allowance_micro_usd() > 0);
        if auth.billing == crate::model_gateway::ModelBilling::MeteredApi
            && (!budget.is_available() || !reserved_budget_available)
        {
            drop(metered_turn);
            let reason = format!("[budget] {}", budget.owner_message(&config.name));
            if let Some(next) = candidates.get(index + 1) {
                let transition = failover_report(model, next, health::BlockKind::Budget, &reason);
                record_failover(org, &transition).await?;
                continuity_note = Some(transition.reason.clone());
                failovers.push(transition);
                continue;
            }
            let report = blocked_report(config, model, &reason, failovers);
            record_outcome(org, &report).await?;
            return Ok(report);
        }
        let remaining = metered_turn
            .as_ref()
            .map(crate::spend::MeteredTurnPermit::allowance_usd)
            .or_else(|| {
                budget
                    .remaining_micro_usd()
                    .map(|remaining| remaining as f64 / 1_000_000.0)
            })
            // The value is only observed by the per-session cost fuse for a
            // metered candidate, which passed the check above. Subscription
            // sessions deliberately do not use the fuse.
            .unwrap_or_default();
        let metered = auth.billing == crate::model_gateway::ModelBilling::MeteredApi;
        let mcp_servers = crate::connected_tool::session_servers(
            authority.pool(),
            &config.name,
            "exec",
            None,
            None,
        )
        .await?;
        let controls = acp::AgentControls::company_actor(package.system_prompt.clone())?
            .with_mcp_servers(mcp_servers);
        let process_authority = RuntimeProcessAuthority::AuthorityEvent {
            company: config.name.clone(),
            actor: "exec".into(),
            responsibility: "portfolio".into(),
            event_id: model_attempt_event_id,
            session_id: auth.session_id.clone(),
        };
        let outcome = acp::with_agent(
            std::sync::Arc::clone(runtime_transport),
            process_authority.clone(),
            &auth,
            "/company",
            "exec",
            "portfolio",
            controls,
            observer.clone(),
            {
                let company = config.name.clone();
                let model = model.clone();
                let cancellation = cancellation.clone();
                let session_org = org.clone();
                move |session| {
                    Box::pin(async move {
                        session_org
                            .emit_event(
                                "model_session_ready",
                                Some("exec"),
                                session.readiness_observation(),
                            )
                            .await?;
                        run_turn(
                            session,
                            &turn_context,
                            &company,
                            &model,
                            remaining,
                            metered,
                            &cancellation,
                        )
                        .await
                    })
                }
            },
        )
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

        let blocked_kind = (report.termination == Termination::Blocked)
            .then(|| health::block_kind_from_message(&report.reason))
            .flatten();
        let failover_kind = blocked_kind.filter(|kind| health::is_provider_failover_kind(*kind));
        if let Some(usage) = usage {
            record_usage(org, &auth, usage, failover_kind).await?;
        }
        // Keep the lane through final durable accounting so a waiting turn
        // recalculates from this outcome. Cooldown and failover bookkeeping
        // are not part of model-session admission.
        drop(metered_turn);
        if blocked_kind == Some(health::BlockKind::Context) {
            // Exec uses the same disposable hot-session contract as Staff.
            // Retrying this locator deterministically resends the oversized
            // provider history, so retain all durable company state and drop
            // only the exact portfolio-session locator. The next wake then
            // reconstructs from the bounded company snapshot.
            acp::discard_session_locator_from_runtime(
                runtime_transport.as_ref(),
                &process_authority,
                &config.name,
                "exec",
                "portfolio",
            )
            .await?;
            org.emit_event(
                "model_context_reconstruction_scheduled",
                Some("exec"),
                serde_json::json!({
                    "model": model,
                    "responsibility": "portfolio",
                    "reason": report.reason.chars().take(300).collect::<String>(),
                }),
            )
            .await?;
            prior_tool_calls.append(&mut report.tool_calls);
            report.tool_calls = prior_tool_calls;
            report.failovers = failovers;
            record_outcome(org, &report).await?;
            return Ok(report);
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

            let budget = spend.budget_state(config);
            if !budget.is_available() {
                let reason = format!("[budget] {}", budget.owner_message(&config.name));
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
        owner_reply: None,
    };
    record_outcome(org, &report).await?;
    Ok(report)
}

#[expect(
    clippy::too_many_arguments,
    reason = "the host-to-agent launch membrane keeps exact model and productive settlement identity explicit"
)]
pub(crate) async fn agent_auth_for_model(
    model: &str,
    effort: &str,
    capabilities: &crate::capability::CapabilityIssuer,
    company: &str,
    actor: &str,
    responsibility: &str,
    work_id: Option<uuid::Uuid>,
    attempt_id: Option<uuid::Uuid>,
) -> Result<acp::AgentAuth> {
    let session_id = uuid::Uuid::new_v4().simple().to_string();
    let access = crate::model_gateway::client()?.auth_for(
        model,
        capabilities,
        company,
        actor,
        &session_id,
        responsibility,
        work_id,
        attempt_id,
    )?;
    Ok(acp::AgentAuth {
        model: model.to_string(),
        effort: effort.to_string(),
        company: company.to_string(),
        session_id: session_id.clone(),
        coordination_token_env: "RESTLESS_SESSION_CAPABILITY".to_string(),
        coordination_token: capabilities.issue_actor_session(company, actor, &session_id)?,
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
        owner_reply: None,
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
    org: &OrgIntel,
    auth: &acp::AgentAuth,
    usage: acp::TurnUsage,
    failure_kind: Option<health::BlockKind>,
) -> Result<()> {
    let reported_turn_cost_usd = match auth.billing {
        crate::model_gateway::ModelBilling::MeteredApi => usage.cost_usd,
        crate::model_gateway::ModelBilling::Subscription => Some(0.0),
    };
    // ACP reports remain useful session telemetry, but the relay owns
    // canonical charged-use records. Never turn this presentation float into a
    // second ledger write.
    org.emit_event(
        "turn_usage",
        Some("exec"),
        serde_json::json!({
            "model": auth.model,
            "configured_effort": auth.effort,
            "billing": auth.billing.as_str(),
            "tokens": usage.used,
            "context_size": usage.size,
            "context_used_pct": percent(usage.used, usage.size),
            "reported_turn_cost_usd": reported_turn_cost_usd,
            "charged_cost_source": "host_model_relay",
            // Keep this compatibility field explicitly labelled by its
            // semantics for existing projections.
            "cost_usd": reported_turn_cost_usd,
            "cost_semantics": "acp_cumulative_minus_persisted_session_baseline_noncanonical",
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

/// The full turn inside one ACP session: work prompt, then the termination
/// decision as a second prompt on the same session (it has full context).
async fn run_turn(
    session: &AgentSession,
    context: &str,
    company: &str,
    model: &str,
    remaining_budget_usd: f64,
    enforce_spend_budget: bool,
    cancellation: &CancellationToken,
) -> Result<(WakeReport, Option<acp::TurnUsage>)> {
    // Run for as long as the agent is alive, not for a fixed wall-clock
    // budget. How the turn ends is a deterministic observation — the agent
    // finished, went silent, hit the ceiling, or the transport broke — never a
    // guess about whether the model is "stuck or just thinking", which is
    // judgement and not the daemon's.
    let end = session
        .prompt_live(
            context,
            move |usage| {
                enforce_spend_budget
                    && usage
                        .cost_usd
                        .is_some_and(|cost| cost >= remaining_budget_usd)
            },
            cancellation,
        )
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
        owner_reply: (!transcript.last_message_text.trim().is_empty())
            .then(|| transcript.last_message_text.trim().to_string()),
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
            // Some OpenAI-compatible transports deliver an upstream refusal
            // as the completed assistant message. Classify that explicit
            // error envelope before asking the same unpaid session for a
            // termination decision; otherwise Restless spends a second call
            // and records the provider failure as model indecision.
            if let Some(blocked) = health::classify_provider_error_content(&transcript.text) {
                return Ok((report(Termination::Blocked, blocked.message(), None), usage));
            }
            // The decision envelope is internal coordination, not the
            // owner-facing reply that the live activity dock previews.
            session.set_live_observer_enabled(false);
            let decision = termination_decision(session, cancellation).await;
            Ok((
                report(
                    decision.termination,
                    decision.reason,
                    decision.retry_after_seconds,
                ),
                usage,
            ))
        }
    }
}

/// The Exec end-of-turn ask: the decision itself is the model's judgement;
/// the envelope is the daemon's deterministic read of it. Work kickoff is
/// absent because graph facts own it. `waiting` is deliberately distinct from
/// `continue`: a durable Staff completion/message will wake Exec, so polling
/// would only spend money to rediscover that the job is still running.
pub(crate) const TERMINATION_PROMPT: &str =
    "The turn is ending now. Based on everything above, decide how the \
    work stands and answer with JSON only, no prose:\n\
    {\"decision\": \"continue\" | \"waiting\" | \"blocked\" | \"outcome_met\" | \"abandon\", \
     \"reason\": \"<one line>\"}\n\
    - continue: more machine-doable executive work remains now; schedule a near-term continuation\n\
    - waiting: delegated Work or an observable external process is already in flight and its durable completion/failure event will wake you; do not poll it\n\
    - blocked: the company cannot advance the active outcome until a human or external event acts; \
      use this even when this wake's narrower instruction is finished, and say exactly what is needed\n\
    - outcome_met: the active company milestone itself is fully achieved, with no remaining owner or \
      external gate required by its outcome contract; this closes the milestone, so never use it merely \
      because the current wake's checklist is finished\n\
    - abandon: the work is not worth continuing — say why";

/// Ask the Exec to end the turn explicitly. This small postflight must never
/// erase a completed, metered work turn: transport loss, timeout, or malformed
/// JSON records Continue plus a bounded substrate retry. Only a parsed model
/// decision or a classified provider refusal may produce another state.
async fn termination_decision(
    session: &AgentSession,
    cancellation: &CancellationToken,
) -> TerminationDecision {
    for attempt in 0..2 {
        let prompted = tokio::select! {
            () = cancellation.cancelled() => {
                let _ = session.cancel().await;
                return retry_termination("the owner interrupted the turn to send new direction");
            }
            prompted = tokio::time::timeout(TERMINATION_TIMEOUT, session.prompt(TERMINATION_PROMPT)) => prompted,
        };
        let Ok(prompted) = prompted else {
            let _ = session.cancel().await;
            tracing::warn!(
                timeout_s = TERMINATION_TIMEOUT.as_secs(),
                "termination decision timed out; continuing on the tick"
            );
            return retry_termination(format!(
                "termination decision timed out after {}s",
                TERMINATION_TIMEOUT.as_secs()
            ));
        };
        if let Err(error) = prompted {
            tracing::warn!(%error, "termination decision transport failed; preserving work turn");
            return retry_termination(format!(
                "termination decision transport failed after the work turn: {error:#}"
            ));
        }
        let transcript = session.take_transcript();
        match parse_termination(&transcript.text) {
            Some(parsed) => return parsed,
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
                if let Some(blocked) = health::classify_provider_error_content(&transcript.text) {
                    return TerminationDecision {
                        termination: Termination::Blocked,
                        reason: blocked.message(),
                        retry_after_seconds: None,
                    };
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
                return retry_termination(
                    "exec produced no parseable termination decision twice; preserving the completed work turn",
                );
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
    let waiting = output.decision == "waiting";
    let termination = match output.decision.as_str() {
        "continue" => Termination::Continue,
        "waiting" => Termination::Continue,
        "blocked" | "blocked_on_owner" => Termination::Blocked,
        "changes_requested" => Termination::ChangesRequested,
        "outcome_met" => Termination::OutcomeMet,
        "abandon" => Termination::Abandon,
        _ => return None,
    };
    Some(TerminationDecision {
        termination,
        reason: output.reason,
        retry_after_seconds: (termination == Termination::Continue && !waiting)
            .then_some(CONTINUE_WAKE_DELAY_SECONDS),
    })
}

fn retry_termination(reason: impl Into<String>) -> TerminationDecision {
    TerminationDecision {
        termination: Termination::Continue,
        reason: reason.into(),
        retry_after_seconds: Some(CONTINUE_WAKE_DELAY_SECONDS),
    }
}

/// Gather the wake's read-only snapshot (the only IO in context assembly;
/// `context::assemble` is pure). Files win over memory — this is all the
/// continuity the Exec gets, so it is all here.
async fn gather_snapshot(
    runtime_transport: &dyn RuntimeTransport,
    org: &OrgIntel,
    authority: &crate::authority::AuthorityStore,
    config: &CompanyConfig,
    reason: &str,
    spent_usd: f64,
    remaining_usd: Option<f64>,
) -> Result<ContextSnapshot> {
    let current_plan = read_company_file(
        runtime_transport,
        &config.name,
        "/company/org/exec/current-plan.md",
    )
    .await?;
    let latest_journal = latest_journal_entry(runtime_transport, &config.name).await?;
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
    let unread_owner_message_ids = inbox
        .iter()
        .filter(|message| message.from_actor == "owner")
        .map(|message| message.id)
        .collect::<HashSet<_>>();
    let focus = org.owner_conversation_focus("exec").await?;
    let recent_owner_conversation = org
        .owner_conversation_since("exec", focus.after_message_id, 12)
        .await?
        .into_iter()
        .filter(|message| !unread_owner_message_ids.contains(&message.id))
        .collect();
    let owed_judgements = org.handoffs_assigned_to("exec").await?;
    Ok(ContextSnapshot {
        company: config.name.clone(),
        operating_rules: crate::context::COMPANY_OPERATING_RULES.to_string(),
        mission: config.mission.clone(),
        outcome_standard: config.outcome_standard,
        legal_identity: crate::legal::safe_projection(authority, &config.name).await?,
        current_plan,
        latest_journal,
        open_work: open,
        recent_owner_conversation,
        inbox,
        owed_judgements,
        wake_reason: reason.to_string(),
        budget_remaining_usd: remaining_usd,
        budget_ceiling_usd: config.spend_ceiling_usd.as_usd(),
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
        let schedule_reason = if report.reason.starts_with("termination decision")
            || report.reason.starts_with("[transport]")
        {
            "recover interrupted Exec substrate".to_string()
        } else {
            format!(
                "continue active Exec milestone: {}",
                report.reason.chars().take(240).collect::<String>()
            )
        };
        org.add_schedule("exec", None, &schedule_reason, fire_at)
            .await?;
    }
    Ok(())
}

/// Best-effort terminal record for a wake that escaped the normal closed turn
/// path. The caller first proves that the latest wake has no later wake_end,
/// so this cannot manufacture duplicate terminal events.
pub(crate) async fn record_interrupted_outcome(
    org: &OrgIntel,
    config: &CompanyConfig,
    detail: &str,
) -> Result<()> {
    let report = WakeReport {
        company: config.name.clone(),
        model: config.model.clone(),
        failovers: Vec::new(),
        termination: Termination::Continue,
        reason: format!(
            "[transport] Exec wake ended outside the closed turn path: {}",
            detail.chars().take(500).collect::<String>()
        ),
        retry_after_seconds: Some(CONTINUE_WAKE_DELAY_SECONDS),
        tool_calls: Vec::new(),
        said: String::new(),
        owner_reply: None,
    };
    record_outcome(org, &report).await
}

async fn read_company_file(
    runtime_transport: &dyn RuntimeTransport,
    company: &str,
    path: &str,
) -> Result<String> {
    let path = CompanyPath::parse(path.to_owned()).map_err(|error| anyhow::anyhow!(error))?;
    let contents = match runtime_transport
        .read(company, &path, EXEC_CONTINUITY_FILE_LIMIT + 1)
        .await
    {
        Ok(contents) => contents,
        Err(RuntimeTransportError::NotFound) => return Ok(String::new()),
        Err(error) => {
            return Err(anyhow::anyhow!(error))
                .with_context(|| format!("read Exec continuity file {}", path.as_str()));
        }
    };
    if contents.len() > EXEC_CONTINUITY_FILE_LIMIT {
        anyhow::bail!(
            "Exec continuity file {} exceeds the {}-byte context bound",
            path.as_str(),
            EXEC_CONTINUITY_FILE_LIMIT
        );
    }
    String::from_utf8(contents)
        .with_context(|| format!("Exec continuity file {} is not UTF-8", path.as_str()))
}

/// The most recent journal entry's filename and content, for rehydration.
async fn latest_journal_entry(
    runtime_transport: &dyn RuntimeTransport,
    company: &str,
) -> Result<Option<String>> {
    const JOURNAL_ROOT: &str = "/company/org/exec/journal";
    let journal_root = CompanyPath::parse(JOURNAL_ROOT).map_err(|error| anyhow::anyhow!(error))?;
    let mut entries = match runtime_transport.list(company, &journal_root).await {
        Ok(entries) => entries,
        Err(RuntimeTransportError::NotFound) => return Ok(None),
        Err(error) => {
            return Err(anyhow::anyhow!(error)).context("list Exec continuity journal");
        }
    };
    entries.retain(|entry| entry.metadata.kind == RuntimeFileKind::File);
    entries.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
    let Some(latest) = entries.pop() else {
        return Ok(None);
    };
    let path = CompanyPath::parse(format!("{JOURNAL_ROOT}/{}", latest.name))
        .map_err(|error| anyhow::anyhow!(error))
        .context("validate latest Exec journal path")?;
    let contents = match runtime_transport
        .read(company, &path, EXEC_CONTINUITY_FILE_LIMIT + 1)
        .await
    {
        Ok(contents) => contents,
        Err(RuntimeTransportError::NotFound) => return Ok(None),
        Err(error) => {
            return Err(anyhow::anyhow!(error)).context("read latest Exec continuity journal");
        }
    };
    if contents.len() > EXEC_CONTINUITY_FILE_LIMIT {
        anyhow::bail!(
            "Exec journal file {} exceeds the {}-byte context bound",
            path.as_str(),
            EXEC_CONTINUITY_FILE_LIMIT
        );
    }
    let contents = String::from_utf8(contents)
        .with_context(|| format!("Exec journal file {} is not UTF-8", path.as_str()))?;
    Ok(Some(format!("== {} ==\n{contents}", latest.name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use restlessd::runtime_transport::{
        RuntimeActivity, RuntimeComponentCheck, RuntimeComponentStatus, RuntimeDirectoryEntry,
        RuntimeDuplex, RuntimeFileMetadata, RuntimeProcess, RuntimeProcessControl,
        RuntimeProcessExit, RuntimeProcessSpec, RuntimeReadiness, RuntimeService, RuntimeSignal,
    };
    use std::sync::Mutex;

    struct CompletedProbe;

    #[async_trait]
    impl RuntimeProcessControl for CompletedProbe {
        async fn signal(&self, _signal: RuntimeSignal) -> Result<(), RuntimeTransportError> {
            Ok(())
        }

        async fn wait(&self) -> Result<RuntimeProcessExit, RuntimeTransportError> {
            Ok(RuntimeProcessExit {
                code: Some(0),
                signal: None,
                finished_at: Utc::now(),
            })
        }
    }

    struct NetworkExecTransport {
        journal_entries: Vec<String>,
        process_specs: Mutex<Vec<RuntimeProcessSpec>>,
    }

    impl NetworkExecTransport {
        fn healthy() -> Self {
            Self {
                journal_entries: vec!["0001.md".into(), "0010.md".into()],
                process_specs: Mutex::new(Vec::new()),
            }
        }

        fn journal_metadata() -> RuntimeFileMetadata {
            RuntimeFileMetadata {
                kind: RuntimeFileKind::File,
                size: 32,
                modified_at: Utc::now(),
                mode: 0o600,
            }
        }
    }

    #[async_trait]
    impl RuntimeTransport for NetworkExecTransport {
        async fn readiness(
            &self,
            company: &str,
        ) -> Result<RuntimeReadiness, RuntimeTransportError> {
            assert_eq!(company, "hosted_test");
            Ok(RuntimeReadiness {
                runtime_id: "runtime-hosted-test".into(),
                runtime_generation: 7,
                runtime_image: "registry.example/runtime@sha256:test".into(),
                source_revision: "revision".into(),
                volume_name: "volume-hosted-test".into(),
                observed_at: Utc::now(),
                components: vec![
                    RuntimeComponentCheck {
                        name: "runtime_agent".into(),
                        status: RuntimeComponentStatus::Ready,
                    },
                    RuntimeComponentCheck {
                        name: "persistent_volume".into(),
                        status: RuntimeComponentStatus::Ready,
                    },
                    RuntimeComponentCheck {
                        name: "process_execution".into(),
                        status: RuntimeComponentStatus::Ready,
                    },
                ],
            })
        }

        async fn start_process(
            &self,
            specification: RuntimeProcessSpec,
        ) -> Result<RuntimeProcess, RuntimeTransportError> {
            self.process_specs
                .lock()
                .expect("process specifications")
                .push(specification);
            let filesystem = "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/runtime 8388608 1024 8387584 1% /company\n";
            Ok(RuntimeProcess {
                process_id: uuid::Uuid::new_v4(),
                pid: 42,
                stdin: Box::pin(tokio::io::sink()),
                stdout: Box::pin(std::io::Cursor::new(filesystem.as_bytes().to_vec())),
                stderr: Box::pin(tokio::io::empty()),
                control: Box::new(CompletedProbe),
            })
        }

        async fn stat(
            &self,
            _company: &str,
            _path: &CompanyPath,
        ) -> Result<RuntimeFileMetadata, RuntimeTransportError> {
            Err(RuntimeTransportError::Unavailable)
        }

        async fn list(
            &self,
            company: &str,
            path: &CompanyPath,
        ) -> Result<Vec<RuntimeDirectoryEntry>, RuntimeTransportError> {
            assert_eq!(company, "hosted_test");
            assert_eq!(path.as_str(), "/company/org/exec/journal");
            Ok(self
                .journal_entries
                .iter()
                .map(|name| RuntimeDirectoryEntry {
                    name: name.clone(),
                    metadata: Self::journal_metadata(),
                })
                .collect())
        }

        async fn read(
            &self,
            company: &str,
            path: &CompanyPath,
            maximum_bytes: usize,
        ) -> Result<Vec<u8>, RuntimeTransportError> {
            assert_eq!(company, "hosted_test");
            assert_eq!(maximum_bytes, EXEC_CONTINUITY_FILE_LIMIT + 1);
            match path.as_str() {
                "/company/org/exec/current-plan.md" => Ok(b"# Current plan\nShip it.\n".to_vec()),
                "/company/org/exec/journal/0010.md" => Ok(b"Owner input reached Exec.\n".to_vec()),
                _ => Err(RuntimeTransportError::NotFound),
            }
        }

        async fn atomic_write(
            &self,
            _company: &str,
            _operation_id: uuid::Uuid,
            _path: &CompanyPath,
            _contents: &[u8],
            _mode: u32,
        ) -> Result<(), RuntimeTransportError> {
            Err(RuntimeTransportError::Unavailable)
        }

        async fn rename(
            &self,
            _company: &str,
            _operation_id: uuid::Uuid,
            _source: &CompanyPath,
            _destination: &CompanyPath,
        ) -> Result<(), RuntimeTransportError> {
            Err(RuntimeTransportError::Unavailable)
        }

        async fn digest(
            &self,
            _company: &str,
            _path: &CompanyPath,
        ) -> Result<[u8; 32], RuntimeTransportError> {
            Err(RuntimeTransportError::Unavailable)
        }

        async fn open_service(
            &self,
            _company: &str,
            _operation_id: uuid::Uuid,
            _service: RuntimeService,
            _idle_timeout: std::time::Duration,
        ) -> Result<RuntimeDuplex, RuntimeTransportError> {
            Err(RuntimeTransportError::Unavailable)
        }

        async fn activity(&self, _company: &str) -> Result<RuntimeActivity, RuntimeTransportError> {
            Err(RuntimeTransportError::Unavailable)
        }
    }

    #[tokio::test]
    async fn a_network_runtime_carries_exec_preflight_and_continuity_without_host_docker() {
        assert!(restlessd::hosted_runtime::require_local_docker(Some("network")).is_err());
        let transport = NetworkExecTransport::healthy();

        assert!(health::preflight(&transport, "hosted_test")
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            read_company_file(
                &transport,
                "hosted_test",
                "/company/org/exec/current-plan.md"
            )
            .await
            .unwrap(),
            "# Current plan\nShip it.\n"
        );
        assert_eq!(
            latest_journal_entry(&transport, "hosted_test")
                .await
                .unwrap()
                .as_deref(),
            Some("== 0010.md ==\nOwner input reached Exec.\n")
        );

        let specifications = transport
            .process_specs
            .lock()
            .expect("process specifications");
        assert_eq!(specifications.len(), 1);
        let probe = &specifications[0];
        assert_eq!(probe.executable, "df");
        assert_eq!(probe.arguments, ["-Pk", "/company"]);
        assert_eq!(probe.working_directory.as_str(), "/company");
        assert!(matches!(
            &probe.authority,
            RuntimeProcessAuthority::InfrastructureProbe { company, probe }
                if company == "hosted_test" && probe == "exec-preflight-disk"
        ));
    }

    #[tokio::test]
    async fn a_runtime_journal_name_cannot_escape_the_company_path() {
        let transport = NetworkExecTransport {
            journal_entries: vec!["../../outside.md".into()],
            process_specs: Mutex::new(Vec::new()),
        };
        assert!(latest_journal_entry(&transport, "hosted_test")
            .await
            .unwrap_err()
            .to_string()
            .contains("validate latest Exec journal path"));
    }

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
        assert_eq!(decision.retry_after_seconds, None);
        assert!(TERMINATION_PROMPT.contains("current wake's checklist"));
    }

    #[test]
    fn a_continue_decision_schedules_the_next_exec_wake() {
        let decision = parse_termination(
            r#"{"decision":"continue","reason":"deployment still needs verification"}"#,
        )
        .unwrap();
        assert_eq!(decision.termination, Termination::Continue);
        assert_eq!(
            decision.retry_after_seconds,
            Some(CONTINUE_WAKE_DELAY_SECONDS)
        );
    }

    #[test]
    fn waiting_for_durable_delegated_work_does_not_poll() {
        let decision = parse_termination(
            r#"{"decision":"waiting","reason":"the delegated CI Attempt is running"}"#,
        )
        .unwrap();
        assert_eq!(decision.termination, Termination::Continue);
        assert_eq!(decision.retry_after_seconds, None);
        assert!(TERMINATION_PROMPT.contains("do not poll it"));
    }

    #[test]
    fn termination_postflight_failure_preserves_work_and_retries() {
        let decision = retry_termination("ACP connection closed");
        assert_eq!(decision.termination, Termination::Continue);
        assert_eq!(decision.retry_after_seconds, Some(60));
        assert_eq!(decision.reason, "ACP connection closed");
    }
}
