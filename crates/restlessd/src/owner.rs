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
use axum::extract::{DefaultBodyLimit, Multipart, Path as AxumPath, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE, COOKIE, SET_COOKIE};
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

use crate::{approval, attention, credential, reconcile, runtime, Daemon};

const OWNER_COOKIE: &str = "restless_owner";
const ATTACH_COOKIE: &str = "restless_attach";
const TICKET_TTL: Duration = Duration::from_secs(30);
const ATTACH_TTL: Duration = Duration::from_secs(30 * 60);
const CONTROL_TTL_SECONDS: i64 = 45;
const MAX_ATTACHMENTS: usize = 6;
const MAX_ATTACHMENT_BYTES: usize = 5 * 1024 * 1024;
const ATTACHMENT_BLOCK: &str = "\n\n[Restless attachments]\n";
const ATTACHMENT_MARKER: &str = "<!--restless-attachments:";
const INTENT_MARKER: &str = "\n\n<!--restless-intent:";
const CONTEXT_BLOCK: &str = "\n\n[Owner cockpit context]\n";
const CONTEXT_MARKER: &str = "\n\n<!--restless-context:";

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

#[derive(Debug, Deserialize)]
struct SignIn {
    token: String,
}

#[derive(Debug, Deserialize)]
struct PartyAction {
    party: String,
}

#[derive(Default)]
struct OwnerMessageInput {
    body: String,
    work_id: Option<Uuid>,
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

#[derive(Debug, Deserialize, Default)]
struct ConversationQuery {
    work_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Default)]
struct CockpitQuery {
    #[serde(default)]
    probe_credentials: bool,
}

#[derive(Debug, Deserialize)]
struct OwnerReviewInput {
    decision: String,
    #[serde(default)]
    feedback: String,
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

    let api = Router::new()
        .route("/session", post(sign_in).delete(sign_out))
        .route("/companies", get(company_catalog))
        .route("/companies/{company}/archive", post(archive_company))
        .route("/companies/{company}/restore", post(restore_company))
        .route("/companies/{company}/attention", get(attention_view))
        .route("/companies/{company}/cockpit", get(cockpit_view))
        .route(
            "/companies/{company}/actors/{actor}/conversation",
            get(actor_conversation).post(send_actor_message),
        )
        .route(
            "/companies/{company}/attachments/{attachment}",
            get(download_attachment),
        )
        .route(
            "/companies/{company}/handoffs/{handoff}/review",
            post(review_outcome),
        )
        .route("/companies/{company}/approvals/grant", post(grant))
        .route("/companies/{company}/approvals/decline", post(decline))
        .route("/companies/{company}/approvals/revoke", post(revoke))
        .route("/companies/{company}/browser/ticket", post(issue_ticket))
        .route("/companies/{company}/browser/status", get(browser_status))
        .route("/companies/{company}/browser/take", post(take_control))
        .route("/companies/{company}/browser/heartbeat", post(heartbeat))
        .route("/companies/{company}/browser/return", post(return_control))
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024));

    let source = runtime::source_root()?;
    let web = source.join("web/build");
    let static_files = ServeDir::new(&web).fallback(ServeFile::new(web.join("index.html")));
    let app = Router::new()
        .nest("/api", api)
        .route("/desktop/{company}", get(open_desktop))
        .route("/desktop/{company}/control", get(open_controlled_desktop))
        .route("/desktop/{company}/websockify", get(desktop_websocket))
        .route("/desktop/{company}/{*asset}", get(desktop_asset))
        .fallback_service(static_files)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("bind owner gateway {address}"))?;
    tracing::info!(addr = %address, "owner gateway listening");
    axum::serve(listener, app).await.context("owner gateway")
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

async fn company_catalog(State(state): State<OwnerState>, headers: HeaderMap) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
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
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
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
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
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

/// The owner cockpit's cross-plane read. This is deliberately an aggregation
/// at the presentation boundary, not a second writer: each field is read from
/// the plane that owns it and carries explicit source health when that plane
/// cannot answer. Authority remains readable when recoverable OrgIntel is
/// unavailable.
async fn cockpit_view(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<CockpitQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
    };

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
        .map(|actor| {
            let spent: f64 = spend_breakdown
                .iter()
                .filter(|(id, _, _)| id == &actor.id)
                .map(|(_, _, usd)| usd)
                .sum();
            let session_running = if actor.id == "exec" {
                state
                    .daemon
                    .in_flight
                    .lock()
                    .map(|running| running.is_active(&company))
                    .unwrap_or(false)
            } else {
                state.daemon.staff.is_actor_running(&company, &actor.id)
            };
            serde_json::json!({
                "actor_id": actor.id,
                "role": actor.kind,
                "display": actor.display,
                "model": actor.model,
                "team_id": actor.team_id,
                "spent_usd": round_owner_usd(spent),
                "session_running": session_running,
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
        .filter(|actor| !matches!(actor.id.as_str(), "owner" | "exec" | "world" | "daemon"))
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
        },
        "receipts": receipts,
        "refreshed_at": Utc::now(),
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
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
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
    let messages = match query.work_id {
        Some(work_id) => org.owner_work_conversation(&actor, work_id, 100).await,
        None => org.owner_conversation(&actor, 100).await,
    };
    match messages {
        Ok(messages) => Json(serde_json::json!({
            "actor": {
                "id": actor_row.id,
                "display": actor_row.display,
                "role": actor_row.kind,
            },
            "messages": messages.into_iter().map(|message| {
                let (body, intent) = split_intent_receipt(&message.body);
                let (body, attachments) = split_attachment_block(body);
                let (body, context_path) = split_context_marker(body);
                serde_json::json!({
                    "id": message.id,
                    "from_actor": message.from_actor,
                    "to_actor": message.to_actor,
                    "body": body,
                    "attachments": attachments,
                    "intent": intent,
                    "context_path": context_path,
                    "created_at": message.created_at,
                    "read_at": message.read_at,
                })
            }).collect::<Vec<_>>(),
        }))
        .into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            format!("{error:#}"),
        ),
    }
}

async fn review_outcome(
    State(state): State<OwnerState>,
    AxumPath((company, handoff)): AxumPath<(String, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<OwnerReviewInput>,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
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

async fn send_actor_message(
    State(state): State<OwnerState>,
    AxumPath((company, actor)): AxumPath<(String, String)>,
    headers: HeaderMap,
    multipart: Multipart,
) -> impl IntoResponse {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
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
    if input.context_path.as_deref().is_some_and(|path| {
        path != format!("/{company}") && !path.starts_with(&format!("/{company}/"))
    }) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "message",
            "cockpit context must belong to this company",
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
    if let Err(error) = org.add_actor("owner", "owner", "The Owner").await {
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

    let recorded_body = message_with_context(body, input.context_path.as_deref());
    let recorded_body = message_with_attachments(&recorded_body, &stored);
    let sent = match input.work_id {
        Some(work_id) => {
            org.send_work_message("owner", &actor, work_id, &recorded_body)
                .await
        }
        None => {
            org.send_message("owner", Some(&actor), &recorded_body)
                .await
        }
    };
    match sent {
        Ok(message_id) => Json(serde_json::json!({ "message_id": message_id })).into_response(),
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
            Some("context_path") => {
                let value = field
                    .text()
                    .await
                    .map_err(|error| format!("read cockpit context: {error}"))?;
                let value = value.trim();
                if !value.is_empty() {
                    if value.chars().count() > 512
                        || !value.starts_with('/')
                        || value.contains("//")
                    {
                        return Err("cockpit context must be a bounded local path".into());
                    }
                    input.context_path = Some(value.to_string());
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
    State(state): State<OwnerState>,
    AxumPath((company, attachment)): AxumPath<(String, Uuid)>,
    headers: HeaderMap,
) -> Response<Body> {
    if let Some(response) = require_owner(&state, &headers) {
        return response;
    }
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
            requesting_actor: ticket.requesting_actor,
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
            let _ = org.add_actor("owner", "owner", "The Owner").await;
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
}
