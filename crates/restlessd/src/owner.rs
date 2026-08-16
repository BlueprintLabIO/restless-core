//! Authenticated owner transport for the static SPA and persistent desktop.
//!
//! This is intentionally a narrow BFF: owner projection, source-owned
//! approval actions, and browser attach/lease transport. It is not a generic
//! REST facade over the company computer.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use anyhow::{Context as _, Result};
use axum::body::Body;
use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode};
use axum::response::{IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Duration as ChronoDuration, Utc};
use futures_util::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use tokio_tungstenite::{client_async, tungstenite};
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

use crate::{approval, attention, runtime, Daemon};

const OWNER_COOKIE: &str = "restless_owner";
const ATTACH_COOKIE: &str = "restless_attach";
const TICKET_TTL: Duration = Duration::from_secs(30);
const ATTACH_TTL: Duration = Duration::from_secs(30 * 60);
const CONTROL_TTL_SECONDS: i64 = 45;

#[derive(Clone)]
struct OwnerState {
    daemon: Arc<Daemon>,
    tickets: Arc<Mutex<HashMap<String, AttachTicket>>>,
    attaches: Arc<Mutex<HashMap<String, AttachSession>>>,
    secure_cookie: bool,
}

#[derive(Clone)]
struct AttachTicket {
    company: String,
    generation: String,
    item_id: String,
    client_id: String,
    expires_at: SystemTime,
}

#[derive(Clone)]
struct AttachSession {
    company: String,
    client_id: String,
    expires_at: SystemTime,
}

#[derive(Debug, Deserialize)]
struct SignIn {
    token: String,
}

#[derive(Debug, Deserialize)]
struct PartyAction {
    party: String,
}

#[derive(Debug, Deserialize)]
struct TicketRequest {
    item_id: String,
    client_id: String,
}

#[derive(Debug, Serialize)]
struct TicketResponse {
    desktop_url: String,
    expires_in_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct TicketQuery {
    ticket: String,
}

#[derive(Debug, Deserialize)]
struct ControlRequest {
    client_id: String,
}

#[derive(Debug, Deserialize, Default)]
struct DesktopMode {
    client_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: &'static str,
    message: String,
}

pub async fn serve(daemon: Arc<Daemon>) -> Result<()> {
    let address = std::env::var("RESTLESS_OWNER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:7788".to_string())
        .parse::<SocketAddr>()
        .context("parse RESTLESS_OWNER_ADDR")?;
    let secure_cookie = std::env::var("RESTLESS_OWNER_SECURE_COOKIE")
        .map(|value| value != "0" && !value.eq_ignore_ascii_case("false"))
        .unwrap_or(false);
    let state = OwnerState {
        daemon,
        tickets: Arc::new(Mutex::new(HashMap::new())),
        attaches: Arc::new(Mutex::new(HashMap::new())),
        secure_cookie,
    };

    let app = build(state, runtime::source_root()?.join("web/build"));

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("bind owner gateway {address}"))?;
    tracing::info!(addr = %address, "owner gateway listening");
    axum::serve(listener, app).await.context("owner gateway")
}

/// The gateway's whole routing table, separate from the listener so tests can
/// drive the real one. `web` is a parameter only so a test does not need a
/// built SPA on disk; production always passes the source checkout's build.
fn build(state: OwnerState, web: std::path::PathBuf) -> Router {
    let api = Router::new()
        .route("/session", post(sign_in).delete(sign_out))
        .route("/companies/{company}/attention", get(attention_view))
        .route("/companies/{company}/approvals/grant", post(grant))
        .route("/companies/{company}/approvals/decline", post(decline))
        .route("/companies/{company}/approvals/revoke", post(revoke))
        .route("/companies/{company}/browser/ticket", post(issue_ticket))
        .route("/companies/{company}/browser/status", get(browser_status))
        .route("/companies/{company}/browser/take", post(take_control))
        .route("/companies/{company}/browser/heartbeat", post(heartbeat))
        .route("/companies/{company}/browser/return", post(return_control));

    // The cockpit's read/write routes. They are the CLI's line protocol over
    // HTTP and they stamp `principal: "owner"`, so they must sit behind this
    // gateway's credential — they had a loopback listener of their own until
    // that made an unauthenticated port capable of everything an authenticated
    // one was. `api::public` (health, spec, docs) carries no company data and
    // stays outside the gate.
    let v1 = crate::api::guarded(Arc::clone(&state.daemon)).layer(
        axum::middleware::from_fn_with_state(state.clone(), guard_owner),
    );

    let static_files = ServeDir::new(&web).fallback(ServeFile::new(web.join("index.html")));
    Router::new()
        .nest("/api", api)
        .route("/desktop/{company}", get(open_desktop))
        .route("/desktop/{company}/control", get(open_controlled_desktop))
        .route("/desktop/{company}/websockify", get(desktop_websocket))
        .route("/desktop/{company}/{*asset}", get(desktop_asset))
        .fallback_service(static_files)
        .with_state(state)
        .merge(v1)
        .merge(crate::api::public())
}

/// Generate/rotate the one-owner credential. Only its digest is persisted;
/// the returned value is the one moment the owner can copy it.
pub fn rotate_token(root: &Path) -> Result<String> {
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let digest = token_digest(&token);
    let path = root.join("owner-token.sha256");
    let temporary = root.join(".owner-token.sha256.tmp");
    std::fs::write(&temporary, format!("{digest}\n"))
        .with_context(|| format!("write {}", temporary.display()))?;
    std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(&temporary, &path).with_context(|| format!("replace {}", path.display()))?;
    Ok(token)
}

async fn sign_in(
    State(state): State<OwnerState>,
    headers: HeaderMap,
    Json(input): Json<SignIn>,
) -> impl IntoResponse {
    if !valid_token(&state.daemon.root, &input.token) {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "authentication",
            "invalid owner credential",
        );
    }
    let mut cookie = format!(
        "{OWNER_COOKIE}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=43200",
        input.token
    );
    if secure_request(&state, &headers) {
        cookie.push_str("; Secure");
    }
    let mut response = Json(serde_json::json!({ "authenticated": true })).into_response();
    response
        .headers_mut()
        .insert(SET_COOKIE, HeaderValue::from_str(&cookie).expect("cookie"));
    response
}

async fn sign_out() -> impl IntoResponse {
    let mut response = Json(serde_json::json!({ "authenticated": false })).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_static("restless_owner=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"),
    );
    response
}

async fn attention_view(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
    };
    let org = state.daemon.orgintel.get(&company).await.ok();
    match attention::project(&config, &state.daemon.authority, org.as_ref()).await {
        Ok(view) => Json(view).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "projection",
            format!("{error:#}"),
        ),
    }
}

async fn grant(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Json(input): Json<PartyAction>,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    let org = state.daemon.orgintel.get(&company).await.ok();
    match approval::grant(
        &state.daemon.root,
        &company,
        &input.party,
        &state.daemon.authority,
        org.as_ref(),
        "owner",
    )
    .await
    {
        Ok(message) => Json(serde_json::json!({ "message": message })).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "approval", format!("{error:#}")),
    }
}

async fn decline(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Json(input): Json<PartyAction>,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    match approval::decline(
        &state.daemon.root,
        &company,
        &input.party,
        &state.daemon.authority,
        "owner",
    )
    .await
    {
        Ok(message) => Json(serde_json::json!({ "message": message })).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "approval", format!("{error:#}")),
    }
}

async fn revoke(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Json(input): Json<PartyAction>,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    let org = state.daemon.orgintel.get(&company).await.ok();
    match approval::revoke(
        &state.daemon.root,
        &company,
        &input.party,
        &state.daemon.authority,
        org.as_ref(),
        "owner",
    )
    .await
    {
        Ok(message) => Json(serde_json::json!({ "message": message })).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "approval", format!("{error:#}")),
    }
}

async fn issue_ticket(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Json(input): Json<TicketRequest>,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    if Uuid::parse_str(&input.client_id).is_err() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "client",
            "client_id must be a UUID",
        );
    }
    let current = match runtime::generation(&company).await {
        Ok(Some(generation)) => generation,
        _ => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "runtime",
                "company runtime is unavailable",
            )
        }
    };
    if input.item_id != "runtime-rescue" {
        let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
            Ok(config) => config,
            Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
        };
        let org = state.daemon.orgintel.get(&company).await.ok();
        let view = match attention::project(&config, &state.daemon.authority, org.as_ref()).await {
            Ok(view) => view,
            Err(error) => {
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "projection",
                    format!("{error:#}"),
                )
            }
        };
        let Some(item) = view.items.iter().find(|item| item.id == input.item_id) else {
            return api_error(
                StatusCode::NOT_FOUND,
                "attention",
                "attention source is no longer outstanding",
            );
        };
        let Some(reference) = item.runtime_attach.as_ref() else {
            return api_error(
                StatusCode::CONFLICT,
                "runtime",
                "this item has no live runtime attachment",
            );
        };
        if current != reference.generation {
            return api_error(
                StatusCode::CONFLICT,
                "runtime",
                "runtime generation changed; refresh the item",
            );
        }
    }
    let ticket = Uuid::new_v4().simple().to_string();
    state.tickets.lock().expect("ticket registry").insert(
        ticket.clone(),
        AttachTicket {
            company: company.clone(),
            generation: current,
            item_id: input.item_id,
            client_id: input.client_id,
            expires_at: SystemTime::now() + TICKET_TTL,
        },
    );
    Json(TicketResponse {
        desktop_url: format!("/desktop/{company}?ticket={ticket}"),
        expires_in_seconds: TICKET_TTL.as_secs(),
    })
    .into_response()
}

async fn open_desktop(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Query(query): Query<TicketQuery>,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    let ticket = state
        .tickets
        .lock()
        .expect("ticket registry")
        .remove(&query.ticket);
    let Some(ticket) = ticket else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "ticket",
            "attach ticket is invalid or already used",
        );
    };
    if ticket.company != company || ticket.expires_at <= SystemTime::now() {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "ticket",
            "attach ticket is expired or belongs to another company",
        );
    }
    let current = runtime::generation(&company).await.ok().flatten();
    if current.as_deref() != Some(ticket.generation.as_str()) {
        return api_error(
            StatusCode::CONFLICT,
            "runtime",
            "attach ticket names a stale runtime generation",
        );
    }
    let attach = Uuid::new_v4().simple().to_string();
    state.attaches.lock().expect("attach registry").insert(
        attach.clone(),
        AttachSession {
            company: company.clone(),
            client_id: ticket.client_id,
            expires_at: SystemTime::now() + ATTACH_TTL,
        },
    );
    tracing::info!(company, item = %ticket.item_id, "owner desktop attached");
    let target = format!(
        "/desktop/{company}/vnc.html?autoconnect=1&resize=scale&view_only=1&path=desktop/{company}/websockify"
    );
    let mut response = Redirect::to(&target).into_response();
    let secure = if secure_request(&state, &headers) {
        "; Secure"
    } else {
        ""
    };
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&format!(
            "{ATTACH_COOKIE}={attach}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{}",
            ATTACH_TTL.as_secs(),
            secure
        ))
        .expect("attach cookie"),
    );
    response
}

async fn open_controlled_desktop(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Query(query): Query<DesktopMode>,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    let Some(attach) = valid_attach(&state, &company, &headers) else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "desktop attachment is absent or expired",
        );
    };
    let requested = query.client_id.as_deref().unwrap_or(&attach.client_id);
    let control = runtime::read_browser_control(&company).await.ok().flatten();
    let allowed = control.as_ref().is_some_and(|value| {
        value["controller"] == "owner"
            && value["client_id"].as_str() == Some(requested)
            && value["expires_at"]
                .as_str()
                .and_then(|value| value.parse::<chrono::DateTime<Utc>>().ok())
                .is_some_and(|expires| expires > Utc::now())
    });
    if !allowed {
        return api_error(
            StatusCode::CONFLICT,
            "controller",
            "this browser tab does not hold control",
        );
    }
    Redirect::to(&format!(
        "/desktop/{company}/vnc.html?autoconnect=1&resize=scale&view_only=0&path=desktop/{company}/websockify"
    ))
    .into_response()
}

async fn desktop_asset(
    State(state): State<OwnerState>,
    AxumPath((company, asset)): AxumPath<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if require_owner(&state, &headers).is_some()
        || valid_attach(&state, &company, &headers).is_none()
    {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "desktop attachment is absent or expired",
        );
    }
    match runtime::desktop_asset(&company, &asset).await {
        Ok(bytes) => {
            let mut response = Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(bytes))
                .expect("response");
            response.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static(desktop_content_type(&asset)),
            );
            response
                .headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        Err(error) => api_error(
            StatusCode::BAD_GATEWAY,
            "desktop",
            format!("desktop asset bridge: {error:#}"),
        ),
    }
}

fn desktop_content_type(asset: &str) -> &'static str {
    match asset.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

async fn desktop_websocket(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    if valid_attach(&state, &company, &headers).is_none() {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "desktop attachment is absent or expired",
        );
    }
    upgrade
        .on_upgrade(move |socket| async move {
            if let Err(error) = proxy_websocket(socket, &company).await {
                tracing::warn!(company, "desktop websocket ended: {error:#}");
            }
        })
        .into_response()
}

async fn proxy_websocket(browser: WebSocket, company: &str) -> Result<()> {
    let stream = runtime::desktop_stream(company).await?;
    let request = "ws://127.0.0.1:6080/websockify";
    let (runtime, _) = client_async(request, stream).await?;
    let (mut browser_tx, mut browser_rx) = browser.split();
    let (mut runtime_tx, mut runtime_rx) = runtime.split();
    loop {
        tokio::select! {
            incoming = browser_rx.next() => match incoming {
                Some(Ok(message)) => {
                    let translated = match message {
                        AxumMessage::Text(value) => tungstenite::Message::Text(value.to_string().into()),
                        AxumMessage::Binary(value) => tungstenite::Message::Binary(value),
                        AxumMessage::Ping(value) => tungstenite::Message::Ping(value),
                        AxumMessage::Pong(value) => tungstenite::Message::Pong(value),
                        AxumMessage::Close(_) => break,
                    };
                    runtime_tx.send(translated).await?;
                }
                _ => break,
            },
            incoming = runtime_rx.next() => match incoming {
                Some(Ok(message)) => {
                    let translated = match message {
                        tungstenite::Message::Text(value) => AxumMessage::Text(value.to_string().into()),
                        tungstenite::Message::Binary(value) => AxumMessage::Binary(value),
                        tungstenite::Message::Ping(value) => AxumMessage::Ping(value),
                        tungstenite::Message::Pong(value) => AxumMessage::Pong(value),
                        tungstenite::Message::Close(_) => break,
                        tungstenite::Message::Frame(_) => continue,
                    };
                    browser_tx.send(translated).await?;
                }
                _ => break,
            }
        }
    }
    Ok(())
}

async fn browser_status(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    match runtime::doctor(&company).await {
        Ok(report) => Json(serde_json::json!({
            "generation": runtime::generation(&company).await.ok().flatten(),
            "browser": report.browser,
            "control": runtime::read_browser_control(&company).await.ok().flatten(),
        }))
        .into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime",
            format!("{error:#}"),
        ),
    }
}

async fn take_control(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Json(input): Json<ControlRequest>,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    let Some(attach) = valid_attach(&state, &company, &headers) else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "open this runtime attachment before taking control",
        );
    };
    if attach.client_id != input.client_id {
        return api_error(
            StatusCode::FORBIDDEN,
            "controller",
            "attachment belongs to another browser tab",
        );
    }
    let prior = runtime::read_browser_control(&company).await.ok().flatten();
    if prior.as_ref().is_some_and(|value| {
        value["controller"] == "owner"
            && value["client_id"].as_str() != Some(input.client_id.as_str())
            && lease_is_live(value)
    }) {
        return api_error(
            StatusCode::CONFLICT,
            "controller",
            "another owner tab already controls this browser",
        );
    }
    let requester = prior.as_ref().and_then(|value| {
        value["requester"]
            .as_str()
            .or_else(|| value["session_id"].as_str())
            .map(str::to_string)
    });
    let state_value = serde_json::json!({
        "controller": "owner",
        "client_id": input.client_id,
        "requester": requester,
        "acquired_at": Utc::now(),
        "expires_at": Utc::now() + ChronoDuration::seconds(CONTROL_TTL_SECONDS),
    });
    match runtime::write_browser_control(&company, &state_value).await {
        Ok(()) => Json(state_value).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime",
            format!("{error:#}"),
        ),
    }
}

async fn heartbeat(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Json(input): Json<ControlRequest>,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    let Some(mut current) = runtime::read_browser_control(&company).await.ok().flatten() else {
        return api_error(
            StatusCode::CONFLICT,
            "controller",
            "browser is not owner-controlled",
        );
    };
    if current["controller"] != "owner" || current["client_id"].as_str() != Some(&input.client_id) {
        return api_error(
            StatusCode::CONFLICT,
            "controller",
            "this browser tab does not hold control",
        );
    }
    current["expires_at"] =
        serde_json::json!(Utc::now() + ChronoDuration::seconds(CONTROL_TTL_SECONDS));
    match runtime::write_browser_control(&company, &current).await {
        Ok(()) => Json(current).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime",
            format!("{error:#}"),
        ),
    }
}

async fn return_control(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Json(input): Json<ControlRequest>,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    let Some(current) = runtime::read_browser_control(&company).await.ok().flatten() else {
        return api_error(
            StatusCode::CONFLICT,
            "controller",
            "browser has no controller lease",
        );
    };
    if current["controller"] != "owner" || current["client_id"].as_str() != Some(&input.client_id) {
        return api_error(
            StatusCode::CONFLICT,
            "controller",
            "this browser tab does not hold control",
        );
    }
    let requester = current["requester"].as_str().map(str::to_string);
    let next = match requester.as_deref() {
        Some(requester) => serde_json::json!({
            "controller": "agent",
            "session_id": requester,
            "returned_at": Utc::now(),
        }),
        None => serde_json::json!({ "controller": "unclaimed", "returned_at": Utc::now() }),
    };
    if let Err(error) = runtime::write_browser_control(&company, &next).await {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime",
            format!("{error:#}"),
        );
    }
    if let Some(requester) = requester {
        if let Ok(org) = state.daemon.orgintel.get(&company).await {
            let _ = org.add_actor("owner", "owner", "The Owner").await;
            let _ = org.add_actor(&requester, "staff", &requester).await;
            let _ = org
                .send_message(
                    "owner",
                    Some(&requester),
                    "Browser control returned. Inspect the same page state and verify the source condition; hand-back is not proof of completion.",
                )
                .await;
        }
    }
    Json(next).into_response()
}

/// `require_owner` as a layer, for the `/v1` routes. They are written as a thin
/// transport over `dispatch_value` and know nothing about cookies; the gate is
/// applied to all of them at once here so a new route cannot be added without
/// one by forgetting a line.
async fn guard_owner(
    State(state): State<OwnerState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response<Body> {
    if let Some(refusal) = require_owner(&state, request.headers()) {
        return refusal;
    }
    next.run(request).await.into_response()
}

fn require_owner(state: &OwnerState, headers: &HeaderMap) -> Option<Response<Body>> {
    let token = cookie(headers, OWNER_COOKIE).unwrap_or_default();
    if valid_token(&state.daemon.root, &token) {
        None
    } else {
        Some(api_error(
            StatusCode::UNAUTHORIZED,
            "authentication",
            "owner sign-in required",
        ))
    }
}

fn secure_request(state: &OwnerState, headers: &HeaderMap) -> bool {
    state.secure_cookie
        || headers
            .get("x-forwarded-proto")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.eq_ignore_ascii_case("https"))
}

fn valid_attach(state: &OwnerState, company: &str, headers: &HeaderMap) -> Option<AttachSession> {
    let id = cookie(headers, ATTACH_COOKIE)?;
    let mut attaches = state.attaches.lock().expect("attach registry");
    attaches.retain(|_, attach| attach.expires_at > SystemTime::now());
    attaches
        .get(&id)
        .filter(|attach| attach.company == company)
        .cloned()
}

fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|pair| {
            let (key, value) = pair.trim().split_once('=')?;
            (key == name).then(|| value.to_string())
        })
}

fn valid_token(root: &Path, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    let stored = std::fs::read_to_string(root.join("owner-token.sha256"))
        .ok()
        .map(|value| value.trim().to_string())
        .unwrap_or_default();
    constant_time_eq(stored.as_bytes(), token_digest(token).as_bytes())
}

fn token_digest(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
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

fn lease_is_live(value: &serde_json::Value) -> bool {
    value["expires_at"]
        .as_str()
        .and_then(|value| value.parse::<chrono::DateTime<Utc>>().ok())
        .is_some_and(|expires| expires > Utc::now())
}

fn api_error(
    status: StatusCode,
    error: &'static str,
    message: impl Into<String>,
) -> Response<Body> {
    let body = serde_json::to_vec(&ErrorResponse {
        error,
        message: message.into(),
    })
    .expect("error json");
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("error response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use tower::ServiceExt as _;

    #[test]
    fn digest_comparison_is_exact() {
        let digest = token_digest("right-token");
        assert!(constant_time_eq(
            digest.as_bytes(),
            token_digest("right-token").as_bytes()
        ));
        assert!(!constant_time_eq(
            digest.as_bytes(),
            token_digest("wrong-token").as_bytes()
        ));
    }

    fn gateway(root: &std::path::Path) -> Router {
        build(
            OwnerState {
                daemon: crate::test_daemon(root),
                tickets: Arc::new(Mutex::new(HashMap::new())),
                attaches: Arc::new(Mutex::new(HashMap::new())),
                secure_cookie: false,
            },
            root.join("no-spa-here"),
        )
    }

    async fn status_of(root: &std::path::Path, method: &str, uri: &str) -> StatusCode {
        gateway(root)
            .oneshot(
                HttpRequest::builder()
                    .method(method)
                    .uri(uri)
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response")
            .status()
    }

    /// The reason the cockpit's own listener was removed. Every `/v1` route
    /// stamps `principal: "owner"`, so an uncredentialled request that reached
    /// one could act as the owner — which is what a second, unauthenticated
    /// port made possible while it existed.
    ///
    /// Written as a sweep over reads *and* writes rather than one example,
    /// because the failure this guards against is a route added outside the
    /// layer, and a single-route test would not notice.
    #[tokio::test]
    async fn no_v1_route_answers_without_the_owner_cookie() {
        let dir = tempfile::tempdir().expect("tempdir");
        for (method, uri) in [
            ("GET", "/v1/companies/acme/goals"),
            ("GET", "/v1/companies/acme/people"),
            ("GET", "/v1/companies/acme/inbox"),
            ("GET", "/v1/companies/acme/spend"),
            ("GET", "/v1/companies/acme/authority"),
            ("POST", "/v1/companies/acme/tell"),
            ("POST", "/v1/companies/acme/wake"),
            ("POST", "/v1/companies/acme/up"),
            ("POST", "/v1/companies"),
        ] {
            assert_eq!(
                status_of(dir.path(), method, uri).await,
                StatusCode::UNAUTHORIZED,
                "{method} {uri} answered without a credential"
            );
        }
    }

    /// The gateway's own routes were already gated; asserted here so the two
    /// surfaces cannot drift into different answers for the same question.
    #[tokio::test]
    async fn the_attention_queue_needs_the_owner_cookie() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(
            status_of(dir.path(), "GET", "/api/companies/acme/attention").await,
            StatusCode::UNAUTHORIZED
        );
    }

    /// Health must not need a credential, or nothing can check it — the start
    /// script polls it, and a gate here would make "is it up?" unanswerable.
    #[tokio::test]
    async fn health_and_the_spec_stay_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        for uri in ["/v1/health", "/v1/openapi.yaml", "/v1/docs"] {
            assert_eq!(
                status_of(dir.path(), "GET", uri).await,
                StatusCode::OK,
                "{uri} must answer without a credential"
            );
        }
    }
}
