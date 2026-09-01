//! Hosted Runtime Bridge registration and connection custody.
//!
//! A hosted account plane has no Docker authority. The released Runtime opens
//! this authenticated outbound channel instead. This module owns connection
//! identity and liveness only; process/file/desktop messages are added as
//! versioned protocol capabilities rather than smuggled through owner routes.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use axum::extract::ws::{Message, WebSocket};
use chrono::{DateTime, Utc};
use futures_util::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::capability::CapabilityIssuer;

pub(crate) const PROTOCOL_VERSION: u32 = 1;
const REGISTER_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_REGISTER_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const PROBE_TIMEOUT: Duration = Duration::from_secs(1);
const REQUIRED_FEATURES: &[&str] = &[
    "activity.v1",
    "desktop.v1",
    "files.v1",
    "process.v1",
    "streams.v1",
];
const KNOWN_FEATURES: &[&str] = &[
    "registration.v1",
    "activity.v1",
    "desktop.v1",
    "files.v1",
    "process.v1",
    "streams.v1",
];

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlaneScope {
    pub(crate) owner_id: Uuid,
    pub(crate) plane_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    protocol_version: u32,
    owner_id: Uuid,
    plane_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    company: String,
    runtime_id: String,
    runtime_generation: u64,
    desired_revision: i64,
    runtime_image: String,
    volume_name: String,
    persistent_volume_ready: bool,
    source_revision: String,
    supported_features: Vec<String>,
    capability: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Observation {
    pub(crate) owner_id: Uuid,
    pub(crate) plane_id: Uuid,
    pub(crate) company_id: Uuid,
    pub(crate) cell_id: Uuid,
    pub(crate) company: String,
    pub(crate) runtime_id: String,
    pub(crate) runtime_generation: u64,
    pub(crate) desired_revision: i64,
    pub(crate) runtime_image: String,
    pub(crate) volume_name: String,
    pub(crate) persistent_volume_ready: bool,
    pub(crate) source_revision: String,
    pub(crate) supported_features: Vec<String>,
    pub(crate) connected_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct Accepted {
    protocol_version: u32,
    status: &'static str,
    connection_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    runtime_id: String,
    runtime_generation: u64,
}

#[derive(Clone, Default)]
pub(crate) struct Registry {
    entries: Arc<Mutex<HashMap<String, ActiveConnection>>>,
}

#[derive(Clone)]
struct ActiveConnection {
    connection_id: Uuid,
    observation: Observation,
    outbound: mpsc::Sender<String>,
    pending: Arc<Mutex<HashMap<Uuid, oneshot::Sender<AgentResponse>>>>,
}

#[derive(Debug, Deserialize)]
struct AgentResponse {
    #[serde(rename = "type")]
    kind: String,
    operation_id: Uuid,
    runtime_id: String,
    runtime_generation: u64,
    #[serde(flatten)]
    body: HashMap<String, serde_json::Value>,
}

struct ConnectionGuard {
    registry: Registry,
    company: String,
    connection_id: Uuid,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let mut entries = self
            .registry
            .entries
            .lock()
            .expect("Runtime Bridge registry");
        if entries
            .get(&self.company)
            .is_some_and(|active| active.connection_id == self.connection_id)
        {
            entries.remove(&self.company);
        }
    }
}

impl Registry {
    pub(crate) async fn accept(
        &self,
        mut socket: WebSocket,
        issuer: CapabilityIssuer,
        scope: PlaneScope,
    ) -> Result<()> {
        let first = tokio::time::timeout(REGISTER_TIMEOUT, socket.next())
            .await
            .context("Runtime Bridge registration timed out")?
            .context("Runtime Bridge closed before registration")?
            .context("read Runtime Bridge registration")?;
        let Message::Text(raw) = first else {
            bail!("Runtime Bridge registration must be one text frame");
        };
        if raw.len() > MAX_REGISTER_BYTES {
            bail!("Runtime Bridge registration exceeds the bounded frame size");
        }
        let registration: Registration =
            serde_json::from_str(&raw).context("decode Runtime Bridge registration")?;
        let observation = authenticate_registration(registration, &issuer, scope)?;
        let connection_id = Uuid::new_v4();
        let (outbound, mut outbound_rx) = mpsc::channel::<String>(32);
        let pending = Arc::new(Mutex::new(HashMap::new()));

        {
            let mut entries = self.entries.lock().expect("Runtime Bridge registry");
            if let Some(active) = entries.get(&observation.company) {
                if observation.runtime_generation < active.observation.runtime_generation {
                    bail!("Runtime Bridge registration carries a stale Runtime generation");
                }
                if observation.runtime_generation == active.observation.runtime_generation
                    && observation.runtime_id != active.observation.runtime_id
                {
                    bail!("Runtime Bridge generation conflicts with the active Runtime identity");
                }
            }
            entries.insert(
                observation.company.clone(),
                ActiveConnection {
                    connection_id,
                    observation: observation.clone(),
                    outbound,
                    pending: Arc::clone(&pending),
                },
            );
        }
        let _connection = ConnectionGuard {
            registry: self.clone(),
            company: observation.company.clone(),
            connection_id,
        };

        let accepted = Accepted {
            protocol_version: PROTOCOL_VERSION,
            status: "registered",
            connection_id,
            company_id: observation.company_id,
            cell_id: observation.cell_id,
            runtime_id: observation.runtime_id.clone(),
            runtime_generation: observation.runtime_generation,
        };
        socket
            .send(Message::Text(serde_json::to_string(&accepted)?.into()))
            .await
            .context("acknowledge Runtime Bridge registration")?;

        let (mut sink, mut source) = socket.split();
        loop {
            tokio::select! {
                outbound = outbound_rx.recv() => {
                    let Some(outbound) = outbound else { break };
                    sink.send(Message::Text(outbound.into()))
                        .await
                        .context("send Runtime Bridge command")?;
                }
                incoming = source.next() => {
                    let Some(incoming) = incoming else { break };
                    match incoming.context("read Runtime Bridge frame")? {
                        Message::Ping(payload) => sink
                            .send(Message::Pong(payload))
                            .await
                            .context("answer Runtime Bridge ping")?,
                        Message::Pong(_) => {}
                        Message::Close(_) => break,
                        Message::Text(raw) => {
                            if raw.len() > MAX_RESPONSE_BYTES {
                                bail!("Runtime Bridge response exceeds the bounded frame size");
                            }
                            let response: AgentResponse = serde_json::from_str(&raw)
                                .context("decode Runtime Bridge response")?;
                            if !matches!(
                                response.kind.as_str(),
                                "activity.result" | "file.result" | "process.result" | "error"
                            ) {
                                bail!("Runtime Bridge response kind is not implemented");
                            }
                            let sender = pending
                                .lock()
                                .expect("Runtime Bridge pending responses")
                                .remove(&response.operation_id)
                                .context("Runtime Bridge response has no pending operation")?;
                            let _ = sender.send(response);
                        }
                        Message::Binary(_) => {
                            bail!("Runtime Bridge sent an unsupported binary frame")
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[cfg(test)]
    fn observe(&self, company: &str) -> Option<Observation> {
        self.entries
            .lock()
            .expect("Runtime Bridge registry")
            .get(company)
            .map(|active| active.observation.clone())
    }

    pub(crate) fn observe_cell(&self, cell_id: Uuid) -> Option<Observation> {
        self.entries
            .lock()
            .expect("Runtime Bridge registry")
            .values()
            .find(|active| active.observation.cell_id == cell_id)
            .map(|active| active.observation.clone())
    }

    fn active_for_cell(&self, cell_id: Uuid) -> Result<ActiveConnection> {
        self.entries
            .lock()
            .expect("Runtime Bridge registry")
            .values()
            .find(|active| active.observation.cell_id == cell_id)
            .cloned()
            .context("Runtime Bridge is not connected")
    }

    async fn request(
        &self,
        active: &ActiveConnection,
        feature: &str,
        kind: &str,
        fields: serde_json::Value,
    ) -> Result<AgentResponse> {
        if !active
            .observation
            .supported_features
            .iter()
            .any(|candidate| candidate == feature)
        {
            bail!("Runtime Bridge does not implement the requested feature");
        }
        let Some(fields) = fields.as_object() else {
            bail!("Runtime Bridge command fields must be an object");
        };
        let operation_id = Uuid::new_v4();
        let mut command = fields.clone();
        command.insert("type".into(), serde_json::json!(kind));
        command.insert(
            "protocol_version".into(),
            serde_json::json!(PROTOCOL_VERSION),
        );
        command.insert("operation_id".into(), serde_json::json!(operation_id));
        command.insert(
            "runtime_id".into(),
            serde_json::json!(active.observation.runtime_id),
        );
        command.insert(
            "runtime_generation".into(),
            serde_json::json!(active.observation.runtime_generation),
        );
        let active = self
            .entries
            .lock()
            .expect("Runtime Bridge registry")
            .get(&active.observation.company)
            .filter(|current| current.connection_id == active.connection_id)
            .cloned()
            .context("Runtime Bridge connection changed before command dispatch")?;
        let (sender, receiver) = oneshot::channel();
        if active
            .pending
            .lock()
            .expect("Runtime Bridge pending responses")
            .insert(operation_id, sender)
            .is_some()
        {
            bail!("Runtime Bridge operation identity collision");
        }
        if active
            .outbound
            .send(serde_json::Value::Object(command).to_string())
            .await
            .is_err()
        {
            active
                .pending
                .lock()
                .expect("Runtime Bridge pending responses")
                .remove(&operation_id);
            bail!("Runtime Bridge disconnected before activity probe");
        }
        let response = match tokio::time::timeout(PROBE_TIMEOUT, receiver).await {
            Ok(Ok(response)) => response,
            _ => {
                active
                    .pending
                    .lock()
                    .expect("Runtime Bridge pending responses")
                    .remove(&operation_id);
                bail!("Runtime Bridge activity probe timed out");
            }
        };
        if response.runtime_id != active.observation.runtime_id
            || response.runtime_generation != active.observation.runtime_generation
        {
            bail!("Runtime Bridge response does not match the active Runtime");
        }
        Ok(response)
    }

    pub(crate) async fn probe_readiness(&self, cell_id: Uuid) -> Result<()> {
        let active = self.active_for_cell(cell_id)?;
        let activity = self
            .request(
                &active,
                "activity.v1",
                "activity.observe",
                serde_json::json!({}),
            )
            .await?;
        let observed_at = activity
            .body
            .get("observed_at")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| value.parse::<DateTime<Utc>>().ok())
            .context("Runtime Bridge activity response lacks observed_at")?;
        let age = Utc::now().signed_duration_since(observed_at);
        if activity.kind != "activity.result"
            || !activity
                .body
                .get("active_processes")
                .is_some_and(serde_json::Value::is_array)
            || age.num_seconds() < -5
            || age.num_seconds() > 5
        {
            bail!("Runtime Bridge activity response is invalid or stale");
        }

        if active
            .observation
            .supported_features
            .iter()
            .any(|feature| feature == "files.v1")
        {
            let file = self
                .request(
                    &active,
                    "files.v1",
                    "file.read",
                    serde_json::json!({"path":"/company/mission.md","max_bytes":4096}),
                )
                .await?;
            if file.kind != "file.result"
                || file.body.get("path").and_then(serde_json::Value::as_str)
                    != Some("/company/mission.md")
                || !file
                    .body
                    .get("sha256")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|digest| {
                        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
            {
                bail!("Runtime Bridge file probe is invalid");
            }
        }

        if active
            .observation
            .supported_features
            .iter()
            .any(|feature| feature == "process.v1")
        {
            let process = self
                .request(
                    &active,
                    "process.v1",
                    "process.run",
                    serde_json::json!({
                        "program":"/usr/bin/true",
                        "args":[],
                        "cwd":"/company",
                        "environment":{},
                        "timeout_ms":1000,
                        "max_output_bytes":1024
                    }),
                )
                .await?;
            if process.kind != "process.result"
                || process
                    .body
                    .get("exit_code")
                    .and_then(serde_json::Value::as_i64)
                    != Some(0)
                || process
                    .body
                    .get("timed_out")
                    .and_then(serde_json::Value::as_bool)
                    != Some(false)
            {
                bail!("Runtime Bridge process probe is invalid");
            }
        }
        Ok(())
    }
}

impl Observation {
    pub(crate) fn has_complete_v1(&self) -> bool {
        REQUIRED_FEATURES.iter().all(|required| {
            self.supported_features
                .iter()
                .any(|value| value == required)
        })
    }
}

fn authenticate_registration(
    registration: Registration,
    issuer: &CapabilityIssuer,
    scope: PlaneScope,
) -> Result<Observation> {
    if registration.protocol_version != PROTOCOL_VERSION {
        bail!("unsupported Runtime Bridge protocol version");
    }
    if registration.owner_id != scope.owner_id || registration.plane_id != scope.plane_id {
        bail!("Runtime Bridge owner or plane identity does not match this account plane");
    }
    if registration.company_id.is_nil()
        || registration.cell_id.is_nil()
        || registration.runtime_generation == 0
        || registration.desired_revision < 1
    {
        bail!("Runtime Bridge company, cell and generation identities must be non-zero");
    }
    validate_bounded_identity("company", &registration.company, 96)?;
    validate_bounded_identity("runtime_id", &registration.runtime_id, 160)?;
    validate_bounded_identity("volume_name", &registration.volume_name, 160)?;
    if !immutable_image(&registration.runtime_image) {
        bail!("Runtime Bridge image must be an immutable sha256 registry reference");
    }
    if registration.source_revision.len() != 40
        || !registration
            .source_revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        bail!("Runtime Bridge source revision must be an exact lowercase Git revision");
    }
    let features = registration
        .supported_features
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if features.len() != registration.supported_features.len()
        || !features.contains("registration.v1")
        || features
            .iter()
            .any(|feature| !KNOWN_FEATURES.contains(feature))
    {
        bail!("Runtime Bridge feature set is duplicate, unknown or lacks registration.v1");
    }
    let grant = issuer
        .verify_coordination(&registration.capability)
        .context("verify Runtime Bridge capability")?;
    if grant.company != registration.company || grant.actor != "exec" {
        bail!("Runtime Bridge capability does not match the registered company");
    }

    let mut supported_features = registration.supported_features;
    supported_features.sort();
    Ok(Observation {
        owner_id: registration.owner_id,
        plane_id: registration.plane_id,
        company_id: registration.company_id,
        cell_id: registration.cell_id,
        company: registration.company,
        runtime_id: registration.runtime_id,
        runtime_generation: registration.runtime_generation,
        desired_revision: registration.desired_revision,
        runtime_image: registration.runtime_image,
        volume_name: registration.volume_name,
        persistent_volume_ready: registration.persistent_volume_ready,
        source_revision: registration.source_revision,
        supported_features,
        connected_at: Utc::now(),
    })
}

fn validate_bounded_identity(name: &str, value: &str, max: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > max
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        bail!("Runtime Bridge {name} is not a bounded identity");
    }
    Ok(())
}

fn immutable_image(value: &str) -> bool {
    let Some((repository, digest)) = value.rsplit_once("@sha256:") else {
        return false;
    };
    !repository.is_empty()
        && !repository.contains(char::is_whitespace)
        && digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::{authenticate_registration, PlaneScope, Registration, Registry, REQUIRED_FEATURES};
    use crate::capability::CapabilityIssuer;
    use axum::{
        extract::{ws::WebSocketUpgrade, State},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use futures_util::{SinkExt as _, StreamExt as _};
    use tokio_tungstenite::tungstenite::Message as ClientMessage;
    use uuid::Uuid;

    #[derive(Clone)]
    struct TestState {
        registry: Registry,
        issuer: CapabilityIssuer,
        scope: PlaneScope,
    }

    async fn bridge(
        State(state): State<TestState>,
        upgrade: WebSocketUpgrade,
    ) -> impl IntoResponse {
        upgrade.on_upgrade(move |socket| async move {
            state
                .registry
                .accept(socket, state.issuer, state.scope)
                .await
                .unwrap();
        })
    }

    fn fixture() -> (
        tempfile::TempDir,
        CapabilityIssuer,
        PlaneScope,
        Registration,
    ) {
        let root = tempfile::tempdir().unwrap();
        let issuer = CapabilityIssuer::open(root.path()).unwrap();
        let scope = PlaneScope {
            owner_id: Uuid::new_v4(),
            plane_id: Uuid::new_v4(),
        };
        let registration = Registration {
            protocol_version: 1,
            owner_id: scope.owner_id,
            plane_id: scope.plane_id,
            company_id: Uuid::new_v4(),
            cell_id: Uuid::new_v4(),
            company: "hosted_test".into(),
            runtime_id: "restless-cell-runtime-1".into(),
            runtime_generation: 7,
            desired_revision: 3,
            runtime_image: format!("ghcr.io/example/runtime@sha256:{}", "a".repeat(64)),
            volume_name: "restless-cell-volume-1".into(),
            persistent_volume_ready: true,
            source_revision: "b".repeat(40),
            supported_features: std::iter::once("registration.v1")
                .chain(REQUIRED_FEATURES.iter().copied())
                .map(str::to_string)
                .collect(),
            capability: issuer.issue_runtime_bridge("hosted_test").unwrap(),
        };
        (root, issuer, scope, registration)
    }

    #[test]
    fn registration_is_bound_to_plane_company_release_and_full_protocol() {
        let (_root, issuer, scope, registration) = fixture();
        let observation = authenticate_registration(registration, &issuer, scope).unwrap();
        assert_eq!(observation.company, "hosted_test");
        assert_eq!(observation.runtime_generation, 7);
        assert_eq!(observation.supported_features.len(), 6);
        assert!(observation.has_complete_v1());
    }

    #[test]
    fn registration_refuses_foreign_scope_mutable_images_and_partial_agents() {
        let (_root, issuer, scope, mut registration) = fixture();
        registration.plane_id = Uuid::new_v4();
        assert!(authenticate_registration(registration, &issuer, scope).is_err());

        let (_root, issuer, scope, mut registration) = fixture();
        registration.runtime_image = "ghcr.io/example/runtime:latest".into();
        assert!(authenticate_registration(registration, &issuer, scope).is_err());

        let (_root, issuer, scope, mut registration) = fixture();
        registration
            .supported_features
            .push("host-control.v1".into());
        assert!(authenticate_registration(registration, &issuer, scope).is_err());

        let (_root, issuer, scope, mut registration) = fixture();
        registration.supported_features = vec!["registration.v1".into()];
        let observation = authenticate_registration(registration, &issuer, scope).unwrap();
        assert!(!observation.has_complete_v1());
    }

    #[test]
    fn a_runtime_capability_cannot_register_another_company() {
        let (_root, issuer, scope, mut registration) = fixture();
        registration.company = "foreign_test".into();
        assert!(authenticate_registration(registration, &issuer, scope).is_err());
    }

    #[test]
    fn registry_starts_empty() {
        assert!(Registry::default().observe("hosted_test").is_none());
    }

    #[tokio::test]
    async fn outbound_websocket_registration_is_observed_and_disconnect_is_removed() {
        let (_root, issuer, scope, mut registration) = fixture();
        registration.supported_features = vec!["registration.v1".into(), "activity.v1".into()];
        let registry = Registry::default();
        let app = Router::new()
            .route("/bridge", get(bridge))
            .with_state(TestState {
                registry: registry.clone(),
                issuer,
                scope,
            });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let (mut client, _) = tokio_tungstenite::connect_async(format!("ws://{address}/bridge"))
            .await
            .unwrap();
        client
            .send(ClientMessage::Text(
                serde_json::to_string(&registration).unwrap().into(),
            ))
            .await
            .unwrap();
        let accepted = client.next().await.unwrap().unwrap();
        let accepted = accepted.into_text().unwrap();
        let accepted: serde_json::Value = serde_json::from_str(&accepted).unwrap();
        assert_eq!(accepted["status"], "registered");
        assert_eq!(accepted["runtime_generation"], 7);
        assert_eq!(
            registry.observe("hosted_test").unwrap().runtime_id,
            "restless-cell-runtime-1"
        );

        let probe_registry = registry.clone();
        let cell_id = registration.cell_id;
        let probe = tokio::spawn(async move { probe_registry.probe_readiness(cell_id).await });
        let command = client.next().await.unwrap().unwrap().into_text().unwrap();
        let command: serde_json::Value = serde_json::from_str(&command).unwrap();
        assert_eq!(command["type"], "activity.observe");
        client
            .send(ClientMessage::Text(
                serde_json::json!({
                    "type": "activity.result",
                    "operation_id": command["operation_id"],
                    "runtime_id": registration.runtime_id,
                    "runtime_generation": registration.runtime_generation,
                    "active_processes": [],
                    "observed_at": chrono::Utc::now(),
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
        probe.await.unwrap().unwrap();

        client.close(None).await.unwrap();
        for _ in 0..20 {
            if registry.observe("hosted_test").is_none() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(registry.observe("hosted_test").is_none());
        server.abort();
    }
}
