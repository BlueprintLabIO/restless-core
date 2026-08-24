//! Addressed lead conversations and completed-Attempt review preparation.
//!
//! Conversation stays an OrgIntel/message concern. It may inspect a detached
//! Runtime review copy, but it does not manufacture a Work Attempt or rewrite
//! the source checkout.

use std::collections::HashSet;

use anyhow::{Context as _, Result};
use restless_orgintel::{MessageRow, WorkAttemptState, WorkStatus};

use crate::conversation::ConversationStreams;
use crate::exec::Termination;
use crate::runtime::{self, CompanyConfig};
use crate::spend::SpendLedger;

use super::execution::{run_staff_with_failover, StaffRun};
use super::workspace::prepare_review_copy;
use super::StaffRegistry;

/// Wake an accountable team lead for addressed conversation or judgement.
/// This is deliberately the same supervised actor process as Work execution,
/// without manufacturing a Work Attempt for conversation. The trigger is a
/// deterministic owed condition; the response and repair remain judgement.
pub struct ConversationRuntime<'a> {
    pub spend: &'a SpendLedger,
    pub authority: &'a crate::authority::AuthorityStore,
    pub capabilities: &'a crate::capability::CapabilityIssuer,
    pub registry: &'a StaffRegistry,
    pub streams: &'a ConversationStreams,
}

struct ConversationWorkspace {
    workdir: String,
    review_context: String,
}

fn unavailable_review_workspace(reason: impl std::fmt::Display) -> ConversationWorkspace {
    ConversationWorkspace {
        workdir: "/company".into(),
        review_context: format!(
            "# Completed Attempt review target\nA completed Attempt was linked to this owner conversation, but its exact review copy is unavailable: {reason}. Do not use a completed source worktree as scratch space and do not repair it in this coordination wake. Inspect existing linked native evidence only; if the outcome needs repair, create attributable revision Work."
        ),
    }
}

/// Select the current completed Attempt's recorded source version and prepare
/// one detached Runtime worktree for a lead's owner-linked review. The copy is
/// an ordinary supporting artifact reference, never a replacement candidate
/// or a durable review state machine.
async fn completed_attempt_review_workspace(
    org: &restless_orgintel::OrgIntel,
    container: &str,
    work_id: Option<uuid::Uuid>,
) -> ConversationWorkspace {
    let Some(work_id) = work_id else {
        return ConversationWorkspace {
            workdir: "/company".into(),
            review_context: String::new(),
        };
    };
    let work = match org.get_work(work_id).await {
        Ok(Some(work)) => work,
        Ok(None) => return unavailable_review_workspace("the linked Work no longer exists"),
        Err(error) => {
            return unavailable_review_workspace(format!("could not read Work: {error:#}"))
        }
    };
    if work.status != WorkStatus::Completed {
        return ConversationWorkspace {
            workdir: "/company".into(),
            review_context: String::new(),
        };
    }
    if work.repo.is_none() {
        return unavailable_review_workspace(
            "the completed Work has no repository-bound source; its existing native artifact remains the review target",
        );
    }

    let attempts = match org.list_work_attempts(Some(work.id)).await {
        Ok(attempts) => attempts,
        Err(error) => {
            return unavailable_review_workspace(format!("could not read Work Attempts: {error:#}"))
        }
    };
    let Some(attempt) = attempts.iter().rev().find(|attempt| {
        attempt.revision == work.revision && attempt.state == WorkAttemptState::Produced
    }) else {
        return unavailable_review_workspace(
            "the completed Work has no produced Attempt at its current revision",
        );
    };
    let terminal_commit = match org
        .find_event_body(
            "attempt_process_ended",
            "attempt_id",
            &attempt.id.to_string(),
        )
        .await
    {
        Ok(Some(event)) => event
            .pointer("/workspace/end/source_commit")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        Ok(None) => None,
        Err(error) => {
            return unavailable_review_workspace(format!(
                "could not read the Attempt terminal observation: {error:#}"
            ))
        }
    };
    let artifacts = match org.list_artifact_refs(Some(work.id)).await {
        Ok(artifacts) => artifacts,
        Err(error) => {
            return unavailable_review_workspace(format!("could not read Work evidence: {error:#}"))
        }
    };
    let source_commit = terminal_commit.or_else(|| {
        artifacts
            .iter()
            .rev()
            .find(|artifact| artifact.attempt_id == Some(attempt.id))
            .and_then(|artifact| artifact.source_commit.clone())
    });
    let Some(source_commit) = source_commit else {
        return unavailable_review_workspace(
            "the completed Attempt has no exact recorded source commit",
        );
    };

    let prepared = match prepare_review_copy(container, &work, attempt.id, &source_commit).await {
        Ok(prepared) => prepared,
        Err(error) => return unavailable_review_workspace(error),
    };
    let is_linked = artifacts.iter().any(|artifact| {
        artifact.kind == "review_copy"
            && artifact.uri == prepared.workdir
            && artifact.attempt_id == Some(attempt.id)
            && artifact.source_commit.as_deref() == Some(prepared.source_commit.as_str())
            && artifact.state == restless_orgintel::ArtifactRefState::Available
    });
    if !is_linked {
        let note = "Supporting review copy prepared by the Runtime from the exact completed Attempt commit; it is not a replacement candidate.";
        let digest = prepared.source_after.fingerprint();
        if let Err(error) = org
            .link_work_artifact(restless_orgintel::NewArtifactRef {
                kind: "review_copy",
                uri: &prepared.workdir,
                note,
                // This reference names the source Attempt's accountable actor,
                // rather than inventing a second Runtime producer identity.
                created_by: &attempt.actor_id,
                work_id: Some(work.id),
                attempt_id: Some(attempt.id),
                digest: digest.as_deref(),
                source_commit: Some(&prepared.source_commit),
                runtime_generation: None,
                label: "Supporting review copy (not candidate)",
            })
            .await
        {
            return unavailable_review_workspace(format!(
                "the detached copy could not be tied to its Attempt evidence: {error:#}"
            ));
        }
    }
    if let Err(error) = org
        .emit_event(
            "review_evidence_prepared",
            Some(&attempt.actor_id),
            serde_json::json!({
                "work_id": work.id,
                "attempt_id": attempt.id,
                "source_workdir": prepared.source_before.workdir,
                "source_commit": prepared.source_commit,
                "review_workdir": prepared.workdir,
                "source_changed_during_preparation": prepared.source_before != prepared.source_after,
            }),
        )
        .await
    {
        tracing::warn!(%error, attempt_id = %attempt.id, "review copy is linked but its compactable preparation event was not recorded");
    }
    ConversationWorkspace {
        workdir: prepared.workdir.clone(),
        review_context: format!(
            "# Prepared supporting review evidence\nYour working directory is `{}`: a detached, clean review copy prepared from completed Attempt {} at recorded commit {}. The source Attempt checkout remains authoritative and was observed unchanged during preparation. You may run bounded executable inspection and place review-only supporting output here, but do not edit candidate/project files, commit, publish, or present this copy as a replacement candidate. If inspection finds a defect, create attributable revision Work instead.",
            prepared.workdir,
            attempt.id,
            prepared.source_commit,
        ),
    }
}

pub async fn dispatch_actor_conversation(
    config: &CompanyConfig,
    org: &restless_orgintel::OrgIntel,
    runtime: ConversationRuntime<'_>,
    actor: &str,
    reason: &str,
) -> Result<bool> {
    if actor == "exec" || matches!(actor, "owner" | "world" | "daemon") {
        return Ok(false);
    }
    if runtime.spend.over_ceiling(config).is_some()
        || runtime.registry.is_actor_running(&config.name, actor)
    {
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
                if candidate.id == team.lead_actor_id {
                    " · accountable lead"
                } else {
                    ""
                },
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
    let mut mail = Vec::new();
    for message in addressed
        .iter()
        .filter(|message| message.from_actor != "owner")
    {
        mail.push(internal_message_context(
            message,
            org.message_work_id(message.id).await?,
        ));
    }
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
    let owner_message_ids = addressed
        .iter()
        .filter(|message| message.from_actor == "owner")
        .map(|message| message.id)
        .collect::<Vec<_>>();
    let owner_input = addressed
        .iter()
        .filter(|message| message.from_actor == "owner")
        .map(|message| format!("- owner message {}: {}", message.id, message.body))
        .collect::<Vec<_>>();
    let reply_work_id = match owner_message_ids.last() {
        Some(message_id) => org.message_work_id(*message_id).await?,
        None => None,
    };
    let task = format!(
        "# Team charter\n{}\n\n# Roster\n{}\n\n# Team Work\n{}\n\n# Team Work edges\n{}\n\n# Addressed internal messages\n{}\n\n# Judgement you owe\n{}\n\n\
         Resolve local blockers by changing the smallest relevant mechanism: roster, brief, context, skill, model, tool, dependency, or Work graph. The scheduler starts ready Work; do not narrate handoffs manually.\n\n\
         The roster is available capacity, not a headcount target. Inspect `restless people` before adding anyone. New Staff is one possible sourcing posture, not the automatic answer to a missing capability. If evidence calls for new internal capacity, use `restless people create --id <durable-domain>-<craft> --role <role> --display <colleague-name> [--model <model>] --reason <difference>`; then `restless teams assign --actor <id> --team <this team> --reason <difference or repair>`. Reuse those actors across Work and revisions; never encode Staff, team position, environment, stage, implementation or retry in the id.\n\n\
         # Sourcing a missing capability [shared skill]\n{}\n\n\
         When creating dependent Work, declare every initial dependency in the same `restless work add` with repeatable `--requires <prerequisite-work-id>` and `--revises <producer-work-id>` flags. Those commit atomically. Use `restless work edge` only to repair an existing graph: for requires, `--from` is the prerequisite and `--to` is the dependent; revises runs reviewer to producer. Remove a mistaken local edge with `--remove --as {actor} --reason <evidence>`. Adding edges after node creation can let the scheduler start a half-built node.\n\n\
         Keep Work sparse and factual. Your current lead-owned Work already carries the whole charter; do not mirror your own plan or checklist as child nodes. Add child Work only when another actor will own a real bounded responsibility. Work and artifacts prove what crossed actors, while whole-outcome acceptance remains your judgement after native inspection. Never claim a Staff contribution that has no Work → Attempt → observed result.\n\n\
         For a pending judgement you can settle, use `restless work resolve-handoff --handoff <id> --state resolved --resolution <answer>`. If it is genuinely outside the charter, use `restless work escalate-handoff --handoff <id> --as {actor} --reason <evidence and smallest decision>`; it goes to the Exec, not directly to the owner. Resume repaired failed Work with `restless work resume --work <id> --as {actor} --reason <what changed>`.\n\n\
         If the owner wrote, your final assistant response is the reply the owner will receive. Do not use `restless message` to reply to the owner. Speak for the whole team. If the owner directed a change, make the Work graph change before claiming it did. Follow the shared conversation contract below and end with exactly one intent marker: `<!--restless-intent:{{\"kind\":\"conversation|work_feedback|direction|authority\",\"summary\":\"one short interpretation\"}}-->` using one real kind.\n\n\
         Ask the Exec only for cross-team resources, company priority, strategy, or charter guidance. Authority and irreducible human last miles remain owner boundaries.\n\n# Presenting to the owner [shared skill]\n{}\n\n# Conversing with the owner [shared contract]\n{}",
        team.brief.trim(),
        if members.is_empty() { "(none)".into() } else { members.join("\n") },
        if team_work.is_empty() { "(none)".into() } else { team_work.join("\n") },
        if team_edges.is_empty() { "(none)".into() } else { team_edges.join("\n") },
        if mail.is_empty() { "(none)".into() } else { mail.join("\n") },
        if owed.is_empty() { "(none)".into() } else { owed.join("\n") },
        crate::capability_sourcing::SOURCE_CAPABILITY.trim(),
        crate::owner_brief::PRESENT_TO_OWNER.trim(),
        crate::owner_brief::CONVERSE_WITH_OWNER.trim(),
    );
    let turn_prompt = conversation_turn_prompt(reason, &owner_input);

    let candidates = crate::model_gateway::available_candidates(
        config,
        actor_row.model.as_deref(),
        runtime.authority,
    )
    .await?;
    let container = runtime::container_name(&config.name);
    let conversation_workspace =
        completed_attempt_review_workspace(org, &container, reply_work_id).await;
    runtime.registry.try_claim(&config.name, actor)?;
    let company = config.name.clone();
    let actor = actor.to_string();
    let name = actor_row.display.clone();
    let role = actor_row.role.clone();
    let org = org.clone();
    let registry = runtime.registry.clone();
    let spend = runtime.spend.clone();
    let spend_ceiling = config.spend_ceiling_usd;
    let authority = runtime.authority.clone();
    let capabilities = runtime.capabilities.clone();
    let spine = format!(
        "\n# The company you work for\n{}\n\n# Why you woke\n{}\n{}\n",
        config.mission.trim(),
        reason,
        conversation_workspace.review_context,
    );
    let live_turn = runtime.streams.start(&company, &actor, &owner_message_ids);
    let observer = (!owner_message_ids.is_empty()).then(|| live_turn.observer());
    tokio::spawn(async move {
        let outcome = run_staff_with_failover(StaffRun {
            container,
            workdir: conversation_workspace.workdir,
            company: company.clone(),
            actor: actor.clone(),
            name: name.clone(),
            task,
            turn_prompt,
            role,
            spine,
            candidates,
            org: org.clone(),
            spend,
            spend_ceiling,
            authority,
            capabilities,
            conversation: true,
            accountable_lead: true,
            observer,
        })
        .await;
        match &outcome {
            Ok(outcome) if outcome.termination != Termination::Blocked => {
                let recorded = if owner_message_ids.is_empty() {
                    Ok(None)
                } else if let Some(work_id) = reply_work_id {
                    org.send_work_message_to_owner(&actor, work_id, &outcome.summary)
                        .await
                        .map(Some)
                } else {
                    org.send_message(&actor, None, &outcome.summary)
                        .await
                        .map(Some)
                };
                match recorded {
                    Ok(recorded_message_id) => {
                        for id in &message_ids {
                            let _ = org.mark_read(*id).await;
                        }
                        if let Some(message_id) = recorded_message_id {
                            live_turn.complete(message_id, outcome.output_tokens);
                        }
                    }
                    Err(error) => {
                        live_turn.fail(&format!("could not record the reply: {error:#}"));
                    }
                }
                let _ = org
                    .emit_event(
                        "actor_wake_end",
                        Some(&actor),
                        serde_json::json!({ "termination": outcome.termination, "reason": outcome.summary }),
                    )
                    .await;
            }
            Ok(outcome) => {
                live_turn.fail(&outcome.summary);
                let reason = format!(
                    "{name} could not complete its team coordination turn: {}",
                    outcome.summary
                );
                // A temporarily unavailable lead does not turn direct team
                // mail into an Exec relay. The addressed message stays owed
                // to this actor and the scheduler can retry it when the
                // model/runtime path recovers. Only an already-pending
                // judgement is allowed to fall through, because that is an
                // explicit authority assignment rather than a narration of
                // the failed turn.
                let _ = org.fallthrough_handoffs_to_exec(&actor, &reason).await;
            }
            Err(error) => {
                let reason = format!("{name} coordination turn crashed: {error:#}");
                live_turn.fail(&reason);
                // See the blocked branch above: preserve the direct
                // recipient and only move a concrete pending judgement.
                let _ = org.fallthrough_handoffs_to_exec(&actor, &reason).await;
            }
        }
        registry.release(&company, &actor);
    });
    Ok(true)
}

const COORDINATION_EXECUTION_BOUNDARY: &str = concat!(
    "This is an accountable-lead coordination wake, not a claimed productive Work Attempt. Inspect ",
    "company state and existing evidence to make the smallest factual decision. Use ordinary Restless ",
    "CLI only to update the actor, team, Work, handoff, or direct-message graph. Do not edit project or ",
    "repository files, create or modify a candidate artifact, run a productive repair, build, or make a ",
    "Git commit in this wake. A system-context prepared review copy may receive bounded executable ",
    "inspection and supporting review output only; it never replaces the source candidate or authorises an ",
    "external effect. If a Work is blocked, use its observed evidence to ",
    "revise, resume, abandon, or prepare the exact owner judgement only after an attributable Work has ",
    "changed the mechanism; never make a hidden repair yourself. Any product file, artifact, test ",
    "output, or commit created directly by this wake is not attributable and must not be presented as ",
    "one. Do not use Exec as a status relay; contact Exec only for a genuine cross-team, portfolio, ",
    "resource, or charter question."
);

const INTERNAL_MESSAGE_BOUNDARY: &str = concat!(
    "There is no owner input in this wake. The addressed facts above are already the relevant ",
    "message context: do not invent `restless message list` or `restless message history` commands. ",
    "Never run `restless message` without `--to` here: omitting the recipient sends a message to ",
    "the owner and is not an inspection command. Send a direct message only with `--to <actor>` ",
    "when changed information affects that colleague's Work; do not send acknowledgement or status ",
    "theatre."
);

pub(super) fn conversation_turn_prompt(reason: &str, owner_input: &[String]) -> String {
    if owner_input.is_empty() {
        format!(
            "# This wake\n{reason}\n\n# Coordination execution boundary [invariant]\n{COORDINATION_EXECUTION_BOUNDARY}\n\n# Internal-message boundary\n{INTERNAL_MESSAGE_BOUNDARY}\n\nResolve the addressed coordination or judgement in your system context. Work until the bounded team-lead turn is done or genuinely blocked."
        )
    } else {
        format!(
            "# This wake\n{reason}\n\n# Coordination execution boundary [invariant]\n{COORDINATION_EXECUTION_BOUNDARY}\n\n# Owner input [authoritative in source; interpret before applying]\n{}\n\nAddress the owner input using the team context and conversation contract in your system prompt.",
            owner_input.join("\n")
        )
    }
}

/// A Work-linked coordination wake needs the ordinary message's exact scope,
/// not a new handoff type. Naming the existing id lets a lead return changed
/// information through the same Work feedback path, where a later Attempt can
/// bind it immutably if the live actor did not observe it in time.
pub(super) fn internal_message_context(
    message: &MessageRow,
    work_id: Option<uuid::Uuid>,
) -> String {
    match work_id {
        Some(work_id) => format!(
            "- Work {work_id}, message {} [internal Work feedback] from {}: {}\n  If your response changes this Work, send exactly one direct Work-linked reply with `restless message --work {work_id} --to {} \"<decision>\"`. Do not send an unlinked acknowledgement, status, or command fragment.",
            message.id, message.from_actor, message.body, message.from_actor
        ),
        None => format!(
            "- message {} [internal coordination] from {}: {}",
            message.id, message.from_actor, message.body
        ),
    }
}
