//! ACP session execution, model failover, and metering for one Staff turn.
//!
//! This module owns only the ephemeral cognitive process. Work/Attempt state
//! remains in OrgIntel and completion/recovery lives beside its Runtime
//! observations.

use anyhow::{bail, Result};

use crate::acp::{self, AgentAuth};
use crate::exec::{self, Termination};
use crate::health;
use crate::spend::SpendLedger;

use super::context::{actor_posture, workspace_instruction};

/// One claimed Work Attempt across all provider candidates.
pub(super) struct StaffRun {
    pub(super) container: String,
    pub(super) workdir: String,
    pub(super) company: String,
    pub(super) actor: String,
    pub(super) name: String,
    /// Trusted assignment/team state assembled by OrgIntel and the bridge.
    pub(super) task: String,
    /// The immediate input for this ACP turn: owner text, Work feedback, or a
    /// bounded instruction to begin the already-claimed Attempt.
    pub(super) turn_prompt: String,
    pub(super) role: String,
    pub(super) spine: String,
    pub(super) candidates: Vec<String>,
    pub(super) org: restless_orgintel::OrgIntel,
    pub(super) spend: SpendLedger,
    pub(super) meter: crate::spend::TurnMeter,
    pub(super) spend_ceiling: crate::runtime::SpendCeiling,
    pub(super) authority: crate::authority::AuthorityStore,
    pub(super) conversation: bool,
    /// One durable actor keeps the same accountable posture whether Work or
    /// conversation woke it.
    pub(super) accountable_lead: bool,
    pub(super) observer: Option<acp::SessionObserver>,
}

pub(super) struct StaffOutcome {
    pub(super) termination: Termination,
    pub(super) summary: String,
    pub(super) output_tokens: Option<u64>,
}

pub(super) async fn run_staff_with_failover(run: StaffRun) -> Result<StaffOutcome> {
    let mut continuity_note: Option<String> = None;

    for (index, model) in run.candidates.iter().enumerate() {
        run.org
            .emit_event(
                "model_attempt",
                Some(&run.actor),
                serde_json::json!({ "model": model, "attempt": index + 1 }),
            )
            .await?;

        let auth = match crate::exec::agent_auth_for_model(model).await {
            Ok(auth) => auth,
            Err(error) => {
                let text = format!("{error:#}");
                let blocked = health::classify_provider_error(&text)
                    .unwrap_or_else(|| health::Blocked::transport(&text));
                crate::model_gateway::record_cooldown(
                    &run.authority,
                    &run.company,
                    model,
                    blocked.kind,
                    &blocked.message(),
                )
                .await?;
                if let Some(next) = run.candidates.get(index + 1) {
                    record_staff_failover(
                        &run.org,
                        &run.actor,
                        model,
                        next,
                        blocked.kind,
                        &blocked.message(),
                    )
                    .await?;
                    continuity_note = Some(blocked.message());
                    continue;
                }
                return Ok(StaffOutcome {
                    termination: Termination::Blocked,
                    summary: blocked.message(),
                    output_tokens: None,
                });
            }
        };

        let billing = auth.billing;
        // The shared ledger gate is acquired before opening the ACP session.
        // It is only charged-turn admission: no Work state, queue, or durable
        // reservation is introduced here.
        let metered_turn = run.spend.acquire_metered_turn(&run.company, billing).await;
        let remaining_micro_usd = run
            .spend
            .remaining_micro_usd_for(&run.company, run.spend_ceiling);
        let remaining_budget_usd = remaining_micro_usd as f64 / 1_000_000.0;
        if billing == crate::model_gateway::ModelBilling::MeteredApi && remaining_micro_usd == 0 {
            drop(metered_turn);
            return Ok(StaffOutcome {
                termination: Termination::Blocked,
                summary: format!(
                    "[budget] {} has spent its ${:.2} ceiling; the owner must raise it before a queued provider turn starts",
                    run.company, run.spend_ceiling.as_usd()
                ),
                output_tokens: None,
            });
        }
        let spine = continuity_note.as_ref().map_or_else(
            || run.spine.clone(),
            |failure| {
                format!(
                    "{}\n# Provider continuity\nA previous model attempt failed: {failure}\n\
                     Continue the same assignment from the existing company files. Before any \
                     material external effect, reconcile the Authority receipt and idempotency key.\n",
                    run.spine
                )
            },
        );
        let outcome = run_staff(StaffBrief {
            container: run.container.clone(),
            auth,
            workdir: run.workdir.clone(),
            company: run.company.clone(),
            actor: run.actor.clone(),
            name: run.name.clone(),
            task: run.task.clone(),
            turn_prompt: run.turn_prompt.clone(),
            role: run.role.clone(),
            spine,
            remaining_budget_usd,
            enforce_spend_budget: billing == crate::model_gateway::ModelBilling::MeteredApi,
            conversation: run.conversation,
            accountable_lead: run.accountable_lead,
            observer: run.observer.clone(),
        })
        .await;

        let failure_kind = match &outcome {
            Ok((Termination::Blocked, reason, _, _)) => health::block_kind_from_message(reason)
                .or_else(|| health::classify_provider_error(reason).map(|blocked| blocked.kind)),
            Err(error) => {
                let text = format!("{error:#}");
                Some(
                    health::classify_provider_error(&text)
                        .unwrap_or_else(|| health::Blocked::transport(&text))
                        .kind,
                )
            }
            _ => None,
        };

        // Meter before deciding whether to continue. A classified unpriced
        // provider refusal is telemetry, not mysterious spend; subscription
        // usage is recorded as zero charged dollars, with the catalogue
        // estimate retained only in the event.
        if let Ok((_, _, spent, _)) = &outcome {
            record_staff_usage(
                &run.meter,
                &run.org,
                &run.company,
                &run.actor,
                model,
                billing,
                spent,
                failure_kind,
            )
            .await?;
        }
        // A waiting charged turn sees this final ledger record before it
        // starts. Cooldown and failover bookkeeping do not hold the lane.
        drop(metered_turn);

        if let Some(kind) = failure_kind.filter(|kind| health::is_provider_failover_kind(*kind)) {
            let reason = match &outcome {
                Ok((_, reason, _, _)) => reason.clone(),
                Err(error) => format!("{error:#}"),
            };
            crate::model_gateway::record_cooldown(
                &run.authority,
                &run.company,
                model,
                kind,
                &reason,
            )
            .await?;
        }

        if let (Some(kind), Some(next)) = (
            failure_kind.filter(|kind| health::is_provider_failover_kind(*kind)),
            run.candidates.get(index + 1),
        ) {
            let reason = match &outcome {
                Ok((_, reason, _, _)) => reason.clone(),
                Err(error) => format!("{error:#}"),
            };
            record_staff_failover(&run.org, &run.actor, model, next, kind, &reason).await?;
            continuity_note = Some(reason);
            continue;
        }

        if failure_kind.is_none() {
            run.authority
                .clear_model_cooldown(&run.company, model)
                .await?;
        }
        return outcome.map(|(termination, summary, _, output_tokens)| StaffOutcome {
            termination,
            summary,
            output_tokens,
        });
    }

    unreachable!("validated Staff model policy always has a candidate")
}

async fn record_staff_failover(
    org: &restless_orgintel::OrgIntel,
    actor: &str,
    from: &str,
    to: &str,
    kind: health::BlockKind,
    reason: &str,
) -> Result<()> {
    org.emit_event(
        "model_failover",
        Some(actor),
        serde_json::json!({
            "from": from,
            "to": to,
            "kind": kind.as_str(),
            "reason": reason.chars().take(300).collect::<String>(),
        }),
    )
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn record_staff_usage(
    meter: &crate::spend::TurnMeter,
    org: &restless_orgintel::OrgIntel,
    company: &str,
    actor: &str,
    model: &str,
    billing: crate::model_gateway::ModelBilling,
    snapshots: &[acp::TurnUsage],
    failure_kind: Option<health::BlockKind>,
) -> Result<()> {
    let Some((usage, charged_cost_usd)) = record_final_staff_spend(
        meter,
        company,
        actor,
        model,
        billing,
        snapshots,
        failure_kind,
    ) else {
        return Ok(());
    };
    org.emit_event(
        "turn_usage",
        Some(actor),
        serde_json::json!({
            "model": model,
            "billing": billing.as_str(),
            // Compatibility fields retain their names, while the explicit
            // fields state the ACP semantics for new readers.
            "tokens": usage.used,
            "tokens_in_context": usage.used,
            "context_size": usage.size,
            "context_used_pct": if usage.size == 0 {
                0
            } else {
                usage.used.saturating_mul(100) / usage.size
            },
            "cost_usd": charged_cost_usd,
            "cumulative_session_cost_usd": charged_cost_usd,
            "usage_semantics": "final_cumulative_session_snapshot",
            "estimated_list_cost_usd": (billing == crate::model_gateway::ModelBilling::Subscription)
                .then_some(usage.cost_usd)
                .flatten(),
            "unpriced_provider_refusal": (billing == crate::model_gateway::ModelBilling::MeteredApi
                && usage.cost_usd.is_none()
                && failure_kind.is_some_and(|kind| matches!(kind,
                    health::BlockKind::Credential | health::BlockKind::Quota | health::BlockKind::Model | health::BlockKind::NoOp))),
        }),
    )
    .await?;
    Ok(())
}

/// ACP usage updates are session snapshots: context usage is the latest value
/// and cost is cumulative. Re-prompts on one Staff session therefore refine
/// one bill; summing them charges every prefix again.
fn final_session_usage(snapshots: &[acp::TurnUsage]) -> Option<acp::TurnUsage> {
    snapshots.iter().copied().fold(None, |previous, current| {
        let cost_usd = match (previous.and_then(|usage| usage.cost_usd), current.cost_usd) {
            (Some(previous), Some(current)) => Some(previous.max(current)),
            (previous, current) => current.or(previous),
        };
        Some(acp::TurnUsage {
            used: current.used,
            size: current.size,
            cost_usd,
        })
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn record_final_staff_spend(
    meter: &crate::spend::TurnMeter,
    company: &str,
    actor: &str,
    model: &str,
    billing: crate::model_gateway::ModelBilling,
    snapshots: &[acp::TurnUsage],
    failure_kind: Option<health::BlockKind>,
) -> Option<(acp::TurnUsage, Option<f64>)> {
    let usage = final_session_usage(snapshots)?;
    let charged_cost_usd = match billing {
        crate::model_gateway::ModelBilling::MeteredApi => usage.cost_usd,
        crate::model_gateway::ModelBilling::Subscription => Some(0.0),
    };
    if crate::exec::should_record_spend(billing, usage, failure_kind) {
        meter.record(company, actor, model, usage.used, charged_cost_usd);
    }
    Some((usage, charged_cost_usd))
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
    turn_prompt: String,
    /// What this actor IS. Reaches the agent's own briefing, so the
    /// specialisation is something it knows about itself rather than a label
    /// only the daemon can see.
    role: String,
    /// The shared spine, or empty in `minimal_team` and `single_agent`.
    spine: String,
    /// Snapshot of the company envelope left when this provider session was
    /// launched. ACP reports cumulative session cost, so the in-turn fuse can
    /// compare every later update to this one value without double counting.
    remaining_budget_usd: f64,
    /// Subscription routes report zero charged dollars by design. Match the
    /// Exec path and apply this dollar fuse only when the provider reports a
    /// metered cost.
    enforce_spend_budget: bool,
    /// A lead/actor response turn has no claimed Work Attempt. It uses the
    /// same process, model, failover and supervision path with a team-scoped
    /// brief rather than inventing a second runtime class.
    conversation: bool,
    accountable_lead: bool,
    observer: Option<acp::SessionObserver>,
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

/// The Staff counterpart to Exec's in-turn dollar fuse. This intentionally
/// consumes only the provider's cumulative charged cost, never a catalogue
/// estimate for a subscription route. It is a local guard against one live
/// session running beyond its remaining company envelope; concurrent-session
/// allocation remains the existing coarse company-level policy.
pub(super) fn staff_spend_limit_reached(
    enforce_spend_budget: bool,
    remaining_budget_usd: f64,
    usage: &acp::TurnUsage,
) -> bool {
    enforce_spend_budget
        && usage
            .cost_usd
            .is_some_and(|cost| cost >= remaining_budget_usd)
}

/// Staff judge the bounded assignment they own, not the Exec's company-wide
/// milestone. Reusing the Exec envelope here made a critic who had written a
/// complete review report `blocked` merely because another critic or a later
/// revision still remained. That destroys the meaning of the Work
/// state and makes successful handoffs look like failures.
const SPECIALIST_TERMINATION_PROMPT: &str =
    "Your assigned specialist task is ending now. Based on the task you were given, answer with JSON only, no prose:\n\
    {\"decision\": \"continue\" | \"blocked\" | \"changes_requested\" | \"outcome_met\" | \"abandon\", \
     \"reason\": \"<one line>\"}\n\
    - continue: more machine-doable work remains in your assigned task\n\
    - blocked: you cannot complete your assigned task until a human or external event acts; say exactly what is needed\n\
    - changes_requested: you are a reviewer and found concrete changes; this follows the Work graph's revises edge\n\
    - outcome_met: your assigned task and its requested outputs are complete\n\
    - abandon: your assigned task is not worth continuing; say why\n\
    Judge only your assignment. Other company work, later review, or another actor's task does not make your completed task blocked.";

const LEAD_TERMINATION_PROMPT: &str =
    "Your accountable outcome Work is ending now. Based on the complete charter you own, answer with JSON only, no prose:\n\
    {\"decision\": \"continue\" | \"blocked\" | \"changes_requested\" | \"outcome_met\" | \"abandon\", \
     \"reason\": \"<one line>\"}\n\
    - continue: more machine-doable outcome, integration, native review, or Staff-result work remains\n\
    - blocked: the outcome cannot advance until a human or external event acts; say exactly what is needed\n\
    - changes_requested: this Work is a review responsibility and concrete changes are required\n\
    - outcome_met: the whole assigned outcome is complete, natively inspected, and every claimed Staff contribution is observable\n\
    - abandon: the assigned outcome is not worth continuing; say why\n\
    Judge this lead-owned outcome, not the company portfolio. Unrelated company work does not make a completed outcome blocked.";

pub(super) fn termination_prompt(accountable_lead: bool) -> &'static str {
    if accountable_lead {
        LEAD_TERMINATION_PROMPT
    } else {
        SPECIALIST_TERMINATION_PROMPT
    }
}

async fn run_staff(
    brief: StaffBrief,
) -> Result<(Termination, String, Vec<acp::TurnUsage>, Option<u64>)> {
    let StaffBrief {
        container,
        auth,
        workdir,
        company,
        actor,
        name,
        task,
        turn_prompt,
        role,
        spine,
        remaining_budget_usd,
        enforce_spend_budget,
        conversation,
        accountable_lead,
        observer,
    } = brief;
    let (container, auth, workdir, actor) =
        (container.as_str(), &auth, workdir.as_str(), actor.as_str());
    let assignment = if conversation {
        "woken for a bounded coordination conversation"
    } else {
        "assigned one claimed Work Attempt"
    };
    let posture = actor_posture(accountable_lead);
    let termination_prompt = termination_prompt(accountable_lead);
    let workspace = workspace_instruction(workdir, conversation);
    let system_prompt = format!(
        "# Company operating rules [authoritative — applies to every actor]\n{}\n\n\
         You are {name}, the {role} of {company}, {assignment}. Your stable OrgIntel actor id is `{actor}`.\n\
         {posture}\n\
         {workspace}\n\
         {spine}\n\
         # Trusted assignment context [OrgIntel decision]\n{task}\n\n\
         Work until the task is done or you are stuck. {ending}",
        crate::context::COMPANY_OPERATING_RULES.trim(),
        posture = posture,
        workspace = workspace,
        ending = if conversation {
            "After using any tools you need, end with the complete owner-facing reply and its required intent marker. Do not narrate private reasoning in that reply."
        } else {
            "The session ends when you stop writing; you will then be asked for a decision envelope."
        },
    );
    const CONTINUE_PROMPT: &str =
        "Continue the task. If it is done or you are stuck, stop writing.";
    let controls = acp::AgentControls::company_actor(system_prompt)?;
    let controls = if conversation {
        controls.for_team_coordination()
    } else {
        controls
    };
    acp::with_agent(
        container,
        auth,
        workdir,
        actor,
        controls,
        observer,
        move |session| {
            Box::pin(async move {
                let mut next = turn_prompt;
                // Each prompt yields another cumulative session snapshot. Keep the
                // observations for failure telemetry, then charge only the final
                // snapshot once when this provider attempt ends.
                let mut spent: Vec<acp::TurnUsage> = Vec::new();
                loop {
                    let end = session
                        .prompt_live(&next, move |usage| {
                            staff_spend_limit_reached(
                                enforce_spend_budget,
                                remaining_budget_usd,
                                usage,
                            )
                        })
                        .await;
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
                        if health::block_kind_from_message(&blocked)
                            .is_some_and(health::is_provider_failover_kind)
                        {
                            return Ok((Termination::Blocked, blocked, spent, None));
                        }
                        bail!("staff turn: {blocked}");
                    }

                    if conversation {
                        let transcript = end.into_transcript();
                        // ACP may emit conversational bridge text around tools.
                        // Only its final assistant block is the owner's durable
                        // reply; the earlier blocks are ephemeral activity.
                        let reply = transcript.last_message_text.trim().to_string();
                        if reply.is_empty() {
                            return Ok((
                                Termination::Blocked,
                                "team lead produced no owner-facing reply".to_string(),
                                spent,
                                transcript.output_tokens,
                            ));
                        }
                        if let Some(blocked) = health::classify_provider_error(&reply) {
                            return Ok((
                                Termination::Blocked,
                                blocked.message(),
                                spent,
                                transcript.output_tokens,
                            ));
                        }
                        return Ok((
                            Termination::OutcomeMet,
                            reply,
                            spent,
                            transcript.output_tokens,
                        ));
                    }

                    let end = session.prompt_live(termination_prompt, |_| false).await;
                    if let Some(usage) = end.usage() {
                        spent.push(usage);
                    }
                    if let Some(blocked) = staff_halt(&end) {
                        if health::block_kind_from_message(&blocked)
                            .is_some_and(health::is_provider_failover_kind)
                        {
                            return Ok((Termination::Blocked, blocked, spent, None));
                        }
                        bail!("staff termination ask: {blocked}");
                    }
                    let said = end.into_transcript().text;
                    match exec::parse_termination(&said) {
                        Some(decision) if matches!(decision.termination, Termination::Continue) => {
                            next = CONTINUE_PROMPT.to_string();
                        }
                        Some(decision) => {
                            return Ok((decision.termination, decision.reason, spent, None))
                        }
                        None => {
                            // Before blaming the model, check whether it spoke at
                            // all. omp streams an upstream error body through as
                            // message CONTENT, so a provider refusal arrives as
                            // assistant text: the turn "succeeds", tokens are
                            // consumed, and nothing in the transport looks wrong.
                            //
                            // This is F1 in its third costume. It was fixed for the
                            // Exec path in sprint 02 and the identical gap sat here
                            // untouched until a critic ran on a second provider
                            // hit `429 [1113] Insufficient balance` and was reported
                            // as "staff produced no parseable termination decision"
                            // — which blames the specialist for the wallet.
                            if let Some(blocked) = health::classify_provider_error(&said) {
                                tracing::warn!(
                                    kind = blocked.kind.as_str(),
                                    "staff blocked by the provider, not by its own output"
                                );
                                return Ok((Termination::Blocked, blocked.message(), spent, None));
                            }
                            tracing::warn!(
                                said = %said.chars().take(600).collect::<String>(),
                                "staff termination unparseable"
                            );
                            return Ok((
                                Termination::Blocked,
                                "staff produced no parseable termination decision".to_string(),
                                spent,
                                None,
                            ));
                        }
                    }
                }
            })
        },
    )
    .await
}
