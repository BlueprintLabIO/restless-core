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
use sqlx::postgres::PgListener;
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
    // Each cell owns its database, so `NOTIFY` fires there and nowhere else.
    // One listener per cell, multiplexed into this loop — a single listener on
    // the admin connection would hear nothing and degrade every wake to the
    // periodic scan, silently.
    let (wakes, mut inbox) = tokio::sync::mpsc::channel::<String>(256);
    let mut listening = std::collections::HashSet::<String>::new();
    ensure_cell_listeners(&daemon, &wakes, &mut listening).await;
    scan_all_companies(&daemon, &in_flight).await;
    loop {
        let next_due = next_due_delay(&daemon).await;
        tokio::select! {
            _ = tokio::time::sleep(next_due) => {
                // A company created since boot needs its own listener; this is
                // also where a cell whose listener never started is retried.
                ensure_cell_listeners(&daemon, &wakes, &mut listening).await;
                fire_pending(&daemon, &in_flight).await;
                scan_all_companies(&daemon, &in_flight).await;
            }
            _ = daemon.schedule_wake.notified() => {
                tracing::info!("native schedule wake observed; reconciling durable due state");
                ensure_cell_listeners(&daemon, &wakes, &mut listening).await;
                fire_pending(&daemon, &in_flight).await;
                scan_all_companies(&daemon, &in_flight).await;
            }
            Some(payload) = inbox.recv() => {
                // Wake delivery crosses a process and a database boundary now,
                // so make arrival observable: a silent scheduler is otherwise
                // indistinguishable from a cell whose listener never attached.
                tracing::debug!(payload, "cell wake received");
                handle_notification(&daemon, &in_flight, &payload).await;
            }
        }
    }
}

/// Start a listener for every configured cell that does not have one.
async fn ensure_cell_listeners(
    daemon: &Arc<Daemon>,
    wakes: &tokio::sync::mpsc::Sender<String>,
    listening: &mut std::collections::HashSet<String>,
) {
    for company in configured_companies(daemon) {
        if listening.contains(&company) {
            continue;
        }
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
        listening.insert(company.clone());
        let wakes = wakes.clone();
        tokio::spawn(listen_to_cell(company, url, wakes));
    }
}

/// One cell's wake listener. Reconnects forever: losing it would silently
/// degrade that company to the periodic scan.
async fn listen_to_cell(company: String, url: String, wakes: tokio::sync::mpsc::Sender<String>) {
    loop {
        let mut listener = match PgListener::connect(&url).await {
            Ok(listener) => listener,
            Err(error) => {
                tracing::warn!(company, "cell LISTEN connect failed: {error}; retrying");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };
        if listener.listen(OrgIntel::NOTIFY_CHANNEL).await.is_err() {
            tracing::warn!(company, "cell LISTEN failed; retrying");
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }
        tracing::info!(company, "listening for cell wakes");
        loop {
            match listener.recv().await {
                Ok(notification) => {
                    if wakes
                        .send(notification.payload().to_string())
                        .await
                        .is_err()
                    {
                        return; // scheduler stopped
                    }
                }
                Err(error) => {
                    tracing::warn!(company, "cell LISTEN dropped: {error}; reconnecting");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    break;
                }
            }
        }
    }
}

/// Companies configured in this plane, by config file.
fn configured_companies(daemon: &Arc<Daemon>) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(daemon.root.join("companies")) else {
        return Vec::new();
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
    companies
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
    for company in configured_companies(daemon) {
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
    for company in configured_companies(daemon) {
        scan_company(daemon, in_flight, &company).await;
    }
}

async fn scan_company(daemon: &Arc<Daemon>, in_flight: &InFlight, company: &str) {
    if !matches!(runtime::status(company).await, Ok(ContainerStatus::Running)) {
        return;
    }
    let Ok(config) = CompanyConfig::load(&daemon.root, company) else {
        return;
    };
    let Ok(org) = daemon.orgintel.get(company).await else {
        return;
    };

    if let Err(error) =
        crate::staff::reconcile_execution_substrate(&org, &runtime::container_name(company)).await
    {
        tracing::warn!(
            company,
            "could not reconcile exact execution substrate: {error:#}"
        );
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
    let reason = match (judgements, conversation) {
        (0, 0) => return,
        (0, _) => "unread conversation is owed to the Exec",
        (_, 0) => "organisational judgement is owed to the Exec",
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
    let mut message_ids = Vec::new();
    let mut owner_message_ids = Vec::new();
    for message in org.inbox(Some("exec")).await? {
        if org.message_is_work_attempt_input(message.id).await? {
            continue;
        }
        message_ids.push(message.id);
        if message.from_actor == "owner" {
            owner_message_ids.push(message.id);
        }
    }
    // Every pending judgement assigned to the Exec is in the context this turn
    // is about to assemble (`exec::gather_snapshot`), so a turn that completes
    // has genuinely been given all of them. Capture the set before the turn so
    // a judgement created *during* it stays owed to the next one.
    let owed_judgements = org
        .handoffs_assigned_to("exec")
        .await?
        .into_iter()
        .map(|handoff| handoff.id)
        .collect::<Vec<_>>();
    let prior_reply_id = org
        .owner_conversation("exec", 200)
        .await?
        .into_iter()
        .filter(|message| message.from_actor == "exec")
        .map(|message| message.id)
        .max()
        .unwrap_or(0);
    let live_turn = daemon
        .activities
        .start_messages(&config.name, "exec", &owner_message_ids);
    let observer = (!owner_message_ids.is_empty()).then(|| live_turn.observer());
    let outcome = exec::wake(
        config,
        &daemon.spend,
        &daemon.authority,
        &daemon.capabilities,
        org,
        reason,
        observer,
        cancellation,
    )
    .await;

    if cancellation.is_cancelled() {
        if !owner_message_ids.is_empty() {
            // A replacement message is normally persisted before an
            // interruption. The full-screen owner chat can also cancel the
            // exact pending message without adding synthetic prose, so do not
            // claim a fresh direction is queued when that durable input was
            // explicitly consumed.
            let replacement_is_owed = org.inbox(Some("exec")).await.ok().is_some_and(|messages| {
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
        Ok(report) if !owner_message_ids.is_empty() => {
            let mut recorded_reply = org
                .owner_conversation("exec", 200)
                .await?
                .into_iter()
                .filter(|message| message.from_actor == "exec" && message.id > prior_reply_id)
                .max_by_key(|message| message.id);

            // The live assistant block is already the text the owner saw.
            // Preserve it durably when the model answered but missed the
            // explicit message tool; a company-level `blocked` decision is
            // not a failed conversation.
            if recorded_reply.is_none() {
                if let Some(reply) = report.owner_reply.as_deref() {
                    let message_id = org.send_message("exec", None, reply).await?;
                    recorded_reply = org
                        .owner_conversation("exec", 200)
                        .await?
                        .into_iter()
                        .find(|message| message.id == message_id);
                }
            }

            if let Some(reply) = recorded_reply {
                for message_id in message_ids {
                    let _ = org.mark_read(message_id).await;
                }
                let _ = org.mark_handoffs_delivered(&owed_judgements).await;
                live_turn.complete(Some(reply.id), None);
            } else if report.termination == exec::Termination::Blocked {
                live_turn.fail(&report.reason);
            } else {
                live_turn.fail("Exec finished without recording a reply.");
            }
        }
        Ok(report) if report.termination != exec::Termination::Blocked => {
            for message_id in message_ids {
                let _ = org.mark_read(message_id).await;
            }
            let _ = org.mark_handoffs_delivered(&owed_judgements).await;
        }
        Err(error) => {
            if !owner_message_ids.is_empty() {
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
