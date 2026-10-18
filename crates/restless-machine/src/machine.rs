//! The helper on a paired machine: pair once, then keep one link to the plane open and answer it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use futures_util::{SinkExt as _, StreamExt as _};
use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::tungstenite::Message;

use crate::docker_filter::Scope;
use crate::docker_proxy::DockerProxy;
use crate::link::{Incoming, Link, Side};
use crate::protocol::{
    Control, MachineFacts, PairRequest, PairResponse, HEARTBEAT_SECONDS, LINK_PATH, MACHINE_LABEL, MACHINE_NETWORK,
    PAIR_PATH, STREAM_CONNECT, STREAM_DOCKER, STREAM_PLANE,
};

/// What a paired machine keeps, readable only by root inside the helper's volume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paired {
    pub plane: String,
    pub machine_id: String,
    pub label: String,
    pub credential: String,
}

impl Paired {
    pub fn load(path: &Path) -> Result<Option<Self>> {
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes).context("read the saved pairing")?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("read the saved pairing"),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("tmp");
        std::fs::write(&temporary, serde_json::to_vec(self)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(&temporary, path)?;
        Ok(())
    }
}

fn plane_url(plane: &str, path: &str, websocket: bool) -> Result<url::Url> {
    let mut url = url::Url::parse(plane).context("the plane address must be a URL")?;
    if websocket {
        let scheme = match url.scheme() {
            "https" => "wss",
            "http" => "ws",
            other => bail!("unexpected plane scheme {other}"),
        };
        url.set_scheme(scheme).map_err(|_| anyhow::anyhow!("cannot use {scheme}"))?;
    }
    url.set_path(path);
    Ok(url)
}

/// What this machine is, as Docker reports it.
pub async fn facts(proxy: &DockerProxy) -> MachineFacts {
    let info: Value = match proxy.call(Method::GET, "/info", None).await {
        Ok((status, bytes)) if status.is_success() => serde_json::from_slice(&bytes).unwrap_or_default(),
        _ => Value::Null,
    };
    let text = |key: &str| info.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    MachineFacts {
        hostname: std::env::var("RESTLESS_MACHINE_NAME").ok().filter(|name| !name.trim().is_empty()).unwrap_or_else(|| text("Name")),
        os: text("OperatingSystem"),
        arch: text("Architecture"),
        cpus: info.get("NCPU").and_then(Value::as_u64).unwrap_or_default() as u32,
        memory_bytes: info.get("MemTotal").and_then(Value::as_u64).unwrap_or_default(),
        docker_version: text("ServerVersion"),
        helper_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Exchange a pairing code for this machine's credential.
pub async fn pair(plane: &str, code: &str, facts: MachineFacts) -> Result<Paired> {
    let response = reqwest::Client::new()
        .post(plane_url(plane, PAIR_PATH, false)?)
        .json(&PairRequest { code: code.trim().to_string(), facts })
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .context("reach the plane")?;
    if !response.status().is_success() {
        let status = response.status();
        let body: Value = response.json().await.unwrap_or_default();
        let message = body.get("message").and_then(Value::as_str).unwrap_or("pairing was refused");
        bail!("{message} ({status})");
    }
    let paired: PairResponse = response.json().await.context("read the plane's answer")?;
    Ok(Paired { plane: plane.trim_end_matches('/').to_string(), machine_id: paired.machine_id, label: paired.label, credential: paired.credential })
}

/// Make sure the machine network exists, the helper is on it, and return the helper's address
/// there.
async fn join_machine_network(proxy: &DockerProxy, machine_id: &str) -> Result<String> {
    let (status, _) = proxy.call(Method::GET, &format!("/networks/{MACHINE_NETWORK}"), None).await?;
    if status == http::StatusCode::NOT_FOUND {
        proxy
            .call(
                Method::POST,
                "/networks/create",
                Some(json!({ "Name": MACHINE_NETWORK, "Driver": "bridge", "Labels": { MACHINE_LABEL: machine_id } })),
            )
            .await?;
    }
    let me = std::env::var("HOSTNAME").context("the helper runs in a container")?;
    // Already joined answers 403; either way it is on the network afterwards.
    let _ = proxy
        .call(Method::POST, &format!("/networks/{MACHINE_NETWORK}/connect"), Some(json!({ "Container": me })))
        .await?;
    let (_, bytes) = proxy.call(Method::GET, &format!("/containers/{me}/json"), None).await?;
    let inspected: Value = serde_json::from_slice(&bytes)?;
    inspected
        .pointer(&format!("/NetworkSettings/Networks/{MACHINE_NETWORK}/IPAddress"))
        .and_then(Value::as_str)
        .filter(|address| !address.is_empty())
        .map(str::to_string)
        .context("the helper has no address on the machine network")
}

async fn connect_to_container(proxy: &DockerProxy, target: &str) -> Result<TcpStream> {
    let (name, port) = target.rsplit_once(':').context("connect target is name:port")?;
    let port: u16 = port.parse().context("connect port")?;
    if port == 0 || !proxy.owns_container(name).await {
        bail!("not a Restless container on this machine");
    }
    let (_, bytes) = proxy.call(Method::GET, &format!("/containers/{name}/json"), None).await?;
    let inspected: Value = serde_json::from_slice(&bytes)?;
    let networks = inspected.pointer("/NetworkSettings/Networks").and_then(Value::as_object).context("no networks")?;
    let address = networks
        .get(MACHINE_NETWORK)
        .into_iter()
        .chain(networks.values())
        .filter_map(|network| network.get("IPAddress").and_then(Value::as_str))
        .find(|address| !address.is_empty())
        .context("the container has no address")?;
    Ok(TcpStream::connect((address, port)).await?)
}

fn answer(incoming: Incoming, proxy: Arc<DockerProxy>) {
    tokio::spawn(async move {
        match incoming.kind.as_str() {
            STREAM_DOCKER => {
                let stream = incoming.accept();
                proxy.serve(stream).await;
            }
            STREAM_CONNECT => match connect_to_container(&proxy, &incoming.target).await {
                Ok(mut tcp) => {
                    let mut stream = incoming.accept();
                    let _ = tokio::io::copy_bidirectional(&mut stream, &mut tcp).await;
                }
                Err(error) => incoming.refuse(&format!("{error:#}")),
            },
            _ => incoming.refuse("this machine does not offer that"),
        }
    });
}

/// Listen on the plane ports company containers call, and carry each call to the plane.
fn plane_listeners(link: &Link, ports: &[u16], running: &mut HashMap<u16, JoinHandle<()>>) {
    running.retain(|port, task| {
        let keep = ports.contains(port);
        if !keep {
            task.abort();
        }
        keep
    });
    for port in ports {
        if running.contains_key(port) {
            continue;
        }
        let link = link.clone();
        let port = *port;
        running.insert(
            port,
            tokio::spawn(async move {
                let listener = match TcpListener::bind(("0.0.0.0", port)).await {
                    Ok(listener) => listener,
                    Err(error) => {
                        tracing::warn!(port, %error, "cannot listen for company containers");
                        return;
                    }
                };
                while let Ok((mut tcp, _)) = listener.accept().await {
                    let link = link.clone();
                    tokio::spawn(async move {
                        match link.open(STREAM_PLANE, &port.to_string()).await {
                            Ok(mut stream) => {
                                let _ = tokio::io::copy_bidirectional(&mut tcp, &mut stream).await;
                            }
                            Err(error) => tracing::debug!(port, %error, "plane refused a company call"),
                        }
                    });
                }
            }),
        );
    }
}

pub enum Ended {
    /// The link dropped; try again.
    Dropped,
    /// The owner removed this machine. Its credential is gone.
    Removed,
}

/// Hold one link open until it drops or the machine is removed.
pub async fn link_once(paired: &Paired, socket: &Path, state_path: &Path) -> Result<Ended> {
    let scope = Arc::new(RwLock::new(Scope { machine_id: paired.machine_id.clone(), ..Scope::default() }));
    let proxy = DockerProxy::new(socket.to_path_buf(), Arc::clone(&scope));
    let mut request = plane_url(&paired.plane, LINK_PATH, true)?.as_str().into_client_request()?;
    request
        .headers_mut()
        .insert(http::header::AUTHORIZATION, format!("Bearer {}", paired.credential).parse()?);
    let (socket_stream, _) = match tokio_tungstenite::connect_async(request).await {
        Ok(connected) => connected,
        Err(tokio_tungstenite::tungstenite::Error::Http(response))
            if matches!(response.status().as_u16(), 401 | 403) =>
        {
            let _ = std::fs::remove_file(state_path);
            return Ok(Ended::Removed);
        }
        Err(error) => return Err(error).context("open the link to the plane"),
    };
    tracing::info!(machine = %paired.label, "linked to the plane");
    let (mut sink, mut source) = socket_stream.split();
    let (link, mut frames) = Link::new(Side::Machine);
    proxy.set_link(Some(link.clone()));
    let (text_out, mut texts) = tokio::sync::mpsc::unbounded_channel::<String>();
    let writer = tokio::spawn(async move {
        loop {
            let message = tokio::select! {
                frame = frames.recv() => match frame { Some(frame) => Message::Binary(frame.into()), None => break },
                text = texts.recv() => match text { Some(text) => Message::Text(text.into()), None => break },
            };
            if sink.send(message).await.is_err() {
                break;
            }
        }
    });
    let heartbeat = {
        let proxy = Arc::clone(&proxy);
        let text_out = text_out.clone();
        tokio::spawn(async move {
            loop {
                let control = Control::Heartbeat { facts: facts(&proxy).await };
                if text_out.send(serde_json::to_string(&control).unwrap()).is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(HEARTBEAT_SECONDS)).await;
            }
        })
    };
    let mut listeners = HashMap::new();
    let mut ended = Ended::Dropped;
    while let Some(message) = source.next().await {
        let Ok(message) = message else { break };
        match message {
            Message::Binary(frame) => {
                if let Some(incoming) = link.receive(&frame) {
                    answer(incoming, Arc::clone(&proxy));
                }
            }
            Message::Text(text) => match serde_json::from_str::<Control>(&text) {
                Ok(Control::Settings { machine_id, allowed_images, plane_ports }) => {
                    let helper_address = match join_machine_network(&proxy, &machine_id).await {
                        Ok(address) => Some(address),
                        Err(error) => {
                            tracing::warn!(%error, "could not join the machine network");
                            None
                        }
                    };
                    *scope.write().unwrap() = Scope { machine_id, allowed_images, helper_address };
                    plane_listeners(&link, &plane_ports, &mut listeners);
                }
                Ok(Control::Removed) => {
                    let _ = std::fs::remove_file(state_path);
                    ended = Ended::Removed;
                    break;
                }
                Ok(Control::Heartbeat { .. }) | Err(_) => {}
            },
            Message::Close(_) => break,
            _ => {}
        }
    }
    link.close();
    proxy.set_link(None);
    heartbeat.abort();
    writer.abort();
    for (_, task) in listeners {
        task.abort();
    }
    Ok(ended)
}

/// Keep the machine linked for as long as it stays paired.
pub async fn run(paired: Paired, socket: PathBuf, state_path: PathBuf) -> Result<()> {
    let mut wait = Duration::from_secs(1);
    loop {
        match link_once(&paired, &socket, &state_path).await {
            Ok(Ended::Removed) => {
                tracing::warn!("this computer was removed from Restless; pair it again to use it");
                return Ok(());
            }
            Ok(Ended::Dropped) => wait = Duration::from_secs(1),
            Err(error) => tracing::warn!(error = %format!("{error:#}"), "link failed; retrying"),
        }
        tokio::time::sleep(wait).await;
        wait = (wait * 2).min(Duration::from_secs(60));
    }
}
