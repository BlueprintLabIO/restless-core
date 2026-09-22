//! Authenticated company-scoped HTTP surface for Core-native Documents.
//!
//! This module is intentionally a thin transport over `restless_orgintel`.
//! The route never accepts an acting Actor or company id in a request body:
//! both come from the verified owner-entry principal and the exact route.

use super::*;
use crate::owner_cell_readiness::ReadinessSecret;
use serde_json::Value;

use crate::document_collaboration_token::{
    DocumentCollaborationAccess, DocumentCollaborationTokenInput, DEFAULT_TTL_SECONDS,
};
use axum::http::header::ACCEPT;

const DOCUMENT_BODY_LIMIT: usize = 1_250_000;
pub(super) const DOCUMENT_COLLABORATION_JWKS_PATH: &str =
    "/.well-known/restless-native-documents-jwks.json";
const NATIVE_DOCUMENTS_READINESS_ROUTE: &str =
    "/internal/v1/cells/{cell_id}/native-documents/readiness";
const NATIVE_DOCUMENTS_READINESS_PATH: &str = "/internal/v1/native-documents/ready";
const NATIVE_DOCUMENTS_SERVICE_PORT: u16 = 6688;
const NATIVE_DOCUMENTS_PROTOCOL_VERSION: u32 = 1;
const NATIVE_DOCUMENTS_SCHEMA_VERSION: u32 = 1;
const MAX_NATIVE_DOCUMENTS_HEALTH_BYTES: usize = 2 * 1024;
const MAX_NATIVE_DOCUMENTS_PROXY_MESSAGE_BYTES: usize = 8 * 1024 * 1024 + 64 * 1024;
const CELL_READINESS_TOKEN_FILE_ENV: &str = "RESTLESS_CELL_READINESS_TOKEN_FILE";
// A JSON string can encode one permitted Markdown byte as six bytes (`\u00XX`).
// Keep the larger allowance scoped to the import route and reserve bounded
// headroom for the UUID, reason, property names, and JSON punctuation.
const DOCUMENT_MARKDOWN_JSON_ESCAPE_BYTES_PER_BYTE: usize = 6;
const DOCUMENT_MARKDOWN_IMPORT_ENVELOPE_HEADROOM: usize = 8 * 1024;
const DOCUMENT_MARKDOWN_IMPORT_BODY_LIMIT: usize =
    restless_orgintel::MAX_NATIVE_DOCUMENT_MARKDOWN_CHECKPOINT_BYTES
        * DOCUMENT_MARKDOWN_JSON_ESCAPE_BYTES_PER_BYTE
        + DOCUMENT_MARKDOWN_IMPORT_ENVELOPE_HEADROOM;
const DEFAULT_PAGE_LIMIT: i64 = 50;
const MAX_DOCUMENT_PAGE_LIMIT: i64 = 100;
const MAX_DOCUMENT_CHILD_PAGE_LIMIT: i64 = 50;
const MAX_DOCUMENT_VERSION_PAGE_LIMIT: i64 = 25;
const MAX_DOCUMENT_PARTICIPANT_PAGE_LIMIT: i64 = 50;
const MAX_DOCUMENT_SEARCH_PAGE_LIMIT: i64 = 50;
const MAX_DOCUMENT_LINK_PAGE_LIMIT: i64 = 50;
const MAX_DOCUMENT_SEARCH_OFFSET: i64 = 10_000;
const MAX_DOCUMENT_SEARCH_QUERY_BYTES: usize = 256;
const IDEMPOTENCY_KEY: &str = "idempotency-key";

pub(super) fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    RoomApiState: FromRef<S>,
{
    Router::<S>::new()
        .route(
            "/companies/{company}/documents/{document}/collaboration-requests/{request}/resolve",
            post(resolve_collaboration_request),
        )
        .route(
            "/companies/{company}/documents",
            get(list_documents).post(create_document),
        )
        .route(
            "/companies/{company}/documents/search",
            get(search_documents),
        )
        .route(
            "/companies/{company}/documents/{document}",
            get(get_document).patch(update_document),
        )
        .route(
            "/companies/{company}/documents/{document}/collaboration/token",
            post(issue_document_collaboration_token),
        )
        .route(
            "/companies/{company}/documents/{document}/links",
            get(get_document_links),
        )
        .route(
            "/companies/{company}/documents/{document}/imports/markdown",
            post(import_document_markdown)
                .layer(DefaultBodyLimit::max(DOCUMENT_MARKDOWN_IMPORT_BODY_LIMIT)),
        )
        .route(
            "/companies/{company}/documents/{document}/versions",
            get(list_document_versions).post(create_document_version),
        )
        .route(
            "/companies/{company}/documents/{document}/versions/{version}",
            get(get_document_version),
        )
        .route(
            "/companies/{company}/documents/{document}/versions/{version}/export/markdown",
            get(export_document_markdown),
        )
        .route(
            "/companies/{company}/documents/{document}/versions/{version}/restore",
            post(restore_document_version),
        )
        .route(
            "/companies/{company}/documents/{document}/participants",
            get(list_document_participants),
        )
        .route(
            "/companies/{company}/documents/{document}/participants/{participant}",
            axum::routing::put(set_document_participant).delete(remove_document_participant),
        )
        .route(
            "/companies/{company}/documents/{document}/comments",
            get(list_document_comments).post(create_document_comment),
        )
        .route(
            "/companies/{company}/documents/{document}/comments/{thread}/replies",
            get(list_document_comment_replies).post(reply_to_document_comment),
        )
        .route(
            "/companies/{company}/documents/{document}/comments/{thread}/resolve",
            post(resolve_document_comment),
        )
        .route(
            "/companies/{company}/documents/{document}/reviews",
            get(list_document_reviews).post(request_document_review),
        )
        .route(
            "/companies/{company}/documents/{document}/reviews/{review}",
            get(get_document_review),
        )
        .route(
            "/companies/{company}/documents/{document}/reviews/{review}/accept",
            post(accept_document_review),
        )
        .route(
            "/companies/{company}/documents/{document}/reviews/{review}/request-changes",
            post(request_document_review_changes),
        )
        .route(
            "/companies/{company}/documents/{document}/proposals",
            get(list_document_proposals),
        )
        .route(
            "/companies/{company}/documents/{document}/proposals/{proposal}",
            get(get_document_proposal),
        )
        .route(
            "/companies/{company}/documents/{document}/proposals/{proposal}/accept",
            post(accept_document_proposal),
        )
        .route(
            "/companies/{company}/documents/{document}/proposals/{proposal}/reject",
            post(reject_document_proposal),
        )
        .layer(DefaultBodyLimit::max(DOCUMENT_BODY_LIMIT))
        .layer(middleware::map_response(no_store_response))
}

pub(super) fn public_routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    RoomApiState: FromRef<S>,
{
    Router::<S>::new()
        .route(
            DOCUMENT_COLLABORATION_JWKS_PATH,
            get(document_collaboration_jwks),
        )
        .route(
            NATIVE_DOCUMENTS_READINESS_ROUTE,
            get(native_documents_readiness),
        )
        .route(
            "/api/companies/{company_id}/documents/{document_id}/collaboration",
            get(native_documents_collaboration),
        )
}

#[derive(Clone)]
pub(super) struct NativeDocumentsProxy {
    client: reqwest::Client,
    readiness_secret: Option<ReadinessSecret>,
    service: NativeDocumentsService,
}

#[derive(Clone)]
enum NativeDocumentsService {
    PerCellDns,
    Local {
        root: std::path::PathBuf,
    },
    #[cfg(test)]
    Fixed {
        host: Arc<str>,
        port: u16,
    },
}

impl NativeDocumentsProxy {
    pub(super) fn from_environment() -> Result<Self> {
        if std::env::var_os("RESTLESS_CELL_READINESS_TOKEN").is_some() {
            anyhow::bail!("native Documents readiness requires {CELL_READINESS_TOKEN_FILE_ENV}, not an environment bearer");
        }
        let readiness_secret = match std::env::var_os(CELL_READINESS_TOKEN_FILE_ENV) {
            Some(raw) => {
                let path = std::path::PathBuf::from(raw);
                if !path.is_absolute() {
                    anyhow::bail!(
                        "{CELL_READINESS_TOKEN_FILE_ENV} must be an absolute secret-file path"
                    );
                }
                Some(ReadinessSecret::read(&path)?)
            }
            None => None,
        };
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(1_500))
            .timeout(Duration::from_secs(2))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("build native Documents readiness client")?;
        Ok(Self {
            client,
            readiness_secret,
            service: NativeDocumentsService::PerCellDns,
        })
    }

    #[cfg(test)]
    pub(super) fn disabled_for_test() -> Self {
        Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("test native Documents client"),
            readiness_secret: None,
            service: NativeDocumentsService::PerCellDns,
        }
    }

    pub(super) fn use_local_services(&mut self, root: std::path::PathBuf) {
        self.service = NativeDocumentsService::Local { root };
    }

    pub(super) async fn agent_body(
        &self,
        root: &std::path::Path,
        org: &restless_orgintel::OrgIntel,
        actor: &str,
        document: Uuid,
        issuer: &str,
        payload: &Value,
    ) -> Result<Value> {
        let access = match org.document_access_for_actor(document, actor).await? {
            Some(restless_orgintel::DocumentAccess::Edit) => DocumentCollaborationAccess::Write,
            Some(_) if payload["action"] == "read" => DocumentCollaborationAccess::Read,
            _ => anyhow::bail!("native document is unavailable"),
        };
        let identity = org
            .company_access_identity()
            .await?
            .context("Documents collaboration is not provisioned")?;
        let signer =
            crate::document_collaboration_token::DocumentCollaborationTokenIssuer::open(root)?;
        let token = signer.issue(DocumentCollaborationTokenInput {
            issuer,
            session_principal: &format!("agent:{actor}"),
            company_id: identity.company_id,
            document_id: document,
            actor_id: actor,
            access,
            now: Utc::now(),
            ttl_seconds: DEFAULT_TTL_SECONDS,
        })?;
        let (host, port) = self.service_address(identity.cell_id)?;
        let url = format!(
            "http://{host}:{port}/api/companies/{}/documents/{document}/collaboration/body",
            identity.company_id
        );
        let mut response = self
            .client
            .post(&url)
            .timeout(Duration::from_secs(30))
            .bearer_auth(token)
            .json(payload)
            .send()
            .await
            .context("reach live document body")?;
        if response.status() != reqwest::StatusCode::OK {
            anyhow::bail!(
                "live document command failed (HTTP {}); retain the same edit key on retry",
                response.status().as_u16()
            );
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            anyhow::ensure!(
                bytes.len() + chunk.len() <= 12 * 1024 * 1024,
                "live document response exceeds limit"
            );
            bytes.extend_from_slice(&chunk);
        }
        Ok(serde_json::from_slice(&bytes).context("decode live document response")?)
    }

    fn service_address(&self, cell_id: Uuid) -> Result<(String, u16)> {
        Ok(match &self.service {
            NativeDocumentsService::Local { root } => {
                return crate::local_documents::address(root, cell_id)
            }
            NativeDocumentsService::PerCellDns => (
                format!("restless-docs-{cell_id}"),
                NATIVE_DOCUMENTS_SERVICE_PORT,
            ),
            #[cfg(test)]
            NativeDocumentsService::Fixed { host, port } => (host.to_string(), *port),
        })
    }

    fn readiness_authorized(&self, headers: &HeaderMap) -> bool {
        self.readiness_secret
            .as_ref()
            .is_some_and(|secret| secret.authorizes(headers))
    }

    pub(super) async fn observe_readiness(&self, cell_id: Uuid) -> Result<NativeDocumentsHealth> {
        let (host, port) = self.service_address(cell_id)?;
        let url = format!("http://{host}:{port}{NATIVE_DOCUMENTS_READINESS_PATH}");
        let mut response = self
            .client
            .get(&url)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .context("reach native Documents sidecar readiness")?;
        if response.status() != reqwest::StatusCode::OK || response.url().as_str() != url {
            anyhow::bail!("native Documents sidecar is not ready");
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .split(';')
            .next()
            .unwrap_or_default()
            .trim();
        if content_type != "application/json" {
            anyhow::bail!("native Documents readiness response is not JSON");
        }
        if response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok())
            .is_some_and(|length| length == 0 || length > MAX_NATIVE_DOCUMENTS_HEALTH_BYTES)
        {
            anyhow::bail!("native Documents readiness response is outside its bound");
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .context("read native Documents readiness response")?
        {
            if body.len() + chunk.len() > MAX_NATIVE_DOCUMENTS_HEALTH_BYTES {
                anyhow::bail!("native Documents readiness response is outside its bound");
            }
            body.extend_from_slice(&chunk);
        }
        let observation: NativeDocumentsHealth =
            serde_json::from_slice(&body).context("decode native Documents readiness response")?;
        if observation
            != (NativeDocumentsHealth {
                status: "ready".into(),
                protocol_version: NATIVE_DOCUMENTS_PROTOCOL_VERSION,
                schema_version: NATIVE_DOCUMENTS_SCHEMA_VERSION,
            })
        {
            anyhow::bail!(
                "native Documents readiness response does not match the released contract"
            );
        }
        Ok(observation)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeDocumentsHealth {
    status: String,
    protocol_version: u32,
    schema_version: u32,
}

pub(super) fn is_collaboration_proxy_path(path: &str) -> bool {
    let Some(rest) = path.strip_prefix("/api/companies/") else {
        return false;
    };
    let mut segments = rest.split('/');
    canonical_document_uuid(segments.next())
        && segments.next() == Some("documents")
        && canonical_document_uuid(segments.next())
        && segments.next() == Some("collaboration")
        && segments.next().is_none()
}

fn canonical_document_uuid(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        Uuid::parse_str(value)
            .ok()
            .is_some_and(|parsed| parsed.to_string() == value)
    })
}

async fn native_documents_readiness(
    State(state): State<RoomApiState>,
    OriginalUri(uri): OriginalUri,
    AxumPath(cell_id): AxumPath<Uuid>,
    headers: HeaderMap,
) -> Response<Body> {
    if uri.query().is_some()
        || headers.contains_key(COOKIE)
        || headers.contains_key(ORIGIN)
        || !state.native_documents_proxy.readiness_authorized(&headers)
    {
        return native_documents_proxy_error(
            StatusCode::UNAUTHORIZED,
            "native_documents_readiness_unauthorized",
        );
    }
    if resolve_native_documents_company(&state, None, Some(cell_id))
        .await
        .is_err()
    {
        return native_documents_proxy_error(
            StatusCode::NOT_FOUND,
            "native_documents_cell_not_found",
        );
    }
    match state
        .native_documents_proxy
        .observe_readiness(cell_id)
        .await
    {
        Ok(observation) => native_documents_proxy_json(StatusCode::OK, observation),
        Err(error) => {
            tracing::warn!(%cell_id, %error, "native Documents sidecar is not ready");
            native_documents_proxy_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "native_documents_not_ready",
            )
        }
    }
}

async fn resolve_collaboration_request(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, request)): AxumPath<(String, Uuid, Uuid)>,
) -> Response<Body> {
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .resolve_document_collaboration(document, principal.actor_id(), request)
        .await
    {
        Ok(value) => document_json(StatusCode::OK, value),
        Err(error) => document_error(error),
    }
}

async fn native_documents_collaboration(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    session_lease: Option<Extension<SessionLease>>,
    OriginalUri(uri): OriginalUri,
    AxumPath((company_id, document_id)): AxumPath<(Uuid, Uuid)>,
    upgrade: WebSocketUpgrade,
) -> Response<Body> {
    if uri.query().is_some() || !is_collaboration_proxy_path(uri.path()) {
        return native_documents_proxy_error(
            StatusCode::BAD_REQUEST,
            "native_documents_collaboration_route",
        );
    }
    if state.network_mode
        && session_lease
            .as_ref()
            .is_none_or(|Extension(lease)| lease.is_ended())
    {
        return native_documents_proxy_error(StatusCode::UNAUTHORIZED, "no_session");
    }
    let (company, org, identity) = match resolve_native_documents_company(
        &state,
        Some(company_id),
        None,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(%company_id, %error, "native Documents company route did not resolve");
            return native_documents_proxy_error(
                StatusCode::NOT_FOUND,
                "native_documents_company_not_found",
            );
        }
    };
    if !principal.permits_company(&company) {
        return native_documents_proxy_error(StatusCode::FORBIDDEN, "company_out_of_scope");
    }
    match org
        .document_access_for_actor(document_id, principal.actor_id())
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            return native_documents_proxy_error(
                StatusCode::NOT_FOUND,
                "native_documents_document_not_found",
            )
        }
        Err(error) => return document_error(error),
    }
    let proxy = state.native_documents_proxy.clone();
    let session_lease = session_lease.map(|Extension(lease)| lease);
    upgrade
        .max_message_size(MAX_NATIVE_DOCUMENTS_PROXY_MESSAGE_BYTES)
        .max_frame_size(MAX_NATIVE_DOCUMENTS_PROXY_MESSAGE_BYTES)
        .on_upgrade(move |socket| async move {
            if let Err(error) = proxy_native_documents_websocket(
                socket,
                company_id,
                identity.cell_id,
                document_id,
                proxy,
                session_lease,
            )
            .await
            {
                tracing::warn!(%company_id, %document_id, %error, "native Documents websocket ended");
            }
        })
        .into_response()
}

async fn resolve_native_documents_company(
    state: &RoomApiState,
    company_id: Option<Uuid>,
    cell_id: Option<Uuid>,
) -> Result<(
    String,
    restless_orgintel::OrgIntel,
    restless_orgintel::CompanyAccessIdentity,
)> {
    let companies = match &state.source {
        RoomOrgIntelSource::Daemon(daemon) => crate::configured_companies(&daemon.root)?,
        #[cfg(test)]
        RoomOrgIntelSource::Fixed { companies, .. } => companies.keys().cloned().collect(),
    };
    let mut matches = Vec::new();
    for company in companies {
        let org = state.orgintel(&company).await?;
        let Some(identity) = org.company_access_identity().await? else {
            continue;
        };
        if company_id.is_none_or(|expected| identity.company_id == expected)
            && cell_id.is_none_or(|expected| identity.cell_id == expected)
        {
            matches.push((company, org, identity));
        }
    }
    match matches.len() {
        1 => Ok(matches.pop().expect("length checked")),
        0 => anyhow::bail!("no immutable native Documents company binding matched"),
        _ => anyhow::bail!("native Documents company binding is ambiguous"),
    }
}

async fn proxy_native_documents_websocket(
    browser: WebSocket,
    company_id: Uuid,
    cell_id: Uuid,
    document_id: Uuid,
    proxy: NativeDocumentsProxy,
    session_lease: Option<SessionLease>,
) -> Result<()> {
    let (host, port) = proxy.service_address(cell_id)?;
    let stream = match session_lease.as_ref() {
        Some(lease) => tokio::select! {
            result = tokio::time::timeout(
                Duration::from_secs(2),
                tokio::net::TcpStream::connect((host.as_str(), port)),
            ) => result.context("native Documents sidecar connect timed out")??,
            _ = lease.ended() => return Ok(()),
        },
        None => tokio::time::timeout(
            Duration::from_secs(2),
            tokio::net::TcpStream::connect((host.as_str(), port)),
        )
        .await
        .context("native Documents sidecar connect timed out")??,
    };
    let target = format!(
        "ws://{host}:{port}/api/companies/{company_id}/documents/{document_id}/collaboration"
    );
    let (sidecar, _) = match session_lease.as_ref() {
        Some(lease) => tokio::select! {
            result = client_async(target, stream) => result?,
            _ = lease.ended() => return Ok(()),
        },
        None => client_async(target, stream).await?,
    };
    let (mut browser_tx, mut browser_rx) = browser.split();
    let (mut sidecar_tx, mut sidecar_rx) = sidecar.split();
    loop {
        tokio::select! {
            _ = optional_session_ended(session_lease.as_ref()) => break,
            incoming = browser_rx.next() => match incoming {
                Some(Ok(message)) => {
                    let translated = match message {
                        AxumMessage::Text(value) => tungstenite::Message::Text(value.to_string().into()),
                        AxumMessage::Binary(value) => tungstenite::Message::Binary(value),
                        AxumMessage::Ping(value) => tungstenite::Message::Ping(value),
                        AxumMessage::Pong(value) => tungstenite::Message::Pong(value),
                        AxumMessage::Close(_) => break,
                    };
                    sidecar_tx.send(translated).await?;
                }
                _ => break,
            },
            incoming = sidecar_rx.next() => match incoming {
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

fn native_documents_proxy_json(status: StatusCode, value: impl Serialize) -> Response<Body> {
    let mut response = (status, Json(value)).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn native_documents_proxy_error(status: StatusCode, code: &'static str) -> Response<Body> {
    native_documents_proxy_json(status, serde_json::json!({ "error": code }))
}

struct DocumentPrincipal(RequestPrincipal);

impl<S> FromRequestParts<S> for DocumentPrincipal
where
    S: Send + Sync,
{
    type Rejection = Response<Body>;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<RequestPrincipal>()
            .cloned()
            .map(Self)
            .ok_or_else(|| {
                document_api_error(
                    StatusCode::UNAUTHORIZED,
                    "no_session",
                    "Document access requires a verified company session",
                )
            })
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct DocumentListQuery {
    #[serde(default)]
    include_archived: bool,
    before_updated_at: Option<String>,
    before_document_id: Option<Uuid>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct VersionListQuery {
    before_version_number: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct CreatedPageQuery {
    after_created_at: Option<String>,
    after_id: Option<Uuid>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ParticipantListQuery {
    after_added_at: Option<String>,
    after_actor_id: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentSearchQuery {
    q: String,
    #[serde(default)]
    include_archived: bool,
    #[serde(default)]
    offset: i64,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct DocumentLinkQuery {
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateDocumentInput {
    title: String,
    kind: restless_orgintel::DocumentKind,
    visibility: restless_orgintel::DocumentVisibility,
    #[serde(default)]
    linked_room_id: Option<Uuid>,
    #[serde(default)]
    inherit_room_visibility: bool,
    content_json: serde_json::Value,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateDocumentInput {
    expected_version: i64,
    title: String,
    kind: restless_orgintel::DocumentKind,
    visibility: restless_orgintel::DocumentVisibility,
    #[serde(default)]
    linked_room_id: Option<Uuid>,
    #[serde(default)]
    inherit_room_visibility: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SetDocumentParticipantInput {
    expected_document_version: i64,
    access: restless_orgintel::DocumentAccess,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoveDocumentParticipantInput {
    expected_document_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateDocumentVersionInput {
    expected_current_version_id: Uuid,
    content_json: serde_json::Value,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreDocumentVersionInput {
    expected_current_version_id: Uuid,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportDocumentMarkdownInput {
    expected_current_version_id: Uuid,
    markdown: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateDocumentCommentInput {
    #[serde(default)]
    block_id: Option<String>,
    content_json: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplyToDocumentCommentInput {
    #[serde(default)]
    reply_to_comment_id: Option<Uuid>,
    content_json: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolveDocumentCommentInput {
    expected_thread_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestDocumentReviewInput {
    expected_document_version: i64,
    expected_current_version_id: Uuid,
    summary: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptDocumentReviewInput {
    expected_document_version: i64,
    expected_review_version: i64,
    accepted_version_name: String,
    #[serde(default)]
    feedback: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestDocumentReviewChangesInput {
    expected_document_version: i64,
    expected_review_version: i64,
    feedback: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolveDocumentProposalInput {
    expected_proposal_version: i64,
    resolution_summary: String,
    #[serde(default)]
    accepted_version_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct DocumentCollaborationTokenResponse {
    token: String,
    token_type: &'static str,
    access: DocumentCollaborationAccess,
    expires_at: DateTime<Utc>,
}

async fn document_collaboration_jwks(State(state): State<RoomApiState>) -> Response<Body> {
    let mut response = Json(state.document_collaboration_tokens.jwks()).into_response();
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    response
}

async fn issue_document_collaboration_token(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let company_identity = match org.company_access_identity().await {
        Ok(Some(identity)) if !identity.company_id.is_nil() && !identity.cell_id.is_nil() => {
            identity
        }
        Ok(_) => {
            return document_api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "document_collaboration_unavailable",
                "company Documents collaboration is not provisioned",
            )
        }
        Err(error) => {
            tracing::error!(%error, company, "could not resolve native Documents company identity");
            return document_api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "company Documents are temporarily unavailable",
            );
        }
    };
    let access = match org
        .document_access_for_actor(document, principal.actor_id())
        .await
    {
        Ok(Some(restless_orgintel::DocumentAccess::Edit)) => DocumentCollaborationAccess::Write,
        Ok(Some(
            restless_orgintel::DocumentAccess::Read | restless_orgintel::DocumentAccess::Comment,
        )) => DocumentCollaborationAccess::Read,
        Ok(None) => return document_error(restless_orgintel::DocumentError::Unavailable),
        Err(error) => return document_error(error),
    };
    let now = Utc::now();
    let token = match state
        .document_collaboration_tokens
        .issue(DocumentCollaborationTokenInput {
            issuer: &state.document_collaboration_issuer,
            session_principal: principal.cache_partition(),
            company_id: company_identity.company_id,
            document_id: document,
            actor_id: principal.actor_id(),
            access,
            now,
            ttl_seconds: DEFAULT_TTL_SECONDS,
        }) {
        Ok(token) => token,
        Err(error) => {
            tracing::error!(%error, company, %document, "could not issue native Documents collaboration token");
            return document_api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "document_collaboration_unavailable",
                "company Documents collaboration is temporarily unavailable",
            );
        }
    };
    document_json(
        StatusCode::OK,
        DocumentCollaborationTokenResponse {
            token,
            token_type: "Bearer",
            access,
            expires_at: now + ChronoDuration::seconds(DEFAULT_TTL_SECONDS),
        },
    )
}

async fn document_orgintel(
    state: &RoomApiState,
    principal: &RequestPrincipal,
    company: &str,
) -> std::result::Result<restless_orgintel::OrgIntel, Response<Body>> {
    if !principal.permits_company(company) {
        return Err(document_api_error(
            StatusCode::FORBIDDEN,
            "company_out_of_scope",
            "this session is not scoped to that company",
        ));
    }
    let org = state.orgintel(company).await.map_err(|error| {
        tracing::error!(%error, company, "could not open company Document store");
        document_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            "company Documents are temporarily unavailable",
        )
    })?;
    match org.active_actor(principal.actor_id()).await {
        Ok(Some(actor)) if actor.actor_class == "human" => Ok(org),
        Ok(_) => Err(document_api_error(
            StatusCode::FORBIDDEN,
            "request_principal",
            "the verified request principal is not an active human Actor",
        )),
        Err(error) => {
            tracing::error!(%error, company, actor = principal.actor_id(), "could not resolve Document principal");
            Err(document_api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "company Documents are temporarily unavailable",
            ))
        }
    }
}

fn document_api_error(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> Response<Body> {
    no_store(api_error(status, code, message))
}

#[derive(Debug)]
struct DocumentApiFailure {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl DocumentApiFailure {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
}

impl IntoResponse for DocumentApiFailure {
    fn into_response(self) -> Response<Body> {
        document_api_error(self.status, self.code, self.message)
    }
}

fn document_error(error: restless_orgintel::DocumentError) -> Response<Body> {
    match error {
        restless_orgintel::DocumentError::Invalid(message) => {
            document_api_error(StatusCode::UNPROCESSABLE_ENTITY, "document", message)
        }
        restless_orgintel::DocumentError::Unavailable => document_api_error(
            StatusCode::NOT_FOUND,
            "document",
            "the Document is unavailable",
        ),
        restless_orgintel::DocumentError::Conflict(message) => {
            document_api_error(StatusCode::CONFLICT, "document_conflict", message)
        }
        restless_orgintel::DocumentError::Corrupt(message) => {
            tracing::error!(message, "native Document state is corrupt");
            document_api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "document_unavailable",
                "company Documents are temporarily unavailable",
            )
        }
        restless_orgintel::DocumentError::Database(error) => {
            tracing::error!(%error, "native Document operation failed");
            document_api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "company Documents are temporarily unavailable",
            )
        }
    }
}

fn no_store(mut response: Response<Body>) -> Response<Body> {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn no_store_response(response: Response<Body>) -> Response<Body> {
    no_store(response)
}

fn document_json<T: Serialize>(status: StatusCode, value: T) -> Response<Body> {
    no_store((status, Json(value)).into_response())
}

fn command_id(headers: &HeaderMap) -> std::result::Result<Uuid, DocumentApiFailure> {
    let mut values = headers.get_all(IDEMPOTENCY_KEY).iter();
    let Some(value) = values.next() else {
        return Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Document mutations require one UUID Idempotency-Key header",
        ));
    };
    if values.next().is_some() {
        return Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Document mutations require exactly one Idempotency-Key header",
        ));
    }
    let value = value.to_str().ok().map(str::trim).unwrap_or_default();
    let command_id = Uuid::parse_str(value).map_err(|_| {
        DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Idempotency-Key must be a non-nil UUID",
        )
    })?;
    if command_id.is_nil() {
        return Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Idempotency-Key must be a non-nil UUID",
        ));
    }
    Ok(command_id)
}

fn validate_participant_actor_id(value: &str) -> std::result::Result<(), DocumentApiFailure> {
    if value.trim().is_empty() || value.len() > 255 || value.chars().any(char::is_control) {
        return Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "participant_actor_id",
            "participant Actor id must contain 1 to 255 non-control bytes",
        ));
    }
    Ok(())
}

fn page_limit(
    requested: Option<i64>,
    maximum: i64,
) -> std::result::Result<i64, DocumentApiFailure> {
    let limit = requested.unwrap_or(DEFAULT_PAGE_LIMIT.min(maximum));
    if !(1..=maximum).contains(&limit) {
        return Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "page_limit",
            format!("page limit must be between 1 and {maximum}"),
        ));
    }
    Ok(limit)
}

fn participant_cursor(
    query: &ParticipantListQuery,
) -> std::result::Result<Option<restless_orgintel::DocumentParticipantCursor>, DocumentApiFailure> {
    match (&query.after_added_at, &query.after_actor_id) {
        (None, None) => Ok(None),
        (Some(added_at), Some(actor_id)) => {
            validate_participant_actor_id(actor_id)?;
            DateTime::parse_from_rfc3339(added_at)
                .map(|added_at| {
                    Some(restless_orgintel::DocumentParticipantCursor {
                        added_at: added_at.with_timezone(&Utc),
                        actor_id: actor_id.clone(),
                    })
                })
                .map_err(|_| {
                    DocumentApiFailure::new(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "document_cursor",
                        "after_added_at must be RFC 3339 and paired with after_actor_id",
                    )
                })
        }
        _ => Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_cursor",
            "after_added_at and after_actor_id must be supplied together",
        )),
    }
}

fn document_list_cursor(
    query: &DocumentListQuery,
) -> std::result::Result<Option<restless_orgintel::DocumentListCursor>, DocumentApiFailure> {
    match (&query.before_updated_at, query.before_document_id) {
        (None, None) => Ok(None),
        (Some(updated_at), Some(id)) => DateTime::parse_from_rfc3339(updated_at)
            .map(|updated_at| {
                Some(restless_orgintel::DocumentListCursor {
                    updated_at: updated_at.with_timezone(&Utc),
                    id,
                })
            })
            .map_err(|_| {
                DocumentApiFailure::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "document_cursor",
                    "before_updated_at must be RFC 3339 and paired with before_document_id",
                )
            }),
        _ => Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_cursor",
            "before_updated_at and before_document_id must be supplied together",
        )),
    }
}

fn created_cursor(
    query: &CreatedPageQuery,
) -> std::result::Result<Option<restless_orgintel::DocumentPageCursor>, DocumentApiFailure> {
    match (&query.after_created_at, query.after_id) {
        (None, None) => Ok(None),
        (Some(created_at), Some(id)) => DateTime::parse_from_rfc3339(created_at)
            .map(|created_at| {
                Some(restless_orgintel::DocumentPageCursor {
                    created_at: created_at.with_timezone(&Utc),
                    id,
                })
            })
            .map_err(|_| {
                DocumentApiFailure::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "document_cursor",
                    "after_created_at must be RFC 3339 and paired with after_id",
                )
            }),
        _ => Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_cursor",
            "after_created_at and after_id must be supplied together",
        )),
    }
}

async fn list_documents(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<DocumentListQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match document_list_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_documents_for_actor(
            principal.actor_id(),
            query.include_archived,
            cursor.as_ref(),
            limit,
        )
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn search_documents(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<DocumentSearchQuery>,
) -> Response<Body> {
    if query.q.trim().is_empty() || query.q.len() > MAX_DOCUMENT_SEARCH_QUERY_BYTES {
        return document_api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_search",
            format!(
                "Document search query must contain 1 to {MAX_DOCUMENT_SEARCH_QUERY_BYTES} bytes"
            ),
        );
    }
    if !(0..=MAX_DOCUMENT_SEARCH_OFFSET).contains(&query.offset) {
        return document_api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_search",
            format!("Document search offset must be between 0 and {MAX_DOCUMENT_SEARCH_OFFSET}"),
        );
    }
    let limit = match page_limit(query.limit, MAX_DOCUMENT_SEARCH_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .search_documents_for_actor(
            principal.actor_id(),
            &query.q,
            query.include_archived,
            query.offset,
            limit,
        )
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn create_document(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Json(input): Json<CreateDocumentInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .create_document(restless_orgintel::NewDocument {
            command_id,
            title: &input.title,
            kind: input.kind,
            visibility: input.visibility,
            linked_room_id: input.linked_room_id,
            inherit_room_visibility: input.inherit_room_visibility,
            owner_actor_id: principal.actor_id(),
            created_by_actor_id: principal.actor_id(),
            content_json: &input.content_json,
            reason: &input.reason,
        })
        .await
    {
        Ok(document) => document_json(StatusCode::CREATED, document),
        Err(error) => document_error(error),
    }
}

async fn get_document(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .get_document_for_actor(document, principal.actor_id())
        .await
    {
        Ok(document) => document_json(StatusCode::OK, document),
        Err(error) => document_error(error),
    }
}

async fn get_document_links(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    Query(query): Query<DocumentLinkQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_LINK_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .document_links_for_actor(document, principal.actor_id(), limit)
        .await
    {
        Ok(links) => document_json(StatusCode::OK, links),
        Err(error) => document_error(error),
    }
}

async fn import_document_markdown(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<ImportDocumentMarkdownInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .import_markdown_as_named_document_version(restless_orgintel::ImportDocumentMarkdown {
            command_id,
            document_id: document,
            actor_id: principal.actor_id(),
            expected_current_version_id: input.expected_current_version_id,
            markdown: &input.markdown,
            reason: &input.reason,
        })
        .await
    {
        Ok(result) => document_json(StatusCode::CREATED, result),
        Err(error) => document_error(error),
    }
}

async fn update_document(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<UpdateDocumentInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .update_document_metadata(restless_orgintel::UpdateDocumentMetadata {
            command_id,
            document_id: document,
            actor_id: principal.actor_id(),
            expected_version: input.expected_version,
            title: &input.title,
            kind: input.kind,
            visibility: input.visibility,
            linked_room_id: input.linked_room_id,
            inherit_room_visibility: input.inherit_room_visibility,
        })
        .await
    {
        Ok(document) => document_json(StatusCode::OK, document),
        Err(error) => document_error(error),
    }
}

async fn list_document_versions(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    Query(query): Query<VersionListQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_VERSION_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match query.before_version_number {
        Some(version_number) if version_number <= 0 => {
            return document_api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "document_cursor",
                "before_version_number must be positive",
            );
        }
        Some(version_number) => Some(restless_orgintel::DocumentVersionCursor { version_number }),
        None => None,
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_versions_for_actor(document, principal.actor_id(), cursor.as_ref(), limit)
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn create_document_version(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<CreateDocumentVersionInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let mut result = org
        .create_named_document_version(restless_orgintel::NewNamedDocumentVersion {
            command_id,
            document_id: document,
            actor_id: principal.actor_id(),
            expected_current_version_id: input.expected_current_version_id,
            content_json: &input.content_json,
            reason: &input.reason,
        })
        .await;
    if matches!(&result, Err(restless_orgintel::DocumentError::Conflict(_))) {
        // Browser sync acknowledges Yjs delivery, not the sidecar's debounced
        // PostgreSQL save. Flush the existing live body before retrying Core's
        // guarded checkpoint; a real content/version mismatch still conflicts.
        let root = match &state.source {
            RoomOrgIntelSource::Daemon(daemon) => Some(&daemon.root),
            #[cfg(test)]
            RoomOrgIntelSource::Fixed { .. } => None,
        };
        if let Some(root) = root {
            if let Err(error) = state
                .native_documents_proxy
                .agent_body(
                    root,
                    &org,
                    principal.actor_id(),
                    document,
                    &state.document_collaboration_issuer,
                    &serde_json::json!({"action":"read"}),
                )
                .await
            {
                tracing::warn!(%error, %document, "could not persist live body before checkpoint retry");
                return document_api_error(StatusCode::SERVICE_UNAVAILABLE, "document_collaboration_unavailable",
                    "Live document persistence is unavailable. Keep this tab open and retry the same save.");
            }
            result = org
                .create_named_document_version(restless_orgintel::NewNamedDocumentVersion {
                    command_id,
                    document_id: document,
                    actor_id: principal.actor_id(),
                    expected_current_version_id: input.expected_current_version_id,
                    content_json: &input.content_json,
                    reason: &input.reason,
                })
                .await;
        }
    }
    match result {
        Ok(document) => document_json(StatusCode::CREATED, document),
        Err(error) => document_error(error),
    }
}

async fn get_document_version(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, version)): AxumPath<(String, Uuid, Uuid)>,
) -> Response<Body> {
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .get_document_version_for_actor(document, version, principal.actor_id())
        .await
    {
        Ok(version) => document_json(StatusCode::OK, version),
        Err(error) => document_error(error),
    }
}

async fn export_document_markdown(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, version)): AxumPath<(String, Uuid, Uuid)>,
) -> Response<Body> {
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .export_document_version_as_markdown(document, version, principal.actor_id())
        .await
    {
        Ok(export) => {
            let mut response = Response::new(Body::from(export.markdown));
            *response.status_mut() = StatusCode::OK;
            response.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("text/markdown; charset=utf-8"),
            );
            let disposition =
                format!("attachment; filename=\"document-{document}-version-{version}.md\"");
            if let Ok(disposition) = HeaderValue::from_str(&disposition) {
                response
                    .headers_mut()
                    .insert(CONTENT_DISPOSITION, disposition);
            }
            no_store(response)
        }
        Err(error) => document_error(error),
    }
}

async fn restore_document_version(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, version)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<RestoreDocumentVersionInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .restore_document_version(restless_orgintel::RestoreDocumentVersion {
            command_id,
            document_id: document,
            actor_id: principal.actor_id(),
            expected_current_version_id: input.expected_current_version_id,
            source_version_id: version,
            reason: &input.reason,
        })
        .await
    {
        Ok(document) => document_json(StatusCode::CREATED, document),
        Err(error) => document_error(error),
    }
}

async fn list_document_participants(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    Query(query): Query<ParticipantListQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_PARTICIPANT_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match participant_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_participants_for_actor(
            document,
            principal.actor_id(),
            cursor.as_ref(),
            limit,
        )
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn set_document_participant(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, participant)): AxumPath<(String, Uuid, String)>,
    headers: HeaderMap,
    Json(input): Json<SetDocumentParticipantInput>,
) -> Response<Body> {
    if let Err(failure) = validate_participant_actor_id(&participant) {
        return failure.into_response();
    }
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .set_document_participant(restless_orgintel::SetDocumentParticipant {
            command_id,
            document_id: document,
            actor_id: principal.actor_id(),
            expected_document_version: input.expected_document_version,
            participant_actor_id: &participant,
            access: input.access,
        })
        .await
    {
        Ok(result) => document_json(StatusCode::OK, result),
        Err(error) => document_error(error),
    }
}

async fn remove_document_participant(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, participant)): AxumPath<(String, Uuid, String)>,
    headers: HeaderMap,
    Json(input): Json<RemoveDocumentParticipantInput>,
) -> Response<Body> {
    if let Err(failure) = validate_participant_actor_id(&participant) {
        return failure.into_response();
    }
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .remove_document_participant(restless_orgintel::RemoveDocumentParticipant {
            command_id,
            document_id: document,
            actor_id: principal.actor_id(),
            expected_document_version: input.expected_document_version,
            participant_actor_id: &participant,
        })
        .await
    {
        Ok(result) => document_json(StatusCode::OK, result),
        Err(error) => document_error(error),
    }
}

async fn create_document_comment(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<CreateDocumentCommentInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    if input.block_id.is_some() {
        // Yjs delivery can precede its debounced database write. Anchor the
        // comment only after the shared body has reached durable storage.
        let root = match &state.source {
            RoomOrgIntelSource::Daemon(daemon) => Some(&daemon.root),
            #[cfg(test)]
            RoomOrgIntelSource::Fixed { .. } => None,
        };
        if let Some(root) = root {
            if let Err(error) = state
                .native_documents_proxy
                .agent_body(
                    root,
                    &org,
                    principal.actor_id(),
                    document,
                    &state.document_collaboration_issuer,
                    &serde_json::json!({"action":"read"}),
                )
                .await
            {
                tracing::warn!(%error, %document, "could not persist live body before anchoring comment");
                return document_api_error(StatusCode::SERVICE_UNAVAILABLE,
                    "document_collaboration_unavailable",
                    "The document could not finish syncing. Keep this tab open and retry your comment.");
            }
        }
    }
    match org
        .create_document_comment_thread(restless_orgintel::NewDocumentCommentThread {
            document_id: document,
            actor_id: principal.actor_id(),
            command_id,
            block_id: input.block_id.as_deref(),
            content_json: &input.content_json,
        })
        .await
    {
        Ok(comment) => document_json(StatusCode::CREATED, comment),
        Err(error) => document_error(error),
    }
}

async fn list_document_comments(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    Query(query): Query<CreatedPageQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_CHILD_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match created_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_comment_threads(document, principal.actor_id(), cursor.as_ref(), limit)
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn reply_to_document_comment(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, thread)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<ReplyToDocumentCommentInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .reply_to_document_comment(restless_orgintel::ReplyToDocumentComment {
            document_id: document,
            thread_id: thread,
            reply_to_comment_id: input.reply_to_comment_id,
            actor_id: principal.actor_id(),
            command_id,
            content_json: &input.content_json,
        })
        .await
    {
        Ok(comment) => document_json(StatusCode::CREATED, comment),
        Err(error) => document_error(error),
    }
}

async fn list_document_comment_replies(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, thread)): AxumPath<(String, Uuid, Uuid)>,
    Query(query): Query<CreatedPageQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match created_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_comments(
            document,
            thread,
            principal.actor_id(),
            cursor.as_ref(),
            limit,
        )
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn resolve_document_comment(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, thread)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<ResolveDocumentCommentInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .resolve_document_comment_thread(restless_orgintel::ResolveDocumentCommentThread {
            document_id: document,
            thread_id: thread,
            actor_id: principal.actor_id(),
            command_id,
            expected_thread_version: input.expected_thread_version,
        })
        .await
    {
        Ok(thread) => document_json(StatusCode::OK, thread),
        Err(error) => document_error(error),
    }
}

async fn list_document_reviews(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    Query(query): Query<CreatedPageQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_CHILD_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match created_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_reviews(document, principal.actor_id(), cursor.as_ref(), limit)
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn request_document_review(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<RequestDocumentReviewInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .request_document_review(restless_orgintel::RequestDocumentReview {
            document_id: document,
            actor_id: principal.actor_id(),
            command_id,
            expected_document_version: input.expected_document_version,
            expected_current_version_id: input.expected_current_version_id,
            summary: &input.summary,
            // A human owner/member may request an ordinary document review.
            // Only a capability-bound hosted Runtime Attempt may bind that
            // review to Work; the coordination route supplies that operation.
            work_dependency: None,
        })
        .await
    {
        Ok(review) => document_json(StatusCode::CREATED, review),
        Err(error) => document_error(error),
    }
}

async fn get_document_review(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, review)): AxumPath<(String, Uuid, Uuid)>,
) -> Response<Body> {
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .get_document_review_for_actor(document, review, principal.actor_id())
        .await
    {
        Ok(review) => document_json(StatusCode::OK, review),
        Err(error) => document_error(error),
    }
}

async fn accept_document_review(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, review)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<AcceptDocumentReviewInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .accept_document_review(restless_orgintel::AcceptDocumentReview {
            document_id: document,
            review_id: review,
            actor_id: principal.actor_id(),
            command_id,
            expected_document_version: input.expected_document_version,
            expected_review_version: input.expected_review_version,
            accepted_version_name: &input.accepted_version_name,
            feedback: &input.feedback,
        })
        .await
    {
        Ok(resolution) => document_json(StatusCode::OK, resolution),
        Err(error) => document_error(error),
    }
}

async fn request_document_review_changes(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, review)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<RequestDocumentReviewChangesInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .request_document_review_changes(restless_orgintel::RequestDocumentReviewChanges {
            document_id: document,
            review_id: review,
            actor_id: principal.actor_id(),
            command_id,
            expected_document_version: input.expected_document_version,
            expected_review_version: input.expected_review_version,
            feedback: &input.feedback,
        })
        .await
    {
        Ok(resolution) => document_json(StatusCode::OK, resolution),
        Err(error) => document_error(error),
    }
}

async fn list_document_proposals(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    Query(query): Query<CreatedPageQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_CHILD_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match created_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_revision_proposals(document, principal.actor_id(), cursor.as_ref(), limit)
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn get_document_proposal(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, proposal)): AxumPath<(String, Uuid, Uuid)>,
) -> Response<Body> {
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .get_document_revision_proposal_for_actor(document, proposal, principal.actor_id())
        .await
    {
        Ok(proposal) => document_json(StatusCode::OK, proposal),
        Err(error) => document_error(error),
    }
}

async fn accept_document_proposal(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, proposal)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<ResolveDocumentProposalInput>,
) -> Response<Body> {
    let Some(accepted_version_name) = input.accepted_version_name.as_deref() else {
        return document_api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document",
            "accepting a proposal requires accepted_version_name",
        );
    };
    resolve_document_proposal(
        state,
        principal,
        company,
        document,
        proposal,
        headers,
        input.expected_proposal_version,
        restless_orgintel::DocumentRevisionDecision::Accept,
        &input.resolution_summary,
        Some(accepted_version_name),
    )
    .await
}

async fn reject_document_proposal(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, proposal)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<ResolveDocumentProposalInput>,
) -> Response<Body> {
    if input.accepted_version_name.is_some() {
        return document_api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document",
            "rejecting a proposal cannot include accepted_version_name",
        );
    }
    resolve_document_proposal(
        state,
        principal,
        company,
        document,
        proposal,
        headers,
        input.expected_proposal_version,
        restless_orgintel::DocumentRevisionDecision::Reject,
        &input.resolution_summary,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn resolve_document_proposal(
    state: RoomApiState,
    principal: RequestPrincipal,
    company: String,
    document: Uuid,
    proposal: Uuid,
    headers: HeaderMap,
    expected_proposal_version: i64,
    decision: restless_orgintel::DocumentRevisionDecision,
    resolution_summary: &str,
    accepted_version_name: Option<&str>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .resolve_document_revision_proposal(restless_orgintel::ResolveDocumentRevisionProposal {
            document_id: document,
            proposal_id: proposal,
            actor_id: principal.actor_id(),
            command_id,
            expected_proposal_version,
            decision,
            resolution_summary,
            accepted_version_name,
        })
        .await
    {
        Ok(resolution) => document_json(StatusCode::OK, resolution),
        Err(error) => document_error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tower::ServiceExt as _;

    #[test]
    fn collaboration_proxy_route_is_uuid_exact() {
        let company = Uuid::new_v4();
        let document = Uuid::new_v4();
        let exact = format!("/api/companies/{company}/documents/{document}/collaboration");
        assert!(is_collaboration_proxy_path(&exact));
        for invalid in [
            format!("{exact}/"),
            format!("{exact}/token"),
            format!(
                "/api/companies/{}/documents/{document}/collaboration",
                company.to_string().to_uppercase()
            ),
            format!("/api/companies/company-slug/documents/{document}/collaboration"),
            format!("/api/companies/{company}/documents/not-a-uuid/collaboration"),
        ] {
            assert!(!is_collaboration_proxy_path(&invalid), "accepted {invalid}");
        }
    }

    #[test]
    fn native_documents_uses_the_cell_secret_file_and_observes_rotation() {
        use axum::http::header::AUTHORIZATION;
        if std::env::var_os("RESTLESS_DOCS_READINESS_TEST_CHILD").is_some() {
            let proxy = NativeDocumentsProxy::from_environment().unwrap();
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_static("Bearer native-documents-readiness-secret-123456"),
            );
            assert!(proxy.readiness_authorized(&headers));
            println!("MOUNTED_READINESS_AUTHORIZED");
            return;
        }
        struct Scratch(std::path::PathBuf);
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let directory =
            std::env::temp_dir().join(format!("restless-docs-readiness-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let directory = Scratch(directory);
        let path = directory.0.join("cell_readiness_token");
        let secret = b"native-documents-readiness-secret-123456".to_vec();
        std::fs::write(&path, &secret).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let proxy = NativeDocumentsProxy {
            client: reqwest::Client::new(),
            readiness_secret: Some(ReadinessSecret::read(&path).unwrap()),
            service: NativeDocumentsService::PerCellDns,
        };
        // Exercise the actual environment loader in a child so parallel tests
        // never see a process-wide secret environment mutation.
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "owner::documents_api::tests::native_documents_uses_the_cell_secret_file_and_observes_rotation", "--nocapture"])
            .env("RESTLESS_DOCS_READINESS_TEST_CHILD", "1")
            .env(CELL_READINESS_TOKEN_FILE_ENV, &path)
            .env_remove("RESTLESS_CELL_READINESS_TOKEN")
            .output().unwrap();
        assert!(
            child.status.success(),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        assert!(String::from_utf8_lossy(&child.stdout).contains("MOUNTED_READINESS_AUTHORIZED"));
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!(
                "Bearer {}",
                String::from_utf8(secret.clone()).unwrap()
            ))
            .unwrap(),
        );
        assert!(proxy.readiness_authorized(&headers));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer native-documents-readiness-secret-123457"),
        );
        assert!(!proxy.readiness_authorized(&headers));
        let replacement = directory.0.join("replacement");
        std::fs::write(&replacement, b"native-documents-readiness-secret-123457").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&replacement, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        std::fs::rename(&replacement, &path).unwrap();
        assert!(proxy.readiness_authorized(&headers));
        headers.append(AUTHORIZATION, HeaderValue::from_static("Bearer duplicate"));
        assert!(!proxy.readiness_authorized(&headers));
        headers.remove(AUTHORIZATION);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer native-documents-readiness-secret-123456"),
        );
        assert!(!proxy.readiness_authorized(&headers));
        std::fs::remove_file(&path).unwrap();
        assert!(!proxy.readiness_authorized(&headers));
    }

    #[tokio::test]
    async fn sidecar_readiness_requires_the_exact_released_observation() {
        async fn serve(value: serde_json::Value) -> (u16, tokio::task::JoinHandle<()>) {
            let app = Router::new().route(
                NATIVE_DOCUMENTS_READINESS_PATH,
                get(move || {
                    let value = value.clone();
                    async move { Json(value) }
                }),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            (port, task)
        }

        let (port, task) = serve(serde_json::json!({
            "status": "ready",
            "protocol_version": NATIVE_DOCUMENTS_PROTOCOL_VERSION,
            "schema_version": NATIVE_DOCUMENTS_SCHEMA_VERSION,
        }))
        .await;
        let proxy = NativeDocumentsProxy {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            readiness_secret: None,
            service: NativeDocumentsService::Fixed {
                host: "127.0.0.1".into(),
                port,
            },
        };
        assert_eq!(
            proxy.observe_readiness(Uuid::new_v4()).await.unwrap(),
            NativeDocumentsHealth {
                status: "ready".into(),
                protocol_version: NATIVE_DOCUMENTS_PROTOCOL_VERSION,
                schema_version: NATIVE_DOCUMENTS_SCHEMA_VERSION,
            }
        );
        task.abort();

        let (port, task) = serve(serde_json::json!({
            "status": "ready",
            "protocol_version": NATIVE_DOCUMENTS_PROTOCOL_VERSION,
            "schema_version": NATIVE_DOCUMENTS_SCHEMA_VERSION,
            "invented": true,
        }))
        .await;
        let proxy = NativeDocumentsProxy {
            service: NativeDocumentsService::Fixed {
                host: "127.0.0.1".into(),
                port,
            },
            ..proxy
        };
        assert!(proxy.observe_readiness(Uuid::new_v4()).await.is_err());
        task.abort();
    }

    #[test]
    fn markdown_import_body_limit_covers_worst_case_json_escaping() {
        let one_control_byte = serde_json::to_string("\u{1}").unwrap();
        assert_eq!(one_control_byte.len() - 2, 6);
        let non_markdown_envelope = serde_json::to_vec(&serde_json::json!({
            "expected_current_version_id": Uuid::nil(),
            "markdown": "",
            "reason": "\u{1}".repeat(500),
        }))
        .unwrap()
        .len();
        assert!(non_markdown_envelope <= DOCUMENT_MARKDOWN_IMPORT_ENVELOPE_HEADROOM);
        assert_eq!(
            DOCUMENT_MARKDOWN_IMPORT_BODY_LIMIT,
            restless_orgintel::MAX_NATIVE_DOCUMENT_MARKDOWN_CHECKPOINT_BYTES
                * DOCUMENT_MARKDOWN_JSON_ESCAPE_BYTES_PER_BYTE
                + DOCUMENT_MARKDOWN_IMPORT_ENVELOPE_HEADROOM
        );
    }

    #[test]
    fn human_document_review_payload_cannot_create_a_work_binding() {
        let input = serde_json::json!({
            "expected_document_version": 1,
            "expected_current_version_id": Uuid::new_v4(),
            "summary": "Review this named version",
            "work_dependency": {
                "work_id": Uuid::new_v4(),
                "attempt_id": Uuid::new_v4(),
                "expected_work_revision": 1,
                "reviewer_actor_id": "owner"
            }
        });
        assert!(
            serde_json::from_value::<super::RequestDocumentReviewInput>(input).is_err(),
            "the human HTTP route must reject Runtime-only Work binding fields"
        );
    }

    struct DocumentRouteFixture {
        state: RoomApiState,
        company: String,
        other_company: String,
        org: restless_orgintel::OrgIntel,
    }

    impl DocumentRouteFixture {
        async fn new() -> Option<Self> {
            let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
            let suffix = Uuid::new_v4().simple().to_string();
            let company = format!("document_api_a_{suffix}");
            let other_company = format!("document_api_b_{suffix}");
            let mut companies = HashMap::new();
            for name in [&company, &other_company] {
                let org = restless_orgintel::OrgIntel::ensure(&database_url, name)
                    .await
                    .expect("ensure Document route fixture company");
                org.ensure_actor("owner", "owner", "owner", "The Owner")
                    .await
                    .unwrap();
                org.ensure_actor("exec", "exec", "exec", "The Exec")
                    .await
                    .unwrap();
                org.ensure_actor("alice", "human", "member", "Alice")
                    .await
                    .unwrap();
                org.ensure_actor("blair", "human", "member", "Blair")
                    .await
                    .unwrap();
                org.ensure_actor("research-analyst", "staff", "analyst", "Research Analyst")
                    .await
                    .unwrap();
                org.ensure_company_access_identity(restless_orgintel::CompanyAccessIdentity {
                    company_id: Uuid::new_v4(),
                    cell_id: Uuid::new_v4(),
                })
                .await
                .unwrap();
                companies.insert(name.to_string(), org);
            }
            let org = companies.get(&company).expect("primary company").clone();
            Some(Self {
                state: RoomApiState::fixed(companies, database_url),
                company,
                other_company,
                org,
            })
        }

        fn identity(actor: &str, role: &str, company: &str) -> VerifiedIdentity {
            VerifiedIdentity {
                user: format!("user-{actor}"),
                issuer: None,
                owner: "fixture-owner".into(),
                scope: CompanyScope::Company {
                    company: company.to_string(),
                },
                role: role.to_string(),
                actor: Some(actor.to_string()),
                company_id: None,
                cell_id: None,
                membership_id: None,
                membership_version: None,
            }
        }

        fn app(&self, actor: &str, role: &str, company: &str) -> Router {
            let principal = RequestPrincipal::from_verified(&Self::identity(actor, role, company))
                .expect("verified fixture principal");
            routes::<RoomApiState>()
                .layer(Extension(principal))
                .with_state(self.state.clone())
        }

        fn unauthenticated_app(&self) -> Router {
            routes::<RoomApiState>().with_state(self.state.clone())
        }
    }

    fn content(block_id: &str, text: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "attrs": {"block_id": block_id},
                "content": [{"type": "text", "text": text}]
            }]
        })
    }

    fn create_body(title: &str) -> serde_json::Value {
        serde_json::json!({
            "title": title,
            "kind": "brief",
            "visibility": "participants",
            "linked_room_id": null,
            "inherit_room_visibility": false,
            "content_json": content("claim", "Initial evidence"),
            "reason": "Initial version"
        })
    }

    fn uuid_field(value: &serde_json::Value, field: &str) -> Uuid {
        Uuid::parse_str(value[field].as_str().unwrap()).unwrap()
    }

    async fn request(
        app: &Router,
        method: Method,
        uri: impl AsRef<str>,
        idempotency_key: Option<Uuid>,
        body: Option<serde_json::Value>,
    ) -> Response<Body> {
        let mut builder = Request::builder().method(method).uri(uri.as_ref());
        if let Some(key) = idempotency_key {
            builder = builder.header(IDEMPOTENCY_KEY, key.to_string());
        }
        let body = match body {
            Some(body) => {
                builder = builder.header(CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(&body).unwrap())
            }
            None => Body::empty(),
        };
        app.clone()
            .oneshot(builder.body(body).unwrap())
            .await
            .expect("Document route response")
    }

    async fn response_json(response: Response<Body>) -> (StatusCode, serde_json::Value) {
        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read Document route response");
        let body = serde_json::from_slice(&bytes).unwrap_or_else(
            |_| serde_json::json!({"raw": String::from_utf8_lossy(&bytes).to_string()}),
        );
        (status, body)
    }

    async fn request_json(
        app: &Router,
        method: Method,
        uri: impl AsRef<str>,
        idempotency_key: Option<Uuid>,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        response_json(request(app, method, uri, idempotency_key, body).await).await
    }

    #[test]
    fn document_cursor_requires_an_exact_pair() {
        let query = DocumentListQuery {
            before_document_id: Some(Uuid::new_v4()),
            ..Default::default()
        };
        assert!(document_list_cursor(&query).is_err());

        let query = CreatedPageQuery {
            after_created_at: Some(Utc::now().to_rfc3339()),
            ..Default::default()
        };
        assert!(created_cursor(&query).is_err());

        let query = ParticipantListQuery {
            after_actor_id: Some("alice".into()),
            ..Default::default()
        };
        assert!(participant_cursor(&query).is_err());

        let query = ParticipantListQuery {
            after_added_at: Some(Utc::now().to_rfc3339()),
            after_actor_id: Some("x".repeat(256)),
            ..Default::default()
        };
        assert!(participant_cursor(&query).is_err());
    }

    #[test]
    fn document_page_limits_are_bounded() {
        assert!(page_limit(Some(0), 50).is_err());
        assert!(page_limit(Some(51), 50).is_err());
        assert_eq!(page_limit(None, 50).unwrap(), DEFAULT_PAGE_LIMIT);
        assert_eq!(page_limit(None, 25).unwrap(), 25);
    }

    #[tokio::test]
    async fn document_routes_require_exact_authenticated_company_and_actor_scope() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping Document route security scenario"
            );
            return;
        };
        let documents = format!("/companies/{}/documents", fixture.company);
        let (status, body) = request_json(
            &fixture.unauthenticated_app(),
            Method::GET,
            &documents,
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"], "no_session");

        let owner = fixture.app("owner", "owner", &fixture.company);
        let (status, created) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(create_body("Private evidence")),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = Uuid::parse_str(created["document_id"].as_str().unwrap()).unwrap();

        let (status, body) = request_json(
            &owner,
            Method::GET,
            format!("/companies/{}/documents/{document}", fixture.other_company),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "company_out_of_scope");

        let alice = fixture.app("alice", "member", &fixture.company);
        let (status, body) = request_json(
            &alice,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "document");

        let agent = fixture.app("research-analyst", "member", &fixture.company);
        let (status, body) = request_json(
            &agent,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "request_principal");
    }

    #[tokio::test]
    async fn work_bound_review_projection_routes_the_exact_designated_reviewer() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping Work-bound review projection scenario"
            );
            return;
        };
        let owner = fixture.app("owner", "owner", &fixture.company);
        let alice = fixture.app("alice", "member", &fixture.company);
        let blair = fixture.app("blair", "member", &fixture.company);
        let documents = format!("/companies/{}/documents", fixture.company);
        let (status, created) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(create_body("Exact review assignment")),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = uuid_field(&created, "document_id");
        let (status, document_view) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let current_version_id = Uuid::parse_str(
            document_view["current_version"]["version"]["id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        let (status, _) = request_json(
            &owner,
            Method::PUT,
            format!("{documents}/{document}/participants/alice"),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_document_version": 1,
                "access": "edit"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = request_json(
            &owner,
            Method::PUT,
            format!("{documents}/{document}/participants/research-analyst"),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_document_version": 2,
                "access": "edit"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let work_id = fixture
            .org
            .add_work(restless_orgintel::NewWork {
                owner_id: "research-analyst",
                title: "Ship reviewed evidence",
                outcome: "Ship reviewed evidence",
                goal_id: None,
                priority: 1,
                expected_artifact: "accepted native Document",
                workspace: restless_orgintel::WorkspaceSpec::default(),
                attempt_limit: Some(1),
            })
            .await
            .unwrap();
        let attempt = fixture
            .org
            .claim_ready_work("document-review-http-test")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(attempt.work.id, work_id);
        fixture
            .org
            .finish_work_attempt(
                attempt.attempt_id,
                restless_orgintel::WorkAttemptState::Blocked,
                "waiting for exact native Document review",
            )
            .await
            .unwrap();

        let command_id = Uuid::new_v4();
        let requested = fixture
            .org
            .request_document_review(restless_orgintel::RequestDocumentReview {
                document_id: document,
                actor_id: "research-analyst",
                command_id,
                expected_document_version: 3,
                expected_current_version_id: current_version_id,
                summary: "Alice reviews the exact evidence",
                work_dependency: Some(restless_orgintel::DocumentWorkReviewDependencyInput {
                    work_id,
                    attempt_id: attempt.attempt_id,
                    expected_work_revision: attempt.work.revision,
                    reviewer_actor_id: "alice",
                }),
            })
            .await
            .unwrap();
        assert_eq!(requested.work_dependency.as_ref().unwrap().work_id, work_id);
        assert_eq!(
            requested
                .work_dependency
                .as_ref()
                .unwrap()
                .reviewer_actor_id,
            "alice"
        );
        let review_id = requested.review.id.to_string();
        let replay = fixture
            .org
            .request_document_review(restless_orgintel::RequestDocumentReview {
                document_id: document,
                actor_id: "research-analyst",
                command_id,
                expected_document_version: 3,
                expected_current_version_id: current_version_id,
                summary: "Alice reviews the exact evidence",
                work_dependency: Some(restless_orgintel::DocumentWorkReviewDependencyInput {
                    work_id,
                    attempt_id: attempt.attempt_id,
                    expected_work_revision: attempt.work.revision,
                    reviewer_actor_id: "alice",
                }),
            })
            .await
            .unwrap();
        assert_eq!(replay.review.id, requested.review.id);
        assert_eq!(
            replay.work_dependency.as_ref().unwrap().work_id,
            requested.work_dependency.as_ref().unwrap().work_id
        );

        let reviews = format!("{documents}/{document}/reviews");
        let (status, assigned) = request_json(
            &alice,
            Method::GET,
            format!("{reviews}/{review_id}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(assigned["work_dependency"]["work_id"], work_id.to_string());
        let (status, assigned_page) = request_json(&alice, Method::GET, &reviews, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            assigned_page["items"][0]["work_dependency"]["reviewer_actor_id"],
            "alice"
        );

        let (status, _) = request_json(
            &blair,
            Method::GET,
            format!("{reviews}/{review_id}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = request_json(&blair, Method::GET, &reviews, None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn collaboration_tokens_and_jwks_are_exact_scoped_and_public_key_only() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping Document collaboration-token scenario"
            );
            return;
        };
        let owner = fixture.app("owner", "owner", &fixture.company);
        let documents = format!("/companies/{}/documents", fixture.company);
        let mut company_body = create_body("Shared evidence");
        company_body["visibility"] = serde_json::Value::String("company".into());
        let (status, created) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(company_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = uuid_field(&created, "document_id");

        let token_path = format!("{documents}/{document}/collaboration/token");
        let (status, owner_token) =
            request_json(&owner, Method::POST, &token_path, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(owner_token["token_type"], "Bearer");
        assert_eq!(owner_token["access"], "write");
        let identity = fixture
            .org
            .company_access_identity()
            .await
            .unwrap()
            .unwrap();
        let owner_principal = RequestPrincipal::from_verified(&DocumentRouteFixture::identity(
            "owner",
            "owner",
            &fixture.company,
        ))
        .unwrap();
        fixture
            .state
            .document_collaboration_tokens
            .verify_at(
                owner_token["token"].as_str().unwrap(),
                crate::document_collaboration_token::ExpectedDocumentCollaborationScope {
                    issuer: &fixture.state.document_collaboration_issuer,
                    session_principal: owner_principal.cache_partition(),
                    company_id: identity.company_id,
                    document_id: document,
                    actor_id: "owner",
                    access: DocumentCollaborationAccess::Write,
                },
                Utc::now(),
            )
            .unwrap();

        let alice = fixture.app("alice", "member", &fixture.company);
        let (status, alice_token) =
            request_json(&alice, Method::POST, &token_path, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(alice_token["access"], "read");

        let (status, body) = request_json(
            &alice,
            Method::POST,
            format!("{documents}/{}/collaboration/token", Uuid::new_v4()),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "document");

        let (status, body) = request_json(
            &owner,
            Method::POST,
            format!(
                "/companies/{}/documents/{document}/collaboration/token",
                fixture.other_company
            ),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "company_out_of_scope");

        let jwks_app = public_routes::<RoomApiState>().with_state(fixture.state.clone());
        let response = request(
            &jwks_app,
            Method::GET,
            DOCUMENT_COLLABORATION_JWKS_PATH,
            None,
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("public, max-age=300"))
        );
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let jwks: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(jwks["keys"].as_array().unwrap().len(), 1);
        assert!(jwks["keys"][0].get("x").is_some());
        assert!(jwks["keys"][0].get("d").is_none());
    }

    #[tokio::test]
    async fn collaboration_token_route_fails_closed_without_company_identity() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping unprovisioned Document collaboration-token scenario"
            );
            return;
        };
        let company = format!("document_api_unbound_{}", Uuid::new_v4().simple());
        let org = restless_orgintel::OrgIntel::ensure(&database_url, &company)
            .await
            .unwrap();
        org.ensure_actor("owner", "owner", "owner", "The Owner")
            .await
            .unwrap();
        let mut companies = HashMap::new();
        companies.insert(company.clone(), org);
        let state = RoomApiState::fixed(companies, database_url);
        let principal = RequestPrincipal::from_verified(&DocumentRouteFixture::identity(
            "owner", "owner", &company,
        ))
        .unwrap();
        let app = routes::<RoomApiState>()
            .layer(Extension(principal))
            .with_state(state);
        let documents = format!("/companies/{company}/documents");
        let (status, created) = request_json(
            &app,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(create_body("Unprovisioned evidence")),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = uuid_field(&created, "document_id");
        let (status, body) = request_json(
            &app,
            Method::POST,
            format!("{documents}/{document}/collaboration/token"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["error"], "document_collaboration_unavailable");
    }

    #[tokio::test]
    async fn document_routes_bound_and_validate_transport_before_mutation() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Document route input scenario");
            return;
        };
        let owner = fixture.app("owner", "owner", &fixture.company);
        let documents = format!("/companies/{}/documents", fixture.company);

        let (status, body) = request_json(
            &owner,
            Method::POST,
            &documents,
            None,
            Some(create_body("No command")),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "idempotency_key");

        let response = owner
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(&documents)
                    .header(CONTENT_TYPE, "application/json")
                    .header(IDEMPOTENCY_KEY, Uuid::new_v4().to_string())
                    .header(IDEMPOTENCY_KEY, Uuid::new_v4().to_string())
                    .body(Body::from(
                        serde_json::to_vec(&create_body("Ambiguous command")).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = response_json(response).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "idempotency_key");

        let response = owner
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(&documents)
                    .header(CONTENT_TYPE, "application/json")
                    .header(IDEMPOTENCY_KEY, Uuid::new_v4().to_string())
                    .body(Body::from("{not-json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, _) = response_json(response).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let mut unknown = create_body("Unknown field");
        unknown["actor_id"] = serde_json::json!("owner");
        let (status, _) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(unknown),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let (status, body) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "title": "Invalid content",
                "kind": "brief",
                "visibility": "participants",
                "content_json": {"type":"doc","content":[{"type":"raw_html"}]},
                "reason": "Invalid"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "document");

        let (status, _) = request_json(
            &owner,
            Method::GET,
            format!("{documents}?limit=101"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let (status, created) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(create_body("Lifecycle boundary")),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = created["document_id"].as_str().unwrap();
        let (status, _) = request_json(
            &owner,
            Method::PATCH,
            format!("{documents}/{document}"),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_version": 1,
                "title": "Forged acceptance",
                "kind": "brief",
                "status": "accepted",
                "visibility": "participants",
                "linked_room_id": null,
                "inherit_room_visibility": false
            })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, unchanged) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(unchanged["document"]["status"], "draft");
        assert_eq!(unchanged["document"]["version"], 1);

        let oversized = serde_json::json!({
            "title": "Oversized",
            "kind": "brief",
            "visibility": "participants",
            "content_json": content("claim", &"x".repeat(DOCUMENT_BODY_LIMIT + 1)),
            "reason": "Too large"
        });
        let response = request(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(oversized),
        )
        .await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
    }

    #[tokio::test]
    async fn document_command_receipts_survive_concurrency_delay_and_participant_revocation() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Document receipt scenario");
            return;
        };
        let owner = fixture.app("owner", "owner", &fixture.company);
        let alice = fixture.app("alice", "member", &fixture.company);
        let documents = format!("/companies/{}/documents", fixture.company);
        let create_key = Uuid::new_v4();
        let create = create_body("Concurrent command");
        let (first_create, second_create) = tokio::join!(
            request_json(
                &owner,
                Method::POST,
                &documents,
                Some(create_key),
                Some(create.clone())
            ),
            request_json(
                &owner,
                Method::POST,
                &documents,
                Some(create_key),
                Some(create.clone())
            )
        );
        assert_eq!(first_create.0, StatusCode::CREATED);
        assert_eq!(second_create, first_create);
        let created = first_create.1;
        let document = uuid_field(&created, "document_id");
        assert_eq!(created["operation"], "document_create");

        let (status, initial) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let initial_version = Uuid::parse_str(
            initial["current_version"]["version"]["id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();

        let metadata_key = Uuid::new_v4();
        let metadata = serde_json::json!({
            "expected_version": 1,
            "title": "Concurrent command refined",
            "kind": "plan",
            "visibility": "participants",
            "linked_room_id": null,
            "inherit_room_visibility": false
        });
        let document_path = format!("{documents}/{document}");
        let (first_metadata, second_metadata) = tokio::join!(
            request_json(
                &owner,
                Method::PATCH,
                &document_path,
                Some(metadata_key),
                Some(metadata.clone())
            ),
            request_json(
                &owner,
                Method::PATCH,
                &document_path,
                Some(metadata_key),
                Some(metadata.clone())
            )
        );
        assert_eq!(first_metadata.0, StatusCode::OK);
        assert_eq!(second_metadata, first_metadata);
        let metadata_result = first_metadata.1;

        let participants = format!("{document_path}/participants");
        let (status, initial_participants) =
            request_json(&owner, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(initial_participants["items"].as_array().unwrap().len(), 1);
        let (status, _) = request_json(
            &owner,
            Method::GET,
            format!("{participants}?limit=51"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let alice_participant = format!("{participants}/alice");
        let read_key = Uuid::new_v4();
        let read_body = serde_json::json!({
            "expected_document_version": 2,
            "access": "read"
        });
        let (status, read_result) = request_json(
            &owner,
            Method::PUT,
            &alice_participant,
            Some(read_key),
            Some(read_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(read_result["operation"], "participant_set");
        let (status, owner_participants) =
            request_json(&owner, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(owner_participants["items"].as_array().unwrap().len(), 2);
        let (status, _) = request_json(&alice, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = request_json(&alice, Method::GET, &document_path, None, None).await;
        assert_eq!(status, StatusCode::OK);

        let comments = format!("{document_path}/comments");
        let comment_body = serde_json::json!({
            "content_json": content("comment", "Read access cannot comment")
        });
        let (status, _) = request_json(
            &alice,
            Method::POST,
            &comments,
            Some(Uuid::new_v4()),
            Some(comment_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let (status, _) = request_json(
            &owner,
            Method::PUT,
            &alice_participant,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_document_version": 3,
                "access": "comment"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = request_json(&alice, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = request_json(
            &alice,
            Method::POST,
            &comments,
            Some(Uuid::new_v4()),
            Some(comment_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let (status, _) = request_json(
            &owner,
            Method::PUT,
            &alice_participant,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_document_version": 4,
                "access": "edit"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = request_json(&alice, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let (status, _) = request_json(
            &alice,
            Method::PUT,
            format!("{participants}/owner"),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_document_version": 5,
                "access": "read"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let versions = format!("{document_path}/versions");
        let version_key = Uuid::new_v4();
        let version_body = serde_json::json!({
            "expected_current_version_id": initial_version,
            "content_json": content("claim", "Edited by Alice"),
            "reason": "Alice edit"
        });
        let (first_version, second_version) = tokio::join!(
            request_json(
                &alice,
                Method::POST,
                &versions,
                Some(version_key),
                Some(version_body.clone())
            ),
            request_json(
                &alice,
                Method::POST,
                &versions,
                Some(version_key),
                Some(version_body.clone())
            )
        );
        assert_eq!(first_version.0, StatusCode::CREATED);
        assert_eq!(second_version, first_version);
        let version_result = first_version.1;
        let revised_version = uuid_field(&version_result, "result_id");

        let restore_key = Uuid::new_v4();
        let restore_body = serde_json::json!({
            "expected_current_version_id": revised_version,
            "reason": "Restore the starting point"
        });
        let restore_path = format!("{versions}/{initial_version}/restore");
        let (first_restore, second_restore) = tokio::join!(
            request_json(
                &alice,
                Method::POST,
                &restore_path,
                Some(restore_key),
                Some(restore_body.clone())
            ),
            request_json(
                &alice,
                Method::POST,
                &restore_path,
                Some(restore_key),
                Some(restore_body.clone())
            )
        );
        assert_eq!(first_restore.0, StatusCode::CREATED);
        assert_eq!(second_restore, first_restore);
        let restore_result = first_restore.1;

        let remove_key = Uuid::new_v4();
        let remove_body = serde_json::json!({"expected_document_version": 7});
        let (status, remove_result) = request_json(
            &owner,
            Method::DELETE,
            &alice_participant,
            Some(remove_key),
            Some(remove_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(remove_result["operation"], "participant_remove");
        let (status, _) = request_json(&alice, Method::GET, &document_path, None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let replay_version = request_json(
            &alice,
            Method::POST,
            &versions,
            Some(version_key),
            Some(version_body.clone()),
        )
        .await;
        // Checkpoint retries now recheck edit access, even for a saved receipt.
        // Revocation must deny this route without losing the original receipt.
        assert_eq!(replay_version.0, StatusCode::NOT_FOUND);
        let replay_restore = request_json(
            &alice,
            Method::POST,
            &restore_path,
            Some(restore_key),
            Some(restore_body),
        )
        .await;
        assert_eq!(
            replay_restore,
            (StatusCode::CREATED, restore_result.clone())
        );
        let replay_create = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(create_key),
            Some(create.clone()),
        )
        .await;
        assert_eq!(replay_create, (StatusCode::CREATED, created));
        let replay_metadata = request_json(
            &owner,
            Method::PATCH,
            &document_path,
            Some(metadata_key),
            Some(metadata),
        )
        .await;
        assert_eq!(replay_metadata, (StatusCode::OK, metadata_result));
        let replay_read = request_json(
            &owner,
            Method::PUT,
            &alice_participant,
            Some(read_key),
            Some(read_body),
        )
        .await;
        assert_eq!(replay_read, (StatusCode::OK, read_result));
        let replay_remove = request_json(
            &owner,
            Method::DELETE,
            &alice_participant,
            Some(remove_key),
            Some(remove_body),
        )
        .await;
        assert_eq!(replay_remove, (StatusCode::OK, remove_result));
        let (status, final_participants) =
            request_json(&owner, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(final_participants["items"].as_array().unwrap().len(), 1);

        let cross_actor = request_json(
            &alice,
            Method::POST,
            &documents,
            Some(create_key),
            Some(create),
        )
        .await;
        assert_eq!(cross_actor.0, StatusCode::CONFLICT);
        let other_owner = fixture.app("owner", "owner", &fixture.other_company);
        let (status, _) = request_json(
            &other_owner,
            Method::GET,
            format!(
                "/companies/{}/documents/{document}/participants",
                fixture.other_company
            ),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let guessed_document = Uuid::new_v4();
        let (status, _) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{guessed_document}/participants"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let oversized_actor = "x".repeat(256);
        let (status, _) = request_json(
            &owner,
            Method::PUT,
            format!("{participants}/{oversized_actor}"),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_document_version": 8,
                "access": "read"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let (status, _) = request_json(
            &owner,
            Method::PUT,
            &alice_participant,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_document_version": 8,
                "access": "edit"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let replay_after_readmission = request_json(
            &alice,
            Method::POST,
            &versions,
            Some(version_key),
            Some(version_body),
        )
        .await;
        assert_eq!(
            replay_after_readmission,
            (StatusCode::CREATED, version_result)
        );
        let (status, after_replay) =
            request_json(&owner, Method::GET, &document_path, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            after_replay["current_version"]["version"]["id"], restore_result["result_id"],
            "readmission must return the old receipt without applying the edit again"
        );
    }

    #[tokio::test]
    async fn version_history_is_metadata_only_and_full_content_is_single_version_scoped() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Document version bound scenario");
            return;
        };
        let owner = fixture.app("owner", "owner", &fixture.company);
        let documents = format!("/companies/{}/documents", fixture.company);
        let large_text = "x".repeat(300_000);
        let body = serde_json::json!({
            "title": "Large evidence",
            "kind": "brief",
            "visibility": "participants",
            "linked_room_id": null,
            "inherit_room_visibility": false,
            "content_json": content("large", &large_text),
            "reason": "Bound history responses"
        });
        let (status, created) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = uuid_field(&created, "document_id");
        let document_path = format!("{documents}/{document}");
        let (status, document_view) =
            request_json(&owner, Method::GET, &document_path, None, None).await;
        assert_eq!(status, StatusCode::OK);
        let version = document_view["current_version"]["version"]["id"]
            .as_str()
            .unwrap();
        let versions = format!("{document_path}/versions");
        let (status, history) = request_json(&owner, Method::GET, &versions, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(history.to_string().len() < 8_192);
        let summary = &history["items"][0];
        for body_field in ["content_json", "plain_text", "rendered_html", "markdown"] {
            assert!(
                summary.get(body_field).is_none(),
                "history leaked {body_field}"
            );
        }
        let (status, _) = request_json(
            &owner,
            Method::GET,
            format!("{versions}?limit=26"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, full_version) = request_json(
            &owner,
            Method::GET,
            format!("{versions}/{version}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            full_version["version"]["content_json"]["content"][0]["content"][0]["text"]
                .as_str()
                .unwrap()
                .len(),
            large_text.len()
        );
    }

    #[tokio::test]
    async fn document_routes_replay_commands_and_complete_comment_review_and_proposal_flows() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping Document route workflow scenario"
            );
            return;
        };
        let owner = fixture.app("owner", "owner", &fixture.company);
        let documents = format!("/companies/{}/documents", fixture.company);
        let create_key = Uuid::new_v4();
        let request_body = create_body("Evidence plan");
        let (status, created) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(create_key),
            Some(request_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = Uuid::parse_str(created["document_id"].as_str().unwrap()).unwrap();
        let (_, initial_view) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        let initial_version = Uuid::parse_str(
            initial_view["current_version"]["version"]["id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();

        let (status, replay) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(create_key),
            Some(request_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(replay, created);
        let (status, conflict) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(create_key),
            Some(create_body("Different meaning")),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(conflict["error"], "document_conflict");

        let (status, second_document) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(create_body("Second evidence plan")),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let second_document_id = second_document["document_id"].as_str().unwrap();
        let (status, first_document_page) = request_json(
            &owner,
            Method::GET,
            format!("{documents}?limit=1"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first_document_page["items"].as_array().unwrap().len(), 1);
        assert_eq!(
            first_document_page["items"][0]["document"]["id"],
            second_document_id
        );
        let list_cursor = &first_document_page["next_cursor"];
        let (status, second_document_page) = request_json(
            &owner,
            Method::GET,
            format!(
                "{documents}?limit=1&before_updated_at={}&before_document_id={}",
                list_cursor["updated_at"].as_str().unwrap(),
                list_cursor["id"].as_str().unwrap()
            ),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second_document_page["items"].as_array().unwrap().len(), 1);
        assert_eq!(
            second_document_page["items"][0]["document"]["id"],
            document.to_string()
        );

        let metadata_key = Uuid::new_v4();
        let metadata_body = serde_json::json!({
            "expected_version": 1,
            "title": "Evidence plan refined",
            "kind": "plan",
            "visibility": "participants",
            "linked_room_id": null,
            "inherit_room_visibility": false
        });
        let (status, updated) = request_json(
            &owner,
            Method::PATCH,
            format!("{documents}/{document}"),
            Some(metadata_key),
            Some(metadata_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(updated["operation"], "document_metadata_update");
        let (status, replay) = request_json(
            &owner,
            Method::PATCH,
            format!("{documents}/{document}"),
            Some(metadata_key),
            Some(metadata_body),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(replay, updated);
        let (status, updated_view) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(updated_view["document"]["title"], "Evidence plan refined");
        assert_eq!(updated_view["document"]["version"], 2);

        let versions = format!("{documents}/{document}/versions");
        let version_key = Uuid::new_v4();
        let version_body = serde_json::json!({
            "expected_current_version_id": initial_version,
            "content_json": content("claim", "Revised evidence"),
            "reason": "Ready for review"
        });
        let (status, revised) = request_json(
            &owner,
            Method::POST,
            &versions,
            Some(version_key),
            Some(version_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let revised_version = Uuid::parse_str(revised["result_id"].as_str().unwrap()).unwrap();
        let (status, replay) = request_json(
            &owner,
            Method::POST,
            &versions,
            Some(version_key),
            Some(version_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(replay, revised);

        let (status, first_page) = request_json(
            &owner,
            Method::GET,
            format!("{versions}?limit=1"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first_page["items"].as_array().unwrap().len(), 1);
        assert!(first_page["items"][0].get("content_json").is_none());
        assert!(first_page["items"][0].get("plain_text").is_none());
        assert_eq!(first_page["next_cursor"]["version_number"], 2);
        let (status, second_page) = request_json(
            &owner,
            Method::GET,
            format!("{versions}?limit=1&before_version_number=2"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second_page["items"][0]["id"], initial_version.to_string());

        let comments = format!("{documents}/{document}/comments");
        let comment_key = Uuid::new_v4();
        let comment_body = serde_json::json!({
            "block_id": "claim",
            "content_json": content("comment", "Please verify the evidence")
        });
        let (status, comment) = request_json(
            &owner,
            Method::POST,
            &comments,
            Some(comment_key),
            Some(comment_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let thread = Uuid::parse_str(comment["thread"]["thread"]["id"].as_str().unwrap()).unwrap();
        let (status, comment_replay) = request_json(
            &owner,
            Method::POST,
            &comments,
            Some(comment_key),
            Some(comment_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(comment_replay["thread"]["thread"]["id"], thread.to_string());

        let replies = format!("{comments}/{thread}/replies");
        let (status, reply) = request_json(
            &owner,
            Method::POST,
            &replies,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({"content_json": content("reply", "Verified")})),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(reply["thread_id"], thread.to_string());
        let (status, resolved) = request_json(
            &owner,
            Method::POST,
            format!("{comments}/{thread}/resolve"),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({"expected_thread_version": 1})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(resolved["status"], "resolved");

        let reviews = format!("{documents}/{document}/reviews");
        let review_one_key = Uuid::new_v4();
        let review_one_body = serde_json::json!({
            "expected_document_version": 3,
            "expected_current_version_id": revised_version,
            "summary": "Review the evidence"
        });
        let (status, review_one) = request_json(
            &owner,
            Method::POST,
            &reviews,
            Some(review_one_key),
            Some(review_one_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let review_one_id = Uuid::parse_str(review_one["review"]["id"].as_str().unwrap()).unwrap();
        assert!(review_one["work_dependency"].is_null());

        let (status, advanced) = request_json(
            &owner,
            Method::POST,
            &versions,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_current_version_id": revised_version,
                "content_json": content("claim", "Final evidence"),
                "reason": "Review rollover"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let final_version = Uuid::parse_str(advanced["result_id"].as_str().unwrap()).unwrap();
        let review_two_key = Uuid::new_v4();
        let review_two_body = serde_json::json!({
            "expected_document_version": 5,
            "expected_current_version_id": final_version,
            "summary": "Review the final evidence"
        });
        let (status, review_two) = request_json(
            &owner,
            Method::POST,
            &reviews,
            Some(review_two_key),
            Some(review_two_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let review_two_id = Uuid::parse_str(review_two["review"]["id"].as_str().unwrap()).unwrap();
        let (status, review_two_replay) = request_json(
            &owner,
            Method::POST,
            &reviews,
            Some(review_two_key),
            Some(review_two_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(review_two_replay["review"]["id"], review_two_id.to_string());
        assert_eq!(review_two_replay["review"]["status"], "requested");
        assert!(review_two_replay["work_dependency"].is_null());

        let (status, review_one_after) = request_json(
            &owner,
            Method::GET,
            format!("{reviews}/{review_one_id}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(review_one_after["review"]["status"], "stale");
        assert!(review_one_after["work_dependency"].is_null());

        let review_accept_key = Uuid::new_v4();
        let review_accept_body = serde_json::json!({
            "expected_document_version": 6,
            "expected_review_version": 1,
            "accepted_version_name": "Accepted evidence"
        });
        let (status, accepted_review) = request_json(
            &owner,
            Method::POST,
            format!("{reviews}/{review_two_id}/accept"),
            Some(review_accept_key),
            Some(review_accept_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(accepted_review["review"]["status"], "accepted");
        let accepted_version = Uuid::parse_str(
            accepted_review["accepted_version"]["version"]["id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        let (status, accepted_review_replay) = request_json(
            &owner,
            Method::POST,
            format!("{reviews}/{review_two_id}/accept"),
            Some(review_accept_key),
            Some(review_accept_body),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            accepted_review_replay["accepted_version"]["version"]["id"],
            accepted_version.to_string()
        );

        let current = fixture
            .org
            .get_document_for_actor(document, "owner")
            .await
            .unwrap();
        fixture
            .org
            .set_document_participant(restless_orgintel::SetDocumentParticipant {
                command_id: Uuid::new_v4(),
                document_id: document,
                actor_id: "owner",
                expected_document_version: current.document.version,
                participant_actor_id: "research-analyst",
                access: restless_orgintel::DocumentAccess::Edit,
            })
            .await
            .unwrap();
        let proposed_content = content("claim", "Agent-proposed evidence");
        let proposal = fixture
            .org
            .propose_document_revision(restless_orgintel::ProposeDocumentRevision {
                document_id: document,
                actor_id: "research-analyst",
                command_id: Uuid::new_v4(),
                base_version_id: accepted_version,
                scope: restless_orgintel::DocumentRevisionScope::WholeDocument,
                block_id: None,
                proposed_content_json: &proposed_content,
                summary: "Tighten the evidence",
            })
            .await
            .unwrap();
        let proposals = format!("{documents}/{document}/proposals");
        let agent = fixture.app("research-analyst", "member", &fixture.company);
        let (status, body) = request_json(
            &agent,
            Method::POST,
            format!("{proposals}/{}/accept", proposal.id),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_proposal_version": 1,
                "resolution_summary": "Agent cannot self-accept",
                "accepted_version_name": "Invalid"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "request_principal");

        let proposal_accept_key = Uuid::new_v4();
        let proposal_accept_body = serde_json::json!({
            "expected_proposal_version": 1,
            "resolution_summary": "Accepted after review",
            "accepted_version_name": "Agent evidence accepted"
        });
        let (status, accepted) = request_json(
            &owner,
            Method::POST,
            format!("{proposals}/{}/accept", proposal.id),
            Some(proposal_accept_key),
            Some(proposal_accept_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(accepted["proposal"]["status"], "accepted");
        let agent_version = Uuid::parse_str(
            accepted["accepted_version"]["version"]["id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        let (status, accepted_replay) = request_json(
            &owner,
            Method::POST,
            format!("{proposals}/{}/accept", proposal.id),
            Some(proposal_accept_key),
            Some(proposal_accept_body),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            accepted_replay["accepted_version"]["version"]["id"],
            agent_version.to_string()
        );

        let rejected_content = content("claim", "A proposal we should decline");
        let rejected_proposal = fixture
            .org
            .propose_document_revision(restless_orgintel::ProposeDocumentRevision {
                document_id: document,
                actor_id: "research-analyst",
                command_id: Uuid::new_v4(),
                base_version_id: agent_version,
                scope: restless_orgintel::DocumentRevisionScope::WholeDocument,
                block_id: None,
                proposed_content_json: &rejected_content,
                summary: "Remove necessary evidence",
            })
            .await
            .unwrap();
        let reject_key = Uuid::new_v4();
        let reject_body = serde_json::json!({
            "expected_proposal_version": 1,
            "resolution_summary": "The evidence is necessary"
        });
        let (status, rejected) = request_json(
            &owner,
            Method::POST,
            format!("{proposals}/{}/reject", rejected_proposal.id),
            Some(reject_key),
            Some(reject_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(rejected["proposal"]["status"], "rejected");
        assert!(rejected["accepted_version"].is_null());
        let (status, rejected_replay) = request_json(
            &owner,
            Method::POST,
            format!("{proposals}/{}/reject", rejected_proposal.id),
            Some(reject_key),
            Some(reject_body),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(rejected_replay["proposal"]["status"], "rejected");

        let restore_key = Uuid::new_v4();
        let restore_body = serde_json::json!({
            "expected_current_version_id": agent_version,
            "reason": "Restore the original evidence"
        });
        let (status, restored) = request_json(
            &owner,
            Method::POST,
            format!("{versions}/{initial_version}/restore"),
            Some(restore_key),
            Some(restore_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let restored_version = restored["result_id"].as_str().unwrap().to_string();
        let (status, restored_replay) = request_json(
            &owner,
            Method::POST,
            format!("{versions}/{initial_version}/restore"),
            Some(restore_key),
            Some(restore_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(restored_replay, restored);
        let (status, restored_view) = request_json(
            &owner,
            Method::GET,
            format!("{versions}/{restored_version}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            restored_view["version"]["restored_from_version_id"],
            initial_version.to_string()
        );
    }

    #[tokio::test]
    async fn document_retrieval_routes_export_import_search_and_bound_links() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Document retrieval routes");
            return;
        };
        let owner = fixture.app("owner", "owner", &fixture.company);
        let documents = format!("/companies/{}/documents", fixture.company);
        let (status, created) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(create_body("Retrieval route target")),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = uuid_field(&created, "document_id");
        let (status, current) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let version = uuid_field(&current["current_version"]["version"], "id");

        let export_path = format!("{documents}/{document}/versions/{version}/export/markdown");
        let export_response = request(&owner, Method::GET, &export_path, None, None).await;
        assert_eq!(export_response.status(), StatusCode::OK);
        assert_eq!(
            export_response.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
        assert_eq!(
            export_response.headers().get(CONTENT_TYPE),
            Some(&HeaderValue::from_static("text/markdown; charset=utf-8"))
        );
        assert!(export_response
            .headers()
            .get(CONTENT_DISPOSITION)
            .unwrap()
            .to_str()
            .unwrap()
            .contains(&version.to_string()));
        let export = String::from_utf8(
            to_bytes(export_response.into_body(), DOCUMENT_BODY_LIMIT)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(export.contains(&format!("source_document_id: {document}")));
        assert!(export.contains(&format!("source_named_version_id: {version}")));

        let (status, search) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/search?q=Initial%20evidence&limit=1"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(search["items"][0]["document_id"], document.to_string());
        assert!(
            search["items"][0]["snippet"]
                .as_str()
                .unwrap()
                .chars()
                .count()
                <= 400
        );
        let (status, links) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{document}/links?limit=1"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(links["document_id"], document.to_string());
        assert!(links["outgoing"].as_array().unwrap().is_empty());

        let import_path = format!("{documents}/{document}/imports/markdown");
        let edited = export.replace("Initial evidence", "Imported route evidence");
        let import_body = serde_json::json!({
            "expected_current_version_id": version,
            "markdown": edited,
            "reason": "Import the reviewed Markdown"
        });
        let (status, body) = request_json(
            &owner,
            Method::POST,
            &import_path,
            None,
            Some(import_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "idempotency_key");
        let (status, body) = request_json(
            &owner,
            Method::POST,
            &import_path,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_current_version_id": version,
                "markdown": "x".repeat(DOCUMENT_BODY_LIMIT + 1),
                "reason": "Exercise the route-specific transport bound"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "document");
        let import_key = Uuid::new_v4();
        let (status, imported) = request_json(
            &owner,
            Method::POST,
            &import_path,
            Some(import_key),
            Some(import_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(imported["operation"], "markdown_import");
        let replay = request_json(
            &owner,
            Method::POST,
            &import_path,
            Some(import_key),
            Some(import_body.clone()),
        )
        .await;
        assert_eq!(replay, (StatusCode::CREATED, imported));
        let stale = request_json(
            &owner,
            Method::POST,
            &import_path,
            Some(Uuid::new_v4()),
            Some(import_body),
        )
        .await;
        assert_eq!(stale.0, StatusCode::CONFLICT);

        let guessed = Uuid::new_v4();
        let (status, _) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{guessed}/versions/{version}/export/markdown"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        for path in [
            format!("{documents}/search?q={}&limit=1", "q".repeat(257)),
            format!("{documents}/search?q=evidence&offset=10001"),
            format!("{documents}/search?q=evidence&limit=51"),
            format!("{documents}/{document}/links?limit=51"),
        ] {
            let (status, _) = request_json(&owner, Method::GET, path, None, None).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        }
    }
}
