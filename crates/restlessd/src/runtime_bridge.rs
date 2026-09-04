//! Account-plane endpoint and connection registry for hosted company Runtimes.
//!
//! A cell-local Runtime Agent dials this plane over WSS. This module verifies
//! the exact durable generation, consumes the one-use registration grant,
//! rotates it in-band, and only then publishes a request channel to Core. It
//! owns no lifecycle API and has no Runtime Supervisor credential.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use futures_util::{SinkExt as _, StreamExt as _};
use sha2::{Digest as _, Sha256};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    sync::{broadcast, mpsc, oneshot, Mutex, Notify, RwLock},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::hosted_runtime::{
    HostedRuntimeError, HostedRuntimeIdentity, RuntimeBridgeCapabilityKey, RuntimeBridgeGrant,
};
use crate::runtime_agent_protocol::{
    FileRequest, FileResponse, GovernedSignal, ProcessSignalRequest, ProcessStartRequest,
    ProcessStdinRequest, RuntimeAgentEvent, RuntimeAgentFeature, RuntimeAgentRequest,
    RuntimeAgentResponse, RuntimeAgentToPlane, RuntimeBridgeCapability, RuntimeCapabilityRenewal,
    RuntimeCapabilityRenewalConfirmed, RuntimeEventEnvelope, RuntimePlaneToAgent,
    RuntimeProtocolError, RuntimeProtocolErrorCode, RuntimeRegistration,
    RuntimeRegistrationAccepted, RuntimeRegistrationConfirmed, RuntimeRegistrationRejected,
    RuntimeRequestEnvelope, RuntimeResponseEnvelope, SensitiveString, ServiceCloseRequest,
    ServiceOpenRequest, ServiceWriteRequest, RUNTIME_AGENT_MAX_CHUNK_BYTES,
    RUNTIME_AGENT_MAX_FRAME_BYTES, RUNTIME_AGENT_MAX_UPLOAD_BYTES, RUNTIME_AGENT_PROTOCOL,
};
use crate::runtime_transport::{
    CompanyPath, RuntimeActiveProcess, RuntimeActivity, RuntimeComponentCheck,
    RuntimeComponentStatus, RuntimeDirectoryEntry, RuntimeDuplex, RuntimeFileKind,
    RuntimeFileMetadata, RuntimeProcess, RuntimeProcessControl, RuntimeProcessExit,
    RuntimeProcessSpec, RuntimeReadiness, RuntimeService, RuntimeSignal, RuntimeTransport,
    RuntimeTransportError,
};

const REGISTRATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const OUTBOUND_QUEUE: usize = 128;
const EVENT_QUEUE: usize = 128;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CAPABILITY_RENEWAL_LEAD: chrono::Duration = chrono::Duration::minutes(5);
const CAPABILITY_RENEWAL_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_READ_BYTES: usize = 32 * 1024 * 1024;
const MAX_LIST_ENTRIES: usize = 10_000;

/// Durable account-plane authority needed for one Runtime registration.
///
/// Implementations must compare every field, including generation, image,
/// source revision and volume. Consuming the grant must be atomic across plane
/// replicas/restarts; an in-memory set is not a production implementation.
#[async_trait]
pub trait RuntimeBridgeAuthority: Send + Sync {
    async fn exact_runtime_is_current(
        &self,
        identity: &HostedRuntimeIdentity,
    ) -> anyhow::Result<bool>;

    /// Atomically consume the JTI and, when the identity was admitted as a
    /// pending replacement, promote it to current. An implementation must
    /// never burn a grant for an identity it did not admit.
    async fn consume_registration_grant(
        &self,
        grant: &RuntimeBridgeGrant,
    ) -> anyhow::Result<RuntimeGrantConsumption>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeGrantConsumption {
    Accepted,
    Replayed,
    IdentityMismatch,
}

#[derive(Clone)]
pub struct RuntimeBridgeRegistry {
    inner: Arc<RegistryInner>,
}

struct RegistryInner {
    capability_key: RuntimeBridgeCapabilityKey,
    authority: Arc<dyn RuntimeBridgeAuthority>,
    connections: RwLock<HashMap<String, Arc<RuntimeBridgeConnection>>>,
}

impl fmt::Debug for RuntimeBridgeRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeBridgeRegistry")
            .field("capability_key", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl RuntimeBridgeRegistry {
    pub fn new(
        capability_key: RuntimeBridgeCapabilityKey,
        authority: Arc<dyn RuntimeBridgeAuthority>,
    ) -> Self {
        Self {
            inner: Arc::new(RegistryInner {
                capability_key,
                authority,
                connections: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Complete the registration handshake and serve one outbound Agent
    /// connection until either side closes it. Invalid handshakes receive one
    /// bounded rejection frame and are never inserted into the registry.
    pub async fn accept_socket(&self, mut socket: WebSocket) -> Result<(), RuntimeBridgeError> {
        let registration = match tokio::time::timeout(REGISTRATION_TIMEOUT, socket.recv()).await {
            Ok(Some(Ok(message))) => parse_registration(message)?,
            Ok(Some(Err(error))) => {
                return Err(RuntimeBridgeError::Transport(error.to_string()));
            }
            Ok(None) => {
                return Err(RuntimeBridgeError::Transport(
                    "agent closed before registration".into(),
                ));
            }
            Err(_) => {
                return Err(RuntimeBridgeError::Protocol(
                    "Runtime Agent registration timed out",
                ));
            }
        };

        let validated = match self.validate_registration(&registration, Utc::now()).await {
            Ok(validated) => validated,
            Err(error) => {
                let rejection = RuntimePlaneToAgent::Rejected(RuntimeRegistrationRejected {
                    code: error.protocol_code(),
                    retryable: error.retryable(),
                });
                let _ = send_frame(&mut socket, &rejection).await;
                return Err(error);
            }
        };

        let connection_id = Uuid::new_v4();
        let rotated = self
            .inner
            .capability_key
            .issue(&registration.identity, Utc::now())
            .map_err(RuntimeBridgeError::Capability)?;
        let rotated =
            RuntimeBridgeCapability::new(rotated).map_err(RuntimeBridgeError::Protocol)?;
        let rotated_grant = self
            .inner
            .capability_key
            .verify(rotated.expose(), &registration.identity, Utc::now())
            .map_err(RuntimeBridgeError::Capability)?;
        send_frame(
            &mut socket,
            &RuntimePlaneToAgent::Registered(RuntimeRegistrationAccepted {
                protocol: RUNTIME_AGENT_PROTOCOL.to_owned(),
                connection_id,
                server_time: Utc::now(),
                next_session_sequence: 1,
                renewed_capability: Some(rotated),
                renewed_capability_expires_at: Some(rotated_grant.expires_at),
            }),
        )
        .await?;

        let confirmation = match tokio::time::timeout(REGISTRATION_TIMEOUT, socket.recv()).await {
            Ok(Some(Ok(message))) => parse_confirmation(message)?,
            Ok(Some(Err(error))) => {
                return Err(RuntimeBridgeError::Transport(error.to_string()));
            }
            Ok(None) => {
                return Err(RuntimeBridgeError::Transport(
                    "agent closed before confirming capability rotation".into(),
                ));
            }
            Err(_) => {
                return Err(RuntimeBridgeError::Protocol(
                    "Runtime Agent capability persistence was not confirmed",
                ));
            }
        };
        if confirmation.connection_id != connection_id || !confirmation.persisted_capability {
            return Err(RuntimeBridgeError::Protocol(
                "Runtime Agent did not persist the connection-bound capability",
            ));
        }

        let (connection, outbound) = RuntimeBridgeConnection::new(
            connection_id,
            registration.identity,
            registration.desired_revision,
        );
        self.attach(Arc::clone(&connection)).await;
        let renewal_connection = Arc::clone(&connection);
        let renewal_key = self.inner.capability_key.clone();
        let renewal = tokio::spawn(async move {
            renewal_connection
                .maintain_reconnect_capability(renewal_key, rotated_grant.expires_at)
                .await;
        });
        let result = connection.run(socket, outbound).await;
        connection.close();
        let _ = renewal.await;
        self.detach(&connection).await;
        // The consumed bootstrap grant remains consumed even when rotation or
        // transport later fails. Fleet can request a fresh scoped grant; reuse
        // of the old one is never a recovery mechanism.
        drop(validated);
        result
    }

    async fn validate_registration(
        &self,
        registration: &RuntimeRegistration,
        now: DateTime<Utc>,
    ) -> Result<RuntimeBridgeGrant, RuntimeBridgeError> {
        if registration.protocol != RUNTIME_AGENT_PROTOCOL
            || registration.desired_revision < 1
            || registration.features.len() != RuntimeAgentFeature::ALL.len()
            || registration
                .features
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != RuntimeAgentFeature::ALL
                    .into_iter()
                    .collect::<BTreeSet<_>>()
        {
            return Err(RuntimeBridgeError::Protocol(
                "Runtime Agent registration contract or feature set is invalid",
            ));
        }
        let grant = self
            .inner
            .capability_key
            .verify(
                registration.capability.expose(),
                &registration.identity,
                now,
            )
            .map_err(RuntimeBridgeError::Capability)?;
        match self
            .inner
            .authority
            .consume_registration_grant(&grant)
            .await
            .map_err(|error| RuntimeBridgeError::Authority(error.to_string()))?
        {
            RuntimeGrantConsumption::Accepted => {}
            RuntimeGrantConsumption::Replayed => {
                return Err(RuntimeBridgeError::ReplayedCapability);
            }
            RuntimeGrantConsumption::IdentityMismatch => {
                return Err(RuntimeBridgeError::IdentityMismatch);
            }
        }
        if !self
            .inner
            .authority
            .exact_runtime_is_current(&registration.identity)
            .await
            .map_err(|error| RuntimeBridgeError::Authority(error.to_string()))?
        {
            return Err(RuntimeBridgeError::IdentityMismatch);
        }
        Ok(grant)
    }

    async fn attach(&self, connection: Arc<RuntimeBridgeConnection>) {
        let company = connection.identity.core_company_name();
        if let Some(previous) = self
            .inner
            .connections
            .write()
            .await
            .insert(company, Arc::clone(&connection))
        {
            previous.close();
        }
    }

    async fn detach(&self, connection: &Arc<RuntimeBridgeConnection>) {
        let company = connection.identity.core_company_name();
        let mut connections = self.inner.connections.write().await;
        if connections
            .get(&company)
            .is_some_and(|current| current.connection_id == connection.connection_id)
        {
            connections.remove(&company);
        }
        connection.close();
    }

    pub async fn connection(
        &self,
        company: &str,
    ) -> Result<Arc<RuntimeBridgeConnection>, RuntimeTransportError> {
        let connection = self
            .inner
            .connections
            .read()
            .await
            .get(company)
            .filter(|connection| !connection.closed.is_cancelled())
            .cloned()
            .ok_or(RuntimeTransportError::Unavailable)?;
        let still_current = self
            .inner
            .authority
            .exact_runtime_is_current(&connection.identity)
            .await
            .unwrap_or(false);
        if still_current {
            return Ok(connection);
        }
        connection.close();
        let mut connections = self.inner.connections.write().await;
        if connections
            .get(company)
            .is_some_and(|current| current.connection_id == connection.connection_id)
        {
            connections.remove(company);
        }
        Err(RuntimeTransportError::Unavailable)
    }

    #[cfg(test)]
    async fn attach_for_test(&self, connection: Arc<RuntimeBridgeConnection>) {
        self.attach(connection).await;
    }
}

#[derive(Default)]
struct ProcessExitState {
    result: Mutex<Option<Result<RuntimeProcessExit, RuntimeTransportError>>>,
    changed: Notify,
}

impl ProcessExitState {
    async fn set(&self, result: Result<RuntimeProcessExit, RuntimeTransportError>) {
        let mut current = self.result.lock().await;
        if current.is_none() {
            *current = Some(result);
            drop(current);
            self.changed.notify_waiters();
        }
    }

    async fn wait(&self) -> Result<RuntimeProcessExit, RuntimeTransportError> {
        loop {
            let changed = self.changed.notified();
            if let Some(result) = self.result.lock().await.clone() {
                return result;
            }
            changed.await;
        }
    }
}

struct BridgeProcessControl {
    connection: Arc<RuntimeBridgeConnection>,
    process_id: Uuid,
    exit: Arc<ProcessExitState>,
}

#[async_trait]
impl RuntimeProcessControl for BridgeProcessControl {
    async fn signal(&self, signal: RuntimeSignal) -> Result<(), RuntimeTransportError> {
        let wire_signal: GovernedSignal = signal.into();
        match self
            .connection
            .request(
                Uuid::new_v4(),
                request_deadline(),
                RuntimeAgentRequest::ProcessSignal(ProcessSignalRequest {
                    process_id: self.process_id,
                    signal: wire_signal,
                }),
            )
            .await?
        {
            RuntimeAgentResponse::ProcessSignalAccepted(accepted)
                if accepted.process_id == self.process_id && accepted.signal == wire_signal =>
            {
                Ok(())
            }
            RuntimeAgentResponse::Error(error) => Err(protocol_error(error)),
            _ => Err(RuntimeTransportError::Conflict),
        }
    }

    async fn wait(&self) -> Result<RuntimeProcessExit, RuntimeTransportError> {
        self.exit.wait().await
    }
}

#[async_trait]
impl RuntimeTransport for RuntimeBridgeRegistry {
    async fn readiness(&self, company: &str) -> Result<RuntimeReadiness, RuntimeTransportError> {
        let connection = self.checked_connection(company).await?;
        let response = connection
            .request(
                Uuid::new_v4(),
                request_deadline(),
                RuntimeAgentRequest::Readiness,
            )
            .await?;
        let RuntimeAgentResponse::Readiness(readiness) = response else {
            return match response {
                RuntimeAgentResponse::Error(error) => Err(protocol_error(error)),
                _ => Err(RuntimeTransportError::Conflict),
            };
        };
        if readiness.protocol != RUNTIME_AGENT_PROTOCOL
            || readiness.runtime_id != connection.identity.runtime_id
            || readiness.runtime_generation != connection.identity.runtime_generation
            || readiness.runtime_image != connection.identity.runtime_image
            || readiness.source_revision != connection.identity.source_revision
            || readiness.volume_name != connection.identity.volume_name
            || readiness.desired_revision != connection.desired_revision
            || readiness.core_version.is_empty()
            || readiness.api_contract_version.is_empty()
            || readiness.assertion_contract_version.is_empty()
            || readiness.schema_version.is_empty()
        {
            return Err(RuntimeTransportError::Conflict);
        }
        let unique = readiness
            .checks
            .iter()
            .map(|check| check.component)
            .collect::<BTreeSet<_>>();
        if readiness.checks.len() != 7 || unique.len() != 7 {
            return Err(RuntimeTransportError::Conflict);
        }
        let components = readiness
            .checks
            .into_iter()
            .map(|check| RuntimeComponentCheck {
                name: readiness_component_name(check.component).to_owned(),
                status: match check.status {
                    crate::runtime_agent_protocol::RuntimeCheckStatus::Ready => {
                        RuntimeComponentStatus::Ready
                    }
                    crate::runtime_agent_protocol::RuntimeCheckStatus::Unavailable => {
                        RuntimeComponentStatus::Degraded
                    }
                },
            })
            .collect::<Vec<_>>();
        if readiness.ready
            != components
                .iter()
                .all(|check| check.status == RuntimeComponentStatus::Ready)
        {
            return Err(RuntimeTransportError::Conflict);
        }
        Ok(RuntimeReadiness {
            runtime_id: connection.identity.runtime_id.clone(),
            runtime_generation: connection.identity.runtime_generation,
            runtime_image: connection.identity.runtime_image.clone(),
            source_revision: connection.identity.source_revision.clone(),
            volume_name: connection.identity.volume_name.clone(),
            observed_at: Utc::now(),
            components,
        })
    }

    async fn start_process(
        &self,
        specification: RuntimeProcessSpec,
    ) -> Result<RuntimeProcess, RuntimeTransportError> {
        specification.validate(Utc::now())?;
        let company = specification.authority.company().to_owned();
        let connection = self.checked_connection(&company).await?;
        let working_directory = (&specification.working_directory).try_into()?;
        let mut environment = BTreeMap::new();
        for variable in &specification.environment {
            let value = SensitiveString::new(variable.value.clone()).map_err(|_| {
                RuntimeTransportError::InvalidRequest(
                    "Runtime process environment value is invalid or too large",
                )
            })?;
            if environment.insert(variable.name.clone(), value).is_some() {
                return Err(RuntimeTransportError::InvalidRequest(
                    "Runtime process environment names must be unique",
                ));
            }
        }
        let process_id = specification.operation_id;
        let stdout_events = connection.subscribe();
        let stderr_events = connection.subscribe();
        let exit_events = connection.subscribe();
        let response = connection
            .request(
                specification.operation_id,
                specification.deadline,
                RuntimeAgentRequest::ProcessStart(ProcessStartRequest {
                    process_id,
                    authority: (&specification.authority).into(),
                    executable: specification.executable,
                    arguments: specification.arguments,
                    working_directory,
                    environment,
                    stdin: true,
                }),
            )
            .await?;
        let RuntimeAgentResponse::ProcessStarted(started) = response else {
            return match response {
                RuntimeAgentResponse::Error(error) => Err(protocol_error(error)),
                _ => Err(RuntimeTransportError::Conflict),
            };
        };
        if started.process_id != process_id || started.pid == 0 {
            return Err(RuntimeTransportError::Conflict);
        }

        let (stdin_writer, stdin_reader) = tokio::io::duplex(64 * 1024);
        let (stdout_writer, stdout_reader) = tokio::io::duplex(64 * 1024);
        let (stderr_writer, stderr_reader) = tokio::io::duplex(64 * 1024);
        let exit = Arc::new(ProcessExitState::default());

        tokio::spawn(pump_process_stdin(
            Arc::clone(&connection),
            process_id,
            stdin_reader,
        ));
        tokio::spawn(pump_process_output(
            Arc::clone(&connection),
            process_id,
            crate::runtime_agent_protocol::ProcessStream::Stdout,
            stdout_writer,
            stdout_events,
        ));
        tokio::spawn(pump_process_output(
            Arc::clone(&connection),
            process_id,
            crate::runtime_agent_protocol::ProcessStream::Stderr,
            stderr_writer,
            stderr_events,
        ));
        tokio::spawn(observe_process_exit(
            Arc::clone(&connection),
            process_id,
            Arc::clone(&exit),
            exit_events,
        ));

        Ok(RuntimeProcess {
            process_id,
            pid: started.pid,
            stdin: Box::pin(stdin_writer),
            stdout: Box::pin(stdout_reader),
            stderr: Box::pin(stderr_reader),
            control: Box::new(BridgeProcessControl {
                connection,
                process_id,
                exit,
            }),
        })
    }

    async fn stat(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<RuntimeFileMetadata, RuntimeTransportError> {
        let connection = self.checked_connection(company).await?;
        let wire_path: crate::runtime_agent_protocol::RuntimePath = path.try_into()?;
        match connection
            .request(
                Uuid::new_v4(),
                request_deadline(),
                RuntimeAgentRequest::File(FileRequest::Stat {
                    path: wire_path.clone(),
                }),
            )
            .await?
        {
            RuntimeAgentResponse::File(FileResponse::Stat(metadata))
                if metadata.path == wire_path =>
            {
                convert_file_metadata(metadata)
            }
            RuntimeAgentResponse::Error(error) => Err(protocol_error(error)),
            _ => Err(RuntimeTransportError::Conflict),
        }
    }

    async fn list(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<Vec<RuntimeDirectoryEntry>, RuntimeTransportError> {
        let connection = self.checked_connection(company).await?;
        let wire_path: crate::runtime_agent_protocol::RuntimePath = path.try_into()?;
        let mut cursor = None;
        let mut seen_cursors = HashSet::new();
        let mut entries = Vec::new();
        loop {
            let response = connection
                .request(
                    Uuid::new_v4(),
                    request_deadline(),
                    RuntimeAgentRequest::File(FileRequest::List {
                        path: wire_path.clone(),
                        cursor: cursor.clone(),
                        limit: 256,
                    }),
                )
                .await?;
            let RuntimeAgentResponse::File(FileResponse::List(page)) = response else {
                return match response {
                    RuntimeAgentResponse::Error(error) => Err(protocol_error(error)),
                    _ => Err(RuntimeTransportError::Conflict),
                };
            };
            if page.path != wire_path || page.entries.len() > 256 {
                return Err(RuntimeTransportError::Conflict);
            }
            for entry in page.entries {
                if entry.name.is_empty()
                    || entry.name.len() > 255
                    || entry.name.contains(['/', '\0', '\r', '\n'])
                    || matches!(entry.name.as_str(), "." | "..")
                {
                    return Err(RuntimeTransportError::Conflict);
                }
                let metadata = convert_list_metadata(&entry)?;
                entries.push(RuntimeDirectoryEntry {
                    name: entry.name,
                    metadata,
                });
                if entries.len() > MAX_LIST_ENTRIES {
                    return Err(RuntimeTransportError::InvalidRequest(
                        "Runtime directory contains too many entries",
                    ));
                }
            }
            let Some(next) = page.next_cursor else {
                break;
            };
            if next.is_empty() || next.len() > 4096 || !seen_cursors.insert(next.clone()) {
                return Err(RuntimeTransportError::Conflict);
            }
            cursor = Some(next);
        }
        Ok(entries)
    }

    async fn read(
        &self,
        company: &str,
        path: &CompanyPath,
        maximum_bytes: usize,
    ) -> Result<Vec<u8>, RuntimeTransportError> {
        if maximum_bytes == 0 || maximum_bytes > MAX_READ_BYTES {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime file read limit must be between one byte and 32 MiB",
            ));
        }
        let connection = self.checked_connection(company).await?;
        let wire_path: crate::runtime_agent_protocol::RuntimePath = path.try_into()?;
        let mut result = Vec::with_capacity(maximum_bytes.min(RUNTIME_AGENT_MAX_CHUNK_BYTES));
        let mut offset = 0_u64;
        while result.len() < maximum_bytes {
            let request_bytes = (maximum_bytes - result.len())
                .min(RUNTIME_AGENT_MAX_CHUNK_BYTES)
                .try_into()
                .expect("Runtime Agent chunk limit fits u32");
            let response = connection
                .request(
                    Uuid::new_v4(),
                    request_deadline(),
                    RuntimeAgentRequest::File(FileRequest::Read {
                        path: wire_path.clone(),
                        offset,
                        max_bytes: request_bytes,
                    }),
                )
                .await?;
            let RuntimeAgentResponse::File(FileResponse::Read(chunk)) = response else {
                return match response {
                    RuntimeAgentResponse::Error(error) => Err(protocol_error(error)),
                    _ => Err(RuntimeTransportError::Conflict),
                };
            };
            if chunk.path != wire_path || chunk.offset != offset {
                return Err(RuntimeTransportError::Conflict);
            }
            let decoded = decode_bounded(&chunk.data_base64, request_bytes as usize)?;
            if decoded.is_empty() && !chunk.eof {
                return Err(RuntimeTransportError::Conflict);
            }
            offset = offset
                .checked_add(decoded.len() as u64)
                .ok_or(RuntimeTransportError::Conflict)?;
            result.extend_from_slice(&decoded);
            if chunk.eof {
                break;
            }
        }
        Ok(result)
    }

    async fn atomic_write(
        &self,
        company: &str,
        operation_id: Uuid,
        path: &CompanyPath,
        contents: &[u8],
        mode: u32,
    ) -> Result<(), RuntimeTransportError> {
        if operation_id.is_nil()
            || contents.len() as u64 > RUNTIME_AGENT_MAX_UPLOAD_BYTES
            || mode > 0o777
        {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime atomic write identity, size, or mode is invalid",
            ));
        }
        let connection = self.checked_connection(company).await?;
        let wire_path: crate::runtime_agent_protocol::RuntimePath = path.try_into()?;
        let digest = sha256_hex(contents);
        if contents.len() <= RUNTIME_AGENT_MAX_CHUNK_BYTES {
            let response = connection
                .request(
                    operation_id,
                    request_deadline(),
                    RuntimeAgentRequest::File(FileRequest::AtomicWrite {
                        path: wire_path.clone(),
                        data_base64: BASE64.encode(contents),
                        expected_sha256: None,
                        mode,
                    }),
                )
                .await?;
            return validate_written(response, &wire_path, contents.len() as u64, &digest);
        }

        let result = upload_file(
            Arc::clone(&connection),
            operation_id,
            wire_path.clone(),
            contents,
            mode,
            &digest,
        )
        .await;
        if result.is_err() {
            let _ = connection
                .request(
                    Uuid::new_v4(),
                    request_deadline(),
                    RuntimeAgentRequest::File(FileRequest::UploadAbort {
                        write_id: operation_id,
                    }),
                )
                .await;
        }
        result
    }

    async fn rename(
        &self,
        company: &str,
        operation_id: Uuid,
        source: &CompanyPath,
        destination: &CompanyPath,
    ) -> Result<(), RuntimeTransportError> {
        if operation_id.is_nil() {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime rename operation ID must be non-nil",
            ));
        }
        let connection = self.checked_connection(company).await?;
        let from: crate::runtime_agent_protocol::RuntimePath = source.try_into()?;
        let to: crate::runtime_agent_protocol::RuntimePath = destination.try_into()?;
        match connection
            .request(
                operation_id,
                request_deadline(),
                RuntimeAgentRequest::File(FileRequest::Rename {
                    from,
                    to: to.clone(),
                    no_replace: false,
                }),
            )
            .await?
        {
            RuntimeAgentResponse::File(FileResponse::Renamed(mutation))
                if mutation.path == to && valid_sha256(&mutation.sha256) =>
            {
                Ok(())
            }
            RuntimeAgentResponse::Error(error) => Err(protocol_error(error)),
            _ => Err(RuntimeTransportError::Conflict),
        }
    }

    async fn digest(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<[u8; 32], RuntimeTransportError> {
        let connection = self.checked_connection(company).await?;
        let wire_path: crate::runtime_agent_protocol::RuntimePath = path.try_into()?;
        match connection
            .request(
                Uuid::new_v4(),
                request_deadline(),
                RuntimeAgentRequest::File(FileRequest::Digest {
                    path: wire_path.clone(),
                }),
            )
            .await?
        {
            RuntimeAgentResponse::File(FileResponse::Digest(digest))
                if digest.path == wire_path =>
            {
                decode_sha256(&digest.sha256)
            }
            RuntimeAgentResponse::Error(error) => Err(protocol_error(error)),
            _ => Err(RuntimeTransportError::Conflict),
        }
    }

    async fn open_service(
        &self,
        company: &str,
        operation_id: Uuid,
        service: RuntimeService,
        idle_timeout: Duration,
    ) -> Result<RuntimeDuplex, RuntimeTransportError> {
        if operation_id.is_nil()
            || idle_timeout < Duration::from_secs(1)
            || idle_timeout > Duration::from_secs(24 * 60 * 60)
        {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime service identity or idle timeout is invalid",
            ));
        }
        service.loopback_port()?;
        let connection = self.checked_connection(company).await?;
        let stream_id = operation_id;
        let wire_service: crate::runtime_agent_protocol::RuntimeService = service.into();
        let events = connection.subscribe();
        let response = connection
            .request(
                operation_id,
                request_deadline(),
                RuntimeAgentRequest::ServiceOpen(ServiceOpenRequest {
                    stream_id,
                    service: wire_service.clone(),
                    idle_timeout_ms: idle_timeout.as_millis().try_into().map_err(|_| {
                        RuntimeTransportError::InvalidRequest(
                            "Runtime service idle timeout is too large",
                        )
                    })?,
                }),
            )
            .await?;
        match response {
            RuntimeAgentResponse::ServiceOpened(opened)
                if opened.stream_id == stream_id && opened.service == wire_service => {}
            RuntimeAgentResponse::Error(error) => return Err(protocol_error(error)),
            _ => return Err(RuntimeTransportError::Conflict),
        }
        let (caller, bridge) = tokio::io::duplex(128 * 1024);
        tokio::spawn(pump_service(connection, stream_id, bridge, events));
        Ok(Box::new(caller))
    }

    async fn activity(&self, company: &str) -> Result<RuntimeActivity, RuntimeTransportError> {
        let connection = self.checked_connection(company).await?;
        let response = connection
            .request(
                Uuid::new_v4(),
                request_deadline(),
                RuntimeAgentRequest::Activity,
            )
            .await?;
        let RuntimeAgentResponse::Activity(activity) = response else {
            return match response {
                RuntimeAgentResponse::Error(error) => Err(protocol_error(error)),
                _ => Err(RuntimeTransportError::Conflict),
            };
        };
        let mut process_ids = HashSet::new();
        let mut active_processes = Vec::with_capacity(activity.processes.len());
        for process in activity.processes {
            let authority: crate::runtime_transport::RuntimeProcessAuthority =
                (&process.authority).into();
            if process.process_id.is_nil()
                || process.pid == 0
                || !process_ids.insert(process.process_id)
                || authority.company() != company
                || authority.validate().is_err()
            {
                return Err(RuntimeTransportError::Conflict);
            }
            active_processes.push(RuntimeActiveProcess {
                process_id: process.process_id,
                pid: process.pid,
                authority,
                started_at: process.started_at,
            });
        }
        let mut stream_ids = HashSet::new();
        for stream in &activity.service_streams {
            let service: RuntimeService = stream.service.clone().into();
            if stream.stream_id.is_nil()
                || !stream_ids.insert(stream.stream_id)
                || service.loopback_port().is_err()
            {
                return Err(RuntimeTransportError::Conflict);
            }
        }
        Ok(RuntimeActivity {
            observed_at: activity.observed_at,
            active_processes,
            open_service_streams: stream_ids.len(),
        })
    }
}

impl RuntimeBridgeRegistry {
    async fn checked_connection(
        &self,
        company: &str,
    ) -> Result<Arc<RuntimeBridgeConnection>, RuntimeTransportError> {
        crate::runtime_transport::validate_company(company)?;
        let connection = self.connection(company).await?;
        if connection.identity.core_company_name() != company {
            return Err(RuntimeTransportError::Conflict);
        }
        Ok(connection)
    }
}

fn request_deadline() -> DateTime<Utc> {
    Utc::now()
        + chrono::Duration::from_std(REQUEST_TIMEOUT)
            .expect("the bounded Runtime request timeout fits chrono")
}

fn readiness_component_name(
    component: crate::runtime_agent_protocol::RuntimeReadinessComponent,
) -> &'static str {
    use crate::runtime_agent_protocol::RuntimeReadinessComponent;
    match component {
        RuntimeReadinessComponent::RuntimeAgent => "runtime_agent",
        RuntimeReadinessComponent::PersistentVolume => "persistent_volume",
        RuntimeReadinessComponent::SessionScratch => "session_scratch",
        RuntimeReadinessComponent::ProcessExecution => "process_execution",
        RuntimeReadinessComponent::Desktop => "desktop",
        RuntimeReadinessComponent::BrowserBroker => "browser_broker",
        RuntimeReadinessComponent::ReleaseHealth => "release_health",
    }
}

fn protocol_error(error: RuntimeProtocolError) -> RuntimeTransportError {
    use RuntimeProtocolErrorCode as Code;
    match error.code {
        Code::InvalidCapability | Code::Expired | Code::PermissionDenied => {
            RuntimeTransportError::Unauthorized
        }
        Code::InvalidIdentity
        | Code::SequenceViolation
        | Code::OperationConflict
        | Code::OperationPending
        | Code::ResourceExists => RuntimeTransportError::Conflict,
        Code::ResourceNotFound => RuntimeTransportError::NotFound,
        Code::ProcessUnavailable | Code::ServiceUnavailable => RuntimeTransportError::Unavailable,
        Code::InvalidProtocol => RuntimeTransportError::Conflict,
        Code::InvalidRequest | Code::LimitExceeded => {
            RuntimeTransportError::Remote("Runtime Agent rejected the bounded request".to_owned())
        }
        Code::Internal => {
            RuntimeTransportError::Remote("Runtime Agent reported an internal failure".to_owned())
        }
    }
}

fn convert_file_metadata(
    metadata: crate::runtime_agent_protocol::RuntimeFileMetadata,
) -> Result<RuntimeFileMetadata, RuntimeTransportError> {
    if metadata.mode > 0o777 {
        return Err(RuntimeTransportError::Conflict);
    }
    Ok(RuntimeFileMetadata {
        kind: match metadata.kind {
            crate::runtime_agent_protocol::RuntimeFileKind::File => RuntimeFileKind::File,
            crate::runtime_agent_protocol::RuntimeFileKind::Directory => RuntimeFileKind::Directory,
        },
        size: metadata.size,
        modified_at: metadata.modified_at,
        mode: metadata.mode,
    })
}

fn convert_list_metadata(
    entry: &crate::runtime_agent_protocol::RuntimeFileListEntry,
) -> Result<RuntimeFileMetadata, RuntimeTransportError> {
    if entry.mode > 0o777 {
        return Err(RuntimeTransportError::Conflict);
    }
    Ok(RuntimeFileMetadata {
        kind: match entry.kind {
            crate::runtime_agent_protocol::RuntimeFileKind::File => RuntimeFileKind::File,
            crate::runtime_agent_protocol::RuntimeFileKind::Directory => RuntimeFileKind::Directory,
        },
        size: entry.size,
        modified_at: entry.modified_at,
        mode: entry.mode,
    })
}

fn decode_bounded(value: &str, maximum: usize) -> Result<Vec<u8>, RuntimeTransportError> {
    let decoded = BASE64
        .decode(value)
        .map_err(|_| RuntimeTransportError::Conflict)?;
    if decoded.len() > maximum {
        return Err(RuntimeTransportError::Conflict);
    }
    Ok(decoded)
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn decode_sha256(value: &str) -> Result<[u8; 32], RuntimeTransportError> {
    if !valid_sha256(value) {
        return Err(RuntimeTransportError::Conflict);
    }
    let mut result = [0_u8; 32];
    for (index, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| RuntimeTransportError::Conflict)?;
    }
    Ok(result)
}

fn validate_written(
    response: RuntimeAgentResponse,
    path: &crate::runtime_agent_protocol::RuntimePath,
    size: u64,
    digest: &str,
) -> Result<(), RuntimeTransportError> {
    match response {
        RuntimeAgentResponse::File(FileResponse::Written(mutation))
            if mutation.path == *path && mutation.size == size && mutation.sha256 == digest =>
        {
            Ok(())
        }
        RuntimeAgentResponse::Error(error) => Err(protocol_error(error)),
        _ => Err(RuntimeTransportError::Conflict),
    }
}

async fn upload_file(
    connection: Arc<RuntimeBridgeConnection>,
    write_id: Uuid,
    path: crate::runtime_agent_protocol::RuntimePath,
    contents: &[u8],
    mode: u32,
    digest: &str,
) -> Result<(), RuntimeTransportError> {
    let response = connection
        .request(
            write_id,
            request_deadline(),
            RuntimeAgentRequest::File(FileRequest::UploadBegin {
                write_id,
                path: path.clone(),
                exact_size: contents.len() as u64,
                exact_sha256: digest.to_owned(),
                expected_sha256: None,
                mode,
            }),
        )
        .await?;
    match response {
        RuntimeAgentResponse::File(FileResponse::UploadBegun(upload))
            if upload.write_id == write_id
                && upload.path == path
                && upload.exact_size == contents.len() as u64
                && upload.exact_sha256 == digest
                && upload.mode == mode
                && upload.next_offset == 0 => {}
        RuntimeAgentResponse::Error(error) => return Err(protocol_error(error)),
        _ => return Err(RuntimeTransportError::Conflict),
    }

    let mut offset = 0_usize;
    while offset < contents.len() {
        let end = (offset + RUNTIME_AGENT_MAX_CHUNK_BYTES).min(contents.len());
        let response = connection
            .request(
                Uuid::new_v4(),
                request_deadline(),
                RuntimeAgentRequest::File(FileRequest::UploadChunk {
                    write_id,
                    offset: offset as u64,
                    data_base64: BASE64.encode(&contents[offset..end]),
                }),
            )
            .await?;
        match response {
            RuntimeAgentResponse::File(FileResponse::UploadChunkAccepted(progress))
                if progress.write_id == write_id
                    && progress.accepted_bytes == (end - offset) as u32
                    && progress.next_offset == end as u64 => {}
            RuntimeAgentResponse::Error(error) => return Err(protocol_error(error)),
            _ => return Err(RuntimeTransportError::Conflict),
        }
        offset = end;
    }

    let response = connection
        .request(
            Uuid::new_v4(),
            request_deadline(),
            RuntimeAgentRequest::File(FileRequest::UploadCommit { write_id }),
        )
        .await?;
    validate_written(response, &path, contents.len() as u64, digest)
}

async fn pump_process_stdin(
    connection: Arc<RuntimeBridgeConnection>,
    process_id: Uuid,
    mut input: tokio::io::DuplexStream,
) {
    let mut buffer = vec![0_u8; 48 * 1024];
    loop {
        let count = tokio::select! {
            _ = connection.closed.cancelled() => return,
            count = input.read(&mut buffer) => match count {
                Ok(count) => count,
                Err(_) => return,
            },
        };
        let eof = count == 0;
        let response = connection
            .request(
                Uuid::new_v4(),
                request_deadline(),
                RuntimeAgentRequest::ProcessStdin(ProcessStdinRequest {
                    process_id,
                    data_base64: BASE64.encode(&buffer[..count]),
                    eof,
                }),
            )
            .await;
        match response {
            Ok(RuntimeAgentResponse::ProcessInputAccepted(accepted))
                if accepted.process_id == process_id
                    && accepted.decoded_bytes == count as u32
                    && accepted.eof == eof => {}
            _ => return,
        }
        if eof {
            return;
        }
    }
}

async fn pump_process_output(
    connection: Arc<RuntimeBridgeConnection>,
    process_id: Uuid,
    stream: crate::runtime_agent_protocol::ProcessStream,
    mut output: tokio::io::DuplexStream,
    mut events: broadcast::Receiver<RuntimeEventEnvelope>,
) {
    loop {
        let event = tokio::select! {
            _ = connection.closed.cancelled() => break,
            event = events.recv() => match event {
                Ok(event) => event,
                Err(_) => break,
            },
        };
        match event.event {
            RuntimeAgentEvent::ProcessOutput(data)
                if data.process_id == process_id && data.stream == stream =>
            {
                let Ok(decoded) = decode_bounded(&data.data_base64, RUNTIME_AGENT_MAX_CHUNK_BYTES)
                else {
                    break;
                };
                if output.write_all(&decoded).await.is_err() {
                    break;
                }
                if data.eof {
                    break;
                }
            }
            RuntimeAgentEvent::ProcessExited(exited) if exited.process_id == process_id => break,
            _ => {}
        }
    }
    let _ = output.shutdown().await;
}

async fn observe_process_exit(
    connection: Arc<RuntimeBridgeConnection>,
    process_id: Uuid,
    exit: Arc<ProcessExitState>,
    mut events: broadcast::Receiver<RuntimeEventEnvelope>,
) {
    loop {
        let event = tokio::select! {
            _ = connection.closed.cancelled() => {
                exit.set(Err(RuntimeTransportError::Unavailable)).await;
                return;
            }
            event = events.recv() => match event {
                Ok(event) => event,
                Err(_) => {
                    exit.set(Err(RuntimeTransportError::Transport(
                        "Runtime process event stream was lost".to_owned(),
                    ))).await;
                    return;
                }
            },
        };
        let RuntimeAgentEvent::ProcessExited(observed) = event.event else {
            continue;
        };
        if observed.process_id != process_id {
            continue;
        }
        if observed.exit_code.is_some() && observed.signal.is_some() {
            exit.set(Err(RuntimeTransportError::Conflict)).await;
        } else {
            exit.set(Ok(RuntimeProcessExit {
                code: observed.exit_code,
                signal: observed.signal,
                finished_at: observed.finished_at,
            }))
            .await;
        }
        return;
    }
}

async fn pump_service(
    connection: Arc<RuntimeBridgeConnection>,
    stream_id: Uuid,
    mut bridge: tokio::io::DuplexStream,
    mut events: broadcast::Receiver<RuntimeEventEnvelope>,
) {
    let mut input_open = true;
    let mut buffer = vec![0_u8; 48 * 1024];
    loop {
        tokio::select! {
            _ = connection.closed.cancelled() => break,
            read = bridge.read(&mut buffer), if input_open => {
                let Ok(count) = read else { break };
                let eof = count == 0;
                let response = connection.request(
                    Uuid::new_v4(),
                    request_deadline(),
                    RuntimeAgentRequest::ServiceWrite(ServiceWriteRequest {
                        stream_id,
                        data_base64: BASE64.encode(&buffer[..count]),
                        eof,
                    }),
                ).await;
                match response {
                    Ok(RuntimeAgentResponse::ServiceWriteAccepted(accepted))
                        if accepted.stream_id == stream_id
                            && accepted.decoded_bytes == count as u32
                            && accepted.eof == eof => {}
                    _ => break,
                }
                input_open = !eof;
            }
            event = events.recv() => {
                let Ok(event) = event else { break };
                match event.event {
                    RuntimeAgentEvent::ServiceOutput(data) if data.stream_id == stream_id => {
                        let Ok(decoded) = decode_bounded(
                            &data.data_base64,
                            RUNTIME_AGENT_MAX_CHUNK_BYTES,
                        ) else { break };
                        if bridge.write_all(&decoded).await.is_err() {
                            break;
                        }
                        if data.eof {
                            let _ = bridge.shutdown().await;
                        }
                    }
                    RuntimeAgentEvent::ServiceClosed(closed) if closed.stream_id == stream_id => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
    let _ = bridge.shutdown().await;
    let _ = connection
        .request(
            Uuid::new_v4(),
            request_deadline(),
            RuntimeAgentRequest::ServiceClose(ServiceCloseRequest { stream_id }),
        )
        .await;
}

pub struct RuntimeBridgeConnection {
    pub connection_id: Uuid,
    pub identity: HostedRuntimeIdentity,
    pub desired_revision: i64,
    outbound: mpsc::Sender<RuntimePlaneToAgent>,
    pending: Mutex<HashMap<Uuid, PendingResponse>>,
    pending_renewal: Mutex<Option<PendingRenewal>>,
    events: broadcast::Sender<RuntimeEventEnvelope>,
    next_session_sequence: AtomicU64,
    closed: CancellationToken,
}

impl fmt::Debug for RuntimeBridgeConnection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeBridgeConnection")
            .field("connection_id", &self.connection_id)
            .field("identity", &self.identity)
            .field("desired_revision", &self.desired_revision)
            .finish_non_exhaustive()
    }
}

struct PendingResponse {
    session_sequence: u64,
    sender: oneshot::Sender<crate::runtime_agent_protocol::RuntimeAgentResponse>,
}

struct PendingRenewal {
    renewal_id: Uuid,
    sender: oneshot::Sender<bool>,
}

impl RuntimeBridgeConnection {
    fn new(
        connection_id: Uuid,
        identity: HostedRuntimeIdentity,
        desired_revision: i64,
    ) -> (Arc<Self>, mpsc::Receiver<RuntimePlaneToAgent>) {
        let (outbound, receiver) = mpsc::channel(OUTBOUND_QUEUE);
        let (events, _) = broadcast::channel(EVENT_QUEUE);
        (
            Arc::new(Self {
                connection_id,
                identity,
                desired_revision,
                outbound,
                pending: Mutex::new(HashMap::new()),
                pending_renewal: Mutex::new(None),
                events,
                next_session_sequence: AtomicU64::new(1),
                closed: CancellationToken::new(),
            }),
            receiver,
        )
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RuntimeEventEnvelope> {
        self.events.subscribe()
    }

    pub async fn request(
        &self,
        operation_id: Uuid,
        deadline: DateTime<Utc>,
        request: crate::runtime_agent_protocol::RuntimeAgentRequest,
    ) -> Result<crate::runtime_agent_protocol::RuntimeAgentResponse, RuntimeTransportError> {
        if operation_id.is_nil() || deadline <= Utc::now() {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime request requires a non-nil operation and future deadline",
            ));
        }
        if self.closed.is_cancelled() {
            return Err(RuntimeTransportError::Unavailable);
        }
        let sequence = match self.next_session_sequence.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |current| current.checked_add(1),
        ) {
            Ok(sequence) => sequence,
            Err(_) => {
                self.close();
                return Err(RuntimeTransportError::Conflict);
            }
        };
        if sequence == 0 {
            self.close();
            return Err(RuntimeTransportError::Conflict);
        }
        let (sender, receiver) = oneshot::channel();
        if self
            .pending
            .lock()
            .await
            .insert(
                operation_id,
                PendingResponse {
                    session_sequence: sequence,
                    sender,
                },
            )
            .is_some()
        {
            self.close();
            return Err(RuntimeTransportError::Conflict);
        }
        let envelope = RuntimeRequestEnvelope {
            operation_id,
            deadline,
            session_sequence: sequence,
            request,
        };
        if self
            .outbound
            .send(RuntimePlaneToAgent::Request(envelope))
            .await
            .is_err()
        {
            self.pending.lock().await.remove(&operation_id);
            self.close();
            return Err(RuntimeTransportError::Unavailable);
        }
        let wait = (deadline - Utc::now())
            .to_std()
            .map_err(|_| RuntimeTransportError::DeadlineExceeded)?;
        match tokio::time::timeout(wait, receiver).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(RuntimeTransportError::Unavailable),
            Err(_) => {
                self.pending.lock().await.remove(&operation_id);
                Err(RuntimeTransportError::DeadlineExceeded)
            }
        }
    }

    async fn route_response(&self, response: RuntimeResponseEnvelope) -> bool {
        let Some(pending) = self.pending.lock().await.remove(&response.operation_id) else {
            return false;
        };
        if pending.session_sequence != response.session_sequence {
            return false;
        }
        pending.sender.send(response.response).is_ok()
    }

    async fn route_renewal_confirmation(
        &self,
        confirmation: RuntimeCapabilityRenewalConfirmed,
    ) -> bool {
        if confirmation.connection_id != self.connection_id {
            return false;
        }
        let Some(pending) = self.pending_renewal.lock().await.take() else {
            return false;
        };
        if pending.renewal_id != confirmation.renewal_id {
            return false;
        }
        pending
            .sender
            .send(confirmation.persisted_capability)
            .is_ok()
    }

    async fn renew_reconnect_capability(
        &self,
        capability_key: &RuntimeBridgeCapabilityKey,
    ) -> Result<DateTime<Utc>, RuntimeBridgeError> {
        if self.closed.is_cancelled() {
            return Err(RuntimeBridgeError::Transport(
                "Runtime Agent connection is closed".into(),
            ));
        }
        let now = Utc::now();
        let capability = capability_key
            .issue(&self.identity, now)
            .map_err(RuntimeBridgeError::Capability)?;
        let grant = capability_key
            .verify(&capability, &self.identity, now)
            .map_err(RuntimeBridgeError::Capability)?;
        let renewal_id = Uuid::new_v4();
        let capability =
            RuntimeBridgeCapability::new(capability).map_err(RuntimeBridgeError::Protocol)?;
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = self.pending_renewal.lock().await;
            if pending.is_some() {
                return Err(RuntimeBridgeError::Protocol(
                    "Runtime Agent already has a pending capability renewal",
                ));
            }
            *pending = Some(PendingRenewal { renewal_id, sender });
        }
        if self
            .outbound
            .send(RuntimePlaneToAgent::RenewCapability(
                RuntimeCapabilityRenewal {
                    connection_id: self.connection_id,
                    renewal_id,
                    renewed_capability: capability,
                    expires_at: grant.expires_at,
                },
            ))
            .await
            .is_err()
        {
            self.pending_renewal.lock().await.take();
            return Err(RuntimeBridgeError::Transport(
                "Runtime Agent outbound channel is closed".into(),
            ));
        }
        match tokio::time::timeout(CAPABILITY_RENEWAL_TIMEOUT, receiver).await {
            Ok(Ok(true)) => Ok(grant.expires_at),
            Ok(Ok(false)) => Err(RuntimeBridgeError::Protocol(
                "Runtime Agent did not persist the renewed capability",
            )),
            Ok(Err(_)) => Err(RuntimeBridgeError::Transport(
                "Runtime Agent capability renewal channel closed".into(),
            )),
            Err(_) => {
                let mut pending = self.pending_renewal.lock().await;
                if pending
                    .as_ref()
                    .is_some_and(|pending| pending.renewal_id == renewal_id)
                {
                    pending.take();
                }
                Err(RuntimeBridgeError::Protocol(
                    "Runtime Agent capability renewal timed out",
                ))
            }
        }
    }

    async fn maintain_reconnect_capability(
        &self,
        capability_key: RuntimeBridgeCapabilityKey,
        mut expires_at: DateTime<Utc>,
    ) {
        loop {
            let renew_at = expires_at - CAPABILITY_RENEWAL_LEAD;
            let wait = (renew_at - Utc::now()).to_std().unwrap_or_default();
            tokio::select! {
                _ = self.closed.cancelled() => return,
                _ = tokio::time::sleep(wait) => {}
            }
            match self.renew_reconnect_capability(&capability_key).await {
                Ok(next_expiry) => expires_at = next_expiry,
                Err(_) => {
                    // A connection without a durably acknowledged next grant
                    // cannot promise recovery after a transient disconnect.
                    self.close();
                    return;
                }
            }
        }
    }

    fn close(&self) {
        self.closed.cancel();
    }

    async fn run(
        self: &Arc<Self>,
        socket: WebSocket,
        mut outbound: mpsc::Receiver<RuntimePlaneToAgent>,
    ) -> Result<(), RuntimeBridgeError> {
        let (mut sink, mut stream) = socket.split();
        let mut expected_event_sequence = 1_u64;
        loop {
            tokio::select! {
                _ = self.closed.cancelled() => break,
                outgoing = outbound.recv() => {
                    let Some(outgoing) = outgoing else { break };
                    let encoded = encode_frame(&outgoing)?;
                    sink.send(Message::Text(encoded.into()))
                        .await
                        .map_err(|error| RuntimeBridgeError::Transport(error.to_string()))?;
                }
                incoming = stream.next() => {
                    let Some(incoming) = incoming else { break };
                    let message = incoming
                        .map_err(|error| RuntimeBridgeError::Transport(error.to_string()))?;
                    match message {
                        Message::Ping(payload) => {
                            sink.send(Message::Pong(payload))
                                .await
                                .map_err(|error| RuntimeBridgeError::Transport(error.to_string()))?;
                            continue;
                        }
                        Message::Pong(_) => continue,
                        Message::Close(_) => break,
                        message => match parse_agent_frame(message)? {
                            RuntimeAgentToPlane::Response(response) => {
                                if !self.route_response(response).await {
                                    return Err(RuntimeBridgeError::Protocol(
                                        "Runtime Agent response did not match one pending request",
                                    ));
                                }
                            }
                            RuntimeAgentToPlane::Event(event) => {
                                if event.event_sequence != expected_event_sequence {
                                    return Err(RuntimeBridgeError::Protocol(
                                        "Runtime Agent event sequence is not monotonic",
                                    ));
                                }
                                expected_event_sequence = expected_event_sequence
                                    .checked_add(1)
                                    .ok_or(RuntimeBridgeError::Protocol(
                                        "Runtime Agent event sequence overflowed",
                                    ))?;
                                let _ = self.events.send(event);
                            }
                            RuntimeAgentToPlane::CapabilityRenewed(confirmation) => {
                                if !self.route_renewal_confirmation(confirmation).await {
                                    return Err(RuntimeBridgeError::Protocol(
                                        "Runtime Agent capability renewal did not match one pending rotation",
                                    ));
                                }
                            }
                            RuntimeAgentToPlane::Register(_)
                            | RuntimeAgentToPlane::RegistrationConfirmed(_) => {
                                return Err(RuntimeBridgeError::Protocol(
                                    "Runtime Agent repeated the registration handshake",
                                ));
                            }
                        },
                    }
                }
            }
        }
        self.closed.cancel();
        self.pending.lock().await.clear();
        self.pending_renewal.lock().await.take();
        Ok(())
    }
}

#[derive(Debug)]
pub enum RuntimeBridgeError {
    Protocol(&'static str),
    Capability(HostedRuntimeError),
    IdentityMismatch,
    ReplayedCapability,
    Authority(String),
    Transport(String),
}

impl RuntimeBridgeError {
    fn protocol_code(&self) -> RuntimeProtocolErrorCode {
        match self {
            Self::Capability(HostedRuntimeError::CapabilityExpired) => {
                RuntimeProtocolErrorCode::Expired
            }
            Self::Capability(_) | Self::ReplayedCapability => {
                RuntimeProtocolErrorCode::InvalidCapability
            }
            Self::IdentityMismatch => RuntimeProtocolErrorCode::InvalidIdentity,
            Self::Protocol(_) => RuntimeProtocolErrorCode::InvalidProtocol,
            Self::Authority(_) | Self::Transport(_) => RuntimeProtocolErrorCode::Internal,
        }
    }

    fn retryable(&self) -> bool {
        matches!(self, Self::Authority(_) | Self::Transport(_))
    }
}

impl fmt::Display for RuntimeBridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(reason) => formatter.write_str(reason),
            Self::Capability(error) => {
                write!(formatter, "Runtime Agent capability failed: {error}")
            }
            Self::IdentityMismatch => {
                formatter.write_str("Runtime Agent is not the current durable generation")
            }
            Self::ReplayedCapability => {
                formatter.write_str("Runtime Agent capability was already consumed")
            }
            Self::Authority(reason) => {
                write!(formatter, "Runtime Bridge authority failed: {reason}")
            }
            Self::Transport(reason) => {
                write!(formatter, "Runtime Agent transport failed: {reason}")
            }
        }
    }
}

impl std::error::Error for RuntimeBridgeError {}

fn parse_registration(message: Message) -> Result<RuntimeRegistration, RuntimeBridgeError> {
    match parse_agent_frame(message)? {
        RuntimeAgentToPlane::Register(registration) => Ok(registration),
        _ => Err(RuntimeBridgeError::Protocol(
            "first Runtime Agent frame must be registration",
        )),
    }
}

fn parse_confirmation(
    message: Message,
) -> Result<RuntimeRegistrationConfirmed, RuntimeBridgeError> {
    match parse_agent_frame(message)? {
        RuntimeAgentToPlane::RegistrationConfirmed(confirmation) => Ok(confirmation),
        _ => Err(RuntimeBridgeError::Protocol(
            "second Runtime Agent frame must confirm capability persistence",
        )),
    }
}

fn parse_agent_frame(message: Message) -> Result<RuntimeAgentToPlane, RuntimeBridgeError> {
    let Message::Text(text) = message else {
        return Err(RuntimeBridgeError::Protocol(
            "Runtime Agent protocol accepts JSON text frames only",
        ));
    };
    if text.len() > RUNTIME_AGENT_MAX_FRAME_BYTES {
        return Err(RuntimeBridgeError::Protocol(
            "Runtime Agent frame exceeds the protocol limit",
        ));
    }
    serde_json::from_str(text.as_str())
        .map_err(|_| RuntimeBridgeError::Protocol("Runtime Agent frame is not valid v1 JSON"))
}

fn encode_frame(frame: &RuntimePlaneToAgent) -> Result<String, RuntimeBridgeError> {
    let encoded = serde_json::to_string(frame)
        .map_err(|_| RuntimeBridgeError::Protocol("Runtime Bridge frame could not be encoded"))?;
    if encoded.len() > RUNTIME_AGENT_MAX_FRAME_BYTES {
        return Err(RuntimeBridgeError::Protocol(
            "Runtime Bridge frame exceeds the protocol limit",
        ));
    }
    Ok(encoded)
}

async fn send_frame(
    socket: &mut WebSocket,
    frame: &RuntimePlaneToAgent,
) -> Result<(), RuntimeBridgeError> {
    socket
        .send(Message::Text(encode_frame(frame)?.into()))
        .await
        .map_err(|error| RuntimeBridgeError::Transport(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_agent_protocol::{RuntimeAgentRequest, RuntimeAgentResponse};
    use axum::{
        extract::{State, WebSocketUpgrade},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use chrono::Duration;
    use tokio_tungstenite::tungstenite::Message as ClientMessage;

    #[derive(Default)]
    struct TestAuthority {
        current: Mutex<Option<HostedRuntimeIdentity>>,
        consumed: Mutex<BTreeSet<Uuid>>,
    }

    #[async_trait]
    impl RuntimeBridgeAuthority for TestAuthority {
        async fn exact_runtime_is_current(
            &self,
            identity: &HostedRuntimeIdentity,
        ) -> anyhow::Result<bool> {
            Ok(self.current.lock().await.as_ref() == Some(identity))
        }

        async fn consume_registration_grant(
            &self,
            grant: &RuntimeBridgeGrant,
        ) -> anyhow::Result<RuntimeGrantConsumption> {
            if self.current.lock().await.as_ref() != Some(&grant.identity) {
                return Ok(RuntimeGrantConsumption::IdentityMismatch);
            }
            Ok(if self.consumed.lock().await.insert(grant.jti) {
                RuntimeGrantConsumption::Accepted
            } else {
                RuntimeGrantConsumption::Replayed
            })
        }
    }

    fn identity() -> HostedRuntimeIdentity {
        let owner_id = Uuid::from_u128(1);
        let plane_id = Uuid::from_u128(2);
        let company_id = Uuid::from_u128(3);
        let cell_id = Uuid::from_u128(4);
        let runtime_id = format!("restless-cell-{cell_id}");
        HostedRuntimeIdentity {
            owner_id,
            plane_id,
            company_id,
            cell_id,
            volume_name: format!("{runtime_id}-data"),
            runtime_id,
            runtime_generation: 1,
            runtime_image: format!("ghcr.io/restless/runtime@sha256:{}", "a".repeat(64)),
            source_revision: "b".repeat(40),
        }
    }

    async fn registry() -> (RuntimeBridgeRegistry, RuntimeBridgeCapabilityKey) {
        let identity = identity();
        let authority = Arc::new(TestAuthority::default());
        *authority.current.lock().await = Some(identity);
        let key = RuntimeBridgeCapabilityKey::from_bytes([7; 32]);
        (RuntimeBridgeRegistry::new(key.clone(), authority), key)
    }

    fn registration(key: &RuntimeBridgeCapabilityKey) -> RuntimeRegistration {
        let identity = identity();
        RuntimeRegistration {
            protocol: RUNTIME_AGENT_PROTOCOL.into(),
            capability: RuntimeBridgeCapability::new(key.issue(&identity, Utc::now()).unwrap())
                .unwrap(),
            identity,
            desired_revision: 1,
            features: RuntimeAgentFeature::ALL.into(),
        }
    }

    async fn attached_registry() -> (
        RuntimeBridgeRegistry,
        Arc<RuntimeBridgeConnection>,
        mpsc::Receiver<RuntimePlaneToAgent>,
    ) {
        let (registry, _) = registry().await;
        let (connection, outbound) = RuntimeBridgeConnection::new(Uuid::new_v4(), identity(), 1);
        registry.attach_for_test(Arc::clone(&connection)).await;
        (registry, connection, outbound)
    }

    async fn respond(
        connection: &RuntimeBridgeConnection,
        envelope: RuntimeRequestEnvelope,
        response: RuntimeAgentResponse,
    ) {
        assert!(
            connection
                .route_response(RuntimeResponseEnvelope {
                    operation_id: envelope.operation_id,
                    session_sequence: envelope.session_sequence,
                    response,
                })
                .await
        );
    }

    #[tokio::test]
    async fn exact_current_registration_is_one_use() {
        let (registry, key) = registry().await;
        let registration = registration(&key);
        registry
            .validate_registration(&registration, Utc::now())
            .await
            .unwrap();
        assert!(matches!(
            registry
                .validate_registration(&registration, Utc::now())
                .await,
            Err(RuntimeBridgeError::ReplayedCapability)
        ));
    }

    #[tokio::test]
    async fn request_response_requires_exact_operation_and_sequence() {
        let (connection, mut outbound) =
            RuntimeBridgeConnection::new(Uuid::new_v4(), identity(), 1);
        let operation_id = Uuid::new_v4();
        let request_connection = Arc::clone(&connection);
        let request = tokio::spawn(async move {
            request_connection
                .request(
                    operation_id,
                    Utc::now() + Duration::seconds(5),
                    RuntimeAgentRequest::Activity,
                )
                .await
        });
        let RuntimePlaneToAgent::Request(envelope) = outbound.recv().await.unwrap() else {
            panic!("expected request envelope");
        };
        assert_eq!(envelope.operation_id, operation_id);
        assert_eq!(envelope.session_sequence, 1);
        assert!(
            connection
                .route_response(RuntimeResponseEnvelope {
                    operation_id,
                    session_sequence: 1,
                    response: RuntimeAgentResponse::Error(
                        crate::runtime_agent_protocol::RuntimeProtocolError {
                            code: RuntimeProtocolErrorCode::ResourceNotFound,
                            message: "test refusal".into(),
                            retryable: false,
                        }
                    ),
                })
                .await
        );
        assert!(matches!(
            request.await.unwrap().unwrap(),
            RuntimeAgentResponse::Error(_)
        ));

        assert!(
            !connection
                .route_response(RuntimeResponseEnvelope {
                    operation_id: Uuid::new_v4(),
                    session_sequence: 2,
                    response: RuntimeAgentResponse::Activity(
                        crate::runtime_agent_protocol::RuntimeActivity {
                            observed_at: Utc::now(),
                            processes: Vec::new(),
                            service_streams: Vec::new(),
                            accepts_new_sessions: true,
                        }
                    ),
                })
                .await
        );
    }

    #[tokio::test]
    async fn reconnect_capability_is_persisted_before_acceptance_and_unconsumed_until_reconnect() {
        let (registry, key) = registry().await;
        let (connection, mut outbound) =
            RuntimeBridgeConnection::new(Uuid::new_v4(), identity(), 1);
        let renewing = Arc::clone(&connection);
        let renewal_key = key.clone();
        let renewal =
            tokio::spawn(async move { renewing.renew_reconnect_capability(&renewal_key).await });

        let RuntimePlaneToAgent::RenewCapability(request) = outbound.recv().await.unwrap() else {
            panic!("expected capability renewal");
        };
        assert_eq!(request.connection_id, connection.connection_id);
        assert!(
            connection
                .route_renewal_confirmation(RuntimeCapabilityRenewalConfirmed {
                    connection_id: request.connection_id,
                    renewal_id: request.renewal_id,
                    persisted_capability: true,
                })
                .await
        );
        assert_eq!(renewal.await.unwrap().unwrap(), request.expires_at);

        let renewed_registration = RuntimeRegistration {
            protocol: RUNTIME_AGENT_PROTOCOL.into(),
            capability: request.renewed_capability,
            identity: identity(),
            desired_revision: 1,
            features: RuntimeAgentFeature::ALL.into(),
        };
        registry
            .validate_registration(&renewed_registration, Utc::now())
            .await
            .expect("a renewal is not consumed until it is used to reconnect");
        assert!(matches!(
            registry
                .validate_registration(&renewed_registration, Utc::now())
                .await,
            Err(RuntimeBridgeError::ReplayedCapability)
        ));
    }

    #[tokio::test]
    async fn newer_connection_atomically_replaces_the_old_one() {
        let (registry, _) = registry().await;
        let (old, _) = RuntimeBridgeConnection::new(Uuid::new_v4(), identity(), 1);
        registry.attach_for_test(Arc::clone(&old)).await;
        let (new, _) = RuntimeBridgeConnection::new(Uuid::new_v4(), identity(), 2);
        registry.attach_for_test(Arc::clone(&new)).await;
        assert!(old.closed.is_cancelled());
        assert_eq!(
            registry
                .connection(&identity().core_company_name())
                .await
                .unwrap()
                .connection_id,
            new.connection_id
        );
    }

    #[tokio::test]
    async fn superseded_durable_identity_immediately_revokes_an_attached_connection() {
        let authority = Arc::new(TestAuthority::default());
        *authority.current.lock().await = Some(identity());
        let registry = RuntimeBridgeRegistry::new(
            RuntimeBridgeCapabilityKey::from_bytes([9; 32]),
            authority.clone(),
        );
        let (connection, _) = RuntimeBridgeConnection::new(Uuid::new_v4(), identity(), 1);
        registry.attach_for_test(Arc::clone(&connection)).await;
        assert!(registry
            .connection(&identity().core_company_name())
            .await
            .is_ok());

        *authority.current.lock().await = None;
        assert!(matches!(
            registry.connection(&identity().core_company_name()).await,
            Err(RuntimeTransportError::Unavailable)
        ));
        assert!(connection.closed.is_cancelled());
    }

    #[tokio::test]
    async fn transport_process_is_full_duplex_and_preserves_attempt_authority() {
        let (registry, connection, mut outbound) = attached_registry().await;
        let company = identity().core_company_name();
        let work_id = Uuid::new_v4();
        let attempt_id = Uuid::new_v4();
        let operation_id = Uuid::new_v4();
        let agent_connection = Arc::clone(&connection);
        let agent = tokio::spawn(async move {
            while let Some(RuntimePlaneToAgent::Request(envelope)) = outbound.recv().await {
                match &envelope.request {
                    RuntimeAgentRequest::ProcessStart(request) => {
                        assert_eq!(request.process_id, operation_id);
                        assert!(matches!(
                            &request.authority,
                            crate::runtime_agent_protocol::ProcessAuthority::Attempt {
                                work_id: observed_work,
                                attempt_id: observed_attempt,
                                ..
                            } if *observed_work == work_id && *observed_attempt == attempt_id
                        ));
                        respond(
                            &agent_connection,
                            envelope.clone(),
                            RuntimeAgentResponse::ProcessStarted(
                                crate::runtime_agent_protocol::ProcessStarted {
                                    process_id: operation_id,
                                    pid: 42,
                                    started_at: Utc::now(),
                                },
                            ),
                        )
                        .await;
                        for (sequence, stream, bytes, eof) in [
                            (
                                1,
                                crate::runtime_agent_protocol::ProcessStream::Stdout,
                                b"hosted output".as_slice(),
                                true,
                            ),
                            (
                                2,
                                crate::runtime_agent_protocol::ProcessStream::Stderr,
                                b"".as_slice(),
                                true,
                            ),
                        ] {
                            let _ = agent_connection.events.send(RuntimeEventEnvelope {
                                operation_id: Some(operation_id),
                                event_sequence: sequence,
                                event: RuntimeAgentEvent::ProcessOutput(
                                    crate::runtime_agent_protocol::ProcessOutput {
                                        process_id: operation_id,
                                        stream,
                                        data_base64: BASE64.encode(bytes),
                                        eof,
                                    },
                                ),
                            });
                        }
                        let _ = agent_connection.events.send(RuntimeEventEnvelope {
                            operation_id: Some(operation_id),
                            event_sequence: 3,
                            event: RuntimeAgentEvent::ProcessExited(
                                crate::runtime_agent_protocol::ProcessExited {
                                    process_id: operation_id,
                                    exit_code: Some(0),
                                    signal: None,
                                    finished_at: Utc::now(),
                                },
                            ),
                        });
                    }
                    RuntimeAgentRequest::ProcessStdin(request) => {
                        let decoded = BASE64.decode(&request.data_base64).unwrap();
                        respond(
                            &agent_connection,
                            envelope.clone(),
                            RuntimeAgentResponse::ProcessInputAccepted(
                                crate::runtime_agent_protocol::ProcessInputAccepted {
                                    process_id: request.process_id,
                                    decoded_bytes: decoded.len() as u32,
                                    eof: request.eof,
                                },
                            ),
                        )
                        .await;
                    }
                    unexpected => panic!("unexpected Runtime request: {unexpected:?}"),
                }
            }
        });

        let mut process = registry
            .start_process(RuntimeProcessSpec {
                operation_id,
                authority: crate::runtime_transport::RuntimeProcessAuthority::Attempt {
                    company,
                    actor: "exec".into(),
                    responsibility: "portfolio".into(),
                    work_id,
                    attempt_id,
                    session_id: "session-1".into(),
                },
                executable: "omp".into(),
                arguments: vec!["acp".into()],
                working_directory: CompanyPath::parse("/company/home").unwrap(),
                environment: Vec::new(),
                deadline: Utc::now() + Duration::seconds(5),
            })
            .await
            .unwrap();
        process.stdin.write_all(b"owner input").await.unwrap();
        process.stdin.shutdown().await.unwrap();
        let mut output = String::new();
        process.stdout.read_to_string(&mut output).await.unwrap();
        assert_eq!(output, "hosted output");
        assert_eq!(process.control.wait().await.unwrap().code, Some(0));
        connection.close();
        agent.abort();
    }

    #[tokio::test]
    async fn transport_chunks_large_atomic_writes_and_verifies_the_commit_digest() {
        let (registry, connection, mut outbound) = attached_registry().await;
        let company = identity().core_company_name();
        let operation_id = Uuid::new_v4();
        let path = CompanyPath::parse("/company/projects/large.bin").unwrap();
        let wire_path: crate::runtime_agent_protocol::RuntimePath = (&path).try_into().unwrap();
        let contents = vec![0x5a; RUNTIME_AGENT_MAX_CHUNK_BYTES + 17];
        let expected = contents.clone();
        let expected_digest = sha256_hex(&expected);
        let agent_connection = Arc::clone(&connection);
        let agent = tokio::spawn(async move {
            let mut received = Vec::new();
            while let Some(RuntimePlaneToAgent::Request(envelope)) = outbound.recv().await {
                let response = match &envelope.request {
                    RuntimeAgentRequest::File(FileRequest::UploadBegin {
                        write_id,
                        path,
                        exact_size,
                        exact_sha256,
                        mode,
                        ..
                    }) => {
                        assert_eq!(*write_id, operation_id);
                        assert_eq!(path, &wire_path);
                        assert_eq!(*exact_size, expected.len() as u64);
                        assert_eq!(exact_sha256, &expected_digest);
                        RuntimeAgentResponse::File(FileResponse::UploadBegun(
                            crate::runtime_agent_protocol::RuntimeFileUpload {
                                write_id: *write_id,
                                path: wire_path.clone(),
                                exact_size: *exact_size,
                                exact_sha256: exact_sha256.clone(),
                                mode: *mode,
                                next_offset: 0,
                            },
                        ))
                    }
                    RuntimeAgentRequest::File(FileRequest::UploadChunk {
                        write_id,
                        offset,
                        data_base64,
                    }) => {
                        assert_eq!(*write_id, operation_id);
                        assert_eq!(*offset, received.len() as u64);
                        let chunk = BASE64.decode(data_base64).unwrap();
                        received.extend_from_slice(&chunk);
                        RuntimeAgentResponse::File(FileResponse::UploadChunkAccepted(
                            crate::runtime_agent_protocol::RuntimeFileUploadProgress {
                                write_id: *write_id,
                                accepted_bytes: chunk.len() as u32,
                                next_offset: received.len() as u64,
                            },
                        ))
                    }
                    RuntimeAgentRequest::File(FileRequest::UploadCommit { write_id }) => {
                        assert_eq!(*write_id, operation_id);
                        assert_eq!(received, expected);
                        RuntimeAgentResponse::File(FileResponse::Written(
                            crate::runtime_agent_protocol::RuntimeFileMutation {
                                path: wire_path.clone(),
                                size: received.len() as u64,
                                sha256: expected_digest.clone(),
                            },
                        ))
                    }
                    unexpected => panic!("unexpected Runtime request: {unexpected:?}"),
                };
                respond(&agent_connection, envelope, response).await;
            }
        });
        registry
            .atomic_write(&company, operation_id, &path, &contents, 0o640)
            .await
            .unwrap();
        connection.close();
        agent.abort();
    }

    #[tokio::test]
    async fn websocket_handshake_gates_requests_until_rotated_capability_is_persisted() {
        async fn bridge(
            State(registry): State<RuntimeBridgeRegistry>,
            upgrade: WebSocketUpgrade,
        ) -> impl IntoResponse {
            upgrade.on_upgrade(move |socket| async move {
                registry.accept_socket(socket).await.unwrap();
            })
        }

        let (registry, key) = registry().await;
        let app = Router::new()
            .route("/internal/v1/runtime-bridge", get(bridge))
            .with_state(registry.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let (mut agent, _) =
            tokio_tungstenite::connect_async(format!("ws://{address}/internal/v1/runtime-bridge"))
                .await
                .unwrap();

        agent
            .send(ClientMessage::Text(
                serde_json::to_string(&RuntimeAgentToPlane::Register(registration(&key)))
                    .unwrap()
                    .into(),
            ))
            .await
            .unwrap();
        let accepted = agent.next().await.unwrap().unwrap().into_text().unwrap();
        let RuntimePlaneToAgent::Registered(accepted) =
            serde_json::from_str::<RuntimePlaneToAgent>(&accepted).unwrap()
        else {
            panic!("expected registration acceptance");
        };
        assert!(accepted.renewed_capability.is_some());
        assert!(
            registry
                .connection(&identity().core_company_name())
                .await
                .is_err(),
            "connection became usable before persistence confirmation"
        );

        agent
            .send(ClientMessage::Text(
                serde_json::to_string(&RuntimeAgentToPlane::RegistrationConfirmed(
                    RuntimeRegistrationConfirmed {
                        connection_id: accepted.connection_id,
                        persisted_capability: true,
                    },
                ))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
        let connection = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if let Ok(connection) = registry.connection(&identity().core_company_name()).await {
                    break connection;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        let heartbeat = vec![1, 2, 3, 4];
        agent
            .send(ClientMessage::Ping(heartbeat.clone().into()))
            .await
            .unwrap();
        let pong = tokio::time::timeout(std::time::Duration::from_secs(2), agent.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(pong, ClientMessage::Pong(heartbeat.into()));
        assert!(registry
            .connection(&identity().core_company_name())
            .await
            .is_ok());

        let operation_id = Uuid::new_v4();
        let requester = Arc::clone(&connection);
        let pending = tokio::spawn(async move {
            requester
                .request(
                    operation_id,
                    Utc::now() + Duration::seconds(5),
                    RuntimeAgentRequest::Activity,
                )
                .await
        });
        let request = agent.next().await.unwrap().unwrap().into_text().unwrap();
        let RuntimePlaneToAgent::Request(request) =
            serde_json::from_str::<RuntimePlaneToAgent>(&request).unwrap()
        else {
            panic!("expected Runtime request");
        };
        agent
            .send(ClientMessage::Text(
                serde_json::to_string(&RuntimeAgentToPlane::Response(RuntimeResponseEnvelope {
                    operation_id: request.operation_id,
                    session_sequence: request.session_sequence,
                    response: RuntimeAgentResponse::Activity(
                        crate::runtime_agent_protocol::RuntimeActivity {
                            observed_at: Utc::now(),
                            processes: Vec::new(),
                            service_streams: Vec::new(),
                            accepts_new_sessions: true,
                        },
                    ),
                }))
                .unwrap()
                .into(),
            ))
            .await
            .unwrap();
        assert!(matches!(
            pending.await.unwrap().unwrap(),
            RuntimeAgentResponse::Activity(_)
        ));
        agent.close(None).await.unwrap();
        server.abort();
    }
}
