//! Work execution and process supervision. OrgIntel owns the deterministic
//! kickoff: a process may start only with an atomically claimed Work Attempt.
//! The registry below observes live processes and enforces a small resource
//! cap; it never owns delegation or task state.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Context as _, Result, bail};
use restless_orgintel::ClaimedWork;
use sha2::Digest as _;
use tokio_util::sync::CancellationToken;

mod context;
mod conversation;
mod execution;
pub(crate) use execution::operator_text;
mod gates;
mod recovery;
mod workspace;

use crate::runtime::{self, CompanyConfig};
use crate::spend::SpendLedger;
use context::{bound_attempt_context, shared_spine};
pub use conversation::{ConversationRuntime, dispatch_actor_conversation};
use execution::{StaffRun, StaffTurnKind, run_staff_with_failover};
pub use recovery::reconcile_execution_substrate;
use recovery::{
    StaffAttemptContext, record_hosted_staff_outcome, record_staff_outcome, record_unknown_recovery,
};
use workspace::{
    WorkspaceObservation, cleanup_attempt_runtime, ensure_hosted_worktree, ensure_worktree,
    observe_workspace, recorded_start_observation, valid_slug, workdir_for,
};

#[cfg(test)]
use context::{actor_posture, team_capacity_context, workspace_instruction};
#[cfg(test)]
use conversation::{conversation_turn_prompt, internal_message_context};
#[cfg(test)]
use execution::{final_staff_usage, staff_spend_limit_reached, termination_prompt};
use gates::reap_orphan_gate_processes;
#[cfg(test)]
use recovery::gate_cwd;

/// Resource guardrail, not a coordination policy. OrgIntel readiness and one
/// live process per durable actor still determine which Staff may run.
const STAFF_CAP_PER_COMPANY: usize = 100;
/// Work Attempts that run at once in one company (S104 O2). Leads split independent
/// work into parallel Staff by default, and every Attempt shares the company's model
/// capacity (one subscription's rate limit, or the metered envelope's four Staff
/// slots), so more at once only queue at the provider. Conversations retain
/// scheduling priority, but all local turns also share the host resource cap.
const PARALLEL_WORK_PER_COMPANY: usize = 4;
const CONVERSATION_BACKOFF_FIRST: std::time::Duration = std::time::Duration::from_secs(30);
const CONVERSATION_BACKOFF_CEILING: std::time::Duration = std::time::Duration::from_secs(300);
pub const COGNITIVE_LEASE_DURATION: std::time::Duration = std::time::Duration::from_secs(5 * 60);
const COGNITIVE_LEASE_RENEWAL: std::time::Duration = std::time::Duration::from_secs(45);
const COGNITIVE_LEASE_DB_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
const COGNITIVE_LEASE_EXPIRY_MARGIN: std::time::Duration = std::time::Duration::from_secs(5);
type ActorKey = (String, String);
type ConversationBackoff = (std::time::Instant, u32);
/// (company, actor) pairs with a live supervised process.
#[derive(Clone)]
struct RunningStaff {
    cancellation: CancellationToken,
    work_id: Option<uuid::Uuid>,
}

/// Process-local supervision for the durable Actor-wide cognitive lease. The
/// database token is authority; this task merely renews it while ACP is alive
/// and turns renewal loss into the same cancellation signal the harness
/// already obeys.
pub struct CognitiveLeaseGuard {
    org: restless_orgintel::OrgIntel,
    lease: restless_orgintel::ActorCognitiveLease,
    stop: CancellationToken,
    renewal: Option<tokio::task::JoinHandle<()>>,
    armed: bool,
}

impl CognitiveLeaseGuard {
    pub async fn claim(
        org: &restless_orgintel::OrgIntel,
        actor: &str,
        cancellation: &CancellationToken,
    ) -> Result<Option<Self>> {
        let Some(lease) = org
            .claim_actor_cognitive_session(actor, COGNITIVE_LEASE_DURATION)
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(Self::from_claim(org, lease, cancellation)))
    }

    pub async fn claim_team_lead(
        org: &restless_orgintel::OrgIntel,
        actor: &str,
        team_id: uuid::Uuid,
        cancellation: &CancellationToken,
    ) -> Result<Option<Self>> {
        let Some(lease) = org
            .claim_team_lead_cognitive_session(actor, team_id, COGNITIVE_LEASE_DURATION)
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(Self::from_claim(org, lease, cancellation)))
    }

    fn from_claim(
        org: &restless_orgintel::OrgIntel,
        lease: restless_orgintel::ActorCognitiveLease,
        cancellation: &CancellationToken,
    ) -> Self {
        let stop = CancellationToken::new();
        let renew_stop = stop.clone();
        let renew_org = org.clone();
        let mut renew_lease = lease.clone();
        let renew_cancellation = cancellation.clone();
        let renewal = tokio::spawn(async move {
            loop {
                let until_deadline = renew_lease
                    .claimed_until
                    .signed_duration_since(chrono::Utc::now())
                    .to_std()
                    .unwrap_or_default();
                let safe_window = until_deadline.saturating_sub(COGNITIVE_LEASE_EXPIRY_MARGIN);
                if safe_window.is_zero() {
                    renew_cancellation.cancel();
                    break;
                }
                tokio::select! {
                    () = renew_stop.cancelled() => break,
                    () = tokio::time::sleep(COGNITIVE_LEASE_RENEWAL.min(safe_window)) => {}
                }
                let remaining = renew_lease
                    .claimed_until
                    .signed_duration_since(chrono::Utc::now())
                    .to_std()
                    .unwrap_or_default()
                    .saturating_sub(COGNITIVE_LEASE_EXPIRY_MARGIN);
                if remaining.is_zero() {
                    renew_cancellation.cancel();
                    break;
                }
                let result = tokio::time::timeout(
                    COGNITIVE_LEASE_DB_TIMEOUT.min(remaining),
                    renew_org.renew_actor_cognitive_session(&renew_lease, COGNITIVE_LEASE_DURATION),
                )
                .await;
                match result {
                    Ok(Ok(renewed)) => renew_lease = renewed,
                    Ok(Err(_)) | Err(_) => {
                        renew_cancellation.cancel();
                        break;
                    }
                }
            }
        });
        Self {
            org: org.clone(),
            lease,
            stop,
            renewal: Some(renewal),
            armed: true,
        }
    }

    pub fn lease(&self) -> &restless_orgintel::ActorCognitiveLease {
        &self.lease
    }

    /// Recheck the database token immediately before any conversation output
    /// is persisted. This closes the interval between renewal ticks.
    pub async fn confirm(&self) -> Result<()> {
        self.org
            .renew_actor_cognitive_session(&self.lease, COGNITIVE_LEASE_DURATION)
            .await?;
        Ok(())
    }

    pub async fn finish(mut self) {
        self.armed = false;
        self.stop.cancel();
        if let Some(renewal) = self.renewal.take() {
            let _ = renewal.await;
        }
        if let Err(error) = self.org.release_actor_cognitive_session(&self.lease).await {
            tracing::warn!(
                actor = %self.lease.actor_id,
                "could not release Actor cognitive-session lease: {error}"
            );
        }
    }
}

impl Drop for CognitiveLeaseGuard {
    fn drop(&mut self) {
        self.stop.cancel();
        if !self.armed {
            return;
        }
        let org = self.org.clone();
        let lease = self.lease.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let _ = org.release_actor_cognitive_session(&lease).await;
            });
        }
    }
}

#[derive(Clone, Default)]
pub struct StaffRegistry {
    running: Arc<Mutex<HashMap<ActorKey, RunningStaff>>>,
    resources: ResourceClaims,
    recovery_pending: Arc<Mutex<HashSet<String>>>,
    resource_retry: Arc<AtomicBool>,
    resource_limits: Arc<Mutex<HashMap<String, (std::time::Instant, runtime::ActorCapacity)>>>,
    /// Coordination messages remain durable in OrgIntel. This local timer
    /// only prevents one unavailable lead session from being relaunched on
    /// every five-second scan; a daemon restart deliberately permits one new
    /// recovery attempt.
    conversation_backoff: Arc<Mutex<HashMap<ActorKey, ConversationBackoff>>>,
}

type ResourceClaims = Arc<Mutex<HashMap<uuid::Uuid, (String, bool)>>>;

/// Covers setup and execution, including starts not yet visible in cgroups.
pub struct ActorResourcePermit {
    claims: ResourceClaims,
    id: uuid::Uuid,
    wake: Arc<tokio::sync::Notify>,
    started: bool,
}

impl ActorResourcePermit {
    pub fn activate(&mut self) {
        self.started = true;
    }
}

impl Drop for ActorResourcePermit {
    fn drop(&mut self) {
        if let Ok(mut claims) = self.claims.lock() {
            claims.remove(&self.id);
        }
        if self.started {
            self.wake.notify_one();
        }
    }
}

#[derive(Debug)]
pub struct CapacityDeferred;

impl std::fmt::Display for CapacityDeferred {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Compute capacity is busy. Queued messages and Work are kept.")
    }
}

impl std::error::Error for CapacityDeferred {}

impl StaffRegistry {
    pub fn recovery_pending(&self, company: &str) -> bool {
        self.recovery_pending.lock().map_or(true, |pending| pending.contains(company))
    }

    pub fn set_recovery_pending(&self, company: &str, waiting: bool) {
        if let Ok(mut pending) = self.recovery_pending.lock() {
            if waiting { pending.insert(company.to_string()); } else { pending.remove(company); }
        }
    }

    pub async fn reserve_actor_capacity(
        &self,
        company: &str,
        bridges: &crate::runtime_bridge::RuntimeBridgeRegistry,
        wake: &Arc<tokio::sync::Notify>,
    ) -> Result<Option<ActorResourcePermit>> {
        if self.recovery_pending(company) {
            self.resource_retry.store(true, Ordering::SeqCst);
            return Ok(None);
        }
        let capacity = if bridges.is_hosted() {
            // Paired machines execute outside this host's work pool.
            runtime::ActorCapacity {
                company_slots: STAFF_CAP_PER_COMPANY,
                shared_slots: None,
                company_memory_slots: usize::MAX,
                shared_memory_slots: None,
                waiting_for_memory: false,
                waiting_for_processes: false,
            }
        } else {
            match runtime::actor_capacity(company).await {
                Ok(Some(capacity)) => capacity,
                // Without observed host bounds, serialize local harness starts.
                Ok(None) => runtime::ActorCapacity {
                    company_slots: 1,
                    shared_slots: Some(1),
                    company_memory_slots: usize::MAX,
                    shared_memory_slots: None,
                    waiting_for_memory: false,
                    waiting_for_processes: false,
                },
                Err(error) => {
                    self.resource_retry.store(true, Ordering::SeqCst);
                    return Err(error);
                }
            }
        };
        self.resource_limits
            .lock()
            .map_err(|_| anyhow::anyhow!("actor capacity observation"))?
            .insert(company.to_string(), (std::time::Instant::now(), capacity));
        self.try_reserve_actor_capacity(company, capacity, wake)
    }

    fn try_reserve_actor_capacity(
        &self,
        company: &str,
        capacity: runtime::ActorCapacity,
        wake: &Arc<tokio::sync::Notify>,
    ) -> Result<Option<ActorResourcePermit>> {
        let mut claims = self
            .resources
            .lock()
            .map_err(|_| anyhow::anyhow!("actor resource guard"))?;
        let company_active = claims.values().filter(|(name, _)| name == company).count();
        let shared_active = claims.values().filter(|(_, shared)| *shared).count();
        if capacity.waiting_for_processes
            || capacity.waits_for_memory(company_active, shared_active)
            || company_active >= capacity.company_slots
            || capacity
                .shared_slots
                .is_some_and(|slots| shared_active >= slots)
        {
            self.resource_retry.store(true, Ordering::SeqCst);
            return Ok(None);
        }
        let id = uuid::Uuid::new_v4();
        claims.insert(id, (company.to_string(), capacity.shared_slots.is_some()));
        Ok(Some(ActorResourcePermit {
            claims: Arc::clone(&self.resources),
            id,
            wake: Arc::clone(wake),
            started: false,
        }))
    }

    pub fn take_resource_retry_hint(&self) -> bool {
        self.resource_retry.swap(false, Ordering::SeqCst)
    }

    pub fn resource_counts(&self, company: &str) -> Option<(usize, usize)> {
        let claims = self.resources.lock().ok()?;
        Some((
            claims.values().filter(|(name, _)| name == company).count(),
            claims.values().filter(|(_, shared)| *shared).count(),
        ))
    }

    pub fn has_capacity(&self, company: &str) -> bool {
        if self.recovery_pending(company) { return false; }
        let logical_capacity = self
            .running
            .lock()
            .map(|running| {
                running
                    .iter()
                    .filter(|((candidate, _), _)| candidate == company)
                    .count()
                    < STAFF_CAP_PER_COMPANY
            })
            .unwrap_or(false);
        if !logical_capacity {
            return false;
        }
        let limits = self
            .resource_limits
            .lock()
            .ok()
            .and_then(|limits| limits.get(company).copied());
        let Some((at, limits)) = limits else {
            return true;
        };
        if at.elapsed() >= std::time::Duration::from_secs(1) {
            return true;
        }
        // Avoid repeating Docker observations for every owed colleague while
        // all known slots are occupied. A real permit release wakes the scan.
        self.resource_counts(company).is_some_and(|(own, shared)| {
            own < limits.company_slots
                && limits.shared_slots.is_none_or(|slots| shared < slots)
                && !limits.waiting_for_processes
                && !limits.waits_for_memory(own, shared)
        })
    }

    /// Whether another Work Attempt may start in this company now.
    pub fn has_work_capacity(&self, company: &str) -> bool {
        self.running
            .lock()
            .map(|running| {
                running
                    .iter()
                    .filter(|((candidate, _), staff)| {
                        candidate == company && staff.work_id.is_some()
                    })
                    .count()
                    < PARALLEL_WORK_PER_COMPANY
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
        if work_id.is_some()
            && running
                .iter()
                .filter(|((candidate, _), staff)| candidate == company && staff.work_id.is_some())
                .count()
                >= PARALLEL_WORK_PER_COMPANY
        {
            bail!("{PARALLEL_WORK_PER_COMPANY} Work Attempts already run in {company}");
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

    /// The owner asked for a fresh turn: the next scheduler scan may dispatch
    /// this actor's conversation without waiting out a failure backoff.
    pub fn clear_conversation_backoff(&self, company: &str, actor: &str) {
        if let Ok(mut backoff) = self.conversation_backoff.lock() {
            backoff.remove(&(company.to_string(), actor.to_string()));
        }
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

    pub fn running_staff(&self) -> Vec<String> {
        self.running
            .lock()
            .map(|running| {
                let mut actors = running
                    .keys()
                    .map(|(company, actor)| format!("{company}/{actor}"))
                    .collect::<Vec<_>>();
                actors.sort();
                actors
            })
            .unwrap_or_else(|_| vec!["registry-unavailable".into()])
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

    /// The Work this actor's live turn is executing, if any. One actor has
    /// one live turn, so a request from that actor about this Work comes from
    /// inside its own Attempt.
    pub fn executing_work(&self, company: &str, actor: &str) -> Option<uuid::Uuid> {
        self.running.lock().ok().and_then(|running| {
            running
                .get(&(company.to_string(), actor.to_string()))
                .and_then(|active| active.work_id)
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
    runtime_bridges: &crate::runtime_bridge::RuntimeBridgeRegistry,
    schedule_wake: &std::sync::Arc<tokio::sync::Notify>,
    claimed: ClaimedWork,
    mut resources: ActorResourcePermit,
) -> Result<()> {
    let actor = claimed.work.owner_id.clone();
    let effective_config = config.for_agent(&actor);
    let config = &effective_config;
    anyhow::ensure!(
        config.has_effective_model_route(config.worker_harness),
        "Connect a model in Company → Intelligence before starting an agent."
    );
    if actor == "owner" {
        bail!("{actor} is not a Staff execution actor");
    }
    if let Some(repo) = &claimed.work.repo {
        if !valid_slug(repo) {
            bail!("invalid repo name {repo:?}");
        }
    }
    let hosted_identity = if runtime_bridges.is_hosted() {
        if !matches!(
            config.worker_harness,
            crate::runtime::AgentHarness::RestlessManaged
                | crate::runtime::AgentHarness::ClaudeAgent
                | crate::runtime::AgentHarness::Codex
        ) {
            bail!("hosted delegated Work requires a supported agent harness");
        }
        let identity = crate::runtime_bridge::expected_identity(authority, &config.name).await?;
        crate::runtime_bridge::preflight(runtime_bridges, &identity).await?;
        Some(identity)
    } else {
        None
    };
    // Reserve the durable actor before the first await after the database
    // claim. Otherwise a queued free-form Exec wake can claim its separate
    // in-memory slot while model/workspace setup is yielding, launching two
    // processes for one actor even though the Work Attempt lease is sound.
    let cancellation = registry.try_claim(&config.name, &actor, Some(claimed.work.id))?;
    let container = runtime::container_name(&config.name);

    let setup = async {
        let actors = org.list_actors().await?;
        let actor_row = actors
            .into_iter()
            .find(|row| row.id == actor)
            .with_context(|| format!("Work owner {actor:?} is not an OrgIntel actor"))?;
        let candidates = crate::model_gateway::available_actor_candidates(
            config,
            config.worker_harness,
            config.agent_preference(&actor, actor_row.model.as_deref()),
            authority,
        )
        .await?;
        let first_model = candidates
            .first()
            .context("staff model policy has no candidates")?;
        // A missing preference adopts the company's first available model.
        // A temporary cooldown must not overwrite an explicit preference;
        // actual models used remain visible on Attempt and model events.
        // Native harness markers describe this attempt's route, not a durable
        // actor preference that should survive a later harness change.
        if actor_row.model.is_none()
            && runtime::validate_company_model_selection(first_model).is_ok()
        {
            org.update_actor_model(&actor, first_model).await?;
        }
        // S102: a routine first Attempt starts on the everyday model, with the primary behind it
        // as failover. A repair, a review, feedback or any doubt keeps the primary.
        let candidates = if config.everyday_model.is_some()
            && claimed.attempt_no == 1
            && claimed.review_targets.is_empty()
            && claimed.feedback.is_empty()
            && crate::decisions::work_suits_everyday_model(org, config, &claimed.work).await
        {
            crate::model_gateway::with_everyday_model(
                config,
                config.worker_harness,
                candidates,
                authority,
            )
            .await?
        } else {
            candidates
        };
        let first_model = candidates
            .first()
            .context("staff model policy has no candidates")?;
        org.set_attempt_model(claimed.attempt_id, first_model)
            .await?;
        let (workdir, start_observation, hosted_environment_fingerprint) = if let Some(identity) =
            hosted_identity.as_ref()
        {
            if claimed.work.repo.is_some() {
                let prepared = ensure_hosted_worktree(
                    runtime_bridges,
                    identity,
                    &claimed.work,
                    claimed.effective_base_ref.as_deref(),
                    claimed.attempt_id,
                    org,
                )
                .await?;
                if prepared.observation.dirty_entries != 0 {
                    bail!("hosted Attempt worktree is dirty before agent admission");
                }
                (
                    prepared.workdir,
                    prepared.observation,
                    Some(prepared.environment_fingerprint),
                )
            } else {
                let environment =
                    crate::runtime_bridge::ready_environment_fingerprint(runtime_bridges, identity)
                        .await?;
                org.bind_attempt_execution_coordinates(
                    claimed.attempt_id,
                    None,
                    None,
                    None,
                    &environment,
                )
                .await?;
                (
                    "/company".to_string(),
                    WorkspaceObservation {
                        workdir: "/company".into(),
                        ..WorkspaceObservation::default()
                    },
                    Some(environment),
                )
            }
        } else if claimed.work.repo.is_some() {
            let workdir = ensure_worktree(
                config,
                &claimed.work,
                claimed.effective_base_ref.as_deref(),
                claimed.attempt_id,
                org,
            )
            .await?;
            let observation = observe_workspace(&container, &workdir).await;
            (workdir, observation, None)
        } else {
            let container_image = tokio::process::Command::new("docker")
                .args(["inspect", "--format", "{{.Image}}", &container])
                .output()
                .await
                .context("fingerprint non-repository Runtime")?;
            if !container_image.status.success() {
                bail!("could not fingerprint non-repository Runtime");
            }
            let environment = format!(
                "{:x}",
                sha2::Sha256::digest(
                    String::from_utf8_lossy(&container_image.stdout)
                        .trim()
                        .as_bytes()
                )
            );
            org.bind_attempt_execution_coordinates(
                claimed.attempt_id,
                None,
                None,
                None,
                &environment,
            )
            .await?;
            let workdir = "/company".to_string();
            let observation = observe_workspace(&container, &workdir).await;
            (workdir, observation, None)
        };
        let accountable_lead = org
            .list_teams()
            .await?
            .iter()
            .any(|team| team.lead_actor_id == actor);
        anyhow::Ok((
            actor_row,
            candidates,
            workdir,
            accountable_lead,
            start_observation,
            hosted_environment_fingerprint,
        ))
    }
    .await;
    let (
        actor_row,
        candidates,
        workdir,
        accountable_lead,
        start_observation,
        hosted_environment_fingerprint,
    ) = match setup {
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
    // Selected skills travel with the Work, not the harness: every Attempt,
    // retry and replacement harness receives the same pinned versions.
    let selected_skills = match org.work_skill_selections(claimed.work.id).await {
        Ok(selected) => selected,
        Err(error) => {
            registry.release(&config.name, &actor);
            return Err(error.into());
        }
    };
    task.push_str("\n\n");
    task.push_str(&crate::skills::contract_section());
    if !selected_skills.is_empty() {
        task.push_str("\n\n# Selected skills for this Work [Work-bound]\n");
        for skill in &selected_skills {
            task.push_str(&format!(
                "- {} (digest {}): run `restless skill use {} --work {} --attempt {}` once before substantial work and follow it.\n",
                skill.skill_name,
                skill.digest,
                skill.skill_name,
                claimed.work.id,
                claimed.attempt_id,
            ));
        }
    }
    context_accounting["automatically_attached"]["selected_skills"] =
        serde_json::to_value(&selected_skills).unwrap_or_default();
    let spine = match shared_spine(
        config,
        org,
        &actor,
        accountable_lead,
        restless_orgintel::ActorContextFocus::WorkAttempt {
            work_id: claimed.work.id,
            attempt_id: claimed.attempt_id,
        },
    )
    .await
    {
        Ok(spine) => spine,
        Err(error) => {
            registry.release(&config.name, &actor);
            return Err(error.into());
        }
    };
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
    let schedule_wake = std::sync::Arc::clone(schedule_wake);
    let spend = spend.clone();
    let spend_ceiling = spend.effective_ceiling(config);
    let worker_harness = config.worker_harness;
    let reasoning_effort = config.reasoning_effort.clone();
    let authority = authority.clone();
    let capabilities = capabilities.clone();
    let gate_capabilities = capabilities.clone();
    let runtime_bridges = runtime_bridges.clone();
    let hosted = hosted_identity.is_some();
    let role = actor_row.role;
    let attempt_id = claimed.attempt_id;
    let work_id = claimed.work.id;
    let live_turn = activities.start_work(&company, &actor, work_id, attempt_id);
    let observer = Some(live_turn.observer());
    resources.activate();
    tokio::spawn(async move {
        let _resources = resources;
        let gate_container = container.clone();
        let outcome = run_staff_with_failover(StaffRun {
            container,
            workdir: workdir.clone(),
            company: company.clone(),
            actor: actor.clone(),
            responsibility: format!("work:{work_id}"),
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
            worker_harness,
            reasoning_effort,
            authority,
            capabilities,
            runtime_bridges: runtime_bridges.clone(),
            hosted_identity: hosted_identity.clone(),
            turn_kind: StaffTurnKind::Work,
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
        let semantic_outcome = outcome.map(|outcome| (outcome.termination, outcome.summary));
        if hosted {
            record_hosted_staff_outcome(
                &org,
                &actor,
                &name,
                work_id,
                attempt_id,
                &workdir,
                start_observation,
                hosted_environment_fingerprint.as_deref(),
                &runtime_bridges,
                hosted_identity.as_ref(),
                semantic_outcome,
            )
            .await;
        } else {
            record_staff_outcome(
                &org,
                StaffAttemptContext {
                    container: &gate_container,
                    company: &company,
                    actor: &actor,
                    name: &name,
                    capabilities: &gate_capabilities,
                    work_id,
                    attempt_id,
                    workdir: &workdir,
                    start_observation,
                },
                semantic_outcome,
            )
            .await;
        }
        registry.release(&company, &actor);
        // The outcome write's work_changed scan can run while this actor
        // still holds its slot, and then skips the actor's next ready Work or
        // follow-up revision. Rescan now that the slot is free, rather than
        // leaving that Work to the five-minute repair sweep.
        schedule_wake.notify_one();
    });
    Ok(())
}

/// Boot sweep: any marked ACP session still running in a company container
/// when the daemon starts is an orphan of the last daemon's lifetime —
/// unsupervised, its transcript unreachable. Reap only those Linux sessions,
/// preserve their Runtime evidence, and leave the productive outcome unknown
/// for the accountable lead to review. A host reboot leaves computers stopped
/// with their Attempts still running, so every company's Attempts are closed.
pub async fn sweep_orphans(
    orgintel: &crate::OrgIntelRegistry,
    registry: &StaffRegistry,
    companies: &[String],
) -> Vec<String> {
    let mut failed = HashSet::new();
    for name in companies {
        // Owner lifecycle changes use this lock too, so a stopped or replaced
        // Runtime cannot race the observed cleanup and Attempt reconciliation.
        let _start = runtime::company_start_guard(name).await;
        let running = match runtime::recovery_runtime_running(name).await {
            Ok(running) => running,
            Err(error) => {
                tracing::warn!(company = name, "could not observe Runtime recovery state: {error:#}");
                registry.set_recovery_pending(name, true);
                failed.insert(name.clone());
                continue;
            }
        };
        if running {
            match runtime::workload_admission(name).await {
                Ok(Some(admission)) if admission.waiting_for_memory => {
                    registry.set_recovery_pending(name, true);
                    failed.insert(name.clone());
                    continue;
                }
                Err(error) => {
                    tracing::warn!(company = name, "recovery waits for a resource reading: {error:#}");
                    registry.set_recovery_pending(name, true);
                    failed.insert(name.clone());
                    continue;
                }
                _ => {}
            }
            if let Err(error) = crate::effect::sweep_orphan_company(name).await {
                tracing::warn!(company = name, "could not verify orphan effect cleanup: {error:#}");
                registry.set_recovery_pending(name, true);
                failed.insert(name.clone());
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
                Err(error) => {
                    tracing::warn!(company = name, "could not reap orphan governed gate processes: {error:#}");
                    failed.insert(name.clone());
                },
            }
            match crate::acp::reap_orphan_sessions(&container).await {
                Ok(reaped) if reaped > 0 => tracing::warn!(
                    company = name,
                    reaped,
                    "reaped marked agent processes from before restart"
                ),
                Ok(_) => {},
                Err(error) => {
                    tracing::warn!(company = name, "could not verify orphan agent cleanup: {error:#}");
                    failed.insert(name.clone());
                },
            }
        }
        if failed.contains(name) {
            registry.set_recovery_pending(name, true);
            continue;
        }
        let Ok(org) = orgintel.get(name).await else {
            registry.set_recovery_pending(name, true);
            failed.insert(name.clone());
            continue;
        };
        let container = running.then(|| runtime::container_name(name));
        let interrupted = match org.list_running_work_attempts().await {
            Ok(attempts) => attempts.into_iter().map(|attempt| attempt.work_id).collect::<Vec<_>>(),
            Err(error) => {
                tracing::warn!(company = name, "could not inspect interrupted attempts: {error:#}");
                registry.set_recovery_pending(name, true);
                failed.insert(name.clone());
                continue;
            }
        };
        recover_unsupervised_attempts(
            &org,
            registry,
            name,
            container.as_deref(),
            "supervisor restarted; cognitive process was lost before trustworthy semantic completion",
        )
        .await;
        // The restart, not the agent, ended these Attempts: continue each
        // Work once from its saved session (Sprint 104 A3).
        for work_id in interrupted {
            match org.resume_after_platform_restart(work_id).await {
                Ok(true) => {
                    tracing::info!(company = name, work = %work_id, "resumed Work interrupted by a restart")
                }
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(company = name, work = %work_id, "could not resume Work after restart: {error:#}")
                }
            }
        }
        registry.set_recovery_pending(name, false);
    }
    failed.into_iter().collect()
}

/// Close every running Attempt that no process in this daemon supervises. Its
/// supervisor failed to record an outcome or did not survive a restart, so it
/// can never finish by itself; until closed, its actor cannot be claimed and
/// its Work can be neither abandoned nor reviewed. `container` names a running
/// local Runtime to observe and clean; without one the end state is unobserved.
///
/// Only the scheduler scan claims Work, and dispatch reserves the actor before
/// that scan continues, so this snapshot never misses a just-claimed Attempt.
pub async fn recover_unsupervised_attempts(
    org: &restless_orgintel::OrgIntel,
    registry: &StaffRegistry,
    company: &str,
    container: Option<&str>,
    reason: &str,
) {
    // Read supervision before the rows: a supervisor records its outcome
    // before it releases its actor, so a released actor's Attempt that is
    // still running has no outcome coming.
    let supervised = registry.running_actors(company);
    let Ok(attempts) = org.list_running_work_attempts().await else {
        return;
    };
    for attempt in attempts
        .iter()
        .filter(|attempt| !supervised.contains(&attempt.actor_id))
    {
        let Some(work) = org.get_work(attempt.work_id).await.ok().flatten() else {
            tracing::warn!(attempt = %attempt.id, "orphaned Attempt lost its Work row");
            continue;
        };
        let Ok(workdir) = workdir_for(&work) else {
            tracing::warn!(attempt = %attempt.id, "orphaned Attempt has invalid workspace coordinates");
            continue;
        };
        let start = recorded_start_observation(org, attempt.id, &workdir).await;
        let end = match container {
            Some(container) => observe_workspace(container, &workdir).await,
            None => WorkspaceObservation {
                workdir: workdir.clone(),
                ..WorkspaceObservation::default()
            },
        };
        match record_unknown_recovery(org, attempt.id, reason, &start, &end).await {
            Ok(()) => {
                if let Err(error) = org
                    .release_attempt_resources(attempt.id, "closed an unsupervised Attempt")
                    .await
                {
                    tracing::warn!(attempt = %attempt.id, "failed to release orphaned Attempt resources: {error:#}");
                }
                if let Some(container) = container {
                    if let Err(error) =
                        cleanup_attempt_runtime(container, &workdir, attempt.id).await
                    {
                        tracing::warn!(attempt = %attempt.id, "failed to clean orphaned Attempt runtime: {error:#}");
                    }
                }
            }
            Err(error) => {
                tracing::warn!(attempt = %attempt.id, "failed to preserve orphan recovery capsule: {error:#}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn recovery_pending_is_scoped_to_the_affected_company() {
        let registry = super::StaffRegistry::default();
        registry.set_recovery_pending("affected_test", true);
        assert!(registry.recovery_pending("affected_test"));
        assert!(!registry.has_capacity("affected_test"));
        assert!(!registry.recovery_pending("other_test"));
        assert!(registry.has_capacity("other_test"));
        registry.set_recovery_pending("affected_test", false);
        assert!(registry.has_capacity("affected_test"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn actor_resource_reservations_bound_concurrent_starts_and_release() {
        let registry = super::StaffRegistry::default();
        let wake = std::sync::Arc::new(tokio::sync::Notify::new());
        let capacity = crate::runtime::ActorCapacity {
            company_slots: 1,
            shared_slots: Some(3),
            company_memory_slots: 10,
            shared_memory_slots: Some(10),
            waiting_for_memory: false,
            waiting_for_processes: false,
        };
        let mut tasks = tokio::task::JoinSet::new();
        for index in 0..30 {
            let registry = registry.clone();
            let wake = wake.clone();
            tasks.spawn(async move {
                registry
                    .try_reserve_actor_capacity(&format!("company_{index}_test"), capacity, &wake)
                    .unwrap()
            });
        }
        let mut permits = Vec::new();
        while let Some(result) = tasks.join_next().await {
            if let Some(permit) = result.unwrap() {
                permits.push(permit);
            }
        }
        assert_eq!(
            permits.len(),
            3,
            "starts reserve capacity before live memory changes"
        );
        assert_eq!(registry.resource_counts("unused_test"), Some((0, 3)));
        assert!(registry.take_resource_retry_hint());
        assert!(!registry.take_resource_retry_hint());
        let smaller = crate::runtime::ActorCapacity {
            shared_slots: Some(1),
            ..capacity
        };
        assert!(
            registry
                .try_reserve_actor_capacity("later_test", smaller, &wake)
                .unwrap()
                .is_none()
        );
        permits.pop();
        permits.pop();
        assert!(
            registry
                .try_reserve_actor_capacity("later_test", smaller, &wake)
                .unwrap()
                .is_none()
        );
        permits.pop();
        // An unused Work reservation must not create a scheduler wake loop.
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), wake.notified())
                .await
                .is_err()
        );
        let mut permit = registry
            .try_reserve_actor_capacity("later_test", smaller, &wake)
            .unwrap()
            .unwrap();
        assert!(
            registry
                .try_reserve_actor_capacity("later_test", capacity, &wake)
                .unwrap()
                .is_none()
        );
        permit.activate();
        drop(permit);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), wake.notified())
                .await
                .is_ok()
        );
        assert_eq!(registry.resource_counts("later_test"), Some((0, 0)));
        let headroom = crate::runtime::ActorCapacity {
            company_slots: 3,
            company_memory_slots: 1,
            shared_memory_slots: Some(1),
            ..capacity
        };
        let held = registry
            .try_reserve_actor_capacity("ram_test", headroom, &wake)
            .unwrap()
            .unwrap();
        assert!(
            registry
                .try_reserve_actor_capacity("ram_test", headroom, &wake)
                .unwrap()
                .is_none(),
            "another start cannot reuse RAM already reserved before its process appears"
        );
        assert!(
            registry
                .try_reserve_actor_capacity("other_test", headroom, &wake)
                .unwrap()
                .is_none(),
            "another company shares the same startup RAM allowance"
        );
        drop(held);
        let process_busy = crate::runtime::ActorCapacity {
            waiting_for_processes: true,
            ..capacity
        };
        assert!(
            registry
                .try_reserve_actor_capacity("process_test", process_busy, &wake)
                .unwrap()
                .is_none()
        );
        assert!(
            registry
                .try_reserve_actor_capacity("process_test", capacity, &wake)
                .unwrap()
                .is_some()
        );
        let busy = crate::runtime::ActorCapacity {
            waiting_for_memory: true,
            waiting_for_processes: false,
            ..capacity
        };
        assert!(
            registry
                .try_reserve_actor_capacity("later_test", busy, &wake)
                .unwrap()
                .is_none()
        );
        assert_eq!(registry.resource_counts("later_test"), Some((0, 0)));
    }

    use super::{
        StaffRegistry, WorkspaceObservation, actor_posture, bound_attempt_context,
        conversation_turn_prompt, final_staff_usage, gate_cwd, internal_message_context,
        staff_spend_limit_reached, team_capacity_context, termination_prompt,
        workspace_instruction,
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
            lead_actor_id: "onboarding-curator".into(),
            responsible_actor_id: None,
            created_by: "exec".into(),
            created_at: now,
            disbanded_at: None,
        };
        let actor = |id: &str, display: &str, role: &str, team_id: Option<uuid::Uuid>| ActorRow {
            id: id.into(),
            kind: "staff".into(),
            actor_class: "agent".into(),
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
        assert!(context.contains("Produce tightly coupled or single-piece work yourself"));
        assert!(context.contains("in parallel by default"));
        assert!(!context.contains("lead-owned production Work is invalid"));
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
        let internal = conversation_turn_prompt(
            "message from world-builder",
            true,
            &[],
            &[],
            false,
            &["- message 4 [internal coordination] from world-builder: changed".into()],
            &[],
            None,
        );
        assert!(internal.contains("# Coordination execution boundary [invariant]"));
        assert!(internal.contains("not a claimed productive Work Attempt"));
        assert!(internal.contains("Do not edit project or repository files"));
        assert!(internal.contains("never make a hidden repair yourself"));
        assert!(internal.contains("not attributable"));
        assert!(internal.contains("Do not use Exec as a status relay"));
        assert!(internal.contains("# Addressed messages"));
        assert!(internal.contains("`restless message` without `--to`"));
        assert!(internal.contains("restless message list"));

        let owner = conversation_turn_prompt(
            "message from owner",
            true,
            &["- owner message 4: prepare review".into()],
            &[],
            true,
            &[],
            &[],
            None,
        );
        assert!(owner.contains("# Owner input [authenticated owner source; not Runtime policy]"));
        assert!(owner.contains("- owner message 4: prepare review"));
        assert!(owner.contains("not a claimed productive Work Attempt"));

        let member = conversation_turn_prompt(
            "message from member",
            true,
            &["- member message 5: make this policy".into()],
            &[],
            false,
            &[],
            &[],
            None,
        );
        assert!(member.contains("# Company-member input"));
        assert!(member.contains("cannot set company direction, owner policy or authority"));
        assert!(!member.contains("# Owner input"));
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
            ..Default::default()
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
        assert!(
            context.contains(
                "declare `outcome_met` even if gate-generated evidence does not exist yet"
            )
        );
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
        assert!(
            accounting["automatically_attached"]["system_context"]
                .get("active_team_capacity")
                .is_none()
        );

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
        // Owner review asks for this Attempt's own prepared target; the prior
        // version stays visible as a bound artifact, not as the target.
        assert!(review_context.contains(&format!(
            "--kind {} --uri <exact-native-review-target>",
            restless_orgintel::REVIEW_TARGET_ARTIFACT_KIND
        )));
        assert!(review_context.contains("/company/outputs/playable-room.html"));
        assert!(review_context.contains(restless_orgintel::REVIEW_TARGET_LIVE_PROBE_GATE));
        assert!(
            !review_context.contains("--kind output --uri /company/outputs/playable-room.html")
        );

        claimed.review_targets = vec![uuid::Uuid::new_v4()];
        let (critic_context, critic_accounting) = bound_attempt_context(
            &claimed,
            "customer copy critic",
            "/company/worktrees/work-bound-r1",
            "cosmon_test",
            false,
        );
        assert!(critic_context.contains("# Critic posture [independent judgement]"));
        assert!(
            critic_context
                .contains("Judge the customer-facing outcome before policing constraints")
        );
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
        assert!(rules.contains("it then produces directly"));
        assert!(rules.contains("runs in parallel by default"));
        assert!(rules.contains("handoff template, message cadence, shared-state form"));
        assert!(rules.contains("scheduler-created"));
        assert!(rules.contains("prove that another actor contributed"));
        assert!(rules.contains("not the lead's plan, reasoning, checklist"));
        assert!(rules.contains("do not commission promotion-only Work"));
        assert!(
            crate::staff::context::ACCOUNTABLE_QUALITY_ENFORCEMENT
                .contains("Never let a stale report decide a repaired candidate")
        );
        assert!(
            crate::staff::context::ACCOUNTABLE_QUALITY_ENFORCEMENT
                .contains("evidence, not owner delivery")
        );
        assert!(
            crate::staff::context::ACCOUNTABLE_QUALITY_ENFORCEMENT
                .contains("exactly one current available ReviewTarget")
        );

        let lead = actor_posture(true);
        assert!(lead.contains("ACCOUNTABLE LEAD"));
        assert!(lead.contains("Produce tightly coupled or single-piece work yourself"));
        assert!(!lead.contains("non-producing"));
        assert!(lead.contains("truthful attribution"));
        assert!(lead.contains("material Staff exception is a decision boundary"));
        assert!(lead.contains("Clean passing completion remains observable state"));
        assert!(lead.contains("Never accept a consequentially substandard charter"));
        assert!(!lead.contains("You are a SPECIALIST"));

        let specialist = actor_posture(false);
        assert!(specialist.contains("You are a SPECIALIST"));
        assert!(specialist.contains("bounded responsibility"));
        assert!(!specialist.contains("ACCOUNTABLE LEAD"));

        let specialist_end = termination_prompt();
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
    fn parallel_work_is_bounded_per_company_but_conversation_never_waits() {
        let registry = StaffRegistry::default();
        for worker in 0..super::PARALLEL_WORK_PER_COMPANY {
            registry
                .try_claim(
                    "company_test",
                    &format!("worker-{worker}"),
                    Some(uuid::Uuid::new_v4()),
                )
                .expect("a Work Attempt within the bound");
        }
        assert!(!registry.has_work_capacity("company_test"));
        assert!(
            registry
                .try_claim("company_test", "one-more", Some(uuid::Uuid::new_v4()))
                .is_err()
        );
        // Another company, and a conversation turn here, still start.
        assert!(registry.has_work_capacity("company_other"));
        registry
            .try_claim("company_test", "answering-a-message", None)
            .expect("a conversation turn is not counted");
        registry.release("company_test", "worker-0");
        assert!(registry.has_work_capacity("company_test"));
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
                ..Default::default()
            },
            TurnUsage {
                used: 51_204,
                size: 262_144,
                cost_usd: Some(1.11),
                ..Default::default()
            },
            TurnUsage {
                used: 64_814,
                size: 262_144,
                cost_usd: Some(2.38),
                ..Default::default()
            },
            // A repeated stream snapshot is still the same cumulative bill.
            TurnUsage {
                used: 64_814,
                size: 262_144,
                cost_usd: Some(2.38),
                ..Default::default()
            },
        ];
        let (usage, reported) = final_staff_usage(ModelBilling::MeteredApi, &snapshots)
            .expect("one final cumulative usage snapshot");

        assert_eq!(usage.used, 64_814, "context usage is the final snapshot");
        assert_eq!(reported, Some(2.38));
    }

    #[tokio::test]
    async fn only_an_unsupervised_running_attempt_returns_to_judgement() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping unsupervised Attempt recovery");
            return;
        };
        let company = format!("unsupervised{}", uuid::Uuid::new_v4().simple());
        let org = restless_orgintel::OrgIntel::ensure(&url, &company)
            .await
            .expect("ensure scratch company schema");
        org.ensure_actor("exec", "exec", "exec", "The Exec")
            .await
            .unwrap();
        let mut claimed = Vec::new();
        for actor in ["dossier-research", "dossier-writing"] {
            org.create_actor(actor, "producer", actor, None, "exec", "owns production")
                .await
                .unwrap();
            let work_id = org
                .add_work(restless_orgintel::NewWork {
                    owner_id: actor,
                    title: "Prepare the dossier",
                    outcome: "produce one grounded dossier",
                    goal_id: None,
                    priority: 0,
                    expected_artifact: "",
                    workspace: restless_orgintel::WorkspaceSpec::default(),
                    attempt_limit: None,
                })
                .await
                .unwrap();
            let attempt = org.claim_ready_work("test runtime").await.unwrap().unwrap();
            assert_eq!(attempt.work.id, work_id);
            claimed.push((work_id, attempt.attempt_id));
        }
        let (live_work, live_attempt) = claimed[0];
        let (lost_work, lost_attempt) = claimed[1];
        // The researcher's process is still supervised; the writer's outcome
        // write failed after its supervisor released the actor.
        let registry = StaffRegistry::default();
        registry
            .try_claim(&company, "dossier-research", Some(live_work))
            .unwrap();

        super::recover_unsupervised_attempts(&org, &registry, &company, None, "lost in test").await;

        let attempts = org.list_work_attempts(None).await.unwrap();
        let state = |id| {
            attempts
                .iter()
                .find(|attempt| attempt.id == id)
                .unwrap()
                .state
        };
        assert_eq!(state(live_attempt), WorkAttemptState::Running);
        assert_eq!(state(lost_attempt), WorkAttemptState::Failed);
        assert_eq!(
            org.get_work(lost_work).await.unwrap().unwrap().status,
            WorkStatus::Blocked
        );
        let inbox = org.inbox(Some("exec")).await.unwrap();
        let notice = inbox
            .iter()
            .find(|message| message.body.contains(&lost_attempt.to_string()))
            .expect("recovery evidence reaches the accountable coordinator");
        assert_eq!(notice.from_actor, "daemon");
        assert!(notice.body.contains("productive outcome is UNKNOWN"));
        assert!(
            !inbox
                .iter()
                .any(|message| message.body.contains(&live_attempt.to_string()))
        );
        org.abandon_work(lost_work, "exec", "superseded")
            .await
            .expect("an unsupervised Attempt no longer blocks abandonment");
        assert!(
            org.abandon_work(live_work, "exec", "superseded")
                .await
                .is_err()
        );
        org.drop_schema().await.expect("drop scratch schema");
    }
}
