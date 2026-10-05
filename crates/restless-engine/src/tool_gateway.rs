//! One MCP server per actor session for every connected tool.
//!
//! The Runtime reaches this listener on the Docker bridge with an expiring
//! tool-session capability. It sees the tools granted to its actor, namespaced
//! `connection__tool`, and never the upstream credential or session: this
//! host-side handler holds both. `reads` pass straight through with a read
//! receipt. `acts` and `reserved` calls go through the effect runner, which
//! applies first-contact approval, per-call owner approval for `reserved`,
//! one durable intent per execution and a receipt.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::{ConnectInfo, DefaultBodyLimit, Path as AxumPath, State},
    http::{header::AUTHORIZATION, HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse as _, Response},
    routing::any,
    Router,
};
use rmcp::{
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData,
        Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities,
        ServerConfig, Tool, ToolAnnotations,
    },
    service::{RequestContext, ServiceError},
    transport::streamable_http_server::{
        session::never::NeverSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    RoleServer, ServerHandler,
};
use sha2::{Digest as _, Sha256};
use tower::ServiceExt as _;
use uuid::Uuid;

use crate::capability::ToolGrant;
use crate::connections::{self, ToolClass, UsableTool, TOOL_SEPARATOR};
use crate::Daemon;

const CALL_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_RESULT_BYTES: usize = 2 * 1024 * 1024;
const RECONCILE_TOOL: &str = "restless_reconcile_effect";
const GOVERNANCE_ARGUMENT: &str = "_restless";

pub fn router(daemon: Arc<Daemon>) -> Router {
    Router::new()
        .route("/tools/{company}", any(handle))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(daemon)
}

/// The URL a Runtime process uses to reach this gateway.
pub fn runtime_url(company: &str) -> String {
    let port = crate::port_with_offset(crate::model_gateway::RUNTIME_RELAY_PORT)
        .unwrap_or(crate::model_gateway::RUNTIME_RELAY_PORT);
    format!("http://host.docker.internal:{port}/tools/{company}")
}

async fn handle(
    State(daemon): State<Arc<Daemon>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    request: Request<Body>,
) -> Response {
    if request.method() != Method::POST || headers.contains_key("origin") {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    if !crate::mcp_gateway::local_runtime_peer(peer.ip()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(token) = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(grant) = daemon.capabilities.verify_tool_session(token) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if grant.company != company {
        return StatusCode::FORBIDDEN.into_response();
    }
    if !session_is_live(&daemon, &grant).await {
        return StatusCode::FORBIDDEN.into_response();
    }
    let scope = ToolScope { daemon, grant };
    let mut config = StreamableHttpServerConfig::default();
    config.legacy_session_mode = false;
    config.json_response = true;
    let port = crate::port_with_offset(crate::model_gateway::RUNTIME_RELAY_PORT)
        .unwrap_or(crate::model_gateway::RUNTIME_RELAY_PORT);
    config.allowed_hosts = vec![
        format!("host.docker.internal:{port}"),
        format!("127.0.0.1:{port}"),
    ];
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

/// An Attempt-bound session lives only while its Attempt runs. A conversation
/// session (no Work) is bounded by the capability's expiry.
async fn session_is_live(daemon: &Daemon, grant: &ToolGrant) -> bool {
    let (Some(work_id), Some(attempt_id)) = (grant.work_id, grant.attempt_id) else {
        return true;
    };
    let Ok(org) = daemon.orgintel.get(&grant.company).await else {
        return false;
    };
    let Ok(attempts) = org.list_running_work_attempts().await else {
        return false;
    };
    attempts.iter().any(|attempt| {
        attempt.id == attempt_id
            && attempt.work_id == work_id
            && attempt.actor_id == grant.actor
            && attempt.interrupt_requested_at.is_none()
    })
}

#[derive(Clone)]
struct ToolScope {
    daemon: Arc<Daemon>,
    grant: ToolGrant,
}

impl ToolScope {
    async fn caller(&self) -> std::result::Result<ToolCaller<'_>, ErrorData> {
        let config = crate::runtime::CompanyConfig::load(&self.daemon.root, &self.grant.company)
            .map_err(|_| {
                ErrorData::internal_error("the company configuration is unavailable", None)
            })?;
        Ok(ToolCaller {
            authority: &self.daemon.authority,
            root: &self.daemon.root,
            org: self.daemon.orgintel.get(&self.grant.company).await.ok(),
            config,
            grant: &self.grant,
        })
    }
}

/// Everything a granted tool call needs, without the HTTP transport.
pub struct ToolCaller<'a> {
    pub authority: &'a crate::authority::AuthorityStore,
    pub root: &'a std::path::Path,
    pub org: Option<restless_orgintel::OrgIntel>,
    pub config: crate::runtime::CompanyConfig,
    pub grant: &'a ToolGrant,
}

fn text_error(message: impl Into<String>) -> CallToolResponse {
    CallToolResult::error(vec![ContentBlock::text(message.into())]).into()
}

fn canonical_digest(value: &serde_json::Value) -> String {
    let canonical = crate::effect::canonical_json(value);
    format!("{:x}", Sha256::digest(canonical.to_string().as_bytes()))
}

impl ToolCaller<'_> {
    fn company(&self) -> &str {
        &self.grant.company
    }

    async fn usable(&self) -> anyhow::Result<Vec<UsableTool>> {
        connections::usable_tools(self.authority.pool(), self.company(), &self.grant.actor).await
    }

    /// Refresh live contracts first, so a tool changed upstream disappears
    /// from the list instead of failing later at call time.
    async fn refreshed_usable(&self) -> anyhow::Result<Vec<UsableTool>> {
        let tools = self.usable().await?;
        let mut seen = std::collections::HashSet::new();
        for tool in &tools {
            if seen.insert(tool.connection.name.clone()) {
                if let Err(error) = connections::refresh_tools(
                    self.authority.pool(),
                    self.root,
                    self.company(),
                    &tool.connection,
                )
                .await
                {
                    tracing::warn!(company = %self.company(), connection = %tool.connection.name,
                        "tool contract refresh failed: {}", connections::classify_failure(&error));
                }
            }
        }
        self.usable().await
    }

    async fn call_read(
        &self,
        tool: &UsableTool,
        arguments: Option<JsonObject>,
    ) -> CallToolResponse {
        let started = Instant::now();
        let request_digest = canonical_digest(&serde_json::Value::Object(
            arguments.clone().unwrap_or_default(),
        ));
        let result = async {
            let client = connections::upstream(self.root, self.company(), &tool.connection).await?;
            let mut params = CallToolRequestParams::new(tool.grant.tool.clone());
            if let Some(arguments) = arguments {
                params = params.with_arguments(arguments);
            }
            match tokio::time::timeout(CALL_TIMEOUT, client.call_tool(params)).await {
                Ok(Ok(result)) => Ok(result),
                Ok(Err(ServiceError::McpError(error))) => {
                    Err(ReadFailure::Refused(error.message.to_string()))
                }
                Ok(Err(_)) => Err(ReadFailure::Lost),
                Err(_) => Err(ReadFailure::Lost),
            }
            .map_err(anyhow::Error::new)
        }
        .await;
        let wall_ms = started.elapsed().as_millis() as i64;
        let (status, result_digest, error_class, response) = match result {
            Ok(result) => {
                let encoded = serde_json::to_vec(&result).unwrap_or_default();
                if encoded.len() > MAX_RESULT_BYTES {
                    ("tool_error", None, Some("result_too_large"),
                        text_error("The tool result exceeded the gateway's size limit; ask for less."))
                } else {
                    let status = if result.is_error == Some(true) { "tool_error" } else { "complete" };
                    (status, Some(format!("{:x}", Sha256::digest(&encoded))), None, result.into())
                }
            }
            Err(error) => match error.downcast_ref::<ReadFailure>() {
                Some(ReadFailure::Refused(message)) => ("tool_error", None, Some("refused"),
                    text_error(format!("The server refused the call: {message}"))),
                _ => ("outcome_unknown", None, Some("call_failed"),
                    text_error("The read failed or timed out. Reads have no side effects; it is safe to try again.")),
            },
        };
        let receipt = connections::record_read_receipt(
            self.authority.pool(),
            self.company(),
            &self.grant.actor,
            self.grant.work_id,
            self.grant.attempt_id,
            &tool.connection.name,
            &tool.grant.tool,
            &tool.observed.digest,
            &request_digest,
            result_digest.as_deref(),
            status,
            error_class,
            wall_ms,
        )
        .await;
        match receipt {
            Ok(id) => with_read_receipt(response, id),
            Err(_) => {
                text_error("The read receipt could not be recorded, so the result is withheld.")
            }
        }
    }

    async fn call_effect(
        &self,
        tool: &UsableTool,
        arguments: Option<JsonObject>,
    ) -> CallToolResponse {
        if tool.connection.frozen {
            return text_error(format!(
                "{} is frozen by the owner: acting tools are refused until it is unfrozen. Reads still work.",
                tool.connection.name
            ));
        }
        let mut arguments = arguments.unwrap_or_default();
        let governance = arguments.remove(GOVERNANCE_ARGUMENT);
        let purpose = governance
            .as_ref()
            .and_then(|value| value.get("purpose"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        if purpose.is_empty() {
            return text_error(format!(
                "{} acts outside the company. Add `{GOVERNANCE_ARGUMENT}: {{\"purpose\": \"…\"}}` saying why, in one sentence.",
                tool.gateway_name()
            ));
        }
        let arguments = serde_json::Value::Object(arguments);
        let key = governance
            .as_ref()
            .and_then(|value| value.get("key"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| self.default_key(tool, &arguments));
        let parties = declared_parties(&arguments, &tool.grant.party_args);
        // Open the session before the intent: a failure to connect is a known
        // "not sent", never an unknown outcome.
        let client = match connections::upstream(self.root, self.company(), &tool.connection).await
        {
            Ok(client) => client,
            Err(error) => {
                return text_error(format!(
                    "{} is unreachable ({}); nothing was sent.",
                    tool.connection.name,
                    connections::classify_failure(&error)
                ))
            }
        };
        let tool_name = tool.grant.tool.clone();
        let call = crate::effect::ToolCall {
            connection: &tool.connection.name,
            tool: &tool.grant.tool,
            reserved: tool.grant.class == ToolClass::Reserved,
            contract_digest: &tool.observed.digest,
            arguments,
            parties,
        };
        let live = std::sync::Mutex::new(None::<CallToolResult>);
        let receipt = crate::effect::request_tool_effect(
            &self.config,
            self.authority,
            self.org.as_ref(),
            call,
            &purpose,
            &key,
            &self.grant.actor,
            |arguments| async {
                let mut params = CallToolRequestParams::new(tool_name);
                if let serde_json::Value::Object(arguments) = arguments {
                    params = params.with_arguments(arguments);
                }
                match tokio::time::timeout(CALL_TIMEOUT, client.call_tool(params)).await {
                    Ok(Ok(result)) => {
                        let is_error = result.is_error == Some(true);
                        let value = serde_json::to_value(&result)?;
                        *live.lock().expect("tool result lock") = Some(result);
                        Ok(crate::effect::ToolResult { is_error, result: value })
                    }
                    // A JSON-RPC error is the server's answer: it did not run.
                    Ok(Err(ServiceError::McpError(error))) => Ok(crate::effect::ToolResult {
                        is_error: true,
                        result: serde_json::json!({"error": {"code": error.code.0, "message": error.message}}),
                    }),
                    Ok(Err(error)) => Err(anyhow::anyhow!("tool call lost: {error}")),
                    Err(_) => Err(anyhow::anyhow!("tool call timed out")),
                }
            },
        )
        .await;
        match receipt {
            Ok(receipt) => {
                let mut result = live
                    .into_inner()
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| replayed_result(&receipt));
                let note = format!(
                    "restless receipt {} (key {}{})",
                    receipt.id,
                    receipt.idempotency_key,
                    if receipt.replayed {
                        ", replayed: this call already ran"
                    } else {
                        ""
                    }
                );
                result.content.push(ContentBlock::text(note));
                result.into()
            }
            Err(error) => {
                let message = format!("{error:#}");
                if message.contains("tool call lost") || message.contains("tool call timed out") {
                    text_error(format!(
                        "The call's outcome is unknown (key {key}). Do not retry it. Check the external state with a read tool on {}, then settle it with {RECONCILE_TOOL}.",
                        tool.connection.name
                    ))
                } else {
                    text_error(message)
                }
            }
        }
    }

    /// Without a caller key, the same arguments within one Attempt (or one
    /// conversation session) are the same intent.
    fn default_key(&self, tool: &UsableTool, arguments: &serde_json::Value) -> String {
        let scope = self
            .grant
            .attempt_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| self.grant.session.clone());
        let digest = canonical_digest(&serde_json::json!({
            "scope": scope,
            "connection": tool.connection.name,
            "tool": tool.grant.tool,
            "arguments": arguments,
        }));
        format!("tool-{}", &digest[..32])
    }

    async fn reconcile(&self, arguments: Option<JsonObject>) -> CallToolResponse {
        let arguments = arguments.unwrap_or_default();
        let Some(key) = arguments.get("key").and_then(serde_json::Value::as_str) else {
            return text_error("`key` is required");
        };
        let Some(executed) = arguments
            .get("executed")
            .and_then(serde_json::Value::as_bool)
        else {
            return text_error(
                "`executed` is required: true if the read shows the call took effect",
            );
        };
        let Some(read_receipt) = arguments
            .get("read_receipt")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
        else {
            return text_error("`read_receipt` must be the receipt id a read returned");
        };
        let external_ref = arguments
            .get("external_ref")
            .and_then(serde_json::Value::as_str);
        match crate::effect::settle_tool_effect(
            self.authority,
            self.company(),
            key,
            executed,
            read_receipt,
            external_ref,
            &self.grant.actor,
        )
        .await
        {
            Ok(receipt) => CallToolResult::success(vec![ContentBlock::text(format!(
                "Settled {key}: {}. Receipt {}.",
                if executed {
                    "it took effect; the key now replays this receipt"
                } else {
                    "it did not take effect; the same key may run again"
                },
                receipt.id
            ))])
            .into(),
            Err(error) => text_error(format!("{error:#}")),
        }
    }
}

#[derive(Debug)]
enum ReadFailure {
    Refused(String),
    Lost,
}

impl std::fmt::Display for ReadFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadFailure::Refused(message) => write!(f, "refused: {message}"),
            ReadFailure::Lost => write!(f, "lost"),
        }
    }
}

impl std::error::Error for ReadFailure {}

fn with_read_receipt(response: CallToolResponse, id: Uuid) -> CallToolResponse {
    match response {
        CallToolResponse::Complete(mut result) => {
            result
                .content
                .push(ContentBlock::text(format!("restless read receipt {id}")));
            CallToolResponse::Complete(result)
        }
        other => other,
    }
}

fn replayed_result(receipt: &crate::effect::Receipt) -> CallToolResult {
    receipt
        .outcome
        .get("result")
        .cloned()
        .and_then(|value| serde_json::from_value::<CallToolResult>(value).ok())
        .unwrap_or_else(|| {
            CallToolResult::success(vec![ContentBlock::text(receipt.outcome.to_string())])
        })
}

/// Parties are whatever the declared party arguments hold: a string, a list
/// of strings, or a comma-separated list. `Name <addr>` reduces to `addr`.
pub fn declared_parties(arguments: &serde_json::Value, party_args: &[String]) -> Vec<String> {
    let mut parties = Vec::new();
    let mut push = |raw: &str| {
        for part in raw.split([',', ';']) {
            let part = part.trim();
            let address = match (part.rfind('<'), part.rfind('>')) {
                (Some(start), Some(end)) if start < end => &part[start + 1..end],
                _ => part,
            };
            let address = address.trim().to_lowercase();
            if !address.is_empty() {
                parties.push(address);
            }
        }
    };
    for name in party_args {
        match arguments.get(name) {
            Some(serde_json::Value::String(value)) => push(value),
            Some(serde_json::Value::Array(values)) => {
                for value in values {
                    match value {
                        serde_json::Value::String(value) => push(value),
                        serde_json::Value::Object(object) => {
                            for field in ["email", "address", "id"] {
                                if let Some(value) =
                                    object.get(field).and_then(serde_json::Value::as_str)
                                {
                                    push(value);
                                    break;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    parties.sort();
    parties.dedup();
    parties
}

fn gateway_tool(tool: &UsableTool) -> Tool {
    let mut schema = match &tool.observed.input_schema {
        serde_json::Value::Object(schema) => schema.clone(),
        _ => JsonObject::new(),
    };
    let class = tool.grant.class;
    if class != ToolClass::Reads {
        let properties = schema
            .entry("properties")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(properties) = properties.as_object_mut() {
            properties.insert(
                GOVERNANCE_ARGUMENT.into(),
                serde_json::json!({
                    "type": "object",
                    "description": "Restless governance: why you are making this call, and optionally your own idempotency key. Not sent to the tool.",
                    "properties": {
                        "purpose": {"type": "string", "description": "One sentence: why this call is needed."},
                        "key": {"type": "string", "description": "Reuse to make a retry the same call; omit to derive one."}
                    },
                    "required": ["purpose"]
                }),
            );
        }
        let required = schema
            .entry("required")
            .or_insert_with(|| serde_json::json!([]));
        if let Some(required) = required.as_array_mut() {
            required.push(serde_json::json!(GOVERNANCE_ARGUMENT));
        }
    }
    let consequence = match class {
        ToolClass::Reads => "Reads freely.",
        ToolClass::Acts => "Acts outside the company: governed, with a receipt. A first contact with a new party waits for the owner.",
        ToolClass::Reserved => "Asks the owner first: every call waits for approval.",
    };
    let description = format!(
        "[{} · {}] {}",
        tool.connection.name,
        consequence,
        tool.observed.description.clone().unwrap_or_default()
    );
    let mut gateway = Tool::new(tool.gateway_name(), description, Arc::new(schema));
    gateway.title = tool.observed.title.clone();
    gateway.annotations = Some(
        ToolAnnotations::new()
            .read_only(class == ToolClass::Reads)
            .destructive(class == ToolClass::Reserved),
    );
    gateway
}

fn reconcile_tool() -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "key": {"type": "string", "description": "The idempotency key of the call whose outcome is unknown."},
            "executed": {"type": "boolean", "description": "True if your read shows the call took effect."},
            "read_receipt": {"type": "string", "description": "The read receipt id returned by the read you used as evidence."},
            "external_ref": {"type": "string", "description": "The external id you found, such as a message id."}
        },
        "required": ["key", "executed", "read_receipt"]
    });
    let serde_json::Value::Object(schema) = schema else {
        unreachable!()
    };
    Tool::new(
        RECONCILE_TOOL,
        "Settle a tool call whose outcome is unknown, using a read made after it as evidence. Never retry an unknown call before settling it.",
        Arc::new(schema),
    )
}

impl ServerHandler for ToolScope {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("restless-tools", "1"))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, ErrorData> {
        let listed = self.caller().await?.list().await?;
        Ok(ListToolsResult::with_all_items(listed))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResponse, ErrorData> {
        self.caller()
            .await?
            .call(&request.name, request.arguments)
            .await
    }
}

impl ToolCaller<'_> {
    pub async fn list(&self) -> std::result::Result<Vec<Tool>, ErrorData> {
        let tools = self
            .refreshed_usable()
            .await
            .map_err(|_| ErrorData::internal_error("connected tools are unavailable", None))?;
        let mut listed = tools.iter().map(gateway_tool).collect::<Vec<_>>();
        if tools
            .iter()
            .any(|tool| tool.grant.class != ToolClass::Reads)
        {
            listed.push(reconcile_tool());
        }
        Ok(listed)
    }

    pub async fn call(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
    ) -> std::result::Result<CallToolResponse, ErrorData> {
        let name = name.to_string();
        if name == RECONCILE_TOOL {
            return Ok(self.reconcile(arguments).await);
        }
        let Some((connection, _)) = name.split_once(TOOL_SEPARATOR) else {
            return Err(ErrorData::invalid_params("unknown tool", None));
        };
        let tools = self
            .usable()
            .await
            .map_err(|_| ErrorData::internal_error("connected tools are unavailable", None))?;
        let Some(tool) = tools
            .into_iter()
            .find(|tool| tool.connection.name == connection && tool.gateway_name() == name)
        else {
            return Err(ErrorData::invalid_params(
                "this tool is not granted to you, or its definition changed and it awaits the owner's grant",
                None,
            ));
        };
        Ok(match tool.grant.class {
            ToolClass::Reads => self.call_read(&tool, arguments).await,
            ToolClass::Acts | ToolClass::Reserved => self.call_effect(&tool, arguments).await,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[test]
    fn parties_come_from_declared_arguments_only() {
        let arguments = serde_json::json!({
            "to": "Ann <ANN@example.com>, bob@example.com",
            "cc": ["carol@example.com"],
            "attendees": [{"email": "dan@example.com"}],
            "body": "mail eve@example.com"
        });
        let parties = declared_parties(&arguments, &["to".into(), "cc".into(), "attendees".into()]);
        assert_eq!(
            parties,
            vec![
                "ann@example.com",
                "bob@example.com",
                "carol@example.com",
                "dan@example.com"
            ]
        );
    }

    /// An in-process MCP server with one read, one acting and one destructive
    /// tool. It counts real executions, so a test can prove a call never ran.
    #[derive(Clone, Default)]
    struct Fake {
        sends: Arc<AtomicUsize>,
        deletes: Arc<AtomicUsize>,
        send_v2: Arc<AtomicBool>,
    }

    fn tool(value: serde_json::Value) -> Tool {
        serde_json::from_value(value).unwrap()
    }

    impl ServerHandler for Fake {
        fn get_info(&self) -> ServerConfig {
            ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
                .with_server_info(Implementation::new("fake-mail", "1"))
        }

        async fn list_tools(
            &self,
            _request: Option<PaginatedRequestParams>,
            _context: RequestContext<RoleServer>,
        ) -> std::result::Result<ListToolsResult, ErrorData> {
            let mut send = serde_json::json!({
                "name": "send", "description": "Send mail",
                "inputSchema": {"type": "object", "properties": {"to": {"type": "string"}, "body": {"type": "string"}}}
            });
            if self.send_v2.load(Ordering::SeqCst) {
                send["inputSchema"]["properties"]["bcc"] = serde_json::json!({"type": "string"});
            }
            Ok(ListToolsResult::with_all_items(vec![
                tool(serde_json::json!({
                    "name": "search", "description": "Search mail",
                    "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}}},
                    "annotations": {"readOnlyHint": true}
                })),
                tool(send),
                tool(serde_json::json!({
                    "name": "delete", "description": "Delete mail",
                    "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}}},
                    "annotations": {"readOnlyHint": false, "destructiveHint": true}
                })),
            ]))
        }

        async fn call_tool(
            &self,
            request: CallToolRequestParams,
            _context: RequestContext<RoleServer>,
        ) -> std::result::Result<CallToolResponse, ErrorData> {
            let arguments = request.arguments.unwrap_or_default();
            assert!(
                !arguments.contains_key(GOVERNANCE_ARGUMENT),
                "governance must not reach the tool"
            );
            let text = match request.name.as_ref() {
                "search" => "1 message".to_string(),
                "send" => format!("sent-{}", self.sends.fetch_add(1, Ordering::SeqCst) + 1),
                "delete" => format!(
                    "deleted-{}",
                    self.deletes.fetch_add(1, Ordering::SeqCst) + 1
                ),
                _ => return Err(ErrorData::invalid_params("unknown tool", None)),
            };
            Ok(CallToolResult::success(vec![ContentBlock::text(text)]).into())
        }
    }

    async fn serve_fake(fake: Fake) -> String {
        let service = StreamableHttpService::new(
            move || Ok(fake.clone()),
            Arc::new(NeverSessionManager::default()),
            {
                let mut config = StreamableHttpServerConfig::default();
                config.legacy_session_mode = false;
                config.json_response = true;
                config.allowed_hosts = Vec::new();
                config
            },
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new().route_service("/mcp", service);
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://127.0.0.1:{}/mcp", address.port())
    }

    fn text_of(response: &CallToolResponse) -> String {
        match response {
            CallToolResponse::Complete(result) => serde_json::to_string(&result.content).unwrap(),
            _ => String::new(),
        }
    }

    fn is_error(response: &CallToolResponse) -> bool {
        matches!(response, CallToolResponse::Complete(result) if result.is_error == Some(true))
    }

    fn args(value: serde_json::Value) -> Option<JsonObject> {
        match value {
            serde_json::Value::Object(map) => Some(map),
            _ => None,
        }
    }

    /// The kernel contract for connected tools, end to end through the
    /// gateway's caller against a real MCP server and a real Authority store.
    #[tokio::test]
    async fn granted_tools_are_governed_by_class() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            return;
        };
        let authority = crate::authority::AuthorityStore::connect(&url)
            .await
            .unwrap();
        let pool = authority.pool().clone();
        let company = format!("tools_{}_test", Uuid::new_v4().simple());
        let root = std::env::temp_dir().join(format!("restless-tools-{}", Uuid::new_v4().simple()));
        std::fs::create_dir_all(&root).unwrap();
        let config: crate::runtime::CompanyConfig =
            serde_json::from_value(serde_json::json!({"name": company})).unwrap();
        let fake = Fake::default();
        let endpoint = serve_fake(fake.clone()).await;

        connections::add(
            &pool,
            &authority,
            &company,
            serde_json::from_value(
                serde_json::json!({"kind": "remote", "name": "mail", "endpoint": endpoint}),
            )
            .unwrap(),
            "owner",
        )
        .await
        .unwrap();
        let connection = connections::probe(&pool, &root, &company, "mail")
            .await
            .unwrap();
        assert_eq!(connection.status, "working", "{:?}", connection.failure);
        assert_eq!(connection.tools.len(), 3);
        let decisions = connection
            .tools
            .iter()
            .map(|tool| {
                let proposed = connections::proposed_grant(tool);
                serde_json::from_value::<connections::ToolDecision>(serde_json::json!({
                    "tool": proposed.tool,
                    "class": proposed.class,
                    "party_args": if proposed.tool == "send" { vec!["to"] } else { vec![] },
                }))
                .unwrap()
            })
            .collect();
        connections::grant(
            &pool, &authority, &company, "mail", "exec", decisions, "owner", None,
        )
        .await
        .unwrap();

        let exec_grant = ToolGrant {
            company: company.clone(),
            actor: "exec".into(),
            session: "tools-test".into(),
            work_id: None,
            attempt_id: None,
        };
        let caller = ToolCaller {
            authority: &authority,
            root: &root,
            org: None,
            config: config.clone(),
            grant: &exec_grant,
        };

        // A grant is per actor: another actor sees nothing.
        let other_grant = ToolGrant {
            actor: "staff-a".into(),
            ..exec_grant.clone()
        };
        let other = ToolCaller {
            authority: &authority,
            root: &root,
            org: None,
            config: config.clone(),
            grant: &other_grant,
        };
        assert!(other.list().await.unwrap().is_empty());
        assert!(other
            .call("mail__search", args(serde_json::json!({"query": "x"})))
            .await
            .is_err());

        let listed = caller.list().await.unwrap();
        let names = listed
            .iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["mail__delete", "mail__search", "mail__send", RECONCILE_TOOL]
        );
        let send_schema = &listed
            .iter()
            .find(|tool| tool.name == "mail__send")
            .unwrap()
            .input_schema;
        assert!(send_schema["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(GOVERNANCE_ARGUMENT)));

        // Reads pass straight through with a read receipt.
        let read = caller
            .call(
                "mail__search",
                args(serde_json::json!({"query": "invoice"})),
            )
            .await
            .unwrap();
        assert!(text_of(&read).contains("1 message") && text_of(&read).contains("read receipt"));

        // An act needs a purpose; without one nothing runs.
        let refused = caller
            .call(
                "mail__send",
                args(serde_json::json!({"to": "ann@example.com", "body": "hi"})),
            )
            .await
            .unwrap();
        assert!(is_error(&refused) && text_of(&refused).contains("purpose"));
        assert_eq!(fake.sends.load(Ordering::SeqCst), 0);

        // First contact with a new party waits for the owner.
        let send = serde_json::json!({"to": "Ann <ann@example.com>", "body": "hi", "_restless": {"purpose": "Reply to Ann's quote request"}});
        let waiting = caller.call("mail__send", args(send.clone())).await.unwrap();
        assert!(is_error(&waiting), "{}", text_of(&waiting));
        assert_eq!(fake.sends.load(Ordering::SeqCst), 0);
        let asks = authority
            .records_of_kind(&company, "approval_required")
            .await
            .unwrap();
        assert_eq!(asks.last().unwrap().body["party"], "ann@example.com");

        // After the owner approves the party, it runs once and replays after.
        authority
            .emit(
                &company,
                "approval_granted",
                Some("owner"),
                serde_json::json!({"party": "ann@example.com", "principal": "owner"}),
            )
            .await
            .unwrap();
        let sent = caller.call("mail__send", args(send.clone())).await.unwrap();
        assert!(
            !is_error(&sent) && text_of(&sent).contains("sent-1"),
            "{}",
            text_of(&sent)
        );
        let replay = caller.call("mail__send", args(send.clone())).await.unwrap();
        assert!(
            text_of(&replay).contains("replayed"),
            "{}",
            text_of(&replay)
        );
        // The same arguments in another key order are the same call.
        let reordered = serde_json::json!({
            "_restless": {"purpose": "Reply to Ann's quote request"},
            "body": "hi",
            "to": "Ann <ann@example.com>"
        });
        let replay = caller.call("mail__send", args(reordered)).await.unwrap();
        assert!(text_of(&replay).contains("replayed"), "{}", text_of(&replay));
        assert_eq!(fake.sends.load(Ordering::SeqCst), 1);

        // A key cannot be reused for different arguments.
        let mut tampered = send.clone();
        tampered["body"] = serde_json::json!("changed");
        let key =
            serde_json::json!({"purpose": "Reply to Ann's quote request", "key": "reply-ann"});
        tampered["_restless"] = key.clone();
        let mut original = send.clone();
        original["_restless"] = key;
        assert!(!is_error(
            &caller.call("mail__send", args(original)).await.unwrap()
        ));
        let refused = caller.call("mail__send", args(tampered)).await.unwrap();
        assert!(
            text_of(&refused).contains("different command"),
            "{}",
            text_of(&refused)
        );
        assert_eq!(fake.sends.load(Ordering::SeqCst), 2);

        // Freeze refuses acts immediately; reads still work.
        connections::set_frozen(&pool, &authority, &company, "mail", true, "owner")
            .await
            .unwrap();
        let mut fresh = send.clone();
        fresh["body"] = serde_json::json!("while frozen");
        let frozen = caller
            .call("mail__send", args(fresh.clone()))
            .await
            .unwrap();
        assert!(text_of(&frozen).contains("frozen"));
        assert!(!is_error(
            &caller
                .call("mail__search", args(serde_json::json!({"query": "x"})))
                .await
                .unwrap()
        ));
        connections::set_frozen(&pool, &authority, &company, "mail", false, "owner")
            .await
            .unwrap();
        assert_eq!(fake.sends.load(Ordering::SeqCst), 2);

        // Reserved: each call waits for the owner, who answers that exact call.
        let delete = serde_json::json!({"id": "m1", "_restless": {"purpose": "Remove the duplicate", "key": "delete-m1"}});
        let waiting = caller
            .call("mail__delete", args(delete.clone()))
            .await
            .unwrap();
        assert!(
            text_of(&waiting).contains("asks the owner first"),
            "{}",
            text_of(&waiting)
        );
        assert_eq!(fake.deletes.load(Ordering::SeqCst), 0);
        let view = crate::attention::project(&config, &authority, None)
            .await
            .unwrap();
        assert!(view
            .items
            .iter()
            .any(|item| item.source.call_key.as_deref() == Some("delete-m1")));
        crate::effect::decide_tool_call(&authority, None, &company, "delete-m1", true, "owner")
            .await
            .unwrap();
        let view = crate::attention::project(&config, &authority, None)
            .await
            .unwrap();
        assert!(!view
            .items
            .iter()
            .any(|item| item.source.call_key.as_deref() == Some("delete-m1")));
        assert!(!is_error(
            &caller.call("mail__delete", args(delete)).await.unwrap()
        ));
        assert_eq!(fake.deletes.load(Ordering::SeqCst), 1);
        // Approval binds the exact call: other arguments under a new key ask again.
        let other_delete = serde_json::json!({"id": "m2", "_restless": {"purpose": "Remove another", "key": "delete-m2"}});
        assert!(text_of(
            &caller
                .call("mail__delete", args(other_delete))
                .await
                .unwrap()
        )
        .contains("asks the owner first"));
        assert_eq!(fake.deletes.load(Ordering::SeqCst), 1);

        // A lost result is unknown: retry is blocked until a later read settles it.
        let call = crate::effect::ToolCall {
            connection: "mail",
            tool: "send",
            reserved: false,
            contract_digest: "d",
            arguments: serde_json::json!({"to": "ann@example.com", "body": "lost"}),
            parties: vec!["ann@example.com".into()],
        };
        let lost = crate::effect::request_tool_effect(
            &config,
            &authority,
            None,
            call,
            "Follow up",
            "lost-1",
            "exec",
            |_| async { anyhow::bail!("tool call lost: connection reset") },
        )
        .await;
        assert!(lost.is_err());
        let again = || crate::effect::ToolCall {
            connection: "mail",
            tool: "send",
            reserved: false,
            contract_digest: "d",
            arguments: serde_json::json!({"to": "ann@example.com", "body": "lost"}),
            parties: vec!["ann@example.com".into()],
        };
        let blocked = crate::effect::request_tool_effect(
            &config,
            &authority,
            None,
            again(),
            "Follow up",
            "lost-1",
            "exec",
            |_| async { panic!("an unknown outcome must not run again") },
        )
        .await
        .unwrap_err();
        assert!(format!("{blocked:#}").contains("unknown outcome"));
        // Evidence must come after the intent; the earlier read does not count.
        let earlier = connections::recent_read_receipts(&pool, &company, "mail", 10)
            .await
            .unwrap();
        let stale = earlier.last().unwrap().id;
        assert!(crate::effect::settle_tool_effect(
            &authority, &company, "lost-1", false, stale, None, "exec"
        )
        .await
        .is_err());
        caller
            .call(
                "mail__search",
                args(serde_json::json!({"query": "in:sent lost"})),
            )
            .await
            .unwrap();
        let evidence = connections::recent_read_receipts(&pool, &company, "mail", 1)
            .await
            .unwrap()[0]
            .id;
        crate::effect::settle_tool_effect(
            &authority, &company, "lost-1", false, evidence, None, "exec",
        )
        .await
        .unwrap();
        let retried = crate::effect::request_tool_effect(
            &config,
            &authority,
            None,
            again(),
            "Follow up",
            "lost-1",
            "exec",
            |_| async {
                Ok(crate::effect::ToolResult {
                    is_error: false,
                    result: serde_json::json!({"content": []}),
                })
            },
        )
        .await
        .unwrap();
        assert_eq!(retried.execution_no, 2);

        // A tool whose contract changes upstream is withheld until re-granted.
        fake.send_v2.store(true, Ordering::SeqCst);
        let names = caller
            .list()
            .await
            .unwrap()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        assert!(
            !names.contains(&"mail__send".to_string())
                && names.contains(&"mail__search".to_string())
        );
        assert!(caller.call("mail__send", args(send)).await.is_err());

        // Disconnect revokes every grant.
        connections::disconnect(&pool, &authority, &root, &company, "mail", "owner")
            .await
            .unwrap();
        assert!(caller.list().await.unwrap().is_empty());

        connections::forget_upstream(&company, "mail").await;
        sqlx::query("DELETE FROM restless_authority.connections WHERE company=$1")
            .bind(&company)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM restless_authority.tool_read_receipts WHERE company=$1")
            .bind(&company)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM restless_authority.records WHERE company=$1")
            .bind(&company)
            .execute(&pool)
            .await
            .unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }
}
