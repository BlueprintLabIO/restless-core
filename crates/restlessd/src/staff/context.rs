//! Stable staff context assembly and workspace instructions.
//!
//! The functions here package current OrgIntel and Runtime facts for a turn;
//! they do not create Work, sessions, or a second source of coordination.

use restless_orgintel::{ActorRow, ClaimedWork, TeamRow, WorkStatus};

use crate::runtime::CompanyConfig;

/// A durable actor keeps one organisational posture across every wake. Before
/// this distinction, a lead's conversation wake called it accountable while
/// its productive Work wake called the same actor a mere specialist.
pub(super) fn actor_posture(accountable_lead: bool) -> &'static str {
    if accountable_lead {
        "You are the ACCOUNTABLE LEAD for this team's whole outcome, not a relay and not a smaller Exec. Apply the natural accountable-team rules above on productive Work and conversation wakes alike. Direct execution is valid; Staff are optional. You retain integration, native review, completion judgement, and truthful attribution of every real contribution."
    } else {
        "You are a SPECIALIST, not a smaller Exec. Own the bounded responsibility your role names, surface material contradictions early, and say plainly when something falls outside it. Do not quietly take over the whole team outcome: a specialist who does every job is a generalist with a job title."
    }
}

/// The lead's local capacity is authoritative state, not an inference from a
/// role name. It is intentionally small: a team charter and its current
/// roster let a lead decide whether a genuine seam exists without replaying a
/// company directory or inventing a headcount target.
pub(super) fn team_capacity_context(team: &TeamRow, actors: &[ActorRow]) -> String {
    let roster = actors
        .iter()
        .filter(|candidate| candidate.team_id == Some(team.id))
        .map(|candidate| {
            format!(
                "- {} · {}{}",
                candidate.display,
                candidate.role,
                if candidate.id == team.lead_actor_id {
                    " · accountable lead"
                } else {
                    ""
                },
            )
        })
        .collect::<Vec<_>>();
    format!(
        "\n# Your available team [internal decision]\n{} — {}\n{}\n\
         This is available capacity, not a headcount target. Work alone when no colleague can own \
         a stable, independently useful responsibility. If you do commission a real contributor, \
         create its bounded Work with `restless work add` before it starts; messages are not \
         assignments.\n",
        team.name,
        team.brief,
        if roster.is_empty() {
            "(no active roster)".to_string()
        } else {
            roster.join("\n")
        }
    )
}

/// What a worker needs to know about the company it works for, beyond its own
/// task: the mission, the plan the Exec is working to, and what else is open.
/// `docs/specs/orgintel.md` §5.2 — shared spine plus local depth.
pub(super) async fn shared_spine(
    config: &CompanyConfig,
    org: &restless_orgintel::OrgIntel,
    actor: &str,
    accountable_lead: bool,
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
    if accountable_lead {
        match (org.list_teams().await, org.list_actors().await) {
            (Ok(teams), Ok(actors)) => {
                if let Some(team) = teams.iter().find(|team| team.lead_actor_id == actor) {
                    spine.push_str(&team_capacity_context(team, &actors));
                }
            }
            (Err(error), _) | (_, Err(error)) => {
                tracing::warn!(%error, "could not read accountable lead team capacity")
            }
        }
    }
    if actor == "exec" {
        spine.push_str(
            "\nYou are the accountable coordinator for this Work; do not message yourself. Use the Work CLI to link the exact artifact version you produced. An owner handoff is only for identity, CAPTCHA, MFA, legal attestation, payment confirmation, or irreducible owner judgement.\n",
        );
    } else if accountable_lead {
        spine.push_str(
            "\nYou are the accountable coordinator for this team's outcome. Resolve ordinary uncertainty and local blockers inside the charter; message Exec only for cross-team resources, company priority, strategy, charter scope, or authority escalation. Use the Work CLI to make every real cross-actor contribution and its exact artifact observable.\n",
        );
    } else {
        let coordinator = org
            .team_lead_for(actor)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "exec".to_string());
        spine.push_str(&format!(
            "\nYour accountable coordinator is {coordinator}. Use `restless message --to {coordinator} \"...\"` for blockers or free-form coordination. Use the Work CLI to link the exact artifact version you produced. An owner handoff is only for identity, CAPTCHA, MFA, legal attestation, payment confirmation, or irreducible owner judgement; ordinary uncertainty goes to {coordinator}.\n"
        ));
    }
    spine
}

/// Assemble the factual membrane for an already-claimed Attempt. This is not
/// a commission template: a lead's purpose, understanding, unknowns, and
/// proof language remain the free-form Work outcome. The bridge adds only
/// facts it can name truthfully from the claim and the persistent Runtime.
///
/// Keep the accounting in the compactable event stream. The Work/Attempt rows
/// remain the source of the immutable inputs; a separate context table would
/// only mirror them and turn launch packaging into another lifecycle.
pub(super) fn bound_attempt_context(
    claimed: &ClaimedWork,
    role: &str,
    workdir: &str,
    company: &str,
    accountable_lead: bool,
) -> (String, serde_json::Value) {
    let workspace = serde_json::json!({
        "runtime_workdir": workdir,
        "repository": claimed.work.repo.clone(),
        "base_ref": claimed.work.base_ref.clone(),
        "integration_branch": claimed.work.integration_branch.clone(),
        "declared_worktree": claimed.work.worktree.clone(),
    });
    let inputs = claimed
        .inputs
        .iter()
        .map(|input| {
            serde_json::json!({
                "artifact_ref_id": input.id,
                "label": input.label.clone(),
                "kind": input.kind.clone(),
                "uri": input.uri.clone(),
                "digest": input.digest.clone(),
                "source_commit": input.source_commit.clone(),
                "runtime_generation": input.runtime_generation.clone(),
            })
        })
        .collect::<Vec<_>>();
    let feedback = claimed
        .feedback
        .iter()
        .map(|message| {
            serde_json::json!({
                "message_id": message.id,
                "from_actor": message.from_actor.clone(),
            })
        })
        .collect::<Vec<_>>();
    let skill_roots = vec![
        "/opt/restless/skills".to_string(),
        "/company/skills".to_string(),
        format!("{workdir}/.agents/skills"),
    ];
    let capability_probes = vec![
        "command -v restless git omp node pnpm".to_string(),
        format!("restless doctor -c {company}"),
        "restless credential check (configured references only; not provider acceptance or authority)"
            .to_string(),
        format!("test -d {workdir}/.agents/skills /company/skills /opt/restless/skills"),
    ];
    let mut system_context = serde_json::json!({
        "company_doctrine": "Restless shared operating rules in the actor system prompt",
        "company_mission": "company mission and collision-avoidance spine in the actor system prompt",
    });
    if accountable_lead {
        system_context["active_team_capacity"] =
            serde_json::json!("current team charter and roster when the accountable lead has one");
    }
    let accounting = serde_json::json!({
        "automatically_attached": {
            "system_context": system_context,
            "actor": { "id": claimed.work.owner_id.clone(), "role": role },
            "work": {
                "work_id": claimed.work.id,
                "revision": claimed.work.revision,
                "attempt_id": claimed.attempt_id,
                "attempt_no": claimed.attempt_no,
                "outcome": claimed.work.outcome.clone(),
                "expected_artifact": claimed.work.expected_artifact.clone(),
                "input_fingerprint": claimed.input_fingerprint.clone(),
            },
            "workspace": workspace,
            "upstream_artifact_versions": inputs,
            "work_feedback": feedback,
            "skill_roots": skill_roots.clone(),
            "capability_probe_locations": capability_probes.clone(),
        },
        "retrieved_depth": {
            "at_launch": [],
            "available_on_demand": [
                "the bound worktree and repository files",
                "project AGENTS.md and .agents/skills under the bound worktree",
                "Git history and working-tree status in the bound worktree",
                "full artifact content at the attached URI",
            ],
        },
        "unused_replay": {
            "lead_conversation": "not attached",
            "full_team_transcript": "not attached",
            "unrelated_actor_messages": "not attached",
        },
    });

    let input_lines = if claimed.inputs.is_empty() {
        "- none".to_string()
    } else {
        claimed
            .inputs
            .iter()
            .map(|input| {
                format!(
                    "- artifact {} · {} [{}] {} · digest {} · commit {} · runtime {}",
                    input.id,
                    if input.label.trim().is_empty() {
                        "unlabelled"
                    } else {
                        &input.label
                    },
                    input.kind,
                    input.uri,
                    input.digest.as_deref().unwrap_or("not recorded"),
                    input.source_commit.as_deref().unwrap_or("not recorded"),
                    input
                        .runtime_generation
                        .as_deref()
                        .unwrap_or("not recorded"),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let feedback_lines = if claimed.feedback.is_empty() {
        "- none".to_string()
    } else {
        claimed
            .feedback
            .iter()
            .map(|message| {
                format!(
                    "- message {} from {}: {}",
                    message.id, message.from_actor, message.body
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let workspace_lines = format!(
        "- Runtime working directory: {workdir}\n- Repository: {}\n- Base ref: {}\n- Declared integration ref: {}\n- Declared worktree: {}",
        claimed.work.repo.as_deref().unwrap_or("none — persistent company files only"),
        claimed.work.base_ref.as_deref().unwrap_or("none"),
        claimed.work.integration_branch.as_deref().unwrap_or("none"),
        claimed.work.worktree.as_deref().unwrap_or("Runtime-generated from Work id and revision"),
    );
    let expected_artifact = claimed.work.expected_artifact.trim();
    let completion_evidence = if expected_artifact.is_empty() {
        "- This Work has no declared output URI. Do not invent one; report the observed native result through the ordinary Work outcome.".to_string()
    } else if expected_artifact.starts_with('/') || expected_artifact.contains("://") {
        format!(
            "- Writing `{uri}` alone does not complete this Work. If it is the exact deliverable and meets the outcome, link it to this Attempt before declaring `outcome_met`:\n  `restless work artifact --work {work_id} --attempt {attempt_id} --kind output --uri {uri}`\n- If the candidate is missing or does not meet the outcome, do not attach it merely to pass the gate; report the specific gap instead.",
            uri = expected_artifact,
            work_id = claimed.work.id,
            attempt_id = claimed.attempt_id,
        )
    } else {
        format!(
            "- `{expected}` describes the expected proof; it is not yet a locator. Before declaring `outcome_met`, link the exact path, URL, or repo+commit you actually produced:\n  `restless work artifact --work {work_id} --attempt {attempt_id} --kind output --uri <exact-output-locator>`\n- If no candidate meets the outcome, do not attach a placeholder merely to pass the gate; report the specific gap instead.",
            expected = expected_artifact,
            work_id = claimed.work.id,
            attempt_id = claimed.attempt_id,
        )
    };
    let context = format!(
        "# Work {} revision {} attempt {}\nAttempt UUID: {}\n{}\n\nExpected artifact / proof: {}\nInput fingerprint: {}\n\n# Completion evidence [deterministic]\n{}\n\n# Bound workspace facts [automatic]\n{}\n\n# Bound upstream artifact versions [automatic]\n{}\n\n# Work-linked feedback [automatic]\n{}\n\n# Skill roots and truthful capability probes [automatic]\n- Skill roots available to OMP: {}\n- Probe Runtime tools at: `{}`\n- Probe company/runtime reachability at: `{}`\n- Probe configured credential references at: `{}`\n- Probe skill directories at: `{}`\nDo not treat a configured credential or an installed executable as provider acceptance, authority, or a successful effect.\n\n# Context accounting\n- Automatically attached: company doctrine and mission, actor role, Work/Attempt identity, exact workspace coordinates, upstream artifact versions, Work-linked feedback, skill roots, probe locations, and {}.\n- Retrieved depth at launch: none. Inspect bound files, project instructions, skills, Git history, and attached artifact content only when useful.\n- Not replayed: lead conversation, full team transcript, and unrelated actor messages.\n",
        claimed.work.id,
        claimed.work.revision,
        claimed.attempt_no,
        claimed.attempt_id,
        claimed.work.outcome,
        claimed.work.expected_artifact,
        claimed.input_fingerprint,
        completion_evidence,
        workspace_lines,
        input_lines,
        feedback_lines,
        skill_roots.join(", "),
        capability_probes[0],
        capability_probes[1],
        capability_probes[2],
        capability_probes[3],
        if accountable_lead {
            "the active team charter and roster when present"
        } else {
            "no team roster"
        },
    );
    (context, accounting)
}

pub(super) fn workspace_instruction(workdir: &str, conversation: bool) -> String {
    if conversation {
        if workdir == "/company" {
            return "Your working directory is /company. This is a coordination turn, not a productive file-editing surface: inspect existing company evidence and use ordinary Restless CLI to change the factual Work graph, but do not edit project or repository files, create any artifact, run a build or test, or invoke an external effect here. If evidence calls for a repair, build, or revision, first create or revise accountable Work with explicit repository coordinates, then let its bound Attempt perform the work; do not create a second plan or workflow."
                .to_string();
        }
        return format!(
            "Your working directory is {workdir}: a detached supporting review copy, not a productive Work checkout. You may inspect and run bounded executable review there, and may leave review-only supporting output there. Do not edit candidate/project files, commit, publish, invoke an external effect, or present review output as a replacement candidate. A repair belongs in attributable revision Work with explicit repository coordinates."
        );
    }
    if workdir == "/company" {
        return "Your working directory is /company, the persistent company Runtime. This Work has no repository or isolated worktree bound to it. Work in ordinary company files only; if the outcome actually requires repository edits, tell your accountable coordinator to replace this node with Work carrying explicit `--repo` and `--base-ref` coordinates rather than discovering a repo or creating a worktree yourself."
            .to_string();
    }
    format!(
        "Your working directory is {workdir} — it is YOURS: a dedicated git worktree. Commit meaningful checkpoints there with clear messages; do not touch other worktrees or the main checkout."
    )
}
