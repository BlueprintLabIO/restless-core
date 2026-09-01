//! Owner transport for the static SPA and persistent desktop.
//!
//! Entry is decided by [`crate::entry::EntryMode`]: loopback in local mode
//! (ADR 0001), a verified identity assertion in network mode (ADR 0007).
//!
//! This is intentionally a narrow BFF: owner projection, source-owned
//! approval actions, and browser attach/lease transport. It is not a generic
//! REST facade over the company computer.

use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use anyhow::{Context as _, Result};
use axum::body::Body;
use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{
    DefaultBodyLimit, Multipart, OriginalUri, Path as AxumPath, Query, Request, State,
};
use axum::http::header::{
    AUTHORIZATION, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE, COOKIE, HOST, ORIGIN,
    SET_COOKIE,
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
use sha2::{Digest as _, Sha256};
use subtle::ConstantTimeEq as _;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::{client_async, tungstenite};
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

use crate::entry::{company_in_path, EntryMode, SessionStore};
use crate::{
    airwallex, approval, attention, authority, company as company_projection, credential, finance,
    legal, reconcile, runtime, runtime_bridge, Daemon,
};

const ATTACH_COOKIE: &str = "restless_attach";
const SESSION_COOKIE: &str = "restless_session";
const TICKET_TTL: Duration = Duration::from_secs(30);
const ATTACH_TTL: Duration = Duration::from_secs(30 * 60);
const REVIEW_TTL: Duration = Duration::from_secs(30 * 60);
/// Owner desktop control is deliberately short-lived. The cockpit renews this
/// only after input reaches the remote desktop; merely leaving a tab open must
/// not strand the Company computer under an absent owner's control.
const CONTROL_TTL_SECONDS: i64 = 60;
const MAX_ATTACHMENTS: usize = 6;
const MAX_ATTACHMENT_BYTES: usize = 5 * 1024 * 1024;
const ATTACHMENT_BLOCK: &str = "\n\n[Restless attachments]\n";
const ATTACHMENT_MARKER: &str = "<!--restless-attachments:";
const INTENT_MARKER: &str = "<!--restless-intent:";
const DETAILS_MARKER: &str = "<!--restless-details:";
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
    entry: EntryMode,
    sessions: Arc<SessionStore>,
    plane_readiness: Option<PlaneReadinessConfig>,
    runtime_bridges: runtime_bridge::Registry,
}

#[derive(Clone)]
pub(crate) struct OwnerConfig {
    address: SocketAddr,
    review_address: SocketAddr,
    review_public_url: String,
    entry: EntryMode,
    plane_readiness: Option<PlaneReadinessConfig>,
}

#[derive(Clone)]
struct PlaneReadinessConfig {
    token: String,
    cell_token: String,
    runtime_bootstrap_token: String,
    owner_id: Uuid,
    plane_id: Uuid,
    hostname: String,
    account_plane_image: String,
    desired_revision: i64,
    release_manifest_digest: String,
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
    source: ReviewSource,
    expected_host: String,
    expires_at: SystemTime,
}

/// Where an isolated review origin reads from. Both are ordinary Runtime truth
/// observed read-only through the owner gateway; neither copies the outcome.
#[derive(Clone)]
enum ReviewSource {
    /// A project service already listening inside the company computer.
    Service { port: u16 },
    /// One produced file and the directory it needs, e.g. a rendered page with
    /// its own stylesheet and images (S19-T5).
    Files { root: PathBuf, entry: String },
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
    interrupt: bool,
    context_requested: bool,
    context_path: Option<String>,
    attachments: Vec<PendingAttachment>,
}

struct PendingAttachment {
    name: String,
    media_type: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
struct OwnerAttachment {
    upload_id: Uuid,
    name: String,
    media_type: String,
    size_bytes: usize,
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
enum OwnerIntentKind {
    Conversation,
    WorkFeedback,
    Direction,
    Authority,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
struct OwnerIntentReceipt {
    kind: OwnerIntentKind,
    summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    next_step: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner_need: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OwnerMessageDetails {
    markdown: String,
}

/// The durable owner/actor transcript returned to both the browser and the
/// terminal client. This is intentionally a small response contract instead
/// of a `serde_json::Value`: messages remain source-owned by OrgIntel, while
/// their owner-safe presentation metadata has one checked schema.
#[derive(Debug, Serialize, ts_rs::TS)]
struct ConversationView {
    actor: ConversationActorView,
    focus: Option<ConversationFocusView>,
    messages: Vec<ConversationMessageView>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct ConversationActorView {
    id: String,
    display: String,
    kind: String,
    role: String,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct ConversationFocusView {
    after_message_id: i64,
    started_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct ConversationMessageView {
    id: i64,
    from_actor: String,
    to_actor: Option<String>,
    body: String,
    attachments: Vec<OwnerAttachment>,
    details: Option<String>,
    intent: Option<OwnerIntentReceipt>,
    context_path: Option<String>,
    created_at: chrono::DateTime<Utc>,
    read_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct ConversationSendResponse {
    message_id: i64,
    interrupted: bool,
    context_attached: bool,
    context_omitted: bool,
    focus: Option<ConversationFocusView>,
}

/// An explicit interruption does not manufacture a second owner message.
/// `cancelled` means the durable input was consumed; `interrupted` says a
/// currently-supervised process also received cancellation.
#[derive(Debug, Serialize, ts_rs::TS)]
struct ConversationInterruptResponse {
    message_id: i64,
    cancelled: bool,
    interrupted: bool,
}

#[derive(Debug, Deserialize, Default)]
struct ConversationQuery {
    work_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct AgentActivityQuery {
    message_id: Option<i64>,
    work_id: Option<Uuid>,
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
    /// Set when the account plane could not admit a model route for this
    /// company at boot. The company is configured but cannot start until the
    /// reason is resolved (cross-layer contract §1.4.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    unstartable_reason: Option<String>,
}

/// The one high-value owner read model. This remains a projection: every
/// field is assembled from its authoritative plane immediately before the
/// response is encoded. Naming it makes the browser contract checkable rather
/// than letting a JSON macro silently grow a second, unreviewed schema.
#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitView {
    company: CockpitCompany,
    source_health: BTreeMap<String, String>,
    people: Vec<CockpitPerson>,
    teams: Vec<CockpitTeam>,
    goals: Vec<CockpitGoal>,
    spend: CockpitSpend,
    authority: CockpitAuthority,
    receipts: Vec<CockpitEffectReceipt>,
    refreshed_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitCompany {
    id: String,
    name: String,
    mission: String,
    model: String,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitPerson {
    actor_id: String,
    kind: String,
    role: String,
    display: String,
    model: Option<String>,
    team_id: Option<Uuid>,
    spent_usd: f64,
    session_running: bool,
    session_observed_at: Option<chrono::DateTime<Utc>>,
    model_cooldown: Option<CockpitModelCooldown>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitModelCooldown {
    model: String,
    kind: String,
    reason: String,
    retry_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitTeam {
    id: Uuid,
    name: String,
    brief: String,
    lead_actor_id: String,
    created_by: String,
    created_at: chrono::DateTime<Utc>,
    member_count: usize,
    in_motion_count: usize,
    blocked_count: usize,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitGoal {
    id: Uuid,
    title: String,
    body: String,
    created_by: String,
    created_at: chrono::DateTime<Utc>,
    closed_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitSpend {
    accounted_usd: f64,
    ceiling_usd: f64,
    remaining_usd: Option<f64>,
    status: String,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitAuthority {
    approved_parties: Vec<String>,
    credentials: Vec<CockpitCredential>,
    legal: CockpitLegal,
    provider: CockpitProvider,
    finance: CockpitFinance,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitCredential {
    binding: String,
    status: String,
    detail: Option<String>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitLegal {
    status: String,
    profile: Option<CockpitLegalProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    detail: Option<String>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitLegalProfile {
    legal_name: String,
    trading_name: Option<String>,
    entity_type: String,
    jurisdiction: String,
    registration_identifier: CockpitRegistrationIdentifier,
    approved_business_address: String,
    invoice_email: Option<String>,
    owner_asserted_by: String,
    owner_asserted_at: chrono::DateTime<Utc>,
    registry_observation: Option<CockpitRegistryObservation>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitRegistrationIdentifier {
    kind: String,
    value: String,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitRegistryObservation {
    source: String,
    status: String,
    observed_at: chrono::DateTime<Utc>,
    legal_name: Option<String>,
    entity_type: Option<String>,
    jurisdiction: Option<String>,
    registration_identifier: Option<CockpitRegistrationIdentifier>,
    detail: Option<String>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitProvider {
    status: String,
    connection: Option<CockpitProviderConnection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    detail: Option<String>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitProviderConnection {
    environment: String,
    account_ref: String,
    api_version: String,
    read_scopes: Vec<String>,
    submit_scopes: Vec<String>,
    approval_workflow_observed: bool,
    observed_at: Option<chrono::DateTime<Utc>>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitFinance {
    status: String,
    envelopes: Vec<CockpitMoneyEnvelope>,
    payments: Vec<CockpitPaymentIntent>,
    last_balance_observation: Option<CockpitBalanceObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    detail: Option<String>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitMoneyEnvelope {
    source_account_ref: String,
    currency: String,
    beneficiary_refs: Vec<String>,
    per_payment_limit_minor: i64,
    aggregate_limit_minor: i64,
    frozen: bool,
    period_started_at: chrono::DateTime<Utc>,
    updated_by: String,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitPaymentIntent {
    work_id: Uuid,
    owner_handoff_id: Uuid,
    source_account_ref: String,
    provider_beneficiary_ref: String,
    amount_minor: i64,
    currency: String,
    purpose: String,
    evidence_refs: Vec<String>,
    idempotency_key: String,
    requesting_actor: String,
    state: String,
    provider: String,
    provider_transfer_id: Option<String>,
    raw_provider_status: Option<String>,
    provider_approval_url: Option<String>,
    settled_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitBalanceObservation {
    observed_at: chrono::DateTime<Utc>,
    body: serde_json::Value,
}

#[derive(Debug, Serialize, ts_rs::TS)]
struct CockpitEffectReceipt {
    id: i64,
    effect_class: Option<serde_json::Value>,
    tool: Option<serde_json::Value>,
    success: Option<serde_json::Value>,
    party: Option<serde_json::Value>,
    actor: Option<serde_json::Value>,
    outcome: Option<serde_json::Value>,
    evidence_quality: CockpitEvidenceQuality,
    at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
enum CockpitEvidenceQuality {
    Governed,
    LegacyUnverified,
}

fn cockpit_legal_profile(profile: legal::LegalProfile) -> CockpitLegalProfile {
    let legal::LegalProfile {
        safe,
        owner_asserted_by,
        owner_asserted_at,
        registry_observation,
    } = profile;
    let legal::LegalProfileInput {
        legal_name,
        trading_name,
        entity_type,
        jurisdiction,
        registration_identifier,
        approved_business_address,
        invoice_email,
    } = safe;
    CockpitLegalProfile {
        legal_name,
        trading_name,
        entity_type,
        jurisdiction,
        registration_identifier: CockpitRegistrationIdentifier {
            kind: registration_identifier.kind,
            value: registration_identifier.value,
        },
        approved_business_address,
        invoice_email,
        owner_asserted_by,
        owner_asserted_at,
        registry_observation: registry_observation.map(|observation| CockpitRegistryObservation {
            source: observation.source,
            status: match observation.status {
                legal::RegistryObservationStatus::Observed => "observed",
                legal::RegistryObservationStatus::Unavailable => "unavailable",
            }
            .into(),
            observed_at: observation.observed_at,
            legal_name: observation.legal_name,
            entity_type: observation.entity_type,
            jurisdiction: observation.jurisdiction,
            registration_identifier: observation.registration_identifier.map(|identifier| {
                CockpitRegistrationIdentifier {
                    kind: identifier.kind,
                    value: identifier.value,
                }
            }),
            detail: observation.detail,
        }),
    }
}

fn cockpit_provider_connection(connection: airwallex::Connection) -> CockpitProviderConnection {
    let airwallex::Connection {
        configured,
        updated_at,
        ..
    } = connection;
    CockpitProviderConnection {
        environment: match configured.environment {
            airwallex::Environment::Sandbox => "sandbox",
            airwallex::Environment::Live => "live",
        }
        .into(),
        account_ref: configured.account_ref,
        api_version: configured.api_version,
        read_scopes: configured.read_scopes,
        submit_scopes: configured.submit_scopes,
        approval_workflow_observed: configured.approval_workflow_observed,
        observed_at: configured.observed_at,
        updated_at,
    }
}

fn cockpit_money_envelope(envelope: finance::MoneyEnvelope) -> CockpitMoneyEnvelope {
    let finance::MoneyEnvelope {
        limits,
        period_started_at,
        updated_by,
        updated_at,
    } = envelope;
    CockpitMoneyEnvelope {
        source_account_ref: limits.source_account_ref,
        currency: limits.currency,
        beneficiary_refs: limits.beneficiary_refs,
        per_payment_limit_minor: limits.per_payment_limit_minor,
        aggregate_limit_minor: limits.aggregate_limit_minor,
        frozen: limits.frozen,
        period_started_at,
        updated_by,
        updated_at,
    }
}

fn cockpit_payment_intent(payment: finance::PaymentIntent) -> CockpitPaymentIntent {
    let state = payment.state.as_str().to_string();
    let finance::PaymentIntent {
        request,
        provider,
        provider_transfer_id,
        raw_provider_status,
        provider_approval_url,
        settled_at,
        created_at,
        updated_at,
        ..
    } = payment;
    CockpitPaymentIntent {
        work_id: request.work_id,
        owner_handoff_id: request.owner_handoff_id,
        source_account_ref: request.source_account_ref,
        provider_beneficiary_ref: request.provider_beneficiary_ref,
        amount_minor: request.amount_minor,
        currency: request.currency,
        purpose: request.purpose,
        evidence_refs: request.evidence_refs,
        idempotency_key: request.idempotency_key,
        requesting_actor: request.requesting_actor,
        state,
        provider,
        provider_transfer_id,
        raw_provider_status,
        provider_approval_url,
        settled_at,
        created_at,
        updated_at,
    }
}

impl OwnerConfig {
    pub(crate) fn is_network(&self) -> bool {
        self.entry.network().is_some()
    }

    pub(crate) async fn from_env() -> Result<Self> {
        let default_address = format!("127.0.0.1:{}", crate::port_with_offset(7788)?);
        let address = std::env::var("RESTLESS_OWNER_ADDR")
            .unwrap_or(default_address)
            .parse::<SocketAddr>()
            .context("parse RESTLESS_OWNER_ADDR")?;
        let default_review_address = format!("127.0.0.1:{}", crate::port_with_offset(7794)?);
        let review_address = std::env::var("RESTLESS_REVIEW_ADDR")
            // 7788 is the owner gateway, 7789 the auth broker, 7790 the model
            // gateway, 7791 coordination, 7792 ingress and 7793 Infisical.
            .unwrap_or(default_review_address)
            .parse::<SocketAddr>()
            .context("parse RESTLESS_REVIEW_ADDR")?;
        let entry = EntryMode::from_env().await?;
        let plane_readiness = match entry.network() {
            Some(network) => Some(PlaneReadinessConfig::from_env(network)?),
            None => None,
        };
        // ADR 0007: the loopback bail is conditional on entry mode, never
        // removed. In local mode the network *is* the boundary, so binding
        // beyond loopback would publish an unauthenticated API.
        if entry.network().is_none() {
            ensure_loopback(address, "RESTLESS_OWNER_ADDR")?;
        }
        ensure_loopback(review_address, "RESTLESS_REVIEW_ADDR")?;
        let review_public_url = std::env::var("RESTLESS_REVIEW_PUBLIC_URL")
            .unwrap_or_else(|_| format!("http://{{ticket}}.localhost:{}", review_address.port()));
        validate_review_public_url(&review_public_url, review_address.port())?;
        Ok(Self {
            address,
            review_address,
            review_public_url,
            entry,
            plane_readiness,
        })
    }
}

impl PlaneReadinessConfig {
    fn from_env(network: &crate::entry::NetworkEntry) -> Result<Self> {
        let token = required_env("RESTLESS_PLANE_READINESS_TOKEN")?;
        if token.len() < 32 {
            anyhow::bail!("RESTLESS_PLANE_READINESS_TOKEN must contain at least 32 characters");
        }
        let cell_token = required_env("RESTLESS_CELL_READINESS_TOKEN")?;
        if cell_token.len() < 32 {
            anyhow::bail!("RESTLESS_CELL_READINESS_TOKEN must contain at least 32 characters");
        }
        if bool::from(
            Sha256::digest(token.as_bytes()).ct_eq(&Sha256::digest(cell_token.as_bytes())),
        ) {
            anyhow::bail!("plane and cell readiness tokens must be distinct");
        }
        let runtime_bootstrap_token = required_env("RESTLESS_RUNTIME_BOOTSTRAP_TOKEN")?;
        if runtime_bootstrap_token.len() < 32 {
            anyhow::bail!("RESTLESS_RUNTIME_BOOTSTRAP_TOKEN must contain at least 32 characters");
        }
        if [&token, &cell_token].iter().any(|other| {
            bool::from(
                Sha256::digest(other.as_bytes())
                    .ct_eq(&Sha256::digest(runtime_bootstrap_token.as_bytes())),
            )
        }) {
            anyhow::bail!("Runtime bootstrap and readiness tokens must all be distinct");
        }
        let account_plane_image = required_env("RESTLESS_ACCOUNT_PLANE_IMAGE")?;
        if !immutable_image_reference(&account_plane_image) {
            anyhow::bail!("RESTLESS_ACCOUNT_PLANE_IMAGE must be an immutable OCI @sha256 digest");
        }
        let desired_revision = required_env("RESTLESS_DESIRED_REVISION")?
            .parse::<i64>()
            .context("RESTLESS_DESIRED_REVISION must be a positive integer")?;
        if desired_revision < 1 {
            anyhow::bail!("RESTLESS_DESIRED_REVISION must be at least 1");
        }
        let release_manifest_digest = required_env("RESTLESS_RELEASE_MANIFEST_DIGEST")?;
        if !sha256_digest(&release_manifest_digest) {
            anyhow::bail!("RESTLESS_RELEASE_MANIFEST_DIGEST must be sha256:<64 lowercase hex>");
        }
        Ok(Self {
            token,
            cell_token,
            runtime_bootstrap_token,
            owner_id: network.owner_id(),
            plane_id: network.plane_id(),
            hostname: network.host().into(),
            account_plane_image,
            desired_revision,
            release_manifest_digest,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let file_name = format!("{name}_FILE");
    let direct = std::env::var(name).ok();
    let file = std::env::var(&file_name).ok();
    if direct.is_some() && file.is_some() {
        anyhow::bail!("set only one of {name} and {file_name}");
    }
    let value = match (direct, file) {
        (Some(value), None) => value,
        (None, Some(path)) => {
            std::fs::read_to_string(&path).with_context(|| format!("read {file_name} {path}"))?
        }
        (None, None) => String::new(),
        (Some(_), Some(_)) => unreachable!(),
    };
    let value = value.trim_end_matches(['\r', '\n']).to_string();
    if value.trim().is_empty() {
        anyhow::bail!("network account plane requires {name}");
    }
    Ok(value)
}

fn sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn immutable_image_reference(value: &str) -> bool {
    let Some((repository, digest)) = value.split_once("@sha256:") else {
        return false;
    };
    !repository.is_empty()
        && !repository.contains(char::is_whitespace)
        && digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
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
        entry,
        plane_readiness,
    } = config;
    let state = OwnerState {
        daemon,
        charter_writes: Arc::new(tokio::sync::Mutex::new(())),
        tickets: Arc::new(Mutex::new(HashMap::new())),
        attaches: Arc::new(Mutex::new(HashMap::new())),
        reviews: Arc::new(Mutex::new(HashMap::new())),
        review_public_url,
        entry,
        sessions: Arc::new(SessionStore::default()),
        plane_readiness,
        runtime_bridges: runtime_bridge::Registry::default(),
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
            "/companies/{company}/actors/{actor}/conversation/{message_id}/interrupt",
            post(interrupt_actor_conversation),
        )
        .route(
            "/companies/{company}/actors/{actor}/activity",
            get(agent_activity_live),
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
        // Kept at the existing path so an already-open cockpit stays
        // compatible. Its meaning is input activity, not a background
        // keepalive: callers may renew only after a real desktop event.
        .route(
            "/companies/{company}/browser/heartbeat",
            post(record_activity),
        )
        .route("/companies/{company}/browser/return", post(return_control))
        .fallback(api_not_found)
        .layer(DefaultBodyLimit::max(32 * 1024 * 1024));

    // Where the cockpit's built SPA lives. Deliberately separate from
    // `source_root()`: that answers "where is the Restless source tree", which
    // the plane needs to build the company Runtime image, and a packaged plane
    // has no source tree. Conflating them meant a containerised plane refused
    // to serve its cockpit at all — observed as
    // `owner gateway stopped: /src is not a Restless source tree`.
    let web = match std::env::var("RESTLESS_COCKPIT_DIR") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => runtime::source_root()?.join("web/build"),
    };
    if !web.join("index.html").is_file() {
        anyhow::bail!(
            "cockpit assets are missing at {}; set RESTLESS_COCKPIT_DIR to the built SPA",
            web.display()
        );
    }
    let static_files = ServeDir::new(&web).fallback(ServeFile::new(web.join("index.html")));
    let app = Router::new()
        .nest("/api", api)
        // Ungated on purpose: a fleet probe must be able to ask which release
        // is running without holding a session, and the answer carries release
        // identity only — never company, owner or configuration detail.
        .route("/health", get(release_health))
        .route(
            "/internal/v1/planes/{plane_id}/readiness",
            post(observe_plane_readiness),
        )
        .route("/internal/v1/runtime-bridge", get(open_runtime_bridge))
        .route(
            "/internal/v1/companies/bootstrap",
            post(bootstrap_hosted_company),
        )
        .route(
            "/internal/v1/runtime-bridge/bootstrap",
            post(issue_runtime_bridge_bootstrap),
        )
        .route("/v1/cells/{cell_id}/ready", post(observe_cell_readiness))
        .route("/entry", get(consume_entry_assertion))
        .route("/entry/logout", post(end_entry_session))
        .route("/desktop/{company}", get(open_desktop))
        .route("/desktop/{company}/observe", get(open_observed_desktop))
        .route("/desktop/{company}/control", get(open_controlled_desktop))
        .route("/desktop/{company}/websockify", get(desktop_websocket))
        .route("/desktop/{company}/{*asset}", get(desktop_asset))
        .fallback_service(static_files)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            enforce_owner_boundary,
        ))
        .with_state(state.clone());

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

/// The one place entry is decided.
///
/// Local mode is unchanged from ADR 0001: loopback host, no forwarding claims,
/// same-origin writes. Network mode (ADR 0007) decides access by verifying an
/// assertion at `/entry` and carrying the resulting session, and re-derives
/// company scope from that session on every request.
async fn enforce_owner_boundary(
    State(state): State<OwnerState>,
    request: Request,
    next: Next,
) -> Response<Body> {
    // Machine readiness has its own dedicated bearer boundary and request
    // identity contract. It is deliberately not a browser-origin/session path.
    if (request.uri().path().starts_with("/internal/v1/planes/")
        && request.uri().path().ends_with("/readiness"))
        || request.uri().path() == "/internal/v1/runtime-bridge"
        || request.uri().path() == "/internal/v1/companies/bootstrap"
        || request.uri().path() == "/internal/v1/runtime-bridge/bootstrap"
        || (request.uri().path().starts_with("/v1/cells/")
            && request.uri().path().ends_with("/ready"))
    {
        return next.run(request).await;
    }
    match state.entry.clone() {
        EntryMode::Local => {
            if let Some(reason) =
                local_owner_boundary_violation(request.method(), request.headers())
            {
                return api_error(StatusCode::FORBIDDEN, "local_owner_boundary", reason);
            }
            next.run(request).await
        }
        EntryMode::Network(network) => {
            let path = request.uri().path().to_string();
            let identity = cookie_value(request.headers(), SESSION_COOKIE)
                .and_then(|token| state.sessions.resolve(&token));
            if let Some(refusal) = network_boundary_violation(
                request.method(),
                request.headers(),
                &path,
                network.host(),
                identity.as_ref(),
            ) {
                return api_error(refusal.status, refusal.code, refusal.message);
            }
            next.run(request).await
        }
    }
}

struct BoundaryRefusal {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
}

/// The whole network-mode entry decision, as one pure function.
///
/// Kept separate from the middleware so the composition is testable, and kept
/// in one place because two call sites that each decide scope is how one of
/// them ends up deciding it differently.
fn network_boundary_violation(
    method: &Method,
    headers: &HeaderMap,
    path: &str,
    expected_host: &str,
    identity: Option<&crate::entry::VerifiedIdentity>,
) -> Option<BoundaryRefusal> {
    if let Some(message) = network_origin_violation(method, headers, expected_host) {
        return Some(BoundaryRefusal {
            status: StatusCode::FORBIDDEN,
            code: "network_owner_boundary",
            message,
        });
    }

    // The door itself cannot require a session.
    if path == "/entry" {
        return None;
    }

    // The SPA shell is inert without its APIs, so it is served to an
    // unauthenticated browser and its API calls are refused below. Everything
    // that reads or changes company state is gated.
    if !(path.starts_with("/api/") || path.starts_with("/desktop/")) {
        return None;
    }

    let Some(identity) = identity else {
        return Some(BoundaryRefusal {
            status: StatusCode::UNAUTHORIZED,
            code: "no_session",
            message: "this plane requires a verified entry assertion",
        });
    };

    // S27-T2: scope comes from the verified assertion, never from the route,
    // the host or a forwarding header. Checked per request, not once at entry.
    if let Some(company) = company_in_path(path) {
        if !identity.scope.permits(company) {
            return Some(BoundaryRefusal {
                status: StatusCode::FORBIDDEN,
                code: "company_out_of_scope",
                message: "this session is not scoped to that company",
            });
        }
    }

    None
}

/// Network-mode browser-origin checks. The plane is reached directly, so a
/// forwarding header is still not proof of anything and the Host must be the
/// plane's own configured hostname.
fn network_origin_violation(
    method: &Method,
    headers: &HeaderMap,
    expected_host: &str,
) -> Option<&'static str> {
    let host = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(':')
                .next()
                .unwrap_or(value)
                .to_ascii_lowercase()
        });
    if host.as_deref() != Some(&expected_host.to_ascii_lowercase()) {
        return Some("owner request host is not this plane's configured hostname");
    }

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
        let origin_host = origin.rsplit('/').next().map(|value| {
            value
                .split(':')
                .next()
                .unwrap_or(value)
                .to_ascii_lowercase()
        });
        if origin_host.as_deref() != Some(&expected_host.to_ascii_lowercase()) {
            return Some("owner request origin does not match this plane's hostname");
        }
    }
    None
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(COOKIE)
        .and_then(|value| value.to_str().ok())?
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.to_string())
}

/// Which release this plane is actually running (S27-T4).
async fn release_health() -> Response<Body> {
    Json(serde_json::json!({
        "status": "ok",
        "release": crate::release::ReleaseIdentity::current(),
    }))
    .into_response()
}

/// The Company Runtime initiates this channel. It is deliberately outside the
/// browser session middleware: the first WebSocket frame carries a signed,
/// company-scoped Runtime capability and the registry binds it to this exact
/// network plane before accepting any protocol traffic.
async fn open_runtime_bridge(
    State(state): State<OwnerState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response<Body> {
    let Some(config) = &state.plane_readiness else {
        return api_error(
            StatusCode::NOT_FOUND,
            "runtime_bridge_unavailable",
            "the hosted Runtime Bridge is available only in configured network mode",
        );
    };
    if headers.contains_key(ORIGIN) || headers.contains_key(COOKIE) {
        return api_error(
            StatusCode::FORBIDDEN,
            "runtime_bridge_browser_refused",
            "the Runtime Bridge is a machine channel, not a browser endpoint",
        );
    }
    let registry = state.runtime_bridges.clone();
    let issuer = state.daemon.capabilities.clone();
    let scope = runtime_bridge::PlaneScope {
        owner_id: config.owner_id,
        plane_id: config.plane_id,
    };
    upgrade
        .on_upgrade(move |socket| async move {
            if registry.accept(socket, issuer, scope).await.is_err() {
                // Do not log the registration payload, token, company name or
                // command data at this shared infrastructure boundary.
                tracing::warn!("Runtime Bridge connection refused or ended");
            }
        })
        .into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeBridgeBootstrapRequest {
    contract_version: u32,
    owner_id: Uuid,
    plane_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    runtime_id: String,
    runtime_generation: u64,
    runtime_image: String,
    volume_name: String,
    source_revision: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HostedCompanyBootstrapRequest {
    contract_version: u32,
    owner_id: Uuid,
    plane_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    model: String,
    reasoning_effort: String,
}

async fn bootstrap_hosted_company(
    State(state): State<OwnerState>,
    headers: HeaderMap,
    Json(request): Json<HostedCompanyBootstrapRequest>,
) -> Response<Body> {
    let Some(plane) = &state.plane_readiness else {
        return api_error(
            StatusCode::NOT_FOUND,
            "company_bootstrap_unavailable",
            "company bootstrap is available only in configured network mode",
        );
    };
    if headers.contains_key(ORIGIN)
        || headers.contains_key(COOKIE)
        || !readiness_token_matches(&headers, &plane.runtime_bootstrap_token)
    {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "company_bootstrap_unauthorized",
            "the dedicated Runtime bootstrap credential is required",
        );
    }
    if request.contract_version != 1
        || request.owner_id != plane.owner_id
        || request.plane_id != plane.plane_id
        || request.company_id.is_nil()
        || request.cell_id.is_nil()
        || request.model.is_empty()
        || request.model.len() > 160
        || !matches!(
            request.reasoning_effort.as_str(),
            "none" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra"
        )
    {
        return api_error(
            StatusCode::CONFLICT,
            "company_bootstrap_identity_mismatch",
            "the bootstrap request does not name an exact company on this plane",
        );
    }
    let company = request.company_id.hyphenated().to_string();
    match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(existing)
            if existing.model == request.model
                && existing.reasoning_effort == request.reasoning_effort => {}
        Ok(_) => {
            return api_error(
                StatusCode::CONFLICT,
                "company_bootstrap_configuration_mismatch",
                "the company already exists with a different immutable bootstrap configuration",
            );
        }
        Err(_) => {
            let config = runtime::CompanyConfig {
                name: company.clone(),
                mission: String::new(),
                spend_ceiling_usd: runtime::SpendCeiling::from_micro_usd(10_000_000),
                model: request.model.clone(),
                worker_runtime: Default::default(),
                reasoning_effort: request.reasoning_effort.clone(),
                model_failover: Vec::new(),
                credentials: Default::default(),
                approved_parties: Vec::new(),
            };
            if runtime::CompanyConfig::save(&state.daemon.root, &config).is_err() {
                return api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "company_bootstrap_config_failed",
                    "the account plane could not persist the company configuration",
                );
            }
        }
    }
    if state
        .daemon
        .authority
        .initialise_company(&company, &[])
        .await
        .is_err()
    {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "company_bootstrap_authority_failed",
            "the account plane could not initialise company Authority",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "company_bootstrap_org_failed",
                "the account plane could not initialise company organisation state",
            );
        }
    };
    if crate::ensure_standing_actors(&org, Some(&request.model))
        .await
        .is_err()
    {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "company_bootstrap_actors_failed",
            "the account plane could not initialise standing actors",
        );
    }
    Json(serde_json::json!({
        "contract_version": 1,
        "owner_id": request.owner_id,
        "plane_id": request.plane_id,
        "company_id": request.company_id,
        "cell_id": request.cell_id,
        "model": request.model,
        "reasoning_effort": request.reasoning_effort,
        "status": "ready",
    }))
    .into_response()
}

async fn issue_runtime_bridge_bootstrap(
    State(state): State<OwnerState>,
    headers: HeaderMap,
    Json(request): Json<RuntimeBridgeBootstrapRequest>,
) -> Response<Body> {
    let Some(config) = &state.plane_readiness else {
        return api_error(
            StatusCode::NOT_FOUND,
            "runtime_bootstrap_unavailable",
            "Runtime bootstrap is available only in configured network mode",
        );
    };
    if headers.contains_key(ORIGIN)
        || headers.contains_key(COOKIE)
        || !readiness_token_matches(&headers, &config.runtime_bootstrap_token)
    {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "runtime_bootstrap_unauthorized",
            "the dedicated Runtime bootstrap credential is required",
        );
    }
    let company = request.company_id.hyphenated().to_string();
    let bounded_identity = |value: &str| {
        !value.is_empty()
            && value.len() <= 160
            && value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_uppercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'_' | b'.')
            })
    };
    if request.contract_version != 1
        || request.owner_id != config.owner_id
        || request.plane_id != config.plane_id
        || request.company_id.is_nil()
        || request.cell_id.is_nil()
        || request.runtime_generation == 0
        || !bounded_identity(&request.runtime_id)
        || !bounded_identity(&request.volume_name)
        || !immutable_image_reference(&request.runtime_image)
        || request.source_revision.len() != 40
        || !request
            .source_revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || runtime::CompanyConfig::load(&state.daemon.root, &company).is_err()
    {
        return api_error(
            StatusCode::CONFLICT,
            "runtime_bootstrap_identity_mismatch",
            "the bootstrap request does not name an exact company Runtime on this plane",
        );
    }
    let scope = crate::capability::HostedRuntimeScope {
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
    match state
        .daemon
        .capabilities
        .issue_hosted_runtime_bridge(&company, scope)
    {
        Ok(capability) => Json(serde_json::json!({
            "contract_version": 1,
            "company_id": request.company_id,
            "cell_id": request.cell_id,
            "runtime_generation": request.runtime_generation,
            "capability": capability,
            "valid_for_seconds": 86_400,
        }))
        .into_response(),
        Err(_) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "runtime_bootstrap_failed",
            "the account plane could not issue the scoped Runtime bootstrap grant",
        ),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CellReadinessRequest {
    contract_version: u32,
    owner_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    runtime_id: String,
    runtime_image: String,
    desired_revision: i64,
}

#[derive(Serialize)]
struct CellReadinessCheck {
    kind: &'static str,
    status: &'static str,
}

#[derive(Serialize)]
struct CellReadinessObservation {
    contract_version: u32,
    owner_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    runtime_id: String,
    runtime_image: String,
    desired_revision: i64,
    core_release: &'static str,
    release_manifest_digest: String,
    status: &'static str,
    ready: bool,
    checks: Vec<CellReadinessCheck>,
    observed_at: chrono::DateTime<Utc>,
    valid_until: chrono::DateTime<Utc>,
}

async fn observe_cell_readiness(
    State(state): State<OwnerState>,
    AxumPath(path_cell_id): AxumPath<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CellReadinessRequest>,
) -> Response<Body> {
    let Some(config) = &state.plane_readiness else {
        return api_error(
            StatusCode::NOT_FOUND,
            "cell_readiness_unavailable",
            "cell readiness is available only in configured network mode",
        );
    };
    if !readiness_token_matches(&headers, &config.cell_token) {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "cell_readiness_unauthorized",
            "the dedicated cell readiness credential is required",
        );
    }
    if request.contract_version != 1
        || path_cell_id != request.cell_id
        || request.owner_id != config.owner_id
        || request.company_id.is_nil()
        || request.cell_id.is_nil()
        || request.desired_revision < 1
        || request.runtime_id.is_empty()
        || request.runtime_id.len() > 160
        || !immutable_image_reference(&request.runtime_image)
    {
        return api_error(
            StatusCode::CONFLICT,
            "cell_readiness_identity_mismatch",
            "the readiness request does not name a valid cell on this owner plane",
        );
    }

    let bridge = state.runtime_bridges.observe_cell(request.cell_id);
    if bridge.as_ref().is_some_and(|bridge| {
        bridge.owner_id != request.owner_id
            || bridge.plane_id != config.plane_id
            || bridge.company_id != request.company_id
            || bridge.runtime_id != request.runtime_id
            || bridge.runtime_image != request.runtime_image
            || bridge.desired_revision != request.desired_revision
    }) {
        return api_error(
            StatusCode::CONFLICT,
            "cell_readiness_runtime_drift",
            "the connected Runtime does not match Fleet's exact desired cell revision",
        );
    }

    let mut authority_record = "pending";
    let mut company_database = "pending";
    let mut orgintel = "pending";
    if let Some(bridge) = &bridge {
        authority_record = match sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM restless_authority.company_migrations WHERE company = $1)",
        )
        .bind(&bridge.company)
        .fetch_one(state.daemon.authority.pool())
        .await
        {
            Ok(true) => "ready",
            Ok(false) => "pending",
            Err(_) => "failed",
        };
        match state.daemon.orgintel.get(&bridge.company).await {
            Ok(company) => {
                company_database = "ready";
                orgintel = if company.is_live().await {
                    "ready"
                } else {
                    "failed"
                };
            }
            Err(_) => {
                company_database = "failed";
                orgintel = "failed";
            }
        }
    }

    let runtime = if bridge.is_some() { "ready" } else { "pending" };
    let persistent_volume = match bridge.as_ref() {
        Some(bridge) if bridge.persistent_volume_ready && !bridge.volume_name.is_empty() => "ready",
        Some(_) => "failed",
        None => "pending",
    };
    let activity_observed = if bridge.as_ref().is_some_and(|bridge| {
        bridge
            .supported_features
            .iter()
            .any(|feature| feature == "activity.v1")
    }) {
        state
            .runtime_bridges
            .probe_readiness(request.cell_id)
            .await
            .is_ok()
    } else {
        false
    };
    let runtime_bridge = match bridge.as_ref() {
        Some(bridge) if bridge.has_complete_v1() && activity_observed => "ready",
        Some(_) | None => "pending",
    };
    let checks = vec![
        CellReadinessCheck {
            kind: "runtime",
            status: runtime,
        },
        CellReadinessCheck {
            kind: "authority_record",
            status: authority_record,
        },
        CellReadinessCheck {
            kind: "company_database",
            status: company_database,
        },
        CellReadinessCheck {
            kind: "persistent_volume",
            status: persistent_volume,
        },
        CellReadinessCheck {
            kind: "orgintel",
            status: orgintel,
        },
        CellReadinessCheck {
            kind: "runtime_bridge",
            status: runtime_bridge,
        },
    ];
    let ready = checks.iter().all(|check| check.status == "ready");
    let failed = checks.iter().any(|check| check.status == "failed");
    let observed_at = Utc::now();
    let observation = CellReadinessObservation {
        contract_version: 1,
        owner_id: request.owner_id,
        company_id: request.company_id,
        cell_id: request.cell_id,
        runtime_id: request.runtime_id,
        runtime_image: request.runtime_image,
        desired_revision: request.desired_revision,
        core_release: crate::release::CORE_VERSION,
        release_manifest_digest: config.release_manifest_digest.clone(),
        status: if ready {
            "ready"
        } else if failed {
            "degraded"
        } else {
            "starting"
        },
        ready,
        checks,
        observed_at,
        valid_until: observed_at + ChronoDuration::seconds(20),
    };
    let mut response = Json(observation).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaneReadinessRequest {
    contract_version: u32,
    owner_id: Uuid,
    plane_id: Uuid,
    hostname: String,
    account_plane_image: String,
    desired_revision: i64,
}

#[derive(Serialize)]
struct PlaneReadinessCheck {
    kind: &'static str,
    status: &'static str,
}

#[derive(Serialize)]
struct PlaneReadinessObservation {
    contract_version: u32,
    owner_id: Uuid,
    plane_id: Uuid,
    hostname: String,
    account_plane_image: String,
    desired_revision: i64,
    core_release: &'static str,
    release_manifest_digest: String,
    status: &'static str,
    ready: bool,
    checks: Vec<PlaneReadinessCheck>,
    observed_at: chrono::DateTime<Utc>,
    valid_until: chrono::DateTime<Utc>,
}

async fn observe_plane_readiness(
    State(state): State<OwnerState>,
    AxumPath(path_plane_id): AxumPath<Uuid>,
    headers: HeaderMap,
    Json(request): Json<PlaneReadinessRequest>,
) -> Response<Body> {
    let Some(config) = &state.plane_readiness else {
        return api_error(
            StatusCode::NOT_FOUND,
            "plane_readiness_unavailable",
            "plane readiness is available only in configured network mode",
        );
    };
    if !readiness_token_matches(&headers, &config.token) {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "plane_readiness_unauthorized",
            "the dedicated plane readiness credential is required",
        );
    }
    if request.contract_version != 1
        || path_plane_id != config.plane_id
        || request.owner_id != config.owner_id
        || request.plane_id != config.plane_id
        || request.hostname != config.hostname
        || request.account_plane_image != config.account_plane_image
        || request.desired_revision != config.desired_revision
    {
        return api_error(
            StatusCode::CONFLICT,
            "plane_readiness_identity_mismatch",
            "the readiness request does not name this exact deployed plane revision",
        );
    }

    let pool = state.daemon.authority.pool();
    let plane_database = sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(pool);
    let authority = sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('restless_authority.records') IS NOT NULL",
    )
    .fetch_one(pool);
    let credential_custody = crate::credential::probe_custody();
    let (plane_database, authority, credential_custody) =
        tokio::join!(plane_database, authority, credential_custody);

    let checks = vec![
        PlaneReadinessCheck {
            kind: "authority",
            status: if authority.is_ok_and(|present| present) {
                "ready"
            } else {
                "failed"
            },
        },
        PlaneReadinessCheck {
            kind: "credential_custody",
            status: if credential_custody.is_ok() {
                "ready"
            } else {
                "failed"
            },
        },
        PlaneReadinessCheck {
            kind: "company_directory",
            status: if crate::configured_companies(&state.daemon.root).is_ok() {
                "ready"
            } else {
                "failed"
            },
        },
        PlaneReadinessCheck {
            kind: "identity_handoff",
            status: if state.entry.network().is_some() {
                "ready"
            } else {
                "failed"
            },
        },
        // serve() refuses to install routes unless the built cockpit exists.
        PlaneReadinessCheck {
            kind: "cockpit",
            status: "ready",
        },
        PlaneReadinessCheck {
            kind: "plane_database",
            status: if plane_database.is_ok() {
                "ready"
            } else {
                "failed"
            },
        },
    ];
    let ready = checks.iter().all(|check| check.status == "ready");
    let observed_at = Utc::now();
    let observation = PlaneReadinessObservation {
        contract_version: 1,
        owner_id: config.owner_id,
        plane_id: config.plane_id,
        hostname: config.hostname.clone(),
        account_plane_image: config.account_plane_image.clone(),
        desired_revision: config.desired_revision,
        core_release: crate::release::CORE_VERSION,
        release_manifest_digest: config.release_manifest_digest.clone(),
        status: if ready { "ready" } else { "degraded" },
        ready,
        checks,
        observed_at,
        valid_until: observed_at + ChronoDuration::seconds(20),
    };
    let mut response = Json(observation).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn readiness_token_matches(headers: &HeaderMap, expected: &str) -> bool {
    let Some(candidate) = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return false;
    };
    let candidate = Sha256::digest(candidate.as_bytes());
    let expected = Sha256::digest(expected.as_bytes());
    bool::from(candidate.ct_eq(&expected))
}

/// Ordinary session revocation. ADR 0007 requires that a removed membership
/// ends by revoking the session, not by waiting for an assertion to expire.
async fn end_entry_session(State(state): State<OwnerState>, headers: HeaderMap) -> Response<Body> {
    if let Some(token) = cookie_value(&headers, SESSION_COOKIE) {
        state.sessions.revoke(&token);
    }
    let mut response = Json(serde_json::json!({ "ended": true })).into_response();
    if let Ok(value) = HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}=; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=0"
    )) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
    response
}

#[derive(Deserialize)]
struct EntryRequest {
    assertion: String,
}

/// The door. Consumes one single-use assertion and exchanges it for a session.
///
/// Fleet redirects to this exact route. On success the assertion is removed
/// immediately with a same-origin 303 and replaced by a host-only cookie.
async fn consume_entry_assertion(
    State(state): State<OwnerState>,
    Query(request): Query<EntryRequest>,
) -> Response<Body> {
    let Some(network) = state.entry.network().cloned() else {
        return api_error(
            StatusCode::NOT_FOUND,
            "local_entry",
            "this plane is in local mode and has no assertion entry point",
        );
    };

    let verified = match network.verify(&request.assertion) {
        Ok(assertion) => assertion,
        Err(refusal) => {
            tracing::warn!(reason = refusal.code(), "refused entry assertion");
            return api_error(StatusCode::UNAUTHORIZED, refusal.code(), refusal.message());
        }
    };

    match crate::entry::consume_once(state.daemon.authority.pool(), &verified).await {
        Ok(true) => {}
        Ok(false) => {
            let refusal = crate::entry::Refusal::Replayed;
            tracing::warn!(reason = refusal.code(), "refused entry assertion");
            return api_error(StatusCode::UNAUTHORIZED, refusal.code(), refusal.message());
        }
        Err(error) => {
            tracing::error!(error = %error, "entry replay store unavailable");
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "entry_store_unavailable",
                "the plane could not durably consume this assertion",
            );
        }
    }
    let identity = verified.identity;

    tracing::info!(
        user = %identity.user,
        owner = %identity.owner,
        role = %identity.role,
        actor = identity.actor.as_deref().unwrap_or("-"),
        correlation = identity.correlation.as_deref().unwrap_or("-"),
        "admitted a verified entry assertion"
    );
    let token = state.sessions.establish(identity, network.session_ttl());

    let cookie = format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age={}",
        network.session_ttl().as_secs()
    );
    let mut response = Redirect::to("/").into_response();
    *response.status_mut() = StatusCode::SEE_OTHER;
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
    response
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
        catalog.push(company_catalog_entry(&state, config, "active").await);
    }
    for company in archived {
        let config = match runtime::CompanyConfig::load_archived(&state.daemon.root, &company) {
            Ok(config) => config,
            Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
        };
        catalog.push(company_catalog_entry(&state, config, "archived").await);
    }
    Json(catalog).into_response()
}

async fn company_catalog_entry(
    state: &OwnerState,
    config: runtime::CompanyConfig,
    lifecycle_status: &'static str,
) -> CompanyCatalogEntry {
    let runtime_status = runtime_status(state, &config.name).await;
    let name = config.name.clone();
    CompanyCatalogEntry {
        id: config.name.clone(),
        name: company_display_name(&config.name),
        mission: config.mission,
        model: config.model,
        spend_ceiling_usd: config.spend_ceiling_usd.as_usd(),
        runtime_status,
        lifecycle_status,
        unstartable_reason: crate::model_gateway::unstartable_reason(&name),
    }
}

async fn runtime_status(state: &OwnerState, company: &str) -> &'static str {
    if matches!(&state.entry, EntryMode::Network(_)) {
        return if state.runtime_bridges.observe(company).is_some() {
            "running"
        } else {
            "unavailable"
        };
    }
    match runtime::status(company).await {
        Ok(runtime::ContainerStatus::Running) => "running",
        Ok(runtime::ContainerStatus::Stopped) => "stopped",
        Ok(runtime::ContainerStatus::Absent) => "absent",
        Err(_) => "unavailable",
    }
}

async fn runtime_generation(state: &OwnerState, company: &str) -> Option<String> {
    if matches!(&state.entry, EntryMode::Network(_)) {
        return state
            .runtime_bridges
            .observe(company)
            .map(|observation| observation.runtime_id);
    }
    runtime::generation(company).await.ok().flatten()
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
    if matches!(&state.entry, EntryMode::Network(_)) {
        if let Err(error) = state
            .runtime_bridges
            .write_file(&company, "/company/mission.md", config.mission.as_bytes())
            .await
        {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "charter_projection",
                format!(
                    "the charter was saved but its hosted Runtime projection failed: {error:#}"
                ),
            );
        }
    }
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

    let mut source_health = BTreeMap::new();
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => {
            source_health.insert("orgintel".into(), "available".into());
            Some(org)
        }
        Err(error) => {
            source_health.insert("orgintel".into(), format!("unavailable: {error}"));
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
                source_health.insert("orgintel".into(), format!("unavailable: {error}"));
                (Vec::new(), Vec::new(), Vec::new(), Vec::new())
            }
        }
    } else {
        (Vec::new(), Vec::new(), Vec::new(), Vec::new())
    };

    let spend_breakdown = state.daemon.spend.breakdown(&company);
    let budget = state.daemon.spend.budget_state(&config);
    let accounted_usd = budget.accounted_micro_usd() as f64 / 1_000_000.0;
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
            CockpitPerson {
                actor_id: actor.id.clone(),
                kind: actor.kind.clone(),
                role: actor.role.clone(),
                display: actor.display.clone(),
                model: actor.model.clone(),
                team_id: actor.team_id,
                spent_usd: round_owner_usd(spent),
                session_running,
                session_observed_at: session_running.then_some(observed_at),
                model_cooldown: actor
                    .model
                    .as_deref()
                    .and_then(|model| cooldowns.iter().find(|cooldown| cooldown.model == model))
                    .map(|cooldown| CockpitModelCooldown {
                        model: cooldown.model.clone(),
                        kind: cooldown.kind.clone(),
                        reason: cooldown.reason.clone(),
                        retry_at: cooldown.retry_at,
                    }),
            }
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
            CockpitTeam {
                id: team.id,
                name: team.name.clone(),
                brief: team.brief.clone(),
                lead_actor_id: team.lead_actor_id.clone(),
                created_by: team.created_by.clone(),
                created_at: team.created_at,
                member_count,
                in_motion_count,
                blocked_count,
            }
        })
        .collect::<Vec<_>>();

    let approved_parties = match approval::approved_parties(&state.daemon.authority, &company).await
    {
        Ok(parties) => {
            source_health.insert("authority".into(), "available".into());
            parties
        }
        Err(error) => {
            source_health.insert("authority".into(), format!("unavailable: {error}"));
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
            .map(|event| CockpitEffectReceipt {
                id: event.id,
                effect_class: event
                    .body
                    .get("effect_class")
                    .or_else(|| event.body.get("capability"))
                    .cloned(),
                tool: event.body.get("tool").cloned(),
                success: event.body.get("success").cloned(),
                party: event.body.get("party").cloned(),
                actor: event
                    .body
                    .get("actor")
                    .cloned()
                    .or_else(|| event.actor_id.clone().map(serde_json::Value::String)),
                outcome: event.body.get("outcome").cloned(),
                evidence_quality: if reconcile::is_governed_receipt(&event.body) {
                    CockpitEvidenceQuality::Governed
                } else {
                    CockpitEvidenceQuality::LegacyUnverified
                },
                at: event.created_at,
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            source_health.insert("authority".into(), format!("unavailable: {error}"));
            Vec::new()
        }
    };

    let mut credentials = Vec::with_capacity(config.credentials.len());
    for (binding, reference) in &config.credentials {
        if query.probe_credentials {
            let probe = credential::probe_reference(reference).await;
            credentials.push(CockpitCredential {
                binding: binding.clone(),
                status: probe.status.as_str().into(),
                detail: probe.detail,
            });
        } else {
            credentials.push(CockpitCredential {
                binding: binding.clone(),
                status: "configured_unprobed".into(),
                detail: Some(
                    "A governed reference is configured. Availability was not probed by this read."
                        .into(),
                ),
            });
        }
    }

    let legal_profile = match legal::get_profile(&state.daemon.authority, &company).await {
        Ok(profile) => CockpitLegal {
            status: "available".into(),
            profile: profile.map(cockpit_legal_profile),
            detail: None,
        },
        Err(error) => CockpitLegal {
            status: "unavailable".into(),
            profile: None,
            detail: Some(format!("{error:#}")),
        },
    };
    let provider = match airwallex::connection(&state.daemon.authority, &company).await {
        Ok(connection) => CockpitProvider {
            status: "available".into(),
            connection: connection.map(cockpit_provider_connection),
            detail: None,
        },
        Err(error) => CockpitProvider {
            status: "unavailable".into(),
            connection: None,
            detail: Some(format!("{error:#}")),
        },
    };
    let finance_state = match tokio::try_join!(
        finance::envelopes(&state.daemon.authority, &company),
        finance::payments(&state.daemon.authority, &company),
        state
            .daemon
            .authority
            .records_of_kind(&company, "finance_balance_observed")
    ) {
        Ok((envelopes, payments, balances)) => CockpitFinance {
            status: "available".into(),
            envelopes: envelopes.into_iter().map(cockpit_money_envelope).collect(),
            payments: payments.into_iter().map(cockpit_payment_intent).collect(),
            last_balance_observation: balances.last().map(|row| CockpitBalanceObservation {
                observed_at: row.created_at,
                body: row.body.clone(),
            }),
            detail: None,
        },
        Err(error) => CockpitFinance {
            status: "unavailable".into(),
            envelopes: Vec::new(),
            payments: Vec::new(),
            last_balance_observation: None,
            detail: Some(format!("{error:#}")),
        },
    };

    let runtime_status = runtime_status(&state, &company).await;
    source_health.insert("runtime".into(), runtime_status.into());

    let remaining_usd = budget
        .remaining_micro_usd()
        .map(|remaining| round_owner_usd(remaining as f64 / 1_000_000.0));
    let spend_status = match budget {
        crate::spend::ModelBudgetState::Available { .. } => "available",
        crate::spend::ModelBudgetState::Exhausted { .. } => "exhausted",
        crate::spend::ModelBudgetState::MeteringUnknown { .. } => "metering_unknown",
    };
    Json(CockpitView {
        company: CockpitCompany {
            id: company,
            name: config.name,
            mission: config.mission,
            model: config.model,
        },
        source_health,
        people,
        teams: team_rows,
        goals: goals
            .into_iter()
            .map(|goal| CockpitGoal {
                id: goal.id,
                title: goal.title,
                body: goal.body,
                created_by: goal.created_by,
                created_at: goal.created_at,
                closed_at: goal.closed_at,
            })
            .collect(),
        spend: CockpitSpend {
            accounted_usd: round_owner_usd(accounted_usd),
            ceiling_usd: config.spend_ceiling_usd.as_usd(),
            remaining_usd,
            status: spend_status.into(),
        },
        authority: CockpitAuthority {
            approved_parties,
            credentials,
            legal: legal_profile,
            provider,
            finance: finance_state,
        },
        receipts,
        refreshed_at: observed_at,
    })
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

#[cfg(test)]
fn render_cockpit_bindings() -> String {
    use ts_rs::TS;

    let config = ts_rs::Config::new().with_large_int("number");
    let mut rendered = String::from(
        "// GENERATED — do not edit.\n\
         //\n\
         // Source: crates/restlessd/src/owner.rs (the owner projection writer).\n\
         // Regenerate: RESTLESS_WRITE_COCKPIT_BINDINGS=1 cargo test -p restlessd cockpit_typescript_bindings_match\n\
         //\n\
         // This is the cockpit response contract, not a client-side view-model.\n\
         \n",
    );
    for declaration in [
        serde_json::Value::decl(&config),
        CockpitCompany::decl(&config),
        CockpitModelCooldown::decl(&config),
        CockpitPerson::decl(&config),
        CockpitTeam::decl(&config),
        CockpitGoal::decl(&config),
        CockpitSpend::decl(&config),
        CockpitCredential::decl(&config),
        CockpitRegistrationIdentifier::decl(&config),
        CockpitRegistryObservation::decl(&config),
        CockpitLegalProfile::decl(&config),
        CockpitLegal::decl(&config),
        CockpitProviderConnection::decl(&config),
        CockpitProvider::decl(&config),
        CockpitMoneyEnvelope::decl(&config),
        CockpitPaymentIntent::decl(&config),
        CockpitBalanceObservation::decl(&config),
        CockpitFinance::decl(&config),
        CockpitEvidenceQuality::decl(&config),
        CockpitEffectReceipt::decl(&config),
        CockpitAuthority::decl(&config),
        CockpitView::decl(&config),
    ] {
        rendered.push_str("export ");
        for (index, line) in declaration.lines().enumerate() {
            if index > 0 {
                rendered.push('\n');
            }
            rendered.push_str(line.trim_end());
        }
        rendered.push_str("\n\n");
    }
    // Generated source should end like ordinary checked-in text: one final
    // newline, not a semantically meaningless blank paragraph.
    rendered.truncate(rendered.trim_end_matches('\n').len());
    rendered.push('\n');
    rendered
}

#[cfg(test)]
fn render_conversation_bindings() -> String {
    use ts_rs::TS;

    let config = ts_rs::Config::new().with_large_int("number");
    let mut rendered = String::from(
        "// GENERATED — do not edit.\n\
         //\n\
         // Source: crates/restlessd/src/owner.rs and crates/restlessd/src/activity.rs.\n\
         // Regenerate: RESTLESS_WRITE_CONVERSATION_BINDINGS=1 cargo test -p restlessd conversation_typescript_bindings_match\n\
         //\n\
         // Shared owner conversation and live-turn response contract.\n\
         \n",
    );
    for declaration in [
        OwnerAttachment::decl(&config),
        OwnerIntentKind::decl(&config),
        OwnerIntentReceipt::decl(&config),
        ConversationActorView::decl(&config),
        ConversationFocusView::decl(&config),
        ConversationMessageView::decl(&config),
        ConversationView::decl(&config),
        ConversationSendResponse::decl(&config),
        ConversationInterruptResponse::decl(&config),
        crate::activity::AgentActivityPhase::decl(&config),
        crate::activity::AgentActivityItem::decl(&config),
        crate::activity::AgentContextUsage::decl(&config),
        crate::activity::AgentActivityState::decl(&config),
    ] {
        rendered.push_str("export ");
        for (index, line) in declaration.lines().enumerate() {
            if index > 0 {
                rendered.push('\n');
            }
            rendered.push_str(line.trim_end());
        }
        rendered.push_str("\n\n");
    }
    rendered.truncate(rendered.trim_end_matches('\n').len());
    rendered.push('\n');
    rendered
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
    Json(ConversationView {
        actor: ConversationActorView {
            id: actor_row.id,
            display: actor_row.display,
            kind: actor_row.kind,
            role: actor_row.role,
        },
        focus: focus.map(|focus| ConversationFocusView {
            after_message_id: focus.after_message_id,
            started_at: focus.started_at,
        }),
        messages: messages
            .into_iter()
            .map(conversation_message_view)
            .collect(),
    })
    .into_response()
}

fn conversation_message_view(message: restless_orgintel::MessageRow) -> ConversationMessageView {
    let (body, intent) = split_intent_receipt(&message.body);
    let (body, details) = split_message_details(body);
    let (body, attachments) = split_attachment_block(body);
    let (body, context_path) = split_context_marker(body);
    ConversationMessageView {
        id: message.id,
        from_actor: message.from_actor,
        to_actor: message.to_actor,
        body: body.to_string(),
        attachments,
        details,
        intent,
        context_path,
        created_at: message.created_at,
        read_at: message.read_at,
    }
}

/// Reconnectable live projection for one agent turn. This endpoint never
/// invents durable transcript, Work, or Attempt rows: it carries only the
/// in-flight ACP state until OrgIntel records the final outcome.
async fn agent_activity_live(
    State(state): State<OwnerState>,
    AxumPath((company, actor)): AxumPath<(String, String)>,
    Query(query): Query<AgentActivityQuery>,
) -> Response<Body> {
    if query.message_id.is_some() && query.work_id.is_some() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "activity",
            "choose either message_id or work_id, not both",
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

    let receiver =
        state
            .daemon
            .activities
            .subscribe(&company, &actor, query.message_id, query.work_id);
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
                .event("activity")
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
                rollback_attachments(&state, &company, &stored).await;
                return api_error(StatusCode::BAD_REQUEST, "attachment", error.to_string());
            }
        };
        match store_owner_attachment(&state, &company, upload_id, &attachment.bytes, &sidecar).await
        {
            Ok(path) => {
                debug_assert_eq!(path, metadata.path);
                stored.push(metadata);
            }
            Err(error) => {
                rollback_attachments(&state, &company, &stored).await;
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
                .activities
                .expect_message(&company, &actor, message_id, input.work_id);
            // Persist the new direction before interrupting. The next wake
            // discovers it from OrgIntel; the cancelled turn never needs the
            // owner to repeat or confirm their message.
            let interrupted = if input.interrupt {
                if actor == "exec" {
                    state
                        .daemon
                        .in_flight
                        .lock()
                        .map(|mut claims| claims.interrupt(&company))
                        .unwrap_or(false)
                } else {
                    state.daemon.staff.interrupt(&company, &actor)
                }
            } else {
                false
            };
            Json(ConversationSendResponse {
                message_id,
                interrupted,
                context_attached: context_path.is_some(),
                context_omitted,
                focus: focus.map(|focus| ConversationFocusView {
                    after_message_id: focus.after_message_id,
                    started_at: focus.started_at,
                }),
            })
            .into_response()
        }
        Err(error) => {
            rollback_attachments(&state, &company, &stored).await;
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    }
}

/// Stop one ordinary owner conversation turn without adding synthetic prose to
/// its durable transcript. The message is marked consumed atomically so a
/// daemon restart cannot silently replay a direction the owner has cancelled.
async fn interrupt_actor_conversation(
    State(state): State<OwnerState>,
    AxumPath((company, actor, message_id)): AxumPath<(String, String, i64)>,
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
    let actor_exists = match org.list_actors().await {
        Ok(actors) => actors.iter().any(|row| row.id == actor),
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    if !actor_exists {
        return api_error(
            StatusCode::NOT_FOUND,
            "actor",
            "requesting actor no longer exists",
        );
    }

    let cancelled = match org
        .interrupt_owner_conversation_message(&actor, message_id)
        .await
    {
        Ok(cancelled) => cancelled,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    if !cancelled {
        return api_error(
            StatusCode::CONFLICT,
            "conversation",
            "message is no longer an unread ordinary owner conversation",
        );
    }

    // End the reconnectable projection first so every attached client sees a
    // terminal state promptly. The durable `read_at` above is still the
    // source of truth if this process restarts between either operation.
    state.daemon.activities.interrupt_message(
        &company,
        &actor,
        message_id,
        "Interrupted by owner.",
    );
    let interrupted = if actor == "exec" {
        state
            .daemon
            .in_flight
            .lock()
            .map(|mut claims| claims.interrupt(&company))
            .unwrap_or(false)
    } else {
        state.daemon.staff.interrupt(&company, &actor)
    };

    Json(ConversationInterruptResponse {
        message_id,
        cancelled,
        interrupted,
    })
    .into_response()
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
            Some("interrupt") => {
                let value = field
                    .text()
                    .await
                    .map_err(|error| format!("read interruption request: {error}"))?;
                input.interrupt = match value.trim() {
                    "true" => true,
                    "false" | "" => false,
                    _ => return Err("interrupt must be true or false".into()),
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
            (visible.trim_end(), Some(receipt))
        }
        _ => (visible.trim_end(), None),
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

async fn store_owner_attachment(
    state: &OwnerState,
    company: &str,
    attachment_id: Uuid,
    bytes: &[u8],
    metadata: &[u8],
) -> Result<String> {
    if !matches!(&state.entry, EntryMode::Network(_)) {
        return runtime::store_owner_attachment(company, attachment_id, bytes, metadata).await;
    }
    let directory = format!("/company/inbox/owner-attachments/{attachment_id}");
    let content = format!("{directory}/content");
    let sidecar = format!("{directory}/metadata.json");
    state
        .runtime_bridges
        .write_file(company, &content, bytes)
        .await?;
    if let Err(error) = state
        .runtime_bridges
        .write_file(company, &sidecar, metadata)
        .await
    {
        let _ = state.runtime_bridges.remove_file(company, &content).await;
        return Err(error);
    }
    Ok(content)
}

async fn remove_owner_attachment(
    state: &OwnerState,
    company: &str,
    attachment_id: Uuid,
) -> Result<()> {
    if !matches!(&state.entry, EntryMode::Network(_)) {
        return runtime::remove_owner_attachment(company, attachment_id).await;
    }
    let directory = format!("/company/inbox/owner-attachments/{attachment_id}");
    let content = state
        .runtime_bridges
        .remove_file(company, &format!("{directory}/content"))
        .await;
    let sidecar = state
        .runtime_bridges
        .remove_file(company, &format!("{directory}/metadata.json"))
        .await;
    content.and(sidecar)
}

async fn rollback_attachments(state: &OwnerState, company: &str, attachments: &[OwnerAttachment]) {
    for attachment in attachments {
        if let Err(error) = remove_owner_attachment(state, company, attachment.upload_id).await {
            tracing::warn!(%error, %company, attachment = %attachment.upload_id, "failed to roll back owner attachment");
        }
    }
}

async fn download_attachment(
    State(state): State<OwnerState>,
    AxumPath((company, attachment)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    let read = if matches!(&state.entry, EntryMode::Network(_)) {
        let directory = format!("/company/inbox/owner-attachments/{attachment}");
        match state
            .runtime_bridges
            .read_file(
                &company,
                &format!("{directory}/content"),
                MAX_ATTACHMENT_BYTES,
            )
            .await
        {
            Ok(bytes) => state
                .runtime_bridges
                .read_file(&company, &format!("{directory}/metadata.json"), 64 * 1024)
                .await
                .map(|metadata| (bytes, metadata)),
            Err(error) => Err(error),
        }
    } else {
        runtime::read_owner_attachment(&company, attachment).await
    };
    let (bytes, metadata) = match read {
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
    let org = state.daemon.orgintel.get(&company).await.ok();
    match approval::decline(
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
            "this item has no prepared outcome the cockpit can open",
        );
    };
    let current = runtime_generation(&state, &company).await;
    if current.as_deref() != Some(reference.generation.as_str()) {
        return api_error(
            StatusCode::CONFLICT,
            "runtime",
            "runtime generation changed; refresh the review",
        );
    }
    let (source, path_and_query) = if reference.kind == "runtime-file" {
        let (root, entry) = match runtime::runtime_review_file_root(&reference.uri) {
            Ok(value) => value,
            Err(error) => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "review",
                    format!("invalid review target: {error:#}"),
                )
            }
        };
        // Observe the exact file before claiming the outcome opens. The cockpit
        // must never frame a target it has not seen.
        let probe = if matches!(&state.entry, EntryMode::Network(_)) {
            state
                .runtime_bridges
                .read_file_large(
                    &company,
                    &reference.uri,
                    runtime::MAX_REVIEW_FILE_BYTES as usize,
                )
                .await
                .map(|_| ())
        } else {
            runtime::probe_runtime_review_file(&company, &reference.uri).await
        };
        if let Err(error) = probe {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "review",
                format!("prepared outcome is unavailable: {error:#}"),
            );
        }
        let path = format!("/{entry}");
        (ReviewSource::Files { root, entry }, path)
    } else {
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
        let probe = if matches!(&state.entry, EntryMode::Network(_)) {
            match state
                .runtime_bridges
                .open_tcp_stream(&company, target.port)
                .await
            {
                Ok(stream) => runtime::runtime_http_request_on(
                    stream,
                    target.port,
                    Method::HEAD,
                    &target.path_and_query,
                    &HeaderMap::new(),
                )
                .await
                .and_then(|response| {
                    if response.status().is_success() || response.status().is_redirection() {
                        Ok(())
                    } else {
                        anyhow::bail!("runtime review target returned {}", response.status())
                    }
                }),
                Err(error) => Err(error),
            }
        } else {
            runtime::probe_runtime_http(&company, &reference.uri).await
        };
        if let Err(error) = probe {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "review",
                format!("live outcome is unavailable: {error:#}"),
            );
        }
        let path = target.path_and_query.clone();
        (ReviewSource::Service { port: target.port }, path)
    };

    let ticket = Uuid::new_v4().simple().to_string();
    let (review_url, expected_host) =
        match materialize_review_url(&state.review_public_url, &ticket, &path_and_query) {
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
            source,
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
    let current = runtime_generation(&state, &session.company).await;
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
    let upstream = match &session.source {
        ReviewSource::Service { port } => {
            let request = if matches!(&state.entry, EntryMode::Network(_)) {
                match state
                    .runtime_bridges
                    .open_tcp_stream(&session.company, *port)
                    .await
                {
                    Ok(stream) => {
                        runtime::runtime_http_request_on(stream, *port, method, path, &headers)
                            .await
                    }
                    Err(error) => Err(error),
                }
            } else {
                runtime::runtime_http_request(&session.company, *port, method, path, &headers).await
            };
            match request {
                Ok(response) => response,
                Err(error) => {
                    return api_error(
                        StatusCode::BAD_GATEWAY,
                        "review",
                        format!("live outcome bridge: {error:#}"),
                    )
                }
            }
        }
        ReviewSource::Files { root, entry } => {
            // The page being served is company-authored, so this is exactly
            // where a traversal out of the prepared outcome would be tried.
            let resolved = match runtime::resolve_review_file(root, entry, path) {
                Ok(resolved) => resolved,
                Err(error) => {
                    return api_error(StatusCode::NOT_FOUND, "review", format!("{error:#}"))
                }
            };
            let read = if matches!(&state.entry, EntryMode::Network(_)) {
                let path = resolved.to_string_lossy();
                state
                    .runtime_bridges
                    .read_file_large(
                        &session.company,
                        &path,
                        runtime::MAX_REVIEW_FILE_BYTES as usize,
                    )
                    .await
                    .and_then(|bytes| {
                        let media_type = runtime::review_file_media_type(&resolved)
                            .context("review file is not a displayable format")?;
                        Ok((media_type, bytes))
                    })
            } else {
                runtime::read_runtime_review_file(&session.company, &resolved).await
            };
            match read {
                Ok((media_type, bytes)) => {
                    let body = if method == Method::HEAD {
                        Body::empty()
                    } else {
                        Body::from(bytes)
                    };
                    let mut response = Response::new(body);
                    response
                        .headers_mut()
                        .insert(CONTENT_TYPE, HeaderValue::from_static(media_type));
                    return finish_review_response(response, &session, path);
                }
                Err(error) => {
                    return api_error(StatusCode::NOT_FOUND, "review", format!("{error:#}"))
                }
            }
        }
    };
    let (parts, body) = upstream.into_parts();
    let response = Response::from_parts(parts, Body::new(body));
    finish_review_response(response, &session, path)
}

/// One header policy for every isolated review origin, whatever it read from.
fn finish_review_response(
    mut response: Response<Body>,
    session: &ReviewSession,
    path: &str,
) -> Response<Body> {
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
    let current = match runtime_generation(&state, &company).await {
        Some(generation) => generation,
        None => {
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
    let current = runtime_generation(&state, &company).await;
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
    let control = read_browser_control(&state, &company).await.ok().flatten();
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
    let bytes = if matches!(&state.entry, EntryMode::Network(_)) {
        state.runtime_bridges.desktop_asset(&company, &asset).await
    } else {
        runtime::desktop_asset(&company, &asset).await
    };
    match bytes {
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
    let hosted_bridges =
        matches!(&state.entry, EntryMode::Network(_)).then(|| state.runtime_bridges.clone());
    upgrade
        .on_upgrade(move |socket| async move {
            if let Err(error) = proxy_websocket(socket, &company, hosted_bridges).await {
                tracing::warn!(company, "desktop websocket ended: {error:#}");
            }
        })
        .into_response()
}

async fn proxy_websocket(
    browser: WebSocket,
    company: &str,
    hosted_bridges: Option<runtime_bridge::Registry>,
) -> Result<()> {
    if let Some(registry) = hosted_bridges {
        let stream = registry.open_tcp_stream(company, 6080).await?;
        return proxy_websocket_stream(browser, stream).await;
    }
    proxy_websocket_stream(browser, runtime::desktop_stream(company).await?).await
}

async fn proxy_websocket_stream<S>(browser: WebSocket, stream: S) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
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

async fn read_browser_control(
    state: &OwnerState,
    company: &str,
) -> Result<Option<serde_json::Value>> {
    if !matches!(&state.entry, EntryMode::Network(_)) {
        return runtime::read_browser_control(company).await;
    }
    let bytes = match state
        .runtime_bridges
        .read_file(company, "/company/run/browser-control.json", 64 * 1024)
        .await
    {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    let value = serde_json::from_slice(&bytes).context("parse hosted browser controller state")?;
    Ok(Some(runtime::normalize_expired_browser_control(value)))
}

async fn write_browser_control(
    state: &OwnerState,
    company: &str,
    value: &serde_json::Value,
) -> Result<()> {
    if !matches!(&state.entry, EntryMode::Network(_)) {
        return runtime::write_browser_control(company, value).await;
    }
    state
        .runtime_bridges
        .write_file(
            company,
            "/company/run/browser-control.json",
            &serde_json::to_vec(value)?,
        )
        .await
}

async fn browser_status(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
) -> impl IntoResponse {
    if matches!(&state.entry, EntryMode::Network(_)) {
        let Some(observation) = state.runtime_bridges.observe(&company) else {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "runtime",
                "hosted Runtime Bridge is not connected",
            );
        };
        let available = observation
            .supported_features
            .iter()
            .any(|feature| feature == "desktop.v1");
        return Json(serde_json::json!({
            "generation": observation.runtime_id,
            "browser": {
                "status": if available { "available" } else { "degraded" },
                "desktop": if available { "running" } else { "unknown" },
                "chromium": if available { "running" } else { "unknown" },
                "automation": if available { "available" } else { "unknown" },
                "web_transport": if available { "running" } else { "unknown" },
                "controller": "observed",
            },
            "control": read_browser_control(&state, &company).await.ok().flatten(),
        }))
        .into_response();
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
    let prior = read_browser_control(&state, &company).await.ok().flatten();
    if let Some(prior) = prior.as_ref() {
        if prior["controller"] == "owner"
            && prior["client_id"].as_str() != Some(input.client_id.as_str())
            && lease_is_live(prior)
        {
            return api_error(
                StatusCode::CONFLICT,
                "controller",
                "another owner tab already controls this browser",
            );
        }
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
        "last_activity_at": Utc::now(),
        "expires_at": Utc::now() + ChronoDuration::seconds(CONTROL_TTL_SECONDS),
    });
    match write_browser_control(&state, &company, &state_value).await {
        Ok(()) => Json(state_value).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime",
            format!("{error:#}"),
        ),
    }
}

async fn record_activity(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<ControlRequest>,
) -> impl IntoResponse {
    let Some(mut current) = read_browser_control(&state, &company).await.ok().flatten() else {
        return api_error(
            StatusCode::CONFLICT,
            "controller",
            "browser is not owner-controlled; desktop input cannot renew a lease",
        );
    };
    if current["controller"] != "owner" || current["client_id"].as_str() != Some(&input.client_id) {
        return api_error(
            StatusCode::CONFLICT,
            "controller",
            "this browser tab does not hold control",
        );
    }
    current["last_activity_at"] = serde_json::json!(Utc::now());
    current["expires_at"] =
        serde_json::json!(Utc::now() + ChronoDuration::seconds(CONTROL_TTL_SECONDS));
    match write_browser_control(&state, &company, &current).await {
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
    let Some(current) = read_browser_control(&state, &company).await.ok().flatten() else {
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
    if let Err(error) = write_browser_control(&state, &company, &next).await {
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
    use axum::body::to_bytes;
    use tower::ServiceExt as _;

    #[tokio::test]
    #[ignore = "serves a dedicated *_test company until interrupted for owner-surface visual QA"]
    async fn live_isolated_owner_surface_server() {
        let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL")
            .expect("set RESTLESS_TEST_DATABASE_URL to an isolated test database");
        let company = std::env::var("RESTLESS_OWNER_SURFACE_TEST_COMPANY")
            .expect("set RESTLESS_OWNER_SURFACE_TEST_COMPANY");
        assert!(
            company.ends_with("_test"),
            "visual QA server may expose only a *_test company"
        );
        let root = runtime::state_root();
        let authority = crate::authority::AuthorityStore::connect(&database_url)
            .await
            .unwrap();
        let daemon = Arc::new(crate::Daemon {
            root: root.clone(),
            capabilities: crate::capability::CapabilityIssuer::open(&root).unwrap(),
            spend: crate::spend::SpendLedger::open(&root).unwrap(),
            authority,
            orgintel: crate::OrgIntelRegistry {
                database_url,
                root: root.clone(),
                handles: std::sync::Mutex::new(HashMap::new()),
            },
            staff: crate::staff::StaffRegistry::default(),
            activities: crate::activity::AgentActivityStreams::default(),
            in_flight: Arc::new(std::sync::Mutex::new(crate::schedule::WakeClaims::default())),
        });
        // Prove the requested company exists in both the configured Runtime
        // set and the isolated OrgIntel database before publishing a surface.
        runtime::CompanyConfig::load(&root, &company).unwrap();
        assert!(daemon.orgintel.get(&company).await.unwrap().is_live().await);
        let address = std::env::var("RESTLESS_OWNER_SURFACE_TEST_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7888".into())
            .parse()
            .unwrap();
        let review_address = std::env::var("RESTLESS_OWNER_SURFACE_TEST_REVIEW_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7894".into())
            .parse()
            .unwrap();
        println!("isolated owner surface for {company}: http://{address}/{company}");
        serve(
            daemon,
            OwnerConfig {
                address,
                review_address,
                review_public_url: format!("http://{{ticket}}.localhost:{}", review_address.port()),
                entry: EntryMode::Local,
                plane_readiness: None,
            },
        )
        .await
        .unwrap();
    }

    fn network_headers(host: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_str(host).unwrap());
        headers
    }

    #[test]
    fn plane_readiness_uses_an_exact_dedicated_bearer() {
        let expected = "plane-readiness-token-at-least-32-characters";
        let mut headers = HeaderMap::new();
        assert!(!readiness_token_matches(&headers, expected));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer another-readiness-token-at-least-32-characters"),
        );
        assert!(!readiness_token_matches(&headers, expected));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer plane-readiness-token-at-least-32-characters"),
        );
        assert!(readiness_token_matches(&headers, expected));
    }

    #[test]
    fn plane_readiness_release_identity_accepts_only_immutable_digests() {
        assert!(immutable_image_reference(&format!(
            "registry.example.test/core@sha256:{}",
            "a".repeat(64)
        )));
        assert!(!immutable_image_reference(
            "registry.example.test/core:latest"
        ));
        assert!(sha256_digest(&format!("sha256:{}", "b".repeat(64))));
        assert!(!sha256_digest(&format!("sha256:{}", "B".repeat(64))));
    }

    #[test]
    fn plane_readiness_observation_has_exactly_the_six_fleet_checks() {
        let observed_at = Utc::now();
        let observation = PlaneReadinessObservation {
            contract_version: 1,
            owner_id: Uuid::nil(),
            plane_id: Uuid::nil(),
            hostname: "owner.example.test".into(),
            account_plane_image: format!("registry/core@sha256:{}", "a".repeat(64)),
            desired_revision: 1,
            core_release: crate::release::CORE_VERSION,
            release_manifest_digest: format!("sha256:{}", "b".repeat(64)),
            status: "ready",
            ready: true,
            checks: [
                "authority",
                "credential_custody",
                "company_directory",
                "identity_handoff",
                "cockpit",
                "plane_database",
            ]
            .into_iter()
            .map(|kind| PlaneReadinessCheck {
                kind,
                status: "ready",
            })
            .collect(),
            observed_at,
            valid_until: observed_at + ChronoDuration::seconds(20),
        };
        let value = serde_json::to_value(observation).expect("readiness serializes");
        assert_eq!(value["checks"].as_array().unwrap().len(), 6);
        assert_eq!(value["ready"], true);
        assert_eq!(value["contract_version"], 1);
    }

    fn identity(scope: crate::entry::CompanyScope) -> crate::entry::VerifiedIdentity {
        crate::entry::VerifiedIdentity {
            user: "user-1".into(),
            owner: "owner-1".into(),
            scope,
            role: "member".into(),
            actor: None,
            correlation: None,
        }
    }

    const PLANE_HOST: &str = "aris.restless.test";

    #[test]
    fn network_entry_refuses_company_reads_without_a_session() {
        let refusal = network_boundary_violation(
            &Method::GET,
            &network_headers(PLANE_HOST),
            "/api/companies/aris/cockpit",
            PLANE_HOST,
            None,
        )
        .expect("no session is refused");
        assert_eq!(refusal.status, StatusCode::UNAUTHORIZED);
        assert_eq!(refusal.code, "no_session");
    }

    #[test]
    fn network_entry_admits_the_door_without_a_session() {
        assert!(network_boundary_violation(
            &Method::POST,
            &{
                let mut headers = network_headers(PLANE_HOST);
                headers.insert(
                    ORIGIN,
                    HeaderValue::from_str(&format!("https://{PLANE_HOST}")).unwrap(),
                );
                headers
            },
            "/entry",
            PLANE_HOST,
            None,
        )
        .is_none());
    }

    /// S27-T2. The plane genuinely serves both companies, so a pass here proves
    /// scoping rather than the absence of the other company.
    #[test]
    fn a_company_scoped_session_reaches_only_its_own_company() {
        let scoped = identity(crate::entry::CompanyScope::Company {
            company: "aris".into(),
        });

        assert!(
            network_boundary_violation(
                &Method::GET,
                &network_headers(PLANE_HOST),
                "/api/companies/aris/cockpit",
                PLANE_HOST,
                Some(&scoped),
            )
            .is_none(),
            "its own company must remain reachable, or the refusal below proves nothing"
        );

        let refusal = network_boundary_violation(
            &Method::GET,
            &network_headers(PLANE_HOST),
            "/api/companies/other/cockpit",
            PLANE_HOST,
            Some(&scoped),
        )
        .expect("another company on the same plane is refused");
        assert_eq!(refusal.status, StatusCode::FORBIDDEN);
        assert_eq!(refusal.code, "company_out_of_scope");

        // The desktop stream is the same boundary, not a second one.
        let refusal = network_boundary_violation(
            &Method::GET,
            &network_headers(PLANE_HOST),
            "/desktop/other/observe",
            PLANE_HOST,
            Some(&scoped),
        )
        .expect("the desktop path is scoped too");
        assert_eq!(refusal.code, "company_out_of_scope");
    }

    #[test]
    fn an_owner_scoped_session_reaches_every_company_on_its_plane() {
        let owner = identity(crate::entry::CompanyScope::Owner);
        for path in ["/api/companies/aris/cockpit", "/desktop/other/observe"] {
            assert!(network_boundary_violation(
                &Method::GET,
                &network_headers(PLANE_HOST),
                path,
                PLANE_HOST,
                Some(&owner),
            )
            .is_none());
        }
    }

    #[test]
    fn network_entry_refuses_a_host_that_is_not_this_plane() {
        let refusal = network_boundary_violation(
            &Method::GET,
            &network_headers("someone-else.restless.test"),
            "/api/companies/aris/cockpit",
            PLANE_HOST,
            Some(&identity(crate::entry::CompanyScope::Owner)),
        )
        .expect("wrong host refused");
        assert_eq!(refusal.code, "network_owner_boundary");
    }

    /// Scope is re-derived per request, so a session cannot be widened by
    /// arriving at a different hostname or carrying a forwarding claim.
    #[test]
    fn scope_ignores_forwarding_claims_and_the_route_it_arrived_on() {
        let scoped = identity(crate::entry::CompanyScope::Company {
            company: "aris".into(),
        });
        let mut headers = network_headers(PLANE_HOST);
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("other.restless.test"),
        );
        headers.insert("x-real-ip", HeaderValue::from_static("10.0.0.1"));

        let refusal = network_boundary_violation(
            &Method::GET,
            &headers,
            "/api/companies/other/cockpit",
            PLANE_HOST,
            Some(&scoped),
        )
        .expect("a forwarding header does not widen scope");
        assert_eq!(refusal.code, "company_out_of_scope");
    }

    #[test]
    fn cockpit_typescript_bindings_match() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../web/src/lib/model/generated/cockpit.ts");
        let rendered = render_cockpit_bindings();

        if std::env::var_os("RESTLESS_WRITE_COCKPIT_BINDINGS").is_some() {
            if let Some(directory) = path.parent() {
                std::fs::create_dir_all(directory).expect("create cockpit bindings directory");
            }
            std::fs::write(&path, rendered).expect("write cockpit bindings");
            return;
        }

        let committed = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{}: {error}\nRegenerate with: RESTLESS_WRITE_COCKPIT_BINDINGS=1 cargo test -p restlessd cockpit_typescript_bindings_match",
                path.display()
            )
        });
        assert_eq!(
            committed, rendered,
            "cockpit TypeScript bindings drifted; regenerate with RESTLESS_WRITE_COCKPIT_BINDINGS=1 cargo test -p restlessd cockpit_typescript_bindings_match"
        );
    }

    #[test]
    fn conversation_typescript_bindings_match() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../web/src/lib/model/generated/conversation.ts");
        let rendered = render_conversation_bindings();

        if std::env::var_os("RESTLESS_WRITE_CONVERSATION_BINDINGS").is_some() {
            if let Some(directory) = path.parent() {
                std::fs::create_dir_all(directory).expect("create conversation bindings directory");
            }
            std::fs::write(&path, rendered).expect("write conversation bindings");
            return;
        }

        let committed = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{}: {error}\nRegenerate with: RESTLESS_WRITE_CONVERSATION_BINDINGS=1 cargo test -p restlessd conversation_typescript_bindings_match",
                path.display()
            )
        });
        assert_eq!(
            committed, rendered,
            "conversation TypeScript bindings drifted; regenerate with RESTLESS_WRITE_CONVERSATION_BINDINGS=1 cargo test -p restlessd conversation_typescript_bindings_match"
        );
    }

    #[test]
    fn conversation_contract_preserves_transcript_and_attachment_wire_names() {
        let at = chrono::DateTime::parse_from_rfc3339("2026-08-28T12:00:00Z")
            .expect("fixture timestamp")
            .with_timezone(&Utc);
        let view = ConversationView {
            actor: ConversationActorView {
                id: "exec".into(),
                display: "Exec".into(),
                kind: "exec".into(),
                role: "Executive".into(),
            },
            focus: Some(ConversationFocusView {
                after_message_id: 41,
                started_at: Some(at),
            }),
            messages: vec![ConversationMessageView {
                id: 42,
                from_actor: "owner".into(),
                to_actor: Some("exec".into()),
                body: "Please verify the launch plan.".into(),
                attachments: vec![OwnerAttachment {
                    upload_id: Uuid::nil(),
                    name: "plan.md".into(),
                    media_type: "text/markdown".into(),
                    size_bytes: 42,
                    path: "/company/inbox/owner-attachments/plan/content".into(),
                }],
                details: None,
                intent: Some(OwnerIntentReceipt {
                    kind: OwnerIntentKind::Conversation,
                    summary: "Launch-plan check".into(),
                    outcome: Some("The launch plan is ready for review.".into()),
                    next_step: Some("Exec checks the prepared plan.".into()),
                    owner_need: None,
                }),
                context_path: Some("/demo_test/company".into()),
                created_at: at,
                read_at: None,
            }],
        };

        let value = serde_json::to_value(view).expect("encode conversation contract");
        assert_eq!(value["focus"]["after_message_id"], 41);
        assert_eq!(value["messages"][0]["from_actor"], "owner");
        assert_eq!(
            value["messages"][0]["attachments"][0]["uploadId"],
            Uuid::nil().to_string()
        );
        assert_eq!(value["messages"][0]["intent"]["kind"], "conversation");
        assert_eq!(
            value["messages"][0]["intent"]["outcome"],
            "The launch plan is ready for review."
        );
    }

    fn cockpit_contract_fixture(degraded: bool) -> CockpitView {
        let at = || {
            chrono::DateTime::parse_from_rfc3339("2026-08-24T00:00:00Z")
                .expect("fixture timestamp")
                .with_timezone(&Utc)
        };
        let source_health = BTreeMap::from([
            ("orgintel".into(), "available".into()),
            (
                "authority".into(),
                if degraded {
                    "unavailable: fixture authority outage".into()
                } else {
                    "available".into()
                },
            ),
            ("runtime".into(), "running".into()),
        ]);
        let legal = if degraded {
            CockpitLegal {
                status: "unavailable".into(),
                profile: None,
                detail: Some("fixture authority outage".into()),
            }
        } else {
            CockpitLegal {
                status: "available".into(),
                profile: Some(CockpitLegalProfile {
                    legal_name: "Fixture Robotics Pty Ltd".into(),
                    trading_name: Some("Fixture Robotics".into()),
                    entity_type: "company".into(),
                    jurisdiction: "AU".into(),
                    registration_identifier: CockpitRegistrationIdentifier {
                        kind: "ACN".into(),
                        value: "123456789".into(),
                    },
                    approved_business_address: "1 Test Street".into(),
                    invoice_email: Some("ops@example.test".into()),
                    owner_asserted_by: "owner".into(),
                    owner_asserted_at: at(),
                    registry_observation: None,
                }),
                detail: None,
            }
        };
        let provider = if degraded {
            CockpitProvider {
                status: "unavailable".into(),
                connection: None,
                detail: Some("fixture authority outage".into()),
            }
        } else {
            CockpitProvider {
                status: "available".into(),
                connection: Some(CockpitProviderConnection {
                    environment: "sandbox".into(),
                    account_ref: "acct_fixture".into(),
                    api_version: "2026-01-01".into(),
                    read_scopes: vec!["balances:read".into()],
                    submit_scopes: vec!["transfers:submit".into()],
                    approval_workflow_observed: true,
                    observed_at: Some(at()),
                    updated_at: at(),
                }),
                detail: None,
            }
        };
        let finance = if degraded {
            CockpitFinance {
                status: "unavailable".into(),
                envelopes: Vec::new(),
                payments: Vec::new(),
                last_balance_observation: None,
                detail: Some("fixture authority outage".into()),
            }
        } else {
            CockpitFinance {
                status: "available".into(),
                envelopes: vec![CockpitMoneyEnvelope {
                    source_account_ref: "acct_fixture".into(),
                    currency: "AUD".into(),
                    beneficiary_refs: vec!["beneficiary_fixture".into()],
                    per_payment_limit_minor: 50_000,
                    aggregate_limit_minor: 100_000,
                    frozen: false,
                    period_started_at: at(),
                    updated_by: "owner".into(),
                    updated_at: at(),
                }],
                payments: vec![CockpitPaymentIntent {
                    work_id: Uuid::from_u128(1),
                    owner_handoff_id: Uuid::from_u128(2),
                    source_account_ref: "acct_fixture".into(),
                    provider_beneficiary_ref: "beneficiary_fixture".into(),
                    amount_minor: 12_34,
                    currency: "AUD".into(),
                    purpose: "fixture payment".into(),
                    evidence_refs: vec!["work:fixture".into()],
                    idempotency_key: "fixture-payment-1".into(),
                    requesting_actor: "exec".into(),
                    state: "reserved".into(),
                    provider: "airwallex".into(),
                    provider_transfer_id: None,
                    raw_provider_status: None,
                    provider_approval_url: None,
                    settled_at: None,
                    created_at: at(),
                    updated_at: at(),
                }],
                last_balance_observation: Some(CockpitBalanceObservation {
                    observed_at: at(),
                    body: serde_json::json!({ "currency": "AUD", "available": "10.00" }),
                }),
                detail: None,
            }
        };
        CockpitView {
            company: CockpitCompany {
                id: "fixture_test".into(),
                name: "Fixture Test".into(),
                mission: "Verify the owner projection.".into(),
                model: "fixture/model".into(),
            },
            source_health,
            people: vec![CockpitPerson {
                actor_id: "exec".into(),
                kind: "exec".into(),
                role: "exec".into(),
                display: "The Exec".into(),
                model: Some("fixture/model".into()),
                team_id: None,
                spent_usd: 1.25,
                session_running: true,
                session_observed_at: Some(at()),
                model_cooldown: None,
            }],
            teams: vec![CockpitTeam {
                id: Uuid::from_u128(3),
                name: "Research".into(),
                brief: "Research the fixture.".into(),
                lead_actor_id: "exec".into(),
                created_by: "owner".into(),
                created_at: at(),
                member_count: 1,
                in_motion_count: 1,
                blocked_count: 0,
            }],
            goals: vec![CockpitGoal {
                id: Uuid::from_u128(4),
                title: "Fixture goal".into(),
                body: "Make the contract observable.".into(),
                created_by: "owner".into(),
                created_at: at(),
                closed_at: None,
            }],
            spend: CockpitSpend {
                accounted_usd: 1.25,
                ceiling_usd: 25.0,
                remaining_usd: Some(23.75),
                status: "available".into(),
            },
            authority: CockpitAuthority {
                approved_parties: vec!["fixture-provider".into()],
                credentials: vec![CockpitCredential {
                    binding: "fixture.api".into(),
                    status: "present".into(),
                    detail: None,
                }],
                legal,
                provider,
                finance,
            },
            receipts: vec![CockpitEffectReceipt {
                id: 7,
                effect_class: Some(serde_json::json!("provider_read")),
                tool: Some(serde_json::json!("fixture")),
                success: Some(serde_json::json!(true)),
                party: Some(serde_json::json!("fixture-provider")),
                actor: Some(serde_json::json!("exec")),
                outcome: Some(serde_json::json!({ "status": "observed" })),
                evidence_quality: CockpitEvidenceQuality::Governed,
                at: at(),
            }],
            refreshed_at: at(),
        }
    }

    #[tokio::test]
    async fn cockpit_router_keeps_populated_and_degraded_response_shapes() {
        let app = Router::new()
            .route(
                "/populated",
                get(|| async { Json(cockpit_contract_fixture(false)) }),
            )
            .route(
                "/degraded",
                get(|| async { Json(cockpit_contract_fixture(true)) }),
            );

        for (path, degraded) in [("/populated", false), ("/degraded", true)] {
            let response = app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .uri(path)
                        .body(Body::empty())
                        .expect("fixture request"),
                )
                .await
                .expect("fixture router response");
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok()),
                Some("application/json")
            );
            let body = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read fixture response body");
            let json: serde_json::Value =
                serde_json::from_slice(&body).expect("fixture response is JSON");
            assert_eq!(json["company"]["id"], "fixture_test");
            assert_eq!(json["people"][0]["actor_id"], "exec");
            assert_eq!(json["receipts"][0]["evidence_quality"], "governed");
            assert_eq!(
                json["authority"]["finance"]["status"],
                if degraded { "unavailable" } else { "available" }
            );
            if degraded {
                assert_eq!(
                    json["authority"]["legal"]["profile"],
                    serde_json::Value::Null
                );
                assert_eq!(
                    json["authority"]["provider"]["detail"],
                    "fixture authority outage"
                );
            } else {
                assert_eq!(
                    json["authority"]["legal"]["profile"]["legal_name"],
                    "Fixture Robotics Pty Ltd"
                );
                assert!(json["authority"]["provider"].get("detail").is_none());
                assert_eq!(
                    json["authority"]["finance"]["payments"][0]["state"],
                    "reserved"
                );
            }
        }
    }

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

        let at_a_glance = concat!(
            "The four drafts are ready.",
            "\n\n<!--restless-intent:{\"kind\":\"conversation\",",
            "\"summary\":\"Campaign preparation result.\",",
            "\"outcome\":\"Four reviewed drafts are ready.\",",
            "\"nextStep\":\"The lead waits for the campaign decision.\",",
            "\"ownerNeed\":\"Approve, change or decline the campaign.\"}-->"
        );
        let (_, receipt) = split_intent_receipt(at_a_glance);
        let receipt = receipt.expect("optional reader fields should parse");
        assert_eq!(
            receipt.outcome.as_deref(),
            Some("Four reviewed drafts are ready.")
        );
        assert_eq!(
            receipt.owner_need.as_deref(),
            Some("Approve, change or decline the campaign.")
        );

        let malformed = "Reply\n\n<!--restless-intent:{\"kind\":\"whatever\",\"summary\":\"x\"}-->";
        assert_eq!(split_intent_receipt(malformed).0, "Reply");
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
