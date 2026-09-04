//! Stable staff context assembly and workspace instructions.
//!
//! The functions here package current OrgIntel and Runtime facts for a turn;
//! they do not create Work, sessions, or a second source of coordination.

use restless_orgintel::{ActorRow, ClaimedWork, TeamRow, WorkStatus};

use crate::runtime::CompanyConfig;

/// Standing quality doctrine for accountable leads. This deliberately names a
/// judgement loop rather than adding another Runtime state machine: Work and
/// Attempt remain the only production/accounting primitives.
pub(super) const ACCOUNTABLE_QUALITY_ENFORCEMENT: &str = r#"# Outcome quality enforcement [accountable-lead doctrine]
Own the accepted native outcome, not the producer's task completion. Before substantial production, retrieve the strongest available product truth, incumbent or gold-standard references, real operating environment, and relevant shared skill. For consequential, creative, ambiguous, or repeatedly failing outcomes, read `/opt/restless/skills/outcome-quality-enforcement/SKILL.md` before commissioning Work.

Maintain a concise durable quality contract in the team charter, Work outcomes, or linked evidence: the observable outcome, priority and trade-offs, authoritative references in order, minimum quality floor, exclusions, required native evidence, independent acceptance test, owner-attention envelope, and stop condition. Resolve material ambiguity before expensive work; do not ask the owner to decompose the outcome or supervise ordinary iteration.

Treat every producer result as a claim. Inspect the real artifact in its native environment. Separate creation from evaluation and, where taste, correctness, safety, or consequence matters, commission a fresh-context independent critic who receives the contract and exact artifact but not the producer's rationale. Verify that the critic and its evidence actually test the promised outcome; green mechanical gates can reject broken work but cannot approve qualitative excellence.

Bind every evaluation and re-evaluation to the exact current candidate identity available in the Runtime—commit, digest, artifact version, or runtime generation. A revision invalidates verdicts and captures from the prior identity even when their paths are unchanged. Require the critic to record the identity it actually operated, and where deterministic identity exists, gate that record against the candidate before accepting the verdict. Never let a stale report decide a repaired candidate.

When the owner asked to review an exact native outcome, close through one Work explicitly declared `--owner-review`. Linking a `review_target` artifact to ordinary Work is evidence, not owner delivery, and does not create Attention. Before declaring the charter complete, inspect the owner projection and prove that exactly one current available ReviewTarget for the accepted candidate is present.

Continue attributable Staff-owned produce → operate → evaluate → repair loops while a consequential contract gap and a credible improvement hypothesis remain. An Attempt limit is a local execution guard, never an outcome-quality ceiling: commission the next sparse revision or replacement Work when needed. Change the approach rather than repeating an exhausted one; reset a contaminated trajectory when its assumptions or rejected form keep anchoring the result. Prefer the simplest root fix that closes the largest gap, and make repeated failures pay rent through a reusable skill, evaluation, test, tool, or durable observation.

Stop only at quality convergence: the contract is proven in the native environment, independent review accepts it when required, the verifier is credible, and remaining gaps are non-consequential—or a concrete external constraint, authority boundary, or irreducible owner judgement prevents further machine work. Report uncertainty honestly and never lower the bar merely because time, tokens, attempts, or enthusiasm are running low."#;

/// A durable actor keeps one organisational posture across every wake. Before
/// this distinction, a lead's conversation wake called it accountable while
/// its productive Work wake called the same actor a mere specialist.
pub(super) fn actor_posture(accountable_lead: bool) -> &'static str {
    if accountable_lead {
        "You are the ACCOUNTABLE LEAD for this team's whole accepted outcome, not a relay, producer, or smaller Exec. You remain a non-producing supervisor on every wake. Frame, commission, observe, guide, redirect, and repair through at least one Staff worker; never edit the candidate, perform its planned production, or silently repair its artifact yourself. You retain native review, quality convergence judgement, and truthful attribution of every real contribution. A material Staff exception is a decision boundary, not a status-update opportunity: before ending that wake, either repair or redirect attributable Work, record the concrete blocker, or escalate the exact judgement. Clean passing completion remains observable state and does not require a ceremonial model wake. Never accept a consequentially substandard charter merely because one bounded Work item passed."
    } else {
        "You are a SPECIALIST, not a smaller Exec. Own the bounded responsibility your role names, surface material contradictions early, and say plainly when something falls outside it. Do not quietly take over the whole team outcome: a specialist who does every job is a generalist with a job title. The Runtime records every terminal Work fact and wakes your accountable lead only for a material exception. Do not send progress or completion mail merely to wake the lead; message the lead only for a genuinely new fact or contradiction that must be judged before your terminal result."
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
        "\n# Commissioned outcome standard [owner/company policy]\n\
         `{}` via `{}`{}. This is an ambition contract, not permission, a team-size target, or a loop quota. Fast still preserves safety, truth, authority and native correctness. Thorough seeks production readiness. Exceptional seeks a clearly superior outcome through strong references, purposeful exploration and independent native evaluation where consequence warrants it. Frontier seeks a new ceiling while reporting uncertainty and diminishing returns honestly. Translate the standard into this outcome's fitness, evidence and stopping judgement.\n\n\
         # Your available team [internal decision]\n{} — {}\n{}\n\
         This is available capacity, not a headcount target. Every executable outcome needs at least \
         one Staff producer: commission one end-to-end worker by default, and add more only when a \
         stable independently useful seam repays coordination cost. Create each producer's bounded \
         Work with `restless work add` before it starts; messages are not assignments, and lead-owned \
         production Work is invalid. When an authenticated external message caused the outcome, add \
         `--source-message <message-id>` so source linkage and Work creation commit once together.\n",
        team.outcome_standard,
        team.outcome_standard_source.as_str(),
        team.standard_source_message_id
            .map(|id| format!(" from owner message {id}"))
            .unwrap_or_default(),
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
            "\nYou are the non-producing accountable supervisor for this team's outcome. Resolve ordinary uncertainty and local blockers inside the charter by guiding or recommissioning Staff; message Exec only for cross-team resources, company priority, strategy, charter scope, or authority escalation. Use the Work CLI to make every Staff contribution and its exact artifact observable. A material Staff exception is a decision boundary: do not end that wake with progress-only conversation while the charter remains incomplete; repair or redirect Staff-owned Work, record a genuine blocker, or escalate the exact judgement. Clean passing completion remains observable and needs no ceremonial model wake.\n",
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
    let is_critic = !claimed.review_targets.is_empty();
    let context_recovery = claimed
        .previous_attempt_summary
        .as_deref()
        .is_some_and(|summary| summary.trim_start().starts_with("[context]"));
    let workspace = serde_json::json!({
        "runtime_workdir": workdir,
        "repository": claimed.work.repo.clone(),
        "declared_base_ref": claimed.work.base_ref.clone(),
        "effective_base_ref": claimed.effective_base_ref.clone(),
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
                "context_recovery": context_recovery,
                "review_targets": claimed.review_targets,
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
        "- Runtime working directory: {workdir}\n- Repository: {}\n- Effective base ref: {}\n- Declared base ref: {}\n- Declared integration ref: {}\n- Declared worktree: {}",
        claimed
            .work
            .repo
            .as_deref()
            .unwrap_or("none — persistent company files only"),
        claimed.effective_base_ref.as_deref().unwrap_or("none"),
        claimed.work.base_ref.as_deref().unwrap_or("none"),
        claimed.work.integration_branch.as_deref().unwrap_or("none"),
        claimed
            .work
            .worktree
            .as_deref()
            .unwrap_or("Runtime-generated from Work id and revision"),
    );
    let expected_artifact = claimed.work.expected_artifact.trim();
    let artifact_kind = if claimed.work.owner_review_required {
        restless_orgintel::REVIEW_TARGET_ARTIFACT_KIND
    } else {
        "output"
    };
    let review_note = if claimed.work.owner_review_required {
        format!(
            " This Work requires owner outcome review: choose exactly one native candidate as the ReviewTarget and link it with kind `{}`. The Runtime runs the declared `{}` gate after your process returns; do not claim that probe passed yourself.",
            restless_orgintel::REVIEW_TARGET_ARTIFACT_KIND,
            restless_orgintel::REVIEW_TARGET_LIVE_PROBE_GATE,
        )
    } else {
        String::new()
    };
    let inherited_output_ids = claimed
        .inputs
        .iter()
        .filter(|artifact| artifact.work_id == Some(claimed.work.id) && artifact.kind == "output")
        .map(|artifact| artifact.id.to_string())
        .collect::<Vec<_>>();
    let completion_evidence = if claimed.work.repo.is_some() && !claimed.work.owner_review_required
    {
        format!(
            "- The Runtime binds this Attempt's clean terminal commit and tree as the exact candidate; do not spend a model turn creating or linking a bookkeeping artifact.\n- `{expected}` describes the expected outcome or gate evidence. Declared Runtime gates run only after your process returns and may materialize that evidence themselves. When the clean candidate is ready for those gates, declare `outcome_met` even if gate-generated evidence does not exist yet; the Runtime, not you, decides pass/fail.\n- If the candidate itself is incomplete, report the specific gap instead of claiming completion.",
            expected = if expected_artifact.is_empty() {
                "the clean repository candidate"
            } else {
                expected_artifact
            },
        )
    } else if expected_artifact.is_empty() {
        "- This Work has no declared output URI. Do not invent one; report the observed native result through the ordinary Work outcome.".to_string()
    } else if expected_artifact.starts_with('/') || expected_artifact.contains("://") {
        format!(
            "- Writing `{uri}` alone does not complete this Work. If it is the exact deliverable and meets the outcome, link it to this Attempt before declaring `outcome_met`:\n  `restless work artifact --work {work_id} --attempt {attempt_id} --kind {artifact_kind} --uri {uri}`\n- If the candidate is missing or does not meet the outcome, do not attach it merely to pass the gate; report the specific gap instead.{review_note}",
            uri = expected_artifact,
            work_id = claimed.work.id,
            attempt_id = claimed.attempt_id,
        )
    } else {
        format!(
            "- `{expected}` describes the expected proof; it is not yet a locator. Before declaring `outcome_met`, link the exact path, URL, or repo+commit you actually produced:\n  `restless work artifact --work {work_id} --attempt {attempt_id} --kind {artifact_kind} --uri <exact-output-locator>`\n- If no candidate meets the outcome, do not attach a placeholder merely to pass the gate; report the specific gap instead.{review_note}",
            expected = expected_artifact,
            work_id = claimed.work.id,
            attempt_id = claimed.attempt_id,
        )
    };
    let integration_note = claimed
        .work
        .integration_branch
        .as_deref()
        .map_or_else(String::new, |branch| {
            format!(
                "\n- This is the final attributed candidate for shared branch `{branch}`. Work only in this Attempt worktree. Do not move `{branch}` or touch its checkout: after your clean exact commit passes the gates, the Runtime will fast-forward that checked-out branch and will refuse a dirty or divergent integration."
            )
        });
    let inherited_output_note = if inherited_output_ids.is_empty() {
        String::new()
    } else {
        format!(
            "\n- This successor Attempt already consumes same-Work output artifact(s) {} as immutable inputs. If an exact inherited version still meets the outcome after applying current feedback, validate and retain it without re-linking or relabelling its producer. If you change the output, link the new version to this Attempt normally.",
            inherited_output_ids.join(", ")
        )
    };
    let completion_evidence =
        format!("{completion_evidence}{inherited_output_note}{integration_note}");
    let craft_posture = if is_critic {
        format!(
            "# Critic posture [independent judgement]\n\
             Judge the customer-facing outcome before policing constraints. Review in this order: \
             (1) is the offer immediately clear, valuable and credible to its intended reader; \
             (2) does the voice feel natural, specific and confident; (3) is the requested action easy; \
             (4) are claims supported and internal boundaries respected. Quality comes first, but it \
             never licenses invented facts. Distinguish an empirical claim from a deliberate commercial \
             offer: a company-chosen prototype, trial, price or service promise is a decision to honour, \
             not a historical fact that must be hedged for lack of prior evidence. If the stated offer \
             conflicts with a real capacity or authority uncertainty, escalate that internal decision \
             instead of weakening customer copy into tentative research language. Keep \
             constraints in the review report; do not rewrite the \
             artifact into governance language. Inspect the exact candidates from producer Work: {}.\n",
            claimed
                .review_targets
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        "# Producer posture [creative ownership]\n\
         Create the strongest outcome for the real audience. Lead with customer value, clarity, specificity and a natural human voice. Use company truth and evidence as material, not as a compliance checklist. State a company-chosen commercial offer directly as something the company intends to honour; do not weaken it into exploring, considering or discussing whether it could be appropriate. Internal approval state, review mechanics, authority language, risk controls and process labels are not customer copy unless the outcome genuinely requires the reader to know them. Do not pre-emptively weaken the work to satisfy an imagined critic. A separate critic owns constraint checking after a compelling candidate exists.\n"
            .to_string()
    };
    let recovery_note = if context_recovery {
        "\n\n# Context-recovery posture [automatic]\n- The immediately preceding Attempt exceeded the provider context limit; its model session is not a usable continuation.\n- Continue from durable files, linked artifacts, compact manifests, and text summaries already present. Do not repeat completed browser capture, media loading, broad repository scans, or another high-volume evidence pass.\n- Use targeted probes only for missing decision-bearing facts. If capture and judgement cannot fit in one bounded pass, stop and ask the accountable lead to split them into separate Work instead of rebuilding the same context."
    } else {
        ""
    };
    let context = format!(
        "# Work {} revision {} attempt {}\nAttempt UUID: {}\n{}\n\n{}\nExpected artifact / proof: {}\nInput fingerprint: {}\n\n# Completion evidence [deterministic]\n{}{}\n\n# Bound workspace facts [automatic]\n{}\n\n# Bound artifact versions [automatic]\n{}\n\n# Work-linked feedback [automatic]\n{}\n\n# Skill roots and truthful capability probes [automatic]\n- Skill roots available to OMP: {}\n- Probe Runtime tools at: `{}`\n- Probe company/runtime reachability at: `{}`\n- Probe configured credential references at: `{}`\n- Probe skill directories at: `{}`\nDo not treat a configured credential or an installed executable as provider acceptance, authority, or a successful effect.\n\n# Context accounting\n- Automatically attached: company doctrine and mission, actor role, Work/Attempt identity, exact workspace coordinates, bound artifact versions, Work-linked feedback, skill roots, probe locations, and {}.\n- Retrieved depth at launch: none. Inspect bound files, project instructions, skills, Git history, and attached artifact content only when useful.\n- Not replayed: lead conversation, full team transcript, and unrelated actor messages.\n",
        claimed.work.id,
        claimed.work.revision,
        claimed.attempt_no,
        claimed.attempt_id,
        claimed.work.outcome,
        craft_posture,
        claimed.work.expected_artifact,
        claimed.input_fingerprint,
        completion_evidence,
        recovery_note,
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
