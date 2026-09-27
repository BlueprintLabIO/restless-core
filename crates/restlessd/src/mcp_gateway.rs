//! Attempt-scoped bridge to reviewed HTTP and isolated local stdio MCP reads.
//!
//! The company Runtime sees only this Docker-bridge listener and an expiring,
//! single-connection capability. A host-side service token and browser profile
//! never enter the Runtime. Public and stdio profiles carry no upstream credential.
//! rmcp handles both MCP protocol directions;
//! this layer owns only Restless authority and the exact tool allowlist.

use std::net::{IpAddr, SocketAddr};
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    service::{RequestContext, RunningService},
    transport::{
        streamable_http_client::StreamableHttpClientTransportConfig,
        streamable_http_server::{
            session::never::NeverSessionManager, StreamableHttpServerConfig, StreamableHttpService,
        },
        StreamableHttpClientTransport,
    },
    RoleClient, RoleServer, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;
use tower::ServiceExt as _;
use uuid::Uuid;

use crate::{capability::McpGrant, connected_tool::{self, LocalMcpServer, ReviewedHttpReadProfile}, Daemon};

const PROBE_TIMEOUT: Duration = Duration::from_secs(12);
const CALL_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_RESULT_BYTES: usize = 1024 * 1024;
const MAX_PUBLIC_RESULT_BYTES: usize = 128 * 1024;
const PUBLIC_CALL_TIMEOUT: Duration = Duration::from_secs(30);
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
    DeepWikiStructure {
        #[serde(rename = "repoName")]
        repo_name: String,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeepWikiStructureArgs {
    #[serde(rename = "repoName")]
    repo_name: String,
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

/// CH statuses are untrusted provider data. Keep only reviewed outcome codes in
/// durable receipts and connection health; never persist an arbitrary string
/// from a listing or an upstream error object as a status.
fn safe_clapping_hands_status(result: &CallToolResult) -> &'static str {
    match classify_read_status(result).as_str() {
        "complete" => "complete",
        "incomplete" => "incomplete",
        "unavailable" => "unavailable",
        "blocked" => "blocked",
        "auth-required" => "auth-required",
        "partial" => "partial",
        "unverified" => "unverified",
        "access-restricted" => "access-restricted",
        "owner-paused" => "owner-paused",
        "runtime-busy" => "runtime-busy",
        "saved-plan-changed" => "saved-plan-changed",
        "task-failed" => "task-failed",
        "tool_error" => "tool_error",
        _ if result.is_error == Some(true) => "unrecognized-tool-error",
        _ => "unrecognized",
    }
}

fn filesystem_read_complete(result: &CallToolResult) -> bool {
    result.is_error != Some(true)
        && result.structured_content.as_ref()
            .and_then(|body| body.get("content"))
            .and_then(|value| value.as_str())
            .is_some()
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
        ReadRequest::DeepWikiStructure { repo_name } => {
            connected_tool::validate_public_repository(&repo_name)?;
            ("read_wiki_structure", serde_json::json!({"repoName":repo_name}))
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
fn validated_clapping_hands_params(params: CallToolRequestParams) -> Result<CallToolRequestParams> {
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

fn validated_profile_params(
    profile: &ReviewedHttpReadProfile,
    params: CallToolRequestParams,
) -> Result<CallToolRequestParams> {
    match profile {
        ReviewedHttpReadProfile::ClappingHands => validated_clapping_hands_params(params),
        ReviewedHttpReadProfile::DeepWikiStructure { repository } => {
            if params.name.as_ref() != "read_wiki_structure"
                || params.meta.is_some()
                || params.input_responses.is_some()
                || params.request_state.is_some()
            {
                bail!("MCP tool is outside the reviewed public read profile");
            }
            let args: DeepWikiStructureArgs = serde_json::from_value(serde_json::Value::Object(
                params.arguments.context("DeepWiki read arguments are required")?,
            ))?;
            if args.repo_name != *repository {
                bail!("DeepWiki read is outside the owner-selected repository");
            }
            Ok(CallToolRequestParams::new("read_wiki_structure").with_arguments(
                serde_json::json!({"repoName":repository}).as_object()
                    .context("DeepWiki arguments are not an object")?.clone(),
            ))
        }
    }
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

fn transport(endpoint: &str, token: Option<String>, max_event_bytes: usize) -> StreamableHttpClientTransport<reqwest_mcp::Client> {
    let client = reqwest_mcp::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .no_proxy()
        .redirect(reqwest_mcp::redirect::Policy::none())
        .build()
        .expect("fixed local MCP HTTP client configuration");
    let mut config = StreamableHttpClientTransportConfig::with_uri(endpoint.to_string())
        .max_concurrent_requests(1)
        .max_sse_event_size(max_event_bytes);
    if let Some(token) = token {
        config = config.auth_header(token);
    }
    StreamableHttpClientTransport::with_client(client, config)
}

fn selected_contract(tools: Vec<Tool>, allowed: &[String], server_version: String) -> Result<(Vec<Tool>, McpProbe)> {
    let mut selected = tools.into_iter()
        .filter(|tool| allowed.iter().any(|name| name == tool.name.as_ref()))
        .collect::<Vec<_>>();
    selected.sort_by(|a, b| a.name.cmp(&b.name));
    let observed_names = selected.iter().map(|tool| tool.name.to_string()).collect::<Vec<_>>();
    let mut expected_names = allowed.to_vec();
    expected_names.sort();
    if observed_names != expected_names {
        bail!("MCP did not expose every permitted tool exactly once");
    }
    let encoded = serde_json::to_vec(&selected).context("encode observed MCP contract")?;
    let probe = McpProbe {
        names: observed_names,
        digest: format!("{:x}", Sha256::digest(&encoded)),
        server_version,
    };
    Ok((selected, probe))
}

fn require_deepwiki_structure_schema(tool: &Tool) -> Result<()> {
    let schema = &tool.input_schema;
    let properties = schema.get("properties").and_then(|value| value.as_object())
        .context("DeepWiki tool has no input properties")?;
    let required = schema.get("required").and_then(|value| value.as_array())
        .context("DeepWiki tool has no required input list")?;
    if tool.name.as_ref() != "read_wiki_structure"
        || properties.len() != 1
        || properties.get("repoName").and_then(|value| value.get("type"))
            .and_then(|value| value.as_str()) != Some("string")
        || required.len() != 1
        || required[0].as_str() != Some("repoName")
    {
        bail!("DeepWiki read_wiki_structure input schema changed");
    }
    Ok(())
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
    let client = tokio::time::timeout(PROBE_TIMEOUT, ().serve(transport(endpoint, Some(token), MAX_RESULT_BYTES)))
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
    Ok(selected_contract(tools, allowed, server_version)?.1)
}

pub(crate) async fn probe_public_upstream(
    endpoint: &str,
    allowed: &[String],
    profile: &ReviewedHttpReadProfile,
) -> Result<McpProbe> {
    let ReviewedHttpReadProfile::DeepWikiStructure { repository } = profile else {
        bail!("public MCP probe requires a reviewed public profile");
    };
    let client = tokio::time::timeout(PROBE_TIMEOUT, ().serve(transport(endpoint, None, MAX_PUBLIC_RESULT_BYTES)))
        .await.context("public MCP handshake timed out")?
        .context("public MCP handshake failed")?;
    let result = async {
        let server_version = client.peer_info().as_ref()
            .and_then(|info| info.server_info.as_ref().map(|server| server.version.clone()))
            .context("public MCP did not provide a server version")?;
        let tools = tokio::time::timeout(PROBE_TIMEOUT, client.list_all_tools())
            .await.context("public MCP discovery timed out")?
            .context("public MCP discovery failed")?;
        let (selected, probe) = selected_contract(tools, allowed, server_version)?;
        require_deepwiki_structure_schema(&selected[0])?;
        let params = CallToolRequestParams::new("read_wiki_structure").with_arguments(
            serde_json::json!({"repoName":repository}).as_object()
                .context("public probe arguments are not an object")?.clone(),
        );
        let observed = tokio::time::timeout(PUBLIC_CALL_TIMEOUT, client.call_tool(params))
            .await.context("public repository read timed out")?
            .context("public repository read failed")?;
        if observed.is_error == Some(true) || serde_json::to_vec(&observed)?.len() > MAX_PUBLIC_RESULT_BYTES {
            bail!("selected public repository did not return a bounded DeepWiki structure");
        }
        Ok::<_, anyhow::Error>(probe)
    }.await;
    let _ = tokio::time::timeout(Duration::from_secs(3), client.cancel()).await;
    result
}

#[derive(Clone)]
enum BrokerReadProfile {
    Http(ReviewedHttpReadProfile),
    Filesystem,
}

enum RequestValidationError {
    InvalidParams,
    Connection,
}

#[derive(Clone)]
struct ScopedMcp {
    pool: PgPool,
    company: String,
    connection: LocalMcpServer,
    profile: BrokerReadProfile,
    daemon: Arc<Daemon>,
    grant: McpGrant,
}

impl ScopedMcp {
    async fn connect_upstream(&self) -> Result<RunningService<RoleClient, ()>> {
        match &self.profile {
            BrokerReadProfile::Http(profile) => {
                let endpoint = self.connection.endpoint.as_deref().context("HTTP MCP endpoint missing")?;
                let token = match profile {
                    ReviewedHttpReadProfile::ClappingHands => Some(read_host_token(
                        self.connection.token_file.as_deref().context("host MCP token missing")?,
                    )?),
                    ReviewedHttpReadProfile::DeepWikiStructure { .. } => None,
                };
                let max_event_bytes = match profile {
                    ReviewedHttpReadProfile::ClappingHands => MAX_RESULT_BYTES,
                    ReviewedHttpReadProfile::DeepWikiStructure { .. } => MAX_PUBLIC_RESULT_BYTES,
                };
                tokio::time::timeout(PROBE_TIMEOUT, ().serve(transport(endpoint, token, max_event_bytes)))
                    .await.context("MCP handshake timed out")?
                    .context("MCP handshake failed")
            }
            BrokerReadProfile::Filesystem => {
                let read_root = connected_tool::require_reviewed_stdio_profile(&self.connection)?;
                let child = crate::stdio_mcp::transport(&self.connection.command, read_root)?;
                tokio::time::timeout(PROBE_TIMEOUT, ().serve(child))
                    .await.context("stdio MCP handshake timed out")?
                    .context("stdio MCP handshake failed")
            }
        }
    }

    async fn grant_still_current(&self) -> bool {
        let enabled: Result<Option<bool>, _> = sqlx::query_scalar(
            "SELECT enabled AND assigned_actor=$3 AND assigned_work_id=$4 \
              AND endpoint IS NOT DISTINCT FROM $5 AND read_profile IS NOT DISTINCT FROM $6 \
              AND target_repository IS NOT DISTINCT FROM $7 \
              AND tool_contract_digest IS NOT DISTINCT FROM $8 \
              AND server_version IS NOT DISTINCT FROM $9 \
              AND transport=$10 AND allowed_tools=$11 AND token_file IS NOT DISTINCT FROM $12 \
              AND policy_revision=$13 AND command=$14 AND args=$15 \
             FROM restless_authority.local_mcp_servers WHERE company=$1 AND name=$2",
        )
        .bind(&self.company).bind(&self.connection.name).bind(&self.grant.actor)
        .bind(self.grant.work_id).bind(&self.connection.endpoint)
        .bind(&self.connection.read_profile).bind(&self.connection.target_repository)
        .bind(&self.connection.tool_contract_digest).bind(&self.connection.server_version)
        .bind(&self.connection.transport).bind(serde_json::json!(self.connection.allowed_tools))
        .bind(&self.connection.token_file)
        .bind(self.connection.policy_revision)
        .bind(&self.connection.command).bind(serde_json::json!(self.connection.args))
        .fetch_optional(&self.pool).await;
        matches!(enabled, Ok(Some(true))) && running_attempt(&self.daemon, &self.grant).await
    }

    async fn observed_contract(&self, client: &RunningService<RoleClient, ()>) -> Result<Vec<Tool>> {
        let server_version = client.peer_info().as_ref()
            .and_then(|info| info.server_info.as_ref().map(|server| server.version.clone()))
            .context("MCP server version missing")?;
        let tools = tokio::time::timeout(PROBE_TIMEOUT, client.list_all_tools())
            .await.context("MCP discovery timeout")?
            .context("MCP discovery failed")?;
        let (selected, probe) = selected_contract(tools, &self.connection.allowed_tools, server_version)?;
        if self.connection.tool_contract_digest.as_deref() != Some(probe.digest.as_str())
            || self.connection.server_version.as_deref() != Some(probe.server_version.as_str()) {
            bail!("MCP tool contract changed; owner must review and reinstall");
        }
        if matches!(&self.profile, BrokerReadProfile::Http(ReviewedHttpReadProfile::DeepWikiStructure { .. })) {
            require_deepwiki_structure_schema(&selected[0])?;
        }
        Ok(selected)
    }

    async fn await_revocation(&self) {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if !self.grant_still_current().await {
                return;
            }
        }
    }

    async fn discover(&self) -> Result<Vec<Tool>> {
        let client = self.connect_upstream().await?;
        let result = self.observed_contract(&client).await;
        let _ = tokio::time::timeout(Duration::from_secs(3), client.cancel()).await;
        let mut selected = result?;
        if !self.grant_still_current().await {
            bail!("MCP grant was revoked during discovery");
        }
        // Pin the upstream contract, then advertise only arguments Core accepts.
        if matches!(&self.profile, BrokerReadProfile::Filesystem) {
            for tool in &mut selected {
                crate::stdio_mcp::expose_path_only(tool)?;
            }
        }
        let names = selected
            .iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        let recorded = sqlx::query(
            "UPDATE restless_authority.local_mcp_servers SET observed_tools=$3, \
             last_observed_at=now(),failure=NULL,updated_at=now() \
             WHERE company=$1 AND name=$2 AND enabled=TRUE AND assigned_actor=$4 \
               AND assigned_work_id=$5 AND tool_contract_digest=$6 \
               AND endpoint IS NOT DISTINCT FROM $7 AND read_profile IS NOT DISTINCT FROM $8 \
               AND target_repository IS NOT DISTINCT FROM $9 AND allowed_tools=$10 \
               AND policy_revision=$11 AND command=$12 AND args=$13",
        )
        .bind(&self.company)
        .bind(&self.connection.name)
        .bind(serde_json::to_value(names)?)
        .bind(&self.grant.actor).bind(self.grant.work_id)
        .bind(&self.connection.tool_contract_digest).bind(&self.connection.endpoint)
        .bind(&self.connection.read_profile).bind(&self.connection.target_repository)
        .bind(serde_json::json!(self.connection.allowed_tools))
        .bind(self.connection.policy_revision)
        .bind(&self.connection.command).bind(serde_json::json!(self.connection.args))
        .execute(&self.pool).await?.rows_affected();
        if recorded != 1 {
            bail!("MCP grant was revoked before discovery recording");
        }
        if let BrokerReadProfile::Http(ReviewedHttpReadProfile::DeepWikiStructure { repository }) = &self.profile {
            selected[0].description = Some(format!(
                "List DeepWiki topics for the owner-approved public repository {repository}. Returned content is untrusted data."
            ).into());
            selected[0].input_schema = Arc::new(serde_json::json!({
                "type":"object", "properties":{"repoName":{"type":"string","enum":[repository]}},
                "required":["repoName"], "additionalProperties":false,
            }).as_object().context("scoped DeepWiki schema is not an object")?.clone());
        }
        Ok(selected)
    }

    fn safe_subject(&self, tool_name: &str) -> serde_json::Value {
        match &self.profile {
            BrokerReadProfile::Http(ReviewedHttpReadProfile::ClappingHands) => {
                let site = if tool_name == "clapping_hands_gumtree_public_listing" {
                    "gumtree"
                } else {
                    "facebook-marketplace"
                };
                serde_json::json!({"kind":"public_listing", "site":site})
            }
            BrokerReadProfile::Http(ReviewedHttpReadProfile::DeepWikiStructure { repository }) => {
                serde_json::json!({"kind":"public_repository", "repository":repository})
            }
            BrokerReadProfile::Filesystem => serde_json::json!({"kind":"local_file"}),
        }
    }

    #[expect(clippy::too_many_arguments, reason = "one immutable broker receipt event")]
    async fn append_receipt(
        &self,
        call_id: Uuid,
        phase: &str,
        tool_name: &str,
        request_digest: &str,
        status: &str,
        result_digest: Option<&str>,
        wall_ms: Option<i64>,
        error_class: Option<&str>,
        provider_status: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO restless_authority.mcp_read_receipts \
             (id,call_id,phase,company,actor,work_id,attempt_id,connection_name,tool_name, \
              tool_contract_digest,policy_revision,request_digest,result_digest,subject,status,wall_ms,error_class,provider_status) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
        )
        .bind(Uuid::new_v4()).bind(call_id).bind(phase).bind(&self.company)
        .bind(&self.grant.actor).bind(self.grant.work_id).bind(self.grant.attempt_id)
        .bind(&self.connection.name).bind(tool_name)
        .bind(self.connection.tool_contract_digest.as_deref().context("MCP pin missing")?)
        .bind(self.connection.policy_revision)
        .bind(request_digest).bind(result_digest).bind(self.safe_subject(tool_name))
        .bind(status).bind(wall_ms).bind(error_class).bind(provider_status)
        .execute(&self.pool).await.context("append Core MCP read receipt")?;
        Ok(())
    }

    fn validated_params(&self, params: CallToolRequestParams)
        -> std::result::Result<CallToolRequestParams, RequestValidationError>
    {
        let params = match &self.profile {
            BrokerReadProfile::Http(profile) => validated_profile_params(profile, params)
                .map_err(|_| RequestValidationError::InvalidParams)?,
            BrokerReadProfile::Filesystem => {
                let read_root = connected_tool::require_reviewed_stdio_profile(&self.connection)
                    .map_err(|_| RequestValidationError::Connection)?;
                crate::stdio_mcp::validated_read_params(params, read_root)
                    .map_err(|_| RequestValidationError::InvalidParams)?
            }
        };
        if !self.connection.allowed_tools.iter().any(|name| name == params.name.as_ref()) {
            return Err(RequestValidationError::InvalidParams);
        }
        Ok(params)
    }

    async fn append_rejected_params_receipt(&self, tool_name: &str) -> Result<()> {
        // Rejected arguments may contain arbitrary private text. Deliberately
        // omit even their digest; this fixed marker says why no request hash
        // can identify the rejected payload.
        let request_digest = format!("{:x}", Sha256::digest(b"invalid-mcp-arguments-withheld"));
        self.append_receipt(Uuid::new_v4(), "terminal", tool_name, &request_digest,
            "not_invoked", None, Some(0), Some("invalid_params"), None).await
    }

    async fn invoke_validated(&self, params: CallToolRequestParams) -> Result<CallToolResult> {
        let tool_name = params.name.to_string();
        let request_digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&params)?));
        let call_id = Uuid::new_v4();
        let started = Instant::now();
        self.append_receipt(call_id, "started", &tool_name, &request_digest,
            "started", None, None, None, None).await?;
        let mut call_started = false;
        let observed = self.invoke_inner(params, &tool_name, &mut call_started).await;
        // A provider may have executed even if the grant disappears before
        // delivery. Suppress its result and record that uncertainty once.
        let revoked_after_result = observed.is_ok() && !self.grant_still_current().await;
        let outcome = if revoked_after_result {
            Err(anyhow::anyhow!("MCP grant was revoked before returning the read"))
        } else {
            observed
        };
        let (status, result_digest, error_class, provider_status) = match &outcome {
            Ok(result) => {
                let provider_status = match &self.profile {
                    BrokerReadProfile::Http(ReviewedHttpReadProfile::ClappingHands) =>
                        Some(safe_clapping_hands_status(result)),
                    _ => None,
                };
                let status = if result.is_error == Some(true) {
                    "tool_error"
                } else if matches!(&self.profile, BrokerReadProfile::Http(ReviewedHttpReadProfile::ClappingHands))
                    && provider_status == Some("complete")
                    || matches!(&self.profile, BrokerReadProfile::Filesystem)
                        && filesystem_read_complete(result) {
                    "complete"
                } else {
                    "response_observed_unverified"
                };
                (status, Some(format!("{:x}", Sha256::digest(serde_json::to_vec(result)?))), None, provider_status)
            }
            Err(_) if revoked_after_result => ("outcome_unknown", None, Some("grant_revoked_after_result"), None),
            Err(_) if call_started => ("outcome_unknown", None, Some("broker_call_failed"), None),
            Err(_) => ("not_invoked", None, Some("broker_pre_call_failed"), None),
        };
        self.append_receipt(call_id, "terminal", &tool_name, &request_digest,
            status, result_digest.as_deref(), Some(started.elapsed().as_millis() as i64),
            error_class, provider_status).await?;
        outcome
    }

    async fn invoke_inner(
        &self,
        params: CallToolRequestParams,
        tool_name: &str,
        call_started: &mut bool,
    ) -> Result<CallToolResult> {
        let client = self.connect_upstream().await?;
        let result = async {
            self.observed_contract(&client).await?;
            if !self.grant_still_current().await {
                bail!("MCP grant was revoked before invocation");
            }
            let timeout = match &self.profile {
                BrokerReadProfile::Http(ReviewedHttpReadProfile::ClappingHands) | BrokerReadProfile::Filesystem => CALL_TIMEOUT,
                BrokerReadProfile::Http(ReviewedHttpReadProfile::DeepWikiStructure { .. }) => PUBLIC_CALL_TIMEOUT,
            };
            *call_started = true;
            let result = tokio::select! {
                result = tokio::time::timeout(timeout, client.call_tool(params)) => Some(result),
                _ = self.await_revocation() => None,
            };
            let result = result.context("MCP grant revoked during call")?
                .context("MCP call timed out")??;
            let max_result = match &self.profile {
                BrokerReadProfile::Http(ReviewedHttpReadProfile::ClappingHands) | BrokerReadProfile::Filesystem => MAX_RESULT_BYTES,
                BrokerReadProfile::Http(ReviewedHttpReadProfile::DeepWikiStructure { .. }) => MAX_PUBLIC_RESULT_BYTES,
            };
            if serde_json::to_vec(&result)?.len() > max_result {
                bail!("MCP result exceeds its bound");
            }
            if !self.grant_still_current().await {
                bail!("MCP grant was revoked before the read result was returned");
            }
            let read_status: String = match &self.profile {
                BrokerReadProfile::Http(ReviewedHttpReadProfile::ClappingHands) => safe_clapping_hands_status(&result).into(),
                BrokerReadProfile::Http(ReviewedHttpReadProfile::DeepWikiStructure { .. }) if result.is_error == Some(true) => "tool_error".into(),
                BrokerReadProfile::Http(ReviewedHttpReadProfile::DeepWikiStructure { .. }) => "response_observed_unverified".into(),
                BrokerReadProfile::Filesystem if result.is_error == Some(true) => "tool_error".into(),
                BrokerReadProfile::Filesystem if filesystem_read_complete(&result) => "complete".into(),
                BrokerReadProfile::Filesystem => "response_observed_unverified".into(),
            };
            let observed_success = result.is_error != Some(true)
                && (matches!(&self.profile, BrokerReadProfile::Http(ReviewedHttpReadProfile::DeepWikiStructure { .. }))
                    || read_status == "complete");
            let read_site = match &self.profile {
                BrokerReadProfile::Http(ReviewedHttpReadProfile::ClappingHands) if tool_name == "clapping_hands_gumtree_public_listing" => "gumtree",
                BrokerReadProfile::Http(ReviewedHttpReadProfile::ClappingHands) => "facebook-marketplace",
                BrokerReadProfile::Http(ReviewedHttpReadProfile::DeepWikiStructure { .. }) => "deepwiki",
                BrokerReadProfile::Filesystem => "local-filesystem",
            };
            let recorded = sqlx::query(
                "UPDATE restless_authority.local_mcp_servers SET \
                 last_success_at=CASE WHEN $3 THEN now() ELSE last_success_at END, \
                 last_read_status=$4,failure=CASE WHEN $5 THEN $4 ELSE NULL END, \
                 last_read_site=$6,last_read_tool=$7,updated_at=now() \
                 WHERE company=$1 AND name=$2 AND enabled=TRUE AND assigned_actor=$8 \
                   AND assigned_work_id=$9 AND tool_contract_digest=$10 \
                   AND endpoint IS NOT DISTINCT FROM $11 AND read_profile IS NOT DISTINCT FROM $12 \
                   AND target_repository IS NOT DISTINCT FROM $13 AND allowed_tools=$14 \
                   AND policy_revision=$15 AND command=$16 AND args=$17",
            )
            .bind(&self.company).bind(&self.connection.name).bind(observed_success)
            .bind(&read_status).bind(result.is_error == Some(true)).bind(read_site)
            .bind(tool_name).bind(&self.grant.actor).bind(self.grant.work_id)
            .bind(&self.connection.tool_contract_digest)
            .bind(&self.connection.endpoint).bind(&self.connection.read_profile)
            .bind(&self.connection.target_repository)
            .bind(serde_json::json!(self.connection.allowed_tools))
            .bind(self.connection.policy_revision)
            .bind(&self.connection.command).bind(serde_json::json!(self.connection.args))
            .execute(&self.pool).await?.rows_affected();
            if recorded != 1 {
                bail!("MCP grant was revoked before result recording");
            }
            tracing::info!(company=%self.company, name=%self.connection.name, status=read_status,
                "Core MCP read observed");
            Ok(result)
        }.await;
        let _ = tokio::time::timeout(Duration::from_secs(3), client.cancel()).await;
        result
    }

    async fn record_failure(&self, class: &str) {
        if !self.grant_still_current().await {
            return;
        }
        let _ = sqlx::query(
            "UPDATE restless_authority.local_mcp_servers SET failure=$3,last_read_status='unavailable',updated_at=now() \
             WHERE company=$1 AND name=$2 AND enabled=TRUE AND assigned_actor=$4 \
               AND assigned_work_id=$5 AND tool_contract_digest=$6 \
               AND endpoint IS NOT DISTINCT FROM $7 AND read_profile IS NOT DISTINCT FROM $8 \
               AND target_repository IS NOT DISTINCT FROM $9 AND allowed_tools=$10 \
               AND policy_revision=$11 AND command=$12 AND args=$13",
        )
        .bind(&self.company)
        .bind(&self.connection.name)
        .bind(class)
        .bind(&self.grant.actor).bind(self.grant.work_id)
        .bind(&self.connection.tool_contract_digest).bind(&self.connection.endpoint)
        .bind(&self.connection.read_profile).bind(&self.connection.target_repository)
        .bind(serde_json::json!(self.connection.allowed_tools))
        .bind(self.connection.policy_revision)
        .bind(&self.connection.command).bind(serde_json::json!(self.connection.args))
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
            Err(_error) => {
                self.record_failure("discovery_failed").await;
                tracing::warn!(company=%self.company, name=%self.connection.name, "Core MCP discovery failed");
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
        let tool_name = request.name.to_string();
        if !self
            .connection
            .allowed_tools
            .iter()
            .any(|name| name == &tool_name)
        {
            return Err(ErrorData::invalid_params("MCP tool is not permitted", None));
        }
        let request = match self.validated_params(request) {
            Ok(request) => request,
            Err(RequestValidationError::InvalidParams) => {
                if self.append_rejected_params_receipt(&tool_name).await.is_err() {
                    return Err(ErrorData::internal_error("MCP read receipt unavailable", None));
                }
                return Err(ErrorData::invalid_params("MCP read arguments are invalid", None));
            }
            Err(RequestValidationError::Connection) => {
                self.record_failure("connection_profile_failed").await;
                return Ok(CallToolResult::error(vec![ContentBlock::text(
                    "Connected MCP read profile is unavailable; inspect the connection status.",
                )]).into());
            }
        };
        match self.invoke_validated(request).await {
            Ok(result) => Ok(result.into()),
            Err(_error) => {
                self.record_failure("call_failed").await;
                tracing::warn!(company=%self.company, name=%self.connection.name, "Core MCP call failed");
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
    let tool_name = params.name.to_string();
    let params = match scope.validated_params(params) {
        Ok(params) => params,
        Err(RequestValidationError::InvalidParams) => {
            if scope.connection.allowed_tools.iter().any(|name| name == &tool_name)
                && scope.append_rejected_params_receipt(&tool_name).await.is_err()
            {
                return StatusCode::BAD_GATEWAY.into_response();
            }
            return StatusCode::BAD_REQUEST.into_response();
        }
        Err(RequestValidationError::Connection) => {
            scope.record_failure("facade_connection_profile_failed").await;
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };
    match scope.invoke_validated(params).await {
        Ok(result) => Json(result).into_response(),
        Err(_error) => {
            scope.record_failure("facade_read_failed").await;
            tracing::warn!(company=%scope.company, name=%scope.connection.name,
                "Core MCP read facade failed");
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
        || !matches!(connection.transport.as_str(), "host_http" | "public_http" | "broker_stdio")
        || connection.assigned_actor != grant.actor
        || connection.assigned_work_id != Some(grant.work_id)
        || connection.policy_revision.to_string() != grant.pin
    {
        return Err(StatusCode::FORBIDDEN);
    }
    let profile = if connection.transport == "broker_stdio" {
        connected_tool::require_reviewed_stdio_profile(&connection)
            .map_err(|_| StatusCode::FORBIDDEN)?;
        BrokerReadProfile::Filesystem
    } else {
        BrokerReadProfile::Http(
            connected_tool::reviewed_http_read_profile(&connection)
                .map_err(|_| StatusCode::FORBIDDEN)?,
        )
    };
    if !running_attempt(&daemon, &grant).await {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(ScopedMcp {
        pool: daemon.authority.pool().clone(),
        company,
        connection,
        profile,
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
