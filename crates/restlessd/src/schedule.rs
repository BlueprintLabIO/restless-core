//! Durable scheduling from OrgIntel facts.
//!
//! Conversation wakes stay free-form. Machine work has one deterministic
//! kickoff: atomically claim the next ready Work node, create its Attempt,
//! then supervise its actor. Time conditions live in `schedules`, not event
//! prose, and every notification is merely a hint to reread canonical rows.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use restless_orgintel::{OrgIntel, WorkAttemptState};
use tokio_util::sync::CancellationToken;

use crate::runtime::{self, CompanyConfig, ContainerStatus};
use crate::{exec, Daemon};

/// LISTEN/NOTIFY, the exact next-due timer and the OS wake entry are the normal
/// sources. This slow sweep repairs a lost listener or newly added cell; it is
/// deliberately not the schedule engine.
const REPAIR_SWEEP_INTERVAL: Duration = Duration::from_secs(300);

/// Free-form Exec conversation liveness. Work custody is the running Attempt.
pub(crate) type InFlight = Arc<Mutex<WakeClaims>>;

#[derive(Default)]
pub(crate) struct WakeClaims {
    active: HashMap<String, CancellationToken>,
    pending: HashMap<String, String>,
    /// Ordinary supervision backoff after a wake that could not run. Owed work
    /// is now re-derived from durable rows on every five-second scan, so a
    /// company whose substrate or provider is down would otherwise be retried
    /// every five seconds forever. The old timestamp watermark suppressed that
    /// by accident — and suppressed real owed work with it (S19-T1). This is
    /// in-memory on purpose: a daemon restart is itself a reason to try once
    /// more, and nothing here is organisational truth.
    backoff: HashMap<String, (std::time::Instant, u32)>,
}

const BACKOFF_FIRST: Duration = Duration::from_secs(30);
const BACKOFF_CEILING: Duration = Duration::from_secs(300);

impl WakeClaims {
    pub(crate) fn claim(&mut self, company: &str) -> bool {
        self.claim_with_cancellation(company).is_some()
    }

    pub(crate) fn claim_with_cancellation(&mut self, company: &str) -> Option<CancellationToken> {
        if !self.active.contains_key(company) {
            self.pending.remove(company);
            let cancellation = CancellationToken::new();
            self.active
                .insert(company.to_string(), cancellation.clone());
            Some(cancellation)
        } else {
            None
        }
    }

    pub(crate) fn is_active(&self, company: &str) -> bool {
        self.active.contains_key(company)
    }

    pub(crate) fn active_companies(&self) -> Vec<String> {
        let mut companies = self.active.keys().cloned().collect::<Vec<_>>();
        companies.sort();
        companies
    }

    fn queue(&mut self, company: &str, reason: &str) {
        self.pending.insert(company.to_string(), reason.to_string());
    }

    /// Whether this company's next automatic wake is still held back by a
    /// previous wake that never ran.
    fn is_backing_off(&self, company: &str) -> bool {
        self.backoff
            .get(company)
            .is_some_and(|(until, _)| std::time::Instant::now() < *until)
    }

    /// A wake that returned `Blocked` or failed outright. It delivered nothing
    /// and consumed nothing, so the owed facts still hold; hold the next
    /// automatic attempt instead of spinning on them.
    fn record_unusable_wake(&mut self, company: &str) {
        let failures = self
            .backoff
            .get(company)
            .map_or(1, |(_, failures)| failures.saturating_add(1));
        let delay = BACKOFF_FIRST
            .saturating_mul(1u32 << failures.saturating_sub(1).min(4))
            .min(BACKOFF_CEILING);
        self.backoff.insert(
            company.to_string(),
            (std::time::Instant::now() + delay, failures),
        );
    }

    fn record_usable_wake(&mut self, company: &str) {
        self.backoff.remove(company);
    }

    fn release(&mut self, company: &str) {
        self.active.remove(company);
    }

    pub(crate) fn interrupt(&mut self, company: &str) -> bool {
        self.active.get(company).is_some_and(|cancellation| {
            cancellation.cancel();
            true
        })
    }

    fn take_ready(&mut self) -> Vec<(String, String)> {
        let ready = self
            .pending
            .keys()
            .filter(|company| !self.active.contains_key(*company) && !self.is_backing_off(company))
            .cloned()
            .collect::<Vec<_>>();
        ready
            .into_iter()
            .filter_map(|company| {
                self.pending
                    .remove(&company)
                    .map(|reason| (company, reason))
            })
            .collect()
    }
}

pub(crate) struct WakeGuard {
    company: String,
    in_flight: InFlight,
}

impl WakeGuard {
    pub(crate) fn new(company: &str, in_flight: &InFlight) -> Self {
        Self {
            company: company.to_string(),
            in_flight: Arc::clone(in_flight),
        }
    }
}

impl Drop for WakeGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.in_flight.lock() {
            guard.release(&self.company);
        }
    }
}

pub async fn run(daemon: Arc<Daemon>) {
    let in_flight = Arc::clone(&daemon.in_flight);
    // The daemon owns one listener per cell and fans body-free hints out to
    // scheduler and realtime consumers. This receiver can lag or reconnect;
    // durable schedule rows, not this channel, remain the authority.
    let mut inbox = daemon.cell_wakes.subscribe();
    ensure_cell_listeners(&daemon).await;
    scan_all_companies(&daemon, &in_flight).await;
    loop {
        let next_due = next_due_delay(&daemon).await;
        tokio::select! {
            _ = tokio::time::sleep(next_due) => {
                // A company created since boot needs its own listener; this is
                // also where a cell whose listener never started is retried.
                ensure_cell_listeners(&daemon).await;
                fire_pending(&daemon, &in_flight).await;
                scan_all_companies(&daemon, &in_flight).await;
            }
            _ = daemon.schedule_wake.notified() => {
                tracing::info!("native schedule wake observed; reconciling durable due state");
                ensure_cell_listeners(&daemon).await;
                fire_pending(&daemon, &in_flight).await;
                scan_all_companies(&daemon, &in_flight).await;
            }
            wake = inbox.recv() => {
                match wake {
                    Ok(wake) => {
                        // Wake delivery crosses a process and database boundary,
                        // so make arrival observable without logging user content.
                        tracing::debug!(
                            company = wake.company,
                            kind = wake.kind.as_deref().unwrap_or("malformed"),
                            "cell wake received"
                        );
                        handle_notification(&daemon, &in_flight, &wake.raw).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(skipped, "scheduler cell-wake receiver lagged; repairing from durable state");
                        scan_all_companies(&daemon, &in_flight).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                }
            }
        }
    }
}

/// Ensure the shared hub owns a listener for every configured cell.
async fn ensure_cell_listeners(daemon: &Arc<Daemon>) {
    let Some(companies) = configured_companies(daemon) else {
        tracing::warn!("cannot read configured companies; preserving existing cell listeners");
        return;
    };
    daemon.cell_wakes.retain_companies(&companies);
    for company in companies {
        let url = match daemon.orgintel.cell_database_url(&company).await {
            Ok(url) => url,
            Err(error) => {
                tracing::warn!(
                    company,
                    "cannot reach this cell to listen for wakes: {error:#}"
                );
                continue;
            }
        };
        daemon.cell_wakes.ensure_company(&company, &url);
    }
}

/// Companies configured in this plane, by config file.
fn configured_companies(daemon: &Arc<Daemon>) -> Option<Vec<String>> {
    let Ok(entries) = std::fs::read_dir(daemon.root.join("companies")) else {
        return None;
    };
    let mut companies = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        if let Some(company) = path.file_stem().and_then(|value| value.to_str()) {
            companies.push(company.to_string());
        }
    }
    companies.sort();
    Some(companies)
}

async fn handle_notification(daemon: &Arc<Daemon>, in_flight: &InFlight, payload: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        tracing::warn!(payload, "unparseable OrgIntel notification");
        return;
    };
    let Some(company) = value["company"].as_str() else {
        return;
    };
    match value["kind"].as_str() {
        Some("mention") if value["body"]["to"] == "exec" => {
            fire_exec(
                daemon,
                in_flight,
                company,
                "a focused Room mention is owed to the Exec",
            )
            .await;
        }
        Some("mention") if value["body"]["to"].as_str().is_some() => {
            scan_company(daemon, in_flight, company).await;
        }
        Some("message") if value["body"]["to"] == "exec" => {
            // A Work owned by the singleton Exec has no higher internal
            // coordinator. Preserve a self-addressed note as transcript, but
            // never turn it into a second free-form Exec process.
            if is_exec_self_message(&value) {
                return;
            }
            let message_id = value["body"]["message_id"].as_i64();
            let routes_through_work = match message_id {
                Some(message_id) => match daemon.orgintel.get(company).await {
                    Ok(org) => org
                        .message_is_work_attempt_input(message_id)
                        .await
                        .unwrap_or(false),
                    Err(_) => false,
                },
                None => false,
            };
            if routes_through_work {
                // Request-changes writes its feedback message and reactivates
                // the Work in one transaction. The message notification may
                // arrive before work_changed; claiming the Attempt here keeps
                // those two hints from starting two Exec sessions.
                scan_company(daemon, in_flight, company).await;
                return;
            }
            fire_exec(
                daemon,
                in_flight,
                company,
                &format!(
                    "message from {}",
                    value["body"]["from"].as_str().unwrap_or("unknown")
                ),
            )
            .await;
        }
        Some("message") if value["body"]["to"].as_str().is_some() => {
            // Member/owner mail to a lead is an owed coordination condition.
            // Work-linked feedback is filtered by the actor dispatcher and
            // remains graph input rather than racing a conversation session.
            // Ordinary Work feedback is queued and delivered by the active
            // session at its next safe checkpoint. Cancellation is a separate
            // authority-bearing `work interrupt` operation.
            scan_company(daemon, in_flight, company).await;
        }
        Some("work_changed" | "artifact_linked" | "handoff_changed" | "schedule_changed") => {
            scan_company(daemon, in_flight, company).await;
        }
        _ => {}
    }
}

async fn next_due_delay(daemon: &Arc<Daemon>) -> Duration {
    let now = chrono::Utc::now();
    let mut earliest = None;
    for company in configured_companies(daemon).unwrap_or_default() {
        let Ok(org) = daemon.orgintel.get(&company).await else {
            continue;
        };
        if let Ok(Some(next)) = org.next_schedule_due_at().await {
            earliest = Some(
                earliest.map_or(next, |current: chrono::DateTime<chrono::Utc>| {
                    current.min(next)
                }),
            );
        }
    }
    match earliest {
        Some(due) if due <= now => Duration::from_millis(50),
        Some(due) => (due - now)
            .to_std()
            .unwrap_or(Duration::from_millis(50))
            .min(REPAIR_SWEEP_INTERVAL),
        None => REPAIR_SWEEP_INTERVAL,
    }
}

fn is_exec_self_message(value: &serde_json::Value) -> bool {
    value["body"]["to"] == "exec" && value["body"]["from"] == "exec"
}

async fn scan_all_companies(daemon: &Arc<Daemon>, in_flight: &InFlight) {
    for company in configured_companies(daemon).unwrap_or_default() {
        scan_company(daemon, in_flight, &company).await;
    }
}

async fn scan_company(daemon: &Arc<Daemon>, in_flight: &InFlight, company: &str) {
    // Hold one admission lease across observation, claim and dispatch. A
    // replacement therefore sees either this scan or the actor it launched,
    // never a false idle gap between them.
    let Some(_lifecycle_lease) = daemon.lifecycle.try_enter() else {
        return;
    };
    let Ok(config) = CompanyConfig::load(&daemon.root, company) else {
        return;
    };
    if daemon.runtime_bridges.is_hosted() {
        let Ok(identity) =
            crate::runtime_bridge::expected_identity(&daemon.authority, company).await
        else {
            return;
        };
        if daemon.runtime_bridges.readiness(&identity)
            != crate::runtime_bridge::BridgeReadiness::Ready
        {
            return;
        }
    } else if !matches!(runtime::status(company).await, Ok(ContainerStatus::Running)) {
        return;
    }
    let Ok(org) = daemon.orgintel.get(company).await else {
        return;
    };

    if !daemon.runtime_bridges.is_hosted() {
        if let Err(error) =
            crate::staff::reconcile_execution_substrate(&org, &runtime::container_name(company))
                .await
        {
            tracing::warn!(
                company,
                "could not reconcile exact execution substrate: {error:#}"
            );
        }
    }

    // Live Attempt completion flushes this outbox immediately. A daemon crash
    // between terminal state and supervisor delivery leaves the owed bit set;
    // reconciliation recreates the same durable lead wake without guessing
    // completion from elapsed time or replaying production.
    if let Err(error) = org.flush_terminal_supervisor_notices(100).await {
        tracing::warn!(
            company,
            "could not flush terminal supervisor facts: {error:#}"
        );
    }

    // A daemon/process restart can cut a direct CLI wake after the durable
    // `wake` event but before `wake_end`. That work is still owed even when no
    // unread owner message remains. Recover it before claiming new schedules
    // or Work so the singleton Exec resumes one company-level thread at a
    // time. During a healthy live turn the in-flight claim suppresses this.
    if recover_interrupted_exec_wake(daemon, in_flight, &org, company).await {
        return;
    }

    if let Ok(schedules) = org.claim_due_schedules().await {
        for schedule in schedules {
            if schedule.actor_id == "exec" {
                fire_exec(
                    daemon,
                    in_flight,
                    company,
                    &format!("scheduled: {}", schedule.reason),
                )
                .await;
            }
        }
    }

    // What the Exec is owed is a durable fact about the exact owed thing, not a
    // comparison against the newest wake event. The Exec is the only actor that
    // can put an ordinary judgement in front of the owner, so a lost trigger
    // here is a lost owner attention item — which is exactly what the old
    // watermark produced: one unrelated wake moved `latest_event_at("wake")`
    // past a pending handoff and it never triggered again, and a lead's message
    // to the Exec had no durable recovery path at all (S19-T1).
    //
    // Both conditions below terminate on their own. A message is consumed by
    // `mark_read` and a judgement by `mark_handoffs_delivered`, and only a turn
    // that actually ran writes either. A wake that never assembled context
    // delivers nothing and is bounded by the failure backoff instead.
    // An owner decision that Authority recorded but the company was never told
    // about leaves Work blocked on a question the owner already answered. This
    // is idempotent on the exact Authority record id, so the ordinary case —
    // already announced by the owner action itself — costs one indexed read.
    crate::approval::announce_decisions(company, &daemon.authority, Some(&org)).await;

    recover_owed_exec_work(daemon, in_flight, &org, company).await;

    // Team judgement and owner-to-lead conversation take precedence over
    // launching more member Work. Otherwise a full staff cap can starve the
    // coordinator that must repair what is already blocked.
    if let Ok(teams) = org.list_teams().await {
        for team in teams {
            if !daemon.staff.has_capacity(company) {
                break;
            }
            match crate::staff::dispatch_actor_conversation(
                &config,
                &org,
                crate::staff::ConversationRuntime {
                    spend: &daemon.spend,
                    authority: &daemon.authority,
                    capabilities: &daemon.capabilities,
                    registry: &daemon.staff,
                    activities: &daemon.activities,
                    runtime_bridges: &daemon.runtime_bridges,
                },
                &team.lead_actor_id,
                "addressed message or team judgement became ready",
            )
            .await
            {
                Ok(_) => {}
                Err(error) => tracing::warn!(
                    company,
                    actor = %team.lead_actor_id,
                    "could not wake team lead: {error:#}"
                ),
            }
        }
    }

    // A structured direct mention addresses the durable Actor, not the team
    // office it happened to hold at send time. Replacement/disband reroutes
    // office mail and judgements, but an active former lead still answers its
    // already-accepted named obligation. The Actor-wide lease below prevents
    // this recovery scan from duplicating a current-lead wake.
    if let Ok(actors) = org.actors_owing_message_mentions(128).await {
        for actor in actors {
            if !daemon.staff.has_capacity(company) {
                break;
            }
            match crate::staff::dispatch_actor_conversation(
                &config,
                &org,
                crate::staff::ConversationRuntime {
                    spend: &daemon.spend,
                    authority: &daemon.authority,
                    capabilities: &daemon.capabilities,
                    registry: &daemon.staff,
                    activities: &daemon.activities,
                    runtime_bridges: &daemon.runtime_bridges,
                },
                &actor,
                "a durable named-Actor Room mention remains owed",
            )
            .await
            {
                Ok(_) => {}
                Err(error) => tracing::warn!(
                    company,
                    actor = %actor,
                    "could not wake mentioned Actor: {error:#}"
                ),
            }
        }
    }

    while daemon.staff.has_capacity(company) {
        // Conversation turns and Work Attempts share one supervised actor.
        // Exclude the registry snapshot before the database claim so a busy
        // actor's ready node stays untouched rather than being claimed and
        // then misclassified as a failed Attempt by dispatch.
        // The singleton Exec also has a free-form wake path outside the Staff
        // registry. Work created during that wake must wait for it to finish;
        // otherwise one durable Exec runs two ACP processes concurrently.
        let exec_waking = in_flight
            .lock()
            .expect("in-flight guard")
            .is_active(company);
        let busy_actors = actor_exclusions(daemon.staff.running_actors(company), exec_waking);
        let claimed = match org
            .claim_ready_work_excluding("dependency graph became ready", &busy_actors)
            .await
        {
            Ok(Some(claimed)) => claimed,
            Ok(None) => break,
            Err(error) => {
                tracing::warn!(company, "could not claim ready Work: {error:#}");
                break;
            }
        };
        let attempt_id = claimed.attempt_id;
        if let Err(error) = crate::staff::dispatch_claimed_work(
            &config,
            &daemon.spend,
            &daemon.authority,
            &daemon.capabilities,
            &org,
            &daemon.staff,
            &daemon.activities,
            &daemon.runtime_bridges,
            claimed,
        )
        .await
        {
            let reason = format!("runtime refused claimed Work: {error:#}");
            let _ = org
                .finish_work_attempt(attempt_id, WorkAttemptState::Failed, &reason)
                .await;
            tracing::warn!(company, "{reason}");
        }
    }
}

/// Re-derive what the singleton Exec is owed from durable rows, every scan.
///
/// Two facts, both self-consuming: unread conversation addressed to the Exec
/// (any sender — a lead reporting a prepared outcome is exactly the case the
/// old owner-only filter dropped), and pending judgement assigned to the Exec
/// that no completed turn has been given. Neither is inferred from a wake
/// timestamp, so an unrelated wake, a restart with no `NOTIFY`, and a lost
/// in-memory follow-up all leave the owed fact intact and still triggering.
async fn recover_owed_exec_work(
    daemon: &Arc<Daemon>,
    in_flight: &InFlight,
    org: &OrgIntel,
    company: &str,
) {
    let wake_active = in_flight
        .lock()
        .map(|claims| claims.is_active(company))
        .unwrap_or(true);
    if wake_active || daemon.staff.is_actor_running(company, "exec") {
        return;
    }

    let judgements = org.undelivered_handoff_count("exec").await.unwrap_or(0);
    let conversation = org.owed_conversation_count("exec").await.unwrap_or(0);
    let mention = org
        .next_pending_message_mention("exec")
        .await
        .ok()
        .flatten()
        .is_some();
    let reason = match (judgements, conversation, mention) {
        (0, 0, false) => return,
        (0, 0, true) => "a focused Room mention is owed to the Exec",
        (0, _, _) => "unread conversation is owed to the Exec",
        (_, 0, false) => "organisational judgement is owed to the Exec",
        _ => "unread conversation and organisational judgement are owed to the Exec",
    };
    fire_exec(daemon, in_flight, company, reason).await;
}

async fn recover_interrupted_exec_wake(
    daemon: &Arc<Daemon>,
    in_flight: &InFlight,
    org: &OrgIntel,
    company: &str,
) -> bool {
    if in_flight
        .lock()
        .expect("in-flight guard")
        .is_active(company)
    {
        return false;
    }
    let Ok(latest_wake) = org.latest_event("wake").await else {
        return false;
    };
    let Ok(latest_wake_end) = org.latest_event("wake_end").await else {
        return false;
    };
    if !exec_wake_is_interrupted(
        latest_wake.as_ref().map(|event| event.id),
        latest_wake_end.as_ref().map(|event| event.id),
    ) {
        return false;
    }

    let original = latest_wake
        .as_ref()
        .and_then(|event| event.body.get("reason"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown trigger")
        .chars()
        .take(500)
        .collect::<String>();
    fire_exec(
        daemon,
        in_flight,
        company,
        &format!(
            "recovering an interrupted Exec wake after Runtime Bridge restart; \
             rehydrate durable work and reconcile any material effect before retrying. \
             Original trigger: {original}"
        ),
    )
    .await;
    true
}

fn exec_wake_is_interrupted(latest_wake_id: Option<i64>, latest_wake_end_id: Option<i64>) -> bool {
    latest_wake_id.is_some_and(|wake| latest_wake_end_id.is_none_or(|end| wake > end))
}

fn actor_exclusions(mut running: Vec<String>, exec_waking: bool) -> Vec<String> {
    if exec_waking && !running.iter().any(|actor| actor == "exec") {
        running.push("exec".into());
    }
    running
}

/// Run one Exec turn through the same conversation boundary regardless of
/// whether a message notification, restart reconciliation, schedule, or the
/// operator CLI initiated it. OrgIntel remains the durable transcript; the
/// in-memory projection exists only while this call owns the ACP process.
pub(crate) async fn run_exec_turn(
    daemon: &Daemon,
    config: &CompanyConfig,
    org: &OrgIntel,
    reason: &str,
    cancellation: &CancellationToken,
) -> Result<exec::WakeReport> {
    // The lease row is FK-bound to the durable Actor. Fresh companies may
    // reach this entry point before `exec::wake` has ever bootstrapped it, so
    // initialise the stable principals before competing for the mutex.
    org.ensure_actor_with_model("exec", "exec", "exec", "The Exec", Some(&config.model))
        .await?;
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await?;
    let Some(lease_guard) =
        crate::staff::CognitiveLeaseGuard::claim(org, "exec", cancellation).await?
    else {
        anyhow::bail!("Exec already has a durable cognitive session or running Work Attempt");
    };
    let outcome =
        run_exec_turn_with_lease(daemon, config, org, reason, cancellation, &lease_guard).await;
    lease_guard.finish().await;
    outcome
}

async fn run_exec_turn_with_lease(
    daemon: &Daemon,
    config: &CompanyConfig,
    org: &OrgIntel,
    reason: &str,
    cancellation: &CancellationToken,
    lease_guard: &crate::staff::CognitiveLeaseGuard,
) -> Result<exec::WakeReport> {
    let conversation_inbox = org.conversation_inbox("exec").await?;
    let mut message_ids = Vec::new();
    let mut owner_message_ids = Vec::new();
    for message in &conversation_inbox {
        message_ids.push(message.id);
        if message.from_actor == "owner" {
            owner_message_ids.push(message.id);
        }
    }
    // Every pending judgement remains in the assembled context, but only an
    // as-yet-undelivered judgement outranks a focused mention or belongs in
    // this turn's atomic consumption set. A delivered-yet-pending judgement is
    // context, not a fresh delivery obligation; treating it as one would
    // starve the mention while recovery kept waking for that mention forever.
    let judgements = org.conversation_handoffs("exec").await?;
    let owed_judgements = judgements
        .iter()
        .filter(|handoff| handoff.delivered_at.is_none())
        .map(restless_orgintel::OwnerHandoffRow::conversation_input)
        .collect::<Vec<_>>();
    // Direct conversation and formal handoffs keep their existing priority.
    // Otherwise process exactly one unresolved Room mention so a single model
    // context never has to answer several unrelated Threads at once.
    let pending_mention = if message_ids.is_empty() && owed_judgements.is_empty() {
        org.claim_next_pending_message_mention(lease_guard.lease())
            .await?
    } else {
        None
    };
    let live_turn = daemon
        .activities
        .start_messages(&config.name, "exec", &owner_message_ids);
    let observer = (!owner_message_ids.is_empty()).then(|| live_turn.observer());
    let outcome = exec::wake(
        config,
        &daemon.spend,
        &daemon.authority,
        &daemon.capabilities,
        &daemon.runtime_bridges,
        org,
        reason,
        &conversation_inbox,
        &judgements,
        pending_mention.as_ref().map(|claim| &claim.context),
        observer,
        cancellation,
    )
    .await;

    if cancellation.is_cancelled() {
        if pending_mention.is_some() {
            live_turn.fail("Interrupted before the focused Room mention was answered.");
            anyhow::bail!("focused Room mention turn was interrupted before a complete answer");
        } else if !owner_message_ids.is_empty() {
            // A replacement message is normally persisted before an
            // interruption. The full-screen owner chat can also cancel the
            // exact pending message without adding synthetic prose, so do not
            // claim a fresh direction is queued when that durable input was
            // explicitly consumed.
            let replacement_is_owed =
                org.conversation_inbox("exec")
                    .await
                    .ok()
                    .is_some_and(|messages| {
                        messages.iter().any(|message| message.from_actor == "owner")
                    });
            live_turn.fail(if replacement_is_owed {
                "Interrupted by owner; new direction is queued for a fresh turn."
            } else {
                "Interrupted by owner."
            });
        }
        return outcome;
    }

    match &outcome {
        Ok(report)
            if pending_mention.is_some()
                && report.reply_complete
                && report.termination == exec::Termination::OutcomeMet =>
        {
            let mention = pending_mention
                .as_ref()
                .expect("the mention match guard proves a focused mention");
            let recorded: anyhow::Result<i64> = async {
                let reply = report.owner_reply.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("the Exec finished a mention turn without a reply")
                })?;
                lease_guard.confirm().await?;
                Ok(org
                    .reply_to_claimed_message_mention(mention, reply)
                    .await?
                    .message
                    .id)
            }
            .await;
            match recorded {
                Ok(message_id) => live_turn.complete(Some(message_id), None),
                Err(error) => {
                    tracing::warn!(
                        company = %config.name,
                        mention_id = %mention.context.mention.id,
                        "could not persist the focused Room mention reply: {error:#}"
                    );
                    live_turn.fail("Exec finished but its Room reply was not recorded.");
                    return Err(error);
                }
            }
        }
        Ok(report) if pending_mention.is_some() => {
            // A blocked turn leaves the exact mention unresolved for a later
            // retry, but its live projection must still reach a terminal fact.
            live_turn.fail(&report.reason);
            return Err(anyhow::anyhow!(
                "focused Room mention did not produce a terminal complete answer: {}",
                report.reason
            ));
        }
        Ok(report) if !owner_message_ids.is_empty() && report.reply_complete => {
            // Owner-directed CLI sends are rejected while this Actor lease is
            // present. Persist the one complete final assistant block through
            // the exact token fence; partial/refusal transcripts never consume
            // owner input even when they contain text.
            if let Some(reply) = report.owner_reply.as_deref() {
                let reply_message_id = org
                    .finalize_cognitive_conversation(
                        lease_guard.lease(),
                        Some(reply),
                        None,
                        &message_ids,
                        &owed_judgements,
                    )
                    .await?
                    .expect("an owner reply was supplied to finalization");
                live_turn.complete(Some(reply_message_id), None);
            } else {
                live_turn.fail("Exec finished without recording a reply.");
                return Err(anyhow::anyhow!(
                    "Exec completed an owner conversation without a final reply"
                ));
            }
        }
        Ok(report) if !owner_message_ids.is_empty() => {
            live_turn.fail(&report.reason);
            return Err(anyhow::anyhow!(
                "Exec owner conversation did not produce a complete usable answer: {}",
                report.reason
            ));
        }
        Ok(report) if report.reply_complete => {
            org.finalize_cognitive_conversation(
                lease_guard.lease(),
                None,
                None,
                &message_ids,
                &owed_judgements,
            )
            .await?;
        }
        Err(error) => {
            if !owner_message_ids.is_empty() || pending_mention.is_some() {
                live_turn.fail(&format!("Exec reply failed: {error:#}"));
            }
            let latest_wake = org.latest_event("wake").await.ok().flatten();
            let latest_wake_end = org.latest_event("wake_end").await.ok().flatten();
            if exec_wake_is_interrupted(
                latest_wake.as_ref().map(|event| event.id),
                latest_wake_end.as_ref().map(|event| event.id),
            ) {
                let _ = exec::record_interrupted_outcome(org, config, &format!("{error:#}")).await;
            }
        }
        _ => {}
    }
    outcome
}

/// Fire a free-form Exec conversation. It never owns or mutates Work.
async fn fire_exec(daemon: &Arc<Daemon>, in_flight: &InFlight, company: &str, reason: &str) {
    let Some(_lifecycle_lease) = daemon.lifecycle.try_enter() else {
        return;
    };
    if daemon.staff.is_actor_running(company, "exec") {
        in_flight
            .lock()
            .expect("in-flight guard")
            .queue(company, reason);
        return;
    }
    let cancellation = {
        let mut guard = in_flight.lock().expect("in-flight guard");
        // Keep the reason queued rather than dropping it: `take_ready` releases
        // it once the backoff expires, and a claimed schedule is not owed work
        // any durable row would re-derive.
        if guard.is_backing_off(company) {
            guard.queue(company, reason);
            return;
        }
        let Some(cancellation) = guard.claim_with_cancellation(company) else {
            guard.queue(company, reason);
            return;
        };
        cancellation
    };
    let daemon = Arc::clone(daemon);
    let in_flight = Arc::clone(in_flight);
    let company = company.to_string();
    let reason = reason.to_string();
    tokio::spawn(async move {
        let _guard = WakeGuard::new(&company, &in_flight);
        let outcome = async {
            let config = CompanyConfig::load(&daemon.root, &company)?;
            let org = daemon.orgintel.get(&company).await?;
            run_exec_turn(&daemon, &config, &org, &reason, &cancellation).await
        }
        .await;
        // A turn that could not run delivered nothing, so its owed facts are
        // still owed and would otherwise re-trigger on the next five-second
        // scan.
        let usable =
            matches!(&outcome, Ok(report) if report.termination != exec::Termination::Blocked);
        if let Ok(mut guard) = in_flight.lock() {
            if usable {
                guard.record_usable_wake(&company);
            } else {
                guard.record_unusable_wake(&company);
            }
        }
        if let Err(error) = outcome {
            tracing::warn!(company, "Exec conversation wake failed: {error:#}");
        }
    });
}

async fn fire_pending(daemon: &Arc<Daemon>, in_flight: &InFlight) {
    if daemon.lifecycle.is_draining() {
        return;
    }
    let ready = in_flight.lock().expect("in-flight guard").take_ready();
    for (company, reason) in ready {
        fire_exec(daemon, in_flight, &company, &reason).await;
    }
}

#[cfg(test)]
mod tests {
    use super::{actor_exclusions, exec_wake_is_interrupted, is_exec_self_message, WakeClaims};

    #[test]
    fn trigger_during_active_wake_becomes_one_follow_up() {
        let mut claims = WakeClaims::default();
        assert!(claims.claim("probe"));
        assert!(!claims.claim("probe"));
        claims.queue("probe", "first");
        claims.queue("probe", "newer");
        assert!(claims.take_ready().is_empty());
        claims.release("probe");
        assert_eq!(claims.take_ready(), vec![("probe".into(), "newer".into())]);
    }

    #[test]
    fn manual_wake_consumes_pending_continuation() {
        let mut claims = WakeClaims::default();
        assert!(claims.claim("probe"));
        claims.queue("probe", "owner message");
        claims.release("probe");
        assert!(claims.claim("probe"));
        claims.release("probe");
        assert!(claims.take_ready().is_empty());
    }

    #[test]
    fn owner_interruption_cancels_the_exact_active_exec_turn() {
        let mut claims = WakeClaims::default();
        let cancellation = claims
            .claim_with_cancellation("probe")
            .expect("claim the active turn");

        assert!(claims.interrupt("probe"));
        assert!(cancellation.is_cancelled());
        assert!(!claims.interrupt("other-company"));
    }

    #[test]
    fn a_free_form_exec_wake_excludes_exec_work_claims() {
        assert_eq!(
            actor_exclusions(vec!["delivery-build".into()], true),
            vec!["delivery-build", "exec"]
        );
        assert_eq!(actor_exclusions(vec!["exec".into()], true), vec!["exec"]);
        assert!(actor_exclusions(Vec::new(), false).is_empty());
    }

    #[test]
    fn an_exec_note_to_itself_never_creates_a_second_wake() {
        assert!(is_exec_self_message(&serde_json::json!({
            "body": { "from": "exec", "to": "exec" }
        })));
        assert!(!is_exec_self_message(&serde_json::json!({
            "body": { "from": "daemon", "to": "exec" }
        })));
    }

    #[test]
    fn restart_recovery_covers_direct_wakes_without_unread_mail() {
        assert!(!exec_wake_is_interrupted(None, None));
        assert!(exec_wake_is_interrupted(Some(41), None));
        assert!(exec_wake_is_interrupted(Some(41), Some(40)));
        assert!(!exec_wake_is_interrupted(Some(41), Some(42)));
    }

    /// Owed work is now re-derived from durable rows on every scan, so a
    /// company whose wake cannot run must not be retried every five seconds.
    /// The queued reason survives the hold instead of being dropped.
    #[test]
    fn a_wake_that_could_not_run_holds_the_next_automatic_attempt() {
        let mut claims = WakeClaims::default();
        claims.record_unusable_wake("probe");
        assert!(claims.is_backing_off("probe"));
        assert!(!claims.is_backing_off("other-company"));

        claims.queue("probe", "judgement is owed to the Exec");
        assert!(claims.take_ready().is_empty());

        claims.record_usable_wake("probe");
        assert!(!claims.is_backing_off("probe"));
        assert_eq!(
            claims.take_ready(),
            vec![("probe".into(), "judgement is owed to the Exec".into())]
        );
    }

    /// Repeated failure lengthens the hold rather than repeating a fixed one,
    /// and stops lengthening at the ceiling.
    #[test]
    fn repeated_unusable_wakes_back_off_further_up_to_a_ceiling() {
        let mut claims = WakeClaims::default();
        let held_for = |claims: &WakeClaims| {
            claims
                .backoff
                .get("probe")
                .map(|(until, _)| *until - std::time::Instant::now())
                .expect("a hold was recorded")
        };

        claims.record_unusable_wake("probe");
        let first = held_for(&claims);
        claims.record_unusable_wake("probe");
        let second = held_for(&claims);
        assert!(second > first, "{second:?} should exceed {first:?}");

        for _ in 0..12 {
            claims.record_unusable_wake("probe");
        }
        assert!(held_for(&claims) <= super::BACKOFF_CEILING);
    }
}
