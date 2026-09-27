//! Attempt-scoped bridge to host-owned, loopback Streamable HTTP MCP servers.
//!
//! The company Runtime sees only this Docker-bridge listener and an expiring,
//! single-connection capability. The host-side service token and browser
//! profile never enter the Runtime. rmcp handles both MCP protocol directions;
//! this layer owns only Restless authority and the exact tool allowlist.

use std::net::{IpAddr, SocketAddr};
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use axum::{
    body::{to_bytes, Body},
    extract::{ConnectInfo, DefaultBodyLimit, Path as AxumPath, State},
    http::{header::AUTHORIZATION, HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse as _, Response},
    routing::any,
    Json, Router,
};
use rmcp::{
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData,
        Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig,
        Tool,
    },
    service::RequestContext,
    transport::{
        streamable_http_client::StreamableHttpClientTransportConfig,
        streamable_http_server::{
            session::never::NeverSessionManager, StreamableHttpServerConfig, StreamableHttpService,
        },
        StreamableHttpClientTransport,
    },
    RoleServer, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;
use tower::ServiceExt as _;

use crate::{capability::McpGrant, connected_tool::LocalMcpServer, Daemon};

const PROBE_TIMEOUT: Duration = Duration::from_secs(12);
const CALL_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_RESULT_BYTES: usize = 1024 * 1024;
const MAX_FACADE_REQUEST_BYTES: usize = 16 * 1024;

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
enum ReadRequest {
    MarketplaceSearch {
        query: String,
        limit: Option<u8>,
        #[serde(rename = "fastSearch")]
        fast_search: Option<bool>,
    },
    MarketplaceDetails {
        urls: Vec<String>,
        #[serde(rename = "streamDetails")]
        stream_details: Option<bool>,
    },
    GumtreeListing {
        url: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MarketplaceSearchArgs {
    query: String,
    limit: Option<u8>,
    #[serde(rename = "fastSearch")]
    fast_search: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MarketplaceDetailsArgs {
    urls: Vec<String>,
    #[serde(rename = "streamDetails")]
    stream_details: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GumtreeListingArgs {
    url: String,
}

pub(crate) struct McpProbe {
    pub(crate) names: Vec<String>,
    pub(crate) digest: String,
    pub(crate) server_version: String,
}

fn classify_read_status(result: &CallToolResult) -> String {
    let Some(body) = result.structured_content.as_ref() else {
        return if result.is_error == Some(true) {
            "tool_error"
        } else {
            "unverified"
        }
        .into();
    };
    if result.is_error == Some(true) {
        return body
            .pointer("/error/code")
            .and_then(|value| value.as_str())
            .unwrap_or("tool_error")
            .to_string();
    }
    if let Some(status) = body.get("status").and_then(|value| value.as_str()) {
        return status.to_string();
    }
    if let Some(rows) = body.get("rows").and_then(|value| value.as_array()) {
        if rows.is_empty() {
            return "unverified".into();
        }
        let statuses = rows
            .iter()
            .filter_map(|row| row.get("status").and_then(|value| value.as_str()))
            .collect::<Vec<_>>();
        if statuses.len() != rows.len() {
            return "unverified".into();
        }
        if statuses.iter().all(|status| *status == "complete") {
            return "complete".into();
        }
        if statuses.iter().all(|status| *status == statuses[0]) {
            return statuses[0].to_string();
        }
        return "partial".into();
    }
    if body
        .get("cards")
        .and_then(|value| value.as_array())
        .is_some()
        && body
            .get("observedAt")
            .and_then(|value| value.as_str())
            .is_some()
    {
        return "complete".into();
    }
    "unverified".into()
}

fn valid_listing_url(raw: &str, site: &str) -> bool {
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return false;
    }
    let parts = url.path().split('/').collect::<Vec<_>>();
    match site {
        "facebook" => {
            url.host_str() == Some("www.facebook.com")
                && matches!(parts.as_slice(), ["", "marketplace", "item", id, ""] | ["", "marketplace", "item", id]
                    if !id.is_empty() && id.len() <= 24 && id.bytes().all(|byte| byte.is_ascii_digit()))
        }
        "gumtree" => {
            url.host_str() == Some("www.gumtree.com.au")
                && matches!(parts.as_slice(), ["", "web", "listing", category, id]
                    if !category.is_empty() && category.len() <= 64
                        && category.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                        && !id.is_empty() && id.len() <= 16 && id.bytes().all(|byte| byte.is_ascii_digit()))
        }
        _ => false,
    }
}

fn read_request_params(request: ReadRequest) -> Result<CallToolRequestParams> {
    let (name, arguments) = match request {
        ReadRequest::MarketplaceSearch {
            query,
            limit,
            fast_search,
        } => {
            let query = query.trim();
            if query.is_empty()
                || query.encode_utf16().count() > 100
                || query.chars().any(char::is_control)
                || !limit.is_none_or(|limit| (1..=24).contains(&limit))
            {
                bail!("invalid Marketplace search request");
            }
            let mut arguments = serde_json::json!({"query":query,"limit":limit.unwrap_or(12)});
            if let Some(fast_search) = fast_search {
                arguments["fastSearch"] = serde_json::json!(fast_search);
            }
            ("clapping_hands_marketplace_search", arguments)
        }
        ReadRequest::MarketplaceDetails {
            urls,
            stream_details,
        } => {
            if !(1..=8).contains(&urls.len())
                || !urls.iter().all(|url| valid_listing_url(url, "facebook"))
                || urls.iter().collect::<std::collections::HashSet<_>>().len() != urls.len()
            {
                bail!("invalid Marketplace detail request");
            }
            let mut arguments = serde_json::json!({"urls":urls});
            if let Some(stream_details) = stream_details {
                arguments["streamDetails"] = serde_json::json!(stream_details);
            }
            ("clapping_hands_marketplace_details", arguments)
        }
        ReadRequest::GumtreeListing { url } => {
            if !valid_listing_url(&url, "gumtree") {
                bail!("invalid Gumtree listing request");
            }
            (
                "clapping_hands_gumtree_public_listing",
                serde_json::json!({"url":url}),
            )
        }
    };
    Ok(CallToolRequestParams::new(name).with_arguments(
        arguments
            .as_object()
            .context("MCP facade arguments are not an object")?
            .clone(),
    ))
}

/// The same Attempt grant can reach both the MCP SDK route and the narrow CLI
/// facade. Normalize and validate arguments here so neither transport can
/// widen the reviewed read-only CH profile.
fn validated_read_params(params: CallToolRequestParams) -> Result<CallToolRequestParams> {
    if params.meta.is_some() || params.input_responses.is_some() || params.request_state.is_some() {
        bail!("MCP read request has unsupported protocol fields");
    }
    let arguments = serde_json::Value::Object(
        params
            .arguments
            .context("MCP read arguments are required")?,
    );
    let request = match params.name.as_ref() {
        "clapping_hands_marketplace_search" => {
            let args: MarketplaceSearchArgs = serde_json::from_value(arguments)?;
            ReadRequest::MarketplaceSearch {
                query: args.query,
                limit: args.limit,
                fast_search: args.fast_search,
            }
        }
        "clapping_hands_marketplace_details" => {
            let args: MarketplaceDetailsArgs = serde_json::from_value(arguments)?;
            ReadRequest::MarketplaceDetails {
                urls: args.urls,
                stream_details: args.stream_details,
            }
        }
        "clapping_hands_gumtree_public_listing" => {
            let args: GumtreeListingArgs = serde_json::from_value(arguments)?;
            ReadRequest::GumtreeListing { url: args.url }
        }
        _ => bail!("MCP tool is outside the reviewed read profile"),
    };
    read_request_params(request)
}

pub(crate) fn read_host_token(token_file: &str) -> Result<String> {
    let path = Path::new(token_file);
    if !path.is_absolute() {
        bail!("host MCP token file must be an absolute path");
    }
    let metadata = std::fs::symlink_metadata(path).context("inspect host MCP token file")?;
    if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o077 != 0 {
        bail!("host MCP token file must be a private regular file");
    }
    if metadata.len() > 256 {
        bail!("host MCP token file exceeds its bound");
    }
    let token = std::fs::read_to_string(path).context("read host MCP token file")?;
    let token = token.trim_end_matches(['\n', '\r']);
    if token.len() < 32 || token.len() > 256 || !token.bytes().all(|b| b.is_ascii_graphic()) {
        bail!("host MCP token has an invalid shape");
    }
    Ok(token.to_string())
}

fn transport(endpoint: &str, token: String) -> StreamableHttpClientTransport<reqwest_mcp::Client> {
    let client = reqwest_mcp::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .no_proxy()
        .redirect(reqwest_mcp::redirect::Policy::none())
        .build()
        .expect("fixed local MCP HTTP client configuration");
    StreamableHttpClientTransport::with_client(
        client,
        StreamableHttpClientTransportConfig::with_uri(endpoint.to_string())
            .auth_header(token)
            .max_concurrent_requests(1)
            .max_sse_event_size(MAX_RESULT_BYTES),
    )
}

/// Probe with the upstream MCP SDK. The exact allowlist must be present before
/// Authority records a connection as enabled; newly added upstream tools are
/// never auto-granted.
pub(crate) async fn probe_upstream(
    endpoint: &str,
    token_file: &str,
    allowed: &[String],
) -> Result<McpProbe> {
    let token = read_host_token(token_file)?;
    let client = tokio::time::timeout(PROBE_TIMEOUT, ().serve(transport(endpoint, token)))
        .await
        .context("host MCP handshake timed out")?
        .context("host MCP handshake failed")?;
    let server_version = client
        .peer_info()
        .as_ref()
        .and_then(|info| {
            info.server_info
                .as_ref()
                .map(|server| server.version.clone())
        })
        .context("host MCP did not provide a server version")?;
    let tools = tokio::time::timeout(PROBE_TIMEOUT, client.list_all_tools())
        .await
        .context("host MCP tool discovery timed out")?
        .context("host MCP tool discovery failed")?;
    let _ = client.cancel().await;
    let mut selected = tools
        .into_iter()
        .filter(|tool| allowed.iter().any(|name| name == tool.name.as_ref()))
        .collect::<Vec<_>>();
    selected.sort_by(|a, b| a.name.cmp(&b.name));
    if selected.len() != allowed.len() {
        bail!("host MCP did not expose every permitted tool");
    }
    let encoded = serde_json::to_vec(&selected).context("encode observed MCP contract")?;
    Ok(McpProbe {
        names: selected.iter().map(|tool| tool.name.to_string()).collect(),
        digest: format!("{:x}", Sha256::digest(&encoded)),
        server_version,
    })
}

#[derive(Clone)]
struct ScopedMcp {
    pool: PgPool,
    company: String,
    connection: LocalMcpServer,
    daemon: Arc<Daemon>,
    grant: McpGrant,
}

impl ScopedMcp {
    async fn await_revocation(&self) {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let enabled: Result<Option<bool>, _> = sqlx::query_scalar(
                "SELECT enabled FROM restless_authority.local_mcp_servers WHERE company=$1 AND name=$2",
            )
            .bind(&self.company)
            .bind(&self.connection.name)
            .fetch_optional(&self.pool)
            .await;
            if !matches!(enabled, Ok(Some(true)))
                || !running_attempt(&self.daemon, &self.grant).await
            {
                return;
            }
        }
    }

    async fn discover(&self) -> Result<Vec<Tool>> {
        let endpoint = self
            .connection
            .endpoint
            .as_deref()
            .context("host MCP endpoint missing")?;
        let token_file = self
            .connection
            .token_file
            .as_deref()
            .context("host MCP token missing")?;
        let token = read_host_token(token_file)?;
        let client = tokio::time::timeout(PROBE_TIMEOUT, ().serve(transport(endpoint, token)))
            .await
            .context("MCP handshake timeout")??;
        let tools = tokio::time::timeout(PROBE_TIMEOUT, client.list_all_tools())
            .await
            .context("MCP discovery timeout")??;
        let server_version = client.peer_info().as_ref().and_then(|info| {
            info.server_info
                .as_ref()
                .map(|server| server.version.clone())
        });
        let _ = client.cancel().await;
        let mut selected = tools
            .into_iter()
            .filter(|tool| {
                self.connection
                    .allowed_tools
                    .iter()
                    .any(|name| name == tool.name.as_ref())
            })
            .collect::<Vec<_>>();
        selected.sort_by(|a, b| a.name.cmp(&b.name));
        if selected.len() != self.connection.allowed_tools.len() {
            bail!("permitted MCP tool disappeared upstream");
        }
        let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&selected)?));
        if self.connection.tool_contract_digest.as_deref() != Some(digest.as_str()) {
            bail!("permitted MCP tool definitions changed; owner must reinstall");
        }
        if server_version.as_deref() != self.connection.server_version.as_deref() {
            bail!("MCP server version changed; owner must reinstall");
        }
        let names = selected
            .iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        sqlx::query(
            "UPDATE restless_authority.local_mcp_servers SET observed_tools=$3, \
             last_observed_at=now(),failure=NULL,updated_at=now() \
             WHERE company=$1 AND name=$2 AND enabled=TRUE",
        )
        .bind(&self.company)
        .bind(&self.connection.name)
        .bind(serde_json::to_value(names)?)
        .execute(&self.pool)
        .await?;
        Ok(selected)
    }

    async fn invoke(&self, params: CallToolRequestParams) -> Result<CallToolResult> {
        let params = validated_read_params(params)?;
        let tool_name = params.name.to_string();
        if !self
            .connection
            .allowed_tools
            .iter()
            .any(|name| name == params.name.as_ref())
        {
            bail!("MCP tool is outside this connection's allowlist");
        }
        let endpoint = self
            .connection
            .endpoint
            .as_deref()
            .context("host MCP endpoint missing")?;
        let token_file = self
            .connection
            .token_file
            .as_deref()
            .context("host MCP token missing")?;
        let token = read_host_token(token_file)?;
        let client = tokio::time::timeout(PROBE_TIMEOUT, ().serve(transport(endpoint, token)))
            .await
            .context("MCP handshake timeout")??;
        let server_version = client.peer_info().as_ref().and_then(|info| {
            info.server_info
                .as_ref()
                .map(|server| server.version.clone())
        });
        if server_version.as_deref() != self.connection.server_version.as_deref() {
            let _ = client.cancel().await;
            bail!("MCP server version changed before invocation");
        }
        let tools = tokio::time::timeout(PROBE_TIMEOUT, client.list_all_tools())
            .await
            .context("MCP discovery timeout")??;
        let mut selected = tools
            .into_iter()
            .filter(|tool| {
                self.connection
                    .allowed_tools
                    .iter()
                    .any(|name| name == tool.name.as_ref())
            })
            .collect::<Vec<_>>();
        selected.sort_by(|a, b| a.name.cmp(&b.name));
        let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&selected)?));
        if selected.len() != self.connection.allowed_tools.len()
            || self.connection.tool_contract_digest.as_deref() != Some(digest.as_str())
        {
            let _ = client.cancel().await;
            bail!("MCP tool contract changed before invocation");
        }
        let enabled: Option<bool> = sqlx::query_scalar(
            "SELECT enabled FROM restless_authority.local_mcp_servers WHERE company=$1 AND name=$2",
        )
        .bind(&self.company)
        .bind(&self.connection.name)
        .fetch_optional(&self.pool)
        .await?;
        if enabled != Some(true) || !running_attempt(&self.daemon, &self.grant).await {
            let _ = client.cancel().await;
            bail!("MCP grant was revoked before invocation");
        }
        let result = tokio::select! {
            result = tokio::time::timeout(CALL_TIMEOUT, client.call_tool(params)) => Some(result),
            _ = self.await_revocation() => None,
        };
        let _ = client.cancel().await;
        let result = result
            .context("MCP grant revoked during call")?
            .context("MCP call timed out")??;
        if serde_json::to_vec(&result)?.len() > MAX_RESULT_BYTES {
            bail!("MCP result exceeds its bound");
        }
        let read_status = classify_read_status(&result);
        let complete = result.is_error != Some(true) && read_status == "complete";
        let read_site = result
            .structured_content
            .as_ref()
            .and_then(|body| body.get("source"))
            .and_then(|value| value.as_str())
            .or_else(|| {
                if tool_name.starts_with("clapping_hands_marketplace_") {
                    Some("facebook-marketplace")
                } else if tool_name == "clapping_hands_gumtree_public_listing" {
                    Some("gumtree")
                } else {
                    None
                }
            });
        sqlx::query(
            "UPDATE restless_authority.local_mcp_servers SET \
             last_success_at=CASE WHEN $3 THEN now() ELSE last_success_at END, \
             last_read_status=$4,failure=CASE WHEN $5 THEN $4 ELSE NULL END, \
             last_read_site=$6,last_read_tool=$7,updated_at=now() \
             WHERE company=$1 AND name=$2 AND enabled=TRUE",
        )
        .bind(&self.company)
        .bind(&self.connection.name)
        .bind(complete)
        .bind(&read_status)
        .bind(result.is_error == Some(true))
        .bind(read_site)
        .bind(&tool_name)
        .execute(&self.pool)
        .await?;
        tracing::info!(company=%self.company, name=%self.connection.name, status=read_status,
            "host MCP read completed");
        Ok(result)
    }

    async fn record_failure(&self, class: &str) {
        let _ = sqlx::query(
            "UPDATE restless_authority.local_mcp_servers SET failure=$3,last_read_status='unavailable',updated_at=now() \
             WHERE company=$1 AND name=$2 AND enabled=TRUE",
        )
        .bind(&self.company)
        .bind(&self.connection.name)
        .bind(class)
        .execute(&self.pool)
        .await;
    }
}

impl ServerHandler for ScopedMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("restless-local-mcp", "1"))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, ErrorData> {
        match self.discover().await {
            Ok(tools) => Ok(ListToolsResult::with_all_items(tools)),
            Err(error) => {
                self.record_failure("discovery_failed").await;
                tracing::warn!(company=%self.company, name=%self.connection.name, "host MCP discovery failed: {error:#}");
                Err(ErrorData::internal_error(
                    "connected MCP discovery failed",
                    None,
                ))
            }
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResponse, ErrorData> {
        if !self
            .connection
            .allowed_tools
            .iter()
            .any(|name| name == request.name.as_ref())
        {
            return Err(ErrorData::invalid_params("MCP tool is not permitted", None));
        }
        match self.invoke(request).await {
            Ok(result) => Ok(result.into()),
            Err(error) => {
                self.record_failure("call_failed").await;
                tracing::warn!(company=%self.company, name=%self.connection.name, "host MCP call failed: {error:#}");
                Ok(CallToolResult::error(vec![ContentBlock::text(
                    "Connected MCP read failed or timed out. The result is unknown; inspect the connection status before retrying.",
                )]).into())
            }
        }
    }
}

/// Mount on the already reachable Runtime model relay. This route retains its
/// own actor capability and live Attempt checks; it does not accept a model
/// capability. The peer gate also refuses requests from outside the local
/// Docker bridge and host loopback, even though the relay binds 0.0.0.0.
pub(crate) fn router(daemon: Arc<Daemon>) -> Router {
    Router::new()
        .route("/mcp/{company}/{name}", any(handle))
        .route("/mcp-read/{company}/{name}", any(handle_read))
        .layer(DefaultBodyLimit::max(MAX_RESULT_BYTES))
        .with_state(daemon)
}

async fn handle(
    State(daemon): State<Arc<Daemon>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    AxumPath((company, name)): AxumPath<(String, String)>,
    headers: HeaderMap,
    request: Request<Body>,
) -> Response {
    if request.method() != Method::POST || headers.contains_key("origin") {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let scope = match scoped_connection(daemon, peer, company, name, &headers).await {
        Ok(scope) => scope,
        Err(status) => return status.into_response(),
    };
    let mut config = StreamableHttpServerConfig::default();
    config.legacy_session_mode = false;
    config.json_response = true;
    let port = crate::port_with_offset(crate::model_gateway::RUNTIME_RELAY_PORT)
        .unwrap_or(crate::model_gateway::RUNTIME_RELAY_PORT);
    config.allowed_hosts = vec![format!("host.docker.internal:{port}")];
    let service = StreamableHttpService::new(
        move || Ok(scope.clone()),
        Arc::new(NeverSessionManager::default()),
        config,
    );
    match service.oneshot(request).await {
        Ok(response) => response.into_response(),
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}

async fn handle_read(
    State(daemon): State<Arc<Daemon>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    AxumPath((company, name)): AxumPath<(String, String)>,
    headers: HeaderMap,
    request: Request<Body>,
) -> Response {
    if request.method() != Method::POST || headers.contains_key("origin") {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let scope = match scoped_connection(daemon, peer, company, name, &headers).await {
        Ok(scope) => scope,
        Err(status) => return status.into_response(),
    };
    let body = match to_bytes(request.into_body(), MAX_FACADE_REQUEST_BYTES).await {
        Ok(body) => body,
        Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
    };
    let params = match serde_json::from_slice::<ReadRequest>(&body)
        .context("decode bounded MCP read request")
        .and_then(read_request_params)
    {
        Ok(params) => params,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    tracing::info!(
        company = %scope.company,
        actor = %scope.grant.actor,
        work_id = %scope.grant.work_id,
        attempt_id = %scope.grant.attempt_id,
        tool_name = %params.name,
        "Core MCP read facade invoked"
    );
    match scope.invoke(params).await {
        Ok(result) => Json(result).into_response(),
        Err(error) => {
            scope.record_failure("facade_read_failed").await;
            tracing::warn!(company=%scope.company, name=%scope.connection.name,
                "Core MCP read facade failed: {error:#}");
            StatusCode::BAD_GATEWAY.into_response()
        }
    }
}

async fn scoped_connection(
    daemon: Arc<Daemon>,
    peer: SocketAddr,
    company: String,
    name: String,
    headers: &HeaderMap,
) -> std::result::Result<ScopedMcp, StatusCode> {
    if !local_runtime_peer(peer.ip()) {
        return Err(StatusCode::FORBIDDEN);
    }
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let Some(token) = token else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Ok(grant) = daemon.capabilities.verify_mcp(token) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    if grant.company != company || grant.name != name {
        return Err(StatusCode::FORBIDDEN);
    }
    let Some(connection) =
        (match crate::connected_tool::local_mcp_list(daemon.authority.pool(), &company).await {
            Ok(all) => all.into_iter().find(|item| item.name == name),
            Err(_) => return Err(StatusCode::SERVICE_UNAVAILABLE),
        })
    else {
        return Err(StatusCode::NOT_FOUND);
    };
    if !connection.enabled
        || connection.transport != "host_http"
        || connection.assigned_actor != grant.actor
        || connection.assigned_work_id != Some(grant.work_id)
    {
        return Err(StatusCode::FORBIDDEN);
    }
    if crate::connected_tool::require_reviewed_host_read_profile(
        &connection.name,
        connection.endpoint.as_deref().unwrap_or_default(),
        &connection.allowed_tools,
    )
    .is_err()
    {
        return Err(StatusCode::FORBIDDEN);
    }
    if !running_attempt(&daemon, &grant).await {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(ScopedMcp {
        pool: daemon.authority.pool().clone(),
        company,
        connection,
        daemon,
        grant,
    })
}

fn local_runtime_peer(ip: IpAddr) -> bool {
    if ip.is_loopback() {
        return true;
    }
    matches!(ip, IpAddr::V4(ipv4) if ipv4.octets()[0..2] == [172, 17])
}

async fn running_attempt(daemon: &Daemon, grant: &McpGrant) -> bool {
    let Ok(org) = daemon.orgintel.get(&grant.company).await else {
        return false;
    };
    let Ok(attempts) = org.list_running_work_attempts().await else {
        return false;
    };
    attempts.iter().any(|attempt| {
        attempt.id == grant.attempt_id
            && attempt.work_id == grant.work_id
            && attempt.actor_id == grant.actor
            && attempt.interrupt_requested_at.is_none()
    })
}
