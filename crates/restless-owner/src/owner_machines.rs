//! Settings › Computers and Company › Computers (S99): the machines the owner holds, how a new one
//! pairs, and which companies use which. Only the account owner pairs, renames, pauses, removes or
//! assigns; a company's members see that company's machines. The machine side (pairing, the link,
//! the install scripts and the image a new machine starts from) is public and authenticates with a
//! pairing code or the machine's own credential.
use super::*;
use axum::extract::ws::{Message as WsMessage, WebSocketUpgrade};
use axum::http::header::AUTHORIZATION;
use futures_util::{SinkExt as _, StreamExt as _};
use restless_engine::machines::{self, WireMessage};
use restless_machine::protocol::{
    PairRequest, CONNECT_PS1_PATH, CONNECT_SH_PATH, LINK_PATH, PAIR_PATH,
};
use serde_json::{json, Value};

/// Where a new machine fetches the image it starts from, with its pairing code.
pub(crate) const HELPER_IMAGE_PATH: &str = "/machines/helper-image";
const PAIRING_HEADER: &str = "x-restless-pairing";

/// Paths a machine reaches without a browser session.
pub(crate) fn is_machine_path(path: &str) -> bool {
    matches!(
        path,
        PAIR_PATH | LINK_PATH | CONNECT_SH_PATH | CONNECT_PS1_PATH | HELPER_IMAGE_PATH
    )
}

fn refuse(error: anyhow::Error) -> Response<Body> {
    let message = format!("{error:#}");
    if message.ends_with('.') && message.chars().count() <= 200 && !message.contains("error") {
        api_error(StatusCode::BAD_REQUEST, "machines", message)
    } else {
        tracing::warn!(error = %message, "machine change failed");
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "machines",
            "Computers could not be changed. Try again.",
        )
    }
}

fn owner_only(principal: &RequestPrincipal) -> Option<Response<Body>> {
    (!principal.is_account_owner()).then(|| {
        api_error(
            StatusCode::FORBIDDEN,
            "machines",
            "Only the account owner manages computers.",
        )
    })
}

/// Where a machine reaches this plane, for the install line and the helper. A network plane names
/// its own host (on Cloud `owner-….restless.run`, behind its tunnel; owners browse it through
/// `app.restless.run`) with the account site's scheme. A local appliance is the host the browser
/// used.
fn plane_origin(state: &OwnerState, headers: &HeaderMap) -> Option<String> {
    if let Some(network) = state.entry.network() {
        let scheme = if network.issuer().starts_with("http://") {
            "http"
        } else {
            "https"
        };
        let configured = valid_host(network.host())?;
        // The configured host carries no port; keep the one the browser used to reach it.
        let reached = headers
            .get(HOST)
            .and_then(|value| value.to_str().ok())
            .and_then(valid_host);
        let host = match reached {
            Some(reached)
                if reached.rsplit_once(':').map_or(reached, |(name, _)| name) == configured =>
            {
                reached
            }
            _ => configured,
        };
        return Some(format!("{scheme}://{host}"));
    }
    request_origin(headers)
}

fn valid_host(host: &str) -> Option<&str> {
    (!host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']')))
    .then_some(host)
}

/// The origin a browser used, from its Host (Cloud's edge sends an empty X-Forwarded-Host).
fn request_origin(headers: &HeaderMap) -> Option<String> {
    let named = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.is_empty())
    };
    let host = valid_host(named("x-forwarded-host").or_else(|| named(HOST.as_str()))?)?;
    let bare = host.rsplit_once(':').map_or(host, |(name, _)| name);
    let local = bare == "localhost"
        || bare.ends_with(".localhost")
        || bare == "127.0.0.1"
        || bare == "[::1]";
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .filter(|scheme| matches!(*scheme, "http" | "https"))
        .unwrap_or(if local { "http" } else { "https" });
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
        Ok(held) => {
            let apps: serde_json::Map<String, Value> = held
                .iter()
                .map(|machine| (machine.id.clone(), json!(machines::apps_for(&machine.id))))
                .collect();
            Json(json!({ "machines": held, "apps": apps })).into_response()
        }
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
    let Some(origin) = plane_origin(&state, &headers) else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "machines",
            "This plane's address is unknown.",
        );
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
        Ok(None) => api_error(
            StatusCode::NOT_FOUND,
            "machines",
            "That computer no longer exists.",
        ),
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
        return api_error(
            StatusCode::FORBIDDEN,
            "machines",
            "You are not in that company.",
        );
    }
    let pool = state.daemon.authority.pool();
    let (assignments, held) = match (
        machines::company_assignments(pool, &company).await,
        machines::list(pool).await,
    ) {
        (Ok(assignments), Ok(held)) => (assignments, held),
        (Err(error), _) | (_, Err(error)) => return refuse(error),
    };
    let rows: Vec<Value> = assignments
        .into_iter()
        .filter_map(|assignment| {
            let machine = held
                .iter()
                .find(|machine| machine.id == assignment.machine_id)?;
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
    let home_sites = runtime::CompanyConfig::load(&state.daemon.root, &company)
        .map(|config| config.home_sites)
        .unwrap_or_default();
    Json(json!({
        "machines": rows,
        "can_manage": principal.is_account_owner(),
        "move": restless_engine::machine_move::progress(&company),
        "home_sites": home_sites,
    }))
    .into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HomeSitesInput {
    sites: Vec<String>,
}

/// The sites this company reaches through its helper machines, from a home IP (S99 step 2). The
/// helpers learn the new list at once, and the company browser takes it on a fresh start.
pub(super) async fn set_home_sites(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<HomeSitesInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let mut sites: Vec<String> = input
        .sites
        .iter()
        .map(|site| {
            site.trim()
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_start_matches("www.")
                .trim_end_matches('/')
                .to_ascii_lowercase()
        })
        .filter(|site| !site.is_empty())
        .collect();
    if let Some(bad) = sites.iter().find(|site| !runtime::valid_home_site(site)) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "home_sites",
            format!("{bad} is not a site name. Use names like ebay.com.au."),
        );
    }
    sites.sort();
    sites.dedup();
    if sites.len() > 40 {
        return api_error(
            StatusCode::BAD_REQUEST,
            "home_sites",
            "Name at most 40 sites.",
        );
    }
    let mut config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(_) => return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist."),
    };
    config.home_sites = sites.clone();
    if let Err(error) = runtime::CompanyConfig::save(&state.daemon.root, &config) {
        return refuse(error);
    }
    let pool = state.daemon.authority.pool();
    if let Ok(assignments) = machines::company_assignments(pool, &company).await {
        for assignment in assignments
            .iter()
            .filter(|a| a.role == machines::ROLE_HELPER)
        {
            machines::push_settings(pool, &assignment.machine_id).await;
        }
    }
    // A running computer restarts its browser with the new sites; an asleep one takes them on waking.
    if runtime::status(&company).await.ok() == Some(runtime::ContainerStatus::Running) {
        if let Err(error) = runtime::refresh_computer(&config).await {
            tracing::warn!(
                company,
                "home sites saved; the computer did not restart: {error:#}"
            );
        }
    }
    Json(json!({ "home_sites": sites })).into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AppsInput {
    apps: Vec<restless_machine::protocol::MachineApp>,
}

/// The owner's own apps on one of their computers (S99 step 2): local services, such as an MCP
/// server that needs that computer's files, which companies reach through the computer's link as
/// `machine-app://<computer>/<app>/<path>`.
pub(super) async fn set_apps(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(id): AxumPath<String>,
    Json(input): Json<AppsInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    match machines::set_apps(state.daemon.authority.pool(), &id, input.apps).await {
        Ok(()) => Json(json!({ "apps": machines::apps_for(&id) })).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "machines", format!("{error:#}")),
    }
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
        return api_error(
            StatusCode::BAD_REQUEST,
            "machines",
            "The plane already serves every company.",
        );
    }
    match machines::assign(
        state.daemon.authority.pool(),
        &company,
        &input.machine_id,
        machines::ROLE_HELPER,
    )
    .await
    {
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

pub(crate) async fn pair(
    State(state): State<OwnerState>,
    headers: HeaderMap,
    Json(input): Json<PairRequest>,
) -> Response<Body> {
    let address = observed_address(&headers);
    match machines::pair(
        state.daemon.authority.pool(),
        &input.code,
        input.facts,
        address.as_deref(),
    )
    .await
    {
        Ok(paired) => Json(paired).into_response(),
        Err(error) => {
            let message = format!("{error:#}");
            if message.ends_with('.') && !message.contains("error") {
                api_error(StatusCode::FORBIDDEN, "pairing", message)
            } else {
                tracing::warn!(error = %message, "pairing failed");
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "pairing",
                    "Pairing could not be saved. Try again.",
                )
            }
        }
    }
}

pub(crate) async fn link(
    State(state): State<OwnerState>,
    headers: HeaderMap,
    websocket: WebSocketUpgrade,
) -> Response<Body> {
    let credential = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
        .to_string();
    let pool = state.daemon.authority.pool().clone();
    let (machine_id, epoch) = match machines::authenticate(&pool, &credential).await {
        Ok(Some(found)) => found,
        Ok(None) => {
            return api_error(
                StatusCode::UNAUTHORIZED,
                "machine",
                "This computer is not paired with this plane.",
            )
        }
        Err(error) => {
            tracing::warn!(error = %format!("{error:#}"), "machine authentication failed");
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "machine",
                "Try again shortly.",
            );
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
pub(crate) async fn helper_image(
    State(state): State<OwnerState>,
    headers: HeaderMap,
) -> Response<Body> {
    let code = headers
        .get(PAIRING_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    match machines::pairing_is_open(state.daemon.authority.pool(), code).await {
        Ok(true) => {}
        Ok(false) => {
            return api_error(
                StatusCode::FORBIDDEN,
                "pairing",
                "That code has expired or was already used.",
            )
        }
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "pairing",
                "Try again shortly.",
            )
        }
    }
    let Some(image) = machines::allowed_images().await.into_iter().next() else {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "pairing",
            "This plane has no company image to share yet.",
        );
    };
    let child = machines::save_image(&image.id).await;
    let Ok(mut child) = child else {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "pairing",
            "The plane could not read its image.",
        );
    };
    let stdout = child.stdout.take().unwrap();
    tokio::spawn(async move {
        let _ = child.wait().await;
    });
    let body = Body::from_stream(tokio_util::io::ReaderStream::with_capacity(
        stdout,
        256 * 1024,
    ));
    let mut response = Response::new(body);
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/x-tar"));
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

const CONNECT_SH: &str = include_str!("machine_connect.sh");
const CONNECT_PS1: &str = include_str!("machine_connect.ps1");

async fn script(
    state: &OwnerState,
    headers: &HeaderMap,
    template: &str,
    content_type: &'static str,
) -> Response<Body> {
    let Some(origin) = plane_origin(state, headers) else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "machines",
            "This plane's address is unknown.",
        );
    };
    // The exact image a new computer pulls from Core's public registry; empty when the plane runs a
    // local build, and the script then fetches it from the plane.
    let allowed = machines::allowed_images().await.into_iter().next();
    let image = allowed
        .as_ref()
        .map(|image| image.reference.clone())
        .filter(|reference| reference.contains("@sha256:"))
        .unwrap_or_default();
    // The image id the plane allows: a fallback download from the plane must load exactly this.
    let image_id = allowed
        .map(|image| image.id)
        .filter(|id| id.starts_with("sha256:") && id[7..].bytes().all(|b| b.is_ascii_hexdigit()))
        .unwrap_or_default();
    let mut response = Response::new(Body::from(
        template
            .replace("__RESTLESS_PLANE__", &origin)
            .replace("__RESTLESS_IMAGE_ID__", &image_id)
            .replace("__RESTLESS_IMAGE__", &image),
    ));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(crate) async fn connect_sh(
    State(state): State<OwnerState>,
    headers: HeaderMap,
) -> Response<Body> {
    script(
        &state,
        &headers,
        CONNECT_SH,
        "text/x-shellscript; charset=utf-8",
    )
    .await
}

pub(crate) async fn connect_ps1(
    State(state): State<OwnerState>,
    headers: HeaderMap,
) -> Response<Body> {
    script(&state, &headers, CONNECT_PS1, "text/plain; charset=utf-8").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_install_line_uses_the_plane_the_owner_reached() {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("owner-ab12.restless.run"));
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        assert_eq!(
            request_origin(&headers).as_deref(),
            Some("https://owner-ab12.restless.run")
        );
        // As Cloud's edge sends it: an empty forwarded host beside the real one.
        headers.insert("x-forwarded-host", HeaderValue::from_static(""));
        assert_eq!(
            request_origin(&headers).as_deref(),
            Some("https://owner-ab12.restless.run")
        );
        headers.insert(HOST, HeaderValue::from_static("evil.example/$(rm -rf ~)"));
        assert_eq!(request_origin(&headers), None);
    }

    #[test]
    fn the_scripts_name_the_plane_and_never_a_secret() {
        for template in [CONNECT_SH, CONNECT_PS1] {
            assert!(template.contains("__RESTLESS_PLANE__"));
            assert!(!template.to_lowercase().contains("token"));
        }
        assert!(
            is_machine_path("/connect")
                && is_machine_path("/machines/link")
                && !is_machine_path("/api/machines")
        );
    }
}
