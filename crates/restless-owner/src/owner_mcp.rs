//! Restless as an MCP server for one person. An outside client (Claude Code,
//! Claude Desktop, Codex) presents a personal token; every tool call rebuilds
//! that person's principal, re-checks their membership, and is served by the
//! same owner API handlers the cockpit uses, so a member can do here exactly
//! what they could do in the cockpit and nothing more.

use super::*;
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use restless_engine::member_access;
use rmcp::{
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorData,
        Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities,
        ServerConfig, Tool,
    },
    service::RequestContext,
    transport::streamable_http_server::{
        session::never::NeverSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    RoleServer, ServerHandler,
};
use tower::ServiceExt as _;

pub(super) const MCP_PATH: &str = "/mcp";
const MAX_API_RESPONSE: usize = 8 * 1024 * 1024;
const CONVERSATION_TAIL: usize = 30;

/// The owner API, with state, that tool calls are served by.
pub(super) type Api = Router;

pub(super) async fn serve(state: OwnerState, api: Api, request: Request) -> Response<Body> {
    // A browser always sends Origin; an MCP client does not. Refusing it keeps
    // a page from spending a token it somehow learned.
    if request.headers().contains_key(ORIGIN) {
        return api_error(
            StatusCode::FORBIDDEN,
            "mcp",
            "browsers may not call this endpoint",
        );
    }
    let Some(token) = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return unauthorized();
    };
    let access = match member_access::resolve(state.daemon.authority.pool(), token.trim()).await {
        Ok(Some(access)) => access,
        Ok(None) => return unauthorized(),
        Err(error) => {
            tracing::error!(%error, "could not resolve an MCP access token");
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "mcp",
                "access could not be checked",
            );
        }
    };
    let holder = Holder { state, api, access };
    if let Err(message) = holder.principal(&Method::GET, "/api/companies").await {
        return api_error(StatusCode::FORBIDDEN, "mcp", message);
    }
    let mut config = StreamableHttpServerConfig::default();
    config.legacy_session_mode = false;
    config.json_response = true;
    // The bearer is the authority and browsers are refused above; the host
    // differs between loopback, a LAN name and a tailnet name.
    let config = config.disable_allowed_hosts();
    let service = StreamableHttpService::new(
        move || Ok(holder.clone()),
        Arc::new(NeverSessionManager::default()),
        config,
    );
    match service.oneshot(request).await {
        Ok(response) => response.into_response(),
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}

fn unauthorized() -> Response<Body> {
    let mut response = api_error(
        StatusCode::UNAUTHORIZED,
        "mcp",
        "a valid personal access token is required",
    );
    response
        .headers_mut()
        .insert(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    response
}

/// The browser boundary's decision, made for a token instead of a cookie:
/// a local-owner token works only in local mode, a member token only while
/// that membership is current and only on the routes a member may use.
fn token_principal(
    network: bool,
    identity: Option<&VerifiedIdentity>,
    membership_current: bool,
    method: &Method,
    path: &str,
) -> Result<RequestPrincipal, String> {
    match (network, identity) {
        (false, None) => Ok(RequestPrincipal::local_owner()),
        (true, Some(identity)) => {
            if !membership_current {
                return Err("this membership changed; issue a new token".into());
            }
            let principal = RequestPrincipal::from_verified(identity)
                .ok_or("this token has no company identity")?;
            if let Some(refusal) = membership_boundary_violation(method, path, &principal) {
                return Err(refusal.message.into());
            }
            Ok(principal)
        }
        _ => Err("this token was issued under a different entry mode; issue a new one".into()),
    }
}

#[derive(Clone)]
struct Holder {
    state: OwnerState,
    api: Api,
    access: member_access::Access,
}

enum ApiBody {
    Empty,
    Json(serde_json::Value),
    Form(Vec<(&'static str, String)>),
}

impl Holder {
    /// The token holder's principal for this exact request, decided the way
    /// the browser boundary decides it.
    async fn principal(&self, method: &Method, path: &str) -> Result<RequestPrincipal, String> {
        let current = match (&self.state.entry, &self.access.identity) {
            (EntryMode::Network(_), Some(identity)) => {
                match network_session_is_current(&self.state, identity).await {
                    Ok(current) => current,
                    Err(error) => {
                        tracing::error!(%error, "could not validate an MCP token's membership");
                        return Err("company membership could not be validated".into());
                    }
                }
            }
            _ => false,
        };
        token_principal(
            self.state.entry.network().is_some(),
            self.access.identity.as_ref(),
            current,
            method,
            path,
        )
    }

    /// Serve one owner API request as the holder. `path` is under `/api`.
    async fn call_api(
        &self,
        method: Method,
        path: &str,
        body: ApiBody,
    ) -> Result<serde_json::Value, String> {
        let principal = self.principal(&method, &format!("/api{path}")).await?;
        let builder = axum::http::Request::builder().method(method).uri(path);
        let mut request = match body {
            ApiBody::Empty => builder.body(Body::empty()),
            ApiBody::Json(value) => builder
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(value.to_string())),
            ApiBody::Form(fields) => {
                let boundary = format!("restless-{}", Uuid::new_v4().simple());
                let mut form = String::new();
                for (name, value) in fields {
                    form.push_str(&format!(
                        "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
                    ));
                }
                form.push_str(&format!("--{boundary}--\r\n"));
                builder
                    .header(CONTENT_TYPE, format!("multipart/form-data; boundary={boundary}"))
                    .body(Body::from(form))
            }
        }
        .map_err(|error| error.to_string())?;
        request.extensions_mut().insert(principal);
        let response = self
            .api
            .clone()
            .oneshot(request)
            .await
            .map_err(|error| error.to_string())?;
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), MAX_API_RESPONSE)
            .await
            .map_err(|error| error.to_string())?;
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        if !status.is_success() {
            let message = value
                .pointer("/error/message")
                .or_else(|| value.get("message"))
                .and_then(|message| message.as_str())
                .unwrap_or(status.canonical_reason().unwrap_or("request failed"));
            return Err(message.to_string());
        }
        Ok(value)
    }

    async fn run(&self, name: &str, args: &JsonObject) -> Result<serde_json::Value, String> {
        let text = |key: &str| {
            args.get(key)
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        };
        let company = || {
            text("company")
                .filter(|company| is_segment(company))
                .ok_or_else(|| "give a company id from list_companies".to_string())
        };
        let actor = || {
            let actor = text("actor").unwrap_or("exec");
            if is_segment(actor) {
                Ok(actor)
            } else {
                Err("actor must be an actor id such as exec".to_string())
            }
        };
        match name {
            "list_companies" => {
                let value = self
                    .call_api(Method::GET, "/companies", ApiBody::Empty)
                    .await?;
                Ok(value)
            }
            "inbox" => {
                let view = self
                    .call_api(
                        Method::GET,
                        &format!("/companies/{}/attention", company()?),
                        ApiBody::Empty,
                    )
                    .await?;
                Ok(inbox_summary(&view))
            }
            "decide" => {
                let allow = args
                    .get("allow")
                    .and_then(|value| value.as_bool())
                    .ok_or("allow must be true or false")?;
                let body = match (text("call_key"), text("party")) {
                    (Some(call_key), _) => serde_json::json!({"call_key": call_key}),
                    (None, Some(party)) => serde_json::json!({"party": party}),
                    (None, None) => return Err("give the item's party or call_key".into()),
                };
                let verb = if allow { "grant" } else { "decline" };
                self.call_api(
                    Method::POST,
                    &format!("/companies/{}/approvals/{verb}", company()?),
                    ApiBody::Json(body),
                )
                .await
            }
            "message" => {
                let body = text("body").ok_or("body is required")?;
                let mut fields = vec![
                    ("client_command_id", format!("mcp-{}", Uuid::new_v4())),
                    ("body", body.to_string()),
                ];
                if let Some(attention_id) = text("attention_id") {
                    fields.push(("attention_id", attention_id.to_string()));
                }
                self.call_api(
                    Method::POST,
                    &format!("/companies/{}/actors/{}/conversation", company()?, actor()?),
                    ApiBody::Form(fields),
                )
                .await
            }
            "conversation" => {
                let view = self
                    .call_api(
                        Method::GET,
                        &format!("/companies/{}/actors/{}/conversation", company()?, actor()?),
                        ApiBody::Empty,
                    )
                    .await?;
                Ok(conversation_tail(&view))
            }
            _ => Err(format!("unknown tool {name}")),
        }
    }
}

/// Ids become path segments, so nothing that could change the path passes.
fn is_segment(value: &str) -> bool {
    value.len() <= 128
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && value != "."
        && value != ".."
}

/// The Inbox as a model needs it: what each item asks and how to answer.
fn inbox_summary(view: &serde_json::Value) -> serde_json::Value {
    let items = view
        .get("items")
        .and_then(|items| items.as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    serde_json::json!({
                        "id": item.get("id"),
                        "title": item.get("title"),
                        "category": item.get("category"),
                        "what_happened": item.get("what_happened"),
                        "recommendation": item.get("recommendation"),
                        "requested_action": item.get("requested_action"),
                        "if_no_action": item.get("if_no_action"),
                        "party": item.pointer("/source/party"),
                        "call_key": item.pointer("/source/call_key"),
                        "actions": item.get("actions"),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    serde_json::json!({ "items": items })
}

fn conversation_tail(view: &serde_json::Value) -> serde_json::Value {
    let messages = view
        .get("messages")
        .and_then(|messages| messages.as_array())
        .map(|messages| {
            let start = messages.len().saturating_sub(CONVERSATION_TAIL);
            messages[start..]
                .iter()
                .map(|message| {
                    serde_json::json!({
                        "from": message.get("from_actor"),
                        "body": message.get("body"),
                        "at": message.get("created_at"),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    serde_json::json!({ "actor": view.get("actor"), "messages": messages })
}

fn tool(name: &'static str, description: &'static str, schema: serde_json::Value) -> Tool {
    let serde_json::Value::Object(schema) = schema else {
        unreachable!()
    };
    Tool::new(name, description, Arc::new(schema))
}

fn tools() -> Vec<Tool> {
    let company =
        serde_json::json!({"type": "string", "description": "The company id from list_companies."});
    let actor =
        serde_json::json!({"type": "string", "description": "Who to talk to. Defaults to exec."});
    vec![
        tool(
            "list_companies",
            "List the companies you can reach.",
            serde_json::json!({"type": "object", "properties": {}}),
        ),
        tool(
            "inbox",
            "What is waiting on you in a company: approvals, decisions and human steps, each with Exec's recommendation.",
            serde_json::json!({"type": "object", "properties": {"company": company}, "required": ["company"]}),
        ),
        tool(
            "decide",
            "Allow or decline an Inbox approval. Pass the item's call_key for a held tool call, otherwise its party.",
            serde_json::json!({"type": "object", "properties": {
                "company": company,
                "party": {"type": "string"},
                "call_key": {"type": "string"},
                "allow": {"type": "boolean"}
            }, "required": ["company", "allow"]}),
        ),
        tool(
            "message",
            "Send a message to Exec (or another actor) as yourself. Pass attention_id to answer a specific Inbox item.",
            serde_json::json!({"type": "object", "properties": {
                "company": company,
                "actor": actor,
                "body": {"type": "string"},
                "attention_id": {"type": "string"}
            }, "required": ["company", "body"]}),
        ),
        tool(
            "conversation",
            "Read the latest messages in your conversation with Exec (or another actor).",
            serde_json::json!({"type": "object", "properties": {"company": company, "actor": actor}, "required": ["company"]}),
        ),
    ]
}

impl ServerHandler for Holder {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("restless", env!("CARGO_PKG_VERSION")))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResponse, ErrorData> {
        let args = request.arguments.unwrap_or_default();
        let result = match self.run(&request.name, &args).await {
            Ok(value) => CallToolResult::success(vec![ContentBlock::text(
                serde_json::to_string_pretty(&value).unwrap_or_default(),
            )]),
            Err(message) => CallToolResult::error(vec![ContentBlock::text(message)]),
        };
        Ok(CallToolResponse::Complete(result))
    }
}

/// Personal tokens live under `/api/mcp-access`; each person sees and revokes
/// only their own.
pub(super) const ACCESS_PATH: &str = "/api/mcp-access";

/// The identity a new token will speak for: the session's, or the local owner.
fn issuing_identity(
    state: &OwnerState,
    session_lease: Option<&SessionLease>,
) -> Result<Option<VerifiedIdentity>, Response<Body>> {
    match (&state.entry, session_lease) {
        (EntryMode::Local, _) => Ok(None),
        (EntryMode::Network(_), Some(lease)) => Ok(Some(lease.identity.clone())),
        (EntryMode::Network(_), None) => Err(api_error(
            StatusCode::UNAUTHORIZED,
            "mcp_access",
            "enter Restless before issuing a token",
        )),
    }
}

#[derive(Deserialize)]
pub(super) struct IssueInput {
    label: String,
}

pub(super) async fn list_access(
    State(state): State<OwnerState>,
    session_lease: Option<Extension<SessionLease>>,
) -> Response<Body> {
    let identity = match issuing_identity(&state, session_lease.as_deref()) {
        Ok(identity) => identity,
        Err(refusal) => return refusal,
    };
    let holder = member_access::holder(identity.as_ref());
    match member_access::list(state.daemon.authority.pool(), &holder).await {
        Ok(tokens) => Json(serde_json::json!({ "tokens": tokens })).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "mcp_access",
            format!("{error:#}"),
        ),
    }
}

pub(super) async fn issue_access(
    State(state): State<OwnerState>,
    session_lease: Option<Extension<SessionLease>>,
    Json(input): Json<IssueInput>,
) -> Response<Body> {
    let identity = match issuing_identity(&state, session_lease.as_deref()) {
        Ok(identity) => identity,
        Err(refusal) => return refusal,
    };
    match member_access::issue(
        state.daemon.authority.pool(),
        identity.as_ref(),
        &input.label,
    )
    .await
    {
        Ok((token, secret)) => {
            Json(serde_json::json!({ "token": token, "secret": secret })).into_response()
        }
        Err(error) => api_error(StatusCode::BAD_REQUEST, "mcp_access", format!("{error:#}")),
    }
}

pub(super) async fn revoke_access(
    State(state): State<OwnerState>,
    session_lease: Option<Extension<SessionLease>>,
    AxumPath(id): AxumPath<Uuid>,
) -> Response<Body> {
    let identity = match issuing_identity(&state, session_lease.as_deref()) {
        Ok(identity) => identity,
        Err(refusal) => return refusal,
    };
    let holder = member_access::holder(identity.as_ref());
    match member_access::revoke(state.daemon.authority.pool(), &holder, id).await {
        Ok(true) => Json(serde_json::json!({ "revoked": true })).into_response(),
        Ok(false) => api_error(
            StatusCode::NOT_FOUND,
            "mcp_access",
            "no such token of yours",
        ),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "mcp_access",
            format!("{error:#}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member() -> VerifiedIdentity {
        VerifiedIdentity {
            user: "user-ann".into(),
            issuer: None,
            owner: "acct".into(),
            scope: CompanyScope::Company {
                company: "acme".into(),
            },
            role: "member".into(),
            actor: Some("ann".into()),
            company_id: Some(Uuid::new_v4()),
            cell_id: Some(Uuid::new_v4()),
            membership_id: Some("m1".into()),
            membership_version: Some(1),
        }
    }

    #[test]
    fn a_member_token_reaches_only_what_the_member_may_do() {
        let ann = member();
        let talk = token_principal(
            true,
            Some(&ann),
            true,
            &Method::POST,
            "/api/companies/acme/actors/exec/conversation",
        )
        .expect("a member may talk to Exec");
        assert_eq!(talk.actor_id(), "ann");
        assert!(!talk.permits_company("other"));

        for (method, path) in [
            (Method::GET, "/api/companies/acme/attention"),
            (Method::POST, "/api/companies/acme/approvals/grant"),
        ] {
            assert!(
                token_principal(true, Some(&ann), true, &method, path).is_err(),
                "{method} {path} is an owner operation"
            );
        }
        assert!(
            token_principal(
                true,
                Some(&ann),
                false,
                &Method::GET,
                "/api/companies/acme/actors/exec/conversation"
            )
            .is_err(),
            "a removed or changed membership stops the token"
        );
    }

    #[test]
    fn a_token_works_only_in_the_entry_mode_it_was_issued_in() {
        assert!(token_principal(false, None, false, &Method::GET, "/api/companies").is_ok());
        assert!(token_principal(true, None, true, &Method::GET, "/api/companies").is_err());
        assert!(
            token_principal(false, Some(&member()), true, &Method::GET, "/api/companies").is_err()
        );
    }
}
