//! Shared, body-free wake hints from company-cell PostgreSQL databases.
//!
//! LISTEN/NOTIFY is deliberately not a queue. One reconnecting listener per
//! cell fans hints out inside this daemon; every consumer rereads its own
//! durable source of truth and repairs lost, duplicated, or lagged hints.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use restless_orgintel::OrgIntel;
use sqlx::postgres::PgListener;
use tokio::sync::{broadcast, watch};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const WAKE_FANOUT_CAPACITY: usize = 512;
const LISTENER_CONNECT_RETRY: Duration = Duration::from_secs(5);
const LISTENER_RECV_RETRY: Duration = Duration::from_secs(2);

pub(crate) const DEFAULT_GLOBAL_STREAM_LIMIT: usize = 1_024;
pub(crate) const DEFAULT_COMPANY_STREAM_LIMIT: usize = 256;
pub(crate) const DEFAULT_PRINCIPAL_STREAM_LIMIT: usize = 8;

/// A notification hint. `raw` preserves the existing scheduler contract;
/// Room consumers inspect only body-free identifiers and never trust them as
/// a cursor or event payload.
#[derive(Clone, Debug)]
pub(crate) struct CellWake {
    pub(crate) company: String,
    pub(crate) kind: Option<String>,
    pub(crate) room_id: Option<Uuid>,
    all_rooms: bool,
    pub(crate) event_id: Option<i64>,
    pub(crate) raw: Arc<str>,
}

impl CellWake {
    pub(crate) fn wakes_room(&self, company: &str, room_id: Uuid) -> bool {
        self.company == company
            && self.kind.as_deref() == Some("room_event")
            && (self.room_id == Some(room_id) || (self.room_id.is_none() && self.all_rooms))
    }
}

#[derive(Clone)]
pub(crate) struct CellWakeHub {
    inner: Arc<CellWakeHubInner>,
}

impl Drop for CellWakeHub {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) != 1 {
            return;
        }
        for listener in self
            .inner
            .listeners
            .lock()
            .expect("cell listeners poisoned")
            .values()
        {
            listener.cancellation.cancel();
        }
    }
}

struct CellWakeHubInner {
    scheduler_wakes: broadcast::Sender<CellWake>,
    listeners: Mutex<HashMap<String, ListenerEntry>>,
    admissions: Mutex<AdmissionCounts>,
    limits: StreamLimits,
}

struct ListenerEntry {
    database_url: String,
    ready: watch::Receiver<bool>,
    cancellation: CancellationToken,
    wakes: broadcast::Sender<CellWake>,
}

struct ListenerStart {
    company: String,
    database_url: String,
    ready: watch::Sender<bool>,
    cancellation: CancellationToken,
    company_wakes: broadcast::Sender<CellWake>,
}

#[derive(Clone, Copy)]
struct StreamLimits {
    global: usize,
    company: usize,
    principal: usize,
}

#[derive(Default)]
struct AdmissionCounts {
    global: usize,
    companies: HashMap<String, usize>,
    principals: HashMap<(String, String), usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StreamAdmissionRefusal {
    Principal,
    Company,
    Global,
}

/// A live stream owns this permit. Dropping an HTTP body, ending a lease, or
/// losing Room participation drops the permit without a separate cleanup
/// path that could be skipped.
pub(crate) struct StreamAdmission {
    hub: Weak<CellWakeHubInner>,
    company: String,
    principal: String,
}

impl Drop for StreamAdmission {
    fn drop(&mut self) {
        let Some(hub) = self.hub.upgrade() else {
            return;
        };
        let mut counts = hub.admissions.lock().expect("stream admissions poisoned");
        counts.global = counts.global.saturating_sub(1);
        decrement(&mut counts.companies, &self.company);
        decrement(
            &mut counts.principals,
            &(self.company.clone(), self.principal.clone()),
        );
    }
}

fn decrement<K>(counts: &mut HashMap<K, usize>, key: &K)
where
    K: std::cmp::Eq + std::hash::Hash,
{
    if let Some(count) = counts.get_mut(key) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            counts.remove(key);
        }
    }
}

impl Default for CellWakeHub {
    fn default() -> Self {
        Self::with_stream_limits(
            DEFAULT_GLOBAL_STREAM_LIMIT,
            DEFAULT_COMPANY_STREAM_LIMIT,
            DEFAULT_PRINCIPAL_STREAM_LIMIT,
        )
    }
}

impl CellWakeHub {
    pub(crate) fn with_stream_limits(global: usize, company: usize, principal: usize) -> Self {
        assert!(global > 0 && company > 0 && principal > 0);
        let (scheduler_wakes, _) = broadcast::channel(WAKE_FANOUT_CAPACITY);
        Self {
            inner: Arc::new(CellWakeHubInner {
                scheduler_wakes,
                listeners: Mutex::new(HashMap::new()),
                admissions: Mutex::new(AdmissionCounts::default()),
                limits: StreamLimits {
                    global,
                    company,
                    principal,
                },
            }),
        }
    }

    /// Subscribe before starting/confirming the cell listener. Consumers then
    /// perform their first durable read, closing the usual read-then-LISTEN
    /// lost-wake window once the shared listener is established. Startup or
    /// reconnect loss is repaired by each consumer's bounded fallback read.
    pub(crate) fn subscribe_company(
        &self,
        company: &str,
        database_url: &str,
    ) -> broadcast::Receiver<CellWake> {
        let (receiver, start) = self.prepare_company(company, database_url, true);
        if let Some(start) = start {
            self.spawn_listener(start);
        }
        receiver.expect("company subscription was requested")
    }

    /// Scheduler consumers need every company hint, not just Room hints.
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<CellWake> {
        self.inner.scheduler_wakes.subscribe()
    }

    pub(crate) fn ensure_company(&self, company: &str, database_url: &str) {
        let (_, start) = self.prepare_company(company, database_url, false);
        if let Some(start) = start {
            self.spawn_listener(start);
        }
    }

    /// Cancel and forget listeners for companies no longer configured on this
    /// plane. Dropping the per-company sender is intentional: existing Room
    /// receivers observe closure and terminate rather than replaying a
    /// deconfigured tenant forever. Transient disconnects keep the entry and
    /// sender alive while the listener reconnects.
    pub(crate) fn retain_companies(&self, configured: &[String]) {
        let configured = configured
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let mut listeners = self
            .inner
            .listeners
            .lock()
            .expect("cell listeners poisoned");
        listeners.retain(|company, entry| {
            let keep = configured.contains(company.as_str());
            if !keep {
                entry.cancellation.cancel();
            }
            keep
        });
    }

    pub(crate) fn remove_company(&self, company: &str) {
        if let Some(entry) = self
            .inner
            .listeners
            .lock()
            .expect("cell listeners poisoned")
            .remove(company)
        {
            entry.cancellation.cancel();
        }
    }

    fn prepare_company(
        &self,
        company: &str,
        database_url: &str,
        subscribe: bool,
    ) -> (Option<broadcast::Receiver<CellWake>>, Option<ListenerStart>) {
        let mut listeners = self
            .inner
            .listeners
            .lock()
            .expect("cell listeners poisoned");
        if let Some(entry) = listeners.get_mut(company) {
            // Subscribe under the registry lock while the existing listener
            // and sender are stable, closing the subscribe/read wait gap.
            let receiver = subscribe.then(|| entry.wakes.subscribe());
            if entry.database_url == database_url {
                let _listener_is_ready = *entry.ready.borrow();
                return (receiver, None);
            }

            // Cell replacement changes the connection, not the company's
            // in-process wake identity. Existing streams retain this sender.
            entry.cancellation.cancel();
            let (ready_tx, ready_rx) = watch::channel(false);
            let cancellation = CancellationToken::new();
            entry.database_url = database_url.to_string();
            entry.ready = ready_rx;
            entry.cancellation = cancellation.clone();
            let start = ListenerStart {
                company: company.to_string(),
                database_url: database_url.to_string(),
                ready: ready_tx,
                cancellation,
                company_wakes: entry.wakes.clone(),
            };
            return (receiver, Some(start));
        }

        let (wakes, _) = broadcast::channel(WAKE_FANOUT_CAPACITY);
        let receiver = subscribe.then(|| wakes.subscribe());
        let (ready_tx, ready_rx) = watch::channel(false);
        let cancellation = CancellationToken::new();
        listeners.insert(
            company.to_string(),
            ListenerEntry {
                database_url: database_url.to_string(),
                ready: ready_rx,
                cancellation: cancellation.clone(),
                wakes: wakes.clone(),
            },
        );
        (
            receiver,
            Some(ListenerStart {
                company: company.to_string(),
                database_url: database_url.to_string(),
                ready: ready_tx,
                cancellation,
                company_wakes: wakes,
            }),
        )
    }

    fn spawn_listener(&self, start: ListenerStart) {
        tokio::spawn(listen_to_cell(
            start.company,
            start.database_url,
            start.ready,
            start.cancellation,
            start.company_wakes,
            self.inner.scheduler_wakes.clone(),
        ));
    }

    #[cfg(test)]
    fn company_sender(&self, company: &str) -> Option<broadcast::Sender<CellWake>> {
        self.inner
            .listeners
            .lock()
            .expect("cell listeners poisoned")
            .get(company)
            .map(|entry| entry.wakes.clone())
    }

    #[cfg(test)]
    fn listener_cancellation(&self, company: &str) -> Option<CancellationToken> {
        self.inner
            .listeners
            .lock()
            .expect("cell listeners poisoned")
            .get(company)
            .map(|entry| entry.cancellation.clone())
    }

    #[cfg(test)]
    fn has_listener(&self, company: &str) -> bool {
        self.inner
            .listeners
            .lock()
            .expect("cell listeners poisoned")
            .contains_key(company)
    }

    pub(crate) fn try_admit(
        &self,
        company: &str,
        principal: &str,
    ) -> Result<StreamAdmission, StreamAdmissionRefusal> {
        let mut counts = self
            .inner
            .admissions
            .lock()
            .expect("stream admissions poisoned");
        let principal_key = (company.to_string(), principal.to_string());
        if counts.principals.get(&principal_key).copied().unwrap_or(0)
            >= self.inner.limits.principal
        {
            return Err(StreamAdmissionRefusal::Principal);
        }
        if counts.companies.get(company).copied().unwrap_or(0) >= self.inner.limits.company {
            return Err(StreamAdmissionRefusal::Company);
        }
        if counts.global >= self.inner.limits.global {
            return Err(StreamAdmissionRefusal::Global);
        }
        counts.global += 1;
        *counts.companies.entry(company.to_string()).or_default() += 1;
        *counts.principals.entry(principal_key).or_default() += 1;
        Ok(StreamAdmission {
            hub: Arc::downgrade(&self.inner),
            company: company.to_string(),
            principal: principal.to_string(),
        })
    }

    #[cfg(test)]
    pub(crate) async fn wait_until_ready(&self, company: &str, bound: Duration) -> bool {
        let mut ready = {
            let listeners = self
                .inner
                .listeners
                .lock()
                .expect("cell listeners poisoned");
            listeners.get(company).map(|entry| entry.ready.clone())
        };
        let Some(ref mut ready) = ready else {
            return false;
        };
        if *ready.borrow() {
            return true;
        }
        tokio::time::timeout(bound, async {
            loop {
                ready.changed().await.map_err(|_| ())?;
                if *ready.borrow() {
                    return Ok::<_, ()>(());
                }
            }
        })
        .await
        .is_ok_and(|result| result.is_ok())
    }

    #[cfg(test)]
    pub(crate) fn active_streams(&self) -> usize {
        self.inner
            .admissions
            .lock()
            .expect("stream admissions poisoned")
            .global
    }
}

async fn listen_to_cell(
    company: String,
    database_url: String,
    ready: watch::Sender<bool>,
    cancellation: CancellationToken,
    company_wakes: broadcast::Sender<CellWake>,
    scheduler_wakes: broadcast::Sender<CellWake>,
) {
    loop {
        if cancellation.is_cancelled() {
            return;
        }
        let connection = PgListener::connect(&database_url);
        let mut listener = match tokio::select! {
            listener = connection => listener,
            _ = cancellation.cancelled() => return,
        } {
            Ok(listener) => listener,
            Err(error) => {
                tracing::warn!(company, "cell LISTEN connect failed: {error}; retrying");
                if sleep_or_cancel(LISTENER_CONNECT_RETRY, &cancellation).await {
                    return;
                }
                continue;
            }
        };
        let listen = listener.listen(OrgIntel::NOTIFY_CHANNEL);
        match tokio::select! {
            result = listen => Some(result),
            _ = cancellation.cancelled() => None,
        } {
            None => return,
            Some(Err(error)) => {
                tracing::warn!(company, "cell LISTEN failed: {error}; retrying");
                if sleep_or_cancel(LISTENER_CONNECT_RETRY, &cancellation).await {
                    return;
                }
                continue;
            }
            Some(Ok(())) => {}
        }
        let _ = ready.send(true);
        tracing::info!(company, "listening for shared cell wakes");
        loop {
            let notification = tokio::select! {
                notification = listener.recv() => notification,
                _ = cancellation.cancelled() => return,
            };
            match notification {
                Ok(notification) => {
                    let raw: Arc<str> = Arc::from(notification.payload());
                    if let Some(wake) = parse_wake_for_company(&company, raw) {
                        let _ = company_wakes.send(wake.clone());
                        let _ = scheduler_wakes.send(wake);
                    }
                }
                Err(error) => {
                    let _ = ready.send(false);
                    tracing::warn!(company, "cell LISTEN dropped: {error}; reconnecting");
                    if sleep_or_cancel(LISTENER_RECV_RETRY, &cancellation).await {
                        return;
                    }
                    break;
                }
            }
        }
    }
}

async fn sleep_or_cancel(duration: Duration, cancellation: &CancellationToken) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(duration) => false,
        _ = cancellation.cancelled() => true,
    }
}

fn parse_wake_for_company(company: &str, raw: Arc<str>) -> Option<CellWake> {
    let value = match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(value) => value,
        Err(_) => {
            return Some(CellWake {
                company: company.to_string(),
                kind: None,
                room_id: None,
                all_rooms: false,
                event_id: None,
                raw,
            });
        }
    };
    let payload_company = value.get("company").and_then(serde_json::Value::as_str);
    // Test cells can share one physical database. Every listener receives the
    // database-wide channel, so discard another schema's hint here rather than
    // amplifying it once per configured company.
    if payload_company != Some(company) {
        return None;
    }
    let kind = value
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let room_id = value
        .pointer("/body/room_id")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok());
    let all_rooms = value
        .pointer("/body/scope")
        .and_then(serde_json::Value::as_str)
        == Some("all_rooms");
    let event_id = value
        .pointer("/body/event_id")
        .and_then(serde_json::Value::as_i64);
    Some(CellWake {
        company: company.to_string(),
        kind,
        room_id,
        all_rooms,
        event_id,
        raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_wake_parser_is_body_free_scoped_and_compaction_wakes_every_room() {
        let room = Uuid::new_v4();
        let raw: Arc<str> = Arc::from(
            serde_json::json!({
                "company": "aris",
                "kind": "room_event",
                "body": { "room_id": room, "event_id": 42, "scope": "room" }
            })
            .to_string(),
        );
        let wake = parse_wake_for_company("aris", raw).unwrap();
        assert!(wake.wakes_room("aris", room));
        assert_eq!(wake.event_id, Some(42));
        assert!(!wake.raw.contains("message body"));
        assert!(parse_wake_for_company(
            "other",
            Arc::from(r#"{"company":"aris","kind":"room_event","body":{}}"#)
        )
        .is_none());

        let compacted = parse_wake_for_company(
            "aris",
            Arc::from(
                r#"{"company":"aris","kind":"room_event","body":{"room_id":null,"event_id":43,"scope":"all_rooms"}}"#,
            ),
        )
        .unwrap();
        assert!(compacted.wakes_room("aris", Uuid::new_v4()));
        let malformed = parse_wake_for_company(
            "aris",
            Arc::from(r#"{"company":"aris","kind":"room_event","body":{"event_id":44}}"#),
        )
        .unwrap();
        assert!(!malformed.wakes_room("aris", Uuid::new_v4()));
    }

    #[test]
    fn stream_admission_is_bounded_and_drop_releases_every_dimension() {
        let hub = CellWakeHub::with_stream_limits(2, 2, 1);
        let first = hub.try_admit("aris", "alice").unwrap();
        assert!(matches!(
            hub.try_admit("aris", "alice"),
            Err(StreamAdmissionRefusal::Principal)
        ));
        let second = hub.try_admit("aris", "bob").unwrap();
        assert!(matches!(
            hub.try_admit("aris", "carol"),
            Err(StreamAdmissionRefusal::Company)
        ));
        assert!(matches!(
            hub.try_admit("other", "mallory"),
            Err(StreamAdmissionRefusal::Global)
        ));
        assert_eq!(hub.active_streams(), 2);
        drop(first);
        assert!(hub.try_admit("aris", "alice").is_ok());
        drop(second);
    }

    #[test]
    fn dropping_the_last_hub_handle_cancels_cell_listener_tasks() {
        let hub = CellWakeHub::default();
        let cancellation = CancellationToken::new();
        let (_, ready) = watch::channel(false);
        let (wakes, _) = broadcast::channel(1);
        hub.inner.listeners.lock().unwrap().insert(
            "aris".into(),
            ListenerEntry {
                database_url: "postgres://example.invalid/aris".into(),
                ready,
                cancellation: cancellation.clone(),
                wakes,
            },
        );
        let clone = hub.clone();
        drop(hub);
        assert!(!cancellation.is_cancelled());
        drop(clone);
        assert!(cancellation.is_cancelled());
    }

    fn test_wake(company: &str, room_id: Uuid, event_id: i64) -> CellWake {
        CellWake {
            company: company.into(),
            kind: Some("room_event".into()),
            room_id: Some(room_id),
            all_rooms: false,
            event_id: Some(event_id),
            raw: Arc::from("{}"),
        }
    }

    #[tokio::test]
    async fn noisy_company_fanout_cannot_lag_or_wake_another_company() {
        let hub = CellWakeHub::default();
        let mut aris = hub.subscribe_company("aris", "not-a-postgresql-url-a");
        let mut beta = hub.subscribe_company("beta", "not-a-postgresql-url-b");
        let aris_sender = hub.company_sender("aris").unwrap();
        let beta_sender = hub.company_sender("beta").unwrap();
        let aris_room = Uuid::new_v4();
        for event_id in 1..=(WAKE_FANOUT_CAPACITY as i64 + 20) {
            let _ = aris_sender.send(test_wake("aris", aris_room, event_id));
        }
        assert!(matches!(
            aris.try_recv(),
            Err(broadcast::error::TryRecvError::Lagged(_))
        ));
        assert!(matches!(
            beta.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));

        let beta_room = Uuid::new_v4();
        beta_sender.send(test_wake("beta", beta_room, 1)).unwrap();
        assert!(beta.recv().await.unwrap().wakes_room("beta", beta_room));
    }

    #[tokio::test]
    async fn url_rotation_preserves_streams_while_deconfiguration_closes_them() {
        let hub = CellWakeHub::default();
        let mut stream = hub.subscribe_company("aris", "not-a-postgresql-url-a");
        let original_sender = hub.company_sender("aris").unwrap();
        let original_cancellation = hub.listener_cancellation("aris").unwrap();

        hub.ensure_company("aris", "not-a-postgresql-url-b");
        assert!(original_cancellation.is_cancelled());
        let replacement_cancellation = hub.listener_cancellation("aris").unwrap();
        assert!(!replacement_cancellation.is_cancelled());
        let room = Uuid::new_v4();
        original_sender.send(test_wake("aris", room, 1)).unwrap();
        assert!(stream.recv().await.unwrap().wakes_room("aris", room));
        drop(original_sender);

        hub.retain_companies(&[]);
        assert!(!hub.has_listener("aris"));
        assert!(replacement_cancellation.is_cancelled());
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(1), stream.recv()).await,
            Ok(Err(broadcast::error::RecvError::Closed))
        ));
    }
}
