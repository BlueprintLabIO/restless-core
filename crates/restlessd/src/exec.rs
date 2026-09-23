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
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::acp;
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
const CONTINUE_WAKE_DELAY_SECONDS: u32 = 60;

/// Commit a complete direct-conversation reply while the actor lease is held,
/// before the separate termination judgement starts.
pub(crate) type CompleteReplyHook = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + 'static>> + Send + Sync,
>;

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
    /// Final assistant block: an owner reply (including proactive updates),
    /// or the explicit quiet marker for an uneventful background turn.
    #[serde(skip)]
    pub(crate) owner_reply: Option<String>,
    /// True only when the productive turn itself ended normally with a final
    /// assistant answer. A resumable/interrupted transcript may contain partial
    /// text, but it is never safe to use that text to resolve a Room mention.
    #[serde(skip)]
    pub(crate) reply_complete: bool,
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
    spend: &SpendLedger,
    authority: &crate::authority::AuthorityStore,
    capabilities: &crate::capability::CapabilityIssuer,
    runtime_bridges: &crate::runtime_bridge::RuntimeBridgeRegistry,
    org: &OrgIntel,
    reason: &str,
    owner_actor_id: &str,
    human_is_membership_owner: bool,
    conversation_inbox: &[restless_orgintel::MessageRow],
    owed_judgements: &[restless_orgintel::OwnerHandoffRow],
    pending_mention: Option<&crate::mentions::MentionContext>,
    observer: Option<acp::SessionObserver>,
    complete_reply_hook: Option<CompleteReplyHook>,
    cancellation: &CancellationToken,
) -> Result<WakeReport> {
    let effective_config = config.for_agent("exec");
    let config = &effective_config;
    anyhow::ensure!(
        config.has_effective_model_route(config.coordination_harness),
        "Choose an intelligence provider and model in Company → Intelligence provider before starting the Exec."
    );
    let container = runtime::container_name(&config.name);
    let hosted_identity = if runtime_bridges.is_hosted() {
        Some(crate::runtime_bridge::expected_identity(authority, &config.name).await?)
    } else {
        None
    };
    let focused_mention = pending_mention.is_some();
    // Exec conversation is free-form. Machine work is created and claimed
    // through OrgIntel's Work graph, never inferred from this wake.
    org.ensure_actor_with_model(
        "exec",
        "exec",
        "exec",
        "The Exec",
        config.configured_model(),
    )
    .await?;
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await?;
    let exec_model = org
        .active_actor("exec")
        .await?
        .and_then(|actor| actor.model);
    let conversation_focus = org.human_conversation_focus(owner_actor_id, "exec").await?;
    let responsibility = pending_mention.map_or_else(
        || {
            crate::context::human_conversation_responsibility(
                "portfolio",
                owner_actor_id,
                conversation_focus.after_message_id,
            )
        },
        |mention| {
            let mention_id = match mention {
                crate::mentions::MentionContext::Room(context) => context.mention.id,
                crate::mentions::MentionContext::Document(context) => context.mention.id,
            };
            crate::context::focused_mention_responsibility("portfolio", mention_id)
        },
    );

    // Preflight: a company whose computer is stopped or whose disk is full
    // must not be woken. Nothing below this line is free — context assembly
    // reads the volume and the turn spends money — so the cheap deterministic
    // checks come first (F2, F3, F12).
    if let Some(identity) = &hosted_identity {
        if let Err(error) = crate::runtime_bridge::preflight(runtime_bridges, identity).await {
            return blocked_wake(org, config, &format!("[runtime] {error:#}")).await;
        }
    } else if let Some(blocked) = health::preflight(&config.name).await? {
        return blocked_wake(org, config, &blocked.message()).await;
    }
    let initial_budget = spend.budget_state(config);

    // T7: gather the snapshot (IO), then assemble (pure, digested). The
    // digest lands in the wake event so the Exec's worldview is auditable.
    let spent_usd = spend.spent_usd(&config.name);
    let snapshot = gather_snapshot(
        &container,
        org,
        authority,
        config,
        reason,
        owner_actor_id,
        human_is_membership_owner,
        conversation_inbox,
        owed_judgements,
        pending_mention,
        spent_usd,
        initial_budget
            .remaining_micro_usd()
            .map(|remaining| remaining as f64 / 1_000_000.0),
        hosted_identity.as_ref().map(|_| (String::new(), None)),
    )
    .await?;
    let package = context::assemble(&snapshot);
    let candidates = crate::model_gateway::available_candidates(
        config,
        config.agent_preference("exec", exec_model.as_deref()),
        authority,
    )
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
        org.emit_event(
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
        let harness = config.coordination_harness;
        let outcome = match harness {
            crate::runtime::AgentHarness::RestlessManaged
            | crate::runtime::AgentHarness::ClaudeAgent
            | crate::runtime::AgentHarness::CustomAcp => {
                let controls = acp::AgentControls::company_actor(package.system_prompt.clone())?
                    .with_mcp_servers(mcp_servers);
                if let Some(identity) = &hosted_identity {
                    let transport = crate::runtime_bridge::open_agent_transport(
                        runtime_bridges,
                        identity,
                        &auth,
                        "/company",
                        "exec",
                        &responsibility,
                        harness,
                        &package.system_prompt,
                    )
                    .await?;
                    let complete_reply_hook = complete_reply_hook.clone();
                    acp::with_remote_agent(
                        transport,
                        harness,
                        &auth,
                        "/company",
                        "exec",
                        &responsibility,
                        controls,
                        observer.clone(),
                        {
                            let company = config.name.clone();
                            let model = model.clone();
                            let cancellation = cancellation.clone();
                            let session_org = org.clone();
                            let turn_context = turn_context.clone();
                            let session_responsibility = responsibility.clone();
                            move |session| {
                                Box::pin(async move {
                                    run_ready_exec_session(
                                        session,
                                        &session_org,
                                        &turn_context,
                                        &company,
                                        &model,
                                        &session_responsibility,
                                        remaining,
                                        metered,
                                        focused_mention,
                                        complete_reply_hook,
                                        &cancellation,
                                    )
                                    .await
                                })
                            }
                        },
                    )
                    .await
                    .map(acp::SessionOutcome::Completed)
                } else {
                    let complete_reply_hook = complete_reply_hook.clone();
                    acp::with_agent_outcome(
                        &container,
                        harness,
                        &auth,
                        "/company",
                        "exec",
                        &responsibility,
                        controls,
                        observer.clone(),
                        {
                            let company = config.name.clone();
                            let model = model.clone();
                            let cancellation = cancellation.clone();
                            let session_org = org.clone();
                            let turn_context = turn_context.clone();
                            let session_responsibility = responsibility.clone();
                            move |session| {
                                Box::pin(async move {
                                    run_ready_exec_session(
                                        session,
                                        &session_org,
                                        &turn_context,
                                        &company,
                                        &model,
                                        &session_responsibility,
                                        remaining,
                                        metered,
                                        focused_mention,
                                        complete_reply_hook,
                                        &cancellation,
                                    )
                                    .await
                                })
                            }
                        },
                    )
                    .await
                }
            }
            crate::runtime::AgentHarness::Codex => {
                if hosted_identity.is_some() {
                    anyhow::bail!("hosted Runtime Exec requires the restless-managed ACP harness");
                }
                let complete_reply_hook = complete_reply_hook.clone();
                crate::codex::with_agent_outcome(
                    &container,
                    &auth,
                    "/company",
                    "exec",
                    &responsibility,
                    &package.system_prompt,
                    mcp_servers,
                    observer.clone(),
                    {
                        let company = config.name.clone();
                        let model = model.clone();
                        let cancellation = cancellation.clone();
                        let session_org = org.clone();
                        let turn_context = turn_context.clone();
                        let session_responsibility = responsibility.clone();
                        move |session| {
                            Box::pin(async move {
                                run_ready_exec_session(
                                    session,
                                    &session_org,
                                    &turn_context,
                                    &company,
                                    &model,
                                    &session_responsibility,
                                    remaining,
                                    metered,
                                    focused_mention,
                                    complete_reply_hook,
                                    &cancellation,
                                )
                                .await
                            })
                        }
                    },
                )
                .await
            }
        };

        // The turn itself was already classified inside `run_turn`, once, by
        // the one function entitled to do it. Failures around session opening
        // have no `TurnEnd` and no usage, but may still be provider-specific.
        let outcome = match outcome {
            Ok(acp::SessionOutcome::Completed(result)) => Ok(result),
            Ok(acp::SessionOutcome::CleanupFailed {
                outcome: (mut report, usage),
                error,
            }) if report.reply_complete => {
                report.termination = Termination::Blocked;
                report.reason = format!(
                    "{COMPLETION_PROTOCOL_PREFIX}The completed owner reply was preserved, but agent session cleanup failed: {error:#}"
                );
                report.retry_after_seconds = None;
                Ok((report, usage))
            }
            Ok(acp::SessionOutcome::CleanupFailed { error, .. }) => {
                Err(error.context("agent session cleanup failed after an incomplete Exec turn"))
            }
            Err(error) => Err(error),
        };
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
            if hosted_identity.is_none() {
                match harness {
                    crate::runtime::AgentHarness::RestlessManaged
                    | crate::runtime::AgentHarness::ClaudeAgent
                    | crate::runtime::AgentHarness::CustomAcp => {
                        acp::discard_session_locator(
                            &container,
                            harness,
                            &config.name,
                            "exec",
                            &responsibility,
                        )
                        .await?;
                    }
                    crate::runtime::AgentHarness::Codex => {
                        crate::codex::discard_session_locator(
                            &container,
                            &config.name,
                            "exec",
                            &responsibility,
                        )
                        .await?;
                    }
                }
            }
            org.emit_event(
                "model_context_reconstruction_scheduled",
                Some("exec"),
                serde_json::json!({
                    "model": model,
                    "responsibility": &responsibility,
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
        if !report.reply_complete {
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
        reply_complete: false,
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
    let access = if model.starts_with("native-custom-") {
        crate::custom_harness::session_access(company, actor, model).await?
    } else if model.starts_with("native-") {
        crate::native_harness::session_access(company, actor, model).await?
    } else {
        crate::model_gateway::client()?.auth_for(
            model,
            capabilities,
            company,
            actor,
            &session_id,
            responsibility,
            work_id,
            attempt_id,
        )?
    };
    Ok(acp::AgentAuth {
        model: model.to_string(),
        effort: effort.to_string(),
        company: company.to_string(),
        session_id: session_id.clone(),
        coordination_token_env: "RESTLESS_SESSION_CAPABILITY".to_string(),
        coordination_token: capabilities.issue_actor_session(
            company,
            actor,
            &session_id,
            work_id,
            attempt_id,
        )?,
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
        reply_complete: false,
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
        crate::model_gateway::ModelBilling::NativeApi => usage.cost_usd,
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
trait ExecutiveSession: Sync {
    fn readiness_observation(&self) -> serde_json::Value;
    fn set_live_observer_enabled(&self, enabled: bool);
    fn prompt_exec<'a>(
        &'a self,
        text: &'a str,
        enforce_spend_budget: bool,
        remaining_budget_usd: f64,
        cancellation: &'a CancellationToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = acp::TurnEnd> + Send + 'a>>;
    fn prompt_once<'a>(
        &'a self,
        text: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<acp::TurnTranscript>> + Send + 'a>>;
    fn cancel<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>>;
}

impl ExecutiveSession for acp::AgentSession {
    fn readiness_observation(&self) -> serde_json::Value {
        acp::AgentSession::readiness_observation(self)
    }

    fn set_live_observer_enabled(&self, enabled: bool) {
        acp::AgentSession::set_live_observer_enabled(self, enabled);
    }

    fn prompt_exec<'a>(
        &'a self,
        text: &'a str,
        enforce_spend_budget: bool,
        remaining_budget_usd: f64,
        cancellation: &'a CancellationToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = acp::TurnEnd> + Send + 'a>> {
        Box::pin(acp::AgentSession::prompt_live(
            self,
            text,
            move |usage| {
                enforce_spend_budget
                    && usage
                        .cost_usd
                        .is_some_and(|cost| cost >= remaining_budget_usd)
            },
            cancellation,
        ))
    }

    fn prompt_once<'a>(
        &'a self,
        text: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<acp::TurnTranscript>> + Send + 'a>>
    {
        Box::pin(async move {
            acp::AgentSession::prompt(self, text).await?;
            Ok(acp::AgentSession::take_transcript(self))
        })
    }

    fn cancel<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(acp::AgentSession::cancel(self))
    }
}

impl ExecutiveSession for crate::codex::CodexSession {
    fn readiness_observation(&self) -> serde_json::Value {
        crate::codex::CodexSession::readiness_observation(self)
    }

    fn set_live_observer_enabled(&self, enabled: bool) {
        crate::codex::CodexSession::set_live_observer_enabled(self, enabled);
    }

    fn prompt_exec<'a>(
        &'a self,
        text: &'a str,
        enforce_spend_budget: bool,
        remaining_budget_usd: f64,
        cancellation: &'a CancellationToken,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = acp::TurnEnd> + Send + 'a>> {
        Box::pin(crate::codex::CodexSession::prompt_live(
            self,
            text,
            enforce_spend_budget,
            remaining_budget_usd,
            cancellation,
        ))
    }

    fn prompt_once<'a>(
        &'a self,
        text: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<acp::TurnTranscript>> + Send + 'a>>
    {
        Box::pin(async move {
            match crate::codex::CodexSession::prompt_live(
                self,
                text,
                false,
                f64::MAX,
                &CancellationToken::new(),
            )
            .await
            {
                acp::TurnEnd::Completed { transcript } => Ok(transcript),
                end => anyhow::bail!("Codex postflight did not complete: {:?}", end),
            }
        })
    }

    fn cancel<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(crate::codex::CodexSession::cancel(self))
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the durable Exec launch observation and charged turn share one exact boundary"
)]
async fn run_ready_exec_session(
    session: &dyn ExecutiveSession,
    org: &restless_orgintel::OrgIntel,
    turn_context: &str,
    company: &str,
    model: &str,
    responsibility: &str,
    remaining_budget_usd: f64,
    enforce_spend_budget: bool,
    focused_mention: bool,
    complete_reply_hook: Option<CompleteReplyHook>,
    cancellation: &CancellationToken,
) -> Result<(WakeReport, Option<acp::TurnUsage>)> {
    let readiness = session.readiness_observation();
    let launch_id = acp::required_readiness_text(&readiness, "launch_id")?;
    let harness = acp::required_readiness_text(&readiness, "harness")?;
    let harness_build = acp::required_readiness_text(&readiness, "harness_build")?;
    let transport = acp::required_readiness_text(&readiness, "transport")?;
    let ready_model = acp::required_readiness_text(&readiness, "model")?;
    let configured_effort = acp::required_readiness_text(&readiness, "configured_effort")?;
    let provider_session_id = acp::required_readiness_text(&readiness, "session_id")?;
    let resumed = acp::required_readiness_bool(&readiness, "resumed")?;
    let reconstructed = acp::required_readiness_bool(&readiness, "reconstructed")?;
    org.record_agent_session(restless_orgintel::NewAgentSession {
        launch_id,
        actor_id: "exec",
        responsibility,
        work_id: None,
        attempt_id: None,
        harness,
        harness_build,
        transport,
        model: ready_model,
        configured_effort,
        provider_session_id,
        capabilities: &readiness,
        resumed,
        reconstructed,
    })
    .await?;
    org.emit_event("model_session_ready", Some("exec"), readiness)
        .await?;
    session.set_live_observer_enabled(true);
    run_turn(
        session,
        turn_context,
        company,
        model,
        remaining_budget_usd,
        enforce_spend_budget,
        focused_mention,
        complete_reply_hook,
        cancellation,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_turn(
    session: &dyn ExecutiveSession,
    context: &str,
    company: &str,
    model: &str,
    remaining_budget_usd: f64,
    enforce_spend_budget: bool,
    focused_mention: bool,
    complete_reply_hook: Option<CompleteReplyHook>,
    cancellation: &CancellationToken,
) -> Result<(WakeReport, Option<acp::TurnUsage>)> {
    // Run for as long as the agent is alive, not for a fixed wall-clock
    // budget. How the turn ends is a deterministic observation — the agent
    // finished, went silent, hit the ceiling, or the transport broke — never a
    // guess about whether the model is "stuck or just thinking", which is
    // judgement and not the daemon's.
    let end = session
        .prompt_exec(
            context,
            enforce_spend_budget,
            remaining_budget_usd,
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
    let report = |termination, reason, retry_after_seconds, reply_complete| WakeReport {
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
        reply_complete,
    };

    match verdict {
        // Recoverable: the work is on the volume and a fresh session
        // rehydrates from it, so this costs the owner nothing.
        health::Verdict::Resume(reason) => {
            tracing::warn!(company, %reason, "turn stopped early; resuming next wake");
            Ok((
                report(Termination::Continue, reason, Some(60), false),
                usage,
            ))
        }
        // Only the owner can clear this. `record_outcome` latches the
        // milestone and mails once — never a re-wake loop (F1).
        health::Verdict::Blocked(blocked) => {
            tracing::warn!(company, reason = %blocked.message(), "turn blocked the company");
            Ok((
                report(Termination::Blocked, blocked.message(), None, false),
                usage,
            ))
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
                return Ok((
                    report(Termination::Blocked, blocked.message(), None, false),
                    usage,
                ));
            }
            if focused_mention {
                let answer = transcript.last_message_text.trim();
                return Ok((
                    if answer.is_empty() {
                        report(
                            Termination::Continue,
                            "the focused collaboration mention turn completed without a final answer"
                                .to_string(),
                            Some(CONTINUE_WAKE_DELAY_SECONDS),
                            false,
                        )
                    } else {
                        report(
                            Termination::OutcomeMet,
                            "the focused collaboration mention received a complete answer"
                                .to_string(),
                            None,
                            true,
                        )
                    },
                    usage,
                ));
            }
            if let (Some(hook), Some(reply)) = (
                complete_reply_hook.as_ref(),
                (!transcript.last_message_text.trim().is_empty())
                    .then(|| transcript.last_message_text.clone()),
            ) {
                if let Err(error) = hook(reply).await {
                    // The scheduler retries the exact same idempotent
                    // finalization after postflight if this commit was not
                    // confirmed. Keep the model and termination path intact.
                    tracing::warn!(company, %error, "could not persist complete Exec reply before termination judgement");
                }
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
                    !transcript.last_message_text.trim().is_empty(),
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
pub(crate) const TERMINATION_PROMPT: &str = "The turn is ending now. Based on everything above, decide how the \
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
/// erase a completed, metered work turn. Transport loss or timeout records a
/// protocol blockage rather than rerunning productive work. Deterministic
/// extraction recovers a valid wrapped envelope without another model call.
async fn termination_decision(
    session: &dyn ExecutiveSession,
    cancellation: &CancellationToken,
) -> TerminationDecision {
    let prompted = tokio::select! {
        () = cancellation.cancelled() => {
            let _ = session.cancel().await;
            return retry_termination("the owner interrupted the turn to send new direction");
        }
        prompted = tokio::time::timeout(
            TERMINATION_TIMEOUT,
            session.prompt_once(TERMINATION_PROMPT),
        ) => prompted,
    };
    let Ok(prompted) = prompted else {
        let _ = session.cancel().await;
        tracing::warn!(
            timeout_s = TERMINATION_TIMEOUT.as_secs(),
            "termination decision timed out; preserving completed work turn"
        );
        return protocol_blocked(format!(
            "Exec completion protocol timed out after {}s; the productive turn is preserved for accountable review",
            TERMINATION_TIMEOUT.as_secs()
        ));
    };
    let transcript = match prompted {
        Ok(transcript) => transcript,
        Err(error) => {
            tracing::warn!(%error, "termination decision transport failed; preserving work turn");
            return protocol_blocked(format!(
                "Exec completion protocol transport failed; the productive turn is preserved for accountable review: {error:#}"
            ));
        }
    };
    if let Some(parsed) = parse_termination(&transcript.text) {
        return parsed;
    }
    if let Some(blocked) = health::classify_provider_error_content(&transcript.text) {
        return TerminationDecision {
            termination: Termination::Blocked,
            reason: blocked.message(),
            retry_after_seconds: None,
        };
    }
    tracing::warn!(
        said = %transcript.text.chars().take(600).collect::<String>(),
        "termination decision unparseable; preserving completed work turn"
    );
    protocol_blocked(
        "Exec completion protocol was malformed or ambiguous; no separately admitted effect-free correction session is available on this path, so the productive turn is preserved for accountable review",
    )
}

/// Parse the termination envelope. Delegation is absent on purpose: the
/// actor writes Work graph rows with the CLI while it has full context.
pub(crate) fn parse_termination(text: &str) -> Option<TerminationDecision> {
    // Providers and harnesses may wrap the requested object in a code fence or
    // append diagnostics containing braces. Parsing from the first `{` through
    // the last `}` makes that harmless transport decoration destroy an otherwise
    // valid decision. Inspect complete top-level objects only, so a nested
    // example inside unrelated JSON is not mistaken for the answer. Exactly one
    // recognized envelope is required; multiple answers are ambiguous even if
    // they happen to agree.
    let mut recognized = None;
    let mut object_start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        if object_start.is_none() {
            if character == '{' {
                object_start = Some(index);
                depth = 1;
                in_string = false;
                escaped = false;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth != 0 {
                    continue;
                }
                let start = object_start.take().expect("an object is being scanned");
                let object = &text[start..index + character.len_utf8()];
                let Ok(output) = serde_json::from_str::<TerminationOutput>(object) else {
                    continue;
                };
                let waiting = output.decision == "waiting";
                let termination = match output.decision.as_str() {
                    "continue" => Termination::Continue,
                    "waiting" => Termination::Continue,
                    "blocked" | "blocked_on_owner" => Termination::Blocked,
                    "changes_requested" => Termination::ChangesRequested,
                    "outcome_met" => Termination::OutcomeMet,
                    "abandon" => Termination::Abandon,
                    _ => continue,
                };
                if recognized.is_some() {
                    return None;
                }
                recognized = Some(TerminationDecision {
                    termination,
                    reason: output.reason,
                    retry_after_seconds: (termination == Termination::Continue && !waiting)
                        .then_some(CONTINUE_WAKE_DELAY_SECONDS),
                });
            }
            _ => {}
        }
    }
    recognized
}

fn retry_termination(reason: impl Into<String>) -> TerminationDecision {
    TerminationDecision {
        termination: Termination::Continue,
        reason: reason.into(),
        retry_after_seconds: Some(CONTINUE_WAKE_DELAY_SECONDS),
    }
}

pub(crate) const COMPLETION_PROTOCOL_PREFIX: &str = "[completion_protocol] ";

fn protocol_blocked(reason: impl Into<String>) -> TerminationDecision {
    TerminationDecision {
        termination: Termination::Blocked,
        reason: format!("{COMPLETION_PROTOCOL_PREFIX}{}", reason.into()),
        retry_after_seconds: None,
    }
}

/// Gather the wake's read-only snapshot (the only IO in context assembly;
/// `context::assemble` is pure). Files win over memory — this is all the
/// continuity the Exec gets, so it is all here.
#[expect(
    clippy::too_many_arguments,
    reason = "snapshot IO keeps company, authority, immediate focus and budget inputs explicit"
)]
async fn gather_snapshot(
    container: &str,
    org: &OrgIntel,
    authority: &crate::authority::AuthorityStore,
    config: &CompanyConfig,
    reason: &str,
    owner_actor_id: &str,
    human_is_membership_owner: bool,
    conversation_inbox: &[restless_orgintel::MessageRow],
    owed_judgements: &[restless_orgintel::OwnerHandoffRow],
    pending_mention: Option<&crate::mentions::MentionContext>,
    spent_usd: f64,
    remaining_usd: Option<f64>,
    hosted_files: Option<(String, Option<String>)>,
) -> Result<ContextSnapshot> {
    let (current_plan, latest_journal) = match hosted_files {
        Some(files) => files,
        None => (
            read_company_file(container, "/company/org/exec/current-plan.md").await?,
            latest_journal_entry(container).await?,
        ),
    };
    // These reads use independent records and never mutate company state. Run
    // two queries concurrently within each existing database pool, keeping
    // the fan-out bounded so a slow database is not flooded with snapshot
    // work. Reuse Work and effect rows below instead of rereading them for
    // organisational signals. Focus -> conversation remains ordered below.
    let (work, goals, legal_identity, effect_records) = tokio::try_join!(
        async { org.list_work().await.map_err(anyhow::Error::from) },
        async { org.list_goals().await.map_err(anyhow::Error::from) },
        crate::legal::safe_projection(authority, &config.name),
        authority.records_of_kind(&config.name, "effect"),
    )?;
    let effect_ledger =
        crate::reconcile::effect_ledger(authority, &config.name, &effect_records)
            .await?
            .summary();
    let org_signals = health::organisational(spent_usd, &work, &effect_records)
        .into_iter()
        .map(|signal| format!("[{}] {}", signal.kind, signal.detail))
        .collect();
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
        .filter(|item| match pending_mention {
            None => true,
            Some(mention) => mention.work_id() == Some(item.id),
        })
        .collect();
    let inbox = if pending_mention.is_some() {
        Vec::new()
    } else {
        conversation_inbox.to_vec()
    };
    let unread_owner_message_ids = inbox
        .iter()
        .filter(|message| message.from_actor == owner_actor_id)
        .map(|message| message.id)
        .collect::<HashSet<_>>();
    let inbox_skills = org
        .message_skill_selections(&inbox.iter().map(|message| message.id).collect::<Vec<_>>())
        .await?;
    let recent_owner_conversation = if pending_mention.is_some() {
        Vec::new()
    } else {
        let focus = org.human_conversation_focus(owner_actor_id, "exec").await?;
        org.human_conversation_since(owner_actor_id, "exec", focus.after_message_id, 12)
            .await?
            .into_iter()
            .filter(|message| !unread_owner_message_ids.contains(&message.id))
            .collect()
    };
    let owed_judgements = if pending_mention.is_some() {
        Vec::new()
    } else {
        owed_judgements.to_vec()
    };
    Ok(ContextSnapshot {
        company: config.name.clone(),
        owner_actor_id: owner_actor_id.to_string(),
        human_is_membership_owner,
        operating_rules: crate::context::COMPANY_OPERATING_RULES.to_string(),
        mission: config.mission.clone(),
        outcome_standard: config.outcome_standard,
        legal_identity,
        current_plan,
        latest_journal,
        open_work: open,
        open_goals: goals
            .into_iter()
            .filter(|goal| goal.closed_at.is_none())
            .collect(),
        recent_owner_conversation,
        inbox,
        inbox_skills,
        owed_judgements,
        pending_mention: pending_mention.cloned(),
        wake_reason: reason.to_string(),
        budget_remaining_usd: remaining_usd,
        budget_ceiling_usd: config.spend_ceiling_usd.as_usd(),
        effect_ledger,
        org_signals,
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
        // A scheduled responsibility already owns its continuation clock.
        // Its fenced Opportunity is deferred by the caller after this turn;
        // a second free-standing Exec schedule would replay the same work.
        let claimed_opportunity = org.list_open_opportunities().await?.into_iter().any(|row| {
            row.actor_id == "exec"
                && row.lease_owner.as_deref() == Some("exec")
                && row
                    .lease_expires_at
                    .is_some_and(|until| until > chrono::Utc::now())
        });
        if claimed_opportunity {
            return Ok(());
        }
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
        let objective = format!(
            "Resume the active Exec milestone after a bounded retry delay. Preserve the existing company policy and authority boundaries. Context: {}",
            schedule_reason.chars().take(500).collect::<String>()
        );
        org.create_exact_schedule_with_responsibility(
            "exec",
            &schedule_reason,
            fire_at,
            "local_mac",
            Uuid::new_v4(),
            1,
            &objective,
            serde_json::json!({ "window_seconds": 3600 }),
        )
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
        reply_complete: false,
    };
    record_outcome(org, &report).await
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
    fn termination_parser_ignores_surrounding_json_like_diagnostics() {
        let decision = parse_termination(
            "diagnostic {\"request_id\":\"abc\"}\n```json\n{\"decision\":\"blocked\",\"reason\":\"approval required\"}\n```\nmetadata {\"latency_ms\":42}",
        )
        .unwrap();
        assert_eq!(decision.termination, Termination::Blocked);
        assert_eq!(decision.reason, "approval required");
        assert!(parse_termination(
            "{\"decision\":\"blocked\",\"reason\":\"example\"}\n{\"decision\":\"outcome_met\",\"reason\":\"actual\"}"
        )
        .is_none());
    }

    #[test]
    fn owner_interruption_preserves_work_and_retries() {
        let decision = retry_termination("the owner interrupted the turn to send new direction");
        assert_eq!(decision.termination, Termination::Continue);
        assert_eq!(decision.retry_after_seconds, Some(60));
        assert_eq!(
            decision.reason,
            "the owner interrupted the turn to send new direction"
        );
    }
}
