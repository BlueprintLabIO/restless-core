//! Work execution and process supervision. OrgIntel owns the deterministic
//! kickoff: a process may start only with an atomically claimed Work Attempt.
//! The registry below observes live processes and enforces a small resource
//! cap; it never owns delegation or task state.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context as _, Result};
use restless_orgintel::{ClaimedWork, WorkAttemptState, WorkStatus};
use sha2::{Digest as _, Sha256};

use crate::acp::{self, AgentAuth};
use crate::exec::{self, Termination};
use crate::health;
use crate::runtime::{self, CompanyConfig};
use crate::spend::SpendLedger;

/// Resource guardrail, not a coordination policy. OrgIntel readiness and one
/// live process per durable actor still determine which Staff may run.
const STAFF_CAP_PER_COMPANY: usize = 100;
/// (company, actor) pairs with a live supervised process.
#[derive(Clone, Default)]
pub struct StaffRegistry {
    running: Arc<Mutex<HashSet<(String, String)>>>,
}

impl StaffRegistry {
    pub fn has_capacity(&self, company: &str) -> bool {
        self.running
            .lock()
            .map(|running| {
                running
                    .iter()
                    .filter(|(candidate, _)| candidate == company)
                    .count()
                    < STAFF_CAP_PER_COMPANY
            })
            .unwrap_or(false)
    }

    fn try_claim(&self, company: &str, actor: &str) -> Result<()> {
        let mut running = self
            .running
            .lock()
            .map_err(|_| anyhow::anyhow!("staff registry"))?;
        if running.contains(&(company.to_string(), actor.to_string())) {
            bail!("actor {actor} is already running");
        }
        let active = running.iter().filter(|(c, _)| c == company).count();
        if active >= STAFF_CAP_PER_COMPANY {
            bail!("staff cap ({STAFF_CAP_PER_COMPANY}) reached for {company}");
        }
        running.insert((company.to_string(), actor.to_string()));
        Ok(())
    }

    fn release(&self, company: &str, actor: &str) {
        if let Ok(mut running) = self.running.lock() {
            running.remove(&(company.to_string(), actor.to_string()));
        }
    }

    /// Whether this durable actor currently has a supervised process. The
    /// actor id is what OrgIntel owns; the registry's internal key remains the
    /// short staff name used for its worktree and process.
    pub fn is_actor_running(&self, company: &str, actor: &str) -> bool {
        self.running
            .lock()
            .map(|running| running.contains(&(company.to_string(), actor.to_string())))
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
                    .map(|(_, actor)| actor.clone())
                    .collect();
                actors.sort();
                actors
            })
            .unwrap_or_default()
    }
}

/// Repo and explicit worktree segments cross the Runtime boundary.
fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 32
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

async fn mail_exec(org: &restless_orgintel::OrgIntel, body: &str) {
    if let Err(error) = org.add_actor("daemon", "daemon", "The daemon").await {
        tracing::warn!("failed to ensure daemon actor: {error:#}");
        return;
    }
    if let Err(error) = org.send_message("daemon", Some("exec"), body).await {
        tracing::warn!(%error, body, "failed to mail exec");
    }
}

/// Start exactly one already-claimed Work Attempt. No other public function
/// can launch a Staff actor.
pub async fn dispatch_claimed_work(
    config: &CompanyConfig,
    spend: &SpendLedger,
    authority: &crate::authority::AuthorityStore,
    org: &restless_orgintel::OrgIntel,
    registry: &StaffRegistry,
    claimed: ClaimedWork,
) -> Result<()> {
    let actor = claimed.work.owner_id.clone();
    if actor == "owner" {
        bail!("{actor} is not a Staff execution actor");
    }
    if let Some(repo) = &claimed.work.repo {
        if !valid_slug(repo) {
            bail!("invalid repo name {repo:?}");
        }
    }
    let actors = org.list_actors().await?;
    let actor_row = actors
        .into_iter()
        .find(|row| row.id == actor)
        .with_context(|| format!("Work owner {actor:?} is not an OrgIntel actor"))?;
    let candidates =
        crate::model_gateway::available_candidates(config, actor_row.model.as_deref(), authority)
            .await?;
    let first_model = candidates
        .first()
        .context("staff model policy has no candidates")?;
    org.add_actor_with_model(
        &actor,
        &actor_row.kind,
        &actor_row.display,
        Some(first_model),
    )
    .await?;
    org.set_attempt_model(claimed.attempt_id, first_model)
        .await?;
    registry.try_claim(&config.name, &actor)?;

    let workdir = match if claimed.work.repo.is_some() {
        ensure_worktree(config, &claimed.work).await
    } else {
        Ok("/company".to_string())
    } {
        Ok(workdir) => workdir,
        Err(error) => {
            let note = format!("workspace setup failed before the process started: {error:#}");
            let _ = org
                .finish_work_attempt(claimed.attempt_id, WorkAttemptState::Failed, &note)
                .await;
            registry.release(&config.name, &actor);
            return Err(error);
        }
    };

    let spine = shared_spine(config, org, &actor).await;
    let company = config.name.clone();
    let name = actor_row.display;
    let task = format!(
        "# Work {} revision {} attempt {}\nAttempt UUID: {}\n{}\n\nExpected artifact: {}\nInput fingerprint: {}\nInputs:\n{}\n\nOwner/operator feedback through message {}:\n{}",
        claimed.work.id,
        claimed.work.revision,
        claimed.attempt_no,
        claimed.attempt_id,
        claimed.work.outcome,
        claimed.work.expected_artifact,
        claimed.input_fingerprint,
        claimed.inputs.iter().map(|input| format!("- {} [{}] {}", input.label, input.kind, input.uri)).collect::<Vec<_>>().join("\n"),
        claimed.feedback.last().map(|message| message.id).unwrap_or(0),
        claimed.feedback.iter().map(|message| format!("- {}: {}", message.from_actor, message.body)).collect::<Vec<_>>().join("\n")
    );
    let container = runtime::container_name(&config.name);
    let org = org.clone();
    let registry = registry.clone();
    let meter = spend.meter();
    let authority = authority.clone();
    let role = actor_row.kind;
    let attempt_id = claimed.attempt_id;
    let work_id = claimed.work.id;
    tokio::spawn(async move {
        let gate_container = container.clone();
        let outcome = run_staff_with_failover(StaffRun {
            container,
            workdir: workdir.clone(),
            company: company.clone(),
            actor: actor.clone(),
            name: name.clone(),
            task,
            role,
            spine,
            candidates,
            org: org.clone(),
            meter,
            authority,
            conversation: false,
        })
        .await;
        record_staff_outcome(
            &org,
            StaffAttemptContext {
                container: &gate_container,
                actor: &actor,
                name: &name,
                work_id,
                attempt_id,
                workdir: &workdir,
            },
            outcome,
        )
        .await;
        registry.release(&company, &actor);
    });
    Ok(())
}

/// Wake an accountable team lead for addressed conversation or judgement.
/// This is deliberately the same supervised actor process as Work execution,
/// without manufacturing a Work Attempt for conversation. The trigger is a
/// deterministic owed condition; the response and repair remain judgement.
pub async fn dispatch_actor_conversation(
    config: &CompanyConfig,
    spend: &SpendLedger,
    authority: &crate::authority::AuthorityStore,
    org: &restless_orgintel::OrgIntel,
    registry: &StaffRegistry,
    actor: &str,
    reason: &str,
) -> Result<bool> {
    if actor == "exec" || matches!(actor, "owner" | "world" | "daemon") {
        return Ok(false);
    }
    if spend.over_ceiling(config).is_some() || registry.is_actor_running(&config.name, actor) {
        return Ok(false);
    }

    let teams = org.list_teams().await?;
    let Some(team) = teams.iter().find(|team| team.lead_actor_id == actor) else {
        // Sprint 06 makes the lead the addressable team surface. Ordinary
        // members still receive exact Work feedback through the graph.
        return Ok(false);
    };
    let actors = org.list_actors().await?;
    let actor_row = actors
        .iter()
        .find(|row| row.id == actor)
        .with_context(|| format!("team lead {actor:?} is not an active actor"))?;
    let inbox = org.inbox(Some(actor)).await?;
    let mut addressed = Vec::new();
    for message in inbox {
        if !org.message_is_work_attempt_input(message.id).await? {
            addressed.push(message);
        }
    }
    let judgements = org.handoffs_assigned_to(actor).await?;
    if addressed.is_empty() && judgements.is_empty() {
        return Ok(false);
    }

    let members = actors
        .iter()
        .filter(|candidate| candidate.team_id == Some(team.id))
        .map(|candidate| {
            format!(
                "- {} · {}{} · model {}",
                candidate.id,
                candidate.kind,
                (candidate.id == team.lead_actor_id)
                    .then_some(" · accountable lead")
                    .unwrap_or(""),
                candidate.model.as_deref().unwrap_or("inherited")
            )
        })
        .collect::<Vec<_>>();
    let member_ids = actors
        .iter()
        .filter(|candidate| candidate.team_id == Some(team.id))
        .map(|candidate| candidate.id.as_str())
        .collect::<HashSet<_>>();
    let team_work_rows = org
        .list_work()
        .await?
        .into_iter()
        .filter(|work| member_ids.contains(work.owner_id.as_str()))
        .collect::<Vec<_>>();
    let team_work_ids = team_work_rows
        .iter()
        .map(|work| work.id)
        .collect::<HashSet<_>>();
    let team_work = team_work_rows
        .iter()
        .map(|work| {
            format!(
                "- {} rev {} [{:?}] {} · owner {} · Goal {} · {}",
                work.id,
                work.revision,
                work.status,
                work.title,
                work.owner_id,
                work.goal_id
                    .map(|goal| goal.to_string())
                    .unwrap_or_else(|| "unassigned".into()),
                work.resolution
            )
        })
        .collect::<Vec<_>>();
    let team_edges = org
        .work_graph_snapshot()
        .await?
        .edges
        .into_iter()
        .filter(|edge| {
            team_work_ids.contains(&edge.from_work_id) && team_work_ids.contains(&edge.to_work_id)
        })
        .map(|edge| {
            format!(
                "- {:?}: {} -> {}",
                edge.kind, edge.from_work_id, edge.to_work_id
            )
        })
        .collect::<Vec<_>>();
    let mail = addressed
        .iter()
        .map(|message| {
            let trust = if message.from_actor == "owner" {
                "owner direction — interpret and apply"
            } else {
                "internal coordination"
            };
            format!(
                "- message {} [{trust}] from {}: {}",
                message.id, message.from_actor, message.body
            )
        })
        .collect::<Vec<_>>();
    let owed = judgements
        .iter()
        .map(|handoff| {
            format!(
                "- handoff {} on Work {} from {}: {}\n  prepared: {}\n  resume when: {}",
                handoff.id,
                handoff.work_id,
                handoff.requested_by,
                handoff.requested_action,
                handoff.prepared_state,
                handoff.resume_condition
            )
        })
        .collect::<Vec<_>>();
    let message_ids = addressed
        .iter()
        .map(|message| message.id)
        .collect::<Vec<_>>();
    let task = format!(
        "# Team charter\n{}\n\n# Roster\n{}\n\n# Team Work\n{}\n\n# Team Work edges\n{}\n\n# Addressed messages\n{}\n\n# Judgement you owe\n{}\n\n\
         You are accountable for this team's outcome, not a relay. Resolve local blockers by changing the smallest relevant mechanism: roster, brief, context, skill, model, tool, dependency, or Work graph. The scheduler starts ready Work; do not narrate handoffs manually.\n\n\
         Assemble the smallest differentiated roster. Inspect `restless people` before adding anyone. If no active actor buys the missing capability, use `restless people create --id <stable-id> --role <role> --display <name> [--model <model>] --reason <difference>`; then `restless teams assign --actor <id> --team <this team> --reason <difference or repair>`. Reuse those actors across Work and revisions; never mint v2/v3 actors.\n\n\
         When creating dependent Work, declare every initial dependency in the same `restless work add` with repeatable `--requires <prerequisite-work-id>` and `--revises <producer-work-id>` flags. Those commit atomically. Use `restless work edge` only to repair an existing graph: for requires, `--from` is the prerequisite and `--to` is the dependent; revises runs reviewer to producer. Remove a mistaken local edge with `--remove --as {actor} --reason <evidence>`. Adding edges after node creation can let the scheduler start a half-built node.\n\n\
         Graph state outranks prose. Only completed final-acceptance Work makes acceptance canonical. Evidence and research Work are inputs to that judgement; a free-form status message may report them but must not declare the outcome accepted, canonical, or shipped while a dependency or final-acceptance Work remains open.\n\n\
         For a pending judgement you can settle, use `restless work resolve-handoff --handoff <id> --state resolved --resolution <answer>`. If it is genuinely outside the charter, use `restless work escalate-handoff --handoff <id> --as {actor} --reason <evidence and smallest decision>`; it goes to the Exec, not directly to the owner. Resume repaired failed Work with `restless work resume --work <id> --as {actor} --reason <what changed>`.\n\n\
         If the owner wrote, reply before ending with `restless message --from {actor} '<plain reply>'`. Speak for the whole team: what is moving, blocked, and what you changed. If the owner directed a change, make the Work graph change before claiming it did. End the reply with exactly one intent marker: `<!--restless-intent:{{\"kind\":\"conversation|work_feedback|direction|authority\",\"summary\":\"one short interpretation\"}}-->` using one real kind.\n\n\
         Ask the Exec only for cross-team resources, company priority, strategy, or charter guidance. Authority and irreducible human last miles remain owner boundaries.",
        team.brief.trim(),
        if members.is_empty() { "(none)".into() } else { members.join("\n") },
        if team_work.is_empty() { "(none)".into() } else { team_work.join("\n") },
        if team_edges.is_empty() { "(none)".into() } else { team_edges.join("\n") },
        if mail.is_empty() { "(none)".into() } else { mail.join("\n") },
        if owed.is_empty() { "(none)".into() } else { owed.join("\n") },
    );

    let candidates =
        crate::model_gateway::available_candidates(config, actor_row.model.as_deref(), authority)
            .await?;
    registry.try_claim(&config.name, actor)?;
    let company = config.name.clone();
    let actor = actor.to_string();
    let name = actor_row.display.clone();
    let role = actor_row.kind.clone();
    let org = org.clone();
    let registry = registry.clone();
    let meter = spend.meter();
    let authority = authority.clone();
    let container = runtime::container_name(&company);
    let spine = format!(
        "\n# The company you work for\n{}\n\n# Why you woke\n{}\n",
        config.mission.trim(),
        reason
    );
    tokio::spawn(async move {
        let outcome = run_staff_with_failover(StaffRun {
            container,
            workdir: "/company".into(),
            company: company.clone(),
            actor: actor.clone(),
            name: name.clone(),
            task,
            role,
            spine,
            candidates,
            org: org.clone(),
            meter,
            authority,
            conversation: true,
        })
        .await;
        match &outcome {
            Ok((termination, summary)) if *termination != Termination::Blocked => {
                for id in message_ids {
                    let _ = org.mark_read(id).await;
                }
                let _ = org
                    .emit_event(
                        "actor_wake_end",
                        Some(&actor),
                        serde_json::json!({ "termination": termination, "reason": summary }),
                    )
                    .await;
            }
            Ok((_, summary)) => {
                let reason =
                    format!("{name} could not complete its team coordination turn: {summary}");
                let _ = org.fallthrough_handoffs_to_exec(&actor, &reason).await;
                let _ = org.send_message(&actor, Some("exec"), &reason).await;
            }
            Err(error) => {
                let reason = format!("{name} coordination turn crashed: {error:#}");
                let _ = org.fallthrough_handoffs_to_exec(&actor, &reason).await;
                let _ = org.send_message(&actor, Some("exec"), &reason).await;
            }
        }
        registry.release(&company, &actor);
    });
    Ok(true)
}

/// What a worker needs to know about the company it works for, beyond its own
/// task: the mission, the plan the Exec is working to, and what else is open.
/// `docs/specs/orgintel.md` §5.2 — shared spine plus local depth.
async fn shared_spine(
    config: &CompanyConfig,
    org: &restless_orgintel::OrgIntel,
    actor: &str,
) -> String {
    let mut spine = format!("\n# The company you work for\n{}\n", config.mission.trim());
    match org.list_work().await {
        Ok(work) => {
            let open: Vec<String> = work
                .iter()
                .filter(|c| matches!(c.status, WorkStatus::Active | WorkStatus::Blocked))
                .map(|c| {
                    format!(
                        "- [{}] {} (owner: {})",
                        format!("{:?}", c.status).to_lowercase(),
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
        Err(error) => tracing::warn!(%error, "could not read Work graph for the staff spine"),
    }
    let coordinator = org
        .team_lead_for(actor)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "exec".to_string());
    spine.push_str(&format!(
        "\nYour accountable coordinator is {coordinator}. Use `restless message --to {coordinator} \"...\"` for blockers or free-form coordination. Use the Work CLI to link the exact artifact version you produced. An owner handoff is only for identity, CAPTCHA, MFA, legal attestation, payment confirmation, or irreducible owner judgement; ordinary uncertainty goes to {coordinator}.\n"
    ));
    spine
}

/// One claimed Work Attempt across all provider candidates.
struct StaffRun {
    container: String,
    workdir: String,
    company: String,
    actor: String,
    name: String,
    task: String,
    role: String,
    spine: String,
    candidates: Vec<String>,
    org: restless_orgintel::OrgIntel,
    meter: crate::spend::TurnMeter,
    authority: crate::authority::AuthorityStore,
    conversation: bool,
}

async fn run_staff_with_failover(run: StaffRun) -> Result<(Termination, String)> {
    let mut continuity_note: Option<String> = None;

    for (index, model) in run.candidates.iter().enumerate() {
        run.org
            .add_actor_with_model(&run.actor, &run.role, &run.name, Some(model.as_str()))
            .await?;
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
                return Ok((Termination::Blocked, blocked.message()));
            }
        };

        let billing = auth.billing;
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
            role: run.role.clone(),
            spine,
            conversation: run.conversation,
        })
        .await;

        let failure_kind = match &outcome {
            Ok((Termination::Blocked, reason, _)) => health::block_kind_from_message(reason)
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
        if let Ok((_, _, spent)) = &outcome {
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

        if let Some(kind) = failure_kind.filter(|kind| health::is_provider_failover_kind(*kind)) {
            let reason = match &outcome {
                Ok((_, reason, _)) => reason.clone(),
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
                Ok((_, reason, _)) => reason.clone(),
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
        return outcome.map(|(termination, reason, _)| (termination, reason));
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
fn record_final_staff_spend(
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
    /// What this actor IS. Reaches the agent's own briefing, so the
    /// specialisation is something it knows about itself rather than a label
    /// only the daemon can see.
    role: String,
    /// The shared spine, or empty in `minimal_team` and `single_agent`.
    spine: String,
    /// A lead/actor response turn has no claimed Work Attempt. It uses the
    /// same process, model, failover and supervision path with a team-scoped
    /// brief rather than inventing a second runtime class.
    conversation: bool,
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

/// Staff judge the bounded assignment they own, not the Exec's company-wide
/// milestone. Reusing the Exec envelope here made a critic who had written a
/// complete review report `blocked` merely because another critic or a later
/// revision still remained. That destroys the meaning of the Work
/// state and makes successful handoffs look like failures.
const STAFF_TERMINATION_PROMPT: &str =
    "Your assigned specialist task is ending now. Based on the task you were given, answer with JSON only, no prose:\n\
    {\"decision\": \"continue\" | \"blocked\" | \"changes_requested\" | \"outcome_met\" | \"abandon\", \
     \"reason\": \"<one line>\"}\n\
    - continue: more machine-doable work remains in your assigned task\n\
    - blocked: you cannot complete your assigned task until a human or external event acts; say exactly what is needed\n\
    - changes_requested: you are a reviewer and found concrete changes; this follows the Work graph's revises edge\n\
    - outcome_met: your assigned task and its requested outputs are complete\n\
    - abandon: your assigned task is not worth continuing; say why\n\
    Judge only your assignment. Other company work, later review, or another actor's task does not make your completed task blocked.";

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
        conversation,
    } = brief;
    let (container, auth, workdir, actor) =
        (container.as_str(), &auth, workdir.as_str(), actor.as_str());
    let assignment = if conversation {
        "woken for a bounded coordination conversation"
    } else {
        "assigned one claimed Work Attempt"
    };
    let workspace = if conversation {
        "Your working directory is /company. Use the existing company files and ordinary Restless CLI; do not create a second plan or workflow."
    } else {
        "Your working directory is {workdir} — it is YOURS: a dedicated git worktree. Commit meaningful checkpoints there with clear messages; do not touch other worktrees or the main checkout."
    };
    let prompt = format!(
        "You are {name}, the {role} of {company}, {assignment}.\n\
         You are a SPECIALIST, not a smaller Exec. Do the job your role names and \
         say so plainly when something falls outside it — a specialist who quietly \
         does everything is a generalist with a job title, and the reason you exist \
         is that one actor doing every job did one of them badly.\n\
         {workspace}\n\
         {spine}\n\
         # Task\n{task}\n\n\
         Work until the task is done or you are stuck. The session ends when you stop \
         writing; you will then be asked for a decision envelope.",
        workspace = workspace.replace("{workdir}", workdir),
    );
    const CONTINUE_PROMPT: &str =
        "Continue the task. If it is done or you are stuck, stop writing.";
    acp::with_agent(container, auth, workdir, actor, move |session| {
        Box::pin(async move {
            let mut next = prompt;
            // Each prompt yields another cumulative session snapshot. Keep the
            // observations for failure telemetry, then charge only the final
            // snapshot once when this provider attempt ends.
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
                    if health::block_kind_from_message(&blocked)
                        .is_some_and(health::is_provider_failover_kind)
                    {
                        return Ok((Termination::Blocked, blocked, spent));
                    }
                    bail!("staff turn: {blocked}");
                }

                let end = session
                    .prompt_live(STAFF_TERMINATION_PROMPT, |_| false)
                    .await;
                if let Some(usage) = end.usage() {
                    spent.push(usage);
                }
                if let Some(blocked) = staff_halt(&end) {
                    if health::block_kind_from_message(&blocked)
                        .is_some_and(health::is_provider_failover_kind)
                    {
                        return Ok((Termination::Blocked, blocked, spent));
                    }
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
                        // untouched until a critic ran on a second provider
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

/// Close the exact Attempt that launched this process. Completion is accepted
/// only after its declared artifact and deterministic gates are observed.
struct StaffAttemptContext<'a> {
    container: &'a str,
    actor: &'a str,
    name: &'a str,
    work_id: uuid::Uuid,
    attempt_id: uuid::Uuid,
    workdir: &'a str,
}

async fn record_staff_outcome(
    org: &restless_orgintel::OrgIntel,
    context: StaffAttemptContext<'_>,
    outcome: Result<(Termination, String)>,
) {
    let StaffAttemptContext {
        container,
        actor,
        name,
        work_id,
        attempt_id,
        workdir,
    } = context;
    let record = async {
        match outcome {
            Ok((Termination::OutcomeMet, summary)) => {
                finish_claimed_attempt(
                    org,
                    container,
                    work_id,
                    attempt_id,
                    Termination::OutcomeMet,
                    &summary,
                )
                .await?;
                if let Some(work) = org.get_work(work_id).await? {
                    if work.status == WorkStatus::Blocked {
                        let coordinator = org
                            .team_lead_for(&work.owner_id)
                            .await?
                            .unwrap_or_else(|| "exec".to_string());
                        org.send_message(
                            actor,
                            Some(&coordinator),
                            &format!(
                                "{name} could not pass completion gates for Work {work_id}: {}",
                                work.resolution
                            ),
                        )
                        .await?;
                    }
                }
            }
            Ok((Termination::ChangesRequested, summary)) => {
                finish_claimed_attempt(
                    org,
                    container,
                    work_id,
                    attempt_id,
                    Termination::ChangesRequested,
                    &summary,
                )
                .await?
            }
            Ok((Termination::Blocked, summary)) => {
                org.finish_work_attempt(attempt_id, WorkAttemptState::Blocked, &summary)
                    .await?;
                let coordinator = org
                    .team_lead_for(actor)
                    .await?
                    .unwrap_or_else(|| "exec".to_string());
                org.send_message(
                    actor,
                    Some(&coordinator),
                    &format!("{name} blocked on Work {work_id}: {summary}"),
                )
                .await?;
            }
            Ok((Termination::Abandon, summary)) => {
                org.finish_work_attempt(attempt_id, WorkAttemptState::Abandoned, &summary)
                    .await?;
            }
            Ok((Termination::Continue, summary)) => {
                org.finish_work_attempt(attempt_id, WorkAttemptState::Failed, &summary)
                    .await?;
            }
            Err(error) => {
                org.finish_work_attempt(
                    attempt_id,
                    WorkAttemptState::Failed,
                    &format!("crashed mid-turn: {error:#}"),
                )
                .await?;
                org.emit_event(
                    "staff_crash",
                    Some(actor),
                    serde_json::json!({ "error": format!("{error:#}"), "worktree": workdir }),
                )
                .await?;
                let coordinator = org
                    .team_lead_for(actor)
                    .await?
                    .unwrap_or_else(|| "exec".to_string());
                org.send_message(
                    actor,
                    Some(&coordinator),
                    &format!(
                        "{name} crashed mid-attempt on Work {work_id}; worktree preserved at {workdir}. Repair or reassign before resuming."
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

/// Apply one actor's structured result to its claimed Attempt. Shared by
/// Staff and Exec-owned Work so graph semantics do not fork by actor kind.
pub async fn finish_claimed_attempt(
    org: &restless_orgintel::OrgIntel,
    container: &str,
    work_id: uuid::Uuid,
    attempt_id: uuid::Uuid,
    termination: Termination,
    summary: &str,
) -> Result<()> {
    match termination {
        Termination::OutcomeMet => {
            let work = org
                .get_work(work_id)
                .await?
                .context("claimed Work disappeared")?;
            let artifacts = org.list_artifact_refs(Some(work_id)).await?;
            let observed = work.expected_artifact.trim().is_empty()
                || artifacts.iter().any(|artifact| {
                    artifact.attempt_id == Some(attempt_id)
                        && artifact.state == restless_orgintel::ArtifactRefState::Available
                });
            if !observed {
                org.finish_work_attempt(
                    attempt_id,
                    WorkAttemptState::Failed,
                    &format!(
                        "declared complete without linking expected artifact: {}",
                        work.expected_artifact
                    ),
                )
                .await?;
            } else if run_gates(org, container, work_id, attempt_id).await? {
                org.finish_work_attempt(attempt_id, WorkAttemptState::Produced, summary)
                    .await?;
            } else {
                org.finish_work_attempt(
                    attempt_id,
                    WorkAttemptState::Failed,
                    "one or more deterministic Work gates failed",
                )
                .await?;
            }
        }
        Termination::ChangesRequested => {
            org.finish_work_attempt(attempt_id, WorkAttemptState::ChangesRequested, summary)
                .await?;
        }
        Termination::Blocked => {
            org.finish_work_attempt(attempt_id, WorkAttemptState::Blocked, summary)
                .await?;
        }
        Termination::Abandon => {
            org.finish_work_attempt(attempt_id, WorkAttemptState::Abandoned, summary)
                .await?;
        }
        Termination::Continue => {
            org.finish_work_attempt(attempt_id, WorkAttemptState::Failed, summary)
                .await?;
        }
    }
    Ok(())
}

async fn run_gates(
    org: &restless_orgintel::OrgIntel,
    container: &str,
    work_id: uuid::Uuid,
    attempt_id: uuid::Uuid,
) -> Result<bool> {
    let gates = org.list_work_gates(work_id).await?;
    for gate in gates {
        let argv: Vec<String> = serde_json::from_value(gate.command.clone())
            .with_context(|| format!("gate {} has invalid argv", gate.name))?;
        let (program, args) = argv.split_first().context("gate command is empty")?;
        let output = tokio::process::Command::new("docker")
            .args(["exec", "-u", "company", "-w", &gate.cwd, container, program])
            .args(args)
            .output()
            .await
            .with_context(|| format!("run gate {}", gate.name))?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let digest = format!("{:x}", Sha256::digest(combined.as_bytes()));
        org.record_gate_run(restless_orgintel::NewGateRun {
            gate_id: gate.id,
            attempt_id,
            exit_code: output.status.code(),
            output_digest: &digest,
            output_excerpt: &combined.chars().take(2_000).collect::<String>(),
            passed: output.status.success(),
        })
        .await?;
    }
    Ok(org.gates_passed(work_id, attempt_id).await?)
}

/// Create or reuse the workspace recorded on Work. Git remains the source of
/// file truth; OrgIntel stores only the path and exact artifact versions.
async fn ensure_worktree(
    config: &CompanyConfig,
    work: &restless_orgintel::WorkRow,
) -> Result<String> {
    let container = runtime::container_name(&config.name);
    let repo = work.repo.as_deref().context("Work repo is missing")?;
    let short = work.id.simple().to_string();
    let generated = format!("work-{}-r{}", &short[..12], work.revision);
    let leaf = work.worktree.as_deref().unwrap_or(&generated);
    if !valid_slug(leaf) {
        bail!("invalid Work worktree {leaf:?}");
    }
    let path = format!("/company/worktrees/{leaf}");
    let exists = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            &container,
            "test",
            "-f",
            &format!("{path}/.git"),
        ])
        .status()
        .await
        .context("probe Work worktree")?;
    if exists.success() {
        return Ok(path);
    }
    let mkdir = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            &container,
            "mkdir",
            "-p",
            "/company/worktrees",
        ])
        .output()
        .await
        .context("create worktree directory")?;
    if !mkdir.status.success() {
        bail!(
            "worktree directory failed: {}",
            String::from_utf8_lossy(&mkdir.stderr)
        );
    }
    let branch = format!("work/{leaf}");
    let mut command = tokio::process::Command::new("docker");
    command.args([
        "exec",
        "-u",
        "company",
        &container,
        "git",
        "-C",
        &format!("/company/repos/{repo}"),
        "worktree",
        "add",
        &path,
        "-b",
        &branch,
    ]);
    if let Some(base) = work.base_ref.as_deref() {
        command.arg(base);
    }
    let output = command.output().await.context("create Work worktree")?;
    if !output.status.success() {
        let reuse = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-u",
                "company",
                &container,
                "git",
                "-C",
                &format!("/company/repos/{repo}"),
                "worktree",
                "add",
                &path,
                &branch,
            ])
            .output()
            .await
            .context("reuse Work branch")?;
        if !reuse.status.success() {
            bail!(
                "worktree setup failed: {}",
                String::from_utf8_lossy(&reuse.stderr)
            );
        }
    }
    Ok(path)
}

/// Boot sweep: any marked ACP session still running in a company container
/// when the daemon starts is an orphan of the last daemon's lifetime —
/// unsupervised, its transcript unreachable. Reap only those Linux sessions,
/// mark running Attempts failed with the workspace preserved.
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
        if reaped > 0 {
            tracing::warn!(
                company = name,
                reaped,
                "reaped marked agent processes from before restart"
            );
        }
        let Ok(org) = orgintel.get(name).await else {
            continue;
        };
        let Ok(attempts) = org.list_work_attempts(None).await else {
            continue;
        };
        for attempt in attempts
            .iter()
            .filter(|attempt| attempt.state == WorkAttemptState::Running)
        {
            let note = "supervisor restarted; process lost, worktree preserved";
            if let Err(error) = org
                .finish_work_attempt(attempt.id, WorkAttemptState::Failed, note)
                .await
            {
                tracing::warn!("failed to close orphaned Work Attempt: {error:#}");
                continue;
            }
            let mail = format!(
                "actor {} was mid-attempt when the supervisor restarted; its workspace is intact. The accountable lead must inspect the preserved state, change the failed mechanism, and explicitly resume or reassign the blocked Work.",
                attempt.actor_id
            );
            mail_exec(&org, &mail).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::record_final_staff_spend;
    use crate::acp::TurnUsage;
    use crate::model_gateway::ModelBilling;
    use crate::spend::SpendLedger;

    #[test]
    fn cumulative_acp_snapshots_charge_one_final_session_total() {
        // Reduced from the live lead turn: OMP reported a new cumulative
        // session snapshot after each re-prompt. Summing these would charge
        // $6.34; the provider's final cumulative total is $2.38.
        let snapshots = [
            TurnUsage {
                used: 35_873,
                size: 262_144,
                cost_usd: Some(0.47),
            },
            TurnUsage {
                used: 51_204,
                size: 262_144,
                cost_usd: Some(1.11),
            },
            TurnUsage {
                used: 64_814,
                size: 262_144,
                cost_usd: Some(2.38),
            },
            // A repeated stream snapshot is still the same cumulative bill.
            TurnUsage {
                used: 64_814,
                size: 262_144,
                cost_usd: Some(2.38),
            },
        ];
        let root = std::env::temp_dir().join(format!(
            "restless-cumulative-usage-test-{}",
            uuid::Uuid::new_v4()
        ));
        assert!(!root.exists(), "test spend root must be fresh");
        let ledger = SpendLedger::open(&root).unwrap();
        let meter = ledger.meter();

        let (usage, charged) = record_final_staff_spend(
            &meter,
            "usage_test",
            "validation-lead",
            "moonshot/kimi-k3",
            ModelBilling::MeteredApi,
            &snapshots,
            None,
        )
        .expect("one final cumulative usage snapshot");

        assert_eq!(usage.used, 64_814, "context usage is the final snapshot");
        assert_eq!(charged, Some(2.38));
        assert!(
            (ledger.spent_usd("usage_test") - 2.38).abs() < f64::EPSILON,
            "repeated cumulative snapshots must not be summed"
        );
        assert_eq!(
            ledger.breakdown("usage_test"),
            vec![(
                "validation-lead".to_string(),
                "moonshot/kimi-k3".to_string(),
                2.38,
            )]
        );

        drop(meter);
        drop(ledger);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
