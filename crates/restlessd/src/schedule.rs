//! The scheduler (sprint 01 T6): the company must act without the owner
//! typing. Two trigger types share one loop:
//!
//! - time-driven — a wake the Exec scheduled for itself (a durable
//!   `wake_scheduled` event whose fire time arrives) or the coarse periodic
//!   tick (open milestone, no wake in a while). Fires with no event.
//! - event-driven — Postgres LISTEN/NOTIFY raised by OrgIntel writes (a
//!   commitment completed, mail addressed to the Exec). Fires with no timer.
//!
//! At-least-once, duplicates tolerated (§9.3): the in-flight guard stops
//! overlap within one daemon, and OrgIntel state makes re-fired wakes
//! continuations rather than restarts. Schedules live in OrgIntel events,
//! so both trigger types survive a restlessd restart.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use restless_orgintel::{CommitmentState, OrgIntel};
use sqlx::postgres::PgListener;

use crate::runtime::{self, CompanyConfig, ContainerStatus};
use crate::{Daemon, exec};

/// How often time triggers are evaluated.
const SCAN_INTERVAL: Duration = Duration::from_secs(5);
/// Fallback cadence: an open, unblocked Exec milestone with no wake in this
/// window gets one. Continue-schedules fire at their exact time; the tick
/// only catches what fell through (a crash, a continue with no minutes).
const TICK_IDLE_MINUTES: i64 = 15;

/// Per-company wake guard: one wake at a time, however many triggers fire.
type InFlight = Arc<Mutex<HashSet<String>>>;

pub async fn run(daemon: Arc<Daemon>) {
    let in_flight: InFlight = Arc::new(Mutex::new(HashSet::new()));
    let mut listener = connect(&daemon).await;
    reconcile_missed_events(&daemon, &in_flight).await;
    let mut scan = tokio::time::interval(SCAN_INTERVAL);
    loop {
        tokio::select! {
            _ = scan.tick() => scan_time_triggers(&daemon, &in_flight).await,
            notification = listener.recv() => {
                match notification {
                    Ok(notification) => {
                        handle_notification(&daemon, &in_flight, notification.payload()).await;
                    }
                    Err(error) => {
                        tracing::warn!("orgintel LISTEN dropped: {error}; reconnecting");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        listener = connect(&daemon).await;
                        reconcile_missed_events(&daemon, &in_flight).await;
                    }
                }
            }
        }
    }
}

/// NOTIFY is lossy while the listener is down (restart, connection drop) —
/// but the event stream is durable. On every (re)connect, sweep for results
/// that landed unseen: unread Exec mail or a non-Exec completion newer than
/// the last wake. This is what makes delivery at-least-once for real (§9.3)
/// and what lets event-driven wakeups survive a restlessd restart.
async fn reconcile_missed_events(daemon: &Arc<Daemon>, in_flight: &InFlight) {
    let Ok(entries) = std::fs::read_dir(daemon.root.join("companies")) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else { continue };
        if in_flight.lock().expect("in-flight guard").contains(name) {
            continue;
        }
        if !matches!(runtime::status(name).await, Ok(ContainerStatus::Running)) {
            continue;
        }
        if CompanyConfig::load(&daemon.root, name).is_err() {
            continue;
        }
        let Ok(org) = daemon.orgintel.get(name).await else { continue };
        let Some(last_wake) = org.latest_event_at("wake").await.ok().flatten() else { continue };
        if let Ok(mail) = org.inbox(Some("exec")).await {
            if mail.iter().any(|message| message.created_at > last_wake) {
                fire(daemon, in_flight, name, "event: unread mail waiting (reconciled)").await;
                continue;
            }
        }
        if let Ok(commitments) = org.list_commitments().await {
            if let Some(done) = commitments.iter().find(|c| {
                c.state == CommitmentState::Completed
                    && c.owner_id != "exec"
                    && c.updated_at > last_wake
            }) {
                let reason = format!(
                    "event: commitment completed by {}: {} (reconciled)",
                    done.owner_id, done.title
                );
                fire(daemon, in_flight, name, &reason).await;
            }
        }
    }
}

/// LISTEN forever; the database was probed at boot, so keep retrying rather
/// than killing the daemon over a transient drop.
async fn connect(daemon: &Daemon) -> PgListener {
    loop {
        match PgListener::connect(&daemon.orgintel.database_url).await {
            Ok(mut listener) => {
                if let Err(error) = listener.listen(OrgIntel::NOTIFY_CHANNEL).await {
                    tracing::warn!("orgintel LISTEN failed: {error}; retrying");
                } else {
                    tracing::info!(channel = OrgIntel::NOTIFY_CHANNEL, "scheduler listening");
                    return listener;
                }
            }
            Err(error) => tracing::warn!("orgintel LISTEN connect failed: {error}; retrying"),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Event-driven trigger: a notification payload is
/// `{"company", "kind", "body"}` — wake the right actor's company.
async fn handle_notification(daemon: &Arc<Daemon>, in_flight: &InFlight, payload: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        tracing::warn!(payload, "unparseable orgintel notification");
        return;
    };
    let Some(company) = value["company"].as_str() else { return };
    let reason = match value["kind"].as_str() {
        // A dependent result landed: someone ELSE's commitment completed
        // under the Exec. The Exec's own completions need no wake — it was
        // awake to complete them (this guard is what stops a done milestone
        // from immediately re-waking the company into a new milestone).
        Some("commitment_completed") if value["body"]["owner"] != "exec" => {
            format!("event: commitment completed by {}: {}",
                value["body"]["owner"].as_str().unwrap_or("unknown"),
                value["body"]["title"].as_str().unwrap_or("untitled"))
        }
        // Mail addressed to the Exec (e.g. the owner unblocking it).
        Some("message") if value["body"]["to"] == "exec" => {
            format!("event: mail from {}", value["body"]["from"].as_str().unwrap_or("unknown"))
        }
        _ => return,
    };
    fire(daemon, in_flight, company, &reason).await;
}

/// Time-driven triggers: the Exec's own scheduled continuation, and the
/// periodic tick for open unblocked milestones that have gone quiet.
async fn scan_time_triggers(daemon: &Arc<Daemon>, in_flight: &InFlight) {
    let Ok(entries) = std::fs::read_dir(daemon.root.join("companies")) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else { continue };
        if in_flight.lock().expect("in-flight guard").contains(name) {
            continue;
        }
        // A stopped container has nothing to wake; skip before touching the DB.
        if !matches!(runtime::status(name).await, Ok(ContainerStatus::Running)) {
            continue;
        }
        if CompanyConfig::load(&daemon.root, name).is_err() {
            continue;
        }
        let Ok(org) = daemon.orgintel.get(name).await else { continue };
        let Ok(commitments) = org.list_commitments().await else { continue };
        let milestone = commitments.iter().find(|c| {
            c.owner_id == "exec"
                && matches!(
                    c.state,
                    CommitmentState::Proposed | CommitmentState::Active | CommitmentState::Blocked
                )
        });
        let Some(milestone) = milestone else { continue }; // no open milestone: nothing to drive
        let now = Utc::now();
        let last_wake = org.latest_event_at("wake").await.ok().flatten();

        // The Exec's own schedule: due and not already honored by a wake.
        if let Ok(Some(fire_at)) = org.latest_wake_schedule().await {
            let honored = last_wake.is_some_and(|at| at >= fire_at);
            if fire_at <= now && !honored {
                fire(daemon, in_flight, name, "scheduled continuation (exec-set)").await;
                continue;
            }
        }

        // The tick: blocked waits on the owner, not on time — only proposed
        // and active milestones are tick-driven.
        if matches!(milestone.state, CommitmentState::Blocked) {
            continue;
        }
        let quiet = last_wake
            .map(|at| (now - at) > chrono::Duration::minutes(TICK_IDLE_MINUTES))
            .unwrap_or(false); // a company with no wakes was woken by hand first
        if quiet {
            let reason = format!("periodic tick: no wake in {TICK_IDLE_MINUTES}m");
            fire(daemon, in_flight, name, &reason).await;
        }
    }
}

/// Fire one wake under the in-flight guard; the wake itself is spawned so a
/// 20-minute turn never stalls the trigger loop.
async fn fire(daemon: &Arc<Daemon>, in_flight: &InFlight, company: &str, reason: &str) {
    {
        let mut guard = in_flight.lock().expect("in-flight guard");
        if !guard.insert(company.to_string()) {
            return; // already waking
        }
    }
    tracing::info!(company, reason, "scheduler wake");
    let daemon = Arc::clone(daemon);
    let in_flight = Arc::clone(in_flight);
    let company = company.to_string();
    let reason = reason.to_string();
    tokio::spawn(async move {
        let outcome = async {
            let config = CompanyConfig::load(&daemon.root, &company)?;
            let org = daemon.orgintel.get(&company).await?;
            exec::wake(&config, &daemon.gateway, &org, &reason).await
        }
        .await;
        match outcome {
            Ok(report) => tracing::info!(
                company,
                termination = ?report.termination,
                "scheduled wake completed"
            ),
            Err(error) => tracing::warn!(company, "scheduled wake failed: {error:#}"),
        }
        in_flight.lock().expect("in-flight guard").remove(&company);
    });
}
