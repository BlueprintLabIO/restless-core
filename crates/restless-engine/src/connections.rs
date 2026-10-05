//! One connection model for every external tool (Sprint 57, ADR 0014).
//!
//! A connection is a remote MCP server (by URL) or a local MCP server (by
//! command). Both run outside the company Runtime: remote calls leave from
//! this host, local servers run in a disposable host-side Docker worker. The
//! Runtime only ever sees the tool gateway (`tool_gateway.rs`) and an
//! expiring session capability.
//!
//! Authority owns three facts here: which connections exist (and their
//! observed tool contract), which tools the owner granted to whom and in which
//! consequence class, and whether a connection is frozen. Credentials are
//! references resolved at call time; token values never reach Postgres rows,
//! receipts, or the Runtime.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use rmcp::{
    model::Tool,
    service::RunningService,
    transport::{
        async_rw::AsyncRwTransport,
        auth::{
            AuthError, AuthorizationManager, AuthorizationRequest, AuthorizationSession,
            CredentialStore, StoredCredentials,
        },
        streamable_http_client::StreamableHttpClientTransportConfig,
        StreamableHttpClientTransport,
    },
    RoleClient, ServiceExt as _,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
const LOCAL_STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
const IDLE_UPSTREAM: Duration = Duration::from_secs(300);
const MAX_RESULT_BYTES: usize = 2 * 1024 * 1024;
const OAUTH_SESSION_TTL: Duration = Duration::from_secs(15 * 60);
/// Namespaces gateway tool names: `<connection>__<tool>`.
pub const TOOL_SEPARATOR: &str = "__";

/// What a granted tool may do and how Authority governs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolClass {
    /// Ordinary work: gateway allowlist and a read receipt.
    Reads,
    /// A governed effect: intent, idempotency, receipt, freeze, first contact.
    Acts,
    /// A governed effect that also needs the owner's approval on every call.
    Reserved,
}

impl ToolClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reads => "reads",
            Self::Acts => "acts",
            Self::Reserved => "reserved",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConnectionAuth {
    None,
    /// A static bearer credential: a credential reference (`infisical:…`,
    /// `env:…`) or the name of a company credential binding.
    Bearer {
        credential: String,
    },
    /// MCP authorization (OAuth 2.1). Tokens live under `credential`; an owner
    /// may bring their own client registration.
    Oauth {
        credential: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_secret: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        scopes: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservedTool {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub annotations: Option<serde_json::Value>,
    /// Digest of the exact upstream definition. A grant pins it.
    pub digest: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Connection {
    pub name: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub args: Vec<String>,
    /// Environment name → credential reference, for local servers.
    pub env: BTreeMap<String, String>,
    #[serde(skip_serializing)]
    pub auth: ConnectionAuth,
    pub auth_type: String,
    pub status: String,
    pub frozen: bool,
    pub account: Option<String>,
    pub server_name: Option<String>,
    pub server_version: Option<String>,
    pub tools: Vec<ObservedTool>,
    pub failure: Option<String>,
    pub source: Option<String>,
    pub revision: Uuid,
    pub last_probe_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GrantedTool {
    pub tool: String,
    pub class: ToolClass,
    /// Argument names whose values name the external party reached
    /// (for example `to`, `cc`, `bcc`). The first-contact rule applies to them.
    #[serde(default)]
    pub party_args: Vec<String>,
    /// The upstream definition digest at grant time.
    #[serde(default)]
    pub digest: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionGrant {
    pub connection: String,
    pub revision: Uuid,
    /// An actor id, or `*` for every actor in the company.
    pub grantee: String,
    pub tools: Vec<GrantedTool>,
    pub granted_by: String,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// One granted tool as an actor may currently use it.
#[derive(Debug, Clone)]
pub struct UsableTool {
    pub connection: Connection,
    pub grant: GrantedTool,
    pub observed: ObservedTool,
    pub grant_revision: Uuid,
}

impl UsableTool {
    pub fn gateway_name(&self) -> String {
        format!(
            "{}{TOOL_SEPARATOR}{}",
            self.connection.name, self.grant.tool
        )
    }
}

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.connections (\
           company TEXT NOT NULL, name TEXT NOT NULL, \
           kind TEXT NOT NULL CHECK (kind IN ('remote','local')), \
           endpoint TEXT, command TEXT, args JSONB NOT NULL DEFAULT '[]'::jsonb, \
           env JSONB NOT NULL DEFAULT '{}'::jsonb, auth JSONB NOT NULL DEFAULT '{\"type\":\"none\"}'::jsonb, \
           status TEXT NOT NULL, frozen BOOLEAN NOT NULL DEFAULT FALSE, account TEXT, \
           server_name TEXT, server_version TEXT, tools JSONB NOT NULL DEFAULT '[]'::jsonb, \
           failure TEXT, source TEXT, revision UUID NOT NULL DEFAULT gen_random_uuid(), \
           last_probe_at TIMESTAMPTZ, created_by TEXT NOT NULL, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           PRIMARY KEY (company, name)\
         )",
    )
    .execute(pool)
    .await
    .context("create connections")?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.connection_grants (\
           company TEXT NOT NULL, connection TEXT NOT NULL, revision UUID PRIMARY KEY, \
           grantee TEXT NOT NULL, tools JSONB NOT NULL, granted_by TEXT NOT NULL, \
           granted_at TIMESTAMPTZ NOT NULL DEFAULT now(), expires_at TIMESTAMPTZ, \
           revoked_at TIMESTAMPTZ, \
           FOREIGN KEY (company, connection) REFERENCES restless_authority.connections(company, name) ON DELETE CASCADE\
         )",
    )
    .execute(pool)
    .await
    .context("create connection grants")?;
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS connection_grants_one_live \
         ON restless_authority.connection_grants (company, connection, grantee) \
         WHERE revoked_at IS NULL",
    )
    .execute(pool)
    .await
    .context("index live connection grants")?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.tool_read_receipts (\
           id UUID PRIMARY KEY, company TEXT NOT NULL, actor TEXT NOT NULL, \
           work_id UUID, attempt_id UUID, connection TEXT NOT NULL, tool TEXT NOT NULL, \
           contract_digest TEXT NOT NULL, request_digest TEXT NOT NULL, result_digest TEXT, \
           status TEXT NOT NULL CHECK (status IN ('complete','tool_error','outcome_unknown','not_invoked')), \
           error_class TEXT, wall_ms BIGINT, observed_at TIMESTAMPTZ NOT NULL DEFAULT now()\
         )",
    )
    .execute(pool)
    .await
    .context("create tool read receipts")?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS tool_read_receipts_by_connection \
         ON restless_authority.tool_read_receipts (company, connection, observed_at DESC)",
    )
    .execute(pool)
    .await
    .context("index tool read receipts")?;
    Ok(())
}

pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 40
        || name.contains(TOOL_SEPARATOR)
        || name.starts_with('-')
        || !name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        bail!("connection name must be 1-40 lowercase letters, digits, '-' or '_' without '__'");
    }
    Ok(())
}

fn validate_endpoint(endpoint: &str) -> Result<()> {
    let url = url::Url::parse(endpoint).context("connection endpoint is not a URL")?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    if !(url.scheme() == "https" || (url.scheme() == "http" && local))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        bail!("remote MCP endpoints must be https (http only for localhost) without credentials or fragments");
    }
    Ok(())
}

fn validate_env_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 64
        || !name.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_uppercase() || byte == b'_' || (index > 0 && byte.is_ascii_digit())
        })
    {
        bail!("invalid environment name {name:?}");
    }
    Ok(())
}

fn validate_party_arg(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        bail!("invalid party argument name {name:?}");
    }
    Ok(())
}

fn row_to_connection(row: &sqlx::postgres::PgRow) -> Result<Connection> {
    let auth: ConnectionAuth = serde_json::from_value(row.try_get("auth")?)
        .context("stored connection auth is malformed")?;
    let auth_type = match &auth {
        ConnectionAuth::None => "none",
        ConnectionAuth::Bearer { .. } => "bearer",
        ConnectionAuth::Oauth { .. } => "oauth",
    }
    .to_string();
    Ok(Connection {
        name: row.try_get("name")?,
        kind: row.try_get("kind")?,
        endpoint: row.try_get("endpoint")?,
        command: row.try_get("command")?,
        args: serde_json::from_value(row.try_get("args")?)?,
        env: serde_json::from_value(row.try_get("env")?)?,
        auth,
        auth_type,
        status: row.try_get("status")?,
        frozen: row.try_get("frozen")?,
        account: row.try_get("account")?,
        server_name: row.try_get("server_name")?,
        server_version: row.try_get("server_version")?,
        tools: serde_json::from_value(row.try_get("tools")?)?,
        failure: row.try_get("failure")?,
        source: row.try_get("source")?,
        revision: row.try_get("revision")?,
        last_probe_at: row.try_get("last_probe_at")?,
        created_at: row.try_get("created_at")?,
    })
}

const CONNECTION_COLUMNS: &str = "name, kind, endpoint, command, args, env, auth, status, frozen, \
    account, server_name, server_version, tools, failure, source, revision, last_probe_at, created_at";

pub async fn list(pool: &PgPool, company: &str) -> Result<Vec<Connection>> {
    let rows = sqlx::query(&format!(
        "SELECT {CONNECTION_COLUMNS} FROM restless_authority.connections \
         WHERE company=$1 ORDER BY name"
    ))
    .bind(company)
    .fetch_all(pool)
    .await
    .context("list connections")?;
    rows.iter().map(row_to_connection).collect()
}

pub async fn get(pool: &PgPool, company: &str, name: &str) -> Result<Option<Connection>> {
    let row = sqlx::query(&format!(
        "SELECT {CONNECTION_COLUMNS} FROM restless_authority.connections \
         WHERE company=$1 AND name=$2"
    ))
    .bind(company)
    .bind(name)
    .fetch_optional(pool)
    .await
    .context("read connection")?;
    row.as_ref().map(row_to_connection).transpose()
}

pub async fn require(pool: &PgPool, company: &str, name: &str) -> Result<Connection> {
    get(pool, company, name)
        .await?
        .with_context(|| format!("connection {name:?} does not exist"))
}

/// What the owner supplies to add a connection.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NewConnection {
    Remote {
        name: String,
        endpoint: String,
        #[serde(default = "default_auth")]
        auth: ConnectionAuth,
        #[serde(default)]
        source: Option<String>,
    },
    Local {
        name: String,
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
        #[serde(default)]
        source: Option<String>,
    },
}

fn default_auth() -> ConnectionAuth {
    ConnectionAuth::None
}

pub async fn add(
    pool: &PgPool,
    authority: &crate::authority::AuthorityStore,
    company: &str,
    new: NewConnection,
    created_by: &str,
) -> Result<Connection> {
    let (name, kind, endpoint, command, args, env, auth, source) = match new {
        NewConnection::Remote {
            name,
            endpoint,
            auth,
            source,
        } => {
            validate_endpoint(&endpoint)?;
            (
                name,
                "remote",
                Some(endpoint),
                None,
                Vec::new(),
                BTreeMap::new(),
                auth,
                source,
            )
        }
        NewConnection::Local {
            name,
            command,
            args,
            env,
            source,
        } => {
            if command.trim().is_empty()
                || command.contains('\0')
                || args.iter().any(|arg| arg.contains('\0'))
                || args.len() > 64
            {
                bail!("local MCP command is empty or contains NUL");
            }
            for name in env.keys() {
                validate_env_name(name)?;
            }
            (
                name,
                "local",
                None,
                Some(command),
                args,
                env,
                ConnectionAuth::None,
                source,
            )
        }
    };
    validate_name(&name)?;
    if let ConnectionAuth::Oauth { credential, .. } | ConnectionAuth::Bearer { credential } = &auth
    {
        if credential.trim().is_empty() {
            bail!("connection credential reference is empty");
        }
    }
    let status = match &auth {
        ConnectionAuth::Oauth { .. } => "awaiting_sign_in",
        _ => "awaiting_probe",
    };
    let inserted = sqlx::query(
        "INSERT INTO restless_authority.connections \
         (company,name,kind,endpoint,command,args,env,auth,status,source,created_by) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT DO NOTHING",
    )
    .bind(company)
    .bind(&name)
    .bind(kind)
    .bind(&endpoint)
    .bind(&command)
    .bind(serde_json::json!(args))
    .bind(serde_json::json!(env))
    .bind(serde_json::to_value(&auth)?)
    .bind(status)
    .bind(&source)
    .bind(created_by)
    .execute(pool)
    .await
    .context("record connection")?
    .rows_affected();
    if inserted != 1 {
        bail!("connection {name:?} already exists");
    }
    authority
        .emit(
            company,
            "connection_added",
            Some(created_by),
            serde_json::json!({
                "connection": name, "kind": kind, "endpoint": endpoint, "command": command,
                "args": args, "env_names": env.keys().collect::<Vec<_>>(),
                "auth_type": match &auth { ConnectionAuth::None => "none", ConnectionAuth::Bearer { .. } => "bearer", ConnectionAuth::Oauth { .. } => "oauth" },
                "source": source,
            }),
        )
        .await?;
    require(pool, company, &name).await
}

/// Read the observed contract from the server and record it. Tools whose
/// definition changed since their grant fail closed individually at call
/// time; probing never widens a grant.
pub async fn probe(pool: &PgPool, root: &Path, company: &str, name: &str) -> Result<Connection> {
    let connection = require(pool, company, name).await?;
    if connection.status == "disconnected" {
        bail!("connection {name:?} is disconnected; add it again to reconnect");
    }
    let result = async {
        let client = open_upstream(root, company, &connection).await?;
        let info = client.peer_info().map(|info| (*info).clone());
        let tools = tokio::time::timeout(PROBE_TIMEOUT, client.list_all_tools())
            .await
            .context("tool discovery timed out")?
            .context("tool discovery failed")?;
        let _ = tokio::time::timeout(Duration::from_secs(3), client.cancel()).await;
        Ok::<_, anyhow::Error>((info, tools))
    }
    .await;
    match result {
        Ok((info, tools)) => {
            let observed = observe_tools(tools)?;
            let (server_name, server_version) = info
                .and_then(|info| info.server_info.map(|server| (server.name, server.version)))
                .unzip();
            sqlx::query(
                "UPDATE restless_authority.connections SET status='working', failure=NULL, \
                 tools=$3, server_name=$4, server_version=$5, last_probe_at=now(), updated_at=now() \
                 WHERE company=$1 AND name=$2 AND status<>'disconnected'",
            )
            .bind(company)
            .bind(name)
            .bind(serde_json::to_value(&observed)?)
            .bind(server_name)
            .bind(server_version)
            .execute(pool)
            .await?;
        }
        Err(error) => {
            let failure = classify_failure(&error);
            sqlx::query(
                "UPDATE restless_authority.connections SET status=$3, failure=$4, \
                 last_probe_at=now(), updated_at=now() \
                 WHERE company=$1 AND name=$2 AND status<>'disconnected'",
            )
            .bind(company)
            .bind(name)
            .bind(if failure == "sign_in_required" {
                "awaiting_sign_in"
            } else {
                "failed"
            })
            .bind(&failure)
            .execute(pool)
            .await?;
            forget_upstream(company, name).await;
        }
    }
    require(pool, company, name).await
}

/// Re-read a working connection's live tool contracts through its pooled
/// session. A tool whose definition changed upstream gets a new digest, which
/// withholds it from every grant until the owner grants it again.
pub async fn refresh_tools(
    pool: &PgPool,
    root: &Path,
    company: &str,
    connection: &Connection,
) -> Result<()> {
    let client = upstream(root, company, connection).await?;
    let tools = tokio::time::timeout(PROBE_TIMEOUT, client.list_all_tools())
        .await
        .context("tool discovery timed out")?
        .context("tool discovery failed")?;
    let observed = observe_tools(tools)?;
    if observed == connection.tools {
        return Ok(());
    }
    sqlx::query(
        "UPDATE restless_authority.connections SET tools=$3, updated_at=now() \
         WHERE company=$1 AND name=$2 AND revision=$4 AND status='working'",
    )
    .bind(company)
    .bind(&connection.name)
    .bind(serde_json::to_value(&observed)?)
    .bind(connection.revision)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fixed failure classes: provider error text can carry tokens or private
/// data and is not persisted.
pub fn classify_failure(error: &anyhow::Error) -> String {
    let text = format!("{error:#}");
    if text.contains("AuthorizationRequired")
        || text.contains("sign-in required")
        || text.contains("401")
    {
        "sign_in_required"
    } else if text.contains("timed out") {
        "timed_out"
    } else if text.contains("handshake") {
        "handshake_failed"
    } else if text.contains("discovery") {
        "discovery_failed"
    } else if text.contains("credential") || text.contains("Infisical") || text.contains("env") {
        "credential_unavailable"
    } else if text.contains("docker") || text.contains("worker") {
        "local_worker_failed"
    } else {
        "unreachable"
    }
    .to_string()
}

pub fn observe_tools(tools: Vec<Tool>) -> Result<Vec<ObservedTool>> {
    let mut observed = tools
        .into_iter()
        .map(|tool| {
            let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&tool)?));
            Ok(ObservedTool {
                name: tool.name.to_string(),
                title: tool.title.clone(),
                description: tool.description.as_ref().map(|value| value.to_string()),
                input_schema: serde_json::Value::Object((*tool.input_schema).clone()),
                annotations: tool
                    .annotations
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()?,
                digest,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    observed.sort_by(|a, b| a.name.cmp(&b.name));
    let mut seen = std::collections::HashSet::new();
    for tool in &observed {
        if tool.name.is_empty()
            || tool.name.len() > 96
            || !tool
                .name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            bail!("server advertised an unusable tool name");
        }
        if !seen.insert(tool.name.clone()) {
            bail!("server advertised tool {:?} twice", tool.name);
        }
    }
    Ok(observed)
}

/// Party-reaching argument names that commonly appear in send-like tools.
const PARTY_ARGUMENTS: &[&str] = &[
    "to",
    "cc",
    "bcc",
    "recipient",
    "recipients",
    "email",
    "emails",
    "attendees",
    "invitees",
    "phone",
    "phone_number",
    "channel",
    "user",
    "users",
];

/// The default class for a newly observed tool. MCP annotations from an
/// untrusted server are hints, so this is a proposal the owner (or Exec)
/// adjusts, never a decision: an unannotated tool is treated as `acts`.
pub fn proposed_grant(tool: &ObservedTool) -> GrantedTool {
    let hint = |key: &str| {
        tool.annotations
            .as_ref()
            .and_then(|annotations| annotations.get(key))
            .and_then(serde_json::Value::as_bool)
    };
    let class = if hint("readOnlyHint") == Some(true) {
        ToolClass::Reads
    } else if hint("destructiveHint") == Some(true) {
        ToolClass::Reserved
    } else {
        ToolClass::Acts
    };
    let party_args = if class == ToolClass::Reads {
        Vec::new()
    } else {
        tool.input_schema
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .map(|properties| {
                properties
                    .keys()
                    .filter(|key| PARTY_ARGUMENTS.contains(&key.to_ascii_lowercase().as_str()))
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default()
    };
    GrantedTool {
        tool: tool.name.clone(),
        class,
        party_args,
        digest: tool.digest.clone(),
    }
}

/// What the owner decides for one tool. The digest is taken from the
/// connection's current observation, never from the caller.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolDecision {
    pub tool: String,
    pub class: ToolClass,
    #[serde(default)]
    pub party_args: Option<Vec<String>>,
}

/// Grant tools of a working connection to one actor or the whole company
/// (`*`). A new grant replaces the grantee's previous one atomically.
#[allow(clippy::too_many_arguments)]
pub async fn grant(
    pool: &PgPool,
    authority: &crate::authority::AuthorityStore,
    company: &str,
    name: &str,
    grantee: &str,
    decisions: Vec<ToolDecision>,
    granted_by: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<ConnectionGrant> {
    let connection = require(pool, company, name).await?;
    if connection.status != "working" {
        bail!("connection {name:?} must be probed and working before it can be granted");
    }
    if grantee != "*" {
        crate::capability::CapabilityIssuer::validate_actor_name(grantee)?;
    }
    if decisions.is_empty() {
        bail!("a grant names at least one tool");
    }
    let mut tools = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for decision in decisions {
        let observed = connection
            .tools
            .iter()
            .find(|tool| tool.name == decision.tool)
            .with_context(|| {
                format!(
                    "connection {name:?} does not offer tool {:?}",
                    decision.tool
                )
            })?;
        if !seen.insert(decision.tool.clone()) {
            bail!("tool {:?} appears twice in the grant", decision.tool);
        }
        let party_args = match decision.party_args {
            Some(arguments) => arguments,
            None => proposed_grant(observed).party_args,
        };
        for argument in &party_args {
            validate_party_arg(argument)?;
        }
        tools.push(GrantedTool {
            tool: decision.tool,
            class: decision.class,
            party_args: if decision.class == ToolClass::Reads {
                Vec::new()
            } else {
                party_args
            },
            digest: observed.digest.clone(),
        });
    }
    tools.sort_by(|a, b| a.tool.cmp(&b.tool));
    let revision = Uuid::new_v4();
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE restless_authority.connection_grants SET revoked_at=now() \
         WHERE company=$1 AND connection=$2 AND grantee=$3 AND revoked_at IS NULL",
    )
    .bind(company)
    .bind(name)
    .bind(grantee)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO restless_authority.connection_grants \
         (company,connection,revision,grantee,tools,granted_by,expires_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company)
    .bind(name)
    .bind(revision)
    .bind(grantee)
    .bind(serde_json::to_value(&tools)?)
    .bind(granted_by)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    authority
        .emit(
            company,
            "connection_granted",
            Some(granted_by),
            serde_json::json!({
                "connection": name, "revision": revision, "grantee": grantee,
                "tools": tools, "expires_at": expires_at,
            }),
        )
        .await?;
    Ok(ConnectionGrant {
        connection: name.to_string(),
        revision,
        grantee: grantee.to_string(),
        tools,
        granted_by: granted_by.to_string(),
        granted_at: Utc::now(),
        expires_at,
    })
}

pub async fn revoke_grant(
    pool: &PgPool,
    authority: &crate::authority::AuthorityStore,
    company: &str,
    name: &str,
    grantee: &str,
    by: &str,
) -> Result<bool> {
    let revoked = sqlx::query(
        "UPDATE restless_authority.connection_grants SET revoked_at=now() \
         WHERE company=$1 AND connection=$2 AND grantee=$3 AND revoked_at IS NULL",
    )
    .bind(company)
    .bind(name)
    .bind(grantee)
    .execute(pool)
    .await?
    .rows_affected();
    if revoked > 0 {
        authority
            .emit(
                company,
                "connection_grant_revoked",
                Some(by),
                serde_json::json!({ "connection": name, "grantee": grantee }),
            )
            .await?;
    }
    Ok(revoked > 0)
}

pub async fn grants(
    pool: &PgPool,
    company: &str,
    name: Option<&str>,
) -> Result<Vec<ConnectionGrant>> {
    let rows = sqlx::query(
        "SELECT connection, revision, grantee, tools, granted_by, granted_at, expires_at \
         FROM restless_authority.connection_grants \
         WHERE company=$1 AND ($2::TEXT IS NULL OR connection=$2) AND revoked_at IS NULL \
           AND (expires_at IS NULL OR expires_at > now()) \
         ORDER BY connection, grantee",
    )
    .bind(company)
    .bind(name)
    .fetch_all(pool)
    .await
    .context("list connection grants")?;
    rows.iter()
        .map(|row| {
            Ok(ConnectionGrant {
                connection: row.try_get("connection")?,
                revision: row.try_get("revision")?,
                grantee: row.try_get("grantee")?,
                tools: serde_json::from_value(row.try_get("tools")?)?,
                granted_by: row.try_get("granted_by")?,
                granted_at: row.try_get("granted_at")?,
                expires_at: row.try_get("expires_at")?,
            })
        })
        .collect()
}

pub async fn set_frozen(
    pool: &PgPool,
    authority: &crate::authority::AuthorityStore,
    company: &str,
    name: &str,
    frozen: bool,
    by: &str,
) -> Result<Connection> {
    let changed = sqlx::query(
        "UPDATE restless_authority.connections SET frozen=$3, updated_at=now() \
         WHERE company=$1 AND name=$2 AND frozen<>$3",
    )
    .bind(company)
    .bind(name)
    .bind(frozen)
    .execute(pool)
    .await?
    .rows_affected();
    if changed > 0 {
        authority
            .emit(
                company,
                if frozen {
                    "connection_frozen"
                } else {
                    "connection_unfrozen"
                },
                Some(by),
                serde_json::json!({ "connection": name }),
            )
            .await?;
    }
    require(pool, company, name).await
}

/// Revoke every grant, forget the credential and stop using the connection.
/// The row remains as history so receipts keep naming it.
pub async fn disconnect(
    pool: &PgPool,
    authority: &crate::authority::AuthorityStore,
    root: &Path,
    company: &str,
    name: &str,
    by: &str,
) -> Result<Connection> {
    let connection = require(pool, company, name).await?;
    if let ConnectionAuth::Oauth { credential, .. } = &connection.auth {
        if let Err(error) = token_store(root, company, name, credential).clear().await {
            tracing::warn!(company, name, %error, "connection token could not be cleared");
        }
    }
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE restless_authority.connection_grants SET revoked_at=now() \
         WHERE company=$1 AND connection=$2 AND revoked_at IS NULL",
    )
    .bind(company)
    .bind(name)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE restless_authority.connections SET status='disconnected', \
         auth='{\"type\":\"none\"}'::jsonb, env='{}'::jsonb, account=NULL, \
         revision=gen_random_uuid(), updated_at=now() WHERE company=$1 AND name=$2",
    )
    .bind(company)
    .bind(name)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    forget_upstream(company, name).await;
    authority
        .emit(
            company,
            "connection_disconnected",
            Some(by),
            serde_json::json!({ "connection": name }),
        )
        .await?;
    require(pool, company, name).await
}

/// The tools an actor may use right now. A tool whose upstream definition
/// changed since the grant is withheld until the owner grants it again.
pub async fn usable_tools(pool: &PgPool, company: &str, actor: &str) -> Result<Vec<UsableTool>> {
    let connections = list(pool, company)
        .await?
        .into_iter()
        .filter(|connection| connection.status == "working")
        .map(|connection| (connection.name.clone(), connection))
        .collect::<HashMap<_, _>>();
    let mut usable: BTreeMap<String, UsableTool> = BTreeMap::new();
    for grant in grants(pool, company, None).await? {
        if grant.grantee != "*" && grant.grantee != actor {
            continue;
        }
        let Some(connection) = connections.get(&grant.connection) else {
            continue;
        };
        for granted in grant.tools {
            let Some(observed) = connection
                .tools
                .iter()
                .find(|tool| tool.name == granted.tool)
            else {
                continue;
            };
            if observed.digest != granted.digest {
                continue;
            }
            let tool = UsableTool {
                connection: connection.clone(),
                grant: granted,
                observed: observed.clone(),
                grant_revision: grant.revision,
            };
            // An actor-specific grant is more specific than the company one.
            let key = tool.gateway_name();
            if grant.grantee == actor || !usable.contains_key(&key) {
                usable.insert(key, tool);
            }
        }
    }
    Ok(usable.into_values().collect())
}

/// Tools whose definitions changed upstream since they were granted.
pub async fn changed_tools(pool: &PgPool, company: &str) -> Result<Vec<(String, String)>> {
    let connections = list(pool, company).await?;
    let mut changed = Vec::new();
    for grant in grants(pool, company, None).await? {
        let Some(connection) = connections
            .iter()
            .find(|item| item.name == grant.connection)
        else {
            continue;
        };
        for granted in grant.tools {
            let current = connection
                .tools
                .iter()
                .find(|tool| tool.name == granted.tool);
            if current.is_none_or(|tool| tool.digest != granted.digest) {
                changed.push((grant.connection.clone(), granted.tool));
            }
        }
    }
    changed.sort();
    changed.dedup();
    Ok(changed)
}

#[allow(clippy::too_many_arguments)]
pub async fn record_read_receipt(
    pool: &PgPool,
    company: &str,
    actor: &str,
    work_id: Option<Uuid>,
    attempt_id: Option<Uuid>,
    connection: &str,
    tool: &str,
    contract_digest: &str,
    request_digest: &str,
    result_digest: Option<&str>,
    status: &str,
    error_class: Option<&str>,
    wall_ms: i64,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO restless_authority.tool_read_receipts \
         (id,company,actor,work_id,attempt_id,connection,tool,contract_digest,request_digest, \
          result_digest,status,error_class,wall_ms) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
    )
    .bind(id)
    .bind(company)
    .bind(actor)
    .bind(work_id)
    .bind(attempt_id)
    .bind(connection)
    .bind(tool)
    .bind(contract_digest)
    .bind(request_digest)
    .bind(result_digest)
    .bind(status)
    .bind(error_class)
    .bind(wall_ms)
    .execute(pool)
    .await
    .context("record tool read receipt")?;
    Ok(id)
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadReceipt {
    pub id: Uuid,
    pub actor: String,
    pub connection: String,
    pub tool: String,
    pub status: String,
    pub observed_at: DateTime<Utc>,
    pub result_digest: Option<String>,
}

pub async fn read_receipt(pool: &PgPool, company: &str, id: Uuid) -> Result<Option<ReadReceipt>> {
    let row = sqlx::query(
        "SELECT id, actor, connection, tool, status, observed_at, result_digest \
         FROM restless_authority.tool_read_receipts WHERE company=$1 AND id=$2",
    )
    .bind(company)
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        Ok(ReadReceipt {
            id: row.try_get("id")?,
            actor: row.try_get("actor")?,
            connection: row.try_get("connection")?,
            tool: row.try_get("tool")?,
            status: row.try_get("status")?,
            observed_at: row.try_get("observed_at")?,
            result_digest: row.try_get("result_digest")?,
        })
    })
    .transpose()
}

pub async fn recent_read_receipts(
    pool: &PgPool,
    company: &str,
    connection: &str,
    limit: i64,
) -> Result<Vec<ReadReceipt>> {
    let rows = sqlx::query(
        "SELECT id, actor, connection, tool, status, observed_at, result_digest \
         FROM restless_authority.tool_read_receipts WHERE company=$1 AND connection=$2 \
         ORDER BY observed_at DESC LIMIT $3",
    )
    .bind(company)
    .bind(connection)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(ReadReceipt {
                id: row.try_get("id")?,
                actor: row.try_get("actor")?,
                connection: row.try_get("connection")?,
                tool: row.try_get("tool")?,
                status: row.try_get("status")?,
                observed_at: row.try_get("observed_at")?,
                result_digest: row.try_get("result_digest")?,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Credentials

/// Resolve a credential reference or a company credential binding name.
async fn resolve_credential(root: &Path, company: &str, credential: &str) -> Result<String> {
    let value = if credential.contains(':') {
        crate::credential::resolve_reference(credential).await?
    } else {
        let config = crate::runtime::CompanyConfig::load(root, company)?;
        crate::credential::resolve(&config, credential).await?
    };
    Ok(value.trim_end_matches(['\r', '\n']).to_string())
}

/// Where OAuth tokens for one connection live. Infisical when the plane has
/// it; otherwise a private file in the account plane's own state directory.
/// Neither location is reachable from the company Runtime.
pub enum TokenStore {
    Reference(String),
    PrivateFile(PathBuf),
}

pub fn token_store(root: &Path, company: &str, name: &str, credential: &str) -> TokenStore {
    if credential.starts_with("infisical:") {
        TokenStore::Reference(credential.to_string())
    } else {
        TokenStore::PrivateFile(
            root.join("connections")
                .join(company)
                .join(format!("{name}.oauth.json")),
        )
    }
}

/// The default token location for a new OAuth connection.
pub fn default_oauth_credential(company: &str, name: &str) -> String {
    if crate::credential::infisical_configured() {
        format!("infisical:/restless/{company}/connections/{name}#oauth")
    } else {
        format!("plane-file:{company}/{name}")
    }
}

impl TokenStore {
    async fn read(&self) -> Result<Option<String>> {
        match self {
            Self::Reference(reference) => {
                match crate::credential::resolve_reference(reference).await {
                    Ok(value) if value.trim().is_empty() || value.trim() == "{}" => Ok(None),
                    Ok(value) => Ok(Some(value)),
                    Err(error) if format!("{error:#}").contains("not found") => Ok(None),
                    Err(error) => Err(error),
                }
            }
            Self::PrivateFile(path) => match std::fs::read_to_string(path) {
                Ok(value) => Ok(Some(value)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error).context("read connection token file"),
            },
        }
    }

    async fn write(&self, value: &str) -> Result<()> {
        match self {
            Self::Reference(reference) => {
                crate::credential::store_reference(reference, value).await
            }
            Self::PrivateFile(path) => {
                use std::io::Write as _;
                use std::os::unix::fs::OpenOptionsExt as _;
                let parent = path.parent().context("token file has no parent")?;
                std::fs::create_dir_all(parent)?;
                std::fs::set_permissions(
                    parent,
                    std::os::unix::fs::PermissionsExt::from_mode(0o700),
                )?;
                let temporary = path.with_extension(format!("tmp-{}", Uuid::new_v4().simple()));
                let mut file = std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .mode(0o600)
                    .open(&temporary)
                    .context("create connection token file")?;
                file.write_all(value.as_bytes())?;
                file.sync_all()?;
                std::fs::rename(&temporary, path).context("replace connection token file")?;
                Ok(())
            }
        }
    }

    pub async fn clear(&self) -> Result<()> {
        match self {
            Self::Reference(reference) => crate::credential::store_reference(reference, "{}").await,
            Self::PrivateFile(path) => match std::fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error).context("remove connection token file"),
            },
        }
    }
}

struct OauthCredentialStore(Arc<TokenStore>);

#[async_trait::async_trait]
impl CredentialStore for OauthCredentialStore {
    async fn load(&self) -> std::result::Result<Option<StoredCredentials>, AuthError> {
        let raw = self.0.read().await.map_err(|error| {
            AuthError::InternalError(format!("token store: {}", classify_failure(&error)))
        })?;
        match raw {
            None => Ok(None),
            Some(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|_| AuthError::InternalError("stored token is malformed".into())),
        }
    }

    async fn save(&self, credentials: StoredCredentials) -> std::result::Result<(), AuthError> {
        let raw = serde_json::to_string(&credentials)
            .map_err(|_| AuthError::InternalError("token could not be encoded".into()))?;
        self.0.write(&raw).await.map_err(|error| {
            AuthError::InternalError(format!("token store: {}", classify_failure(&error)))
        })
    }

    async fn clear(&self) -> std::result::Result<(), AuthError> {
        self.0.clear().await.map_err(|error| {
            AuthError::InternalError(format!("token store: {}", classify_failure(&error)))
        })
    }
}

async fn oauth_manager(
    root: &Path,
    company: &str,
    name: &str,
    endpoint: &str,
    credential: &str,
) -> Result<AuthorizationManager> {
    let mut manager = AuthorizationManager::new(endpoint)
        .await
        .map_err(|error| anyhow::anyhow!("authorization discovery failed: {error}"))?;
    manager.set_credential_store(OauthCredentialStore(Arc::new(token_store(
        root, company, name, credential,
    ))));
    Ok(manager)
}

async fn oauth_access_token(root: &Path, company: &str, connection: &Connection) -> Result<String> {
    let ConnectionAuth::Oauth { credential, .. } = &connection.auth else {
        bail!("connection does not use OAuth");
    };
    let endpoint = connection
        .endpoint
        .as_deref()
        .context("OAuth connection has no endpoint")?;
    let mut manager = oauth_manager(root, company, &connection.name, endpoint, credential).await?;
    let present = manager
        .initialize_from_store()
        .await
        .map_err(|error| anyhow::anyhow!("token store unavailable: {error}"))?;
    if !present {
        bail!("sign-in required (AuthorizationRequired)");
    }
    manager
        .get_access_token()
        .await
        .map_err(|error| match error {
            AuthError::AuthorizationRequired => {
                anyhow::anyhow!("sign-in required (AuthorizationRequired)")
            }
            other => anyhow::anyhow!("token refresh failed: {other}"),
        })
}

struct PendingSignIn {
    company: String,
    connection: String,
    session: AuthorizationSession,
    started: Instant,
}

static PENDING_SIGN_INS: LazyLock<tokio::sync::Mutex<HashMap<String, PendingSignIn>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

/// Begin MCP authorization for an OAuth connection. The returned URL is
/// opened by the owner in their own browser; the provider redirects back to
/// the account plane's callback, never to the company Runtime.
pub async fn begin_sign_in(
    pool: &PgPool,
    root: &Path,
    company: &str,
    name: &str,
    redirect_uri: &str,
) -> Result<String> {
    let connection = require(pool, company, name).await?;
    let ConnectionAuth::Oauth {
        credential,
        client_id,
        client_secret,
        scopes,
    } = &connection.auth
    else {
        bail!("connection {name:?} does not use OAuth sign-in");
    };
    let endpoint = connection
        .endpoint
        .as_deref()
        .context("OAuth connection has no endpoint")?;
    let mut manager = oauth_manager(root, company, name, endpoint, credential).await?;
    let resolution = manager
        .resolve_metadata()
        .await
        .map_err(|error| anyhow::anyhow!("authorization discovery failed: {error}"))?;
    manager.set_metadata(resolution.metadata);
    let mut request = AuthorizationRequest::new(redirect_uri).with_client_name("Restless");
    if !scopes.is_empty() {
        request = request.with_scopes(scopes.iter().map(String::as_str));
    }
    if let Some(client_id) = client_id {
        request = request.with_preregistered_client(client_id);
        if let Some(secret) = client_secret {
            request = request.with_client_secret(resolve_credential(root, company, secret).await?);
        }
    }
    let session = AuthorizationSession::new(manager, request)
        .await
        .map_err(|(_, error)| anyhow::anyhow!("authorization could not start: {error}"))?;
    let url = session.get_authorization_url().to_string();
    let state = url::Url::parse(&url)?
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.to_string())
        .context("authorization URL has no state")?;
    let mut pending = PENDING_SIGN_INS.lock().await;
    pending.retain(|_, sign_in| sign_in.started.elapsed() < OAUTH_SESSION_TTL);
    pending.insert(
        state,
        PendingSignIn {
            company: company.to_string(),
            connection: name.to_string(),
            session,
            started: Instant::now(),
        },
    );
    Ok(url)
}

/// The company a pending sign-in belongs to, so the callback can check the
/// signed-in owner before completing it.
pub async fn pending_sign_in_company(state: &str) -> Option<String> {
    PENDING_SIGN_INS
        .lock()
        .await
        .get(state)
        .filter(|sign_in| sign_in.started.elapsed() < OAUTH_SESSION_TTL)
        .map(|sign_in| sign_in.company.clone())
}

/// Complete a sign-in from the provider's redirect. Returns the company and
/// connection it belonged to.
pub async fn complete_sign_in(
    pool: &PgPool,
    authority: &crate::authority::AuthorityStore,
    root: &Path,
    code: &str,
    state: &str,
    issuer: Option<&str>,
) -> Result<(String, String)> {
    let sign_in = PENDING_SIGN_INS
        .lock()
        .await
        .remove(state)
        .filter(|sign_in| sign_in.started.elapsed() < OAUTH_SESSION_TTL)
        .context("this sign-in link expired or was already used; start it again")?;
    sign_in
        .session
        .handle_callback_with_issuer(code, state, issuer)
        .await
        .map_err(|error| anyhow::anyhow!("sign-in could not be completed: {error}"))?;
    sqlx::query(
        "UPDATE restless_authority.connections SET status='awaiting_probe', failure=NULL, \
         revision=gen_random_uuid(), updated_at=now() \
         WHERE company=$1 AND name=$2 AND status<>'disconnected'",
    )
    .bind(&sign_in.company)
    .bind(&sign_in.connection)
    .execute(pool)
    .await?;
    forget_upstream(&sign_in.company, &sign_in.connection).await;
    authority
        .emit(
            &sign_in.company,
            "connection_signed_in",
            Some("owner"),
            serde_json::json!({ "connection": sign_in.connection }),
        )
        .await?;
    probe(pool, root, &sign_in.company, &sign_in.connection).await?;
    Ok((sign_in.company, sign_in.connection))
}

// ---------------------------------------------------------------------------
// Upstream sessions

type Upstream = Arc<RunningService<RoleClient, ()>>;

struct PooledUpstream {
    revision: Uuid,
    client: Upstream,
    last_used: Instant,
}

static UPSTREAMS: LazyLock<tokio::sync::Mutex<HashMap<(String, String), PooledUpstream>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

/// A reusable session to the connection's server. Local workers are slow to
/// start, so sessions are pooled per connection revision and closed when
/// idle, revised, frozen out or disconnected.
pub async fn upstream(root: &Path, company: &str, connection: &Connection) -> Result<Upstream> {
    let key = (company.to_string(), connection.name.clone());
    let mut pool = UPSTREAMS.lock().await;
    let stale = pool
        .iter()
        .filter(|(_, entry)| entry.last_used.elapsed() > IDLE_UPSTREAM || entry.client.is_closed())
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    for stale_key in stale {
        if let Some(entry) = pool.remove(&stale_key) {
            entry.client.cancellation_token().cancel();
        }
    }
    if let Some(entry) = pool.get_mut(&key) {
        if entry.revision == connection.revision && !entry.client.is_closed() {
            entry.last_used = Instant::now();
            return Ok(entry.client.clone());
        }
        if let Some(entry) = pool.remove(&key) {
            entry.client.cancellation_token().cancel();
        }
    }
    drop(pool);
    let client = Arc::new(open_upstream(root, company, connection).await?);
    UPSTREAMS.lock().await.insert(
        key,
        PooledUpstream {
            revision: connection.revision,
            client: client.clone(),
            last_used: Instant::now(),
        },
    );
    Ok(client)
}

pub async fn forget_upstream(company: &str, name: &str) {
    if let Some(entry) = UPSTREAMS
        .lock()
        .await
        .remove(&(company.to_string(), name.to_string()))
    {
        entry.client.cancellation_token().cancel();
    }
}

async fn open_upstream(
    root: &Path,
    company: &str,
    connection: &Connection,
) -> Result<RunningService<RoleClient, ()>> {
    match connection.kind.as_str() {
        "remote" => {
            let endpoint = connection
                .endpoint
                .as_deref()
                .context("remote connection has no endpoint")?;
            let token = match &connection.auth {
                ConnectionAuth::None => None,
                ConnectionAuth::Bearer { credential } => {
                    Some(resolve_credential(root, company, credential).await?)
                }
                ConnectionAuth::Oauth { .. } => {
                    Some(oauth_access_token(root, company, connection).await?)
                }
            };
            let client = reqwest_mcp::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .redirect(reqwest_mcp::redirect::Policy::none())
                .build()
                .context("build MCP HTTP client")?;
            let mut config = StreamableHttpClientTransportConfig::with_uri(endpoint.to_string())
                .max_sse_event_size(MAX_RESULT_BYTES);
            if let Some(token) = token {
                config = config.auth_header(token);
            }
            let transport = StreamableHttpClientTransport::with_client(client, config);
            tokio::time::timeout(PROBE_TIMEOUT, ().serve(transport))
                .await
                .context("remote MCP handshake timed out")?
                .map_err(|error| {
                    let text = error.to_string();
                    if text.contains("401") || text.contains("Unauthorized") {
                        anyhow::anyhow!("remote MCP handshake failed: sign-in required (401)")
                    } else {
                        anyhow::anyhow!("remote MCP handshake failed")
                    }
                })
        }
        "local" => open_local_worker(root, company, connection).await,
        other => bail!("unknown connection kind {other:?}"),
    }
}

/// Run a local MCP server in a fresh host-side Docker worker built from the
/// company image (so its toolchain matches the company computer). It has
/// ordinary outbound networking, no host mounts, a private cache volume for
/// package managers, and credentials passed by environment name only so the
/// values never appear in any process's argv.
async fn open_local_worker(
    root: &Path,
    company: &str,
    connection: &Connection,
) -> Result<RunningService<RoleClient, ()>> {
    let command = connection
        .command
        .as_deref()
        .context("local connection has no command")?;
    let mut secrets = BTreeMap::new();
    for (name, credential) in &connection.env {
        secrets.insert(
            name.clone(),
            resolve_credential(root, company, credential).await?,
        );
    }
    let image = crate::runtime::company_image();
    let worker = format!(
        "restless-mcp-{}-{}",
        connection.name,
        Uuid::new_v4().simple()
    );
    let cache = format!("restless-mcp-cache-{company}");
    let mut docker = tokio::process::Command::new("docker");
    docker.env_clear();
    docker
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("HOME", "/tmp");
    if let Ok(host) = std::env::var("DOCKER_HOST") {
        docker.env("DOCKER_HOST", host);
    }
    docker.args([
        "run",
        "--rm",
        "-i",
        "--pull=never",
        "--name",
        &worker,
        "--label",
        "io.restless.mcp-worker=true",
        "--label",
        &format!("io.restless.company={company}"),
        "--cap-drop",
        "ALL",
        "--security-opt",
        "no-new-privileges",
        "--pids-limit",
        "256",
        "--memory",
        "1g",
        "--user",
        "2001:2000",
        "--tmpfs",
        "/tmp:rw,nosuid,nodev,size=256m",
        "--mount",
        &format!("type=volume,src={cache},dst=/cache"),
        "--env",
        "HOME=/tmp",
        "--env",
        "TMPDIR=/tmp",
        "--env",
        "npm_config_cache=/cache/npm",
        "--env",
        "UV_CACHE_DIR=/cache/uv",
        "--env",
        "XDG_CACHE_HOME=/cache/xdg",
        "--workdir",
        "/tmp",
        "--entrypoint",
        "",
    ]);
    for (name, value) in &secrets {
        docker.arg("--env").arg(name);
        docker.env(name, value);
    }
    docker.arg(&image).arg(command).args(&connection.args);
    let mut child = docker
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("start local MCP worker")?;
    let stdout = child
        .stdout
        .take()
        .context("local MCP worker has no stdout")?;
    let stdin = child
        .stdin
        .take()
        .context("local MCP worker has no stdin")?;
    let handshake = tokio::time::timeout(
        LOCAL_STARTUP_TIMEOUT,
        ().serve(AsyncRwTransport::<RoleClient, _, _>::new(stdout, stdin)),
    )
    .await;
    match handshake {
        Ok(Ok(client)) => {
            // Closing the client drops its stdin; `docker run -i` forwards the EOF and
            // the worker exits. The reaper below removes the container either way.
            let worker_name = worker.clone();
            tokio::spawn(async move {
                let _ = child.wait().await;
                remove_worker(&worker_name).await;
            });
            Ok(client)
        }
        failed => {
            let _ = child.kill().await;
            remove_worker(&worker).await;
            match failed {
                Err(_) => bail!("local MCP worker handshake timed out"),
                _ => bail!("local MCP worker handshake failed"),
            }
        }
    }
}

async fn remove_worker(name: &str) {
    let _ = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::process::Command::new("docker")
            .args(["rm", "-f", name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    )
    .await;
}

/// Short summary of working connections for actor context, so planning can
/// rely on capabilities instead of discovering them mid-Attempt.
pub async fn context_summary(pool: &PgPool, company: &str, actor: &str) -> Result<Option<String>> {
    let tools = usable_tools(pool, company, actor).await?;
    if tools.is_empty() {
        return Ok(None);
    }
    let mut by_connection: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for tool in &tools {
        by_connection
            .entry(tool.connection.name.clone())
            .or_default()
            .push(format!(
                "{} ({})",
                tool.grant.tool,
                tool.grant.class.as_str()
            ));
    }
    let mut lines = vec![
        "Connected tools you may use through the `restless-tools` MCP server. \
         `reads` are ordinary work. `acts` produce a governed effect with a receipt: \
         give each call a purpose, and never retry one whose outcome is unknown; \
         check the external state with a read and settle it with `restless_reconcile_effect`. \
         `reserved` calls wait for the owner's approval."
            .to_string(),
    ];
    for (connection, tools) in by_connection {
        lines.push(format!("- {connection}: {}", tools.join(", ")));
    }
    Ok(Some(lines.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed(
        name: &str,
        annotations: Option<serde_json::Value>,
        properties: &[&str],
    ) -> ObservedTool {
        let properties = properties
            .iter()
            .map(|name| (name.to_string(), serde_json::json!({"type": "string"})))
            .collect::<serde_json::Map<_, _>>();
        ObservedTool {
            name: name.into(),
            title: None,
            description: None,
            input_schema: serde_json::json!({"type":"object","properties":properties}),
            annotations,
            digest: "d".into(),
        }
    }

    #[test]
    fn hints_only_propose_classes_and_unannotated_tools_act() {
        let read = proposed_grant(&observed(
            "search",
            Some(serde_json::json!({"readOnlyHint": true})),
            &["query"],
        ));
        assert_eq!(read.class, ToolClass::Reads);
        assert!(read.party_args.is_empty());
        let send = proposed_grant(&observed("send", None, &["to", "cc", "body"]));
        assert_eq!(send.class, ToolClass::Acts);
        assert_eq!(send.party_args, vec!["cc".to_string(), "to".to_string()]);
        let delete = proposed_grant(&observed(
            "delete",
            Some(serde_json::json!({"readOnlyHint": false, "destructiveHint": true})),
            &["id"],
        ));
        assert_eq!(delete.class, ToolClass::Reserved);
    }

    #[test]
    fn names_cannot_collide_with_the_gateway_namespace() {
        assert!(validate_name("gmail").is_ok());
        assert!(validate_name("my-crm_2").is_ok());
        assert!(validate_name("a__b").is_err());
        assert!(validate_name("Gmail").is_err());
        assert!(validate_name("").is_err());
        assert!(validate_endpoint("https://example.com/mcp").is_ok());
        assert!(validate_endpoint("http://localhost:9000/mcp").is_ok());
        assert!(validate_endpoint("http://example.com/mcp").is_err());
        assert!(validate_endpoint("https://user:pw@example.com/mcp").is_err());
    }
}
