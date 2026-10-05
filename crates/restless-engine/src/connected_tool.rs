//! Provider-neutral connected-tool state and ACP attachment materialisation.
//!
//! External applications remain the source of their own records. Authority
//! retains only why a connection exists, which actor may receive it, and the
//! last authenticated capability observation. OAuth material is deliberately
//! absent from Postgres; the scoped runtime credential directory is referenced
//! by path and consumed by the mature `mcp-remote` bridge.

use std::path::Path;
use std::process::Stdio;

use agent_client_protocol::schema::v1::{
    EnvVariable, HttpHeader, McpServer, McpServerHttp, McpServerStdio,
};
use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use restless_orgintel::{NewOwnerHandoff, OrgIntel, OwnerHandoffCategory, OwnerHandoffState};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row as _};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, BufReader};
use uuid::Uuid;

const MCP_REMOTE: &str = "/usr/local/bin/mcp-remote";
const MCP_REMOTE_CLIENT: &str = "/usr/local/bin/mcp-remote-client";
const RUNTIME_CREDENTIAL_ROOT: &str = "/company/home/.restless/connected-tools";
/// Internal marker consumed by the Codex launch adapter. The marker is never
/// passed to the MCP child; the child receives only selected ambient variable
/// names from its already-scoped actor session.
pub const BROKER_AWARE_ACTOR_ENV_MARKER: &str = "RESTLESS_INTERNAL_BROKER_AWARE_ACTOR_ENV";
const CLAPPING_HANDS_SOURCING_PORT: u16 = 7799;
const CLAPPING_HANDS_READ_TOOLS: [&str; 3] = [
    "clapping_hands_gumtree_public_listing",
    "clapping_hands_marketplace_details",
    "clapping_hands_marketplace_search",
];
// Keep existing three-tool Work pins valid while the bounded Gumtree batch is
// explicitly installed for a fresh Work. A service advertising a new tool does
// not grant it to an already-running Attempt.
const CLAPPING_HANDS_BATCH_READ_TOOLS: [&str; 4] = [
    "clapping_hands_gumtree_public_listing",
    "clapping_hands_gumtree_public_listings",
    "clapping_hands_marketplace_details",
    "clapping_hands_marketplace_search",
];
// Product photos are seller-provided evidence. They are exposed only by a
// separate, bounded exact-item read, so adding the tool requires a fresh owner
// pin instead of silently widening an existing four-tool Attempt.
const CLAPPING_HANDS_PHOTO_READ_TOOLS: [&str; 5] = [
    "clapping_hands_gumtree_public_listing",
    "clapping_hands_gumtree_public_listings",
    "clapping_hands_marketplace_details",
    "clapping_hands_marketplace_photo",
    "clapping_hands_marketplace_search",
];
const DEEPWIKI_ENDPOINT: &str = "https://mcp.deepwiki.com/mcp";
const DEEPWIKI_READ_TOOL: &str = "read_wiki_structure";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    AwaitingOwner,
    Enabled,
    Failed,
    Disabled,
}

impl ConnectionStatus {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "awaiting_owner" => Ok(Self::AwaitingOwner),
            "enabled" => Ok(Self::Enabled),
            "failed" => Ok(Self::Failed),
            "disabled" => Ok(Self::Disabled),
            other => bail!("unknown connected-tool status {other:?}"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectedTool {
    pub name: String,
    pub endpoint: String,
    pub purpose: String,
    pub assigned_actor: String,
    pub assigned_work_id: Option<Uuid>,
    pub assigned_attempt_id: Option<Uuid>,
    pub status: ConnectionStatus,
    pub credential_reference: String,
    pub requested_scopes: Vec<String>,
    pub observed_tools: Vec<String>,
    pub workspace_reference: Option<String>,
    pub owner_handoff_id: Option<String>,
    pub last_observed_at: Option<DateTime<Utc>>,
    pub failure: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConnectionLaunch {
    pub connection: ConnectedTool,
    pub authorization_url: Option<String>,
    pub owner_handoff_id: Option<Uuid>,
}

/// An owner-installed MCP. Legacy stdio runs inside the company Runtime;
/// host_http and broker_stdio pass through Core's Attempt-scoped gateway.
/// The host token file path is private state and never reaches actors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalMcpServer {
    pub name: String,
    pub transport: String,
    pub command: String,
    pub args: Vec<String>,
    pub endpoint: Option<String>,
    #[serde(skip_serializing)]
    pub token_file: Option<String>,
    pub read_profile: Option<String>,
    pub target_repository: Option<String>,
    pub assigned_actor: String,
    pub assigned_work_id: Option<Uuid>,
    pub broker_aware: bool,
    pub enabled: bool,
    pub allowed_tools: Vec<String>,
    /// Optional hard count of brokered reads for the fixed Work pin.
    /// Recurring Opportunity Works have a separate grant and are not capped here.
    pub max_calls_per_work: Option<i32>,
    pub observed_tools: Vec<String>,
    pub tool_contract_digest: Option<String>,
    pub policy_revision: Uuid,
    pub server_version: Option<String>,
    pub last_observed_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_read_status: Option<String>,
    pub last_read_site: Option<String>,
    pub last_read_tool: Option<String>,
    pub failure: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct McpReadReceipt {
    call_id: Uuid,
    phase: String,
    actor: String,
    work_id: Uuid,
    attempt_id: Uuid,
    connection_name: String,
    tool_name: String,
    tool_contract_digest: String,
    policy_revision: Uuid,
    request_digest: String,
    result_digest: Option<String>,
    subject: serde_json::Value,
    status: String,
    provider_status: Option<String>,
    observed_at: DateTime<Utc>,
    wall_ms: Option<i64>,
    error_class: Option<String>,
    schedule_id: Option<Uuid>,
    opportunity_id: Option<Uuid>,
    responsibility_id: Option<Uuid>,
    responsibility_version: Option<i32>,
    recurring_policy_revision: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecurringMcpPolicy {
    pub connection_name: String,
    pub schedule_id: Uuid,
    pub responsibility_id: Uuid,
    pub responsibility_version: i32,
    pub assigned_actor: String,
    pub enabled: bool,
    pub allowed_tools: Vec<String>,
    pub server_version: String,
    pub tool_contract_digest: String,
    pub endpoint: String,
    pub connection_policy_revision: Uuid,
    pub policy_revision: Uuid,
    pub approved_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.provider_connections (\
           company TEXT NOT NULL, name TEXT NOT NULL, endpoint TEXT NOT NULL, \
           purpose TEXT NOT NULL, assigned_actor TEXT NOT NULL, status TEXT NOT NULL, \
           credential_reference TEXT NOT NULL, requested_scopes JSONB NOT NULL DEFAULT '[]'::jsonb, \
           observed_tools JSONB NOT NULL DEFAULT '[]'::jsonb, workspace_reference TEXT, \
           owner_handoff_id TEXT, last_observed_at TIMESTAMPTZ, failure TEXT, \
           assigned_work_id UUID, assigned_attempt_id UUID, \
           created_by TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           PRIMARY KEY (company, name)\
         )",
    )
    .execute(pool)
    .await
    .context("create provider connections")?;
    sqlx::query(
        "ALTER TABLE restless_authority.provider_connections \
         ADD COLUMN IF NOT EXISTS assigned_work_id UUID, \
         ADD COLUMN IF NOT EXISTS assigned_attempt_id UUID",
    )
    .execute(pool)
    .await
    .context("add connected-tool execution scope")?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.local_mcp_servers (\
           company TEXT NOT NULL, name TEXT NOT NULL, command TEXT NOT NULL, \
           args JSONB NOT NULL DEFAULT '[]'::jsonb, assigned_actor TEXT NOT NULL, \
           broker_aware BOOLEAN NOT NULL DEFAULT FALSE, \
           enabled BOOLEAN NOT NULL DEFAULT TRUE, created_by TEXT NOT NULL, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           PRIMARY KEY (company, name)\
         )",
    )
    .execute(pool)
    .await
    .context("create company-local MCP descriptors")?;
    sqlx::query(
        "ALTER TABLE restless_authority.local_mcp_servers \
         ADD COLUMN IF NOT EXISTS broker_aware BOOLEAN NOT NULL DEFAULT FALSE, \
         ADD COLUMN IF NOT EXISTS transport TEXT NOT NULL DEFAULT 'stdio', \
         ADD COLUMN IF NOT EXISTS endpoint TEXT, \
         ADD COLUMN IF NOT EXISTS token_file TEXT, \
         ADD COLUMN IF NOT EXISTS read_profile TEXT, \
         ADD COLUMN IF NOT EXISTS target_repository TEXT, \
         ADD COLUMN IF NOT EXISTS assigned_work_id UUID, \
         ADD COLUMN IF NOT EXISTS max_calls_per_work INTEGER, \
         ADD COLUMN IF NOT EXISTS allowed_tools JSONB NOT NULL DEFAULT '[]'::jsonb, \
         ADD COLUMN IF NOT EXISTS observed_tools JSONB NOT NULL DEFAULT '[]'::jsonb, \
         ADD COLUMN IF NOT EXISTS tool_contract_digest TEXT, \
         ADD COLUMN IF NOT EXISTS policy_revision UUID NOT NULL DEFAULT gen_random_uuid(), \
         ADD COLUMN IF NOT EXISTS server_version TEXT, \
         ADD COLUMN IF NOT EXISTS last_observed_at TIMESTAMPTZ, \
         ADD COLUMN IF NOT EXISTS last_success_at TIMESTAMPTZ, \
         ADD COLUMN IF NOT EXISTS last_read_status TEXT, \
         ADD COLUMN IF NOT EXISTS last_read_site TEXT, \
         ADD COLUMN IF NOT EXISTS last_read_tool TEXT, \
         ADD COLUMN IF NOT EXISTS failure TEXT",
    )
    .execute(pool)
    .await
    .context("add broker-aware local MCP opt-in")?;
    sqlx::query(
        "DO $$ BEGIN \
           IF NOT EXISTS (SELECT 1 FROM pg_constraint \
             WHERE conname='local_mcp_max_calls_per_work_positive' \
               AND conrelid='restless_authority.local_mcp_servers'::regclass) THEN \
             ALTER TABLE restless_authority.local_mcp_servers \
               ADD CONSTRAINT local_mcp_max_calls_per_work_positive CHECK (max_calls_per_work > 0); \
           END IF; \
         END $$",
    )
    .execute(pool)
    .await
    .context("constrain broker read-call budget")?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.mcp_recurring_read_policies (\
           company TEXT NOT NULL, connection_name TEXT NOT NULL, schedule_id UUID NOT NULL, \
           responsibility_id UUID NOT NULL, responsibility_version INTEGER NOT NULL, \
           assigned_actor TEXT NOT NULL, enabled BOOLEAN NOT NULL DEFAULT FALSE, \
           allowed_tools JSONB NOT NULL, server_version TEXT NOT NULL, \
           tool_contract_digest TEXT NOT NULL, endpoint TEXT NOT NULL, \
           connection_policy_revision UUID NOT NULL, policy_revision UUID NOT NULL DEFAULT gen_random_uuid(), \
           approved_at TIMESTAMPTZ NOT NULL DEFAULT now(), revoked_at TIMESTAMPTZ, \
           PRIMARY KEY (company, connection_name, schedule_id)\
         )",
    )
    .execute(pool)
    .await
    .context("create recurring MCP read policies")?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.mcp_read_receipts (\
           id UUID PRIMARY KEY, call_id UUID NOT NULL, \
           phase TEXT NOT NULL CHECK (phase IN ('started','terminal')), \
           company TEXT NOT NULL, actor TEXT NOT NULL, \
           work_id UUID NOT NULL, attempt_id UUID NOT NULL, connection_name TEXT NOT NULL, \
           tool_name TEXT NOT NULL, tool_contract_digest TEXT NOT NULL, policy_revision UUID NOT NULL, \
           request_digest TEXT NOT NULL, result_digest TEXT, subject JSONB NOT NULL, \
           status TEXT NOT NULL CHECK (status IN ('started','complete','tool_error', \
             'response_observed_unverified','outcome_unknown','not_invoked')), \
           provider_status TEXT, \
           schedule_id UUID, opportunity_id UUID, responsibility_id UUID, \
           responsibility_version INTEGER, recurring_policy_revision UUID, \
           observed_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           wall_ms BIGINT, error_class TEXT, UNIQUE(call_id, phase)\
         )",
    )
    .execute(pool)
    .await
    .context("create Core MCP read receipts")?;
    sqlx::query(
        "ALTER TABLE restless_authority.mcp_read_receipts \
         ADD COLUMN IF NOT EXISTS provider_status TEXT, \
         ADD COLUMN IF NOT EXISTS schedule_id UUID, \
         ADD COLUMN IF NOT EXISTS opportunity_id UUID, \
         ADD COLUMN IF NOT EXISTS responsibility_id UUID, \
         ADD COLUMN IF NOT EXISTS responsibility_version INTEGER, \
         ADD COLUMN IF NOT EXISTS recurring_policy_revision UUID",
    )
    .execute(pool)
    .await
    .context("add normalized provider status to Core MCP read receipts")?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS mcp_read_receipts_scope_idx \
         ON restless_authority.mcp_read_receipts(company, connection_name, observed_at DESC)",
    )
    .execute(pool)
    .await
    .context("index Core MCP read receipts")?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS mcp_read_receipts_work_started_idx \
         ON restless_authority.mcp_read_receipts(company, connection_name, work_id) \
         WHERE phase='started'",
    )
    .execute(pool)
    .await
    .context("index Work broker read reservations")?;
    Ok(())
}

pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 48
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        bail!("connected-tool name must be a lowercase ASCII slug of at most 48 characters");
    }
    Ok(())
}

pub fn validate_endpoint(endpoint: &str) -> Result<()> {
    let parsed = url::Url::parse(endpoint).context("connected-tool endpoint must be a URL")?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() || parsed.fragment().is_some() {
        bail!("connected-tool endpoint must be an HTTPS URL without a fragment");
    }
    if parsed.username() != "" || parsed.password().is_some() {
        bail!("connected-tool endpoint must not contain credentials");
    }
    Ok(())
}

pub fn validate_local_server(
    name: &str,
    command: &str,
    args: &[String],
    actor: &str,
) -> Result<()> {
    validate_name(name)?;
    let path = Path::new(command);
    if !path.is_absolute()
        || !command.starts_with("/company/")
        || path.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        bail!("local MCP command must be an absolute path inside the company Runtime volume");
    }
    validate_actor(actor)?;
    if args.len() > 32
        || args
            .iter()
            .any(|arg| arg.len() > 2048 || arg.contains('\0'))
    {
        bail!("local MCP arguments exceed their bound");
    }
    Ok(())
}

fn validate_actor(actor: &str) -> Result<()> {
    if actor.is_empty()
        || actor.len() > 64
        || !actor.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
    {
        bail!("local MCP actor must be a lowercase ASCII actor id");
    }
    Ok(())
}

fn validate_max_calls_per_work(max_calls_per_work: Option<i32>, unlimited_read_calls: bool) -> Result<()> {
    if max_calls_per_work.is_some() && unlimited_read_calls {
        bail!("choose a finite MCP read-call limit or explicitly unlimited reads, not both");
    }
    if max_calls_per_work.is_some_and(|limit| !(1..=10_000).contains(&limit)) {
        bail!("MCP max calls per Work must be between 1 and 10000");
    }
    Ok(())
}

pub async fn local_mcp_list(pool: &PgPool, company: &str) -> Result<Vec<LocalMcpServer>> {
    sqlx::query(
        "SELECT name,transport,command,args,endpoint,token_file,read_profile,target_repository,assigned_actor,assigned_work_id, \
                broker_aware,enabled,allowed_tools,max_calls_per_work,observed_tools,tool_contract_digest,policy_revision,server_version, \
                last_observed_at,last_success_at,last_read_status,last_read_site,last_read_tool,failure FROM restless_authority.local_mcp_servers \
         WHERE company=$1 ORDER BY name",
    )
    .bind(company)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        Ok(LocalMcpServer {
            name: row.try_get("name")?,
            transport: row.try_get("transport")?,
            command: row.try_get("command")?,
            args: serde_json::from_value(row.try_get::<serde_json::Value, _>("args")?)?,
            endpoint: row.try_get("endpoint")?,
            token_file: row.try_get("token_file")?,
            read_profile: row.try_get("read_profile")?,
            target_repository: row.try_get("target_repository")?,
            assigned_actor: row.try_get("assigned_actor")?,
            assigned_work_id: row.try_get("assigned_work_id")?,
            broker_aware: row.try_get("broker_aware")?,
            enabled: row.try_get("enabled")?,
            allowed_tools: serde_json::from_value(row.try_get("allowed_tools")?)?,
            max_calls_per_work: row.try_get("max_calls_per_work")?,
            observed_tools: serde_json::from_value(row.try_get("observed_tools")?)?,
            tool_contract_digest: row.try_get("tool_contract_digest")?,
            policy_revision: row.try_get("policy_revision")?,
            server_version: row.try_get("server_version")?,
            last_observed_at: row.try_get("last_observed_at")?,
            last_success_at: row.try_get("last_success_at")?,
            last_read_status: row.try_get("last_read_status")?,
            last_read_site: row.try_get("last_read_site")?,
            last_read_tool: row.try_get("last_read_tool")?,
            failure: row.try_get("failure")?,
        })
    })
    .collect()
}

pub async fn recurring_mcp_policies(
    pool: &PgPool,
    company: &str,
    name: Option<&str>,
) -> Result<Vec<RecurringMcpPolicy>> {
    if let Some(name) = name {
        validate_name(name)?;
    }
    sqlx::query(
        "SELECT connection_name,schedule_id,responsibility_id,responsibility_version,assigned_actor, \
                enabled,allowed_tools,server_version,tool_contract_digest,endpoint, \
                connection_policy_revision,policy_revision,approved_at,revoked_at \
         FROM restless_authority.mcp_recurring_read_policies \
         WHERE company=$1 AND ($2::text IS NULL OR connection_name=$2) \
         ORDER BY connection_name,schedule_id",
    )
    .bind(company)
    .bind(name)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        Ok(RecurringMcpPolicy {
            connection_name: row.try_get("connection_name")?,
            schedule_id: row.try_get("schedule_id")?,
            responsibility_id: row.try_get("responsibility_id")?,
            responsibility_version: row.try_get("responsibility_version")?,
            assigned_actor: row.try_get("assigned_actor")?,
            enabled: row.try_get("enabled")?,
            allowed_tools: serde_json::from_value(row.try_get("allowed_tools")?)?,
            server_version: row.try_get("server_version")?,
            tool_contract_digest: row.try_get("tool_contract_digest")?,
            endpoint: row.try_get("endpoint")?,
            connection_policy_revision: row.try_get("connection_policy_revision")?,
            policy_revision: row.try_get("policy_revision")?,
            approved_at: row.try_get("approved_at")?,
            revoked_at: row.try_get("revoked_at")?,
        })
    })
    .collect()
}

pub fn reviewed_recurring_ch_connection(server: &LocalMcpServer) -> Result<()> {
    if !server.enabled
        || server.transport != "host_http"
        || server.read_profile.as_deref() != Some("clapping_hands_v1")
        || server.assigned_work_id.is_none()
        || !(server
            .allowed_tools
            .iter()
            .map(String::as_str)
            .eq(CLAPPING_HANDS_BATCH_READ_TOOLS)
            || server
                .allowed_tools
                .iter()
                .map(String::as_str)
                .eq(CLAPPING_HANDS_PHOTO_READ_TOOLS))
        || server.server_version.as_deref().is_none_or(str::is_empty)
        || server.tool_contract_digest.as_deref().is_none_or(str::is_empty)
    {
        bail!("recurring MCP permits only an enabled reviewed four- or five-tool Clapping Hands read profile");
    }
    reviewed_http_read_profile(server)?;
    Ok(())
}

pub fn recurring_policy_matches_connection(
    policy: &RecurringMcpPolicy,
    server: &LocalMcpServer,
) -> bool {
    policy.enabled
        && policy.connection_name == server.name
        && policy.assigned_actor == server.assigned_actor
        && policy.connection_policy_revision == server.policy_revision
        && policy.endpoint == server.endpoint.as_deref().unwrap_or_default()
        && policy.allowed_tools == server.allowed_tools
        && Some(policy.server_version.as_str()) == server.server_version.as_deref()
        && Some(policy.tool_contract_digest.as_str()) == server.tool_contract_digest.as_deref()
        && reviewed_recurring_ch_connection(server).is_ok()
}

pub async fn approve_recurring_ch_policy(
    pool: &PgPool,
    org: &OrgIntel,
    company: &str,
    name: &str,
    schedule_id: Uuid,
    responsibility_id: Uuid,
    responsibility_version: i32,
    actor: &str,
) -> Result<RecurringMcpPolicy> {
    validate_name(name)?;
    validate_actor(actor)?;
    let server = local_mcp_list(pool, company)
        .await?
        .into_iter()
        .find(|server| server.name == name)
        .context("reviewed CH connection not found")?;
    reviewed_recurring_ch_connection(&server)?;
    if server.assigned_actor != actor {
        bail!("recurring policy actor must match the existing CH connection actor");
    }
    let staff = org.active_actor(actor).await?.context("recurring policy Staff actor is inactive")?;
    if staff.kind != "staff" || staff.actor_class != "agent" {
        bail!("recurring MCP policy requires an active Staff actor");
    }
    let schedule = org.get_schedule(schedule_id).await?.context("recurring schedule not found")?;
    if schedule.cancelled_at.is_some()
        || schedule.recurrence.is_none()
        || schedule.work_id.is_some()
        || schedule.responsibility_id != Some(responsibility_id)
        || schedule.responsibility_version != Some(responsibility_version)
    {
        bail!("schedule is not an active recurring binding to that immutable responsibility version");
    }
    let endpoint = server.endpoint.as_deref().context("CH endpoint missing")?;
    let token_file = server.token_file.as_deref().context("CH host token missing")?;
    let observed = crate::mcp_gateway::probe_upstream(endpoint, token_file, &server.allowed_tools).await?;
    if Some(observed.server_version.as_str()) != server.server_version.as_deref()
        || Some(observed.digest.as_str()) != server.tool_contract_digest.as_deref()
    {
        bail!("CH tool contract changed; inspect and re-pin the connection before recurring approval");
    }
    let mut tx = pool.begin().await?;
    let still_pinned: Option<Uuid> = sqlx::query_scalar(
        "SELECT policy_revision FROM restless_authority.local_mcp_servers \
         WHERE company=$1 AND name=$2 AND enabled=TRUE AND transport='host_http' \
           AND read_profile='clapping_hands_v1' AND assigned_actor=$3 \
           AND endpoint=$4 AND token_file=$5 AND allowed_tools=$6 \
           AND server_version=$7 AND tool_contract_digest=$8 AND policy_revision=$9 \
         FOR SHARE",
    )
    .bind(company)
    .bind(name)
    .bind(actor)
    .bind(endpoint)
    .bind(token_file)
    .bind(serde_json::json!(server.allowed_tools))
    .bind(&observed.server_version)
    .bind(&observed.digest)
    .bind(server.policy_revision)
    .fetch_optional(&mut *tx)
    .await?;
    if still_pinned != Some(server.policy_revision) {
        bail!("CH connection changed during recurring approval");
    }
    sqlx::query(
        "INSERT INTO restless_authority.mcp_recurring_read_policies \
         (company,connection_name,schedule_id,responsibility_id,responsibility_version, \
          assigned_actor,enabled,allowed_tools,server_version,tool_contract_digest,endpoint, \
          connection_policy_revision,policy_revision,approved_at,revoked_at) \
         VALUES ($1,$2,$3,$4,$5,$6,TRUE,$7,$8,$9,$10,$11,gen_random_uuid(),now(),NULL) \
         ON CONFLICT (company,connection_name,schedule_id) DO UPDATE SET \
           responsibility_id=EXCLUDED.responsibility_id, \
           responsibility_version=EXCLUDED.responsibility_version, \
           assigned_actor=EXCLUDED.assigned_actor,enabled=TRUE, \
           allowed_tools=EXCLUDED.allowed_tools,server_version=EXCLUDED.server_version, \
           tool_contract_digest=EXCLUDED.tool_contract_digest,endpoint=EXCLUDED.endpoint, \
           connection_policy_revision=EXCLUDED.connection_policy_revision, \
           policy_revision=gen_random_uuid(),approved_at=now(),revoked_at=NULL",
    )
    .bind(company).bind(name).bind(schedule_id).bind(responsibility_id)
    .bind(responsibility_version).bind(actor)
    .bind(serde_json::json!(server.allowed_tools)).bind(&observed.server_version)
    .bind(&observed.digest).bind(endpoint).bind(server.policy_revision)
    .execute(&mut *tx).await?;
    tx.commit().await?;
    recurring_mcp_policies(pool, company, Some(name)).await?
        .into_iter().find(|policy| policy.schedule_id == schedule_id)
        .context("approved recurring policy disappeared")
}

pub async fn revoke_recurring_ch_policy(
    pool: &PgPool,
    company: &str,
    name: &str,
    schedule_id: Uuid,
) -> Result<RecurringMcpPolicy> {
    validate_name(name)?;
    let affected = sqlx::query(
        "UPDATE restless_authority.mcp_recurring_read_policies \
         SET enabled=FALSE,policy_revision=gen_random_uuid(),revoked_at=now() \
         WHERE company=$1 AND connection_name=$2 AND schedule_id=$3",
    )
    .bind(company).bind(name).bind(schedule_id).execute(pool).await?.rows_affected();
    if affected != 1 {
        bail!("recurring CH policy not found");
    }
    recurring_mcp_policies(pool, company, Some(name)).await?
        .into_iter().find(|policy| policy.schedule_id == schedule_id)
        .context("revoked recurring policy disappeared")
}

/// Installing or moving an MCP assignment requires a fresh, non-running Work
/// owned by the actor who will receive the next Attempt grant.
pub async fn validate_assignable_mcp_work(
    org: &OrgIntel,
    actor: &str,
    work_id: Uuid,
) -> Result<()> {
    if org.active_actor(actor).await?.is_none() {
        bail!("local MCP actor {actor:?} is not active");
    }
    let work = org.get_work(work_id).await?.context("local MCP Work not found")?;
    if work.owner_id != actor
        || !matches!(work.status, restless_orgintel::WorkStatus::Proposed | restless_orgintel::WorkStatus::Blocked)
    {
        bail!("local MCP installation requires proposed or blocked Work owned by the assigned actor");
    }
    if org.list_running_work_attempts().await?.iter().any(|attempt| attempt.work_id == work_id) {
        bail!("interrupt the running Work Attempt before changing its MCP tools");
    }
    Ok(())
}

pub async fn list_mcp_read_receipts(
    pool: &PgPool,
    company: &str,
    name: Option<&str>,
) -> Result<Vec<McpReadReceipt>> {
    if let Some(name) = name {
        validate_name(name)?;
    }
    sqlx::query(
        "SELECT call_id,phase,actor,work_id,attempt_id,connection_name,tool_name, \
                tool_contract_digest,policy_revision,request_digest,result_digest,subject,status,provider_status,observed_at, \
                wall_ms,error_class,schedule_id,opportunity_id,responsibility_id,responsibility_version,recurring_policy_revision \
         FROM restless_authority.mcp_read_receipts \
         WHERE company=$1 AND ($2::text IS NULL OR connection_name=$2) \
         ORDER BY observed_at DESC LIMIT 100",
    )
    .bind(company).bind(name).fetch_all(pool).await?
    .into_iter().map(|row| Ok(McpReadReceipt {
        call_id: row.try_get("call_id")?, phase: row.try_get("phase")?,
        actor: row.try_get("actor")?, work_id: row.try_get("work_id")?,
        attempt_id: row.try_get("attempt_id")?,
        connection_name: row.try_get("connection_name")?,
        tool_name: row.try_get("tool_name")?,
        tool_contract_digest: row.try_get("tool_contract_digest")?,
        policy_revision: row.try_get("policy_revision")?,
        request_digest: row.try_get("request_digest")?,
        result_digest: row.try_get("result_digest")?,
        subject: row.try_get("subject")?, status: row.try_get("status")?,
        provider_status: row.try_get("provider_status")?,
        observed_at: row.try_get("observed_at")?, wall_ms: row.try_get("wall_ms")?,
        error_class: row.try_get("error_class")?,
        schedule_id: row.try_get("schedule_id")?,
        opportunity_id: row.try_get("opportunity_id")?,
        responsibility_id: row.try_get("responsibility_id")?,
        responsibility_version: row.try_get("responsibility_version")?,
        recurring_policy_revision: row.try_get("recurring_policy_revision")?,
    })).collect()
}

pub async fn install_local_mcp(
    pool: &PgPool,
    company: &str,
    name: &str,
    command: &str,
    args: &[String],
    assigned_actor: &str,
    work_id: Uuid,
    broker_aware: bool,
) -> Result<LocalMcpServer> {
    validate_local_server(name, command, args, assigned_actor)?;
    let provider_name_in_use: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.provider_connections \
         WHERE company=$1 AND name=$2 AND status <> 'disabled')",
    )
    .bind(company)
    .bind(name)
    .fetch_one(pool)
    .await?;
    if provider_name_in_use {
        bail!(
            "MCP server name {name:?} is already used by an enabled or pending provider connection"
        );
    }
    let args = serde_json::to_value(args)?;
    sqlx::query(
        "INSERT INTO restless_authority.local_mcp_servers \
           (company,name,transport,command,args,assigned_actor,assigned_work_id,broker_aware,enabled,created_by) \
         VALUES ($1,$2,'stdio',$3,$4,$5,$6,$7,TRUE,'owner') \
         ON CONFLICT (company,name) DO UPDATE SET command=EXCLUDED.command,args=EXCLUDED.args, \
           transport='stdio',endpoint=NULL,token_file=NULL,read_profile=NULL,target_repository=NULL,assigned_actor=EXCLUDED.assigned_actor, \
           assigned_work_id=EXCLUDED.assigned_work_id,broker_aware=EXCLUDED.broker_aware, \
           enabled=TRUE,allowed_tools='[]'::jsonb,max_calls_per_work=NULL,observed_tools='[]'::jsonb, \
           tool_contract_digest=NULL,policy_revision=gen_random_uuid(),server_version=NULL,last_observed_at=NULL,last_success_at=NULL,last_read_status=NULL, \
           failure=NULL,updated_at=now()",
    )
    .bind(company)
    .bind(name)
    .bind(command)
    .bind(args)
    .bind(assigned_actor)
    .bind(work_id)
    .bind(broker_aware)
    .execute(pool)
    .await?;
    local_mcp_list(pool, company)
        .await?
        .into_iter()
        .find(|server| server.name == name)
        .context("installed local MCP descriptor disappeared")
}

/// A host-owned service keeps its browser and bearer outside the company
/// Runtime. Only its exact, observed tool names are exposed by the gateway.
#[expect(
    clippy::too_many_arguments,
    reason = "one owner-approved local MCP grant"
)]
pub async fn install_host_mcp(
    pool: &PgPool,
    company: &str,
    name: &str,
    endpoint: &str,
    token_file: &str,
    actor: &str,
    work_id: Uuid,
    allowed_tools: &[String],
    max_calls_per_work: Option<i32>,
    unlimited_read_calls: bool,
    expected_policy_revision: Option<Uuid>,
) -> Result<LocalMcpServer> {
    validate_name(name)?;
    validate_max_calls_per_work(max_calls_per_work, unlimited_read_calls)?;
    validate_host_endpoint(endpoint)?;
    if allowed_tools.is_empty() || allowed_tools.len() > 16 {
        bail!("host MCP requires 1-16 explicitly permitted tools");
    }
    let mut allowed = allowed_tools.to_vec();
    allowed.sort();
    allowed.dedup();
    if allowed.len() != allowed_tools.len()
        || allowed.iter().any(|name| {
            name.is_empty() || name.len() > 128 || name.chars().any(char::is_whitespace)
        })
    {
        bail!("host MCP tool allowlist has a duplicate or invalid name");
    }
    require_reviewed_host_read_profile(name, endpoint, &allowed)?;
    let provider_name_in_use: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.provider_connections \
         WHERE company=$1 AND name=$2 AND status <> 'disabled')",
    )
    .bind(company)
    .bind(name)
    .fetch_one(pool)
    .await?;
    if provider_name_in_use {
        bail!("MCP name {name:?} is already used by a provider connection");
    }
    let probe = crate::mcp_gateway::probe_upstream(endpoint, token_file, &allowed).await?;
    let updated = sqlx::query(
        "INSERT INTO restless_authority.local_mcp_servers \
           (company,name,transport,command,args,endpoint,token_file,read_profile,assigned_actor,assigned_work_id, \
            broker_aware,enabled,allowed_tools,max_calls_per_work,observed_tools,tool_contract_digest,server_version,last_observed_at,created_by) \
         VALUES ($1,$2,'host_http','','[]'::jsonb,$3,$4,'clapping_hands_v1',$5,$6,FALSE,TRUE,$7,$8,$9,$10,$11,now(),'owner') \
         ON CONFLICT (company,name) DO UPDATE SET transport='host_http',command='',args='[]'::jsonb, \
           endpoint=EXCLUDED.endpoint,token_file=EXCLUDED.token_file,read_profile='clapping_hands_v1',target_repository=NULL,assigned_actor=EXCLUDED.assigned_actor, \
           assigned_work_id=EXCLUDED.assigned_work_id,broker_aware=FALSE,enabled=TRUE, \
           allowed_tools=EXCLUDED.allowed_tools, \
           max_calls_per_work=CASE WHEN $13 THEN NULL ELSE COALESCE(EXCLUDED.max_calls_per_work,local_mcp_servers.max_calls_per_work) END, \
           observed_tools=EXCLUDED.observed_tools, \
           tool_contract_digest=EXCLUDED.tool_contract_digest,policy_revision=gen_random_uuid(),server_version=EXCLUDED.server_version,last_observed_at=now(), \
           last_success_at=NULL,last_read_status=NULL,last_read_site=NULL,last_read_tool=NULL,failure=NULL,updated_at=now() \
         WHERE $12::uuid IS NULL OR local_mcp_servers.policy_revision=$12",
    )
    .bind(company).bind(name).bind(endpoint).bind(token_file).bind(actor).bind(work_id)
    .bind(serde_json::to_value(&allowed)?)
    .bind(max_calls_per_work)
    .bind(serde_json::to_value(&probe.names)?)
    .bind(&probe.digest)
    .bind(&probe.server_version)
    .bind(expected_policy_revision)
    .bind(unlimited_read_calls)
    .execute(pool).await?.rows_affected();
    if updated != 1 {
        bail!("MCP connection changed during re-probe; refresh before retrying");
    }
    local_mcp_list(pool, company)
        .await?
        .into_iter()
        .find(|server| server.name == name)
        .context("installed host MCP disappeared")
}

/// A single filesystem MCP read tool, launched by Core inside a
/// networkless, read-only bubblewrap worker. No company Runtime path or
/// provider credential is passed to the actor.
pub async fn install_brokered_stdio_mcp(
    pool: &PgPool,
    company: &str,
    name: &str,
    bundle: &str,
    read_root: &str,
    actor: &str,
    work_id: Uuid,
    max_calls_per_work: Option<i32>,
    unlimited_read_calls: bool,
) -> Result<LocalMcpServer> {
    validate_name(name)?;
    validate_actor(actor)?;
    validate_max_calls_per_work(max_calls_per_work, unlimited_read_calls)?;
    if matches!(name, "clapping-hands" | "deepwiki") {
        bail!("this MCP name is reserved for a reviewed HTTP profile");
    }
    let (bundle, read_root) = crate::stdio_mcp::validate_profile(bundle, read_root)?;
    let bundle = bundle.to_string_lossy().to_string();
    let read_root = read_root.to_string_lossy().to_string();
    let provider_name_in_use: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.provider_connections \
         WHERE company=$1 AND name=$2 AND status <> 'disabled')",
    )
    .bind(company).bind(name).fetch_one(pool).await?;
    if provider_name_in_use {
        bail!("MCP name {name:?} is already used by a provider connection");
    }
    let probe = crate::stdio_mcp::probe(&bundle, &read_root).await?;
    let allowed = vec![crate::stdio_mcp::READ_TOOL.to_string()];
    sqlx::query(
        "INSERT INTO restless_authority.local_mcp_servers \
           (company,name,transport,command,args,endpoint,token_file,read_profile,target_repository,assigned_actor,assigned_work_id, \
            broker_aware,enabled,allowed_tools,max_calls_per_work,observed_tools,tool_contract_digest,server_version,last_observed_at,created_by) \
         VALUES ($1,$2,'broker_stdio',$3,$4,NULL,NULL,'filesystem_read_v1',NULL,$5,$6,FALSE,TRUE,$7,$8,$9,$10,$11,now(),'owner') \
         ON CONFLICT (company,name) DO UPDATE SET transport='broker_stdio',command=EXCLUDED.command,args=EXCLUDED.args, \
           endpoint=NULL,token_file=NULL,read_profile='filesystem_read_v1',target_repository=NULL,assigned_actor=EXCLUDED.assigned_actor, \
           assigned_work_id=EXCLUDED.assigned_work_id,broker_aware=FALSE,enabled=TRUE, \
           allowed_tools=EXCLUDED.allowed_tools, \
           max_calls_per_work=CASE WHEN $12 THEN NULL ELSE COALESCE(EXCLUDED.max_calls_per_work,local_mcp_servers.max_calls_per_work) END, \
           observed_tools=EXCLUDED.observed_tools, \
           tool_contract_digest=EXCLUDED.tool_contract_digest,policy_revision=gen_random_uuid(),server_version=EXCLUDED.server_version,last_observed_at=now(), \
           last_success_at=NULL,last_read_status=NULL,last_read_site=NULL,last_read_tool=NULL,failure=NULL,updated_at=now()",
    )
    .bind(company).bind(name).bind(&bundle).bind(serde_json::json!([read_root]))
    .bind(actor).bind(work_id).bind(serde_json::to_value(&allowed)?)
    .bind(max_calls_per_work)
    .bind(serde_json::to_value(&probe.names)?).bind(&probe.digest).bind(&probe.server_version)
    .bind(unlimited_read_calls)
    .execute(pool).await?;
    local_mcp_list(pool, company).await?.into_iter()
        .find(|server| server.name == name)
        .context("installed brokered stdio MCP disappeared")
}

pub fn require_reviewed_stdio_profile(server: &LocalMcpServer) -> Result<&str> {
    if server.transport != "broker_stdio"
        || server.endpoint.is_some()
        || server.token_file.is_some()
        || server.read_profile.as_deref() != Some("filesystem_read_v1")
        || server.target_repository.is_some()
        || server.args.len() != 1
        || server.allowed_tools != [crate::stdio_mcp::READ_TOOL]
    {
        bail!("stdio MCP is outside the reviewed filesystem read profile");
    }
    let read_root = server.args[0].as_str();
    crate::stdio_mcp::validate_profile(&server.command, read_root)?;
    Ok(read_root)
}

pub fn validate_host_endpoint(endpoint: &str) -> Result<()> {
    let url = url::Url::parse(endpoint).context("host MCP endpoint must be a URL")?;
    if url.scheme() != "http"
        || url.host_str() != Some("127.0.0.1")
        || url.port().is_none()
        || url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("host MCP endpoint must be explicit http://127.0.0.1:<port>/... without credentials");
    }
    Ok(())
}

/// Generic MCP tool metadata cannot prove an operation has no external effect.
/// Until there are per-tool effect adapters, the host bridge exposes only this
/// reviewed local sourcing surface. The owner controls the loopback service.
pub fn require_reviewed_host_read_profile(
    name: &str,
    endpoint: &str,
    allowed_tools: &[String],
) -> Result<()> {
    validate_host_endpoint(endpoint)?;
    let port = crate::port_with_offset(CLAPPING_HANDS_SOURCING_PORT)?;
    let expected_endpoint = format!("http://127.0.0.1:{port}/mcp");
    if name != "clapping-hands"
        || endpoint != expected_endpoint
        || !(allowed_tools.iter().map(String::as_str).eq(CLAPPING_HANDS_READ_TOOLS)
            || allowed_tools.iter().map(String::as_str).eq(CLAPPING_HANDS_BATCH_READ_TOOLS)
            || allowed_tools.iter().map(String::as_str).eq(CLAPPING_HANDS_PHOTO_READ_TOOLS))
    {
        bail!("host MCP requires the reviewed Clapping Hands sourcing read profile; other tools need an external-effect adapter");
    }
    Ok(())
}

/// Reviewed profiles are the policy boundary for HTTP MCP. A new endpoint or
/// tool needs an explicit input validator in the Core gateway before it can be
/// installed; an MCP read-only annotation is not authority.
#[derive(Debug, Clone)]
pub enum ReviewedHttpReadProfile {
    ClappingHands,
    DeepWikiStructure { repository: String },
}

pub fn validate_public_repository(repository: &str) -> Result<()> {
    let Some((owner, repo)) = repository.split_once('/') else {
        bail!("public repository must be owner/repo");
    };
    let valid_part = |part: &str| {
        !part.is_empty()
            && part.len() <= 100
            && part != "."
            && part != ".."
            && part.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
            })
    };
    if !valid_part(owner) || !valid_part(repo) {
        bail!("public repository must be one bounded owner/repo target");
    }
    Ok(())
}

pub fn require_reviewed_public_read_profile(
    profile: &str,
    name: &str,
    endpoint: &str,
    allowed_tools: &[String],
    repository: &str,
) -> Result<ReviewedHttpReadProfile> {
    validate_public_repository(repository)?;
    if profile != "deepwiki_structure_v1"
        || name != "deepwiki"
        || endpoint != DEEPWIKI_ENDPOINT
        || allowed_tools.len() != 1
        || allowed_tools[0] != DEEPWIKI_READ_TOOL
    {
        bail!("public MCP currently permits only DeepWiki read_wiki_structure at its reviewed endpoint");
    }
    Ok(ReviewedHttpReadProfile::DeepWikiStructure {
        repository: repository.to_string(),
    })
}

pub fn reviewed_http_read_profile(server: &LocalMcpServer) -> Result<ReviewedHttpReadProfile> {
    let endpoint = server.endpoint.as_deref().context("HTTP MCP endpoint missing")?;
    match (server.transport.as_str(), server.read_profile.as_deref()) {
        ("host_http", None | Some("clapping_hands_v1")) => {
            if server.target_repository.is_some() || server.token_file.is_none() {
                bail!("Clapping Hands broker is missing its host-only bearer");
            }
            require_reviewed_host_read_profile(&server.name, endpoint, &server.allowed_tools)?;
            Ok(ReviewedHttpReadProfile::ClappingHands)
        }
        ("public_http", Some(profile)) => {
            if server.token_file.is_some() {
                bail!("public MCP profile cannot carry a credential");
            }
            require_reviewed_public_read_profile(
                profile,
                &server.name,
                endpoint,
                &server.allowed_tools,
                server.target_repository.as_deref().context("public MCP repository missing")?,
            )
        }
        _ => bail!("HTTP MCP connection has no reviewed read profile"),
    }
}

/// Owner-selected public Streamable HTTP read. The live provider is probed for
/// its exact tool definition and the selected public repository before grant.
#[expect(clippy::too_many_arguments, reason = "one owner-reviewed HTTP MCP grant")]
pub async fn install_public_http_read(
    pool: &PgPool,
    company: &str,
    profile: &str,
    name: &str,
    endpoint: &str,
    repository: &str,
    actor: &str,
    work_id: Uuid,
    allowed_tools: &[String],
    max_calls_per_work: Option<i32>,
    unlimited_read_calls: bool,
) -> Result<LocalMcpServer> {
    validate_name(name)?;
    validate_max_calls_per_work(max_calls_per_work, unlimited_read_calls)?;
    let reviewed = require_reviewed_public_read_profile(
        profile, name, endpoint, allowed_tools, repository,
    )?;
    let provider_name_in_use: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.provider_connections \
         WHERE company=$1 AND name=$2 AND status <> 'disabled')",
    )
    .bind(company).bind(name).fetch_one(pool).await?;
    if provider_name_in_use {
        bail!("MCP name {name:?} is already used by a provider connection");
    }
    let probe = crate::mcp_gateway::probe_public_upstream(endpoint, allowed_tools, &reviewed).await?;
    sqlx::query(
        "INSERT INTO restless_authority.local_mcp_servers \
         (company,name,transport,command,args,endpoint,token_file,read_profile,target_repository, \
          assigned_actor,assigned_work_id,broker_aware,enabled,allowed_tools,max_calls_per_work,observed_tools, \
          tool_contract_digest,server_version,last_observed_at,created_by) \
         VALUES ($1,$2,'public_http','','[]'::jsonb,$3,NULL,$4,$5,$6,$7,FALSE,TRUE,$8,$9,$10,$11,$12,now(),'owner') \
         ON CONFLICT (company,name) DO UPDATE SET transport='public_http',command='',args='[]'::jsonb, \
          endpoint=EXCLUDED.endpoint,token_file=NULL,read_profile=EXCLUDED.read_profile, \
          target_repository=EXCLUDED.target_repository,assigned_actor=EXCLUDED.assigned_actor, \
          assigned_work_id=EXCLUDED.assigned_work_id,broker_aware=FALSE,enabled=TRUE, \
          allowed_tools=EXCLUDED.allowed_tools, \
          max_calls_per_work=CASE WHEN $13 THEN NULL ELSE COALESCE(EXCLUDED.max_calls_per_work,local_mcp_servers.max_calls_per_work) END, \
          observed_tools=EXCLUDED.observed_tools, \
          tool_contract_digest=EXCLUDED.tool_contract_digest,policy_revision=gen_random_uuid(),server_version=EXCLUDED.server_version, \
          last_observed_at=now(),last_success_at=NULL,last_read_status=NULL,last_read_site=NULL, \
          last_read_tool=NULL,failure=NULL,updated_at=now()",
    )
    .bind(company).bind(name).bind(endpoint).bind(profile).bind(repository)
    .bind(actor).bind(work_id).bind(serde_json::to_value(allowed_tools)?)
    .bind(max_calls_per_work)
    .bind(serde_json::to_value(&probe.names)?).bind(&probe.digest).bind(&probe.server_version)
    .bind(unlimited_read_calls)
    .execute(pool).await?;
    local_mcp_list(pool, company).await?.into_iter()
        .find(|server| server.name == name)
        .context("installed public MCP disappeared")
}

pub async fn disable_local_mcp(
    pool: &PgPool,
    company: &str,
    name: &str,
) -> Result<LocalMcpServer> {
    validate_name(name)?;
    let updated = sqlx::query(
        "UPDATE restless_authority.local_mcp_servers SET enabled=FALSE,policy_revision=gen_random_uuid(),updated_at=now() \
         WHERE company=$1 AND name=$2",
    )
    .bind(company)
    .bind(name)
    .execute(pool)
    .await?
    .rows_affected();
    if updated == 0 {
        bail!("local MCP server {name:?} is not configured for company {company:?}");
    }
    local_mcp_list(pool, company)
        .await?
        .into_iter()
        .find(|server| server.name == name)
        .context("disabled local MCP descriptor disappeared")
}

pub fn runtime_credential_dir(name: &str) -> Result<String> {
    validate_name(name)?;
    Ok(format!("{RUNTIME_CREDENTIAL_ROOT}/{name}"))
}

pub fn host_credential_dir(
    root: &Path,
    company: &str,
    name: &str,
) -> Result<std::path::PathBuf> {
    crate::runtime::validate_company_name(company)?;
    validate_name(name)?;
    Ok(root.join("connected-tools").join(company).join(name))
}

struct CredentialReplacement {
    active: std::path::PathBuf,
    backup: Option<std::path::PathBuf>,
    committed: bool,
}

impl CredentialReplacement {
    fn prepare(active: &Path, force: bool) -> Result<Self> {
        recover_interrupted_reconnect(active)?;
        let backup = force.then(|| {
            active.with_file_name(format!(
                "{}.reconnect-backup",
                active
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("connection")
            ))
        });
        if let Some(backup) = &backup {
            if active.exists() {
                std::fs::rename(active, backup).with_context(|| {
                    format!(
                        "preserve working connected-tool credentials {} before reconnect",
                        active.display()
                    )
                })?;
            }
        }
        Ok(Self {
            active: active.to_path_buf(),
            backup,
            committed: false,
        })
    }

    fn commit(&mut self) -> Result<()> {
        if let Some(backup) = &self.backup {
            if backup.exists() {
                std::fs::remove_dir_all(backup).with_context(|| {
                    format!("remove superseded credential backup {}", backup.display())
                })?;
            }
        }
        self.committed = true;
        Ok(())
    }
}

impl Drop for CredentialReplacement {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        let Some(backup) = &self.backup else {
            return;
        };
        if !backup.exists() {
            return;
        }
        if self.active.exists() {
            let _ = std::fs::remove_dir_all(&self.active);
        }
        let _ = std::fs::rename(backup, &self.active);
    }
}

fn recover_interrupted_reconnect(active: &Path) -> Result<()> {
    let backup = active.with_file_name(format!(
        "{}.reconnect-backup",
        active
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("connection")
    ));
    if !backup.exists() {
        return Ok(());
    }
    if cached_token_material_exists(active)? {
        std::fs::remove_dir_all(&backup)
            .with_context(|| format!("remove stale reconnect backup {}", backup.display()))?;
        return Ok(());
    }
    if active.exists() {
        std::fs::remove_dir_all(active).with_context(|| {
            format!(
                "remove incomplete reconnect credentials {}",
                active.display()
            )
        })?;
    }
    std::fs::rename(&backup, active).with_context(|| {
        format!(
            "restore connected-tool credentials after interrupted reconnect {}",
            active.display()
        )
    })?;
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "one provider-neutral connection observation"
)]
pub async fn stage(
    pool: &PgPool,
    company: &str,
    name: &str,
    endpoint: &str,
    purpose: &str,
    assigned_actor: &str,
    requested_scopes: &[String],
    owner_handoff_id: &str,
    created_by: &str,
    work_id: Uuid,
    attempt_id: Uuid,
) -> Result<ConnectedTool> {
    validate_name(name)?;
    validate_endpoint(endpoint)?;
    if purpose.trim().is_empty() || assigned_actor.trim().is_empty() || created_by.trim().is_empty()
    {
        bail!("connected-tool purpose, assigned actor and requester must be non-empty");
    }
    let credential_reference = format!("runtime-scoped:{}", runtime_credential_dir(name)?);
    sqlx::query(
        "INSERT INTO restless_authority.provider_connections \
         (company,name,endpoint,purpose,assigned_actor,status,credential_reference,requested_scopes,owner_handoff_id,created_by,assigned_work_id,assigned_attempt_id) \
         VALUES ($1,$2,$3,$4,$5,'awaiting_owner',$6,$7,$8,$9,$10,$11) \
         ON CONFLICT (company,name) DO UPDATE SET endpoint=EXCLUDED.endpoint, purpose=EXCLUDED.purpose, \
         assigned_actor=EXCLUDED.assigned_actor, status='awaiting_owner', \
         credential_reference=EXCLUDED.credential_reference, requested_scopes=EXCLUDED.requested_scopes, \
         assigned_work_id=EXCLUDED.assigned_work_id, assigned_attempt_id=EXCLUDED.assigned_attempt_id, \
         observed_tools='[]'::jsonb, workspace_reference=NULL, owner_handoff_id=EXCLUDED.owner_handoff_id, \
         last_observed_at=NULL, failure=NULL, updated_at=now()",
    )
    .bind(company)
    .bind(name)
    .bind(endpoint)
    .bind(purpose.trim())
    .bind(assigned_actor.trim())
    .bind(&credential_reference)
    .bind(serde_json::to_value(requested_scopes)?)
    .bind(owner_handoff_id)
    .bind(created_by.trim())
    .bind(work_id)
    .bind(attempt_id)
    .execute(pool)
    .await
    .context("stage provider connection")?;
    get(pool, company, name)
        .await?
        .context("staged provider connection disappeared")
}

pub async fn enable(
    pool: &PgPool,
    company: &str,
    name: &str,
    observed_tools: &[String],
    workspace_reference: Option<&str>,
) -> Result<ConnectedTool> {
    if observed_tools.is_empty() {
        bail!("a connected tool cannot be enabled without an observed MCP tool list");
    }
    let updated = sqlx::query(
        "UPDATE restless_authority.provider_connections SET status='enabled', observed_tools=$3, \
         workspace_reference=$4, last_observed_at=now(), failure=NULL, updated_at=now() \
         WHERE company=$1 AND name=$2",
    )
    .bind(company)
    .bind(name)
    .bind(serde_json::to_value(observed_tools)?)
    .bind(workspace_reference)
    .execute(pool)
    .await?
    .rows_affected();
    if updated != 1 {
        bail!("no staged connected tool {name:?} for {company}");
    }
    get(pool, company, name)
        .await?
        .context("enabled provider connection disappeared")
}

async fn bind_execution_scope(
    pool: &PgPool,
    company: &str,
    name: &str,
    purpose: &str,
    assigned_actor: &str,
    work_id: Uuid,
    attempt_id: Uuid,
) -> Result<()> {
    let updated = sqlx::query(
        "UPDATE restless_authority.provider_connections \
         SET purpose=$3, assigned_actor=$4, assigned_work_id=$5, assigned_attempt_id=$6, \
             updated_at=now() WHERE company=$1 AND name=$2",
    )
    .bind(company)
    .bind(name)
    .bind(purpose.trim())
    .bind(assigned_actor.trim())
    .bind(work_id)
    .bind(attempt_id)
    .execute(pool)
    .await?
    .rows_affected();
    if updated != 1 {
        bail!("no connected tool {name:?} for {company}");
    }
    Ok(())
}

pub async fn fail(pool: &PgPool, company: &str, name: &str, failure: &str) -> Result<()> {
    sqlx::query(
        "UPDATE restless_authority.provider_connections SET status='failed', failure=$3, \
         updated_at=now() WHERE company=$1 AND name=$2",
    )
    .bind(company)
    .bind(name)
    .bind(failure.chars().take(2_000).collect::<String>())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn disable(pool: &PgPool, company: &str, name: &str) -> Result<ConnectedTool> {
    let updated = sqlx::query(
        "UPDATE restless_authority.provider_connections SET status='disabled', updated_at=now() \
         WHERE company=$1 AND name=$2",
    )
    .bind(company)
    .bind(name)
    .execute(pool)
    .await?
    .rows_affected();
    if updated != 1 {
        bail!("no connected tool {name:?} for {company}");
    }
    get(pool, company, name)
        .await?
        .context("disabled provider connection disappeared")
}

pub async fn observe_workspace(
    pool: &PgPool,
    company: &str,
    name: &str,
    actor: &str,
    workspace_reference: &str,
    observed_tools: &[String],
) -> Result<ConnectedTool> {
    if workspace_reference.trim().is_empty() || observed_tools.is_empty() {
        bail!("workspace observation needs an identity reference and at least one live tool");
    }
    let connection = get(pool, company, name)
        .await?
        .with_context(|| format!("no connected tool {name:?} for {company}"))?;
    if connection.status != ConnectionStatus::Enabled {
        bail!("connected tool {name:?} is not enabled");
    }
    if connection.assigned_actor != actor {
        bail!(
            "connected tool {name:?} is assigned to {:?}, not {actor:?}",
            connection.assigned_actor
        );
    }
    let mut tools = observed_tools.to_vec();
    tools.sort();
    tools.dedup();
    sqlx::query(
        "UPDATE restless_authority.provider_connections SET observed_tools=$3, \
         workspace_reference=$4, last_observed_at=now(), updated_at=now() \
         WHERE company=$1 AND name=$2",
    )
    .bind(company)
    .bind(name)
    .bind(serde_json::to_value(tools)?)
    .bind(workspace_reference.trim())
    .execute(pool)
    .await?;
    get(pool, company, name)
        .await?
        .context("observed provider connection disappeared")
}

pub async fn get(pool: &PgPool, company: &str, name: &str) -> Result<Option<ConnectedTool>> {
    validate_name(name)?;
    let row = sqlx::query(
        "SELECT name,endpoint,purpose,assigned_actor,assigned_work_id,assigned_attempt_id,status,credential_reference,requested_scopes, \
         observed_tools,workspace_reference,owner_handoff_id,last_observed_at,failure \
         FROM restless_authority.provider_connections WHERE company=$1 AND name=$2",
    )
    .bind(company)
    .bind(name)
    .fetch_optional(pool)
    .await?;
    row.map(decode_row).transpose()
}

pub async fn list(pool: &PgPool, company: &str) -> Result<Vec<ConnectedTool>> {
    sqlx::query(
        "SELECT name,endpoint,purpose,assigned_actor,assigned_work_id,assigned_attempt_id,status,credential_reference,requested_scopes, \
         observed_tools,workspace_reference,owner_handoff_id,last_observed_at,failure \
         FROM restless_authority.provider_connections WHERE company=$1 ORDER BY name",
    )
    .bind(company)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(decode_row)
    .collect()
}

fn decode_row(row: sqlx::postgres::PgRow) -> Result<ConnectedTool> {
    Ok(ConnectedTool {
        name: row.try_get("name")?,
        endpoint: row.try_get("endpoint")?,
        purpose: row.try_get("purpose")?,
        assigned_actor: row.try_get("assigned_actor")?,
        assigned_work_id: row.try_get("assigned_work_id")?,
        assigned_attempt_id: row.try_get("assigned_attempt_id")?,
        status: ConnectionStatus::parse(row.try_get::<String, _>("status")?.as_str())?,
        credential_reference: row.try_get("credential_reference")?,
        requested_scopes: serde_json::from_value(row.try_get("requested_scopes")?)?,
        observed_tools: serde_json::from_value(row.try_get("observed_tools")?)?,
        workspace_reference: row.try_get("workspace_reference")?,
        owner_handoff_id: row.try_get("owner_handoff_id")?,
        last_observed_at: row.try_get("last_observed_at")?,
        failure: row.try_get("failure")?,
    })
}

/// Operational recovery, not provider consent. Keep the Work blocked until
/// the lead explicitly resumes it, so no worker races the attachment.
pub async fn attach_existing(
    pool: &PgPool,
    org: &OrgIntel,
    company: &str,
    name: &str,
    work_id: Uuid,
    requested_by: &str,
) -> Result<ConnectedTool> {
    let work = org.get_work(work_id).await?.context("Work not found")?;
    if work.status != restless_orgintel::WorkStatus::Blocked {
        bail!(
            "attach requires blocked Work; interrupt active Work before changing its tools, then resume last"
        );
    }
    let existing = get(pool, company, name)
        .await?
        .context("connection not installed; installation needs separate provider consent")?;
    if existing.status != ConnectionStatus::Enabled {
        bail!("connection is not enabled; attach cannot enable a disabled or failed grant");
    }
    if org
        .list_running_work_attempts()
        .await?
        .iter()
        .any(|attempt| {
            Some(attempt.work_id) == existing.assigned_work_id || attempt.work_id == work_id
        })
    {
        bail!(
            "connection has a running source or target Attempt; finish or interrupt it before attachment"
        );
    }
    let updated = sqlx::query(
        "UPDATE restless_authority.provider_connections SET assigned_actor=$3, assigned_work_id=$4, \
         assigned_attempt_id=NULL, updated_at=now() WHERE company=$1 AND name=$2 AND status='enabled' \
         AND assigned_work_id IS NOT DISTINCT FROM $5 AND assigned_actor=$6",
    )
    .bind(company).bind(name).bind(&work.owner_id).bind(work_id)
    .bind(existing.assigned_work_id).bind(&existing.assigned_actor)
    .execute(pool).await?.rows_affected();
    if updated != 1 {
        bail!(
            "connection changed during attachment; inspect the current connection before retrying"
        );
    }
    org.emit_event(
        "provider_connection_attached",
        Some(requested_by),
        serde_json::json!({
            "name": name, "work_id": work_id, "assigned_actor": work.owner_id,
            "previous_work_id": existing.assigned_work_id, "requires_fresh_attempt": true,
            "provider_authority_changed": false,
        }),
    )
    .await?;
    get(pool, company, name)
        .await?
        .context("attached connection disappeared")
}

pub async fn session_servers(
    pool: &PgPool,
    org: &OrgIntel,
    capabilities: &crate::capability::CapabilityIssuer,
    company: &str,
    actor: &str,
    work_id: Option<Uuid>,
    attempt_id: Option<Uuid>,
    supports_broker_aware: bool,
) -> Result<Vec<McpServer>> {
    let mut servers = Vec::new();
    for connection in list(pool, company).await? {
        if connection.status != ConnectionStatus::Enabled
            || !work_scope_matches(&connection, actor, work_id)
        {
            continue;
        }
        let Some(attempt_id) = attempt_id else {
            continue;
        };
        sqlx::query(
            "UPDATE restless_authority.provider_connections \
             SET assigned_attempt_id=$3, updated_at=now() \
             WHERE company=$1 AND name=$2 AND assigned_work_id=$4",
        )
        .bind(company)
        .bind(&connection.name)
        .bind(attempt_id)
        .bind(work_id)
        .execute(pool)
        .await?;
        let credential_dir = runtime_credential_dir(&connection.name)?;
        let server = McpServerStdio::new(&connection.name, MCP_REMOTE)
            .args(vec![connection.endpoint, "--silent".into()])
            .env(vec![EnvVariable::new(
                "MCP_REMOTE_CONFIG_DIR",
                credential_dir,
            )]);
        servers.push(McpServer::Stdio(server));
    }
    let recurring_policies = recurring_mcp_policies(pool, company, None).await?;
    for server in local_mcp_list(pool, company).await? {
        if !server.enabled || server.assigned_actor != actor {
            continue;
        }
        let fixed_scope = local_work_scope_matches(&server, actor, work_id, attempt_id);
        let mut recurring_scope = None;
        if !fixed_scope && server.transport == "host_http" {
            if let (Some(work_id), Some(attempt_id)) = (work_id, attempt_id) {
                let running = org.list_running_work_attempts().await?.iter().any(|attempt| {
                    attempt.id == attempt_id && attempt.work_id == work_id
                        && attempt.actor_id == actor && attempt.interrupt_requested_at.is_none()
                });
                if running {
                    for policy in recurring_policies.iter().filter(|policy| {
                        policy.connection_name == server.name
                            && policy.assigned_actor == actor
                            && recurring_policy_matches_connection(policy, &server)
                    }) {
                        if org.recurring_work_lineage(
                            work_id, actor, policy.schedule_id,
                            policy.responsibility_id, policy.responsibility_version,
                        ).await?.is_some() {
                            recurring_scope = Some(policy);
                            break;
                        }
                    }
                }
            }
        }
        if !fixed_scope && recurring_scope.is_none() {
            continue;
        }
        match server.transport.as_str() {
            "stdio" => {
                if server.broker_aware && !supports_broker_aware {
                    bail!(
                        "broker-aware local MCP server {:?} requires a Codex actor session",
                        server.name
                    );
                }
                let mut stdio =
                    McpServerStdio::new(&server.name, &server.command).args(server.args);
                if server.broker_aware {
                    stdio = stdio.env(vec![EnvVariable::new(BROKER_AWARE_ACTOR_ENV_MARKER, "1")]);
                }
                servers.push(McpServer::Stdio(stdio));
            }
            "host_http" | "public_http" | "broker_stdio" => {
                if server.transport == "broker_stdio" {
                    require_reviewed_stdio_profile(&server)?;
                } else {
                    reviewed_http_read_profile(&server)?;
                }
                let (Some(work_id), Some(attempt_id)) = (work_id, attempt_id) else {
                    continue;
                };
                let grant = if let Some(policy) = recurring_scope {
                    capabilities.issue_mcp_recurring_session(
                        company, actor, &server.name, &server.policy_revision.to_string(),
                        work_id, attempt_id, policy.schedule_id, policy.policy_revision,
                    )?
                } else {
                    capabilities.issue_mcp_session(
                        company, actor, &server.name, &server.policy_revision.to_string(),
                        work_id, attempt_id,
                    )?
                };
                let port = crate::port_with_offset(crate::model_gateway::RUNTIME_RELAY_PORT)?;
                // The nonce changes the ACP/Codex launch contract when a new
                // grant is issued; the URL carries no secret.
                let url = format!(
                    "http://host.docker.internal:{port}/mcp/{company}/{}?launch={}",
                    server.name,
                    Uuid::new_v4().simple(),
                );
                servers.push(McpServer::Http(
                    McpServerHttp::new(&server.name, url).headers(vec![HttpHeader::new(
                        "Authorization",
                        format!("Bearer {grant}"),
                    )]),
                ));
            }
            other => bail!("unknown local MCP transport {other:?}"),
        }
    }
    // Every granted connection reaches the actor through one gateway server.
    if !crate::connections::usable_tools(pool, company, actor).await?.is_empty() {
        let (work_id, attempt_id) = match (work_id, attempt_id) {
            (Some(work_id), Some(attempt_id)) => (Some(work_id), Some(attempt_id)),
            _ => (None, None),
        };
        let grant = capabilities.issue_tool_session(company, actor, work_id, attempt_id)?;
        let url = format!(
            "{}?launch={}",
            crate::tool_gateway::runtime_url(company),
            Uuid::new_v4().simple()
        );
        servers.push(McpServer::Http(
            McpServerHttp::new("restless-tools", url)
                .headers(vec![HttpHeader::new("Authorization", format!("Bearer {grant}"))]),
        ));
    }
    Ok(servers)
}

fn local_work_scope_matches(
    server: &LocalMcpServer,
    actor: &str,
    work_id: Option<Uuid>,
    attempt_id: Option<Uuid>,
) -> bool {
    server.assigned_actor == actor
        && server.assigned_work_id.is_some()
        && server.assigned_work_id == work_id
        && attempt_id.is_some()
}

fn work_scope_matches(connection: &ConnectedTool, actor: &str, work_id: Option<Uuid>) -> bool {
    connection.assigned_actor == actor
        && work_id.is_some()
        && connection.assigned_work_id == work_id
}

/// Start the mature remote-MCP OAuth bridge on the host, return only after it
/// has prepared the provider authorization URL, and observe completion in the
/// background. The owner never handles a token or callback code.
#[expect(
    clippy::too_many_arguments,
    reason = "one bounded provider-connection request"
)]
pub async fn begin_oauth_install(
    root: &Path,
    authority: &crate::authority::AuthorityStore,
    org: &OrgIntel,
    company: &str,
    name: &str,
    endpoint: &str,
    purpose: &str,
    assigned_actor: &str,
    requested_scopes: &[String],
    work_id: Uuid,
    attempt_id: Uuid,
    requested_by: &str,
    force_reauthentication: bool,
) -> Result<ConnectionLaunch> {
    validate_name(name)?;
    validate_endpoint(endpoint)?;
    let local_name_in_use: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.local_mcp_servers \
         WHERE company=$1 AND name=$2 AND enabled)",
    )
    .bind(company)
    .bind(name)
    .fetch_one(authority.pool())
    .await?;
    if local_name_in_use {
        bail!("MCP server name {name:?} is already used by an enabled company-local server");
    }
    ensure_runtime_bridge_available(company).await?;
    let credential_dir = host_credential_dir(root, company, name)?;
    recover_interrupted_reconnect(&credential_dir)?;
    if !force_reauthentication {
        if let Some(existing) = get(authority.pool(), company, name).await? {
            // Purpose, actor and Attempt scope are Restless-owned assignment
            // metadata, not provider authorization inputs. Rebinding those
            // must never manufacture another OAuth request when the endpoint
            // and granted scopes still live-probe successfully.
            let request_matches =
                existing.endpoint == endpoint && existing.requested_scopes == requested_scopes;
            if request_matches && cached_token_material_exists(&credential_dir)? {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    probe_host_tools(&credential_dir, endpoint),
                )
                .await
                {
                    Ok(Ok(host_observed_tools)) => {
                        sync_credentials_to_runtime(company, name, &credential_dir).await?;
                        let observed_tools = probe_runtime_tools(company, name, endpoint).await?;
                        if observed_tools != host_observed_tools {
                            bail!(
                                "host and fresh Runtime MCP tool observations disagree (host {}, Runtime {})",
                                host_observed_tools.len(),
                                observed_tools.len()
                            );
                        }
                        bind_execution_scope(
                            authority.pool(),
                            company,
                            name,
                            purpose,
                            assigned_actor,
                            work_id,
                            attempt_id,
                        )
                        .await?;
                        let connection =
                            enable(authority.pool(), company, name, &observed_tools, None).await?;
                        let handoff_id = existing
                            .owner_handoff_id
                            .as_deref()
                            .map(Uuid::parse_str)
                            .transpose()
                            .context("decode cached connected-tool owner handoff")?;
                        authority
                            .emit(
                                company,
                                "provider_connection_enabled",
                                Some("daemon"),
                                serde_json::json!({
                                    "name": name,
                                    "endpoint": endpoint,
                                    "assigned_actor": assigned_actor,
                                    "observed_tools": observed_tools,
                                    "owner_handoff_id": handoff_id,
                                    "recovered_from_observed_oauth": true,
                                }),
                            )
                            .await?;
                        let handoff_pending = if let Some(handoff_id) = handoff_id {
                            org.list_owner_handoffs().await?.into_iter().any(|handoff| {
                                handoff.id == handoff_id
                                    && handoff.state == OwnerHandoffState::Pending
                            })
                        } else {
                            false
                        };
                        if handoff_pending {
                            let handoff_id = handoff_id.expect("pending handoff has an id");
                            org.resolve_observed_handoff(
                                handoff_id,
                                "daemon",
                                &format!(
                                    "Authenticated MCP capability observation succeeded for {name}; a fresh {assigned_actor} session will receive the connection."
                                ),
                            )
                            .await?;
                        }
                        return Ok(ConnectionLaunch {
                            connection,
                            authorization_url: None,
                            owner_handoff_id: handoff_id,
                        });
                    }
                    Ok(Err(error)) => tracing::warn!(
                        company,
                        connection = name,
                        error = %format!("{error:#}"),
                        "cached connected-tool credentials were not usable; preparing provider authorization"
                    ),
                    Err(_) => tracing::warn!(
                        company,
                        connection = name,
                        "cached connected-tool probe timed out; preparing provider authorization"
                    ),
                }
            }
        }
    }
    let mut credential_replacement =
        CredentialReplacement::prepare(&credential_dir, force_reauthentication)?;
    std::fs::create_dir_all(&credential_dir).with_context(|| {
        format!(
            "create connected-tool credential directory {}",
            credential_dir.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&credential_dir, std::fs::Permissions::from_mode(0o700))?;
    }

    let mut child = tokio::process::Command::new("npx")
        .args([
            "-y",
            "-p",
            "mcp-remote@0.8.1",
            "mcp-remote-client",
            endpoint,
        ])
        .env("MCP_REMOTE_CONFIG_DIR", &credential_dir)
        // The provider URL belongs in the owner Attention handoff. Suppress
        // mcp-remote's own browser launch so it cannot bypass that boundary.
        .env("BROWSER", "echo")
        // mcp-remote-client treats stdin EOF as an operator shutdown signal.
        // Keep the pipe alive until its own authenticated tool/resource probe
        // exits; `/dev/null` makes a valid OAuth callback look like a missing
        // tool list because the client closes between request and response.
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("start remote MCP OAuth discovery")?;
    let stdin_guard = child.stdin.take().context("hold MCP OAuth stdin open")?;
    let stdout = child.stdout.take().context("capture MCP OAuth stdout")?;
    let stderr = child.stderr.take().context("capture MCP OAuth stderr")?;
    let mut stderr = BufReader::new(stderr);
    let mut prefix = String::new();
    let authorization_url = tokio::time::timeout(std::time::Duration::from_secs(45), async {
        let mut authorization_prompt_seen = false;
        loop {
            let mut line = String::new();
            let bytes = stderr.read_line(&mut line).await?;
            if bytes == 0 {
                bail!("remote MCP OAuth helper exited before preparing authorization");
            }
            prefix.push_str(&line);
            if let Some(url) = authorization_url_from_line(&line, &mut authorization_prompt_seen) {
                return Ok::<String, anyhow::Error>(url);
            }
        }
    })
    .await
    .context("remote MCP OAuth discovery timed out")??;

    let action = format!(
        "Sign in to the provider, select the intended workspace, and approve scopes: {}.",
        requested_scopes.join(" ")
    );
    let prepared = format!(
        "Provider-hosted authorization is prepared at {authorization_url}\nPurpose: {purpose}\nConnection: {name} -> {endpoint}"
    );
    let resume = format!(
        "Restless observes an authenticated MCP tool list for {name} and installs it for actor {assigned_actor}."
    );
    let handoff_id = match org
        .request_owner_handoff(NewOwnerHandoff {
            work_id,
            attempt_id: Some(attempt_id),
            requested_by,
            category: OwnerHandoffCategory::Identity,
            requested_action: &action,
            prepared_state: &prepared,
            resume_condition: &resume,
        })
        .await
    {
        Ok(id) => id,
        Err(error) => {
            let _ = child.kill().await;
            return Err(error).context("create connected-tool owner handoff");
        }
    };

    let connection = match stage(
        authority.pool(),
        company,
        name,
        endpoint,
        purpose,
        assigned_actor,
        requested_scopes,
        &handoff_id.to_string(),
        requested_by,
        work_id,
        attempt_id,
    )
    .await
    {
        Ok(connection) => connection,
        Err(error) => {
            let _ = child.kill().await;
            return Err(error);
        }
    };
    authority
        .emit(
            company,
            "provider_connection_requested",
            Some(requested_by),
            serde_json::json!({
                "name": name,
                "endpoint": endpoint,
                "purpose": purpose,
                "assigned_actor": assigned_actor,
                "requested_scopes": requested_scopes,
                "owner_handoff_id": handoff_id,
            }),
        )
        .await?;

    let authority = authority.clone();
    let org = org.clone();
    let company_owned = company.to_string();
    let name_owned = name.to_string();
    let endpoint_owned = endpoint.to_string();
    let assigned_actor_owned = assigned_actor.to_string();
    let credential_dir_owned = credential_dir.clone();
    tokio::spawn(async move {
        let _stdin_guard = stdin_guard;
        let result = async {
            let mut remaining_stderr = String::new();
            let mut stdout = BufReader::new(stdout);
            let mut stdout_text = String::new();
            let reads = tokio::join!(
                stderr.read_to_string(&mut remaining_stderr),
                stdout.read_to_string(&mut stdout_text)
            );
            reads.0.context("read MCP OAuth stderr")?;
            reads.1.context("read MCP OAuth stdout")?;
            let status = child.wait().await.context("wait for MCP OAuth helper")?;
            let transcript = format!("{prefix}{remaining_stderr}\n{stdout_text}");
            if !status.success() {
                bail!(
                    "remote MCP authorization failed: {}",
                    transcript.lines().rev().take(8).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join(" | ")
                );
            }
            let host_observed_tools = extract_tools(&transcript)?;
            sync_credentials_to_runtime(&company_owned, &name_owned, &credential_dir_owned).await?;
            let observed_tools = probe_runtime_tools(
                &company_owned,
                &name_owned,
                &endpoint_owned,
            )
            .await?;
            if observed_tools != host_observed_tools {
                bail!(
                    "host and fresh Runtime MCP tool observations disagree (host {}, Runtime {})",
                    host_observed_tools.len(),
                    observed_tools.len()
                );
            }
            enable(
                authority.pool(),
                &company_owned,
                &name_owned,
                &observed_tools,
                None,
            )
            .await?;
            authority
                .emit(
                    &company_owned,
                    "provider_connection_enabled",
                    Some("daemon"),
                    serde_json::json!({
                        "name": name_owned,
                        "endpoint": endpoint_owned,
                        "assigned_actor": assigned_actor_owned,
                        "observed_tools": observed_tools,
                        "owner_handoff_id": handoff_id,
                    }),
                )
                .await?;
            org.resolve_observed_handoff(
                handoff_id,
                "daemon",
                &format!(
                    "Authenticated MCP capability observation succeeded for {name_owned}; a fresh {assigned_actor_owned} session will receive the connection."
                ),
            )
            .await?;
            credential_replacement.commit()?;
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if let Err(error) = result {
            let message = format!("{error:#}");
            let _ = fail(authority.pool(), &company_owned, &name_owned, &message).await;
            let _ = org
                .refresh_owner_handoff(
                    handoff_id,
                    "daemon",
                    "Retry provider authentication after Restless repairs the observed connection failure.",
                    &format!(
                        "Connection {name_owned} was not installed. Observed failure: {message}"
                    ),
                    &format!(
                        "Restless observes an authenticated MCP tool list for {name_owned} before resuming Work."
                    ),
                )
                .await;
            tracing::error!(company = %company_owned, connection = %name_owned, error = %message, "connected-tool authorization failed");
        }
    });

    Ok(ConnectionLaunch {
        connection,
        authorization_url: Some(authorization_url),
        owner_handoff_id: Some(handoff_id),
    })
}

fn cached_token_material_exists(credential_dir: &Path) -> Result<bool> {
    let bridge_dir = credential_dir.join("mcp-remote-v1");
    if !bridge_dir.is_dir() {
        return Ok(false);
    }
    for entry in std::fs::read_dir(&bridge_dir)
        .with_context(|| format!("inspect MCP credential directory {}", bridge_dir.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .file_name()
                .to_string_lossy()
                .ends_with("_tokens.json")
        {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn probe_host_tools(credential_dir: &Path, endpoint: &str) -> Result<Vec<String>> {
    let mut child = tokio::process::Command::new("npx")
        .args([
            "-y",
            "-p",
            "mcp-remote@0.8.1",
            "mcp-remote-client",
            endpoint,
        ])
        .env("MCP_REMOTE_CONFIG_DIR", credential_dir)
        .env("BROWSER", "echo")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("probe cached host MCP credentials")?;
    let _stdin_guard = child
        .stdin
        .take()
        .context("hold host MCP probe stdin open")?;
    let mut stdout = child.stdout.take().context("capture host MCP stdout")?;
    let mut stderr = child.stderr.take().context("capture host MCP stderr")?;
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    let reads = tokio::join!(
        stdout.read_to_string(&mut stdout_text),
        stderr.read_to_string(&mut stderr_text)
    );
    reads.0.context("read host MCP stdout")?;
    reads.1.context("read host MCP stderr")?;
    let status = child.wait().await.context("wait for host MCP probe")?;
    let transcript = format!("{stderr_text}\n{stdout_text}");
    if !status.success() {
        bail!(
            "cached host MCP probe failed: {}",
            transcript
                .lines()
                .rev()
                .take(8)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    extract_tools(&transcript)
}

fn authorization_url_from_line(line: &str, prompt_seen: &mut bool) -> Option<String> {
    const PROMPT: &str = "Please authorize this client by visiting:";
    if let Some((_, after_prompt)) = line.split_once(PROMPT) {
        *prompt_seen = true;
        if let Some(url) = after_prompt
            .split_whitespace()
            .find(|part| part.starts_with("https://"))
        {
            return Some(url.trim().to_string());
        }
        return None;
    }
    if !*prompt_seen {
        return None;
    }
    line.split_whitespace()
        .find(|part| part.starts_with("https://"))
        .map(|url| url.trim().to_string())
}

fn extract_tools(transcript: &str) -> Result<Vec<String>> {
    let start = transcript
        .find("Tools:")
        .context("MCP authorization completed without a tool-list observation")?
        + "Tools:".len();
    let after = &transcript[start..];
    let json_start = after.find('{').context("MCP tool list was not JSON")?;
    let mut deserializer = serde_json::Deserializer::from_str(&after[json_start..]);
    let value =
        serde_json::Value::deserialize(&mut deserializer).context("decode MCP tool list")?;
    let mut tools = value
        .get("tools")
        .and_then(serde_json::Value::as_array)
        .context("MCP tool-list response has no tools array")?
        .iter()
        .filter_map(|tool| tool.get("name").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    tools.sort();
    tools.dedup();
    if tools.is_empty() {
        bail!("authenticated MCP exposed no tools");
    }
    Ok(tools)
}

async fn sync_credentials_to_runtime(company: &str, name: &str, source: &Path) -> Result<()> {
    let container = crate::runtime::container_name(company);
    let destination = runtime_credential_dir(name)?;
    let prepare = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            &container,
            "sh",
            "-c",
            "set -eu; umask 077; mkdir -p \"$1\"",
            "restless-connected-tool",
            &destination,
        ])
        .output()
        .await
        .context("prepare runtime connected-tool credential directory")?;
    if !prepare.status.success() {
        bail!(
            "prepare runtime credential directory failed: {}",
            String::from_utf8_lossy(&prepare.stderr).trim()
        );
    }
    let source_contents = format!("{}/.", source.display());
    let target = format!("{container}:{destination}");
    let copied = tokio::process::Command::new("docker")
        .args(["cp", &source_contents, &target])
        .output()
        .await
        .context("copy scoped MCP credentials into Runtime")?;
    if !copied.status.success() {
        bail!(
            "copy scoped MCP credentials into Runtime failed: {}",
            String::from_utf8_lossy(&copied.stderr).trim()
        );
    }
    let secured = tokio::process::Command::new("docker")
        .args([
            "exec",
            &container,
            "sh",
            "-c",
            "set -eu; chown -R company:company \"$1\"; chmod -R go-rwx \"$1\"",
            "restless-connected-tool",
            &destination,
        ])
        .output()
        .await
        .context("secure runtime connected-tool credentials")?;
    if !secured.status.success() {
        bail!(
            "secure runtime connected-tool credentials failed: {}",
            String::from_utf8_lossy(&secured.stderr).trim()
        );
    }
    Ok(())
}

async fn ensure_runtime_bridge_available(company: &str) -> Result<()> {
    let container = crate::runtime::container_name(company);
    let output = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            &container,
            "test",
            "-x",
            MCP_REMOTE_CLIENT,
        ])
        .output()
        .await
        .context("probe the Runtime remote-MCP bridge")?;
    if !output.status.success() {
        bail!(
            "the current Company Runtime image does not contain the pinned remote-MCP bridge; reconcile the Runtime before requesting owner consent"
        );
    }
    Ok(())
}

async fn probe_runtime_tools(company: &str, name: &str, endpoint: &str) -> Result<Vec<String>> {
    let container = crate::runtime::container_name(company);
    let credential_dir = runtime_credential_dir(name)?;
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            "-u",
            "company",
            "-e",
            &format!("MCP_REMOTE_CONFIG_DIR={credential_dir}"),
            "-e",
            "BROWSER=echo",
            &container,
            MCP_REMOTE_CLIENT,
            endpoint,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("probe authenticated MCP from the Company Runtime")?;
    let _stdin_guard = child
        .stdin
        .take()
        .context("hold Runtime MCP probe stdin open")?;
    let mut stdout = child.stdout.take().context("capture Runtime MCP stdout")?;
    let mut stderr = child.stderr.take().context("capture Runtime MCP stderr")?;
    let mut stdout_text = String::new();
    let mut stderr_text = String::new();
    let reads = tokio::join!(
        stdout.read_to_string(&mut stdout_text),
        stderr.read_to_string(&mut stderr_text)
    );
    reads.0.context("read Runtime MCP stdout")?;
    reads.1.context("read Runtime MCP stderr")?;
    let status = child.wait().await.context("wait for Runtime MCP probe")?;
    let transcript = format!("{}\n{}", stderr_text, stdout_text,);
    if !status.success() {
        bail!(
            "fresh Runtime MCP probe failed: {}",
            transcript
                .lines()
                .rev()
                .take(8)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    extract_tools(&transcript)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_names_and_endpoints_are_bounded_before_becoming_paths_or_process_args() {
        assert!(validate_name("sales-crm").is_ok());
        assert!(validate_name("../sales").is_err());
        assert!(validate_name("Sales").is_err());
        assert!(validate_endpoint("https://mcp.example.com/mcp").is_ok());
        assert!(validate_endpoint("http://mcp.example.com/mcp").is_err());
        assert!(validate_endpoint("https://token@example.com/mcp").is_err());
    }

    #[test]
    fn failed_reconnect_restores_the_previous_oauth_material() {
        let root = std::env::temp_dir().join(format!(
            "restless-connected-tool-reconnect-{}",
            Uuid::new_v4()
        ));
        let active = root.join("attio");
        let bridge = active.join("mcp-remote-v1");
        std::fs::create_dir_all(&bridge).unwrap();
        std::fs::write(bridge.join("existing_tokens.json"), b"existing").unwrap();

        {
            let _replacement = CredentialReplacement::prepare(&active, true).unwrap();
            let replacement_bridge = active.join("mcp-remote-v1");
            std::fs::create_dir_all(&replacement_bridge).unwrap();
            std::fs::write(
                replacement_bridge.join("replacement_client_info.json"),
                b"incomplete",
            )
            .unwrap();
        }

        assert!(active.join("mcp-remote-v1/existing_tokens.json").is_file());
        assert!(!active
            .join("mcp-remote-v1/replacement_client_info.json")
            .exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn successful_reconnect_discards_the_superseded_oauth_material() {
        let root = std::env::temp_dir().join(format!(
            "restless-connected-tool-reconnect-{}",
            Uuid::new_v4()
        ));
        let active = root.join("attio");
        let bridge = active.join("mcp-remote-v1");
        std::fs::create_dir_all(&bridge).unwrap();
        std::fs::write(bridge.join("existing_tokens.json"), b"existing").unwrap();

        let mut replacement = CredentialReplacement::prepare(&active, true).unwrap();
        let replacement_bridge = active.join("mcp-remote-v1");
        std::fs::create_dir_all(&replacement_bridge).unwrap();
        std::fs::write(
            replacement_bridge.join("replacement_tokens.json"),
            b"replacement",
        )
        .unwrap();
        replacement.commit().unwrap();
        drop(replacement);

        assert!(active
            .join("mcp-remote-v1/replacement_tokens.json")
            .is_file());
        assert!(!active.join("mcp-remote-v1/existing_tokens.json").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn connected_tool_scope_is_the_work_not_the_durable_actor() {
        let work_id = Uuid::new_v4();
        let attempt_id = Uuid::new_v4();
        let connection = ConnectedTool {
            name: "attio".into(),
            endpoint: "https://mcp.attio.com/mcp".into(),
            purpose: "bounded CRM work".into(),
            assigned_actor: "crm-operations".into(),
            assigned_work_id: Some(work_id),
            assigned_attempt_id: Some(attempt_id),
            status: ConnectionStatus::Enabled,
            credential_reference: "runtime-scoped:/connected-tools/attio".into(),
            requested_scopes: vec!["mcp".into()],
            observed_tools: vec!["whoami".into()],
            workspace_reference: None,
            owner_handoff_id: None,
            last_observed_at: None,
            failure: None,
        };

        assert!(work_scope_matches(
            &connection,
            "crm-operations",
            Some(work_id)
        ));
        assert!(!work_scope_matches(
            &connection,
            "crm-operations",
            Some(Uuid::new_v4())
        ));
        assert!(!work_scope_matches(&connection, "crm-operations", None));
    }

    #[test]
    fn tool_observation_is_parsed_from_the_mature_bridge_transcript() {
        let transcript = r#"[1] Tools: {
  "tools": [
    {"name":"records-search","inputSchema":{"type":"object"}},
    {"name":"records-update","inputSchema":{"type":"object"}}
  ]
}
[1] Requesting resource list..."#;
        assert_eq!(
            extract_tools(transcript).unwrap(),
            vec!["records-search", "records-update"]
        );
    }

    #[test]
    fn oauth_handoff_ignores_discovery_urls_and_captures_the_authorization_url() {
        let mut prompt_seen = false;
        assert_eq!(
            authorization_url_from_line(
                "Using OAuth server https://app.attio.com for https://mcp.attio.com/mcp\n",
                &mut prompt_seen,
            ),
            None
        );
        assert_eq!(
            authorization_url_from_line(
                "Please authorize this client by visiting:\n",
                &mut prompt_seen,
            ),
            None
        );
        assert_eq!(
            authorization_url_from_line(
                "https://app.attio.com/oidc/authorize?client_id=test&state=flow\n",
                &mut prompt_seen,
            ),
            Some("https://app.attio.com/oidc/authorize?client_id=test&state=flow".to_string())
        );
    }
}
