//! Settings › Computers and Company › Computers (S99): the machines the owner holds, how a new one
//! pairs, and which companies use which. Only the account owner pairs, renames, pauses, removes or
//! assigns; a company's members see that company's machines. The machine side (pairing, the link,
//! the install scripts and the image a new machine starts from) is public and authenticates with a
//! pairing code or the machine's own credential.
use super::*;
use axum::extract::ws::{Message as WsMessage, WebSocketUpgrade};
use futures_util::{SinkExt as _, StreamExt as _};
use axum::http::header::AUTHORIZATION;
use restless_engine::machines::{self, WireMessage};
use serde_json::{json, Value};
use restless_machine::protocol::{PairRequest, CONNECT_PS1_PATH, CONNECT_SH_PATH, LINK_PATH, PAIR_PATH};

/// Where a new machine fetches the image it starts from, with its pairing code.
pub(crate) const HELPER_IMAGE_PATH: &str = "/machines/helper-image";
const PAIRING_HEADER: &str = "x-restless-pairing";

/// Paths a machine reaches without a browser session.
pub(crate) fn is_machine_path(path: &str) -> bool {
    matches!(path, PAIR_PATH | LINK_PATH | CONNECT_SH_PATH | CONNECT_PS1_PATH | HELPER_IMAGE_PATH)
}

fn refuse(error: anyhow::Error) -> Response<Body> {
    let message = format!("{error:#}");
    if message.ends_with('.') && message.chars().count() <= 200 && !message.contains("error") {
        api_error(StatusCode::BAD_REQUEST, "machines", message)
    } else {
        tracing::warn!(error = %message, "machine change failed");
        api_error(StatusCode::SERVICE_UNAVAILABLE, "machines", "Computers could not be changed. Try again.")
    }
}

fn owner_only(principal: &RequestPrincipal) -> Option<Response<Body>> {
    (!principal.is_account_owner()).then(|| {
        api_error(StatusCode::FORBIDDEN, "machines", "Only the account owner manages computers.")
    })
}

/// The plane's own origin as the browser reached it, for the install line.
fn plane_origin(headers: &HeaderMap) -> Option<String> {
    // Cloud's edge sends an empty X-Forwarded-Host and names the plane in Host.
    let named = |name: &str| headers.get(name).and_then(|value| value.to_str().ok()).filter(|value| !value.is_empty());
    let host = named("x-forwarded-host").or_else(|| named(HOST.as_str()))?;
    if host.is_empty() || !host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']')) {
        return None;
    }
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .filter(|scheme| matches!(*scheme, "http" | "https"))
        .unwrap_or(if host.starts_with("127.0.0.1") || host.starts_with("localhost") { "http" } else { "https" });
    Some(format!("{scheme}://{host}"))
}

/// The address a machine connects from, as the plane's edge saw it. Shown to the owner only.
fn observed_address(headers: &HeaderMap) -> Option<String> {
    headers
        .get("cf-connecting-ip")
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(|value| value.trim().to_string())
        .filter(|value| value.parse::<std::net::IpAddr>().is_ok())
}

pub(super) async fn list(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    match machines::list(state.daemon.authority.pool()).await {
        Ok(held) => Json(json!({ "machines": held })).into_response(),
        Err(error) => refuse(error),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct NewPairing {
    #[serde(default)]
    label: Option<String>,
}

pub(super) async fn create_pairing(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    headers: HeaderMap,
    Json(input): Json<NewPairing>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let Some(origin) = plane_origin(&headers) else {
        return api_error(StatusCode::BAD_REQUEST, "machines", "This plane's address is unknown.");
    };
    match machines::create_pairing(state.daemon.authority.pool(), input.label.as_deref()).await {
        Ok(pairing) => (
            StatusCode::CREATED,
            Json(json!({
                "code": pairing.code,
                "expires_at": pairing.expires_at,
                "commands": {
                    "unix": format!("curl -fsSL {origin}{CONNECT_SH_PATH} | sh -s -- {}", pairing.code),
                    "windows": format!("& ([scriptblock]::Create((irm {origin}{CONNECT_PS1_PATH}))) {}", pairing.code),
                },
            })),
        )
            .into_response(),
        Err(error) => refuse(error),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct MachineChange {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    paused: Option<bool>,
}

pub(super) async fn change(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(id): AxumPath<String>,
    Json(input): Json<MachineChange>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let pool = state.daemon.authority.pool();
    if let Some(label) = input.label.as_deref() {
        if let Err(error) = machines::rename(pool, &id, label).await {
            return refuse(error);
        }
    }
    if let Some(paused) = input.paused {
        if let Err(error) = machines::set_paused(pool, &id, paused).await {
            return refuse(error);
        }
    }
    match machines::get(pool, &id).await {
        Ok(Some(machine)) => Json(machine).into_response(),
        Ok(None) => api_error(StatusCode::NOT_FOUND, "machines", "That computer no longer exists."),
        Err(error) => refuse(error),
    }
}

pub(super) async fn remove(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(id): AxumPath<String>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    match machines::remove(state.daemon.authority.pool(), &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => refuse(error),
    }
}

/// A company's machines with what members may see of each: name, role, online, OS.
pub(super) async fn company(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if !principal.permits_company(&company) {
        return api_error(StatusCode::FORBIDDEN, "machines", "You are not in that company.");
    }
    let pool = state.daemon.authority.pool();
    let (assignments, held) = match (machines::company_assignments(pool, &company).await, machines::list(pool).await) {
        (Ok(assignments), Ok(held)) => (assignments, held),
        (Err(error), _) | (_, Err(error)) => return refuse(error),
    };
    let rows: Vec<Value> = assignments
        .into_iter()
        .filter_map(|assignment| {
            let machine = held.iter().find(|machine| machine.id == assignment.machine_id)?;
            Some(json!({
                "id": machine.id,
                "label": machine.label,
                "kind": machine.kind,
                "role": assignment.role,
                "online": machine.online,
                "paused": machine.paused,
                "os": machine.facts.os,
                "cpus": machine.facts.cpus,
                "memory_bytes": machine.facts.memory_bytes,
                "last_seen_at": machine.last_seen_at,
            }))
        })
        .collect();
    Json(json!({
        "machines": rows,
        "can_manage": principal.is_account_owner(),
        "move": restless_engine::machine_move::progress(&company),
    }))
    .into_response()
}

/// Move the company's computer to another machine. Answers at once; the company view reports
/// each step.
pub(super) async fn move_main(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<AssignInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(_) => return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist."),
    };
    let pool = state.daemon.authority.pool().clone();
    match restless_engine::machine_move::start(pool, config, input.machine_id).await {
        Ok(()) => StatusCode::ACCEPTED.into_response(),
        Err(error) => refuse(error),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct AssignInput {
    machine_id: String,
}

pub(super) async fn assign_helper(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<AssignInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    if input.machine_id == machines::PLANE_MACHINE {
        return api_error(StatusCode::BAD_REQUEST, "machines", "The plane already serves every company.");
    }
    match machines::assign(state.daemon.authority.pool(), &company, &input.machine_id, machines::ROLE_HELPER).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => refuse(error),
    }
}

pub(super) async fn unassign(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, machine)): AxumPath<(String, String)>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    match machines::unassign(state.daemon.authority.pool(), &company, &machine).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => refuse(error),
    }
}

// ---------------------------------------------------------------------------------------------
// The machine side.

pub(crate) async fn pair(State(state): State<OwnerState>, headers: HeaderMap, Json(input): Json<PairRequest>) -> Response<Body> {
    let address = observed_address(&headers);
    match machines::pair(state.daemon.authority.pool(), &input.code, input.facts, address.as_deref()).await {
        Ok(paired) => Json(paired).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            if message.ends_with('.') && !message.contains("error") {
                api_error(StatusCode::FORBIDDEN, "pairing", message)
            } else {
                tracing::warn!(error = %message, "pairing failed");
                api_error(StatusCode::SERVICE_UNAVAILABLE, "pairing", "Pairing could not be saved. Try again.")
            }
        }
    }
}

pub(crate) async fn link(State(state): State<OwnerState>, headers: HeaderMap, websocket: WebSocketUpgrade) -> Response<Body> {
    let credential = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
        .to_string();
    let pool = state.daemon.authority.pool().clone();
    let (machine_id, epoch) = match machines::authenticate(&pool, &credential).await {
        Ok(Some(found)) => found,
        Ok(None) => return api_error(StatusCode::UNAUTHORIZED, "machine", "This computer is not paired with this plane."),
        Err(error) => {
            tracing::warn!(error = %format!("{error:#}"), "machine authentication failed");
            return api_error(StatusCode::SERVICE_UNAVAILABLE, "machine", "Try again shortly.");
        }
    };
    let address = observed_address(&headers);
    websocket
        .max_message_size(1024 * 1024)
        .on_upgrade(move |socket| async move {
            let (mut sink, mut source) = socket.split();
            let (inbound_tx, inbound) = tokio::sync::mpsc::channel::<WireMessage>(256);
            let (outbound, mut outbound_rx) = tokio::sync::mpsc::channel::<WireMessage>(256);
            let writer = tokio::spawn(async move {
                while let Some(message) = outbound_rx.recv().await {
                    let message = match message {
                        WireMessage::Binary(frame) => WsMessage::Binary(frame.into()),
                        WireMessage::Text(text) => WsMessage::Text(text.into()),
                    };
                    if sink.send(message).await.is_err() {
                        break;
                    }
                }
                let _ = sink.close().await;
            });
            let reader = tokio::spawn(async move {
                while let Some(Ok(message)) = source.next().await {
                    let message = match message {
                        WsMessage::Binary(frame) => WireMessage::Binary(frame.to_vec()),
                        WsMessage::Text(text) => WireMessage::Text(text.to_string()),
                        WsMessage::Close(_) => break,
                        _ => continue,
                    };
                    if inbound_tx.send(message).await.is_err() {
                        break;
                    }
                }
            });
            machines::serve_link(pool, machine_id, epoch, address, inbound, outbound).await;
            reader.abort();
            writer.abort();
        })
        .into_response()
}

/// The image a new machine starts from (the plane's company image, which carries the helper), for
/// a pairing code that is still open. It streams `docker save`; the install script loads it.
pub(crate) async fn helper_image(State(state): State<OwnerState>, headers: HeaderMap) -> Response<Body> {
    let code = headers.get(PAIRING_HEADER).and_then(|value| value.to_str().ok()).unwrap_or_default();
    match machines::pairing_is_open(state.daemon.authority.pool(), code).await {
        Ok(true) => {}
        Ok(false) => return api_error(StatusCode::FORBIDDEN, "pairing", "That code has expired or was already used."),
        Err(_) => return api_error(StatusCode::SERVICE_UNAVAILABLE, "pairing", "Try again shortly."),
    }
    let Some(image) = machines::allowed_images().await.into_iter().next() else {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "pairing", "This plane has no company image to share yet.");
    };
    let child = tokio::process::Command::new("docker")
        .args(["save", &image.id])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn();
    let Ok(mut child) = child else {
        return api_error(StatusCode::SERVICE_UNAVAILABLE, "pairing", "The plane could not read its image.");
    };
    let stdout = child.stdout.take().unwrap();
    tokio::spawn(async move {
        let _ = child.wait().await;
    });
    let body = Body::from_stream(tokio_util::io::ReaderStream::with_capacity(stdout, 256 * 1024));
    let mut response = Response::new(body);
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/x-tar"));
    response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

const CONNECT_SH: &str = include_str!("machine_connect.sh");
const CONNECT_PS1: &str = include_str!("machine_connect.ps1");

fn script(headers: &HeaderMap, template: &str, content_type: &'static str) -> Response<Body> {
    let Some(origin) = plane_origin(headers) else {
        return api_error(StatusCode::BAD_REQUEST, "machines", "This plane's address is unknown.");
    };
    let mut response = Response::new(Body::from(template.replace("__RESTLESS_PLANE__", &origin)));
    response.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(crate) async fn connect_sh(headers: HeaderMap) -> Response<Body> {
    script(&headers, CONNECT_SH, "text/x-shellscript; charset=utf-8")
}

pub(crate) async fn connect_ps1(headers: HeaderMap) -> Response<Body> {
    script(&headers, CONNECT_PS1, "text/plain; charset=utf-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_install_line_uses_the_plane_the_owner_reached() {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("owner-ab12.restless.run"));
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        assert_eq!(plane_origin(&headers).as_deref(), Some("https://owner-ab12.restless.run"));
        // As Cloud's edge sends it: an empty forwarded host beside the real one.
        headers.insert("x-forwarded-host", HeaderValue::from_static(""));
        assert_eq!(plane_origin(&headers).as_deref(), Some("https://owner-ab12.restless.run"));
        headers.insert(HOST, HeaderValue::from_static("evil.example/$(rm -rf ~)"));
        assert_eq!(plane_origin(&headers), None);
    }

    #[test]
    fn the_scripts_name_the_plane_and_never_a_secret() {
        for template in [CONNECT_SH, CONNECT_PS1] {
            assert!(template.contains("__RESTLESS_PLANE__"));
            assert!(!template.to_lowercase().contains("token"));
        }
        assert!(is_machine_path("/connect") && is_machine_path("/machines/link") && !is_machine_path("/api/machines"));
    }
}
