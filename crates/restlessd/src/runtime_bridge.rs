//! Hosted Company Runtime bridge.
//!
//! Fleet may provision an account plane without a Docker socket. The company
//! Runtime therefore opens one authenticated, outbound websocket to Core.
//! This module owns the machine-to-machine bootstrap and exact live bridge
//! identity; it deliberately exposes no generic remote-shell operation.

use std::fs;
use std::io::Read as _;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use axum::body::{Body, Bytes};
use axum::extract::ws::{Message as WebSocketMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Extension, OriginalUri};
use axum::http::header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, HOST};
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode};
use axum::response::IntoResponse as _;
use axum::routing::{get, post};
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use futures_util::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::capability::HostedRuntimeBridgeScope;
use crate::entry::EntryMode;
use crate::{release, Daemon};

pub(crate) const RUNTIME_BRIDGE_BOOTSTRAP_PATH: &str = "/internal/v1/runtime-bridge/bootstrap";
pub(crate) const RUNTIME_BRIDGE_PATH: &str = "/internal/v1/runtime-bridge";
const RUNTIME_BRIDGE_CONTRACT_VERSION: u32 = 1;
const MAX_BOOTSTRAP_REQUEST_BYTES: usize = 8 * 1024;
const TOKEN_FILE_ENV: &str = "RESTLESS_RUNTIME_BOOTSTRAP_TOKEN_FILE";
const COMPANY_IMAGE_ENV: &str = "RESTLESS_COMPANY_IMAGE";
const CAPABILITY_LIFETIME_SECONDS: u64 = 86_400;
const REGISTER_DEADLINE: Duration = Duration::from_secs(10);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
const BRIDGE_FRESHNESS: Duration = Duration::from_secs(35);
const OUTBOUND_QUEUE_DEPTH: usize = 64;
const OPERATION_TOMBSTONE_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_OPERATION_TOMBSTONES: usize = 4_096;

use restless_runtime_bridge_protocol::{
    ArtifactReceipt, ArtifactSelector, AttemptCleanupReceipt, CandidateIdentity, Frame,
    GateDefinition, GateReceipt, GateResources, Message as BridgeMessage, PromotionReceipt,
    ReleaseIdentity, ReviewCopyReceipt, RuntimeIdentity, SequenceTracker, WorkspaceIdentity,
    WorkspaceObservation, MAX_FRAME_BYTES, PROTOCOL_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BridgeReadiness {
    Ready,
    Missing,
    Stale,
    IdentityMismatch,
}

#[derive(Clone)]
struct ConnectedBridge {
    connection_id: Uuid,
    connection_nonce: Uuid,
    identity: RuntimeIdentity,
    release: ReleaseIdentity,
    credential_id: Uuid,
    credential_epoch: i64,
    credential_expires_at: chrono::DateTime<chrono::Utc>,
    last_seen: Instant,
    next_sequence: u64,
    sender: mpsc::Sender<Frame>,
}

#[derive(Clone)]
struct PendingOperation {
    cell_id: Uuid,
    connection_id: Uuid,
    sender: mpsc::Sender<BridgeMessage>,
}

#[derive(Default)]
struct RegistryState {
    connections: std::collections::HashMap<Uuid, ConnectedBridge>,
    operations: std::collections::HashMap<Uuid, PendingOperation>,
    operation_tombstones: std::collections::HashMap<Uuid, Instant>,
}

/// Ephemeral transport state only. Durable company/generation identity lives
/// in Authority and signed capabilities; a process restart requires an exact
/// Runtime to register again before readiness can become true.
#[derive(Clone, Default)]
pub(crate) struct RuntimeBridgeRegistry {
    state: Arc<std::sync::Mutex<RegistryState>>,
    hosted: bool,
}

impl RuntimeBridgeRegistry {
    pub(crate) fn hosted() -> Self {
        Self {
            state: Arc::default(),
            hosted: true,
        }
    }

    pub(crate) fn is_hosted(&self) -> bool {
        self.hosted
    }
    fn register(
        &self,
        identity: RuntimeIdentity,
        connection_nonce: Uuid,
        release: ReleaseIdentity,
        credential_id: Uuid,
        credential_epoch: i64,
        credential_expires_at: chrono::DateTime<chrono::Utc>,
        sender: mpsc::Sender<Frame>,
    ) -> std::result::Result<Uuid, &'static str> {
        let connection_id = Uuid::new_v4();
        let mut state = self.state.lock().expect("Runtime bridge registry");
        prune_operation_tombstones(&mut state);
        let replaced_connection_id = if let Some(current) = state.connections.get(&identity.cell_id)
        {
            if current.identity.company_id != identity.company_id
                || current.identity.owner_id != identity.owner_id
                || current.identity.plane_id != identity.plane_id
                || current.identity.runtime_generation > identity.runtime_generation
            {
                return Err("stale_or_conflicting_runtime");
            }
            // A reconnect is an atomic replacement. The old reader may later
            // observe its socket close, but `remove` is fenced by connection
            // id so it cannot erase this new live bridge.
            let _ = current.sender.try_send(Frame {
                protocol_version: PROTOCOL_VERSION,
                sequence: current.next_sequence,
                identity: current.identity.clone(),
                message: BridgeMessage::Error {
                    operation_id: None,
                    code: "bridge_replaced".into(),
                    message: "a newer exact Runtime bridge connection registered".into(),
                },
            });
            Some(current.connection_id)
        } else {
            None
        };
        if let Some(replaced_connection_id) = replaced_connection_id {
            let replaced = state
                .operations
                .iter()
                .filter_map(|(operation_id, pending)| {
                    (pending.connection_id == replaced_connection_id).then_some(*operation_id)
                })
                .collect::<Vec<_>>();
            for operation_id in replaced {
                if let Some(pending) = state.operations.remove(&operation_id) {
                    let _ = pending.sender.try_send(BridgeMessage::Error {
                        operation_id: Some(operation_id),
                        code: "bridge_replaced".into(),
                        message: "the Runtime connection changed before this operation settled"
                            .into(),
                    });
                    remember_operation_tombstone(&mut state, operation_id);
                }
            }
        }
        state.connections.insert(
            identity.cell_id,
            ConnectedBridge {
                connection_id,
                connection_nonce,
                identity,
                release,
                credential_id,
                credential_epoch,
                credential_expires_at,
                last_seen: Instant::now(),
                next_sequence: 2,
                sender,
            },
        );
        Ok(connection_id)
    }

    fn heartbeat(&self, cell_id: Uuid, connection_id: Uuid, nonce: Uuid) -> bool {
        let mut state = self.state.lock().expect("Runtime bridge registry");
        let Some(bridge) = state.connections.get_mut(&cell_id) else {
            return false;
        };
        if bridge.connection_id != connection_id || bridge.connection_nonce != nonce {
            return false;
        }
        bridge.last_seen = Instant::now();
        true
    }

    fn remove(&self, cell_id: Uuid, connection_id: Uuid) {
        let mut state = self.state.lock().expect("Runtime bridge registry");
        prune_operation_tombstones(&mut state);
        if state
            .connections
            .get(&cell_id)
            .is_some_and(|bridge| bridge.connection_id == connection_id)
        {
            state.connections.remove(&cell_id);
            let lost = state
                .operations
                .iter()
                .filter_map(|(operation_id, pending)| {
                    (pending.connection_id == connection_id).then_some(*operation_id)
                })
                .collect::<Vec<_>>();
            for operation_id in lost {
                if let Some(pending) = state.operations.remove(&operation_id) {
                    let _ = pending.sender.try_send(BridgeMessage::Error {
                        operation_id: Some(operation_id),
                        code: "bridge_connection_lost".into(),
                        message: "the exact Runtime bridge connection was lost".into(),
                    });
                    remember_operation_tombstone(&mut state, operation_id);
                }
            }
        }
    }

    pub(crate) fn readiness(&self, expected: &RuntimeIdentity) -> BridgeReadiness {
        let state = self.state.lock().expect("Runtime bridge registry");
        let Some(bridge) = state.connections.get(&expected.cell_id) else {
            return BridgeReadiness::Missing;
        };
        if bridge.identity != *expected
            || bridge.release.source_revision != expected.source_revision
            || bridge.release.api_contract_version != release::API_CONTRACT_VERSION
            || bridge.release.assertion_contract_version != crate::entry::ASSERTION_CONTRACT_VERSION
            || bridge.release.schema_version != i64::from(release::SCHEMA_VERSION)
        {
            return BridgeReadiness::IdentityMismatch;
        }
        if bridge.credential_expires_at <= chrono::Utc::now() {
            return BridgeReadiness::Stale;
        }
        if bridge.last_seen.elapsed() > BRIDGE_FRESHNESS {
            return BridgeReadiness::Stale;
        }
        BridgeReadiness::Ready
    }

    pub(crate) fn begin_operation(
        &self,
        expected: &RuntimeIdentity,
        message: BridgeMessage,
    ) -> Result<mpsc::Receiver<BridgeMessage>> {
        let operation_id =
            operation_id(&message).context("Runtime bridge request has no operation identity")?;
        let mut state = self.state.lock().expect("Runtime bridge registry");
        prune_operation_tombstones(&mut state);
        if state.operations.contains_key(&operation_id) {
            anyhow::bail!("duplicate Runtime bridge operation {operation_id}");
        }
        if state.operation_tombstones.contains_key(&operation_id) {
            if matches!(
                &message,
                BridgeMessage::PrepareWorkspace { .. }
                    | BridgeMessage::ObserveWorkspace { .. }
                    | BridgeMessage::RunGate { .. }
                    | BridgeMessage::ObserveArtifact { .. }
                    | BridgeMessage::PromoteCandidate { .. }
                    | BridgeMessage::PrepareReviewCopy { .. }
                    | BridgeMessage::CleanupAttempt { .. }
            ) {
                // Workspace operations are stable, idempotent commands. A
                // reconnect gets a new connection fence and may replay the
                // same operation id; a late result from the prior connection
                // cannot satisfy the new pending operation.
                state.operation_tombstones.remove(&operation_id);
            } else {
                anyhow::bail!("duplicate Runtime bridge operation {operation_id}");
            }
        }
        let bridge = state
            .connections
            .get_mut(&expected.cell_id)
            .context("exact Runtime bridge is not registered")?;
        if bridge.identity != *expected
            || bridge.last_seen.elapsed() > BRIDGE_FRESHNESS
            || bridge.credential_expires_at <= chrono::Utc::now()
        {
            anyhow::bail!("exact Runtime bridge is not freshly ready");
        }
        let connection_id = bridge.connection_id;
        let frame = Frame {
            protocol_version: PROTOCOL_VERSION,
            sequence: bridge.next_sequence,
            identity: expected.clone(),
            message,
        };
        bridge.next_sequence = bridge.next_sequence.saturating_add(1);
        let bridge_sender = bridge.sender.clone();
        let (sender, receiver) = mpsc::channel(OUTBOUND_QUEUE_DEPTH);
        state.operations.insert(
            operation_id,
            PendingOperation {
                cell_id: expected.cell_id,
                connection_id,
                sender,
            },
        );
        if bridge_sender.try_send(frame).is_err() {
            state.operations.remove(&operation_id);
            anyhow::bail!("exact Runtime bridge is backpressured or disconnected");
        }
        Ok(receiver)
    }

    pub(crate) fn send_operation(
        &self,
        expected: &RuntimeIdentity,
        message: BridgeMessage,
    ) -> Result<()> {
        let operation_id =
            operation_id(&message).context("Runtime bridge request has no operation identity")?;
        let mut state = self.state.lock().expect("Runtime bridge registry");
        let pending = state
            .operations
            .get(&operation_id)
            .context("Runtime bridge operation is not active")?
            .clone();
        if pending.cell_id != expected.cell_id {
            anyhow::bail!("Runtime bridge operation belongs to another cell");
        }
        let bridge = state
            .connections
            .get_mut(&expected.cell_id)
            .context("exact Runtime bridge disconnected")?;
        if bridge.connection_id != pending.connection_id
            || bridge.identity != *expected
            || bridge.last_seen.elapsed() > BRIDGE_FRESHNESS
            || bridge.credential_expires_at <= chrono::Utc::now()
        {
            anyhow::bail!("Runtime bridge operation connection changed");
        }
        let frame = Frame {
            protocol_version: PROTOCOL_VERSION,
            sequence: bridge.next_sequence,
            identity: expected.clone(),
            message,
        };
        bridge.next_sequence = bridge.next_sequence.saturating_add(1);
        bridge
            .sender
            .try_send(frame)
            .map_err(|_| anyhow::anyhow!("exact Runtime bridge is backpressured or disconnected"))
    }

    fn send_untracked(&self, expected: &RuntimeIdentity, message: BridgeMessage) -> Result<()> {
        let mut state = self.state.lock().expect("Runtime bridge registry");
        let bridge = state
            .connections
            .get_mut(&expected.cell_id)
            .context("exact Runtime bridge disconnected")?;
        if bridge.identity != *expected
            || bridge.last_seen.elapsed() > BRIDGE_FRESHNESS
            || bridge.credential_expires_at <= chrono::Utc::now()
        {
            anyhow::bail!("exact Runtime bridge is not freshly ready");
        }
        let frame = Frame {
            protocol_version: PROTOCOL_VERSION,
            sequence: bridge.next_sequence,
            identity: expected.clone(),
            message,
        };
        bridge.next_sequence = bridge.next_sequence.saturating_add(1);
        bridge
            .sender
            .try_send(frame)
            .map_err(|_| anyhow::anyhow!("exact Runtime bridge is backpressured or disconnected"))
    }

    fn deliver(&self, cell_id: Uuid, connection_id: Uuid, message: BridgeMessage) -> bool {
        let Some(operation_id) = operation_id(&message) else {
            return false;
        };
        let terminal = matches!(
            message,
            BridgeMessage::Exit { .. }
                | BridgeMessage::PreflightResult { .. }
                | BridgeMessage::WorkspacePrepared { .. }
                | BridgeMessage::WorkspaceObserved { .. }
                | BridgeMessage::GateCompleted { .. }
                | BridgeMessage::ArtifactObserved { .. }
                | BridgeMessage::CandidatePromoted { .. }
                | BridgeMessage::ReviewCopyPrepared { .. }
                | BridgeMessage::AttemptCleaned { .. }
                | BridgeMessage::CoordinationResponse { .. }
                | BridgeMessage::Error { .. }
        );
        let mut state = self.state.lock().expect("Runtime bridge registry");
        prune_operation_tombstones(&mut state);
        let Some(pending) = state.operations.get(&operation_id) else {
            return terminal && state.operation_tombstones.contains_key(&operation_id);
        };
        if pending.cell_id != cell_id || pending.connection_id != connection_id {
            return false;
        }
        let sender = pending.sender.clone();
        if terminal {
            state.operations.remove(&operation_id);
            remember_operation_tombstone(&mut state, operation_id);
        }
        sender.try_send(message).is_ok()
    }

    pub(crate) fn abandon_operation(&self, operation_id: Uuid) {
        let mut state = self.state.lock().expect("Runtime bridge registry");
        prune_operation_tombstones(&mut state);
        state.operations.remove(&operation_id);
        remember_operation_tombstone(&mut state, operation_id);
    }

    fn retire_if_not_authorized(
        &self,
        expected: &RuntimeIdentity,
        credential_id: Uuid,
        credential_epoch: i64,
    ) {
        let connection_id = self
            .state
            .lock()
            .expect("Runtime bridge registry")
            .connections
            .get(&expected.cell_id)
            .filter(|bridge| {
                bridge.identity != *expected
                    || bridge.credential_id != credential_id
                    || bridge.credential_epoch != credential_epoch
            })
            .map(|bridge| bridge.connection_id);
        if let Some(connection_id) = connection_id {
            self.remove(expected.cell_id, connection_id);
        }
    }

    fn connection_is_current_and_fresh(&self, cell_id: Uuid, connection_id: Uuid) -> bool {
        self.state
            .lock()
            .expect("Runtime bridge registry")
            .connections
            .get(&cell_id)
            .is_some_and(|bridge| {
                bridge.connection_id == connection_id
                    && bridge.last_seen.elapsed() <= BRIDGE_FRESHNESS
                    && bridge.credential_expires_at > chrono::Utc::now()
            })
    }
}

fn prune_operation_tombstones(state: &mut RegistryState) {
    state
        .operation_tombstones
        .retain(|_, inserted| inserted.elapsed() <= OPERATION_TOMBSTONE_TTL);
    if state.operation_tombstones.len() > MAX_OPERATION_TOMBSTONES {
        let mut oldest = state
            .operation_tombstones
            .iter()
            .map(|(operation_id, inserted)| (*operation_id, *inserted))
            .collect::<Vec<_>>();
        oldest.sort_unstable_by_key(|(_, inserted)| *inserted);
        for (operation_id, _) in oldest
            .into_iter()
            .take(state.operation_tombstones.len() - MAX_OPERATION_TOMBSTONES)
        {
            state.operation_tombstones.remove(&operation_id);
        }
    }
}

fn remember_operation_tombstone(state: &mut RegistryState, operation_id: Uuid) {
    state
        .operation_tombstones
        .insert(operation_id, Instant::now());
    prune_operation_tombstones(state);
}

fn operation_id(message: &BridgeMessage) -> Option<Uuid> {
    match message {
        BridgeMessage::PreflightRequest { operation_id, .. }
        | BridgeMessage::PreflightResult { operation_id, .. }
        | BridgeMessage::CreateRepository { operation_id, .. }
        | BridgeMessage::RepositoryCreated { operation_id, .. }
        | BridgeMessage::PrepareWorkspace { operation_id, .. }
        | BridgeMessage::WorkspacePrepared { operation_id, .. }
        | BridgeMessage::ObserveWorkspace { operation_id, .. }
        | BridgeMessage::WorkspaceObserved { operation_id, .. }
        | BridgeMessage::RunGate { operation_id, .. }
        | BridgeMessage::GateCompleted { operation_id, .. }
        | BridgeMessage::ObserveArtifact { operation_id, .. }
        | BridgeMessage::ArtifactObserved { operation_id, .. }
        | BridgeMessage::PromoteCandidate { operation_id, .. }
        | BridgeMessage::CandidatePromoted { operation_id, .. }
        | BridgeMessage::PrepareReviewCopy { operation_id, .. }
        | BridgeMessage::ReviewCopyPrepared { operation_id, .. }
        | BridgeMessage::CleanupAttempt { operation_id, .. }
        | BridgeMessage::AttemptCleaned { operation_id, .. }
        | BridgeMessage::LaunchAgent { operation_id, .. }
        | BridgeMessage::AcpStdin { operation_id, .. }
        | BridgeMessage::AcpStdout { operation_id, .. }
        | BridgeMessage::Cancel { operation_id, .. }
        | BridgeMessage::Exit { operation_id, .. }
        | BridgeMessage::CoordinationRequest { operation_id, .. }
        | BridgeMessage::CoordinationResponse { operation_id, .. } => Some(*operation_id),
        BridgeMessage::Error { operation_id, .. } => *operation_id,
        BridgeMessage::Register { .. }
        | BridgeMessage::RegisterAck { .. }
        | BridgeMessage::Heartbeat { .. } => None,
    }
}

pub(crate) async fn expected_identity(
    authority: &crate::authority::AuthorityStore,
    company: &str,
) -> Result<RuntimeIdentity> {
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Uuid,
            Uuid,
            String,
            String,
            i64,
            String,
            String,
            String,
        ),
    >(
        "SELECT owner_id,plane_id,company_id,cell_id,company_handle,runtime_id,runtime_generation,\
                runtime_image,volume_name,source_revision \
         FROM restless_authority.runtime_bridge_generations WHERE company_handle=$1",
    )
    .bind(company)
    .fetch_optional(authority.pool())
    .await
    .context("read expected hosted Runtime identity")?
    .with_context(|| format!("company {company:?} has no bootstrapped hosted Runtime"))?;
    Ok(RuntimeIdentity {
        owner_id: row.0,
        plane_id: row.1,
        company_id: row.2,
        cell_id: row.3,
        company: row.4,
        runtime_id: row.5,
        runtime_generation: row.6,
        runtime_image: row.7,
        volume_name: row.8,
        source_revision: row.9,
    })
}

#[derive(Clone, Debug)]
pub(crate) struct RuntimePreflightObservation {
    pub(crate) release: ReleaseIdentity,
    pub(crate) disk_available_bytes: u64,
    /// Core's receipt time for the bounded round trip. The Runtime's clock is
    /// deliberately not trusted as the freshness authority.
    pub(crate) observed_at: chrono::DateTime<chrono::Utc>,
}

pub(crate) async fn preflight_observation(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
) -> Result<RuntimePreflightObservation> {
    if registry.readiness(identity) != BridgeReadiness::Ready {
        anyhow::bail!("exact hosted Runtime bridge is not freshly ready");
    }
    let operation_id = Uuid::new_v4();
    let deadline = chrono::Utc::now().timestamp_millis() + 10_000;
    let mut responses = registry.begin_operation(
        identity,
        BridgeMessage::PreflightRequest {
            operation_id,
            deadline_ms: deadline,
        },
    )?;
    let response = tokio::time::timeout(Duration::from_secs(10), responses.recv())
        .await
        .context("hosted Runtime preflight timed out")?
        .context("hosted Runtime preflight connection was lost")?;
    match response {
        BridgeMessage::PreflightResult {
            status: restless_runtime_bridge_protocol::PreflightStatus::Ready,
            release: observed,
            disk_available_bytes,
            ..
        } if observed.source_revision == identity.source_revision
            && observed.api_contract_version == release::API_CONTRACT_VERSION
            && observed.assertion_contract_version == crate::entry::ASSERTION_CONTRACT_VERSION
            && observed.schema_version == i64::from(release::SCHEMA_VERSION) =>
        {
            Ok(RuntimePreflightObservation {
                release: observed,
                disk_available_bytes,
                observed_at: chrono::Utc::now(),
            })
        }
        BridgeMessage::PreflightResult { error, .. } => anyhow::bail!(
            "hosted Runtime preflight refused: {}",
            error.unwrap_or_else(|| "release or health mismatch".into())
        ),
        BridgeMessage::Error { message, .. } => {
            anyhow::bail!("hosted Runtime preflight failed: {message}")
        }
        _ => anyhow::bail!("hosted Runtime preflight returned the wrong response"),
    }
}

pub(crate) async fn preflight(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
) -> Result<()> {
    preflight_observation(registry, identity).await.map(drop)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedRuntimeWorkspace {
    pub(crate) observation: WorkspaceObservation,
    pub(crate) reused: bool,
}

pub(crate) async fn prepare_workspace(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
    workspace: WorkspaceIdentity,
    requested_source_ref: Option<String>,
    expected_source_commit: Option<String>,
    expected_source_tree: Option<String>,
) -> Result<PreparedRuntimeWorkspace> {
    let environment_fingerprint = ready_environment_fingerprint(registry, identity).await?;
    let operation_id = workspace_operation_id(identity, &workspace, "prepare");
    let deadline_ms = chrono::Utc::now().timestamp_millis() + 45_000;
    let request = BridgeMessage::PrepareWorkspace {
        operation_id,
        workspace: workspace.clone(),
        requested_source_ref: requested_source_ref.clone(),
        expected_source_commit: expected_source_commit.clone(),
        expected_source_tree: expected_source_tree.clone(),
        deadline_ms,
    };
    let response = run_workspace_operation(registry, identity, operation_id, request).await?;
    match response {
        BridgeMessage::WorkspacePrepared {
            workspace: observed_workspace,
            requested_source_ref: observed_ref,
            observation,
            reused,
            ..
        } if observed_workspace == workspace
            && observed_ref == requested_source_ref
            && observation.environment_fingerprint == environment_fingerprint
            && expected_source_commit
                .as_deref()
                .is_none_or(|expected| observation.source_commit == expected)
            && expected_source_tree
                .as_deref()
                .is_none_or(|expected| observation.source_tree == expected) =>
        {
            Ok(PreparedRuntimeWorkspace {
                observation,
                reused,
            })
        }
        BridgeMessage::WorkspacePrepared { .. } => {
            anyhow::bail!("hosted Runtime prepared different workspace coordinates")
        }
        BridgeMessage::Error { message, .. } => {
            anyhow::bail!("hosted Runtime workspace preparation failed: {message}")
        }
        _ => anyhow::bail!("hosted Runtime workspace preparation returned the wrong response"),
    }
}

pub(crate) async fn ready_environment_fingerprint(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
) -> Result<String> {
    let preflight = preflight_observation(registry, identity).await?;
    environment_fingerprint(identity, &preflight.release)
}

pub(crate) async fn observe_workspace(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
    workspace: WorkspaceIdentity,
    expected_environment_fingerprint: &str,
) -> Result<WorkspaceObservation> {
    if registry.readiness(identity) != BridgeReadiness::Ready {
        anyhow::bail!("exact hosted Runtime bridge is not freshly ready");
    }
    let operation_id = workspace_operation_id(identity, &workspace, "observe");
    let request = BridgeMessage::ObserveWorkspace {
        operation_id,
        workspace: workspace.clone(),
        expected_environment_fingerprint: expected_environment_fingerprint.to_string(),
        deadline_ms: chrono::Utc::now().timestamp_millis() + 45_000,
    };
    let response = run_workspace_operation(registry, identity, operation_id, request).await?;
    match response {
        BridgeMessage::WorkspaceObserved {
            workspace: observed_workspace,
            observation,
            ..
        } if observed_workspace == workspace
            && observation.environment_fingerprint == expected_environment_fingerprint =>
        {
            Ok(observation)
        }
        BridgeMessage::WorkspaceObserved { .. } => {
            anyhow::bail!("hosted Runtime observed different workspace coordinates")
        }
        BridgeMessage::Error { message, .. } => {
            anyhow::bail!("hosted Runtime workspace observation failed: {message}")
        }
        _ => anyhow::bail!("hosted Runtime workspace observation returned the wrong response"),
    }
}

pub(crate) fn gate_operation_id(
    identity: &RuntimeIdentity,
    workspace: &WorkspaceIdentity,
    candidate: &CandidateIdentity,
    definition_digest: &str,
) -> Uuid {
    typed_operation_id(
        identity,
        workspace,
        "gate",
        &serde_json::json!({
            "candidate": candidate,
            "definition_digest": definition_digest,
        }),
    )
}

pub(crate) async fn run_gate(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
    workspace: WorkspaceIdentity,
    candidate: CandidateIdentity,
    definition: GateDefinition,
    resources: GateResources,
) -> Result<GateReceipt> {
    let definition_digest = restless_runtime_bridge_protocol::gate_definition_digest(&definition);
    let operation_id = gate_operation_id(identity, &workspace, &candidate, &definition_digest);
    let timeout_seconds = u64::from(definition.timeout_seconds).saturating_add(15);
    let request = BridgeMessage::RunGate {
        operation_id,
        workspace: workspace.clone(),
        candidate: candidate.clone(),
        definition: definition.clone(),
        definition_digest: definition_digest.clone(),
        resources: resources.clone(),
        deadline_ms: chrono::Utc::now().timestamp_millis()
            + i64::try_from(timeout_seconds.saturating_mul(1_000)).unwrap_or(i64::MAX),
    };
    let response = run_typed_operation(
        registry,
        identity,
        operation_id,
        request,
        Duration::from_secs(timeout_seconds),
        "gate",
    )
    .await?;
    match response {
        BridgeMessage::GateCompleted {
            workspace: observed_workspace,
            receipt,
            ..
        } if observed_workspace == workspace
            && receipt.candidate == candidate
            && receipt.gate_id == definition.gate_id
            && receipt.definition_digest == definition_digest
            && receipt.resources == resources =>
        {
            Ok(receipt)
        }
        BridgeMessage::GateCompleted { .. } => {
            anyhow::bail!("hosted Runtime gate receipt names different frozen evidence")
        }
        BridgeMessage::Error { message, .. } => {
            anyhow::bail!("hosted Runtime gate failed: {message}")
        }
        _ => anyhow::bail!("hosted Runtime gate returned the wrong response"),
    }
}

pub(crate) async fn observe_artifact(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
    workspace: WorkspaceIdentity,
    candidate: CandidateIdentity,
    selector: ArtifactSelector,
) -> Result<ArtifactReceipt> {
    let operation_id = typed_operation_id(
        identity,
        &workspace,
        "artifact",
        &serde_json::json!({"candidate": candidate, "selector": selector}),
    );
    let request = BridgeMessage::ObserveArtifact {
        operation_id,
        workspace: workspace.clone(),
        candidate: candidate.clone(),
        selector: selector.clone(),
        deadline_ms: chrono::Utc::now().timestamp_millis() + 120_000,
    };
    let response = run_typed_operation(
        registry,
        identity,
        operation_id,
        request,
        Duration::from_secs(125),
        "artifact observation",
    )
    .await?;
    match response {
        BridgeMessage::ArtifactObserved {
            workspace: observed_workspace,
            receipt,
            ..
        } if observed_workspace == workspace
            && receipt.candidate == candidate
            && receipt.selector == selector =>
        {
            Ok(receipt)
        }
        BridgeMessage::ArtifactObserved { .. } => {
            anyhow::bail!("hosted Runtime artifact receipt names different frozen evidence")
        }
        BridgeMessage::Error { message, .. } => {
            anyhow::bail!("hosted Runtime artifact observation failed: {message}")
        }
        _ => anyhow::bail!("hosted Runtime artifact observation returned the wrong response"),
    }
}

pub(crate) async fn promote_candidate(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
    workspace: WorkspaceIdentity,
    candidate: CandidateIdentity,
    integration_branch: String,
) -> Result<PromotionReceipt> {
    let operation_id = typed_operation_id(
        identity,
        &workspace,
        "promotion",
        &serde_json::json!({
            "candidate": candidate,
            "integration_branch": integration_branch,
        }),
    );
    let request = BridgeMessage::PromoteCandidate {
        operation_id,
        workspace: workspace.clone(),
        candidate: candidate.clone(),
        integration_branch: integration_branch.clone(),
        deadline_ms: chrono::Utc::now().timestamp_millis() + 120_000,
    };
    let response = run_typed_operation(
        registry,
        identity,
        operation_id,
        request,
        Duration::from_secs(125),
        "candidate promotion",
    )
    .await?;
    match response {
        BridgeMessage::CandidatePromoted {
            workspace: observed_workspace,
            receipt,
            ..
        } if observed_workspace == workspace
            && receipt.candidate == candidate
            && receipt.integration_branch == integration_branch =>
        {
            Ok(receipt)
        }
        BridgeMessage::CandidatePromoted { .. } => {
            anyhow::bail!("hosted Runtime promotion receipt names different frozen evidence")
        }
        BridgeMessage::Error { message, .. } => {
            anyhow::bail!("hosted Runtime candidate promotion failed: {message}")
        }
        _ => anyhow::bail!("hosted Runtime candidate promotion returned the wrong response"),
    }
}

pub(crate) async fn prepare_review_copy(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
    workspace: WorkspaceIdentity,
    candidate: CandidateIdentity,
) -> Result<ReviewCopyReceipt> {
    let operation_id = typed_operation_id(
        identity,
        &workspace,
        "review-copy",
        &serde_json::json!({"candidate": candidate}),
    );
    let request = BridgeMessage::PrepareReviewCopy {
        operation_id,
        workspace: workspace.clone(),
        candidate: candidate.clone(),
        deadline_ms: chrono::Utc::now().timestamp_millis() + 180_000,
    };
    let response = run_typed_operation(
        registry,
        identity,
        operation_id,
        request,
        Duration::from_secs(185),
        "review-copy preparation",
    )
    .await?;
    match response {
        BridgeMessage::ReviewCopyPrepared {
            workspace: observed_workspace,
            receipt,
            ..
        } if observed_workspace == workspace && receipt.candidate == candidate => Ok(receipt),
        BridgeMessage::ReviewCopyPrepared { .. } => {
            anyhow::bail!("hosted Runtime review receipt names different frozen evidence")
        }
        BridgeMessage::Error { message, .. } => {
            anyhow::bail!("hosted Runtime review-copy preparation failed: {message}")
        }
        _ => anyhow::bail!("hosted Runtime review-copy preparation returned the wrong response"),
    }
}

pub(crate) async fn cleanup_attempt(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
    workspace: WorkspaceIdentity,
    expected_environment_fingerprint: String,
) -> Result<AttemptCleanupReceipt> {
    let operation_id = typed_operation_id(
        identity,
        &workspace,
        "cleanup",
        &serde_json::json!({
            "environment_fingerprint": expected_environment_fingerprint,
        }),
    );
    let request = BridgeMessage::CleanupAttempt {
        operation_id,
        workspace: workspace.clone(),
        expected_environment_fingerprint: expected_environment_fingerprint.clone(),
        deadline_ms: chrono::Utc::now().timestamp_millis() + 45_000,
    };
    let response = run_typed_operation(
        registry,
        identity,
        operation_id,
        request,
        Duration::from_secs(50),
        "Attempt cleanup",
    )
    .await?;
    match response {
        BridgeMessage::AttemptCleaned {
            workspace: observed_workspace,
            receipt,
            ..
        } if observed_workspace == workspace && receipt.attempt_id == workspace.attempt_id => {
            Ok(receipt)
        }
        BridgeMessage::AttemptCleaned { .. } => {
            anyhow::bail!("hosted Runtime cleanup receipt names different Attempt evidence")
        }
        BridgeMessage::Error { message, .. } => {
            anyhow::bail!("hosted Runtime Attempt cleanup failed: {message}")
        }
        _ => anyhow::bail!("hosted Runtime Attempt cleanup returned the wrong response"),
    }
}

async fn run_workspace_operation(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
    operation_id: Uuid,
    request: BridgeMessage,
) -> Result<BridgeMessage> {
    run_typed_operation(
        registry,
        identity,
        operation_id,
        request,
        Duration::from_secs(50),
        "workspace",
    )
    .await
}

async fn run_typed_operation(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
    operation_id: Uuid,
    request: BridgeMessage,
    timeout: Duration,
    label: &str,
) -> Result<BridgeMessage> {
    let mut responses = registry.begin_operation(identity, request)?;
    match tokio::time::timeout(timeout, responses.recv()).await {
        Ok(Some(response)) => Ok(response),
        Ok(None) => {
            registry.abandon_operation(operation_id);
            anyhow::bail!("hosted Runtime {label} connection was lost")
        }
        Err(_) => {
            let _ = registry.send_operation(
                identity,
                BridgeMessage::Cancel {
                    operation_id,
                    reason: format!("Core {label} deadline elapsed"),
                },
            );
            registry.abandon_operation(operation_id);
            anyhow::bail!("hosted Runtime {label} operation timed out")
        }
    }
}

fn workspace_operation_id(
    identity: &RuntimeIdentity,
    workspace: &WorkspaceIdentity,
    operation: &str,
) -> Uuid {
    typed_operation_id(identity, workspace, operation, &serde_json::Value::Null)
}

fn typed_operation_id(
    identity: &RuntimeIdentity,
    workspace: &WorkspaceIdentity,
    operation: &str,
    binding: &serde_json::Value,
) -> Uuid {
    use sha2::Digest as _;
    let digest = sha2::Sha256::digest(
        serde_json::to_vec(&(
            "runtime-operation:v3",
            identity,
            workspace,
            operation,
            binding,
        ))
        .expect("typed Runtime operation identity is serializable"),
    );
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // RFC 9562 custom UUID version 8, with the standard variant. This is a
    // transport idempotency key, not a security token.
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn environment_fingerprint(
    identity: &RuntimeIdentity,
    release: &ReleaseIdentity,
) -> Result<String> {
    use sha2::Digest as _;
    Ok(format!(
        "{:x}",
        sha2::Sha256::digest(serde_json::to_vec(&(identity, release))?)
    ))
}

#[expect(
    clippy::too_many_arguments,
    reason = "the hosted launch membrane keeps every exact agent capability explicit"
)]
pub(crate) async fn open_agent_transport(
    registry: &RuntimeBridgeRegistry,
    identity: &RuntimeIdentity,
    auth: &crate::acp::AgentAuth,
    workdir: &str,
    actor: &str,
    responsibility: &str,
    harness: crate::runtime::AgentHarness,
    system_prompt: &str,
) -> Result<tokio::io::DuplexStream> {
    if harness != crate::runtime::AgentHarness::RestlessManaged {
        anyhow::bail!("hosted Runtime Exec currently requires the restless-managed ACP harness");
    }
    preflight(registry, identity).await?;
    let operation_id = Uuid::new_v4();
    let deadline_ms = chrono::Utc::now().timestamp_millis()
        + i64::try_from(restless_runtime_bridge_protocol::MAX_OPERATION_MILLIS)
            .expect("operation limit fits i64");
    let mut responses = registry.begin_operation(
        identity,
        BridgeMessage::LaunchAgent {
            operation_id,
            session_id: auth.session_id.clone(),
            actor: actor.to_string(),
            responsibility: responsibility.to_string(),
            harness: harness.as_str().to_string(),
            model: auth.model.clone(),
            reasoning_effort: auth.effort.clone(),
            workdir: workdir.to_string(),
            prompt: system_prompt.to_string(),
            coordination_capability: auth.coordination_token.clone(),
            model_capability: auth.gateway_token.clone(),
            model_url: auth.gateway_url.clone(),
            deadline_ms,
        },
    )?;
    let (core, bridge) = tokio::io::duplex(256 * 1024);
    let (mut bridge_read, mut bridge_write) = tokio::io::split(bridge);
    let registry = registry.clone();
    let identity = identity.clone();
    tokio::spawn(async move {
        let mut buffer = vec![0_u8; 48 * 1024];
        let mut local_closed = false;
        let mut cancellation_deadline: Option<std::pin::Pin<Box<tokio::time::Sleep>>> = None;
        loop {
            tokio::select! {
                read = bridge_read.read(&mut buffer), if !local_closed => {
                    match read {
                        Ok(0) | Err(_) => {
                            let _ = registry.send_operation(&identity, BridgeMessage::Cancel {
                                operation_id,
                                reason: "Core ACP client closed the session".into(),
                            });
                            local_closed = true;
                            cancellation_deadline = Some(Box::pin(tokio::time::sleep(Duration::from_secs(10))));
                        }
                        Ok(read) => {
                            let data_base64 = base64::engine::general_purpose::STANDARD.encode(&buffer[..read]);
                            if registry.send_operation(&identity, BridgeMessage::AcpStdin { operation_id, data_base64 }).is_err() {
                                break;
                            }
                        }
                    }
                }
                response = responses.recv() => {
                    match response {
                        Some(BridgeMessage::AcpStdout { data_base64, .. }) => {
                            let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data_base64) else { break; };
                            if bridge_write.write_all(&bytes).await.is_err() { break; }
                        }
                        Some(BridgeMessage::Exit { error: None, .. }) => break,
                        Some(BridgeMessage::Exit { error: Some(error), .. })
                        | Some(BridgeMessage::Error { message: error, .. }) => {
                            tracing::warn!(operation_id = %operation_id, "hosted Runtime ACP ended: {error}");
                            break;
                        }
                        None => break,
                        _ => break,
                    }
                }
                _ = async {
                    match cancellation_deadline.as_mut() {
                        Some(deadline) => deadline.await,
                        None => std::future::pending::<()>().await,
                    }
                } => break,
            }
        }
        registry.abandon_operation(operation_id);
        let _ = bridge_write.shutdown().await;
    });
    Ok(core)
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimeBridgeBootstrapRequest {
    pub contract_version: u32,
    pub operation_id: Uuid,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub runtime_id: String,
    pub runtime_generation: i64,
    pub runtime_image: String,
    pub volume_name: String,
    pub source_revision: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimeBridgeBootstrapResponse {
    pub contract_version: u32,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub runtime_generation: i64,
    pub credential_id: Uuid,
    pub credential_epoch: i64,
    pub capability: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
struct RuntimeBootstrapSecret(Arc<[u8]>);

impl std::fmt::Debug for RuntimeBootstrapSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RuntimeBootstrapSecret([REDACTED])")
    }
}

impl RuntimeBootstrapSecret {
    fn read(path: &Path) -> Result<Self> {
        let link = fs::symlink_metadata(path).with_context(|| {
            format!(
                "read Runtime-bootstrap secret metadata at {}",
                path.display()
            )
        })?;
        if link.file_type().is_symlink()
            || !link.file_type().is_file()
            || !(43..=44).contains(&link.len())
        {
            anyhow::bail!("Runtime-bootstrap secret must be one bounded regular non-symlink file");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            if link.permissions().mode() & 0o077 != 0 {
                anyhow::bail!("Runtime-bootstrap secret must not be group- or world-accessible");
            }
        }
        let file = fs::File::open(path)
            .with_context(|| format!("open Runtime-bootstrap secret at {}", path.display()))?;
        let opened = file
            .metadata()
            .context("read opened Runtime-bootstrap secret metadata")?;
        if !opened.is_file() || opened.len() != link.len() {
            anyhow::bail!("Runtime-bootstrap secret changed before it was opened");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            if opened.dev() != link.dev() || opened.ino() != link.ino() {
                anyhow::bail!("Runtime-bootstrap secret changed identity before it was opened");
            }
        }
        let mut bytes = Vec::with_capacity(45);
        file.take(45)
            .read_to_end(&mut bytes)
            .context("read Runtime-bootstrap secret")?;
        if bytes.len() == 44 && bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        if bytes.len() != 43
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || URL_SAFE_NO_PAD.decode(&bytes).ok().map(|value| value.len()) != Some(32)
        {
            anyhow::bail!("Runtime-bootstrap secret must be one 32-byte base64url token");
        }
        Ok(Self(Arc::from(bytes)))
    }

    fn authorizes(&self, headers: &HeaderMap) -> bool {
        let mut values = headers.get_all(AUTHORIZATION).iter();
        let Some(value) = values.next().and_then(|value| value.to_str().ok()) else {
            return false;
        };
        if values.next().is_some() {
            return false;
        }
        value
            .strip_prefix("Bearer ")
            .is_some_and(|candidate| constant_time_equal(&self.0, candidate.as_bytes()))
    }
}

#[derive(Clone)]
struct RuntimeBridgeDeployment {
    owner_id: Uuid,
    plane_id: Uuid,
    plane_hostname: String,
    runtime_image: String,
    source_revision: String,
    secret: RuntimeBootstrapSecret,
}

impl RuntimeBridgeDeployment {
    fn from_environment(entry: &EntryMode) -> Result<Option<Self>> {
        let Some(path) = std::env::var_os(TOKEN_FILE_ENV) else {
            return Ok(None);
        };
        let path = path
            .into_string()
            .map_err(|_| anyhow::anyhow!("{TOKEN_FILE_ENV} must be UTF-8"))?;
        if path.is_empty() || path.contains(['\r', '\n']) {
            anyhow::bail!("{TOKEN_FILE_ENV} must name one secret file");
        }
        let (owner_id, plane_id, plane_hostname) = entry
            .network_coordinates()
            .context("RESTLESS_RUNTIME_BOOTSTRAP_TOKEN_FILE is only valid in network entry mode")?;
        let deployment = Self {
            owner_id,
            plane_id,
            plane_hostname: plane_hostname.to_string(),
            runtime_image: required_environment(COMPANY_IMAGE_ENV)?,
            source_revision: release::SOURCE_REVISION.to_string(),
            secret: RuntimeBootstrapSecret::read(Path::new(&path))?,
        };
        if deployment.owner_id.is_nil()
            || deployment.plane_id.is_nil()
            || !valid_hostname(&deployment.plane_hostname)
            || !valid_immutable_image(&deployment.runtime_image)
            || !valid_source_revision(&deployment.source_revision)
        {
            anyhow::bail!("Runtime-bridge deployment identity is incomplete or mutable");
        }
        Ok(Some(deployment))
    }

    fn validate_request(&self, request: &RuntimeBridgeBootstrapRequest) -> BootstrapResult<()> {
        let expected_runtime_id = format!("restless-cell-{}", request.cell_id);
        let expected_volume_name = format!("{expected_runtime_id}-data");
        if request.contract_version != RUNTIME_BRIDGE_CONTRACT_VERSION
            || request.operation_id.is_nil()
            || request.owner_id.is_nil()
            || request.plane_id.is_nil()
            || request.company_id.is_nil()
            || request.cell_id.is_nil()
            || request.runtime_generation < 1
            || request.runtime_id != expected_runtime_id
            || request.volume_name != expected_volume_name
            || !valid_immutable_image(&request.runtime_image)
            || !valid_source_revision(&request.source_revision)
        {
            return Err(BootstrapFailure::Invalid);
        }
        if request.owner_id != self.owner_id
            || request.plane_id != self.plane_id
            || request.runtime_image != self.runtime_image
            || request.source_revision != self.source_revision
        {
            return Err(BootstrapFailure::IdentityMismatch);
        }
        Ok(())
    }
}

#[derive(Clone)]
struct RuntimeBridgeBootstrapService {
    deployment: RuntimeBridgeDeployment,
    daemon: Arc<Daemon>,
}

impl RuntimeBridgeBootstrapService {
    async fn execute(
        &self,
        request: RuntimeBridgeBootstrapRequest,
    ) -> BootstrapResult<RuntimeBridgeBootstrapResponse> {
        self.deployment.validate_request(&request)?;
        let stored = sqlx::query_as::<_, (Uuid, Uuid, String, String)>(
            "SELECT owner_id,plane_id,company_handle,status \
             FROM restless_authority.company_bootstrap_operations \
             WHERE company_id=$1 AND cell_id=$2 LIMIT 1",
        )
        .bind(request.company_id)
        .bind(request.cell_id)
        .fetch_optional(self.daemon.authority.pool())
        .await
        .map_err(unavailable)?
        .ok_or(BootstrapFailure::IdentityMismatch)?;
        let (owner_id, plane_id, company, status) = stored;
        if owner_id != request.owner_id
            || plane_id != request.plane_id
            || status != "ready"
            || company != format!("c{}", request.company_id.simple())
        {
            return Err(BootstrapFailure::IdentityMismatch);
        }
        let org = self
            .daemon
            .orgintel
            .get(&company)
            .await
            .map_err(unavailable)?;
        let expected_access = restless_orgintel::CompanyAccessIdentity {
            company_id: request.company_id,
            cell_id: request.cell_id,
        };
        if org.company_access_identity().await.map_err(unavailable)? != Some(expected_access) {
            return Err(BootstrapFailure::IdentityMismatch);
        }

        let scope = HostedRuntimeBridgeScope {
            company,
            owner_id: request.owner_id,
            plane_id: request.plane_id,
            company_id: request.company_id,
            cell_id: request.cell_id,
            runtime_id: request.runtime_id,
            runtime_generation: request.runtime_generation,
            runtime_image: request.runtime_image,
            volume_name: request.volume_name,
            source_revision: request.source_revision,
        };
        let credential =
            reserve_runtime_generation(self.daemon.authority.pool(), &scope, request.operation_id)
                .await?;
        self.daemon.runtime_bridges.retire_if_not_authorized(
            &protocol_identity(&scope),
            credential.credential_id,
            credential.credential_epoch,
        );
        let capability = self
            .daemon
            .capabilities
            .issue_hosted_runtime_bridge(
                &scope,
                credential.credential_id,
                credential.credential_epoch,
                credential.expires_at,
            )
            .map_err(unavailable)?;
        Ok(RuntimeBridgeBootstrapResponse {
            contract_version: RUNTIME_BRIDGE_CONTRACT_VERSION,
            company_id: scope.company_id,
            cell_id: scope.cell_id,
            runtime_generation: scope.runtime_generation,
            credential_id: credential.credential_id,
            credential_epoch: credential.credential_epoch,
            capability,
            expires_at: credential.expires_at,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct RuntimeBridgeCredentialLease {
    credential_id: Uuid,
    credential_epoch: i64,
    expires_at: chrono::DateTime<chrono::Utc>,
}

async fn reserve_runtime_generation(
    pool: &sqlx::PgPool,
    scope: &HostedRuntimeBridgeScope,
    operation_id: Uuid,
) -> BootstrapResult<RuntimeBridgeCredentialLease> {
    let mut transaction = pool.begin().await.map_err(unavailable)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(format!("runtime-bridge:cell:{}", scope.cell_id))
        .execute(&mut *transaction)
        .await
        .map_err(unavailable)?;
    let current = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Uuid,
            String,
            String,
            i64,
            String,
            String,
            String,
            Uuid,
            Uuid,
            i64,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        "SELECT owner_id,plane_id,company_id,company_handle,runtime_id,runtime_generation,\
                runtime_image,volume_name,source_revision,credential_operation_id,credential_id,\
                credential_epoch,credential_expires_at \
         FROM restless_authority.runtime_bridge_generations WHERE cell_id=$1 FOR UPDATE",
    )
    .bind(scope.cell_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(unavailable)?;
    let new_expires_at = chrono::DateTime::<chrono::Utc>::from_timestamp(
        chrono::Utc::now().timestamp()
            + i64::try_from(CAPABILITY_LIFETIME_SECONDS).expect("TTL fits i64"),
        0,
    )
    .expect("bounded Runtime bridge expiry");
    let credential = match current {
        None if scope.runtime_generation == 1 => {
            let credential_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO restless_authority.runtime_bridge_generations \
                 (cell_id,owner_id,plane_id,company_id,company_handle,runtime_id,runtime_generation,\
                  runtime_image,volume_name,source_revision,credential_operation_id,credential_id,\
                  credential_epoch,credential_expires_at) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,1,$13)",
            )
            .bind(scope.cell_id)
            .bind(scope.owner_id)
            .bind(scope.plane_id)
            .bind(scope.company_id)
            .bind(&scope.company)
            .bind(&scope.runtime_id)
            .bind(scope.runtime_generation)
            .bind(&scope.runtime_image)
            .bind(&scope.volume_name)
            .bind(&scope.source_revision)
            .bind(operation_id)
            .bind(credential_id)
            .bind(new_expires_at)
            .execute(&mut *transaction)
            .await
            .map_err(unavailable)?;
            RuntimeBridgeCredentialLease {
                credential_id,
                credential_epoch: 1,
                expires_at: new_expires_at,
            }
        }
        Some((
            owner,
            plane,
            company_id,
            company,
            runtime_id,
            generation,
            image,
            volume,
            source,
            current_operation_id,
            credential_id,
            credential_epoch,
            expires_at,
        )) if owner == scope.owner_id
            && plane == scope.plane_id
            && company_id == scope.company_id
            && company == scope.company
            && runtime_id == scope.runtime_id
            && volume == scope.volume_name
            && generation == scope.runtime_generation
            && image == scope.runtime_image
            && source == scope.source_revision
            && current_operation_id == operation_id =>
        {
            RuntimeBridgeCredentialLease {
                credential_id,
                credential_epoch,
                expires_at,
            }
        }
        Some((
            owner,
            plane,
            company_id,
            company,
            runtime_id,
            generation,
            image,
            volume,
            source,
            _,
            _,
            credential_epoch,
            _,
        )) if owner == scope.owner_id
            && plane == scope.plane_id
            && company_id == scope.company_id
            && company == scope.company
            && runtime_id == scope.runtime_id
            && volume == scope.volume_name
            && ((scope.runtime_generation == generation
                && image == scope.runtime_image
                && source == scope.source_revision)
                || scope.runtime_generation == generation + 1) =>
        {
            let credential_epoch = credential_epoch
                .checked_add(1)
                .ok_or(BootstrapFailure::IdentityMismatch)?;
            let credential_id = Uuid::new_v4();
            sqlx::query(
                "UPDATE restless_authority.runtime_bridge_generations \
                 SET runtime_generation=$2,runtime_image=$3,source_revision=$4,\
                     credential_operation_id=$5,credential_id=$6,credential_epoch=$7,\
                     credential_expires_at=$8,updated_at=now() \
                 WHERE cell_id=$1",
            )
            .bind(scope.cell_id)
            .bind(scope.runtime_generation)
            .bind(&scope.runtime_image)
            .bind(&scope.source_revision)
            .bind(operation_id)
            .bind(credential_id)
            .bind(credential_epoch)
            .bind(new_expires_at)
            .execute(&mut *transaction)
            .await
            .map_err(unavailable)?;
            RuntimeBridgeCredentialLease {
                credential_id,
                credential_epoch,
                expires_at: new_expires_at,
            }
        }
        _ => return Err(BootstrapFailure::IdentityMismatch),
    };
    transaction.commit().await.map_err(unavailable)?;
    Ok(credential)
}

async fn durable_runtime_generation_matches(
    pool: &sqlx::PgPool,
    grant: &crate::capability::HostedRuntimeBridgeGrant,
) -> Result<bool> {
    let scope = &grant.scope;
    let current = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            Uuid,
            String,
            String,
            i64,
            String,
            String,
            String,
            Uuid,
            i64,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        "SELECT owner_id,plane_id,company_id,company_handle,runtime_id,runtime_generation,\
                runtime_image,volume_name,source_revision,credential_id,credential_epoch,\
                credential_expires_at \
         FROM restless_authority.runtime_bridge_generations WHERE cell_id=$1",
    )
    .bind(scope.cell_id)
    .fetch_optional(pool)
    .await
    .context("read current durable Runtime-bridge generation")?;
    Ok(current.is_some_and(
        |(
            owner,
            plane,
            company_id,
            company,
            runtime_id,
            generation,
            image,
            volume,
            source,
            credential_id,
            credential_epoch,
            credential_expires_at,
        )| {
            owner == scope.owner_id
                && plane == scope.plane_id
                && company_id == scope.company_id
                && company == scope.company
                && runtime_id == scope.runtime_id
                && generation == scope.runtime_generation
                && image == scope.runtime_image
                && volume == scope.volume_name
                && source == scope.source_revision
                && credential_id == grant.credential_id
                && credential_epoch == grant.credential_epoch
                && credential_expires_at == grant.expires_at
        },
    ))
}

#[derive(Clone)]
enum BootstrapEndpoint {
    Disabled,
    Enabled(Arc<RuntimeBridgeBootstrapService>),
}

pub(crate) fn routes<S>(daemon: &Arc<Daemon>, entry: &EntryMode) -> Result<Router<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let endpoint = match RuntimeBridgeDeployment::from_environment(entry)? {
        Some(deployment) => BootstrapEndpoint::Enabled(Arc::new(RuntimeBridgeBootstrapService {
            deployment,
            daemon: Arc::clone(daemon),
        })),
        None => BootstrapEndpoint::Disabled,
    };
    Ok(Router::<S>::new()
        .route(
            RUNTIME_BRIDGE_BOOTSTRAP_PATH,
            post(runtime_bridge_bootstrap),
        )
        .route(RUNTIME_BRIDGE_PATH, get(runtime_bridge_websocket))
        .layer(DefaultBodyLimit::max(MAX_BOOTSTRAP_REQUEST_BYTES))
        .layer(Extension(Arc::new(endpoint))))
}

async fn runtime_bridge_websocket(
    Extension(endpoint): Extension<Arc<BootstrapEndpoint>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    websocket: WebSocketUpgrade,
) -> Response<Body> {
    let BootstrapEndpoint::Enabled(service) = endpoint.as_ref() else {
        return bootstrap_error(
            StatusCode::NOT_FOUND,
            "runtime_bridge_disabled",
            "the hosted Runtime bridge is not enabled on this plane",
        );
    };
    if uri.path() != RUNTIME_BRIDGE_PATH
        || uri.query().is_some()
        || headers.contains_key("origin")
        || headers.contains_key("cookie")
        || !single_host_matches(&headers, &service.deployment.plane_hostname)
        || !trusted_proxy_envelope_matches(&headers, &service.deployment.plane_hostname)
    {
        return bootstrap_error(
            StatusCode::BAD_REQUEST,
            "runtime_bridge_envelope",
            "Runtime bridge registration requires the exact machine endpoint",
        );
    }
    let Some(token) = single_bearer(&headers) else {
        return bootstrap_error(
            StatusCode::UNAUTHORIZED,
            "runtime_bridge_unauthorized",
            "Runtime bridge authentication failed",
        );
    };
    let grant = match service
        .daemon
        .capabilities
        .verify_hosted_runtime_bridge(token)
    {
        Ok(grant) => grant,
        Err(_) => {
            return bootstrap_error(
                StatusCode::UNAUTHORIZED,
                "runtime_bridge_unauthorized",
                "Runtime bridge authentication failed",
            )
        }
    };
    let daemon = Arc::clone(&service.daemon);
    websocket
        .max_frame_size(MAX_FRAME_BYTES)
        .max_message_size(MAX_FRAME_BYTES)
        .on_upgrade(move |socket| run_registered_bridge(socket, daemon, grant))
        .into_response()
}

async fn run_registered_bridge(
    socket: WebSocket,
    daemon: Arc<Daemon>,
    grant: crate::capability::HostedRuntimeBridgeGrant,
) {
    let scope = &grant.scope;
    let (mut socket_tx, mut socket_rx) = socket.split();
    let first = tokio::time::timeout(REGISTER_DEADLINE, socket_rx.next()).await;
    let Some(Ok(WebSocketMessage::Binary(bytes))) = first.ok().flatten() else {
        let _ = socket_tx.send(WebSocketMessage::Close(None)).await;
        return;
    };
    let Ok(frame) = Frame::decode(&bytes) else {
        let _ = socket_tx.send(WebSocketMessage::Close(None)).await;
        return;
    };
    let expected_identity = protocol_identity(&scope);
    if frame.identity != expected_identity {
        let _ = socket_tx.send(WebSocketMessage::Close(None)).await;
        return;
    }
    let BridgeMessage::Register {
        connection_nonce,
        release: runtime_release,
    } = frame.message
    else {
        let _ = socket_tx.send(WebSocketMessage::Close(None)).await;
        return;
    };
    if runtime_release.source_revision != scope.source_revision
        || runtime_release.api_contract_version != release::API_CONTRACT_VERSION
        || runtime_release.assertion_contract_version != crate::entry::ASSERTION_CONTRACT_VERSION
        || runtime_release.schema_version != i64::from(release::SCHEMA_VERSION)
    {
        let _ = socket_tx.send(WebSocketMessage::Close(None)).await;
        return;
    }
    if !matches!(
        tokio::time::timeout(
            Duration::from_secs(5),
            durable_runtime_generation_matches(daemon.authority.pool(), &grant),
        )
        .await,
        Ok(Ok(true))
    ) {
        let _ = socket_tx.send(WebSocketMessage::Close(None)).await;
        return;
    }
    let mut incoming_sequence = SequenceTracker::default();
    if incoming_sequence.observe(frame.sequence).is_err() {
        let _ = socket_tx.send(WebSocketMessage::Close(None)).await;
        return;
    }
    let (outbound_tx, mut outbound_rx) = mpsc::channel(OUTBOUND_QUEUE_DEPTH);
    let connection_id = match daemon.runtime_bridges.register(
        expected_identity.clone(),
        connection_nonce,
        runtime_release,
        grant.credential_id,
        grant.credential_epoch,
        grant.expires_at,
        outbound_tx,
    ) {
        Ok(connection_id) => connection_id,
        Err(_) => {
            let _ = socket_tx.send(WebSocketMessage::Close(None)).await;
            return;
        }
    };
    let ack = Frame {
        protocol_version: PROTOCOL_VERSION,
        sequence: 1,
        identity: expected_identity.clone(),
        message: BridgeMessage::RegisterAck {
            connection_nonce,
            accepted_at_ms: chrono::Utc::now().timestamp_millis(),
            heartbeat_interval_ms: HEARTBEAT_INTERVAL.as_millis() as u64,
        },
    };
    let Ok(encoded) = ack.encode() else {
        daemon.runtime_bridges.remove(scope.cell_id, connection_id);
        return;
    };
    if socket_tx
        .send(WebSocketMessage::Binary(encoded.into()))
        .await
        .is_err()
    {
        daemon.runtime_bridges.remove(scope.cell_id, connection_id);
        return;
    }

    let mut freshness = tokio::time::interval(Duration::from_secs(5));
    freshness.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = freshness.tick() => {
                if !daemon.runtime_bridges.connection_is_current_and_fresh(scope.cell_id, connection_id) {
                    break;
                }
            }
            incoming = socket_rx.next() => {
                let Some(Ok(message)) = incoming else { break; };
                match message {
                    WebSocketMessage::Binary(bytes) => {
                        let Ok(frame) = Frame::decode(&bytes) else { break; };
                        if frame.identity != expected_identity
                            || incoming_sequence.observe(frame.sequence).is_err()
                        {
                            break;
                        }
                        match frame.message {
                            BridgeMessage::Heartbeat { connection_nonce: nonce, .. }
                                if daemon.runtime_bridges.heartbeat(scope.cell_id, connection_id, nonce) => {}
                            BridgeMessage::CoordinationRequest { operation_id, method, path, body, deadline_ms }
                                if method == restless_runtime_bridge_protocol::CoordinationMethod::Post
                                    && path == "/coordination"
                                    && body.is_some() => {
                                let daemon = Arc::clone(&daemon);
                                let identity = expected_identity.clone();
                                tokio::spawn(async move {
                                    let remaining = deadline_ms.saturating_sub(chrono::Utc::now().timestamp_millis());
                                    let result = if remaining <= 0 {
                                        Err(anyhow::anyhow!("coordination request deadline expired"))
                                    } else {
                                        tokio::time::timeout(
                                            Duration::from_millis(u64::try_from(remaining).unwrap_or_default().min(30_000)),
                                            crate::proxy_runtime_coordination(Arc::clone(&daemon), body.expect("guarded body")),
                                        )
                                        .await
                                        .map_err(|_| anyhow::anyhow!("coordination request timed out"))
                                        .and_then(|result| result)
                                    };
                                    let response = match result {
                                        Ok(body) => BridgeMessage::CoordinationResponse {
                                            operation_id,
                                            status_code: 200,
                                            body: Some(body),
                                        },
                                        Err(error) => BridgeMessage::Error {
                                            operation_id: Some(operation_id),
                                            code: "coordination_failed".into(),
                                            message: format!("coordination request failed: {error}"),
                                        },
                                    };
                                    let _ = daemon.runtime_bridges.send_untracked(&identity, response);
                                });
                            }
                            message => {
                                if !daemon.runtime_bridges.deliver(scope.cell_id, connection_id, message) {
                                    break;
                                }
                            }
                        }
                    }
                    WebSocketMessage::Ping(payload) => {
                        if socket_tx.send(WebSocketMessage::Pong(payload)).await.is_err() { break; }
                    }
                    WebSocketMessage::Pong(_) => {}
                    WebSocketMessage::Close(_) | WebSocketMessage::Text(_) => break,
                }
            }
            outbound = outbound_rx.recv() => {
                let Some(frame) = outbound else { break; };
                let Ok(bytes) = frame.encode() else { break; };
                if socket_tx.send(WebSocketMessage::Binary(bytes.into())).await.is_err() { break; }
            }
        }
    }
    daemon.runtime_bridges.remove(scope.cell_id, connection_id);
}

fn protocol_identity(scope: &HostedRuntimeBridgeScope) -> RuntimeIdentity {
    RuntimeIdentity {
        owner_id: scope.owner_id,
        plane_id: scope.plane_id,
        company_id: scope.company_id,
        cell_id: scope.cell_id,
        company: scope.company.clone(),
        runtime_id: scope.runtime_id.clone(),
        runtime_generation: scope.runtime_generation,
        runtime_image: scope.runtime_image.clone(),
        volume_name: scope.volume_name.clone(),
        source_revision: scope.source_revision.clone(),
    }
}

fn single_bearer(headers: &HeaderMap) -> Option<&str> {
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    value
        .strip_prefix("Bearer ")
        .filter(|value| !value.is_empty() && value.len() <= 16_384 && !value.contains(['\r', '\n']))
}

async fn runtime_bridge_bootstrap(
    Extension(endpoint): Extension<Arc<BootstrapEndpoint>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let BootstrapEndpoint::Enabled(service) = endpoint.as_ref() else {
        return bootstrap_error(
            StatusCode::NOT_FOUND,
            "runtime_bridge_disabled",
            "the hosted Runtime bridge is not enabled on this plane",
        );
    };
    if uri.path() != RUNTIME_BRIDGE_BOOTSTRAP_PATH
        || uri.query().is_some()
        || headers.contains_key("origin")
        || headers.contains_key("cookie")
        || !trusted_proxy_envelope_matches(&headers, &service.deployment.plane_hostname)
        || !single_content_length_is_bounded(&headers)
    {
        return bootstrap_error(
            StatusCode::BAD_REQUEST,
            "runtime_bridge_envelope",
            "Runtime bridge bootstrap requires the exact bounded direct endpoint",
        );
    }
    if !single_host_matches(&headers, &service.deployment.plane_hostname) {
        return bootstrap_error(
            StatusCode::FORBIDDEN,
            "runtime_bridge_identity_mismatch",
            "Runtime bridge bootstrap does not identify this exact account plane",
        );
    }
    if !service.deployment.secret.authorizes(&headers) {
        return bootstrap_error(
            StatusCode::UNAUTHORIZED,
            "runtime_bridge_unauthorized",
            "Runtime bridge bootstrap authentication failed",
        );
    }
    if headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        != Some("application/json")
    {
        return bootstrap_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "runtime_bridge_content_type",
            "Runtime bridge bootstrap requires application/json",
        );
    }
    if body.is_empty() || body.len() > MAX_BOOTSTRAP_REQUEST_BYTES {
        return bootstrap_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "runtime_bridge_body_size",
            "Runtime bridge bootstrap body is outside the contract bound",
        );
    }
    let request = match serde_json::from_slice::<RuntimeBridgeBootstrapRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return bootstrap_error(
                StatusCode::BAD_REQUEST,
                "runtime_bridge_request_invalid",
                "Runtime bridge bootstrap request is invalid",
            )
        }
    };
    match service.execute(request).await {
        Ok(receipt) => bootstrap_json(
            StatusCode::OK,
            serde_json::to_vec(&receipt).expect("bounded Runtime bootstrap response"),
        ),
        Err(BootstrapFailure::Invalid) => bootstrap_error(
            StatusCode::BAD_REQUEST,
            "runtime_bridge_request_invalid",
            "Runtime bridge bootstrap request is invalid",
        ),
        Err(BootstrapFailure::IdentityMismatch) => bootstrap_error(
            StatusCode::FORBIDDEN,
            "runtime_bridge_identity_mismatch",
            "Runtime bridge bootstrap does not match durable company identity",
        ),
        Err(BootstrapFailure::Unavailable(error)) => {
            tracing::error!("Runtime bridge bootstrap unavailable: {error:#}");
            bootstrap_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "runtime_bridge_unavailable",
                "Runtime bridge bootstrap is temporarily unavailable",
            )
        }
    }
}

type BootstrapResult<T> = std::result::Result<T, BootstrapFailure>;

#[derive(Debug)]
enum BootstrapFailure {
    Invalid,
    IdentityMismatch,
    Unavailable(anyhow::Error),
}

fn unavailable(error: impl Into<anyhow::Error>) -> BootstrapFailure {
    BootstrapFailure::Unavailable(error.into())
}

fn required_environment(name: &'static str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} is required"))?;
    if value.is_empty() || value.trim() != value || value.contains(['\r', '\n']) {
        anyhow::bail!("{name} must be one non-empty value");
    }
    Ok(value)
}

fn valid_immutable_image(image: &str) -> bool {
    let Some((repository, digest)) = image.rsplit_once("@sha256:") else {
        return false;
    };
    !repository.is_empty()
        && image.len() <= 512
        && digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_source_revision(value: &str) -> bool {
    (7..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_hostname(value: &str) -> bool {
    value.len() <= 253
        && value.contains('.')
        && !value.contains(['/', ':', '\r', '\n'])
        && value.parse::<std::net::IpAddr>().is_err()
}

fn single_host_matches(headers: &HeaderMap, expected: &str) -> bool {
    let mut hosts = headers.get_all(HOST).iter();
    hosts
        .next()
        .and_then(|value| value.to_str().ok())
        .is_some_and(|host| host == expected)
        && hosts.next().is_none()
}

fn trusted_proxy_envelope_matches(headers: &HeaderMap, expected_host: &str) -> bool {
    if headers.contains_key("forwarded") || headers.contains_key("x-original-host") {
        return false;
    }
    let single_optional = |name: &'static str, expected: &str| {
        let mut values = headers.get_all(name).iter();
        let matches = values
            .next()
            .map(|value| value.to_str().ok() == Some(expected))
            .unwrap_or(true);
        matches && values.next().is_none()
    };
    single_optional("x-forwarded-host", expected_host)
        && single_optional("x-forwarded-proto", "https")
}

fn single_content_length_is_bounded(headers: &HeaderMap) -> bool {
    let mut values = headers.get_all(CONTENT_LENGTH).iter();
    let first = values.next();
    if values.next().is_some() {
        return false;
    }
    first
        .map(|value| {
            value
                .to_str()
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .is_some_and(|length| (1..=MAX_BOOTSTRAP_REQUEST_BYTES).contains(&length))
        })
        .unwrap_or(true)
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn bootstrap_json(status: StatusCode, bytes: Vec<u8>) -> Response<Body> {
    let mut response = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .header(CONTENT_LENGTH, bytes.len().to_string())
        .body(Body::from(bytes))
        .expect("bounded Runtime-bootstrap response");
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn bootstrap_error(
    status: StatusCode,
    error: &'static str,
    message: &'static str,
) -> Response<Body> {
    bootstrap_json(
        status,
        serde_json::to_vec(&serde_json::json!({"error": error, "message": message}))
            .expect("Runtime-bootstrap error JSON"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(generation: i64) -> RuntimeIdentity {
        RuntimeIdentity {
            owner_id: Uuid::new_v4(),
            plane_id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            cell_id: Uuid::new_v4(),
            company: "c0123456789abcdef0123456789abcdef".into(),
            runtime_id: "restless-cell-01234567-89ab-4def-8123-456789abcdef".into(),
            runtime_generation: generation,
            runtime_image: format!("registry.example/runtime@sha256:{}", "a".repeat(64)),
            volume_name: "restless-cell-01234567-89ab-4def-8123-456789abcdef-data".into(),
            source_revision: release::SOURCE_REVISION.into(),
        }
    }

    fn runtime_release() -> ReleaseIdentity {
        ReleaseIdentity {
            core_version: release::CORE_VERSION.into(),
            source_revision: release::SOURCE_REVISION.into(),
            api_contract_version: release::API_CONTRACT_VERSION,
            assertion_contract_version: crate::entry::ASSERTION_CONTRACT_VERSION,
            schema_version: i64::from(release::SCHEMA_VERSION),
            harnesses: [("codex".into(), "codex-cli-test".into())].into(),
            harness_agents: std::collections::BTreeMap::new(),
            harness_dependencies: std::collections::BTreeMap::new(),
        }
    }

    fn credential(epoch: i64) -> (Uuid, i64, chrono::DateTime<chrono::Utc>) {
        (
            Uuid::new_v4(),
            epoch,
            chrono::Utc::now() + chrono::Duration::hours(1),
        )
    }

    #[test]
    fn fleet_request_is_an_exact_contract() {
        let value = serde_json::json!({
            "contract_version": 1,
            "operation_id": "11111111-1111-4111-8111-111111111111",
            "owner_id": "22222222-2222-4222-8222-222222222222",
            "plane_id": "33333333-3333-4333-8333-333333333333",
            "company_id": "44444444-4444-4444-8444-444444444444",
            "cell_id": "55555555-5555-4555-8555-555555555555",
            "runtime_id": "restless-cell-55555555-5555-4555-8555-555555555555",
            "runtime_generation": 7,
            "runtime_image": format!("registry.example/runtime@sha256:{}", "a".repeat(64)),
            "volume_name": "restless-cell-55555555-5555-4555-8555-555555555555-data",
            "source_revision": "b".repeat(40),
        });
        assert!(serde_json::from_value::<RuntimeBridgeBootstrapRequest>(value.clone()).is_ok());
        let mut extra = value;
        extra["legacy_company"] = serde_json::json!("forbidden");
        assert!(serde_json::from_value::<RuntimeBridgeBootstrapRequest>(extra).is_err());
    }

    #[test]
    fn stale_identity_is_refused_and_reconnect_removal_is_fenced() {
        let registry = RuntimeBridgeRegistry::default();
        let current = identity(2);
        let (first_tx, _first_rx) = mpsc::channel(2);
        let (credential_id, credential_epoch, expires_at) = credential(1);
        let first = registry
            .register(
                current.clone(),
                Uuid::new_v4(),
                runtime_release(),
                credential_id,
                credential_epoch,
                expires_at,
                first_tx,
            )
            .unwrap();

        let mut stale = current.clone();
        stale.runtime_generation = 1;
        let (stale_tx, _stale_rx) = mpsc::channel(2);
        assert_eq!(
            registry.register(
                stale,
                Uuid::new_v4(),
                runtime_release(),
                credential_id,
                credential_epoch,
                expires_at,
                stale_tx,
            ),
            Err("stale_or_conflicting_runtime")
        );

        let nonce = Uuid::new_v4();
        let (second_tx, _second_rx) = mpsc::channel(2);
        let second = registry
            .register(
                current.clone(),
                nonce,
                runtime_release(),
                credential_id,
                credential_epoch,
                expires_at,
                second_tx,
            )
            .unwrap();
        assert_ne!(first, second);
        registry.remove(current.cell_id, first);
        assert_eq!(registry.readiness(&current), BridgeReadiness::Ready);
        assert!(!registry.heartbeat(current.cell_id, second, Uuid::new_v4()));
        assert!(registry.heartbeat(current.cell_id, second, nonce));
    }

    #[test]
    fn readiness_requires_fresh_exact_release_and_identity() {
        let registry = RuntimeBridgeRegistry::default();
        let exact = identity(1);
        let (sender, _receiver) = mpsc::channel(2);
        let (credential_id, credential_epoch, expires_at) = credential(1);
        registry
            .register(
                exact.clone(),
                Uuid::new_v4(),
                runtime_release(),
                credential_id,
                credential_epoch,
                expires_at,
                sender,
            )
            .unwrap();
        assert_eq!(registry.readiness(&exact), BridgeReadiness::Ready);

        let mut wrong = exact.clone();
        wrong.runtime_image = format!("registry.example/runtime@sha256:{}", "b".repeat(64));
        assert_eq!(
            registry.readiness(&wrong),
            BridgeReadiness::IdentityMismatch
        );
        registry
            .state
            .lock()
            .unwrap()
            .connections
            .get_mut(&exact.cell_id)
            .unwrap()
            .last_seen = Instant::now() - BRIDGE_FRESHNESS - Duration::from_secs(1);
        assert_eq!(registry.readiness(&exact), BridgeReadiness::Stale);
    }

    #[tokio::test]
    async fn fake_runtime_bridge_prepares_runs_and_observes_repository_work() {
        let registry = RuntimeBridgeRegistry::hosted();
        let exact = identity(1);
        let (sender, mut receiver) = mpsc::channel(32);
        let (credential_id, credential_epoch, expires_at) = credential(1);
        let connection_id = registry
            .register(
                exact.clone(),
                Uuid::new_v4(),
                runtime_release(),
                credential_id,
                credential_epoch,
                expires_at,
                sender,
            )
            .unwrap();
        let fake_registry = registry.clone();
        let fake_identity = exact.clone();
        let workspace = WorkspaceIdentity {
            attempt_id: Uuid::new_v4(),
            work_id: Uuid::new_v4(),
            work_revision: 4,
            repo: "game".into(),
            worktree: "work-fake-r4".into(),
        };
        let fake_workspace = workspace.clone();
        let expected_environment = environment_fingerprint(&exact, &runtime_release()).unwrap();
        let fake_environment = expected_environment.clone();
        let model = "openai/fake-acp".to_string();
        let fake_model = model.clone();
        let fake = tokio::spawn(async move {
            let mut operation_id = None;
            let mut input = Vec::new();
            while let Some(frame) = receiver.recv().await {
                match frame.message {
                    BridgeMessage::PreflightRequest { operation_id, .. } => {
                        assert!(fake_registry.deliver(
                            fake_identity.cell_id,
                            connection_id,
                            BridgeMessage::PreflightResult {
                                operation_id,
                                status: restless_runtime_bridge_protocol::PreflightStatus::Ready,
                                release: runtime_release(),
                                disk_available_bytes: 1024 * 1024 * 1024,
                                observed_at_ms: chrono::Utc::now().timestamp_millis(),
                                error: None,
                            },
                        ));
                    }
                    BridgeMessage::PrepareWorkspace {
                        operation_id,
                        workspace,
                        requested_source_ref,
                        expected_source_commit,
                        expected_source_tree,
                        ..
                    } => {
                        assert_eq!(workspace, fake_workspace);
                        assert_eq!(requested_source_ref.as_deref(), Some("refs/heads/main"));
                        assert!(expected_source_commit.is_none());
                        assert!(expected_source_tree.is_none());
                        assert!(fake_registry.deliver(
                            fake_identity.cell_id,
                            connection_id,
                            BridgeMessage::WorkspacePrepared {
                                operation_id,
                                workspace,
                                requested_source_ref,
                                observation: WorkspaceObservation {
                                    repo_path: "/company/repos/game".into(),
                                    workdir: "/company/worktrees/work-fake-r4".into(),
                                    source_commit: "a".repeat(40),
                                    source_tree: "b".repeat(40),
                                    status_digest: "c".repeat(64),
                                    dirty_entries: 0,
                                    environment_fingerprint: fake_environment.clone(),
                                    observed_at_ms: chrono::Utc::now().timestamp_millis(),
                                },
                                reused: false,
                            },
                        ));
                    }
                    BridgeMessage::LaunchAgent {
                        operation_id: launched,
                        workdir,
                        ..
                    } => {
                        assert_eq!(workdir, "/company/worktrees/work-fake-r4");
                        operation_id = Some(launched);
                    }
                    BridgeMessage::AcpStdin {
                        operation_id: incoming,
                        data_base64,
                    } => {
                        assert_eq!(Some(incoming), operation_id);
                        input.extend(
                            base64::engine::general_purpose::STANDARD
                                .decode(data_base64)
                                .unwrap(),
                        );
                        while let Some(newline) = input.iter().position(|byte| *byte == b'\n') {
                            let line = input.drain(..=newline).collect::<Vec<_>>();
                            let request: serde_json::Value =
                                serde_json::from_slice(&line[..line.len() - 1]).unwrap();
                            let id = request["id"].clone();
                            let method = request["method"].as_str().unwrap();
                            let model_option = serde_json::json!({
                                "id": "model",
                                "name": "Model",
                                "category": "model",
                                "type": "select",
                                "currentValue": fake_model,
                                "options": [{"value": fake_model, "name": "Fake ACP model"}],
                            });
                            let mut messages = match method {
                                "initialize" => vec![serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "protocolVersion": 1,
                                        "agentCapabilities": {},
                                        "authMethods": [],
                                    }
                                })],
                                "session/new" => vec![serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "sessionId": "fake-session",
                                        "configOptions": [model_option],
                                    }
                                })],
                                "session/set_config_option" => vec![serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {"configOptions": [model_option]},
                                })],
                                "session/prompt" => vec![
                                    serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "method": "session/update",
                                        "params": {
                                            "sessionId": "fake-session",
                                            "update": {
                                                "sessionUpdate": "agent_message_chunk",
                                                "content": {"type": "text", "text": "bounded bridge reply"},
                                            }
                                        }
                                    }),
                                    serde_json::json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {"stopReason": "end_turn"},
                                    }),
                                ],
                                other => panic!("unexpected fake ACP method {other}"),
                            };
                            let mut encoded = Vec::new();
                            for message in messages.drain(..) {
                                serde_json::to_writer(&mut encoded, &message).unwrap();
                                encoded.push(b'\n');
                            }
                            assert!(fake_registry.deliver(
                                fake_identity.cell_id,
                                connection_id,
                                BridgeMessage::AcpStdout {
                                    operation_id: incoming,
                                    data_base64: base64::engine::general_purpose::STANDARD
                                        .encode(encoded),
                                },
                            ));
                        }
                    }
                    BridgeMessage::Cancel {
                        operation_id: cancelled,
                        ..
                    } => {
                        assert_eq!(Some(cancelled), operation_id);
                        assert!(fake_registry.deliver(
                            fake_identity.cell_id,
                            connection_id,
                            BridgeMessage::Exit {
                                operation_id: cancelled,
                                code: Some(0),
                                signal: None,
                                error: None,
                            },
                        ));
                        operation_id = None;
                    }
                    BridgeMessage::ObserveWorkspace {
                        operation_id,
                        workspace,
                        expected_environment_fingerprint,
                        ..
                    } => {
                        assert_eq!(workspace, fake_workspace);
                        assert_eq!(expected_environment_fingerprint, fake_environment);
                        assert!(fake_registry.deliver(
                            fake_identity.cell_id,
                            connection_id,
                            BridgeMessage::WorkspaceObserved {
                                operation_id,
                                workspace,
                                observation: WorkspaceObservation {
                                    repo_path: "/company/repos/game".into(),
                                    workdir: "/company/worktrees/work-fake-r4".into(),
                                    source_commit: "d".repeat(40),
                                    source_tree: "e".repeat(40),
                                    status_digest: "f".repeat(64),
                                    dirty_entries: 0,
                                    environment_fingerprint: fake_environment.clone(),
                                    observed_at_ms: chrono::Utc::now().timestamp_millis(),
                                },
                            },
                        ));
                        break;
                    }
                    other => panic!("unexpected fake Runtime frame {other:?}"),
                }
            }
        });
        let prepared = prepare_workspace(
            &registry,
            &exact,
            workspace.clone(),
            Some("refs/heads/main".into()),
            None,
            None,
        )
        .await
        .unwrap();
        assert!(!prepared.reused);
        assert_eq!(prepared.observation.source_commit, "a".repeat(40));
        let auth = crate::acp::AgentAuth {
            model,
            effort: "medium".into(),
            company: exact.company.clone(),
            session_id: Uuid::new_v4().to_string(),
            coordination_token_env: "RESTLESS_SESSION_CAPABILITY=fake-coordinate".into(),
            coordination_token: "fake-coordinate".into(),
            gateway_token_env: "RESTLESS_MODEL_CAPABILITY".into(),
            gateway_token: "fake-model-capability".into(),
            gateway_url: "https://plane.example/internal/v1/model-gateway".into(),
            billing: crate::model_gateway::ModelBilling::Subscription,
        };
        let transport = open_agent_transport(
            &registry,
            &exact,
            &auth,
            &prepared.observation.workdir,
            "worker",
            "work:fake",
            crate::runtime::AgentHarness::RestlessManaged,
            "You are the bounded fake worker.",
        )
        .await
        .unwrap();
        let controls =
            crate::acp::AgentControls::company_actor("You are the bounded fake worker.".into())
                .unwrap();
        let reply = crate::acp::with_remote_agent(
            transport,
            crate::runtime::AgentHarness::RestlessManaged,
            &auth,
            &prepared.observation.workdir,
            "worker",
            "work:fake",
            controls,
            None,
            |session| {
                Box::pin(async move {
                    session.prompt("Return the bounded reply.").await?;
                    Ok(session.take_transcript().text)
                })
            },
        )
        .await
        .unwrap();
        assert_eq!(reply, "bounded bridge reply");
        let observed = observe_workspace(&registry, &exact, workspace, &expected_environment)
            .await
            .unwrap();
        assert_eq!(observed.source_commit, "d".repeat(40));
        assert_eq!(observed.source_tree, "e".repeat(40));
        assert_eq!(observed.dirty_entries, 0);
        tokio::time::timeout(Duration::from_secs(2), fake)
            .await
            .expect("fake Runtime exits after Core closes ACP")
            .unwrap();
    }

    #[tokio::test]
    async fn fake_runtime_bridge_runs_exact_gate_artifact_review_promotion_and_cleanup() {
        let registry = RuntimeBridgeRegistry::hosted();
        let exact = identity(1);
        let (sender, mut receiver) = mpsc::channel(32);
        let (credential_id, credential_epoch, expires_at) = credential(1);
        let connection_id = registry
            .register(
                exact.clone(),
                Uuid::new_v4(),
                runtime_release(),
                credential_id,
                credential_epoch,
                expires_at,
                sender,
            )
            .unwrap();
        let workspace = WorkspaceIdentity {
            attempt_id: Uuid::new_v4(),
            work_id: Uuid::new_v4(),
            work_revision: 7,
            repo: "friend-game".into(),
            worktree: "work-friend-game-r7".into(),
        };
        let environment = environment_fingerprint(&exact, &runtime_release()).unwrap();
        let candidate = CandidateIdentity {
            source_commit: "a".repeat(40),
            source_tree: "b".repeat(40),
            status_digest: "c".repeat(64),
            environment_fingerprint: environment.clone(),
        };
        let definition = GateDefinition {
            gate_id: Uuid::new_v4(),
            name: "playable-smoke".into(),
            sequence_no: 0,
            stage: "focused".into(),
            cwd: "@attempt".into(),
            argv: vec!["npm".into(), "test".into()],
            timeout_seconds: 30,
            resources: vec![restless_runtime_bridge_protocol::GateResource::Port],
        };
        let definition_digest =
            restless_runtime_bridge_protocol::gate_definition_digest(&definition);
        let gate_id = gate_operation_id(&exact, &workspace, &candidate, &definition_digest);
        let resources = GateResources {
            holder_token: gate_id.simple().to_string(),
            tempdir: format!(
                "/company/run/gates/{}/{}/tmp",
                workspace.attempt_id.simple(),
                gate_id.simple()
            ),
            process_group_marker: format!(
                "/company/run/gates/{}/{}/process-group.pid",
                workspace.attempt_id.simple(),
                gate_id.simple()
            ),
            port: Some(31_337),
            display: None,
        };
        let fake_registry = registry.clone();
        let fake_identity = exact.clone();
        let fake_workspace = workspace.clone();
        let fake_candidate = candidate.clone();
        let fake_definition = definition.clone();
        let fake_resources = resources.clone();
        let fake_environment = environment.clone();
        let fake = tokio::spawn(async move {
            let mut artifact_requests = 0_u8;
            while let Some(frame) = receiver.recv().await {
                match frame.message {
                    BridgeMessage::RunGate {
                        operation_id,
                        workspace,
                        candidate,
                        definition,
                        definition_digest,
                        resources,
                        ..
                    } => {
                        assert_eq!(workspace, fake_workspace);
                        assert_eq!(candidate, fake_candidate);
                        assert_eq!(definition, fake_definition);
                        assert_eq!(resources, fake_resources);
                        assert_eq!(
                            definition_digest,
                            restless_runtime_bridge_protocol::gate_definition_digest(&definition)
                        );
                        assert!(fake_registry.deliver(
                            fake_identity.cell_id,
                            connection_id,
                            BridgeMessage::GateCompleted {
                                operation_id,
                                workspace,
                                receipt: GateReceipt {
                                    candidate,
                                    gate_id: definition.gate_id,
                                    definition_digest,
                                    resources,
                                    output_digest: "d".repeat(64),
                                    output_excerpt: "all checks passed".into(),
                                    exit_code: Some(0),
                                    status: restless_runtime_bridge_protocol::GateCompletionStatus::Conclusive,
                                    duration_ms: 12,
                                    leaked_processes: 0,
                                    passed: true,
                                    observed_at_ms: chrono::Utc::now().timestamp_millis(),
                                },
                            },
                        ));
                    }
                    BridgeMessage::ObserveArtifact {
                        operation_id,
                        workspace,
                        candidate,
                        selector,
                        ..
                    } => {
                        artifact_requests += 1;
                        let returned_candidate = if artifact_requests == 1 {
                            CandidateIdentity {
                                source_commit: "1".repeat(40),
                                ..candidate.clone()
                            }
                        } else {
                            candidate.clone()
                        };
                        let returned_commit = returned_candidate.source_commit.clone();
                        assert!(fake_registry.deliver(
                            fake_identity.cell_id,
                            connection_id,
                            BridgeMessage::ArtifactObserved {
                                operation_id,
                                workspace,
                                receipt: ArtifactReceipt {
                                    candidate: returned_candidate,
                                    selector,
                                    uri: format!(
                                        "git:/company/repos/friend-game#{}",
                                        returned_commit
                                    ),
                                    content_digest: "e".repeat(64),
                                    file_count: 24,
                                    total_bytes: 12_345,
                                    observed_at_ms: chrono::Utc::now().timestamp_millis(),
                                },
                            },
                        ));
                    }
                    BridgeMessage::PrepareReviewCopy {
                        operation_id,
                        workspace,
                        candidate,
                        ..
                    } => {
                        assert!(fake_registry.deliver(
                            fake_identity.cell_id,
                            connection_id,
                            BridgeMessage::ReviewCopyPrepared {
                                operation_id,
                                workspace: workspace.clone(),
                                receipt: ReviewCopyReceipt {
                                    candidate: candidate.clone(),
                                    uri: format!(
                                        "/company/reviews/git/{}",
                                        candidate.source_commit
                                    ),
                                    alias_uri: format!(
                                        "/company/reviews/by-attempt/{}",
                                        workspace.attempt_id.simple()
                                    ),
                                    content_digest: "f".repeat(64),
                                    file_count: 24,
                                    access_probed: true,
                                    reused: false,
                                    observed_at_ms: chrono::Utc::now().timestamp_millis(),
                                },
                            },
                        ));
                    }
                    BridgeMessage::PromoteCandidate {
                        operation_id,
                        workspace,
                        candidate,
                        integration_branch,
                        ..
                    } => {
                        assert_eq!(integration_branch, "main");
                        assert!(fake_registry.deliver(
                            fake_identity.cell_id,
                            connection_id,
                            BridgeMessage::CandidatePromoted {
                                operation_id,
                                workspace,
                                receipt: PromotionReceipt {
                                    candidate: candidate.clone(),
                                    integration_branch,
                                    previous_commit: "2".repeat(40),
                                    previous_tree: "3".repeat(40),
                                    promoted_commit: candidate.source_commit.clone(),
                                    promoted_tree: candidate.source_tree.clone(),
                                    status_digest: "4".repeat(64),
                                    reused: false,
                                    observed_at_ms: chrono::Utc::now().timestamp_millis(),
                                },
                            },
                        ));
                    }
                    BridgeMessage::CleanupAttempt {
                        operation_id,
                        workspace,
                        expected_environment_fingerprint,
                        ..
                    } => {
                        assert_eq!(expected_environment_fingerprint, fake_environment);
                        let attempt = workspace.attempt_id.to_string();
                        assert!(fake_registry.deliver(
                            fake_identity.cell_id,
                            connection_id,
                            BridgeMessage::AttemptCleaned {
                                operation_id,
                                workspace: workspace.clone(),
                                receipt: AttemptCleanupReceipt {
                                    attempt_id: workspace.attempt_id,
                                    removed_paths: vec![
                                        format!("/company/run/attempts/{attempt}"),
                                        format!("/company/run/gates/{attempt}"),
                                    ],
                                    residue_count: 0,
                                    observed_at_ms: chrono::Utc::now().timestamp_millis(),
                                },
                            },
                        ));
                        break;
                    }
                    other => panic!("unexpected fake Runtime frame {other:?}"),
                }
            }
        });

        let gate = run_gate(
            &registry,
            &exact,
            workspace.clone(),
            candidate.clone(),
            definition,
            resources,
        )
        .await
        .unwrap();
        assert!(gate.passed);
        let changed = observe_artifact(
            &registry,
            &exact,
            workspace.clone(),
            candidate.clone(),
            ArtifactSelector::RepositoryTree,
        )
        .await;
        assert!(changed
            .unwrap_err()
            .to_string()
            .contains("different frozen evidence"));
        let artifact = observe_artifact(
            &registry,
            &exact,
            workspace.clone(),
            candidate.clone(),
            ArtifactSelector::RepositoryTree,
        )
        .await
        .unwrap();
        assert_eq!(artifact.candidate, candidate);
        let review = prepare_review_copy(&registry, &exact, workspace.clone(), candidate.clone())
            .await
            .unwrap();
        assert!(review.access_probed);
        let promotion = promote_candidate(
            &registry,
            &exact,
            workspace.clone(),
            candidate,
            "main".into(),
        )
        .await
        .unwrap();
        assert_eq!(promotion.integration_branch, "main");
        let cleanup = cleanup_attempt(&registry, &exact, workspace, environment)
            .await
            .unwrap();
        assert_eq!(cleanup.residue_count, 0);
        tokio::time::timeout(Duration::from_secs(2), fake)
            .await
            .expect("fake Runtime settled every typed operation")
            .unwrap();
    }
}
