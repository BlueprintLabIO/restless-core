//! Durable scheduling from OrgIntel facts.
//!
//! Conversation wakes stay free-form. Machine work has one deterministic
//! kickoff: atomically claim the next ready Work node, create its Attempt,
//! then supervise its actor. Time conditions live in `schedules`, not event
//! prose, and every notification is merely a hint to reread canonical rows.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use restless_orgintel::{OrgIntel, WorkAttemptState};
use sqlx::postgres::PgListener;

use crate::runtime::{self, CompanyConfig, ContainerStatus};
use crate::{exec, Daemon};

const SCAN_INTERVAL: Duration = Duration::from_secs(5);

/// Free-form Exec conversation liveness. Work custody is the running Attempt.
pub(crate) type InFlight = Arc<Mutex<WakeClaims>>;

#[derive(Default)]
pub(crate) struct WakeClaims {
    active: HashSet<String>,
    pending: HashMap<String, String>,
}

impl WakeClaims {
    pub(crate) fn claim(&mut self, company: &str) -> bool {
        if self.active.insert(company.to_string()) {
            self.pending.remove(company);
            true
        } else {
            false
        }
    }

    pub(crate) fn is_active(&self, company: &str) -> bool {
        self.active.contains(company)
    }

    fn queue(&mut self, company: &str, reason: &str) {
        self.pending.insert(company.to_string(), reason.to_string());
    }

    fn release(&mut self, company: &str) {
        self.active.remove(company);
    }

    fn take_ready(&mut self) -> Vec<(String, String)> {
        let ready = self
            .pending
            .keys()
            .filter(|company| !self.active.contains(*company))
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
            scan_company(daemon, in_flight, company).await;
        }
        Some("work_changed" | "artifact_linked" | "handoff_changed") => {
            scan_company(daemon, in_flight, company).await;
        }
        _ => {}
    }
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
                &daemon.spend,
                &daemon.authority,
                &org,
                &daemon.staff,
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
        let busy_actors = daemon.staff.running_actors(company);
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
            &org,
            &daemon.staff,
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

/// Fire a free-form Exec conversation. It never owns or mutates Work.
async fn fire_exec(daemon: &Arc<Daemon>, in_flight: &InFlight, company: &str, reason: &str) {
    if daemon.staff.is_actor_running(company, "exec") {
        in_flight
            .lock()
            .expect("in-flight guard")
            .queue(company, reason);
        return;
    }
    {
        let mut guard = in_flight.lock().expect("in-flight guard");
        if !guard.claim(company) {
            guard.queue(company, reason);
            return;
        }
    }
    let daemon = Arc::clone(daemon);
    let in_flight = Arc::clone(in_flight);
    let company = company.to_string();
    let reason = reason.to_string();
    tokio::spawn(async move {
        let _guard = WakeGuard::new(&company, &in_flight);
        let outcome = async {
            let config = CompanyConfig::load(&daemon.root, &company)?;
            let org = daemon.orgintel.get(&company).await?;
            exec::wake(&config, &daemon.spend, &daemon.authority, &org, &reason).await
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
    use super::WakeClaims;

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
}
