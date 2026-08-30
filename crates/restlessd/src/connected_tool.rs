//! Provider-neutral connected-tool state and ACP attachment materialisation.
//!
//! External applications remain the source of their own records. Authority
//! retains only why a connection exists, which actor may receive it, and the
//! last authenticated capability observation. OAuth material is deliberately
//! absent from Postgres; the scoped runtime credential directory is referenced
//! by path and consumed by the mature `mcp-remote` bridge.

use std::path::Path;
use std::process::Stdio;

use agent_client_protocol::schema::v1::{EnvVariable, McpServer, McpServerStdio};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ConnectionStatus {
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
pub(crate) struct ConnectedTool {
    pub(crate) name: String,
    pub(crate) endpoint: String,
    pub(crate) purpose: String,
    pub(crate) assigned_actor: String,
    pub(crate) assigned_work_id: Option<Uuid>,
    pub(crate) assigned_attempt_id: Option<Uuid>,
    pub(crate) status: ConnectionStatus,
    pub(crate) credential_reference: String,
    pub(crate) requested_scopes: Vec<String>,
    pub(crate) observed_tools: Vec<String>,
    pub(crate) workspace_reference: Option<String>,
    pub(crate) owner_handoff_id: Option<String>,
    pub(crate) last_observed_at: Option<DateTime<Utc>>,
    pub(crate) failure: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ConnectionLaunch {
    pub(crate) connection: ConnectedTool,
    pub(crate) authorization_url: Option<String>,
    pub(crate) owner_handoff_id: Option<Uuid>,
}

pub(crate) async fn ensure_schema(pool: &PgPool) -> Result<()> {
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
    Ok(())
}

pub(crate) fn validate_name(name: &str) -> Result<()> {
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

pub(crate) fn validate_endpoint(endpoint: &str) -> Result<()> {
    let parsed = url::Url::parse(endpoint).context("connected-tool endpoint must be a URL")?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() || parsed.fragment().is_some() {
        bail!("connected-tool endpoint must be an HTTPS URL without a fragment");
    }
    if parsed.username() != "" || parsed.password().is_some() {
        bail!("connected-tool endpoint must not contain credentials");
    }
    Ok(())
}

pub(crate) fn runtime_credential_dir(name: &str) -> Result<String> {
    validate_name(name)?;
    Ok(format!("{RUNTIME_CREDENTIAL_ROOT}/{name}"))
}

pub(crate) fn host_credential_dir(
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
pub(crate) async fn stage(
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

pub(crate) async fn enable(
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

pub(crate) async fn fail(pool: &PgPool, company: &str, name: &str, failure: &str) -> Result<()> {
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

pub(crate) async fn disable(pool: &PgPool, company: &str, name: &str) -> Result<ConnectedTool> {
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

pub(crate) async fn observe_workspace(
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

pub(crate) async fn get(pool: &PgPool, company: &str, name: &str) -> Result<Option<ConnectedTool>> {
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

pub(crate) async fn list(pool: &PgPool, company: &str) -> Result<Vec<ConnectedTool>> {
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

pub(crate) async fn session_servers(
    pool: &PgPool,
    company: &str,
    actor: &str,
    work_id: Option<Uuid>,
    attempt_id: Option<Uuid>,
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
    Ok(servers)
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
pub(crate) async fn begin_oauth_install(
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
