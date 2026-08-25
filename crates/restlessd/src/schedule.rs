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
use chrono::{DateTime, Utc};
use restless_orgintel::{OrgIntel, WorkAttemptState};
use sqlx::postgres::PgListener;
use tokio_util::sync::CancellationToken;

use crate::runtime::{self, CompanyConfig, ContainerStatus};
use crate::{exec, Daemon};

const SCAN_INTERVAL: Duration = Duration::from_secs(5);

/// Free-form Exec conversation liveness. Work custody is the running Attempt.
pub(crate) type InFlight = Arc<Mutex<WakeClaims>>;

#[derive(Default)]
pub(crate) struct WakeClaims {
    active: HashMap<String, CancellationToken>,
    pending: HashMap<String, String>,
}

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
            .filter(|company| !self.active.contains_key(*company))
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
    let mut listener = connect(&daemon).await;
    scan_all_companies(&daemon, &in_flight).await;
    let mut scan = tokio::time::interval(SCAN_INTERVAL);
    loop {
        tokio::select! {
            _ = scan.tick() => {
                fire_pending(&daemon, &in_flight).await;
                scan_all_companies(&daemon, &in_flight).await;
            }
            notification = listener.recv() => match notification {
                Ok(notification) => handle_notification(&daemon, &in_flight, notification.payload()).await,
                Err(error) => {
                    tracing::warn!("orgintel LISTEN dropped: {error}; reconnecting");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    listener = connect(&daemon).await;
                    scan_all_companies(&daemon, &in_flight).await;
                }
            }
        }
    }
}

async fn connect(daemon: &Daemon) -> PgListener {
    loop {
        match PgListener::connect(&daemon.orgintel.database_url).await {
            Ok(mut listener) => {
                if listener.listen(OrgIntel::NOTIFY_CHANNEL).await.is_ok() {
                    return listener;
                }
                tracing::warn!("orgintel LISTEN failed; retrying");
            }
            Err(error) => tracing::warn!("orgintel LISTEN connect failed: {error}; retrying"),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
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
            // If that exact worker is already running, the new factual input
            // invalidates its frozen prompt: interrupt only that actor. The
            // preserved Attempt becomes unknown and its lead repairs/resumes
            // the same Work with this message in the next bound context.
            let actor = value["body"]["to"].as_str().unwrap_or_default();
            if let Some(message_id) = value["body"]["message_id"].as_i64() {
                let routes_through_work = match daemon.orgintel.get(company).await {
                    Ok(org) => org
                        .message_is_work_attempt_input(message_id)
                        .await
                        .unwrap_or(false),
                    Err(_) => false,
                };
                if routes_through_work && daemon.staff.interrupt(company, actor) {
                    tracing::info!(
                        company,
                        actor,
                        message_id,
                        "material Work feedback interrupted the exact active Staff session"
                    );
                }
            }
            scan_company(daemon, in_flight, company).await;
        }
        Some("work_changed" | "artifact_linked" | "handoff_changed") => {
            scan_company(daemon, in_flight, company).await;
        }
        _ => {}
    }
}

fn is_exec_self_message(value: &serde_json::Value) -> bool {
    value["body"]["to"] == "exec" && value["body"]["from"] == "exec"
}

async fn scan_all_companies(daemon: &Arc<Daemon>, in_flight: &InFlight) {
    let Ok(entries) = std::fs::read_dir(daemon.root.join("companies")) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let Some(company) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        scan_company(daemon, in_flight, company).await;
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

    // An ordinary judgement reaches the Exec only after a lead (or an
    // unassigned specialist) could not settle it. Wake once for each newly
    // assigned generation; the handoff's creation/escalation time is compared
    // with the last Exec wake so a five-second scan does not become a spend
    // loop when the Exec deliberately leaves it pending.
    if let Ok(owed) = org.handoffs_assigned_to("exec").await {
        let newest = owed
            .iter()
            .filter_map(|handoff| handoff.escalated_at.or(Some(handoff.created_at)))
            .max();
        let last_wake = org.latest_event_at("wake").await.ok().flatten();
        if newest.is_some_and(|at| last_wake.is_none_or(|wake| at > wake)) {
            fire_exec(
                daemon,
                in_flight,
                company,
                "organisational judgement escalated to the Exec",
            )
            .await;
        }
    }

    // Owner mail is itself the durable owed-work fact for a free-form Exec
    // conversation. A daemon restart destroys the ACP process and its live
    // projection, so recover an unread message when no completed wake has
    // observed it, or when the latest wake never reached wake_end. Ordinary
    // completed failures are not blindly retried: their wake_end is the
    // terminal observation, and the owner surface presents the interruption.
    recover_exec_conversation(daemon, in_flight, &org, company).await;

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

async fn recover_exec_conversation(
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

    let Ok(messages) = org.inbox(Some("exec")).await else {
        return;
    };
    let mut newest_owner_message: Option<DateTime<Utc>> = None;
    for message in messages {
        if message.from_actor != "owner" {
            continue;
        }
        if org
            .message_is_work_attempt_input(message.id)
            .await
            .unwrap_or(false)
        {
            continue;
        }
        newest_owner_message = Some(newest_owner_message.map_or(message.created_at, |current| {
            current.max(message.created_at)
        }));
    }
    let Some(message_created_at) = newest_owner_message else {
        return;
    };

    let latest_wake = org.latest_event_at("wake").await.ok().flatten();
    let latest_wake_end = org.latest_event_at("wake_end").await.ok().flatten();
    if exec_conversation_is_owed(message_created_at, latest_wake, latest_wake_end) {
        fire_exec(
            daemon,
            in_flight,
            company,
            "recovering unread owner conversation after an interrupted or missed wake",
        )
        .await;
    }
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

fn exec_conversation_is_owed(
    message_created_at: DateTime<Utc>,
    latest_wake: Option<DateTime<Utc>>,
    latest_wake_end: Option<DateTime<Utc>>,
) -> bool {
    let latest_observation = latest_wake.into_iter().chain(latest_wake_end).max();
    let message_was_never_observed =
        latest_observation.is_none_or(|observed_at| message_created_at > observed_at);
    let wake_was_interrupted =
        latest_wake.is_some_and(|wake_at| latest_wake_end.is_none_or(|end_at| wake_at > end_at));
    message_was_never_observed || wake_was_interrupted
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
            live_turn.fail("Interrupted by owner; new direction is queued for a fresh turn.");
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
    use chrono::{TimeZone, Utc};

    use super::{
        actor_exclusions, exec_conversation_is_owed, exec_wake_is_interrupted,
        is_exec_self_message, WakeClaims,
    };

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
    fn restart_recovery_distinguishes_missed_interrupted_and_completed_wakes() {
        let at = |second| Utc.timestamp_opt(second, 0).unwrap();
        let message = at(20);

        assert!(exec_conversation_is_owed(message, None, None));
        assert!(exec_conversation_is_owed(
            message,
            Some(at(21)),
            Some(at(10))
        ));
        assert!(exec_conversation_is_owed(
            message,
            Some(at(10)),
            Some(at(11))
        ));
        assert!(!exec_conversation_is_owed(
            message,
            Some(at(21)),
            Some(at(22))
        ));
        assert!(!exec_conversation_is_owed(
            message,
            Some(at(10)),
            Some(at(21))
        ));
    }

    #[test]
    fn restart_recovery_covers_direct_wakes_without_unread_mail() {
        assert!(!exec_wake_is_interrupted(None, None));
        assert!(exec_wake_is_interrupted(Some(41), None));
        assert!(exec_wake_is_interrupted(Some(41), Some(40)));
        assert!(!exec_wake_is_interrupted(Some(41), Some(42)));
    }
}
