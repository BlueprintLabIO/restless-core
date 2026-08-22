//! Loopback-only owner transport for the static SPA and persistent desktop.
//!
//! This is intentionally a narrow BFF: owner projection, source-owned
//! approval actions, and browser attach/lease transport. It is not a generic
//! REST facade over the company computer.

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use anyhow::{Context as _, Result};
use axum::body::Body;
use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{
    DefaultBodyLimit, Multipart, OriginalUri, Path as AxumPath, Query, Request, State,
};
use axum::http::header::{
    CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE, COOKIE, HOST, ORIGIN, SET_COOKIE,
};
use axum::http::uri::Authority;
use axum::http::{HeaderMap, HeaderValue, Method, Response, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Redirect};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use chrono::{Duration as ChronoDuration, Utc};
use futures_util::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{client_async, tungstenite};
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

use crate::{
    airwallex, approval, attention, authority, company as company_projection, credential, finance,
    legal, reconcile, runtime, Daemon,
};

const ATTACH_COOKIE: &str = "restless_attach";
const TICKET_TTL: Duration = Duration::from_secs(30);
const ATTACH_TTL: Duration = Duration::from_secs(30 * 60);
const REVIEW_TTL: Duration = Duration::from_secs(30 * 60);
const CONTROL_TTL_SECONDS: i64 = 45;
const MAX_ATTACHMENTS: usize = 6;
const MAX_ATTACHMENT_BYTES: usize = 5 * 1024 * 1024;
const ATTACHMENT_BLOCK: &str = "\n\n[Restless attachments]\n";
const ATTACHMENT_MARKER: &str = "<!--restless-attachments:";
const INTENT_MARKER: &str = "\n\n<!--restless-intent:";
const DETAILS_MARKER: &str = "\n\n<!--restless-details:";
const CONTEXT_BLOCK: &str = "\n\n[Owner cockpit context]\n";
const CONTEXT_MARKER: &str = "\n\n<!--restless-context:";

#[derive(Clone)]
struct OwnerState {
    daemon: Arc<Daemon>,
    charter_writes: Arc<tokio::sync::Mutex<()>>,
    tickets: Arc<Mutex<HashMap<String, AttachTicket>>>,
    attaches: Arc<Mutex<HashMap<String, AttachSession>>>,
    reviews: Arc<Mutex<HashMap<String, ReviewSession>>>,
    review_public_url: String,
}

#[derive(Clone)]
pub(crate) struct OwnerConfig {
    address: SocketAddr,
    review_address: SocketAddr,
    review_public_url: String,
}

#[derive(Clone)]
struct AttachTicket {
    company: String,
    generation: String,
    item_id: String,
    client_id: String,
    requesting_actor: Option<String>,
    expires_at: SystemTime,
}

#[derive(Clone)]
struct AttachSession {
    company: String,
    client_id: String,
    requesting_actor: Option<String>,
    expires_at: SystemTime,
}

#[derive(Clone)]
struct ReviewSession {
    company: String,
    generation: String,
    item_id: String,
    port: u16,
    expected_host: String,
    expires_at: SystemTime,
}

#[derive(Debug, Deserialize)]
struct PartyAction {
    party: String,
}

#[derive(Default)]
struct OwnerMessageInput {
    body: String,
    work_id: Option<Uuid>,
    new_focus: bool,
    context_requested: bool,
    context_path: Option<String>,
    attachments: Vec<PendingAttachment>,
}

struct PendingAttachment {
    name: String,
    media_type: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerAttachment {
    upload_id: Uuid,
    name: String,
    media_type: String,
    size_bytes: usize,
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OwnerIntentKind {
    Conversation,
    WorkFeedback,
    Direction,
    Authority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OwnerIntentReceipt {
    kind: OwnerIntentKind,
    summary: String,
}

#[derive(Debug, Deserialize)]
struct OwnerMessageDetails {
    markdown: String,
}

#[derive(Debug, Deserialize, Default)]
struct ConversationQuery {
    work_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct ConversationLiveQuery {
    message_id: i64,
}

#[derive(Debug, Deserialize, Default)]
struct CockpitQuery {
    #[serde(default)]
    probe_credentials: bool,
}

#[derive(Debug, Deserialize)]
struct CompanyRecoveryInput {
    action: String,
}

#[derive(Debug, Deserialize)]
struct CharterRevisionInput {
    markdown: String,
    base_revision: String,
}

#[derive(Debug, Serialize)]
struct CharterRevisionResponse {
    company: company_projection::CompanyView,
    #[serde(flatten)]
    revision: authority::MandateRevisionOutcome,
}

#[derive(Debug, Deserialize)]
struct OwnerReviewInput {
    decision: String,
    #[serde(default)]
    feedback: String,
}

#[derive(Debug, Deserialize)]
struct OwnerHandoffDecisionInput {
    resolution: String,
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
struct ReviewTicketRequest {
    item_id: String,
}

#[derive(Debug, Serialize)]
struct ReviewTicketResponse {
    review_url: String,
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

#[derive(Debug, Serialize)]
struct CompanyCatalogEntry {
    id: String,
    name: String,
    mission: String,
    model: String,
    spend_ceiling_usd: f64,
    runtime_status: &'static str,
    lifecycle_status: &'static str,
}

impl OwnerConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let address = std::env::var("RESTLESS_OWNER_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7788".to_string())
            .parse::<SocketAddr>()
            .context("parse RESTLESS_OWNER_ADDR")?;
        let review_address = std::env::var("RESTLESS_REVIEW_ADDR")
            // 7788 is the owner gateway, 7789 the auth broker, 7790 the model
            // gateway, 7791 coordination, 7792 ingress and 7793 Infisical.
            .unwrap_or_else(|_| "127.0.0.1:7794".to_string())
            .parse::<SocketAddr>()
            .context("parse RESTLESS_REVIEW_ADDR")?;
        ensure_loopback(address, "RESTLESS_OWNER_ADDR")?;
        ensure_loopback(review_address, "RESTLESS_REVIEW_ADDR")?;
        let review_public_url = std::env::var("RESTLESS_REVIEW_PUBLIC_URL")
            .unwrap_or_else(|_| format!("http://{{ticket}}.localhost:{}", review_address.port()));
        validate_review_public_url(&review_public_url, review_address.port())?;
        Ok(Self {
            address,
            review_address,
            review_public_url,
        })
    }
}

fn ensure_loopback(address: SocketAddr, variable: &str) -> Result<()> {
    if !address.ip().is_loopback() {
        anyhow::bail!("{variable} must remain loopback-only until network authentication exists");
    }
    Ok(())
}

pub async fn serve(daemon: Arc<Daemon>, config: OwnerConfig) -> Result<()> {
    let OwnerConfig {
        address,
        review_address,
        review_public_url,
    } = config;
    let state = OwnerState {
        daemon,
        charter_writes: Arc::new(tokio::sync::Mutex::new(())),
        tickets: Arc::new(Mutex::new(HashMap::new())),
        attaches: Arc::new(Mutex::new(HashMap::new())),
        reviews: Arc::new(Mutex::new(HashMap::new())),
        review_public_url,
    };

    let api = Router::new()
        .route("/companies", get(company_catalog))
        .route("/companies/{company}/archive", post(archive_company))
        .route("/companies/{company}/restore", post(restore_company))
        .route("/companies/{company}/attention", get(attention_view))
        .route("/companies/{company}/cockpit", get(cockpit_view))
        .route("/companies/{company}/company", get(company_view))
        .route(
            "/companies/{company}/company/charter",
            post(revise_company_charter),
        )
        .route(
            "/companies/{company}/company/recover",
            post(recover_company_computer),
        )
        .route(
            "/companies/{company}/actors/{actor}/conversation",
            get(actor_conversation).post(send_actor_message),
        )
        .route(
            "/companies/{company}/actors/{actor}/conversation/live",
            get(actor_conversation_live),
        )
        .route(
            "/companies/{company}/attachments/{attachment}",
            get(download_attachment),
        )
        .route(
            "/companies/{company}/handoffs/{handoff}/review",
            post(review_outcome),
        )
        .route(
            "/companies/{company}/handoffs/{handoff}/decision",
            post(resolve_handoff_decision),
        )
        .route("/companies/{company}/approvals/grant", post(grant))
        .route("/companies/{company}/approvals/decline", post(decline))
        .route("/companies/{company}/approvals/revoke", post(revoke))
        .route("/companies/{company}/browser/ticket", post(issue_ticket))
        .route(
            "/companies/{company}/reviews/ticket",
            post(issue_review_ticket),
        )
        .route("/companies/{company}/browser/status", get(browser_status))
        .route("/companies/{company}/browser/take", post(take_control))
        .route("/companies/{company}/browser/heartbeat", post(heartbeat))
        .route("/companies/{company}/browser/return", post(return_control))
        .fallback(api_not_found)
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024));

    let source = runtime::source_root()?;
    let web = source.join("web/build");
    let static_files = ServeDir::new(&web).fallback(ServeFile::new(web.join("index.html")));
    let app = Router::new()
        .nest("/api", api)
        .route("/desktop/{company}", get(open_desktop))
        .route("/desktop/{company}/observe", get(open_observed_desktop))
        .route("/desktop/{company}/control", get(open_controlled_desktop))
        .route("/desktop/{company}/websockify", get(desktop_websocket))
        .route("/desktop/{company}/{*asset}", get(desktop_asset))
        .fallback_service(static_files)
        .with_state(state.clone())
        .layer(middleware::from_fn(enforce_local_owner_boundary));

    // A separate origin is load-bearing. Reviewed company code may run
    // JavaScript and use root-relative assets, but it never shares the owner
    // cockpit origin or its authority boundary.
    let preview = Router::new().fallback(any(review_proxy)).with_state(state);

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("bind owner gateway {address}"))?;
    let preview_listener = tokio::net::TcpListener::bind(review_address)
        .await
        .with_context(|| format!("bind review gateway {review_address}"))?;
    tracing::info!(addr = %address, "owner gateway listening");
    tracing::info!(addr = %review_address, "isolated review gateway listening");
    tokio::try_join!(
        axum::serve(listener, app),
        axum::serve(preview_listener, preview)
    )
    .map(|_| ())
    .context("owner gateways")
}

async fn enforce_local_owner_boundary(request: Request, next: Next) -> Response<Body> {
    if let Some(reason) = local_owner_boundary_violation(request.method(), request.headers()) {
        return api_error(StatusCode::FORBIDDEN, "local_owner_boundary", reason);
    }
    next.run(request).await
}

fn local_owner_boundary_violation(method: &Method, headers: &HeaderMap) -> Option<&'static str> {
    for forwarded in [
        "forwarded",
        "x-forwarded-for",
        "x-forwarded-host",
        "x-forwarded-proto",
        "x-real-ip",
    ] {
        if headers.contains_key(forwarded) {
            return Some("forwarded owner requests require network authentication");
        }
    }

    let host = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(local_authority);
    let Some(host) = host else {
        return Some("owner request host is not the configured loopback origin");
    };

    if let Some(site) = headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
    {
        if !site.eq_ignore_ascii_case("same-origin") && !site.eq_ignore_ascii_case("none") {
            return Some("cross-site owner requests are refused");
        }
    }

    let origin = headers.get(ORIGIN).and_then(|value| value.to_str().ok());
    if !matches!(*method, Method::GET | Method::HEAD) && origin.is_none() {
        return Some("state-changing owner requests require a same-origin browser origin");
    }
    if let Some(origin) = origin {
        if local_origin(origin).as_ref() != Some(&host) {
            return Some("owner request origin does not match its loopback host");
        }
    }
    None
}

fn local_authority(value: &str) -> Option<(String, u16)> {
    let authority = value.parse::<Authority>().ok()?;
    let port = authority.port_u16().unwrap_or(80);
    local_host(authority.host()).map(|host| (host, port))
}

fn local_origin(value: &str) -> Option<(String, u16)> {
    let origin = url::Url::parse(value).ok()?;
    if origin.scheme() != "http"
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.path() != "/"
        || origin.query().is_some()
        || origin.fragment().is_some()
    {
        return None;
    }
    let port = origin.port_or_known_default()?;
    local_host(origin.host_str()?).map(|host| (host, port))
}

fn local_host(value: &str) -> Option<String> {
    if value.eq_ignore_ascii_case("localhost") {
        return Some("localhost".to_string());
    }
    let unbracketed = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .unwrap_or(value);
    unbracketed
        .parse::<IpAddr>()
        .ok()
        .filter(IpAddr::is_loopback)
        .map(|ip| ip.to_string())
}

async fn company_catalog(State(state): State<OwnerState>) -> impl IntoResponse {
    let companies = match crate::configured_companies(&state.daemon.root) {
        Ok(companies) => companies,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "company",
                format!("{error:#}"),
            )
        }
    };
    let archived = match runtime::archived_company_names(&state.daemon.root) {
        Ok(companies) => companies,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "company",
                format!("{error:#}"),
            )
        }
    };
    let mut catalog = Vec::with_capacity(companies.len() + archived.len());
    for company in companies {
        let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
            Ok(config) => config,
            Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
        };
        catalog.push(company_catalog_entry(config, "active").await);
    }
    for company in archived {
        let config = match runtime::CompanyConfig::load_archived(&state.daemon.root, &company) {
            Ok(config) => config,
            Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
        };
        catalog.push(company_catalog_entry(config, "archived").await);
    }
    Json(catalog).into_response()
}

async fn company_catalog_entry(
    config: runtime::CompanyConfig,
    lifecycle_status: &'static str,
) -> CompanyCatalogEntry {
    let runtime_status = match runtime::status(&config.name).await {
        Ok(runtime::ContainerStatus::Running) => "running",
        Ok(runtime::ContainerStatus::Stopped) => "stopped",
        Ok(runtime::ContainerStatus::Absent) => "absent",
        Err(_) => "unavailable",
    };
    CompanyCatalogEntry {
        id: config.name.clone(),
        name: company_display_name(&config.name),
        mission: config.mission,
        model: config.model,
        spend_ceiling_usd: config.spend_ceiling_usd,
        runtime_status,
        lifecycle_status,
    }
}

async fn archive_company(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
) -> impl IntoResponse {
    if runtime::CompanyConfig::load_archived(&state.daemon.root, &company).is_ok() {
        return Json(serde_json::json!({
            "company": company,
            "lifecycle_status": "archived",
            "changed": false,
        }))
        .into_response();
    }
    if let Err(error) = runtime::CompanyConfig::load(&state.daemon.root, &company) {
        return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}"));
    }
    let message = match runtime::archive(&state.daemon.root, &company).await {
        Ok(message) => message,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "lifecycle",
                format!("{error:#}"),
            )
        }
    };
    if let Err(error) = state
        .daemon
        .authority
        .emit(
            &company,
            "lifecycle",
            Some("owner"),
            serde_json::json!({ "state": "archived", "message": message }),
        )
        .await
    {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "authority",
            format!("company archived but its lifecycle receipt could not be recorded: {error:#}"),
        );
    }
    Json(serde_json::json!({
        "company": company,
        "lifecycle_status": "archived",
        "changed": true,
    }))
    .into_response()
}

async fn restore_company(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
) -> impl IntoResponse {
    if runtime::CompanyConfig::load(&state.daemon.root, &company).is_ok() {
        return Json(serde_json::json!({
            "company": company,
            "lifecycle_status": "active",
            "changed": false,
        }))
        .into_response();
    }
    if let Err(error) = runtime::CompanyConfig::load_archived(&state.daemon.root, &company) {
        return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}"));
    }
    if let Err(error) = runtime::restore(&state.daemon.root, &company) {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "lifecycle",
            format!("{error:#}"),
        );
    }
    if let Err(error) = state
        .daemon
        .authority
        .emit(
            &company,
            "lifecycle",
            Some("owner"),
            serde_json::json!({
                "state": "stopped",
                "restored_from": "archived",
            }),
        )
        .await
    {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "authority",
            format!("company restored but its lifecycle receipt could not be recorded: {error:#}"),
        );
    }
    Json(serde_json::json!({
        "company": company,
        "lifecycle_status": "active",
        "runtime_status": "stopped",
        "changed": true,
    }))
    .into_response()
}

async fn attention_view(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
) -> impl IntoResponse {
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

async fn company_view(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<CockpitQuery>,
) -> impl IntoResponse {
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
    };
    Json(company_projection::project(&state.daemon, &config, query.probe_credentials).await)
        .into_response()
}

async fn revise_company_charter(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<CharterRevisionInput>,
) -> impl IntoResponse {
    if let Err(error) = authority::validate_mandate(&input.markdown) {
        return api_error(StatusCode::BAD_REQUEST, "charter", format!("{error:#}"));
    }

    // One local owner may still have several tabs. Serialising this bounded
    // read/compare/write section makes base_revision a real precondition
    // instead of two simultaneous saves both passing the same read.
    let _write = state.charter_writes.lock().await;
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
    };
    let current_revision = authority::mandate_revision(&config.mission);
    if input.base_revision != current_revision {
        return api_error(
            StatusCode::CONFLICT,
            "charter_revision",
            "The charter changed after this editor opened. Your draft is preserved; refresh the source before saving again.",
        );
    }

    let revision = match authority::revise_mandate(
        &state.daemon.authority,
        &state.daemon.root,
        config,
        input.markdown,
    )
    .await
    {
        Ok(outcome) => outcome,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "charter",
                format!("{error:#}"),
            )
        }
    };
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "charter",
                format!("charter changed but its canonical config could not be reread: {error:#}"),
            )
        }
    };
    Json(CharterRevisionResponse {
        company: company_projection::project(&state.daemon, &config, false).await,
        revision,
    })
    .into_response()
}

async fn recover_company_computer(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<CompanyRecoveryInput>,
) -> impl IntoResponse {
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
    };
    let action = match company_projection::RecoveryAction::parse(&input.action) {
        Some(action) => action,
        None => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "recovery",
                "action must be start, restart or reconcile",
            )
        }
    };
    match company_projection::recover(&state.daemon, &config, action).await {
        Ok(outcome) => Json(outcome).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "recovery",
            format!("{error:#}"),
        ),
    }
}

/// The owner cockpit's cross-plane read. This is deliberately an aggregation
/// at the presentation boundary, not a second writer: each field is read from
/// the plane that owns it and carries explicit source health when that plane
/// cannot answer. Authority remains readable when recoverable OrgIntel is
/// unavailable.
async fn cockpit_view(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<CockpitQuery>,
) -> impl IntoResponse {
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
    };
    let observed_at = Utc::now();

    let mut source_health = serde_json::Map::new();
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => {
            source_health.insert("orgintel".into(), serde_json::json!("available"));
            Some(org)
        }
        Err(error) => {
            source_health.insert(
                "orgintel".into(),
                serde_json::json!(format!("unavailable: {error}")),
            );
            None
        }
    };

    let (actors, teams, goals, work) = if let Some(org) = org.as_ref() {
        match tokio::try_join!(
            org.list_actors(),
            org.list_teams(),
            org.list_goals(),
            org.list_work()
        ) {
            Ok((actors, teams, goals, work)) => (actors, teams, goals, work),
            Err(error) => {
                source_health.insert(
                    "orgintel".into(),
                    serde_json::json!(format!("unavailable: {error}")),
                );
                (Vec::new(), Vec::new(), Vec::new(), Vec::new())
            }
        }
    } else {
        (Vec::new(), Vec::new(), Vec::new(), Vec::new())
    };

    let spend_breakdown = state.daemon.spend.breakdown(&company);
    let accounted_usd: f64 = spend_breakdown.iter().map(|(_, _, usd)| usd).sum();
    let poisoned = state.daemon.spend.spent_usd(&company) > 1_000_000_000.0;
    let cooldowns = state
        .daemon
        .authority
        .active_model_cooldowns(&company)
        .await
        .unwrap_or_default();
    let people = actors
        .iter()
        .filter(|actor| actor.kind != "system")
        .map(|actor| {
            let spent: f64 = spend_breakdown
                .iter()
                .filter(|(id, _, _)| id == &actor.id)
                .map(|(_, _, usd)| usd)
                .sum();
            let conversation_running = actor.id == "exec"
                && state
                    .daemon
                    .in_flight
                    .lock()
                    .map(|running| running.is_active(&company))
                    .unwrap_or(false);
            // Exec is also a legitimate Work owner. Its free-form wake and
            // graph-claimed Attempt are mutually exclusive in the scheduler,
            // but either is observed activity on People.
            let session_running =
                conversation_running || state.daemon.staff.is_actor_running(&company, &actor.id);
            serde_json::json!({
                "actor_id": actor.id,
                "kind": actor.kind,
                "role": actor.role,
                "display": actor.display,
                "model": actor.model,
                "team_id": actor.team_id,
                "spent_usd": round_owner_usd(spent),
                "session_running": session_running,
                "session_observed_at": session_running.then_some(observed_at),
                "model_cooldown": actor.model.as_deref().and_then(|model| {
                    cooldowns.iter().find(|cooldown| cooldown.model == model)
                }),
            })
        })
        .collect::<Vec<_>>();

    // Team structure is read from OrgIntel, never reconstructed from role names
    // or Work titles in the browser. Standing company/system actors remain
    // outside teams even if a malformed row happens to point at one.
    let actor_teams = actors
        .iter()
        .filter(|actor| actor.kind == "staff")
        .filter_map(|actor| actor.team_id.map(|team_id| (actor.id.as_str(), team_id)))
        .collect::<HashMap<_, _>>();
    let team_rows = teams
        .iter()
        .map(|team| {
            let member_count = actor_teams
                .values()
                .filter(|team_id| **team_id == team.id)
                .count();
            let in_motion_count = work
                .iter()
                .filter(|item| {
                    item.status == restless_orgintel::WorkStatus::Active
                        && actor_teams.get(item.owner_id.as_str()) == Some(&team.id)
                })
                .count();
            let blocked_count = work
                .iter()
                .filter(|item| {
                    item.status == restless_orgintel::WorkStatus::Blocked
                        && actor_teams.get(item.owner_id.as_str()) == Some(&team.id)
                })
                .count();
            serde_json::json!({
                "id": team.id,
                "name": team.name,
                "brief": team.brief,
                "lead_actor_id": team.lead_actor_id,
                "created_by": team.created_by,
                "created_at": team.created_at,
                "member_count": member_count,
                "in_motion_count": in_motion_count,
                "blocked_count": blocked_count,
            })
        })
        .collect::<Vec<_>>();

    let approved_parties = match approval::approved_parties(&state.daemon.authority, &company).await
    {
        Ok(parties) => {
            source_health.insert("authority".into(), serde_json::json!("available"));
            parties
        }
        Err(error) => {
            source_health.insert(
                "authority".into(),
                serde_json::json!(format!("unavailable: {error}")),
            );
            Vec::new()
        }
    };

    let receipts = match state
        .daemon
        .authority
        .records_of_kind(&company, "effect")
        .await
    {
        Ok(events) => events
            .iter()
            .rev()
            .take(50)
            .map(|event| {
                serde_json::json!({
                    "id": event.id,
                    "effect_class": event.body.get("effect_class").or_else(|| event.body.get("capability")),
                    "tool": event.body.get("tool"),
                    "success": event.body.get("success"),
                    "party": event.body.get("party"),
                    "actor": event.body.get("actor").cloned().or_else(|| event.actor_id.clone().map(serde_json::Value::String)),
                    "outcome": event.body.get("outcome"),
                    "evidence_quality": if reconcile::is_governed_receipt(&event.body) {
                        "governed"
                    } else {
                        "legacy_unverified"
                    },
                    "at": event.created_at,
                })
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            source_health.insert(
                "authority".into(),
                serde_json::json!(format!("unavailable: {error}")),
            );
            Vec::new()
        }
    };

    let mut credentials = Vec::with_capacity(config.credentials.len());
    for (binding, reference) in &config.credentials {
        if query.probe_credentials {
            let probe = credential::probe_reference(reference).await;
            credentials.push(serde_json::json!({
                "binding": binding,
                "status": probe.status.as_str(),
                "detail": probe.detail,
            }));
        } else {
            credentials.push(serde_json::json!({
                "binding": binding,
                "status": "configured_unprobed",
                "detail": "A governed reference is configured. Availability was not probed by this read.",
            }));
        }
    }

    let legal_profile = match legal::get_profile(&state.daemon.authority, &company).await {
        Ok(profile) => serde_json::json!({
            "status": "available",
            "profile": profile,
        }),
        Err(error) => serde_json::json!({
            "status": "unavailable",
            "detail": format!("{error:#}"),
            "profile": null,
        }),
    };
    let provider = match airwallex::connection(&state.daemon.authority, &company).await {
        Ok(connection) => serde_json::json!({
            "status": "available",
            "connection": connection.map(|connection| serde_json::json!({
                "environment": connection.configured.environment,
                "account_ref": connection.configured.account_ref,
                "api_version": connection.configured.api_version,
                "read_scopes": connection.configured.read_scopes,
                "submit_scopes": connection.configured.submit_scopes,
                "approval_workflow_observed": connection.configured.approval_workflow_observed,
                "observed_at": connection.configured.observed_at,
                "updated_at": connection.updated_at,
            })),
        }),
        Err(error) => serde_json::json!({
            "status": "unavailable",
            "detail": format!("{error:#}"),
            "connection": null,
        }),
    };
    let finance_state = match tokio::try_join!(
        finance::envelopes(&state.daemon.authority, &company),
        finance::payments(&state.daemon.authority, &company),
        state
            .daemon
            .authority
            .records_of_kind(&company, "finance_balance_observed")
    ) {
        Ok((envelopes, payments, balances)) => serde_json::json!({
            "status": "available",
            "envelopes": envelopes,
            "payments": payments,
            "last_balance_observation": balances.last().map(|row| serde_json::json!({
                "observed_at": row.created_at,
                "body": row.body,
            })),
        }),
        Err(error) => serde_json::json!({
            "status": "unavailable",
            "detail": format!("{error:#}"),
            "envelopes": [],
            "payments": [],
            "last_balance_observation": null,
        }),
    };

    let runtime_status = match runtime::status(&company).await {
        Ok(runtime::ContainerStatus::Running) => "running",
        Ok(runtime::ContainerStatus::Stopped) => "stopped",
        Ok(runtime::ContainerStatus::Absent) => "absent",
        Err(_) => "unavailable",
    };
    source_health.insert("runtime".into(), serde_json::json!(runtime_status));

    Json(serde_json::json!({
        "company": {
            "id": company,
            "name": config.name,
            "mission": config.mission,
            "model": config.model,
        },
        "source_health": source_health,
        "people": people,
        "teams": team_rows,
        "goals": goals,
        "spend": {
            "accounted_usd": round_owner_usd(accounted_usd),
            "ceiling_usd": config.spend_ceiling_usd,
            "remaining_usd": if poisoned {
                serde_json::Value::Null
            } else {
                serde_json::json!(round_owner_usd((config.spend_ceiling_usd - accounted_usd).max(0.0)))
            },
            "poisoned": poisoned,
        },
        "authority": {
            "approved_parties": approved_parties,
            "credentials": credentials,
            "legal": legal_profile,
            "provider": provider,
            "finance": finance_state,
        },
        "receipts": receipts,
        "refreshed_at": observed_at,
    }))
    .into_response()
}

fn round_owner_usd(usd: f64) -> f64 {
    let rounded = (usd * 10_000.0).round() / 10_000.0;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

async fn actor_conversation(
    State(state): State<OwnerState>,
    AxumPath((company, actor)): AxumPath<(String, String)>,
    Query(query): Query<ConversationQuery>,
) -> impl IntoResponse {
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    let actor_row = match org.list_actors().await {
        Ok(actors) => actors.into_iter().find(|row| row.id == actor),
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    let Some(actor_row) = actor_row else {
        return api_error(
            StatusCode::NOT_FOUND,
            "actor",
            "requesting actor no longer exists",
        );
    };
    let messages = match match query.work_id {
        Some(work_id) => org.owner_work_conversation(&actor, work_id, 100).await,
        None => org.owner_conversation(&actor, 100).await,
    } {
        Ok(messages) => messages,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    let focus = if query.work_id.is_none() {
        match org.owner_conversation_focus(&actor).await {
            Ok(focus) => Some(focus),
            Err(error) => {
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "orgintel",
                    format!("{error:#}"),
                )
            }
        }
    } else {
        None
    };
    Json(serde_json::json!({
        "actor": {
            "id": actor_row.id,
            "display": actor_row.display,
            "kind": actor_row.kind,
            "role": actor_row.role,
        },
        "focus": focus.map(|focus| serde_json::json!({
            "after_message_id": focus.after_message_id,
            "started_at": focus.started_at,
        })),
        "messages": messages.into_iter().map(|message| {
            let (body, intent) = split_intent_receipt(&message.body);
            let (body, details) = split_message_details(body);
            let (body, attachments) = split_attachment_block(body);
            let (body, context_path) = split_context_marker(body);
            serde_json::json!({
                "id": message.id,
                "from_actor": message.from_actor,
                "to_actor": message.to_actor,
                "body": body,
                "attachments": attachments,
                "intent": intent,
                "details": details,
                "context_path": context_path,
                "created_at": message.created_at,
                "read_at": message.read_at,
            })
        }).collect::<Vec<_>>(),
    }))
    .into_response()
}

/// Reconnectable live projection for one recorded owner message. This endpoint
/// never invents durable transcript rows: it carries only the in-flight ACP
/// reply/activity state until OrgIntel records the final message.
async fn actor_conversation_live(
    State(state): State<OwnerState>,
    AxumPath((company, actor)): AxumPath<(String, String)>,
    Query(query): Query<ConversationLiveQuery>,
) -> Response<Body> {
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    match org.list_actors().await {
        Ok(actors) if actors.iter().any(|row| row.id == actor) => {}
        Ok(_) => {
            return api_error(
                StatusCode::NOT_FOUND,
                "actor",
                "requesting actor no longer exists",
            )
        }
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    }

    let receiver = state
        .daemon
        .conversations
        .subscribe(&company, &actor, query.message_id);
    let stream =
        futures_util::stream::unfold((receiver, true), |(mut receiver, first)| async move {
            if !first && receiver.changed().await.is_err() {
                return None;
            }
            let state = receiver.borrow().clone();
            let data = serde_json::to_string(&state).unwrap_or_else(|_| {
                "{\"phase\":\"failed\",\"error\":\"live projection could not be encoded\"}".into()
            });
            let event = Event::default()
                .event("conversation")
                .id(state.sequence.to_string())
                .data(data);
            Some((Ok::<_, Infallible>(event), (receiver, false)))
        });
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("still-connected"),
        )
        .into_response()
}

async fn review_outcome(
    State(state): State<OwnerState>,
    AxumPath((company, handoff)): AxumPath<(String, Uuid)>,
    Json(input): Json<OwnerReviewInput>,
) -> impl IntoResponse {
    let decision = match input.decision.trim() {
        "accept" => restless_orgintel::OwnerReviewDecision::Accepted,
        "request_changes" => restless_orgintel::OwnerReviewDecision::ChangesRequested,
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "review",
                "decision must be accept or request_changes",
            )
        }
    };
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    match org
        .decide_owner_review(handoff, decision, &input.feedback)
        .await
    {
        Ok(()) => Json(serde_json::json!({
            "handoff_id": handoff,
            "decision": input.decision,
            "recorded": true,
        }))
        .into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "review", format!("{error:#}")),
    }
}

async fn resolve_handoff_decision(
    State(state): State<OwnerState>,
    AxumPath((company, handoff)): AxumPath<(String, Uuid)>,
    Json(input): Json<OwnerHandoffDecisionInput>,
) -> impl IntoResponse {
    if input.resolution.trim().is_empty() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "decision",
            "recording a decision needs the owner's exact answer",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    match org
        .resolve_handoff_as(
            handoff,
            "owner",
            restless_orgintel::OwnerHandoffState::Resolved,
            input.resolution.trim(),
        )
        .await
    {
        Ok(()) => Json(serde_json::json!({
            "handoff_id": handoff,
            "recorded": true,
        }))
        .into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "decision", format!("{error:#}")),
    }
}

async fn send_actor_message(
    State(state): State<OwnerState>,
    AxumPath((company, actor)): AxumPath<(String, String)>,
    multipart: Multipart,
) -> impl IntoResponse {
    let input = match parse_owner_message(multipart).await {
        Ok(input) => input,
        Err(message) => return api_error(StatusCode::BAD_REQUEST, "message", message),
    };
    let body = input.body.trim();
    if body.is_empty() || body.chars().count() > 20_000 {
        return api_error(
            StatusCode::BAD_REQUEST,
            "message",
            "message must contain between 1 and 20,000 characters",
        );
    }
    if input.new_focus && (actor != "exec" || input.work_id.is_some()) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "conversation_focus",
            "New focus is available only for ordinary Exec conversation",
        );
    }
    // Context is useful navigation metadata, not part of message delivery. Parse
    // it as a URL so a root screen with query state (for example
    // `/aris?item=...`) remains company-scoped. A malformed or cross-company
    // link is omitted rather than making the owner's actual message fail.
    let context_path = input
        .context_path
        .as_deref()
        .and_then(|path| canonical_cockpit_context(&company, path));
    let context_omitted = input.context_requested && context_path.is_none();
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    let target_exists = match org.list_actors().await {
        Ok(actors) => actors.iter().any(|row| row.id == actor),
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    if !target_exists {
        return api_error(
            StatusCode::NOT_FOUND,
            "actor",
            "requesting actor no longer exists",
        );
    }
    if let Err(error) = org
        .ensure_actor("owner", "owner", "owner", "The Owner")
        .await
    {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            format!("{error:#}"),
        );
    }
    let mut stored = Vec::with_capacity(input.attachments.len());
    for attachment in input.attachments {
        let upload_id = Uuid::new_v4();
        let metadata = OwnerAttachment {
            upload_id,
            name: attachment.name,
            media_type: attachment.media_type,
            size_bytes: attachment.bytes.len(),
            path: format!("/company/inbox/owner-attachments/{upload_id}/content"),
        };
        let sidecar = match serde_json::to_vec(&metadata) {
            Ok(sidecar) => sidecar,
            Err(error) => {
                rollback_attachments(&company, &stored).await;
                return api_error(StatusCode::BAD_REQUEST, "attachment", error.to_string());
            }
        };
        match runtime::store_owner_attachment(&company, upload_id, &attachment.bytes, &sidecar)
            .await
        {
            Ok(path) => {
                debug_assert_eq!(path, metadata.path);
                stored.push(metadata);
            }
            Err(error) => {
                rollback_attachments(&company, &stored).await;
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "attachment",
                    format!("{error:#}"),
                );
            }
        }
    }

    let recorded_body = message_with_context(body, context_path.as_deref());
    let recorded_body = message_with_attachments(&recorded_body, &stored);
    let sent = match input.work_id {
        Some(work_id) => org
            .send_work_message("owner", &actor, work_id, &recorded_body)
            .await
            .map(|message_id| (message_id, None)),
        None => org
            .send_owner_conversation_message(&actor, &recorded_body, input.new_focus)
            .await
            .map(|(message_id, focus)| (message_id, Some(focus))),
    };
    match sent {
        Ok((message_id, focus)) => {
            state
                .daemon
                .conversations
                .expect(&company, &actor, message_id, input.work_id);
            Json(serde_json::json!({
                "message_id": message_id,
                "context_attached": context_path.is_some(),
                "context_omitted": context_omitted,
                "focus": focus.map(|focus| serde_json::json!({
                    "after_message_id": focus.after_message_id,
                    "started_at": focus.started_at,
                })),
            }))
            .into_response()
        }
        Err(error) => {
            rollback_attachments(&company, &stored).await;
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    }
}

async fn parse_owner_message(
    mut multipart: Multipart,
) -> std::result::Result<OwnerMessageInput, String> {
    let mut input = OwnerMessageInput::default();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| format!("read message form: {error}"))?
    {
        match field.name() {
            Some("body") => {
                input.body = field
                    .text()
                    .await
                    .map_err(|error| format!("read message body: {error}"))?;
            }
            Some("work_id") => {
                let value = field
                    .text()
                    .await
                    .map_err(|error| format!("read Work reference: {error}"))?;
                if !value.trim().is_empty() {
                    input.work_id = Some(
                        Uuid::parse_str(value.trim())
                            .map_err(|error| format!("invalid Work reference: {error}"))?,
                    );
                }
            }
            Some("new_focus") => {
                let value = field
                    .text()
                    .await
                    .map_err(|error| format!("read conversation focus: {error}"))?;
                input.new_focus = match value.trim() {
                    "true" => true,
                    "false" | "" => false,
                    _ => return Err("new_focus must be true or false".into()),
                };
            }
            Some("context_path") => {
                let value = field
                    .text()
                    .await
                    .map_err(|error| format!("read cockpit context: {error}"))?;
                let value = value.trim();
                if !value.is_empty() {
                    input.context_requested = true;
                    if value.chars().count() <= 512 {
                        input.context_path = Some(value.to_string());
                    }
                }
            }
            Some("attachments") => {
                if input.attachments.len() >= MAX_ATTACHMENTS {
                    return Err(format!("attach at most {MAX_ATTACHMENTS} files"));
                }
                let name = safe_attachment_name(field.file_name().unwrap_or("attachment"));
                let media_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|error| format!("read {name}: {error}"))?;
                if bytes.len() > MAX_ATTACHMENT_BYTES {
                    return Err(format!("{name} exceeds the 5 MB attachment limit"));
                }
                input.attachments.push(PendingAttachment {
                    name,
                    media_type,
                    bytes: bytes.to_vec(),
                });
            }
            _ => {}
        }
    }
    Ok(input)
}

fn canonical_cockpit_context(company: &str, value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > 512
        || !value.starts_with('/')
        || value.starts_with("//")
        || value.contains('\\')
    {
        return None;
    }

    let parsed = url::Url::parse(&format!("http://cockpit.invalid{value}")).ok()?;
    if parsed.fragment().is_some() {
        return None;
    }
    let company_root = format!("/{company}");
    let pathname = parsed.path();
    if pathname != company_root && !pathname.starts_with(&format!("{company_root}/")) {
        return None;
    }

    let mut canonical = pathname.to_string();
    if let Some(query) = parsed.query() {
        canonical.push('?');
        canonical.push_str(query);
    }
    (canonical.chars().count() <= 512).then_some(canonical)
}

fn safe_attachment_name(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .filter(|character| !character.is_control())
        .take(180)
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        "attachment".into()
    } else {
        cleaned.to_string()
    }
}

fn message_with_attachments(body: &str, attachments: &[OwnerAttachment]) -> String {
    if attachments.is_empty() {
        return body.to_string();
    }
    let paths = attachments
        .iter()
        .map(|attachment| format!("- {}: {}", attachment.name, attachment.path))
        .collect::<Vec<_>>()
        .join("\n");
    let manifest = serde_json::to_string(attachments).unwrap_or_else(|_| "[]".into());
    format!("{body}{ATTACHMENT_BLOCK}{paths}\n{ATTACHMENT_MARKER}{manifest}-->")
}

fn message_with_context(body: &str, context_path: Option<&str>) -> String {
    match context_path {
        Some(path) => {
            let encoded = serde_json::json!({ "path": path });
            format!("{body}{CONTEXT_BLOCK}{path}{CONTEXT_MARKER}{encoded}-->")
        }
        None => body.to_string(),
    }
}

fn split_attachment_block(body: &str) -> (&str, Vec<OwnerAttachment>) {
    let Some((visible, block)) = body.rsplit_once(ATTACHMENT_BLOCK) else {
        return (body, Vec::new());
    };
    let Some(marker) = block.rfind(ATTACHMENT_MARKER) else {
        return (body, Vec::new());
    };
    let encoded = &block[marker + ATTACHMENT_MARKER.len()..];
    let Some(encoded) = encoded.strip_suffix("-->") else {
        return (body, Vec::new());
    };
    match serde_json::from_str(encoded) {
        Ok(attachments) => (visible, attachments),
        Err(_) => (body, Vec::new()),
    }
}

fn split_intent_receipt(body: &str) -> (&str, Option<OwnerIntentReceipt>) {
    let Some((visible, encoded)) = body.rsplit_once(INTENT_MARKER) else {
        return (body, None);
    };
    let Some(encoded) = encoded.strip_suffix("-->") else {
        return (body, None);
    };
    match serde_json::from_str::<OwnerIntentReceipt>(encoded) {
        Ok(receipt)
            if !receipt.summary.trim().is_empty() && receipt.summary.chars().count() <= 300 =>
        {
            (visible, Some(receipt))
        }
        _ => (body, None),
    }
}

fn split_message_details(body: &str) -> (&str, Option<String>) {
    let Some((visible, encoded)) = body.rsplit_once(DETAILS_MARKER) else {
        return (body, None);
    };
    let Some(encoded) = encoded.strip_suffix("-->") else {
        // Metadata syntax is never owner-facing, even when a provider emits a
        // malformed optional block.
        return (visible.trim_end(), None);
    };
    let details = serde_json::from_str::<OwnerMessageDetails>(encoded)
        .ok()
        .map(|details| details.markdown.trim().to_string())
        .filter(|markdown| !markdown.is_empty() && markdown.chars().count() <= 20_000);
    (visible.trim_end(), details)
}

fn split_context_marker(body: &str) -> (&str, Option<String>) {
    let Some((visible, encoded)) = body.rsplit_once(CONTEXT_MARKER) else {
        return (body, None);
    };
    let Some(encoded) = encoded.strip_suffix("-->") else {
        return (body, None);
    };
    let path = serde_json::from_str::<serde_json::Value>(encoded)
        .ok()
        .and_then(|value| {
            value
                .get("path")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        });
    match path {
        Some(path) => match visible.rsplit_once(CONTEXT_BLOCK) {
            Some((body, rendered)) if rendered == path => (body, Some(path)),
            _ => (visible, Some(path)),
        },
        None => (body, None),
    }
}

async fn rollback_attachments(company: &str, attachments: &[OwnerAttachment]) {
    for attachment in attachments {
        if let Err(error) = runtime::remove_owner_attachment(company, attachment.upload_id).await {
            tracing::warn!(%error, %company, attachment = %attachment.upload_id, "failed to roll back owner attachment");
        }
    }
}

async fn download_attachment(
    AxumPath((company, attachment)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    let (bytes, metadata) = match runtime::read_owner_attachment(&company, attachment).await {
        Ok(value) => value,
        Err(error) => {
            return api_error(StatusCode::NOT_FOUND, "attachment", format!("{error:#}"));
        }
    };
    let metadata: OwnerAttachment = match serde_json::from_slice(&metadata) {
        Ok(metadata) => metadata,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "attachment",
                format!("invalid attachment metadata: {error}"),
            );
        }
    };
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&metadata.media_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    let disposition = format!(
        "inline; filename=\"{}\"",
        metadata.name.replace(['\\', '"'], "_")
    );
    response.headers_mut().insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&disposition)
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    response
}

async fn grant(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<PartyAction>,
) -> impl IntoResponse {
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
    Json(input): Json<PartyAction>,
) -> impl IntoResponse {
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
    Json(input): Json<PartyAction>,
) -> impl IntoResponse {
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

async fn issue_review_ticket(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<ReviewTicketRequest>,
) -> impl IntoResponse {
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
            "review is no longer outstanding",
        );
    };
    let Some(reference) = item.review_target.as_ref() else {
        return api_error(
            StatusCode::CONFLICT,
            "review",
            "this item has no directly reviewable web outcome",
        );
    };
    let current = runtime::generation(&company).await.ok().flatten();
    if current.as_deref() != Some(reference.generation.as_str()) {
        return api_error(
            StatusCode::CONFLICT,
            "runtime",
            "runtime generation changed; refresh the review",
        );
    }
    let target = match runtime::runtime_http_target(&reference.uri) {
        Ok(target) => target,
        Err(error) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "review",
                format!("invalid review target: {error:#}"),
            )
        }
    };
    if let Err(error) = runtime::probe_runtime_http(&company, &reference.uri).await {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "review",
            format!("live outcome is unavailable: {error:#}"),
        );
    }

    let ticket = Uuid::new_v4().simple().to_string();
    let (review_url, expected_host) =
        match materialize_review_url(&state.review_public_url, &ticket, &target.path_and_query) {
            Ok(value) => value,
            Err(error) => {
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "review",
                    format!("review origin is invalid: {error:#}"),
                )
            }
        };
    state.reviews.lock().expect("review registry").insert(
        ticket,
        ReviewSession {
            company: company.clone(),
            generation: reference.generation.clone(),
            item_id: item.id.clone(),
            port: target.port,
            expected_host,
            expires_at: SystemTime::now() + REVIEW_TTL,
        },
    );
    Json(ReviewTicketResponse {
        review_url,
        expires_in_seconds: REVIEW_TTL.as_secs(),
    })
    .into_response()
}

async fn review_proxy(
    State(state): State<OwnerState>,
    method: Method,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response<Body> {
    if !matches!(method, Method::GET | Method::HEAD) {
        return api_error(
            StatusCode::METHOD_NOT_ALLOWED,
            "review",
            "review previews are read-only",
        );
    }
    let host = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let token = host
        .split(':')
        .next()
        .and_then(|hostname| hostname.split('.').next())
        .unwrap_or_default();
    if token.len() != 32 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "review",
            "review ticket is invalid",
        );
    }
    let session = {
        let mut reviews = state.reviews.lock().expect("review registry");
        reviews.retain(|_, review| review.expires_at > SystemTime::now());
        reviews
            .get(token)
            .filter(|review| review.expected_host.eq_ignore_ascii_case(&host))
            .cloned()
    };
    let Some(session) = session else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "review",
            "review ticket is absent or expired",
        );
    };
    let current = runtime::generation(&session.company).await.ok().flatten();
    if current.as_deref() != Some(session.generation.as_str()) {
        return api_error(
            StatusCode::CONFLICT,
            "runtime",
            "review points at a replaced company computer",
        );
    }
    let path = uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");
    let upstream =
        match runtime::runtime_http_request(&session.company, session.port, method, path, &headers)
            .await
        {
            Ok(response) => response,
            Err(error) => {
                return api_error(
                    StatusCode::BAD_GATEWAY,
                    "review",
                    format!("live outcome bridge: {error:#}"),
                )
            }
        };
    let (parts, body) = upstream.into_parts();
    let mut response = Response::from_parts(parts, Body::new(body));
    for name in [
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "set-cookie",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
        // The preview iframe is already isolated on its own origin and
        // sandboxed by the cockpit. Upstream anti-framing headers describe a
        // public deployment, not this owner-only review projection.
        "content-security-policy",
        "x-frame-options",
    ] {
        response.headers_mut().remove(name);
    }
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    tracing::debug!(
        company = %session.company,
        item = %session.item_id,
        path,
        "served isolated live review"
    );
    response
}

fn validate_review_public_url(template: &str, expected_port: u16) -> Result<()> {
    if template.matches("{ticket}").count() != 1 {
        anyhow::bail!("RESTLESS_REVIEW_PUBLIC_URL must contain one {{ticket}} placeholder");
    }
    let (url, _) = materialize_review_url(template, &"a".repeat(32), "/")?;
    let parsed = url::Url::parse(&url)?;
    if parsed.scheme() != "http"
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port_or_known_default() != Some(expected_port)
        || !parsed
            .host_str()
            .is_some_and(|host| host.ends_with(".localhost"))
    {
        anyhow::bail!(
            "review public URL must be the configured http loopback origin on port {expected_port}"
        );
    }
    Ok(())
}

fn materialize_review_url(
    template: &str,
    ticket: &str,
    path_and_query: &str,
) -> Result<(String, String)> {
    let mut url = url::Url::parse(&template.replace("{ticket}", ticket))?;
    let hostname = url
        .host_str()
        .context("review public URL has no host")?
        .to_string();
    if !hostname.starts_with(&format!("{ticket}.")) {
        anyhow::bail!("review ticket must be the first hostname label");
    }
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        anyhow::bail!("review public URL must be an origin without a path, query, or fragment");
    }
    let (path, query) = path_and_query
        .split_once('?')
        .map_or((path_and_query, None), |(path, query)| (path, Some(query)));
    url.set_path(path);
    url.set_query(query);
    let expected_host = match url.port() {
        Some(port) => format!("{hostname}:{port}"),
        None => hostname,
    };
    Ok((url.to_string(), expected_host))
}

async fn issue_ticket(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<TicketRequest>,
) -> impl IntoResponse {
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
    let mut requesting_actor = None;
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
        requesting_actor.clone_from(&reference.requesting_actor);
    }
    let ticket = Uuid::new_v4().simple().to_string();
    state.tickets.lock().expect("ticket registry").insert(
        ticket.clone(),
        AttachTicket {
            company: company.clone(),
            generation: current,
            item_id: input.item_id,
            client_id: input.client_id,
            requesting_actor,
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
    Query(query): Query<TicketQuery>,
) -> impl IntoResponse {
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
            requesting_actor: ticket.requesting_actor,
            expires_at: SystemTime::now() + ATTACH_TTL,
        },
    );
    tracing::info!(company, item = %ticket.item_id, "owner desktop attached");
    let target = desktop_client_url(&company, DesktopClientMode::Observe);
    let mut response = Redirect::to(&target).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&format!(
            "{ATTACH_COOKIE}={attach}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
            ATTACH_TTL.as_secs()
        ))
        .expect("attach cookie"),
    );
    response
}

#[derive(Clone, Copy)]
enum DesktopClientMode {
    Observe,
    Control,
}

/// Keep the imported display client behind one server-owned seam. Observers
/// scale locally so two browser tabs cannot fight over the dimensions of the
/// shared company computer; only the sole controller may resize its framebuffer.
fn desktop_client_url(company: &str, mode: DesktopClientMode) -> String {
    let (resize, view_only) = match mode {
        DesktopClientMode::Observe => ("scale", "1"),
        DesktopClientMode::Control => ("remote", "0"),
    };
    format!(
        "/desktop/{company}/vnc.html?autoconnect=1&reconnect=1&reconnect_delay=1000&shared=1&resize={resize}&view_only={view_only}&path=desktop/{company}/websockify"
    )
}

async fn open_observed_desktop(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if valid_attach(&state, &company, &headers).is_none() {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "desktop attachment is absent or expired",
        );
    }
    Redirect::to(&desktop_client_url(&company, DesktopClientMode::Observe)).into_response()
}

async fn open_controlled_desktop(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Query(query): Query<DesktopMode>,
) -> impl IntoResponse {
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
    Redirect::to(&desktop_client_url(&company, DesktopClientMode::Control)).into_response()
}

async fn desktop_asset(
    State(state): State<OwnerState>,
    AxumPath((company, asset)): AxumPath<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if valid_attach(&state, &company, &headers).is_none() {
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

async fn browser_status(AxumPath(company): AxumPath<String>) -> impl IntoResponse {
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
        "requesting_actor": attach.requesting_actor,
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
    AxumPath(company): AxumPath<String>,
    Json(input): Json<ControlRequest>,
) -> impl IntoResponse {
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
    Json(input): Json<ControlRequest>,
) -> impl IntoResponse {
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
    let requesting_actor = current["requesting_actor"].as_str().map(str::to_string);
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
    if let Some(requesting_actor) = requesting_actor {
        if let Ok(org) = state.daemon.orgintel.get(&company).await {
            let _ = org
                .ensure_actor("owner", "owner", "owner", "The Owner")
                .await;
            let _ = org
                .send_message(
                    "owner",
                    Some(&requesting_actor),
                    "Browser control returned. Inspect the same page state and verify the source condition; hand-back is not proof of completion.",
                )
                .await;
        }
    }
    Json(next).into_response()
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

fn company_display_name(name: &str) -> String {
    name.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + characters.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
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

async fn api_not_found() -> Response<Body> {
    api_error(StatusCode::NOT_FOUND, "api", "unknown owner API route")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_owner_bindings_are_refused_until_real_auth_exists() {
        assert!(ensure_loopback("127.0.0.1:7788".parse().unwrap(), "owner").is_ok());
        assert!(ensure_loopback("[::1]:7788".parse().unwrap(), "owner").is_ok());
        assert!(ensure_loopback("0.0.0.0:7788".parse().unwrap(), "owner").is_err());
        assert!(ensure_loopback("192.0.2.1:7788".parse().unwrap(), "owner").is_err());
    }

    #[test]
    fn local_owner_boundary_allows_reads_and_same_origin_writes() {
        let mut read = HeaderMap::new();
        read.insert(HOST, HeaderValue::from_static("localhost:7788"));
        assert_eq!(local_owner_boundary_violation(&Method::GET, &read), None);

        let mut write = read.clone();
        write.insert(ORIGIN, HeaderValue::from_static("http://localhost:7788"));
        write.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        assert_eq!(local_owner_boundary_violation(&Method::POST, &write), None);

        let mut proxied = HeaderMap::new();
        proxied.insert(HOST, HeaderValue::from_static("127.0.0.1:5173"));
        proxied.insert(ORIGIN, HeaderValue::from_static("http://127.0.0.1:5173"));
        assert_eq!(
            local_owner_boundary_violation(&Method::POST, &proxied),
            None
        );
    }

    #[test]
    fn local_owner_boundary_refuses_proxy_cross_site_and_origin_bypass() {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("localhost:7788"));

        assert!(local_owner_boundary_violation(&Method::POST, &headers).is_some());

        headers.insert(ORIGIN, HeaderValue::from_static("http://127.0.0.1:7788"));
        assert!(local_owner_boundary_violation(&Method::POST, &headers).is_some());

        headers.insert(ORIGIN, HeaderValue::from_static("http://localhost:7788"));
        headers.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        assert!(local_owner_boundary_violation(&Method::POST, &headers).is_some());

        headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        headers.insert("x-forwarded-for", HeaderValue::from_static("127.0.0.1"));
        assert!(local_owner_boundary_violation(&Method::GET, &headers).is_some());

        headers.remove("x-forwarded-for");
        headers.insert(HOST, HeaderValue::from_static("example.com:7788"));
        assert!(local_owner_boundary_violation(&Method::GET, &headers).is_some());
    }

    #[test]
    fn review_url_uses_ticket_as_an_isolated_origin_and_preserves_route() {
        let ticket = "0123456789abcdef0123456789abcdef";
        let (url, host) = materialize_review_url(
            "http://{ticket}.localhost:7794",
            ticket,
            "/for-tutoring-centres?language=en",
        )
        .unwrap();
        assert_eq!(
            url,
            "http://0123456789abcdef0123456789abcdef.localhost:7794/for-tutoring-centres?language=en"
        );
        assert_eq!(host, "0123456789abcdef0123456789abcdef.localhost:7794");
        assert!(materialize_review_url("http://localhost:7794/{ticket}", ticket, "/").is_err());
        assert!(validate_review_public_url("http://preview.localhost:7794", 7794).is_err());
        assert!(validate_review_public_url("http://{ticket}.localhost:7794", 7794).is_ok());
        assert!(validate_review_public_url("https://{ticket}.localhost:7794", 7794).is_err());
        assert!(validate_review_public_url("http://{ticket}.example.com:7794", 7794).is_err());
        assert!(validate_review_public_url("http://{ticket}.localhost:8000", 7794).is_err());
    }

    #[test]
    fn only_the_controller_can_resize_the_shared_desktop() {
        let observer = desktop_client_url("company_test", DesktopClientMode::Observe);
        assert!(observer.contains("resize=scale"));
        assert!(observer.contains("view_only=1"));

        let controller = desktop_client_url("company_test", DesktopClientMode::Control);
        assert!(controller.contains("resize=remote"));
        assert!(controller.contains("view_only=0"));
        assert!(controller.contains("reconnect=1"));
    }

    #[test]
    fn owner_message_metadata_round_trips_without_leaking_into_visible_copy() {
        let attachment = OwnerAttachment {
            upload_id: Uuid::nil(),
            name: "brief.pdf".into(),
            media_type: "application/pdf".into(),
            size_bytes: 42,
            path: "/company/inbox/owner-attachments/00000000-0000-0000-0000-000000000000/content"
                .into(),
        };
        let with_context = message_with_context("Please read this.", Some("/aris/work"));
        let recorded = message_with_attachments(&with_context, std::slice::from_ref(&attachment));

        let (without_attachments, attachments) = split_attachment_block(&recorded);
        let (visible, context) = split_context_marker(without_attachments);
        assert_eq!(visible, "Please read this.");
        assert_eq!(context.as_deref(), Some("/aris/work"));
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].name, "brief.pdf");
        assert!(recorded.contains(&attachment.path));
    }

    #[test]
    fn cockpit_context_is_scoped_by_url_path_not_raw_query_text() {
        assert_eq!(
            canonical_cockpit_context("aris", "/aris?item=release-integrity"),
            Some("/aris?item=release-integrity".into())
        );
        assert_eq!(
            canonical_cockpit_context("aris", "/aris?next=https://example.com/review"),
            Some("/aris?next=https://example.com/review".into())
        );
        assert_eq!(
            canonical_cockpit_context("aris", "/aris/work/42?lens=board"),
            Some("/aris/work/42?lens=board".into())
        );

        assert_eq!(canonical_cockpit_context("aris", "/cosmon?item=42"), None);
        assert_eq!(canonical_cockpit_context("aris", "/aris-other"), None);
        assert_eq!(canonical_cockpit_context("aris", "/aris/../cosmon"), None);
        assert_eq!(canonical_cockpit_context("aris", "//aris/work"), None);
        assert_eq!(
            canonical_cockpit_context("aris", "https://example.com/aris"),
            None
        );
        assert_eq!(canonical_cockpit_context("aris", "/aris#hidden"), None);
    }

    #[test]
    fn only_a_valid_exec_intent_receipt_is_promoted_to_ui_metadata() {
        let body = concat!(
            "I will treat this as durable direction.",
            "\n\n<!--restless-intent:{\"kind\":\"direction\",",
            "\"summary\":\"Prioritise tutor interviews before outreach.\"}-->"
        );
        let (visible, receipt) = split_intent_receipt(body);
        assert_eq!(visible, "I will treat this as durable direction.");
        assert!(matches!(
            receipt.map(|receipt| receipt.kind),
            Some(OwnerIntentKind::Direction)
        ));

        let malformed = "Reply\n\n<!--restless-intent:{\"kind\":\"whatever\",\"summary\":\"x\"}-->";
        assert_eq!(split_intent_receipt(malformed).0, malformed);
        assert!(split_intent_receipt(malformed).1.is_none());
    }

    #[test]
    fn optional_work_details_are_separate_and_malformed_metadata_stays_hidden() {
        let body = concat!(
            "The release is ready.",
            "\n\n<!--restless-details:{\"markdown\":\"- Commit `abc123`\\n- Build passed\"}-->"
        );
        let (visible, details) = split_message_details(body);
        assert_eq!(visible, "The release is ready.");
        assert_eq!(
            details.as_deref(),
            Some("- Commit `abc123`\n- Build passed")
        );

        let malformed = "Answer.\n\n<!--restless-details:not-json-->";
        assert_eq!(split_message_details(malformed), ("Answer.", None));
    }
}
