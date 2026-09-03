//! A deliberately bounded local provider used to prove the released service
//! contract. It exposes exactly one HTTPS/WebSocket or UDP listener on a
//! loopback address. It is not a generic command runner or tunnel.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use hyper_util::rt::TokioIo;
use hyper_util::service::TowerToHyperService;
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::Mutex;
use tokio_rustls::TlsAcceptor;

use crate::published_service_contract::{
    ensure_loopback_bind, token_digest, verify_invitation, Audience, ProviderEndpoint,
    ProviderReadyReceipt, ReadinessProbe, ServiceManifest, ServiceObservations, ServiceProfile,
    CONTRACT_VERSION,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalFixtureConfig {
    pub company: String,
    pub publication_id: String,
    pub candidate_digest: String,
    pub provider_operation_id: String,
    pub profile: ServiceProfile,
    pub manifest: ServiceManifest,
    pub audience: Audience,
    pub bind_host: String,
    pub expires_at: DateTime<Utc>,
    pub invitation_key_base64: String,
    pub marker_path: PathBuf,
    pub observations_path: PathBuf,
    pub revocations_path: PathBuf,
}

impl LocalFixtureConfig {
    fn validate(&self) -> Result<()> {
        ensure_loopback_bind(&self.bind_host)?;
        self.manifest.validate()?;
        if self.profile != self.manifest.profile {
            bail!("fixture profile and service manifest profile differ");
        }
        if self.company.trim().is_empty()
            || self.publication_id.trim().is_empty()
            || self.candidate_digest.trim().is_empty()
            || self.provider_operation_id.trim().is_empty()
        {
            bail!("fixture identifiers must not be empty");
        }
        if self.expires_at <= Utc::now() {
            bail!("fixture publication is already expired");
        }
        let key = self.invitation_key()?;
        if key.len() < 32 {
            bail!("fixture invitation key must contain at least 32 bytes");
        }
        for path in [
            &self.marker_path,
            &self.observations_path,
            &self.revocations_path,
        ] {
            if !path.is_absolute() {
                bail!("fixture state paths must be absolute");
            }
        }
        Ok(())
    }

    fn invitation_key(&self) -> Result<Vec<u8>> {
        base64::engine::general_purpose::STANDARD
            .decode(&self.invitation_key_base64)
            .context("decode fixture invitation key")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalFixtureMarker {
    pub receipt: ProviderReadyReceipt,
    /// The local fixture uses a fresh self-signed certificate. This public
    /// certificate is included so integration clients can trust exactly this
    /// process without disabling TLS verification globally.
    pub tls_certificate_pem: Option<String>,
}

#[derive(Clone)]
struct FixtureState {
    config: LocalFixtureConfig,
    invitation_key: Arc<Vec<u8>>,
    observations: Arc<Mutex<ServiceObservations>>,
}

#[derive(Debug, Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UdpMessage {
    token: Option<String>,
    payload: String,
}

pub async fn run_from_config_path(path: &Path) -> Result<()> {
    let raw = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let config: LocalFixtureConfig =
        serde_json::from_slice(&raw).context("decode local published-service fixture config")?;
    // The only file containing the per-publication verification key is a
    // one-shot handoff. The child keeps it in memory after startup.
    std::fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
    config.validate()?;
    match config.profile {
        ServiceProfile::HttpsWebsocketDemo => run_https(config).await,
        ServiceProfile::GodotEnetUdp => run_udp(config).await,
    }
}

async fn run_https(config: LocalFixtureConfig) -> Result<()> {
    // The workspace currently enables both rustls crypto backends through
    // independent clients. Select one explicitly so provider choice never
    // depends on feature unification order.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let listener = TcpListener::bind(format!("{}:0", config.bind_host))
        .await
        .context("bind bounded HTTPS fixture")?;
    let address = listener
        .local_addr()
        .context("read HTTPS fixture address")?;
    let certified = generate_simple_self_signed(vec!["localhost".to_string()])
        .context("generate local fixture certificate")?;
    let certificate_pem = certified.cert.pem();
    let certificate: CertificateDer<'static> = certified.cert.der().clone();
    let private_key =
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(certified.key_pair.serialize_der()));
    let tls = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate], private_key)
        .context("construct local fixture TLS config")?;
    let acceptor = TlsAcceptor::from(Arc::new(tls));
    let state = FixtureState {
        invitation_key: Arc::new(config.invitation_key()?),
        observations: Arc::new(Mutex::new(load_observations(&config.observations_path)?)),
        config: config.clone(),
    };
    let (health_path, websocket_path) = match &config.manifest.readiness {
        ReadinessProbe::Http {
            path,
            websocket_path,
        } => (path.clone(), websocket_path.clone()),
        _ => bail!("HTTPS fixture received a non-HTTP readiness probe"),
    };
    let app = Router::new()
        .route("/", get(http_root))
        .route(&health_path, get(http_health))
        .route(&websocket_path, get(websocket_upgrade))
        .route("/observe", post(http_observe))
        .with_state(state);
    write_marker(
        &config,
        address,
        format!("https://localhost:{}", address.port()),
        "self-signed-local-tls",
        Some(certificate_pem),
    )?;

    loop {
        let (stream, _) = listener
            .accept()
            .await
            .context("accept HTTPS fixture connection")?;
        let acceptor = acceptor.clone();
        let service = TowerToHyperService::new(app.clone());
        tokio::spawn(async move {
            let Ok(stream) = acceptor.accept(stream).await else {
                return;
            };
            let io = TokioIo::new(stream);
            let _ = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .with_upgrades()
                .await;
        });
    }
}

async fn run_udp(config: LocalFixtureConfig) -> Result<()> {
    let socket = UdpSocket::bind(format!("{}:0", config.bind_host))
        .await
        .context("bind bounded UDP fixture")?;
    let address = socket.local_addr().context("read UDP fixture address")?;
    let state = FixtureState {
        invitation_key: Arc::new(config.invitation_key()?),
        observations: Arc::new(Mutex::new(load_observations(&config.observations_path)?)),
        config: config.clone(),
    };
    write_marker(
        &config,
        address,
        format!("udp://{}", address),
        "datagram",
        None,
    )?;
    let mut buffer = [0_u8; 4096];
    let (readiness_request, readiness_response) = match &config.manifest.readiness {
        ReadinessProbe::Udp { request, response } => (request.as_bytes(), response.as_bytes()),
        _ => bail!("UDP fixture received a non-UDP readiness probe"),
    };
    loop {
        let (length, peer) = socket
            .recv_from(&mut buffer)
            .await
            .context("receive UDP fixture packet")?;
        if &buffer[..length] == readiness_request {
            socket
                .send_to(readiness_response, peer)
                .await
                .context("send UDP readiness response")?;
            continue;
        }
        let decoded = serde_json::from_slice::<UdpMessage>(&buffer[..length]);
        let response = match decoded {
            Ok(message) if message.payload.len() <= 2048 => {
                match authorize_token(&state, message.token.as_deref(), Utc::now()).await {
                    Ok(()) => {
                        record_accepted(&state, true).await?;
                        json!({
                            "ok": true,
                            "publication_id": config.publication_id,
                            "candidate_digest": config.candidate_digest,
                            "payload": message.payload,
                        })
                    }
                    Err(error) => {
                        record_rejected(&state).await?;
                        json!({"ok": false, "error": error.to_string()})
                    }
                }
            }
            Ok(_) => {
                record_rejected(&state).await?;
                json!({"ok": false, "error": "payload exceeds 2048 bytes"})
            }
            Err(error) => {
                record_rejected(&state).await?;
                json!({"ok": false, "error": format!("invalid bounded UDP request: {error}")})
            }
        };
        let encoded = serde_json::to_vec(&response).context("encode UDP fixture response")?;
        socket
            .send_to(&encoded, peer)
            .await
            .context("send UDP fixture response")?;
    }
}

async fn http_health(
    State(state): State<FixtureState>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
) -> Response {
    authenticated_json(&state, &headers, query.token.as_deref(), "healthy").await
}

async fn http_root(
    State(state): State<FixtureState>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
) -> Response {
    authenticated_json(
        &state,
        &headers,
        query.token.as_deref(),
        "published-service-fixture",
    )
    .await
}

async fn http_observe(
    State(state): State<FixtureState>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
) -> Response {
    authenticated_json(&state, &headers, query.token.as_deref(), "observed").await
}

async fn authenticated_json(
    state: &FixtureState,
    headers: &HeaderMap,
    query_token: Option<&str>,
    status: &str,
) -> Response {
    let header_token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    match authorize_token(state, header_token.or(query_token), Utc::now()).await {
        Ok(()) => {
            if let Err(error) = record_accepted(state, true).await {
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, error);
            }
            Json(json!({
                "ok": true,
                "status": status,
                "publication_id": state.config.publication_id,
                "candidate_digest": state.config.candidate_digest,
            }))
            .into_response()
        }
        Err(error) => {
            let _ = record_rejected(state).await;
            error_response(StatusCode::UNAUTHORIZED, error)
        }
    }
}

async fn websocket_upgrade(
    State(state): State<FixtureState>,
    headers: HeaderMap,
    Query(query): Query<TokenQuery>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let header_token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if let Err(error) =
        authorize_token(&state, header_token.or(query.token.as_deref()), Utc::now()).await
    {
        let _ = record_rejected(&state).await;
        return error_response(StatusCode::UNAUTHORIZED, error);
    }
    if let Err(error) = record_accepted(&state, false).await {
        return error_response(StatusCode::INTERNAL_SERVER_ERROR, error);
    }
    upgrade.on_upgrade(move |socket| websocket_session(state, socket))
}

async fn websocket_session(state: FixtureState, mut socket: WebSocket) {
    while let Some(Ok(message)) = socket.next().await {
        match message {
            Message::Text(text) if text.len() <= 16 * 1024 => {
                if record_message(&state).await.is_err() {
                    break;
                }
                if socket.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            Message::Binary(bytes) if bytes.len() <= 16 * 1024 => {
                if record_message(&state).await.is_err() {
                    break;
                }
                if socket.send(Message::Binary(bytes)).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            Message::Ping(bytes) => {
                let _ = socket.send(Message::Pong(bytes)).await;
            }
            _ => {
                let _ = socket.send(Message::Close(None)).await;
                break;
            }
        }
    }
}

async fn authorize_token(
    state: &FixtureState,
    token: Option<&str>,
    now: DateTime<Utc>,
) -> Result<()> {
    if state.config.expires_at <= now {
        bail!("publication is expired");
    }
    if state.config.audience == Audience::Public {
        return Ok(());
    }
    let token = token.context("an invitation token is required")?;
    let digest = token_digest(token);
    if revoked(&state.config.revocations_path, &digest)? {
        bail!("invitation is revoked");
    }
    verify_invitation(
        state.invitation_key.as_slice(),
        token,
        &state.config.company,
        &state.config.publication_id,
        &state.config.candidate_digest,
        now,
    )?;
    Ok(())
}

fn revoked(path: &Path, digest: &str) -> Result<bool> {
    match std::fs::read_to_string(path) {
        Ok(raw) => Ok(raw.lines().any(|line| line.trim() == digest)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

fn load_observations(path: &Path) -> Result<ServiceObservations> {
    match std::fs::read(path) {
        Ok(raw) => {
            serde_json::from_slice(&raw).with_context(|| format!("decode {}", path.display()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(ServiceObservations::default())
        }
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

async fn record_accepted(state: &FixtureState, message: bool) -> Result<()> {
    let mut observations = state.observations.lock().await;
    observations.accepted_connections += 1;
    if message {
        observations.messages_received += 1;
    }
    observations.last_activity_at = Some(Utc::now());
    persist_observations(&state.config.observations_path, &observations)
}

async fn record_rejected(state: &FixtureState) -> Result<()> {
    let mut observations = state.observations.lock().await;
    observations.rejected_connections += 1;
    observations.last_activity_at = Some(Utc::now());
    persist_observations(&state.config.observations_path, &observations)
}

async fn record_message(state: &FixtureState) -> Result<()> {
    let mut observations = state.observations.lock().await;
    observations.messages_received += 1;
    observations.last_activity_at = Some(Utc::now());
    persist_observations(&state.config.observations_path, &observations)
}

fn persist_observations(path: &Path, observations: &ServiceObservations) -> Result<()> {
    let raw = serde_json::to_vec(observations).context("encode service observations")?;
    std::fs::write(path, raw).with_context(|| format!("write {}", path.display()))
}

fn write_marker(
    config: &LocalFixtureConfig,
    address: SocketAddr,
    public_endpoint: String,
    transport_security: &str,
    tls_certificate_pem: Option<String>,
) -> Result<()> {
    let invitation_key_id = format!("sha256:{:x}", Sha256::digest(config.invitation_key()?));
    let marker = LocalFixtureMarker {
        receipt: ProviderReadyReceipt {
            contract_version: CONTRACT_VERSION.to_string(),
            publication_id: config.publication_id.clone(),
            candidate_digest: config.candidate_digest.clone(),
            provider_operation_id: config.provider_operation_id.clone(),
            endpoint: ProviderEndpoint {
                profile: config.profile,
                public_endpoint,
                bound_port: address.port(),
                transport_security: transport_security.to_string(),
            },
            invitation_key_id,
            provider_process_id: std::process::id(),
            ready_at: Utc::now(),
        },
        tls_certificate_pem,
    };
    persist_observations(&config.observations_path, &ServiceObservations::default())?;
    let raw = serde_json::to_vec_pretty(&marker).context("encode provider ready marker")?;
    std::fs::write(&config.marker_path, raw)
        .with_context(|| format!("write {}", config.marker_path.display()))
}

fn error_response(status: StatusCode, error: anyhow::Error) -> Response {
    (
        status,
        Json(json!({"ok": false, "error": error.to_string()})),
    )
        .into_response()
}

pub fn load_marker(path: &Path) -> Result<LocalFixtureMarker> {
    let raw = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_slice(&raw).with_context(|| format!("decode {}", path.display()))
}

pub fn observation_value(path: &Path) -> Result<Value> {
    let raw = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_slice(&raw).with_context(|| format!("decode {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use futures_util::{SinkExt as _, StreamExt as _};
    use rustls::pki_types::ServerName;
    use tokio::time::{Duration, Instant};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
    use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

    use super::*;
    use crate::published_service_contract::{sign_invitation, InvitationClaims};

    fn test_directory() -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "restless-published-service-fixture-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn fixture_config(directory: &Path, profile: ServiceProfile, key: &[u8]) -> LocalFixtureConfig {
        let image = format!(
            "registry.example/restless/fixture@sha256:{}",
            "a".repeat(64)
        );
        let readiness = match profile {
            ServiceProfile::HttpsWebsocketDemo => ReadinessProbe::Http {
                path: "/health".into(),
                websocket_path: "/ws".into(),
            },
            ServiceProfile::GodotEnetUdp => ReadinessProbe::Udp {
                request: "RESTLESS_READY_V1".into(),
                response: "RESTLESS_READY_V1_OK".into(),
            },
        };
        LocalFixtureConfig {
            company: "swift-arrival_test".into(),
            publication_id: "publication-fixture-test".into(),
            candidate_digest: format!("sha256:{}", "b".repeat(64)),
            provider_operation_id: "publication-fixture-test".into(),
            profile,
            manifest: ServiceManifest {
                contract_version: CONTRACT_VERSION.into(),
                image,
                profile,
                internal_port: 7777,
                readiness,
            },
            audience: Audience::NamedInvitees,
            bind_host: "127.0.0.1".into(),
            expires_at: Utc::now() + chrono::Duration::minutes(10),
            invitation_key_base64: base64::engine::general_purpose::STANDARD.encode(key),
            marker_path: directory.join("ready.json"),
            observations_path: directory.join("observations.json"),
            revocations_path: directory.join("revoked.sha256"),
        }
    }

    fn invitation(config: &LocalFixtureConfig, key: &[u8], suffix: &str) -> String {
        sign_invitation(
            key,
            &InvitationClaims::new(
                format!("invite-{suffix}"),
                config.publication_id.clone(),
                "swift-arrival_test".into(),
                config.candidate_digest.clone(),
                "playtester@example.com".into(),
                Utc::now() + chrono::Duration::minutes(5),
            ),
        )
        .unwrap()
    }

    async fn start(
        config: &LocalFixtureConfig,
    ) -> (tokio::task::JoinHandle<Result<()>>, LocalFixtureMarker) {
        std::fs::write(&config.revocations_path, b"").unwrap();
        let config_path = config.marker_path.parent().unwrap().join("handoff.json");
        std::fs::write(&config_path, serde_json::to_vec(config).unwrap()).unwrap();
        let task = tokio::spawn({
            let config_path = config_path.clone();
            async move { run_from_config_path(&config_path).await }
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(marker) = load_marker(&config.marker_path) {
                return (task, marker);
            }
            assert!(!task.is_finished(), "fixture exited before readiness");
            assert!(
                Instant::now() < deadline,
                "fixture readiness was not emitted"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn https_and_websocket_are_real_scoped_revocable_transports() {
        let directory = test_directory();
        let key = [9_u8; 32];
        let config = fixture_config(&directory, ServiceProfile::HttpsWebsocketDemo, &key);
        let token = invitation(&config, &key, "https");
        let (task, marker) = start(&config).await;
        let certificate_pem = marker.tls_certificate_pem.as_deref().unwrap();
        let certificate = reqwest::Certificate::from_pem(certificate_pem.as_bytes()).unwrap();
        let client = reqwest::Client::builder()
            .add_root_certificate(certificate)
            .build()
            .unwrap();
        let health = client
            .get(format!(
                "{}/health",
                marker.receipt.endpoint.public_endpoint
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        let refused = client
            .get(format!(
                "{}/health",
                marker.receipt.endpoint.public_endpoint
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);

        let mut roots = rustls::RootCertStore::empty();
        for certificate in rustls_pemfile::certs(&mut BufReader::new(certificate_pem.as_bytes())) {
            roots.add(certificate.unwrap()).unwrap();
        }
        let tls = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let tcp = tokio::net::TcpStream::connect(("127.0.0.1", marker.receipt.endpoint.bound_port))
            .await
            .unwrap();
        let tls = tokio_rustls::TlsConnector::from(Arc::new(tls))
            .connect(ServerName::try_from("localhost").unwrap(), tcp)
            .await
            .unwrap();
        let request = format!(
            "wss://localhost:{}/ws?token={}",
            marker.receipt.endpoint.bound_port, token
        )
        .into_client_request()
        .unwrap();
        let (mut websocket, _) = tokio_tungstenite::client_async(request, tls).await.unwrap();
        websocket
            .send(TungsteniteMessage::Text("playtest".into()))
            .await
            .unwrap();
        assert_eq!(
            websocket.next().await.unwrap().unwrap(),
            TungsteniteMessage::Text("playtest".into())
        );
        websocket.close(None).await.unwrap();

        std::fs::write(
            &config.revocations_path,
            format!("{}\n", token_digest(&token)),
        )
        .unwrap();
        let revoked = client
            .get(format!(
                "{}/health",
                marker.receipt.endpoint.public_endpoint
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
        let observations: ServiceObservations =
            serde_json::from_slice(&std::fs::read(&config.observations_path).unwrap()).unwrap();
        assert!(observations.accepted_connections >= 2);
        assert!(observations.rejected_connections >= 2);
        assert!(observations.messages_received >= 2);

        task.abort();
        let _ = task.await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn udp_profile_rejects_wrong_build_and_releases_its_only_port() {
        let directory = test_directory();
        let key = [11_u8; 32];
        let config = fixture_config(&directory, ServiceProfile::GodotEnetUdp, &key);
        let token = invitation(&config, &key, "udp");
        let wrong_build = sign_invitation(
            &key,
            &InvitationClaims::new(
                "invite-wrong".into(),
                config.publication_id.clone(),
                "swift-arrival_test".into(),
                format!("sha256:{}", "c".repeat(64)),
                "playtester@example.com".into(),
                Utc::now() + chrono::Duration::minutes(5),
            ),
        )
        .unwrap();
        let (task, marker) = start(&config).await;
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let endpoint = ("127.0.0.1", marker.receipt.endpoint.bound_port);
        socket
            .send_to(b"RESTLESS_READY_V1", endpoint)
            .await
            .unwrap();
        let mut response = [0_u8; 4096];
        let (length, _) = socket.recv_from(&mut response).await.unwrap();
        assert_eq!(&response[..length], b"RESTLESS_READY_V1_OK");
        let valid =
            serde_json::to_vec(&json!({"token": token, "payload": "join-player-2"})).unwrap();
        socket.send_to(&valid, endpoint).await.unwrap();
        let (length, _) = socket.recv_from(&mut response).await.unwrap();
        let response_value: Value = serde_json::from_slice(&response[..length]).unwrap();
        assert_eq!(response_value["ok"], true);
        assert_eq!(response_value["payload"], "join-player-2");

        let invalid =
            serde_json::to_vec(&json!({"token": wrong_build, "payload": "join"})).unwrap();
        socket.send_to(&invalid, endpoint).await.unwrap();
        let (length, _) = socket.recv_from(&mut response).await.unwrap();
        let response_value: Value = serde_json::from_slice(&response[..length]).unwrap();
        assert_eq!(response_value["ok"], false);
        assert!(response_value["error"]
            .as_str()
            .unwrap()
            .contains("another build"));

        task.abort();
        let _ = task.await;
        drop(socket);
        std::net::UdpSocket::bind(endpoint)
            .expect("UDP route should be released after fixture stop");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
