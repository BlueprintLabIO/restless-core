//! Work execution and process supervision. OrgIntel owns the deterministic
//! kickoff: a process may start only with an atomically claimed Work Attempt.
//! The registry below observes live processes and enforces a small resource
//! cap; it never owns delegation or task state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context as _, Result};
use restless_orgintel::{ClaimedWork, WorkAttemptState};
use restlessd::runtime_transport::{RuntimeProcessAuthority, RuntimeTransport};
use sha2::Digest as _;
use tokio_util::sync::CancellationToken;

mod context;
mod conversation;
mod execution;
mod gates;
mod recovery;
mod workspace;

use crate::runtime::{self, CompanyConfig};
use crate::spend::SpendLedger;
use context::{bound_attempt_context, shared_spine};
pub use conversation::{dispatch_actor_conversation, ConversationRuntime};
use execution::{run_staff_with_failover, StaffRun};
pub(crate) use recovery::reconcile_execution_substrate;
use recovery::{record_staff_outcome, record_unknown_recovery, StaffAttemptContext};
use workspace::{
    cleanup_attempt_runtime, cleanup_attempt_runtime_via_transport, ensure_worktree_via_transport,
    observe_workspace, observe_workspace_via_transport, recorded_start_observation,
    terminate_runtime_pid_via_transport, valid_slug, workdir_for,
};

#[cfg(test)]
use context::{actor_posture, team_capacity_context, workspace_instruction};
#[cfg(test)]
use conversation::{conversation_turn_prompt, internal_message_context};
#[cfg(test)]
use execution::{final_staff_usage, staff_spend_limit_reached, termination_prompt};
use gates::{reap_attempt_gate_processes_via_transport, reap_orphan_gate_processes};
#[cfg(test)]
use recovery::gate_cwd;
#[cfg(test)]
use workspace::WorkspaceObservation;

/// Resource guardrail, not a coordination policy. OrgIntel readiness and one
/// live process per durable actor still determine which Staff may run.
const STAFF_CAP_PER_COMPANY: usize = 100;
const CONVERSATION_BACKOFF_FIRST: std::time::Duration = std::time::Duration::from_secs(30);
const CONVERSATION_BACKOFF_CEILING: std::time::Duration = std::time::Duration::from_secs(300);
type ActorKey = (String, String);
type ConversationBackoff = (std::time::Instant, u32);
/// (company, actor) pairs with a live supervised process.
#[derive(Clone)]
struct RunningStaff {
    cancellation: CancellationToken,
    work_id: Option<uuid::Uuid>,
}

#[derive(Clone, Default)]
pub struct StaffRegistry {
    running: Arc<Mutex<HashMap<ActorKey, RunningStaff>>>,
    /// Coordination messages remain durable in OrgIntel. This local timer
    /// only prevents one unavailable lead session from being relaunched on
    /// every five-second scan; a daemon restart deliberately permits one new
    /// recovery attempt.
    conversation_backoff: Arc<Mutex<HashMap<ActorKey, ConversationBackoff>>>,
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

    fn try_claim(
        &self,
        company: &str,
        actor: &str,
        work_id: Option<uuid::Uuid>,
    ) -> Result<CancellationToken> {
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
            RunningStaff {
                cancellation: cancellation.clone(),
                work_id,
            },
        );
        Ok(cancellation)
    }

    fn release(&self, company: &str, actor: &str) {
        if let Ok(mut running) = self.running.lock() {
            running.remove(&(company.to_string(), actor.to_string()));
        }
    }

    fn conversation_is_backing_off(&self, company: &str, actor: &str) -> bool {
        self.conversation_backoff
            .lock()
            .map(|backoff| {
                backoff
                    .get(&(company.to_string(), actor.to_string()))
                    .is_some_and(|(until, _)| std::time::Instant::now() < *until)
            })
            .unwrap_or(true)
    }

    fn record_conversation_wake(&self, company: &str, actor: &str, usable: bool) {
        let Ok(mut backoff) = self.conversation_backoff.lock() else {
            return;
        };
        let key = (company.to_string(), actor.to_string());
        if usable {
            backoff.remove(&key);
            return;
        }
        let failures = backoff
            .get(&key)
            .map_or(1, |(_, failures)| failures.saturating_add(1));
        let delay = CONVERSATION_BACKOFF_FIRST
            .saturating_mul(1u32 << failures.saturating_sub(1).min(4))
            .min(CONVERSATION_BACKOFF_CEILING);
        backoff.insert(key, (std::time::Instant::now() + delay, failures));
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
                    .map(|active| active.cancellation.clone())
            })
            .is_some_and(|cancellation| {
                cancellation.cancel();
                true
            })
    }

    /// Interrupt only when the material feedback belongs to the Work this
    /// actor is currently executing. One actor may own several queued units;
    /// feedback for the next unit must never cancel an unrelated active turn.
    pub fn interrupt_work(&self, company: &str, actor: &str, work_id: uuid::Uuid) -> bool {
        self.running
            .lock()
            .ok()
            .and_then(|running| {
                running
                    .get(&(company.to_string(), actor.to_string()))
                    .filter(|active| active.work_id == Some(work_id))
                    .map(|active| active.cancellation.clone())
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
    runtime_transport: &Arc<dyn RuntimeTransport>,
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
    let cancellation = registry.try_claim(&config.name, &actor, Some(claimed.work.id))?;
    let container = runtime::container_name(&config.name);
    let responsibility = format!("work:{}", claimed.work.id);
    let attempt_runtime_authority = RuntimeProcessAuthority::Attempt {
        company: config.name.clone(),
        actor: actor.clone(),
        responsibility: responsibility.clone(),
        work_id: claimed.work.id,
        attempt_id: claimed.attempt_id,
        session_id: format!("attempt-runtime-{}", claimed.attempt_id.simple()),
    };

    let setup = async {
        let actors = org.list_actors().await?;
        let actor_row = actors
            .into_iter()
            .find(|row| row.id == actor)
            .with_context(|| format!("Work owner {actor:?} is not an OrgIntel actor"))?;
        let candidates = crate::model_gateway::available_actor_candidates(
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
            ensure_worktree_via_transport(
                config,
                Arc::clone(runtime_transport),
                &attempt_runtime_authority,
                &claimed.work,
                claimed.effective_base_ref.as_deref(),
                claimed.attempt_id,
                org,
            )
            .await?
        } else {
            let readiness = runtime_transport
                .readiness(&config.name)
                .await
                .map_err(|error| anyhow::anyhow!(error))
                .context("fingerprint non-repository Runtime")?;
            let environment = format!(
                "{:x}",
                sha2::Sha256::digest(readiness.runtime_image.as_bytes())
            );
            org.bind_attempt_execution_coordinates(
                claimed.attempt_id,
                None,
                None,
                None,
                &environment,
            )
            .await?;
            "/company".to_string()
        };
        let accountable_lead = org
            .list_teams()
            .await?
            .iter()
            .any(|team| team.lead_actor_id == actor);
        let start_observation = observe_workspace_via_transport(
            Arc::clone(runtime_transport),
            attempt_runtime_authority.clone(),
            &workdir,
        )
        .await;
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

    let (mut task, mut context_accounting) = bound_attempt_context(
        &claimed,
        &actor_row.role,
        &workdir,
        &config.name,
        accountable_lead,
    );
    match org.compile_constitution(claimed.work.id, 32 * 1024).await {
        Ok(brief) => {
            task.push_str("\n\n# Company Constitution [owner-released, Work-bound]\n");
            task.push_str(&brief.body);
            context_accounting["automatically_attached"]["company_constitution"] = serde_json::json!({
                "release_id": brief.release_id,
                "brief_digest": brief.digest,
                "bytes": brief.bytes,
                "pillars": brief.pillars.iter().map(|pillar| serde_json::json!({
                    "pillar": pillar.pillar,
                    "status": pillar.status,
                    "digest": pillar.digest,
                    "bytes": pillar.bytes,
                    "included_evidence": pillar.included_evidence_ids.len(),
                    "omitted_evidence": pillar.omitted_evidence_ids.len(),
                })).collect::<Vec<_>>(),
            });
        }
        Err(error)
            if error
                .to_string()
                .contains("no released expression identity") =>
        {
            context_accounting["automatically_attached"]["company_constitution"] =
                serde_json::json!("no owner-authored release");
        }
        Err(error) => {
            tracing::warn!(
                %error,
                work_id = %claimed.work.id,
                "could not compile the Work-bound company constitution"
            );
            task.push_str(&format!(
                "\n\n# Company Constitution [Work-bound]\nThe bounded constitution could not be compiled: {error}. Do not invent replacement facts, voice, visuals or culture. Surface this exact conflict to the accountable lead when identity affects the outcome.\n"
            ));
            context_accounting["automatically_attached"]["company_constitution"] =
                serde_json::json!({ "error": error.to_string() });
        }
    }
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
    let worker_runtime = config.worker_runtime;
    let reasoning_effort = config.reasoning_effort.clone();
    let authority = authority.clone();
    let capabilities = capabilities.clone();
    let runtime_transport = Arc::clone(runtime_transport);
    let attempt_runtime_authority_for_settlement = attempt_runtime_authority.clone();
    let role = actor_row.role;
    let attempt_id = claimed.attempt_id;
    let work_id = claimed.work.id;
    let live_turn = activities.start_work(&company, &actor, work_id, attempt_id);
    let observer = Some(live_turn.observer());
    tokio::spawn(async move {
        let outcome = run_staff_with_failover(StaffRun {
            container,
            runtime_transport: Arc::clone(&runtime_transport),
            workdir: workdir.clone(),
            company: company.clone(),
            actor: actor.clone(),
            responsibility,
            work_id: Some(work_id),
            attempt_id: Some(attempt_id),
            name: name.clone(),
            task,
            turn_prompt,
            role,
            spine,
            candidates,
            org: org.clone(),
            spend,
            spend_ceiling,
            worker_runtime,
            reasoning_effort,
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
                runtime_transport: Arc::clone(&runtime_transport),
                runtime_authority: &attempt_runtime_authority_for_settlement,
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
pub async fn sweep_orphans(
    root: &std::path::Path,
    orgintel: &crate::OrgIntelRegistry,
    runtime_transport: &Arc<dyn RuntimeTransport>,
) {
    let backend = restlessd::hosted_runtime::RuntimeBackendKind::from_entry_mode(
        std::env::var("RESTLESS_ENTRY_MODE").ok().as_deref(),
    );
    match backend {
        Ok(restlessd::hosted_runtime::RuntimeBackendKind::LocalDocker) => {
            sweep_local_orphans(root, orgintel).await;
        }
        Ok(restlessd::hosted_runtime::RuntimeBackendKind::HostedRuntimeBridge) => {
            sweep_transport_orphans(root, orgintel, runtime_transport).await;
        }
        Err(error) => tracing::error!(%error, "cannot select Runtime orphan-recovery backend"),
    }
}

async fn sweep_transport_orphans(
    root: &std::path::Path,
    orgintel: &crate::OrgIntelRegistry,
    runtime_transport: &Arc<dyn RuntimeTransport>,
) {
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
        if runtime_transport.readiness(name).await.is_err() {
            continue;
        }
        let Ok(org) = orgintel.get(name).await else {
            continue;
        };
        let Ok(attempts) = org.list_work_attempts(None).await else {
            continue;
        };
        let running = attempts
            .into_iter()
            .filter(|attempt| attempt.state == WorkAttemptState::Running)
            .collect::<Vec<_>>();
        let Ok(resources) = org.list_live_runtime_resources().await else {
            continue;
        };
        let activity = match runtime_transport.activity(name).await {
            Ok(activity) => activity,
            Err(error) => {
                tracing::warn!(company = name, %error, "could not observe orphaned Runtime processes");
                continue;
            }
        };

        for process in &activity.active_processes {
            if let Err(error) = terminate_runtime_pid_via_transport(
                runtime_transport.as_ref(),
                &process.authority,
                process.pid,
            )
            .await
            {
                tracing::warn!(
                    company = name,
                    process = %process.process_id,
                    %error,
                    "could not terminate orphaned governed Runtime process"
                );
            }
        }

        for attempt in running {
            let Some(work) = org.get_work(attempt.work_id).await.ok().flatten() else {
                tracing::warn!(attempt = %attempt.id, "orphaned Attempt lost its Work row");
                continue;
            };
            let authority = activity
                .active_processes
                .iter()
                .find_map(|process| match &process.authority {
                    RuntimeProcessAuthority::Attempt { attempt_id, .. }
                        if *attempt_id == attempt.id =>
                    {
                        Some(process.authority.clone())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| RuntimeProcessAuthority::Attempt {
                    company: name.to_owned(),
                    actor: work.owner_id.clone(),
                    responsibility: format!("work:{}", attempt.work_id),
                    work_id: attempt.work_id,
                    attempt_id: attempt.id,
                    session_id: format!("attempt-recovery-{}", attempt.id.simple()),
                });
            let markers = resources
                .iter()
                .filter(|resource| {
                    resource.attempt_id == attempt.id && resource.kind == "process_group"
                })
                .map(|resource| resource.value.clone())
                .collect::<Vec<_>>();
            let reaped = reap_attempt_gate_processes_via_transport(
                runtime_transport.as_ref(),
                &authority,
                &markers,
            )
            .await;
            if reaped > 0 {
                tracing::warn!(company = name, attempt = %attempt.id, reaped, "reaped orphaned governed gate process groups");
            }
            let Ok(workdir) = workdir_for(&work) else {
                tracing::warn!(attempt = %attempt.id, "orphaned Attempt has invalid workspace coordinates");
                continue;
            };
            let start = recorded_start_observation(&org, attempt.id, &workdir).await;
            let end = observe_workspace_via_transport(
                Arc::clone(runtime_transport),
                authority.clone(),
                &workdir,
            )
            .await;
            let note = "supervisor restarted; cognitive process was lost before trustworthy semantic completion";
            match record_unknown_recovery(&org, attempt.id, note, &start, &end).await {
                Ok(()) => {
                    if let Err(error) = org
                        .release_attempt_resources(
                            attempt.id,
                            "daemon restart closed orphaned Attempt",
                        )
                        .await
                    {
                        tracing::warn!(attempt = %attempt.id, "failed to release orphaned Attempt resources: {error:#}");
                    }
                    if let Err(error) = cleanup_attempt_runtime_via_transport(
                        runtime_transport.as_ref(),
                        &authority,
                        &workdir,
                        attempt.id,
                    )
                    .await
                    {
                        tracing::warn!(attempt = %attempt.id, "failed to clean orphaned Attempt runtime: {error:#}");
                    }
                }
                Err(error) => {
                    tracing::warn!(attempt = %attempt.id, "failed to preserve orphan recovery capsule: {error:#}");
                }
            }
        }
    }
}

async fn sweep_local_orphans(root: &std::path::Path, orgintel: &crate::OrgIntelRegistry) {
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
        match reap_orphan_gate_processes(&container).await {
            Ok(reaped) if reaped > 0 => tracing::warn!(
                company = name,
                reaped,
                "reaped governed gate process groups from before restart"
            ),
            Ok(_) => {}
            Err(error) => tracing::warn!(
                company = name,
                "could not reap orphan governed gate processes: {error:#}"
            ),
        }
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
            match record_unknown_recovery(&org, attempt.id, note, &start, &end).await {
                Ok(()) => {
                    if let Err(error) = org
                        .release_attempt_resources(
                            attempt.id,
                            "daemon restart closed orphaned Attempt",
                        )
                        .await
                    {
                        tracing::warn!(attempt = %attempt.id, "failed to release orphaned Attempt resources: {error:#}");
                    }
                    if let Err(error) =
                        cleanup_attempt_runtime(&container, &workdir, attempt.id).await
                    {
                        tracing::warn!(attempt = %attempt.id, "failed to clean orphaned Attempt runtime: {error:#}");
                    }
                }
                Err(error) => {
                    tracing::warn!(attempt = %attempt.id, "failed to preserve orphan recovery capsule: {error:#}");
                }
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
    use restless_orgintel::{
        ActorRow, ArtifactRefRow, ArtifactRefState, ClaimedWork, MessageRow, TeamRow,
        WorkAttemptState, WorkRow, WorkStatus,
    };

    #[test]
    fn accountable_lead_context_includes_its_small_current_team_without_a_staff_quota() {
        let team_id = uuid::Uuid::new_v4();
        let now = Utc::now();
        let team = TeamRow {
            id: team_id,
            name: "Onboarding Quality".into(),
            brief: "Owns first-player discovery quality.".into(),
            outcome_standard: restless_orgintel::OutcomeStandard::Exceptional,
            outcome_standard_source: restless_orgintel::OutcomeStandardSource::CompanyDefault,
            standard_source_message_id: None,
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
            outcome_standard: None,
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
                producing_topology: restless_orgintel::ProducingTopology::CoherentSingleWorker,
                commissioned_by: "world-direction".into(),
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
            review_targets: Vec::new(),
            effective_base_ref: None,
            attempt_id: uuid::Uuid::new_v4(),
            attempt_no: 1,
            session_id: uuid::Uuid::new_v4().to_string(),
            input_fingerprint: "input-fingerprint".into(),
            inputs: Vec::new(),
            feedback: Vec::new(),
            previous_attempt_state: None,
            previous_attempt_summary: None,
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
        assert!(context.contains("Effective base ref: none"));
        assert!(context.contains("Declared base ref: none"));
        assert!(context.contains("Runtime will fast-forward"));
        assert!(context.contains("Do not move `main`"));
        assert!(context.contains("# Completion evidence [deterministic]"));
        assert!(context.contains("# Producer posture [creative ownership]"));
        assert!(context.contains("A separate critic owns constraint checking"));
        assert!(!context.contains("# Critic posture [independent judgement]"));
        assert!(!context.contains("the active team charter and roster when present"));
        assert!(context.contains("Runtime binds this Attempt's clean terminal commit and tree"));
        assert!(context.contains("may materialize that evidence themselves"));
        assert!(context
            .contains("declare `outcome_met` even if gate-generated evidence does not exist yet"));
        assert!(!context.contains(&format!(
            "restless work artifact --work {work_id} --attempt {}",
            claimed.attempt_id
        )));
        assert!(context.contains("Not replayed: lead conversation"));
        assert!(!context.contains("# Context-recovery posture [automatic]"));
        assert_eq!(
            accounting["automatically_attached"]["workspace"]["effective_base_ref"],
            serde_json::Value::Null
        );
        assert_eq!(
            accounting["retrieved_depth"]["at_launch"],
            serde_json::json!([])
        );
        assert!(accounting["automatically_attached"]["system_context"]
            .get("active_team_capacity")
            .is_none());

        claimed.inputs.push(ArtifactRefRow {
            id: uuid::Uuid::new_v4(),
            kind: "output".into(),
            uri: "/company/outputs/playable-room.html".into(),
            note: "prior exact version".into(),
            created_by: "world-builder".into(),
            work_id: Some(work_id),
            attempt_id: Some(uuid::Uuid::new_v4()),
            digest: Some("sha256:prior".into()),
            source_commit: Some("1234567890abcdef1234567890abcdef12345678".into()),
            runtime_generation: None,
            label: "prior output".into(),
            state: ArtifactRefState::Available,
            created_at: Utc::now(),
            superseded_at: None,
        });
        let (successor_context, _) = bound_attempt_context(
            &claimed,
            "world builder",
            "/company/worktrees/work-bound-r1",
            "cosmon_test",
            false,
        );
        assert!(successor_context.contains("consumes same-Work output artifact"));
        assert!(successor_context.contains("without re-linking or relabelling its producer"));
        assert!(successor_context.contains("# Bound artifact versions [automatic]"));

        claimed.previous_attempt_state = Some(WorkAttemptState::Blocked);
        claimed.previous_attempt_summary =
            Some("[context] the provider session history exceeded the request limit".into());
        let (recovery_context, recovery_accounting) = bound_attempt_context(
            &claimed,
            "world builder",
            "/company/worktrees/work-bound-r1",
            "cosmon_test",
            false,
        );
        assert!(recovery_context.contains("# Context-recovery posture [automatic]"));
        assert!(recovery_context.contains("Do not repeat completed browser capture"));
        assert_eq!(
            recovery_accounting["automatically_attached"]["work"]["context_recovery"],
            true
        );

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

        claimed.review_targets = vec![uuid::Uuid::new_v4()];
        let (critic_context, critic_accounting) = bound_attempt_context(
            &claimed,
            "customer copy critic",
            "/company/worktrees/work-bound-r1",
            "cosmon_test",
            false,
        );
        assert!(critic_context.contains("# Critic posture [independent judgement]"));
        assert!(critic_context
            .contains("Judge the customer-facing outcome before policing constraints"));
        assert!(critic_context.contains("Quality comes first"));
        assert!(!critic_context.contains("# Producer posture [creative ownership]"));
        assert_eq!(
            critic_accounting["automatically_attached"]["work"]["review_targets"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
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
        assert!(rules.contains("do not commission promotion-only Work"));
        assert!(crate::staff::context::ACCOUNTABLE_QUALITY_ENFORCEMENT
            .contains("Never let a stale report decide a repaired candidate"));
        assert!(crate::staff::context::ACCOUNTABLE_QUALITY_ENFORCEMENT
            .contains("evidence, not owner delivery"));
        assert!(crate::staff::context::ACCOUNTABLE_QUALITY_ENFORCEMENT
            .contains("exactly one current available ReviewTarget"));

        let lead = actor_posture(true);
        assert!(lead.contains("ACCOUNTABLE LEAD"));
        assert!(lead.contains("non-producing supervisor"));
        assert!(lead.contains("at least one Staff worker"));
        assert!(lead.contains("truthful attribution"));
        assert!(lead.contains("material Staff exception is a decision boundary"));
        assert!(lead.contains("Clean passing completion remains observable state"));
        assert!(lead.contains("Never accept a consequentially substandard charter"));
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
            source_tree: Some("tree-aaaaaaaa".into()),
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
        let active_work = uuid::Uuid::new_v4();
        let cancellation = registry
            .try_claim("company_test", "research-lead", Some(active_work))
            .expect("claim the staff turn");

        assert!(!registry.interrupt_work("company_test", "research-lead", uuid::Uuid::new_v4(),));
        assert!(!cancellation.is_cancelled());
        assert!(registry.interrupt_work("company_test", "research-lead", active_work,));
        assert!(cancellation.is_cancelled());

        let registry = StaffRegistry::default();
        let cancellation = registry
            .try_claim("company_test", "research-lead", None)
            .expect("claim the conversational turn");
        assert!(registry.interrupt("company_test", "research-lead"));
        assert!(cancellation.is_cancelled());
        assert!(!registry.interrupt("company_test", "other-actor"));
    }

    #[test]
    fn unusable_lead_wake_backs_off_without_consuming_its_durable_message() {
        let registry = StaffRegistry::default();
        assert!(!registry.conversation_is_backing_off("company_test", "delivery-lead"));

        registry.record_conversation_wake("company_test", "delivery-lead", false);
        assert!(registry.conversation_is_backing_off("company_test", "delivery-lead"));
        assert!(
            !registry.conversation_is_backing_off("company_test", "another-lead"),
            "one oversized responsibility must not stall another department"
        );

        registry.record_conversation_wake("company_test", "delivery-lead", true);
        assert!(!registry.conversation_is_backing_off("company_test", "delivery-lead"));
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
