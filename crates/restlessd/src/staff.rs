//! Work execution and process supervision. OrgIntel owns the deterministic
//! kickoff: a process may start only with an atomically claimed Work Attempt.
//! The registry below observes live processes and enforces a small resource
//! cap; it never owns delegation or task state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context as _, Result};
use restless_orgintel::{ClaimedWork, WorkAttemptState};
use tokio_util::sync::CancellationToken;

mod context;
mod conversation;
mod execution;
mod recovery;
mod workspace;

use crate::runtime::{self, CompanyConfig};
use crate::spend::SpendLedger;
use context::{bound_attempt_context, shared_spine};
pub use conversation::{dispatch_actor_conversation, ConversationRuntime};
use execution::{run_staff_with_failover, StaffRun};
use recovery::{record_staff_outcome, record_unknown_recovery, StaffAttemptContext};
use workspace::{
    ensure_worktree, observe_workspace, recorded_start_observation, valid_slug, workdir_for,
};

#[cfg(test)]
use context::{actor_posture, team_capacity_context, workspace_instruction};
#[cfg(test)]
use conversation::{conversation_turn_prompt, internal_message_context};
#[cfg(test)]
use execution::{final_staff_usage, staff_spend_limit_reached, termination_prompt};
#[cfg(test)]
use recovery::gate_cwd;
#[cfg(test)]
use workspace::WorkspaceObservation;

/// Resource guardrail, not a coordination policy. OrgIntel readiness and one
/// live process per durable actor still determine which Staff may run.
const STAFF_CAP_PER_COMPANY: usize = 100;
/// (company, actor) pairs with a live supervised process.
#[derive(Clone, Default)]
pub struct StaffRegistry {
    running: Arc<Mutex<HashMap<(String, String), CancellationToken>>>,
}

impl StaffRegistry {
    pub fn has_capacity(&self, company: &str) -> bool {
        self.running
            .lock()
            .map(|running| {
                running
                    .iter()
                    .filter(|((candidate, _), _)| candidate == company)
                    .count()
                    < STAFF_CAP_PER_COMPANY
            })
            .unwrap_or(false)
    }

    fn try_claim(&self, company: &str, actor: &str) -> Result<CancellationToken> {
        let mut running = self
            .running
            .lock()
            .map_err(|_| anyhow::anyhow!("staff registry"))?;
        if running.contains_key(&(company.to_string(), actor.to_string())) {
            bail!("actor {actor} is already running");
        }
        let active = running
            .keys()
            .filter(|(candidate, _)| candidate == company)
            .count();
        if active >= STAFF_CAP_PER_COMPANY {
            bail!("staff cap ({STAFF_CAP_PER_COMPANY}) reached for {company}");
        }
        let cancellation = CancellationToken::new();
        running.insert(
            (company.to_string(), actor.to_string()),
            cancellation.clone(),
        );
        Ok(cancellation)
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
            .map(|running| running.contains_key(&(company.to_string(), actor.to_string())))
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
                    .filter(|((running_company, _), _)| running_company == company)
                    .map(|((_, actor), _)| actor.clone())
                    .collect();
                actors.sort();
                actors
            })
            .unwrap_or_default()
    }

    /// Request a bounded interruption of one supervised actor. The live ACP
    /// turn observes this token and returns its durable state to the next
    /// wake instead of letting two conversations overlap.
    pub fn interrupt(&self, company: &str, actor: &str) -> bool {
        self.running
            .lock()
            .ok()
            .and_then(|running| {
                running
                    .get(&(company.to_string(), actor.to_string()))
                    .cloned()
            })
            .is_some_and(|cancellation| {
                cancellation.cancel();
                true
            })
    }
}

/// Start exactly one already-claimed Work Attempt. No other public function
/// can launch a Staff actor.
#[expect(
    clippy::too_many_arguments,
    reason = "the Staff launch boundary keeps authority, ownership and live supervision explicit"
)]
pub async fn dispatch_claimed_work(
    config: &CompanyConfig,
    spend: &SpendLedger,
    authority: &crate::authority::AuthorityStore,
    capabilities: &crate::capability::CapabilityIssuer,
    org: &restless_orgintel::OrgIntel,
    registry: &StaffRegistry,
    activities: &crate::activity::AgentActivityStreams,
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
    // Reserve the durable actor before the first await after the database
    // claim. Otherwise a queued free-form Exec wake can claim its separate
    // in-memory slot while model/workspace setup is yielding, launching two
    // processes for one actor even though the Work Attempt lease is sound.
    let cancellation = registry.try_claim(&config.name, &actor)?;
    let container = runtime::container_name(&config.name);

    let setup = async {
        let actors = org.list_actors().await?;
        let actor_row = actors
            .into_iter()
            .find(|row| row.id == actor)
            .with_context(|| format!("Work owner {actor:?} is not an OrgIntel actor"))?;
        let candidates = crate::model_gateway::available_candidates(
            config,
            actor_row.model.as_deref(),
            authority,
        )
        .await?;
        let first_model = candidates
            .first()
            .context("staff model policy has no candidates")?;
        // A missing preference adopts the company's first available model.
        // A temporary cooldown must not overwrite an explicit preference;
        // actual models used remain visible on Attempt and model events.
        if actor_row.model.is_none() {
            org.update_actor_model(&actor, first_model).await?;
        }
        org.set_attempt_model(claimed.attempt_id, first_model)
            .await?;
        let workdir = if claimed.work.repo.is_some() {
            ensure_worktree(config, &claimed.work).await?
        } else {
            "/company".to_string()
        };
        let accountable_lead = org
            .list_teams()
            .await?
            .iter()
            .any(|team| team.lead_actor_id == actor);
        let start_observation = observe_workspace(&container, &workdir).await;
        anyhow::Ok((
            actor_row,
            candidates,
            workdir,
            accountable_lead,
            start_observation,
        ))
    }
    .await;
    let (actor_row, candidates, workdir, accountable_lead, start_observation) = match setup {
        Ok(setup) => setup,
        Err(error) => {
            registry.release(&config.name, &actor);
            return Err(error);
        }
    };

    let (task, context_accounting) = bound_attempt_context(
        &claimed,
        &actor_row.role,
        &workdir,
        &config.name,
        accountable_lead,
    );
    if let Err(error) = org
        .emit_event(
            "attempt_context_bound",
            Some(&actor),
            serde_json::json!({
                "work_id": claimed.work.id,
                "attempt_id": claimed.attempt_id,
                "accounting": context_accounting,
            }),
        )
        .await
    {
        registry.release(&config.name, &actor);
        return Err(error.into());
    }
    if let Err(error) = org
        .emit_event(
            "attempt_process_started",
            Some(&actor),
            serde_json::json!({
                "work_id": claimed.work.id,
                "attempt_id": claimed.attempt_id,
                "workspace": start_observation.clone(),
            }),
        )
        .await
    {
        registry.release(&config.name, &actor);
        return Err(error.into());
    }

    let spine = shared_spine(config, org, &actor, accountable_lead).await;
    let company = config.name.clone();
    let name = actor_row.display;
    let turn_prompt = if claimed.feedback.is_empty() {
        "Begin or continue the claimed Work Attempt described in your system context. Work until the assigned outcome is done or genuinely blocked."
            .to_string()
    } else {
        format!(
            "# New Work feedback\n{}\n\nApply this feedback to the claimed Work Attempt described in your system context. Work until the assigned outcome is done or genuinely blocked.",
            claimed
                .feedback
                .iter()
                .map(|message| format!(
                    "- message {} from {}: {}",
                    message.id, message.from_actor, message.body
                ))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    let org = org.clone();
    let registry = registry.clone();
    let spend = spend.clone();
    let spend_ceiling = config.spend_ceiling_usd;
    let authority = authority.clone();
    let capabilities = capabilities.clone();
    let role = actor_row.role;
    let attempt_id = claimed.attempt_id;
    let work_id = claimed.work.id;
    let live_turn = activities.start_work(&company, &actor, work_id, attempt_id);
    let observer = Some(live_turn.observer());
    tokio::spawn(async move {
        let gate_container = container.clone();
        let outcome = run_staff_with_failover(StaffRun {
            container,
            workdir: workdir.clone(),
            company: company.clone(),
            actor: actor.clone(),
            responsibility: format!("work:{work_id}"),
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
            conversation: false,
            accountable_lead,
            observer,
            cancellation,
        })
        .await;
        match &outcome {
            Ok(outcome) if outcome.termination != crate::exec::Termination::Blocked => {
                live_turn.complete(None, outcome.output_tokens);
            }
            Ok(outcome) => live_turn.fail(&outcome.summary),
            Err(error) => live_turn.fail(&format!("Work supervision stopped: {error:#}")),
        }
        record_staff_outcome(
            &org,
            StaffAttemptContext {
                container: &gate_container,
                actor: &actor,
                name: &name,
                work_id,
                attempt_id,
                workdir: &workdir,
                start_observation,
            },
            outcome.map(|outcome| (outcome.termination, outcome.summary)),
        )
        .await;
        registry.release(&company, &actor);
    });
    Ok(())
}

/// Boot sweep: any marked ACP session still running in a company container
/// when the daemon starts is an orphan of the last daemon's lifetime —
/// unsupervised, its transcript unreachable. Reap only those Linux sessions,
/// preserve their Runtime evidence, and leave the productive outcome unknown
/// for the accountable lead to review.
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
            let Some(work) = org.get_work(attempt.work_id).await.ok().flatten() else {
                tracing::warn!(attempt = %attempt.id, "orphaned Attempt lost its Work row");
                continue;
            };
            let Ok(workdir) = workdir_for(&work) else {
                tracing::warn!(attempt = %attempt.id, "orphaned Attempt has invalid workspace coordinates");
                continue;
            };
            let start = recorded_start_observation(&org, attempt.id, &workdir).await;
            let end = observe_workspace(&container, &workdir).await;
            let note = "supervisor restarted; cognitive process was lost before trustworthy semantic completion";
            if let Err(error) = record_unknown_recovery(&org, attempt.id, note, &start, &end).await
            {
                tracing::warn!(attempt = %attempt.id, "failed to preserve orphan recovery capsule: {error:#}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        actor_posture, bound_attempt_context, conversation_turn_prompt, final_staff_usage,
        gate_cwd, internal_message_context, staff_spend_limit_reached, team_capacity_context,
        termination_prompt, workspace_instruction, StaffRegistry, WorkspaceObservation,
    };
    use crate::acp::TurnUsage;
    use crate::model_gateway::ModelBilling;
    use chrono::Utc;
    use restless_orgintel::{ActorRow, ClaimedWork, MessageRow, TeamRow, WorkRow, WorkStatus};

    #[test]
    fn accountable_lead_context_includes_its_small_current_team_without_a_staff_quota() {
        let team_id = uuid::Uuid::new_v4();
        let now = Utc::now();
        let team = TeamRow {
            id: team_id,
            name: "Onboarding Quality".into(),
            brief: "Owns first-player discovery quality.".into(),
            lead_actor_id: "onboarding-curator".into(),
            created_by: "exec".into(),
            created_at: now,
            disbanded_at: None,
        };
        let actor = |id: &str, display: &str, role: &str, team_id: Option<uuid::Uuid>| ActorRow {
            id: id.into(),
            kind: "staff".into(),
            role: role.into(),
            display: display.into(),
            model: None,
            team_id,
            retired_at: None,
            retired_by: None,
            retirement_reason: String::new(),
            created_at: now,
        };
        let context = team_capacity_context(
            &team,
            &[
                actor(
                    "onboarding-curator",
                    "Avery Vale",
                    "accountable curator",
                    Some(team_id),
                ),
                actor(
                    "world-builder",
                    "Sera Morn",
                    "environmental readability specialist",
                    Some(team_id),
                ),
                actor("other", "Mina Vale", "unrelated specialist", None),
            ],
        );

        assert!(context.contains("Onboarding Quality — Owns first-player discovery quality."));
        assert!(context.contains("Avery Vale · accountable curator · accountable lead"));
        assert!(context.contains("Sera Morn · environmental readability specialist"));
        assert!(!context.contains("Mina Vale"));
        assert!(context.contains("available capacity, not a headcount target"));
        assert!(context.contains("commission one end-to-end worker by default"));
        assert!(context.contains("lead-owned production Work is invalid"));
        assert!(context.contains("restless work add"));
    }

    #[test]
    fn staff_workspace_prompt_never_calls_the_company_root_an_isolated_worktree() {
        let repo = workspace_instruction("/company/worktrees/work-123-r1", false);
        assert!(repo.contains("dedicated git worktree"));
        assert!(repo.contains("/company/worktrees/work-123-r1"));

        let files = workspace_instruction("/company", false);
        assert!(files.contains("persistent company Runtime"));
        assert!(files.contains("no repository or isolated worktree"));
        assert!(!files.contains("it is YOURS: a dedicated git worktree"));

        let conversation = workspace_instruction("/company", true);
        assert!(conversation.contains("do not create a second plan"));
        assert!(conversation.contains("not a productive file-editing surface"));
        assert!(conversation.contains("then let its bound Attempt perform the work"));
        assert!(!conversation.contains("dedicated git worktree"));
    }

    #[test]
    fn immediate_team_conversation_prompt_preserves_the_execution_boundary() {
        let internal = conversation_turn_prompt("message from world-builder", &[]);
        assert!(internal.contains("# Coordination execution boundary [invariant]"));
        assert!(internal.contains("not a claimed productive Work Attempt"));
        assert!(internal.contains("Do not edit project or repository files"));
        assert!(internal.contains("never make a hidden repair yourself"));
        assert!(internal.contains("not attributable"));
        assert!(internal.contains("Do not use Exec as a status relay"));
        assert!(internal.contains("There is no owner input in this wake"));
        assert!(internal.contains("`restless message` without `--to`"));
        assert!(internal.contains("restless message list"));

        let owner = conversation_turn_prompt(
            "message from owner",
            &["- owner message 4: prepare review".into()],
        );
        assert!(
            owner.contains("# Owner input [authoritative in source; interpret before applying]")
        );
        assert!(owner.contains("- owner message 4: prepare review"));
        assert!(owner.contains("not a claimed productive Work Attempt"));
    }

    #[test]
    fn work_linked_coordination_mail_names_the_exact_reply_scope() {
        let work_id = uuid::Uuid::new_v4();
        let message = MessageRow {
            id: 42,
            from_actor: "world-builder".into(),
            to_actor: Some("product-direction".into()),
            body: "terrain interface changed".into(),
            created_at: Utc::now(),
            read_at: None,
        };
        let context = internal_message_context(&message, Some(work_id));

        assert!(context.contains(&format!("Work {work_id}, message 42")));
        assert!(context.contains(&format!(
            "restless message --work {work_id} --to world-builder"
        )));
        assert!(context.contains("exactly one direct Work-linked reply"));
        assert!(context.contains("Do not send an unlinked acknowledgement"));
    }

    #[test]
    fn metered_staff_turn_stops_at_its_remaining_company_envelope() {
        let usage = TurnUsage {
            used: 42_000,
            size: 200_000,
            cost_usd: Some(0.80),
        };
        assert!(staff_spend_limit_reached(true, 0.80, &usage));
        assert!(staff_spend_limit_reached(true, 0.79, &usage));
        assert!(!staff_spend_limit_reached(true, 0.81, &usage));
        assert!(!staff_spend_limit_reached(false, 0.01, &usage));
        assert!(!staff_spend_limit_reached(
            false,
            0.01,
            &TurnUsage {
                cost_usd: None,
                ..usage
            },
        ));
    }

    #[test]
    fn bound_context_keeps_an_omitted_workspace_detail_truthful_and_usable() {
        let work_id = uuid::Uuid::new_v4();
        let mut claimed = ClaimedWork {
            work: WorkRow {
                id: work_id,
                goal_id: None,
                owner_id: "world-builder".into(),
                title: "Build the playable room".into(),
                outcome: "Create the room in the bound repository.".into(),
                status: WorkStatus::Active,
                resolution: String::new(),
                priority: 0,
                expected_artifact: "/company/outputs/playable-room.html".into(),
                owner_review_required: false,
                repo: Some("cosmon".into()),
                // A lead did not fill every optional coordinate. The Runtime
                // still has an authoritative repository and generated worktree
                // rather than asking Staff to reconstruct either one.
                base_ref: None,
                integration_branch: Some("main".into()),
                worktree: None,
                revision: 1,
                attempt_limit: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            attempt_id: uuid::Uuid::new_v4(),
            attempt_no: 1,
            session_id: uuid::Uuid::new_v4().to_string(),
            input_fingerprint: "input-fingerprint".into(),
            inputs: Vec::new(),
            feedback: Vec::new(),
        };

        let (context, accounting) = bound_attempt_context(
            &claimed,
            "world builder",
            "/company/worktrees/work-bound-r1",
            "cosmon_test",
            false,
        );

        assert!(context.contains("Repository: cosmon"));
        assert!(context.contains("Runtime working directory: /company/worktrees/work-bound-r1"));
        assert!(context.contains("Base ref: none"));
        assert!(context.contains("# Completion evidence [deterministic]"));
        assert!(!context.contains("the active team charter and roster when present"));
        assert!(context.contains(&format!(
            "restless work artifact --work {work_id} --attempt {} --kind output --uri /company/outputs/playable-room.html",
            claimed.attempt_id
        )));
        assert!(context.contains("Not replayed: lead conversation"));
        assert_eq!(
            accounting["automatically_attached"]["workspace"]["base_ref"],
            serde_json::Value::Null
        );
        assert_eq!(
            accounting["retrieved_depth"]["at_launch"],
            serde_json::json!([])
        );
        assert!(accounting["automatically_attached"]["system_context"]
            .get("active_team_capacity")
            .is_none());

        claimed.work.owner_review_required = true;
        let (review_context, _) = bound_attempt_context(
            &claimed,
            "world builder",
            "/company/worktrees/work-bound-r1",
            "cosmon_test",
            false,
        );
        assert!(review_context.contains(&format!(
            "--kind {} --uri /company/outputs/playable-room.html",
            restless_orgintel::REVIEW_TARGET_ARTIFACT_KIND
        )));
        assert!(review_context.contains(restless_orgintel::REVIEW_TARGET_LIVE_PROBE_GATE));
        assert!(!review_context.contains("--kind output --uri /company/outputs/playable-room.html"));
    }

    #[test]
    fn one_actor_keeps_one_organisational_posture_across_wake_types() {
        let rules = crate::context::COMPANY_OPERATING_RULES;
        assert!(rules.contains("causal understanding of the outcome"));
        assert!(rules.contains("Lead production is never valid"));
        assert!(rules.contains("handoff template, message cadence, shared-state form"));
        assert!(rules.contains("scheduler-created"));
        assert!(rules.contains("prove that another actor contributed"));
        assert!(rules.contains("not the lead's plan, reasoning, checklist"));

        let lead = actor_posture(true);
        assert!(lead.contains("ACCOUNTABLE LEAD"));
        assert!(lead.contains("non-producing supervisor"));
        assert!(lead.contains("at least one Staff worker"));
        assert!(lead.contains("truthful attribution"));
        assert!(!lead.contains("You are a SPECIALIST"));

        let specialist = actor_posture(false);
        assert!(specialist.contains("You are a SPECIALIST"));
        assert!(specialist.contains("bounded responsibility"));
        assert!(!specialist.contains("ACCOUNTABLE LEAD"));

        let lead_end = termination_prompt(true);
        assert!(lead_end.contains("must never receive a productive Work Attempt"));
        assert!(lead_end.contains("supervisor invariant"));

        let specialist_end = termination_prompt(false);
        assert!(specialist_end.contains("assigned specialist task"));
        assert!(!specialist_end.contains("accountable outcome Work"));
    }

    #[test]
    fn atomic_gates_follow_each_attempt_revision_worktree() {
        assert_eq!(
            gate_cwd("@attempt", "/company/worktrees/work-abc-r2"),
            "/company/worktrees/work-abc-r2"
        );
        assert_eq!(
            gate_cwd("/company/outputs", "/company/worktrees/work-abc-r2"),
            "/company/outputs"
        );
    }

    #[test]
    fn workspace_change_requires_two_real_git_observations() {
        let missing = WorkspaceObservation {
            workdir: "/company/worktrees/candidate".into(),
            ..WorkspaceObservation::default()
        };
        let start = WorkspaceObservation {
            workdir: missing.workdir.clone(),
            source_commit: Some("aaaaaaaa".into()),
            status_digest: Some("clean".into()),
            dirty_entries: 0,
        };
        let committed = WorkspaceObservation {
            source_commit: Some("bbbbbbbb".into()),
            ..start.clone()
        };
        let dirty = WorkspaceObservation {
            status_digest: Some("dirty".into()),
            dirty_entries: 2,
            ..start.clone()
        };
        assert!(!start.changed_since(&missing));
        assert!(!start.changed_since(&start));
        assert!(committed.changed_since(&start));
        assert!(dirty.changed_since(&start));
        assert!(!dirty.compact().contains("filename"));
    }

    #[test]
    fn owner_interruption_cancels_the_exact_active_staff_turn() {
        let registry = StaffRegistry::default();
        let cancellation = registry
            .try_claim("company_test", "research-lead")
            .expect("claim the staff turn");

        assert!(registry.interrupt("company_test", "research-lead"));
        assert!(cancellation.is_cancelled());
        assert!(!registry.interrupt("company_test", "other-actor"));
    }

    #[test]
    fn cumulative_acp_snapshots_keep_one_final_telemetry_total() {
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
        let (usage, reported) = final_staff_usage(ModelBilling::MeteredApi, &snapshots)
            .expect("one final cumulative usage snapshot");

        assert_eq!(usage.used, 64_814, "context usage is the final snapshot");
        assert_eq!(reported, Some(2.38));
    }
}
