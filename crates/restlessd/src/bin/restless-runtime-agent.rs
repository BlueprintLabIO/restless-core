//! Outbound-only transport loop for the cell-local Runtime Agent.
//!
//! The account plane never dials a company Runtime. This process establishes
//! one WSS connection to the exact released bridge endpoint, completes the
//! one-use capability rotation handshake, and multiplexes bounded request,
//! response, and event frames over that connection.

use std::time::Duration;

use chrono::Utc;
use futures_util::{SinkExt as _, StreamExt as _};
use restlessd::runtime_agent::{
    enter_runtime_agent_security_context, run_company_security_probe, run_file_worker_stdio,
    run_process_worker, run_runtime_agent_security_self_test,
    verify_runtime_agent_security_context, RuntimeAgent, RuntimeAgentConfig, RuntimeAgentError,
    RuntimeCapabilityStore, RuntimeRequestSequence,
};
use restlessd::runtime_agent_protocol::{
    RuntimeAgentToPlane, RuntimeCapabilityRenewalConfirmed, RuntimeEventEnvelope,
    RuntimePlaneToAgent, RuntimeRegistrationConfirmed, RUNTIME_AGENT_MAX_FRAME_BYTES,
    RUNTIME_AGENT_PROTOCOL,
};
use tokio::sync::mpsc;
use tokio::time::{Instant, MissedTickBehavior};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);
const INBOUND_LIVENESS_TIMEOUT: Duration = Duration::from_secs(75);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(30);

fn main() {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("--file-worker") if arguments.next().is_none() => {
            if run_file_worker_stdio().is_err() {
                std::process::exit(70);
            }
            return;
        }
        Some("--process-worker") => {
            if run_process_worker(arguments.collect()).is_err() {
                std::process::exit(70);
            }
            return;
        }
        Some("--security-probe-worker") if arguments.next().is_none() => {
            if run_company_security_probe().is_err() {
                std::process::exit(70);
            }
            return;
        }
        Some("--verify-security-boundary") if arguments.next().is_none() => {
            if enter_runtime_agent_security_context().is_err()
                || run_runtime_agent_security_self_test().is_err()
            {
                std::process::exit(70);
            }
            return;
        }
        Some(_) => std::process::exit(64),
        None => {}
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    // Linux credentials are per-thread kernel state: transition before Tokio
    // constructs any worker threads.
    if let Err(error) = enter_runtime_agent_security_context() {
        tracing::error!(reason = %error, "Runtime Agent privilege transition failed");
        std::process::exit(1);
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => std::process::exit(1),
    };
    if let Err(error) = runtime.block_on(run()) {
        // RuntimeAgentError never displays capability material. Do not attach
        // frame contents or nested transport errors to this process log.
        tracing::error!(reason = %error, "Runtime Agent stopped before connecting");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), RuntimeAgentError> {
    verify_runtime_agent_security_context()?;
    let config = RuntimeAgentConfig::from_environment()?;
    let capability_store = RuntimeCapabilityStore::new(
        config.capability_file.clone(),
        config.capability_state_file.clone(),
    );
    let cell_id = config.identity.cell_id;
    let (agent, mut events) = RuntimeAgent::new(config.clone())?;
    let mut consecutive_failures = 0_u32;

    loop {
        let candidates = match capability_store.candidates() {
            Ok(candidates) => candidates,
            Err(_) => {
                consecutive_failures = consecutive_failures.saturating_add(1);
                let delay = reconnect_delay(consecutive_failures, cell_id);
                tracing::warn!(
                    delay_seconds = delay.as_secs(),
                    "Runtime Bridge capability is not safely readable; awaiting reprovisioning"
                );
                tokio::time::sleep(delay).await;
                continue;
            }
        };

        let mut completed_session = false;
        for capability in candidates {
            match connect_once(&config, &agent, &capability_store, capability, &mut events).await {
                Ok(ConnectionOutcome::SessionEnded) => {
                    completed_session = true;
                    break;
                }
                Ok(ConnectionOutcome::Rejected { retryable: false }) => {
                    // A persisted rotation can expire or be superseded while a
                    // fresh bootstrap grant is staged. Try the next locally
                    // protected candidate without ever disclosing either.
                    continue;
                }
                Ok(ConnectionOutcome::Rejected { retryable: true }) | Err(_) => break,
            }
        }

        consecutive_failures = if completed_session {
            0
        } else {
            consecutive_failures.saturating_add(1)
        };
        let delay = reconnect_delay(consecutive_failures.saturating_add(1), cell_id);
        tracing::warn!(
            delay_seconds = delay.as_secs(),
            had_authenticated_session = completed_session,
            "Runtime Agent connection ended; reconnecting"
        );
        tokio::time::sleep(delay).await;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionOutcome {
    SessionEnded,
    Rejected { retryable: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionError {
    Connect,
    Transport,
    Protocol,
    CapabilityPersistence,
}

async fn connect_once(
    config: &RuntimeAgentConfig,
    agent: &RuntimeAgent,
    capability_store: &RuntimeCapabilityStore,
    capability: restlessd::runtime_agent_protocol::RuntimeBridgeCapability,
    events: &mut mpsc::Receiver<restlessd::runtime_agent_protocol::RuntimeAgentEvent>,
) -> Result<ConnectionOutcome, ConnectionError> {
    let socket_config = WebSocketConfig::default()
        .read_buffer_size(32 * 1024)
        .write_buffer_size(32 * 1024)
        .max_write_buffer_size(RUNTIME_AGENT_MAX_FRAME_BYTES * 2)
        .max_message_size(Some(RUNTIME_AGENT_MAX_FRAME_BYTES))
        .max_frame_size(Some(RUNTIME_AGENT_MAX_FRAME_BYTES));
    let (mut socket, _) = tokio::time::timeout(
        HANDSHAKE_TIMEOUT,
        tokio_tungstenite::connect_async_with_config(
            config.bridge_url.as_str(),
            Some(socket_config),
            true,
        ),
    )
    .await
    .map_err(|_| ConnectionError::Connect)?
    .map_err(|_| ConnectionError::Connect)?;

    send_agent_frame(
        &mut socket,
        &RuntimeAgentToPlane::Register(agent.registration(capability)),
    )
    .await?;

    let decision = tokio::time::timeout(HANDSHAKE_TIMEOUT, receive_plane_frame(&mut socket))
        .await
        .map_err(|_| ConnectionError::Protocol)??;
    let accepted = match decision {
        RuntimePlaneToAgent::Registered(accepted) => accepted,
        RuntimePlaneToAgent::Rejected(rejection) => {
            return Ok(ConnectionOutcome::Rejected {
                retryable: rejection.retryable,
            });
        }
        RuntimePlaneToAgent::Request(_) | RuntimePlaneToAgent::RenewCapability(_) => {
            return Err(ConnectionError::Protocol);
        }
    };
    if accepted.protocol != RUNTIME_AGENT_PROTOCOL
        || accepted.connection_id.is_nil()
        || accepted.next_session_sequence == 0
    {
        return Err(ConnectionError::Protocol);
    }
    let (renewed, expires_at) = accepted
        .renewed_capability
        .zip(accepted.renewed_capability_expires_at)
        .ok_or(ConnectionError::Protocol)?;
    if expires_at <= Utc::now() {
        return Err(ConnectionError::Protocol);
    }

    capability_store
        .persist_rotation(&renewed)
        .map_err(|_| ConnectionError::CapabilityPersistence)?;
    capability_store
        .discard_bootstrap()
        .map_err(|_| ConnectionError::CapabilityPersistence)?;
    send_agent_frame(
        &mut socket,
        &RuntimeAgentToPlane::RegistrationConfirmed(RuntimeRegistrationConfirmed {
            connection_id: accepted.connection_id,
            persisted_capability: true,
        }),
    )
    .await?;

    tracing::info!(
        connection_id = %accepted.connection_id,
        "Runtime Agent authenticated its outbound bridge"
    );
    serve_connection(
        socket,
        agent,
        events,
        capability_store,
        accepted.connection_id,
        accepted.next_session_sequence,
    )
    .await?;
    Ok(ConnectionOutcome::SessionEnded)
}

async fn serve_connection<S>(
    mut socket: tokio_tungstenite::WebSocketStream<S>,
    agent: &RuntimeAgent,
    events: &mut mpsc::Receiver<restlessd::runtime_agent_protocol::RuntimeAgentEvent>,
    capability_store: &RuntimeCapabilityStore,
    connection_id: Uuid,
    next_session_sequence: u64,
) -> Result<(), ConnectionError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut requests = RuntimeRequestSequence::new(next_session_sequence)
        .map_err(|_| ConnectionError::Protocol)?;
    let mut event_sequence = 1_u64;
    let mut last_inbound = Instant::now();
    let mut heartbeat =
        tokio::time::interval_at(Instant::now() + HEARTBEAT_INTERVAL, HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            incoming = socket.next() => {
                let Some(incoming) = incoming else {
                    return Ok(());
                };
                let incoming = incoming.map_err(|_| ConnectionError::Transport)?;
                last_inbound = Instant::now();
                match incoming {
                    Message::Text(text) => {
                        let frame = parse_plane_text(text.as_str())?;
                        match frame {
                            RuntimePlaneToAgent::Request(request) => {
                                let response = agent.handle_request(&mut requests, request, Utc::now()).await;
                                send_agent_frame(&mut socket, &RuntimeAgentToPlane::Response(response)).await?;
                            }
                            RuntimePlaneToAgent::RenewCapability(renewal) => {
                                let confirmation = persist_renewal(
                                    capability_store,
                                    connection_id,
                                    renewal,
                                    Utc::now(),
                                )?;
                                send_agent_frame(
                                    &mut socket,
                                    &RuntimeAgentToPlane::CapabilityRenewed(
                                        confirmation,
                                    ),
                                )
                                .await?;
                            }
                            RuntimePlaneToAgent::Registered(_) | RuntimePlaneToAgent::Rejected(_) => {
                                return Err(ConnectionError::Protocol);
                            }
                        }
                    }
                    Message::Ping(value) => {
                        socket.send(Message::Pong(value)).await.map_err(|_| ConnectionError::Transport)?;
                    }
                    Message::Pong(_) => {}
                    Message::Close(_) => return Ok(()),
                    Message::Binary(_) | Message::Frame(_) => return Err(ConnectionError::Protocol),
                }
            }
            event = events.recv() => {
                let event = event.ok_or(ConnectionError::Transport)?;
                let envelope = RuntimeEventEnvelope {
                    operation_id: None,
                    event_sequence,
                    event,
                };
                event_sequence = event_sequence.checked_add(1).ok_or(ConnectionError::Protocol)?;
                send_agent_frame(&mut socket, &RuntimeAgentToPlane::Event(envelope)).await?;
            }
            _ = heartbeat.tick() => {
                if last_inbound.elapsed() > INBOUND_LIVENESS_TIMEOUT {
                    return Err(ConnectionError::Transport);
                }
                socket
                    .send(Message::Ping(event_sequence.to_be_bytes().to_vec().into()))
                    .await
                    .map_err(|_| ConnectionError::Transport)?;
            }
        }
    }
}

fn persist_renewal(
    capability_store: &RuntimeCapabilityStore,
    connection_id: Uuid,
    renewal: restlessd::runtime_agent_protocol::RuntimeCapabilityRenewal,
    now: chrono::DateTime<Utc>,
) -> Result<RuntimeCapabilityRenewalConfirmed, ConnectionError> {
    if renewal.connection_id != connection_id
        || renewal.renewal_id.is_nil()
        || renewal.expires_at <= now
        || renewal.expires_at > now + chrono::Duration::hours(1)
    {
        return Err(ConnectionError::Protocol);
    }
    capability_store
        .persist_rotation(&renewal.renewed_capability)
        .map_err(|_| ConnectionError::CapabilityPersistence)?;
    Ok(RuntimeCapabilityRenewalConfirmed {
        connection_id,
        renewal_id: renewal.renewal_id,
        persisted_capability: true,
    })
}

async fn receive_plane_frame<S>(
    socket: &mut tokio_tungstenite::WebSocketStream<S>,
) -> Result<RuntimePlaneToAgent, ConnectionError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    loop {
        let message = socket
            .next()
            .await
            .ok_or(ConnectionError::Transport)?
            .map_err(|_| ConnectionError::Transport)?;
        match message {
            Message::Text(text) => return parse_plane_text(text.as_str()),
            Message::Ping(value) => socket
                .send(Message::Pong(value))
                .await
                .map_err(|_| ConnectionError::Transport)?,
            Message::Pong(_) => {}
            Message::Close(_) => return Err(ConnectionError::Transport),
            Message::Binary(_) | Message::Frame(_) => return Err(ConnectionError::Protocol),
        }
    }
}

fn parse_plane_text(text: &str) -> Result<RuntimePlaneToAgent, ConnectionError> {
    if text.len() > RUNTIME_AGENT_MAX_FRAME_BYTES {
        return Err(ConnectionError::Protocol);
    }
    serde_json::from_str(text).map_err(|_| ConnectionError::Protocol)
}

async fn send_agent_frame<S>(
    socket: &mut tokio_tungstenite::WebSocketStream<S>,
    frame: &RuntimeAgentToPlane,
) -> Result<(), ConnectionError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let encoded = serde_json::to_string(frame).map_err(|_| ConnectionError::Protocol)?;
    if encoded.len() > RUNTIME_AGENT_MAX_FRAME_BYTES {
        return Err(ConnectionError::Protocol);
    }
    socket
        .send(Message::Text(encoded.into()))
        .await
        .map_err(|_| ConnectionError::Transport)
}

fn reconnect_delay(failures: u32, cell_id: Uuid) -> Duration {
    let exponent = failures.saturating_sub(1).min(5);
    let base_ms = 1_000_u64 << exponent;
    let jitter_ms =
        u64::from_le_bytes(cell_id.as_bytes()[..8].try_into().expect("UUID width")) % 751;
    Duration::from_millis((base_ms + jitter_ms).min(MAX_RECONNECT_DELAY.as_millis() as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use restlessd::runtime_agent_protocol::{
        RuntimeBridgeCapability, RuntimeCapabilityRenewal, RuntimeProtocolErrorCode,
        RuntimeRegistrationRejected,
    };
    use std::os::unix::fs::PermissionsExt as _;

    #[test]
    fn reconnect_backoff_is_deterministic_and_bounded() {
        let cell = Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
        let values = (1..=64)
            .map(|failure| reconnect_delay(failure, cell))
            .collect::<Vec<_>>();
        assert!(values.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(values.iter().all(|delay| *delay <= MAX_RECONNECT_DELAY));
        assert_eq!(values.last().copied(), Some(MAX_RECONNECT_DELAY));
    }

    #[test]
    fn plane_frames_are_exact_and_bounded() {
        let frame = RuntimePlaneToAgent::Rejected(RuntimeRegistrationRejected {
            code: RuntimeProtocolErrorCode::InvalidCapability,
            retryable: false,
        });
        let encoded = serde_json::to_string(&frame).unwrap();
        assert_eq!(parse_plane_text(&encoded).unwrap(), frame);
        assert_eq!(
            parse_plane_text(&"x".repeat(RUNTIME_AGENT_MAX_FRAME_BYTES + 1)),
            Err(ConnectionError::Protocol)
        );
        assert_eq!(
            parse_plane_text(
                r#"{"type":"rejected","body":{"code":"invalid_capability","retryable":false,"extra":true}}"#
            ),
            Err(ConnectionError::Protocol)
        );
    }

    #[test]
    fn renewal_is_durable_before_the_non_secret_ack() {
        let root = std::env::temp_dir().join(format!("restless-renewal-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let bootstrap = root.join("bootstrap");
        let state = root.join("state");
        let store = RuntimeCapabilityStore::new(bootstrap, state.clone());
        let connection_id = Uuid::new_v4();
        let renewal_id = Uuid::new_v4();
        let token = "n".repeat(96);
        let confirmation = persist_renewal(
            &store,
            connection_id,
            RuntimeCapabilityRenewal {
                connection_id,
                renewal_id,
                renewed_capability: RuntimeBridgeCapability::new(token.clone()).unwrap(),
                expires_at: Utc::now() + chrono::Duration::minutes(15),
            },
            Utc::now(),
        )
        .unwrap();

        assert_eq!(std::fs::read_to_string(&state).unwrap(), token);
        assert_eq!(
            std::fs::metadata(&state).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(store.candidates().unwrap()[0].expose(), token);
        assert!(confirmation.persisted_capability);
        assert_eq!(confirmation.connection_id, connection_id);
        assert_eq!(confirmation.renewal_id, renewal_id);
        assert!(!serde_json::to_string(&confirmation)
            .unwrap()
            .contains(&token));

        std::fs::remove_dir_all(root).unwrap();
    }
}
