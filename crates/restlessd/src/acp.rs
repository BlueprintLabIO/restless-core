//! ACP client (sprint 01 T3 canon, grown into the daemon): start the agent
//! binary as an ordinary supervised process inside the company's persistent container
//! (`docker exec`, §5), speak JSON-RPC over stdio, stream turn updates,
//! cancel on demand. Processes and capabilities are disposable; a provider
//! session may remain hot only inside one actor/responsibility scope.
//!
//! The agent is `omp`, which speaks ACP natively and — unlike the codex-acp
//! binary it replaces — reports per-turn token and dollar usage on the
//! session stream. That single fact moved the spend fuse out of the HTTP path
//! (T2's proxy) and up to here, where the daemon already knows which company
//! the session belongs to.

use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use agent_client_protocol::{
    self as acp,
    schema::{
        v1::{
            CancelNotification, ContentBlock, InitializeRequest, LoadSessionRequest, McpServer,
            NewSessionRequest, PermissionOptionKind, PromptRequest, PromptResponse,
            RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
            SelectedPermissionOutcome, SessionConfigKind, SessionConfigOption,
            SessionConfigOptionCategory, SessionConfigSelectOptions, SessionId,
            SessionNotification, SessionUpdate, SetSessionConfigOptionRequest, TextContent,
            ToolCallStatus,
        },
        ProtocolVersion,
    },
    Agent, ByteStreams, Client, ConnectionTo,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use tokio::io::AsyncWriteExt as _;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};
use tokio_util::sync::CancellationToken;

#[cfg(test)]
static TEST_COORDINATOR_OVERRIDE: std::sync::OnceLock<Mutex<Option<String>>> =
    std::sync::OnceLock::new();

/// Test-only transport seam for an isolated current-code coordination
/// listener. Product launches keep using the Runtime's baked coordinator; the
/// live integration probe must not restart or borrow an older daemon merely to
/// exercise new wire/schema behavior.
#[cfg(test)]
pub(crate) fn set_test_coordinator_override(value: Option<String>) {
    if let Some(value) = &value {
        let port = value
            .strip_prefix("host.docker.internal:")
            .and_then(|port| port.parse::<u16>().ok());
        assert!(port.is_some_and(|port| port > 0));
    }
    *TEST_COORDINATOR_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("test coordinator override") = value;
}

#[cfg(test)]
fn test_coordinator_override() -> Option<String> {
    TEST_COORDINATOR_OVERRIDE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("test coordinator override")
        .clone()
}

pub(crate) fn runtime_coordinator() -> Result<String> {
    #[cfg(test)]
    if let Some(coordinator) = test_coordinator_override() {
        return Ok(coordinator);
    }
    crate::runtime_coordinator()
}

/// Everything a wake needs to start an agent against a provider.
///
/// The agent binary is `omp` (Oh My Pi), which speaks ACP natively and
/// reports its own token and dollar usage per turn. The process receives a
/// signed model-relay capability, never the provider credential, OMP root
/// bearer, or Infisical machine identity.
#[derive(Clone)]
pub struct AgentAuth {
    /// Provider-qualified model, e.g. `moonshot/k3-256k`.
    pub model: String,
    /// Explicit provider-supported reasoning effort for this actor launch.
    pub effort: String,
    pub company: String,
    /// The host-generated session identifier binds coordination and model
    /// grants to this one supervised ACP process.
    pub session_id: String,
    pub coordination_token_env: String,
    pub coordination_token: String,
    pub gateway_token_env: String,
    pub gateway_token: String,
    pub gateway_url: String,
    /// Whether the provider reports a charged API cost or only a catalogue
    /// estimate for subscription access. The Runtime still receives no
    /// provider credential either way.
    pub billing: crate::model_gateway::ModelBilling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionLocator {
    version: u8,
    #[serde(default)]
    harness: crate::runtime::AgentHarness,
    #[serde(default)]
    harness_build: String,
    company: String,
    actor: String,
    responsibility: String,
    cwd: String,
    model: String,
    #[serde(default = "default_reasoning_effort_owned")]
    effort: String,
    session_id: String,
    cumulative_cost_usd: Option<f64>,
}

pub(crate) const DEFAULT_REASONING_EFFORT: &str = "medium";

fn default_reasoning_effort_owned() -> String {
    DEFAULT_REASONING_EFFORT.to_string()
}

fn session_locator_path(
    harness: crate::runtime::AgentHarness,
    company: &str,
    actor: &str,
    responsibility: &str,
) -> Result<String> {
    let profile = AcpProfile::new(harness)?;
    let digest = format!(
        "{:x}",
        Sha256::digest(
            format!("{}\0{company}\0{actor}\0{responsibility}", harness.as_str()).as_bytes()
        )
    );
    Ok(format!("{}/sessions/{digest}.json", profile.config_dir()))
}

fn validate_session_locator(
    locator: &SessionLocator,
    profile: AcpProfile,
    company: &str,
    actor: &str,
    responsibility: &str,
) -> Result<()> {
    if locator.version != 2
        || locator.harness != profile.harness
        || locator.harness_build != profile.build()
        || locator.company != company
        || locator.actor != actor
        || locator.responsibility != responsibility
    {
        anyhow::bail!(
            "refusing ACP session locator outside its company/actor/responsibility/workspace scope"
        );
    }
    Ok(())
}

fn session_locator_is_reusable(
    locator: &SessionLocator,
    profile: AcpProfile,
    cwd: &str,
    model: &str,
    effort: &str,
    load_session_available: bool,
) -> bool {
    locator.harness == profile.harness
        && locator.harness_build == profile.build()
        && locator.cwd == cwd
        && locator.model == model
        && locator.effort == effort
        && load_session_available
}

pub(crate) const AGENT_CONFIG_DIR: &str = "/company/home/.restless/omp-agent";
const OMP_RUNTIME_CONFIG: &str = "/company/home/.restless/omp-agent/restless-runtime.yml";
const CLAUDE_AGENT_CONFIG_DIR: &str = "/company/home/.restless/claude-agent";
const CLAUDE_AGENT_TOOLS: &[&str] = &["Read", "Write", "Edit", "Bash", "Glob", "Grep"];

#[derive(Debug, Clone, Copy)]
struct AcpProfile {
    harness: crate::runtime::AgentHarness,
}

impl AcpProfile {
    fn new(harness: crate::runtime::AgentHarness) -> Result<Self> {
        match harness {
            crate::runtime::AgentHarness::RestlessManaged
            | crate::runtime::AgentHarness::ClaudeAgent => Ok(Self { harness }),
            crate::runtime::AgentHarness::Codex => {
                anyhow::bail!("Codex uses its native App Server transport, not ACP")
            }
        }
    }

    const fn config_dir(self) -> &'static str {
        match self.harness {
            crate::runtime::AgentHarness::RestlessManaged => AGENT_CONFIG_DIR,
            crate::runtime::AgentHarness::ClaudeAgent => CLAUDE_AGENT_CONFIG_DIR,
            crate::runtime::AgentHarness::Codex => unreachable!(),
        }
    }

    const fn build(self) -> &'static str {
        self.harness.build()
    }

    const fn transport(self) -> &'static str {
        "acp-stdio-v1"
    }

    const fn tariff_version(self) -> Option<&'static str> {
        match self.harness {
            crate::runtime::AgentHarness::ClaudeAgent => {
                Some(crate::model_gateway::ANTHROPIC_TARIFF_VERSION)
            }
            crate::runtime::AgentHarness::RestlessManaged => None,
            crate::runtime::AgentHarness::Codex => unreachable!(),
        }
    }

    const fn native_agent_build(self) -> Option<&'static str> {
        self.harness.native_agent_build()
    }

    fn native_tools(self) -> Vec<&'static str> {
        match self.harness {
            crate::runtime::AgentHarness::RestlessManaged => OMP_AGENT_TOOLS.split(',').collect(),
            crate::runtime::AgentHarness::ClaudeAgent => CLAUDE_AGENT_TOOLS.to_vec(),
            crate::runtime::AgentHarness::Codex => unreachable!(),
        }
    }

    fn session_model(self, provider_model: &str) -> Result<String> {
        match self.harness {
            crate::runtime::AgentHarness::RestlessManaged => Ok(provider_model.to_string()),
            crate::runtime::AgentHarness::ClaudeAgent => provider_model
                .strip_prefix("anthropic/")
                .filter(|model| !model.is_empty())
                .map(str::to_string)
                .with_context(|| {
                    format!(
                        "Claude Agent requires an anthropic/<model> route, got {provider_model}"
                    )
                }),
            crate::runtime::AgentHarness::Codex => unreachable!(),
        }
    }

    fn command_args(self, model: &str, effort: &str, system_prompt: &str) -> Vec<String> {
        match self.harness {
            crate::runtime::AgentHarness::RestlessManaged => {
                omp_agent_command_args(model, effort, system_prompt)
            }
            crate::runtime::AgentHarness::ClaudeAgent => vec!["claude-agent-acp".to_string()],
            crate::runtime::AgentHarness::Codex => unreachable!(),
        }
    }

    fn session_meta(
        self,
        system_prompt: &str,
        session_model: &str,
    ) -> Option<serde_json::Map<String, serde_json::Value>> {
        if self.harness != crate::runtime::AgentHarness::ClaudeAgent {
            return None;
        }
        serde_json::json!({
            "systemPrompt": system_prompt,
            "claudeCode": {
                "options": {
                    "settingSources": [],
                    "settings": {
                        "availableModels": [session_model],
                        "enabledPlugins": {},
                        "hooks": {}
                    },
                    "tools": CLAUDE_AGENT_TOOLS,
                    "disallowedTools": [
                        "Agent", "Task", "Skill", "WebSearch", "WebFetch", "AskUserQuestion"
                    ]
                }
            }
        })
        .as_object()
        .cloned()
    }
}

/// OMP owns model and runtime selection as top-level flags. Its `acp`
/// subcommand exposes no flags, so placing these arguments after `acp` starts
/// the server without selecting a model and fails only when the first prompt
/// arrives. Keep the subcommand last and the ordering independently testable.
fn omp_agent_command_args(model: &str, effort: &str, system_prompt_path: &str) -> Vec<String> {
    [
        "omp",
        "--model",
        model,
        "--thinking",
        effort,
        "--system-prompt",
        system_prompt_path,
        "--config",
        OMP_RUNTIME_CONFIG,
        "--no-extensions",
        "--no-rules",
        "--tools",
        OMP_AGENT_TOOLS,
        "acp",
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

/// Resolve the exact provider-qualified model value OMP advertised for this
/// session. Process flags are useful defaults for interactive OMP, but ACP
/// makes the session configuration response authoritative.
fn exact_model_config_selection(
    options: &[SessionConfigOption],
    requested_model: &str,
) -> Result<(String, String)> {
    let option = options
        .iter()
        .find(|option| {
            matches!(
                option.category.as_ref(),
                Some(SessionConfigOptionCategory::Model)
            ) || option.id.to_string() == "model"
        })
        .with_context(|| {
            format!(
                "ACP agent did not advertise a model session option; requested {requested_model}"
            )
        })?;
    let SessionConfigKind::Select(select) = &option.kind else {
        anyhow::bail!(
            "ACP model session option {} is not a select option",
            option.id
        );
    };
    let value = match &select.options {
        SessionConfigSelectOptions::Ungrouped(values) => values
            .iter()
            .find(|value| value.value.to_string() == requested_model)
            .map(|value| value.value.to_string()),
        SessionConfigSelectOptions::Grouped(groups) => groups
            .iter()
            .flat_map(|group| group.options.iter())
            .find(|value| value.value.to_string() == requested_model)
            .map(|value| value.value.to_string()),
        _ => None,
    }
    .with_context(|| {
        format!(
            "ACP agent model selector did not advertise exact requested value {requested_model}; options={}",
            serde_json::to_string(options).unwrap_or_else(|_| "<unserializable>".into())
        )
    })?;
    Ok((option.id.to_string(), value))
}

fn model_config_is_selected(
    options: &[SessionConfigOption],
    config_id: &str,
    expected_value: &str,
) -> bool {
    options.iter().any(|option| {
        option.id.to_string() == config_id
            && matches!(
                &option.kind,
                SessionConfigKind::Select(select)
                    if select.current_value.to_string() == expected_value
            )
    })
}

fn exact_named_config_selection(
    options: &[SessionConfigOption],
    config_id: &str,
    requested_value: &str,
) -> Result<(String, String)> {
    let option = options
        .iter()
        .find(|option| option.id.to_string() == config_id)
        .with_context(|| {
            format!("ACP agent did not advertise required {config_id} session option")
        })?;
    let SessionConfigKind::Select(select) = &option.kind else {
        anyhow::bail!("ACP {config_id} session option is not a select option");
    };
    let present = match &select.options {
        SessionConfigSelectOptions::Ungrouped(values) => values
            .iter()
            .any(|value| value.value.to_string() == requested_value),
        SessionConfigSelectOptions::Grouped(groups) => groups
            .iter()
            .flat_map(|group| group.options.iter())
            .any(|value| value.value.to_string() == requested_value),
        _ => false,
    };
    if !present {
        anyhow::bail!(
            "ACP agent {config_id} selector did not advertise exact requested value {requested_value}"
        );
    }
    Ok((option.id.to_string(), requested_value.to_string()))
}

/// User-relevant pieces of a live ACP turn. The completed OrgIntel message is
/// still authoritative; these events are a bounded, ephemeral owner view.
#[derive(Debug, Clone)]
pub enum LiveSessionEvent {
    ReplyDelta {
        message_id: Option<String>,
        text: String,
    },
    ThoughtDelta,
    ToolStarted {
        id: String,
        title: String,
        kind: String,
    },
    ToolUpdated {
        id: String,
        title: Option<String>,
        status: String,
    },
    GeneratedOutputTokens(u64),
    /// The runtime's current context window snapshot. This is deliberately
    /// distinct from the final generated-output total and may arrive many
    /// times during a turn.
    UsageUpdate {
        used: u64,
        size: u64,
        cost_usd: Option<f64>,
    },
}

pub type SessionObserver = Arc<dyn Fn(LiveSessionEvent) + Send + Sync>;

/// OMP is the actor runtime, not Restless's organisation layer. Keep its
/// ordinary file and shell tools, but do not expose OMP's private `task`
/// subagents: they have no OrgIntel actor, Work, message, or supervised
/// process, so using one as Staff makes a convincing transcript while making
/// the company itself blind to who did the work. Restless delegation has one
/// canon: a claimed Work Attempt.
const OMP_AGENT_TOOLS: &str = "read,bash,edit,write,grep";

/// The complete Restless-owned launch contract for one actor session.
///
/// ACP deliberately has no standard system-prompt field: it transports turns
/// and session attachments. The concrete ACP agent therefore receives the
/// actor's system prompt and native-tool policy when its process is launched,
/// while MCP servers travel through `session/new`. Keeping those concerns in
/// one value prevents Exec and Staff from quietly acquiring different prompt,
/// tool, skill, or integration semantics.
#[derive(Clone)]
pub struct AgentControls {
    system_prompt: String,
    mcp_servers: Vec<McpServer>,
    /// A team-lead conversation is a coordination session, not a productive
    /// Attempt. This is passed to the local CLI as a narrow guard against
    /// accidentally sending an unaddressed message to the owner.
    team_coordination_wake: bool,
}

impl AgentControls {
    pub fn company_actor(system_prompt: String) -> Result<Self> {
        if system_prompt.trim().is_empty() {
            anyhow::bail!("actor system prompt must not be empty");
        }
        Ok(Self {
            system_prompt,
            mcp_servers: Vec::new(),
            team_coordination_wake: false,
        })
    }

    /// Mark this as an accountable team-lead coordination wake. This does not
    /// claim filesystem isolation; the shared Runtime remains intentionally
    /// mutable. It lets the local coordination CLI reject a common accidental
    /// owner-send shape while the actor is deciding a team fact.
    pub fn for_team_coordination(mut self) -> Self {
        self.team_coordination_wake = true;
        self
    }

    /// Attach only connections selected for this actor and session. Provider
    /// discovery and credentials remain Authority concerns; this method merely
    /// carries an already-authorised ACP description to the agent.
    pub fn with_mcp_servers(mut self, mcp_servers: Vec<McpServer>) -> Self {
        self.mcp_servers = mcp_servers;
        self
    }
}

/// OMP is an implementation behind the bridge, not the owner of company
/// policy. Its profile is isolated already; this overlay also prevents host or
/// framework conventions from silently changing an actor's capabilities.
/// Standalone project `AGENTS.md`, `.agents/skills`, and the two explicit
/// company skill roots remain available because they are Runtime-owned company
/// context (§5.4), not ambient developer configuration.
const RESTLESS_OMP_CONFIG: &str = include_str!("../omp-runtime.yml");

pub(crate) async fn write_private_container_file(
    container: &str,
    path: &str,
    contents: &str,
) -> Result<()> {
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-i",
            "-u",
            "company",
            container,
            "sh",
            "-c",
            "set -eu; path=$1; dir=${path%/*}; mkdir -p \"$dir\"; umask 077; tmp=\"$path.$$\"; trap 'rm -f \"$tmp\"' EXIT; cat > \"$tmp\"; mv \"$tmp\" \"$path\"; trap - EXIT",
            "restless-write-private",
            path,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("prepare private agent file {path}"))?;
    let mut stdin = child.stdin.take().context("open agent file stdin")?;
    stdin.write_all(contents.as_bytes()).await?;
    stdin.shutdown().await?;
    drop(stdin);
    let output = child
        .wait_with_output()
        .await
        .with_context(|| format!("write private agent file {path}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "prepare private agent file {path} failed: {}",
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(300)
                .collect::<String>()
        );
    }
    Ok(())
}

async fn read_session_locator(container: &str, path: &str) -> Result<Option<SessionLocator>> {
    let output = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            container,
            "sh",
            "-c",
            "test ! -e \"$1\" || cat \"$1\"",
            "restless-read-session",
            path,
        ])
        .output()
        .await
        .with_context(|| format!("read ACP session locator {path}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "read ACP session locator {path} failed: {}",
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(300)
                .collect::<String>()
        );
    }
    let body = String::from_utf8_lossy(&output.stdout);
    if body.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(body.trim()).with_context(
        || format!("parse ACP session locator {path}"),
    )?))
}

async fn persist_session_locator(
    container: &str,
    path: &str,
    locator: &SessionLocator,
) -> Result<()> {
    let body = serde_json::to_string(locator).context("encode ACP session locator")?;
    write_private_container_file(container, path, &body).await
}

/// Forget only the hot provider session for one exact actor responsibility.
/// Durable company files, messages, Work and evidence remain the reconstruction
/// source. This is used when the provider rejects accumulated session history
/// as too large; retrying the same locator can never make that request smaller.
pub(crate) async fn discard_session_locator(
    container: &str,
    harness: crate::runtime::AgentHarness,
    company: &str,
    actor: &str,
    responsibility: &str,
) -> Result<()> {
    let path = session_locator_path(harness, company, actor, responsibility)?;
    let output = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            container,
            "sh",
            "-c",
            "test ! -e \"$1\" || unlink \"$1\"",
            "restless-discard-session",
            path.as_str(),
        ])
        .output()
        .await
        .with_context(|| format!("discard ACP session locator {path}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "discard ACP session locator {path} failed: {}",
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(300)
                .collect::<String>()
        );
    }
    Ok(())
}

/// Probe the exact short-lived coordination capability before model spend.
/// Native OMP tools are fixed by this launch's argv; this read-only call proves
/// that the same launch id can reach the coordination plane through the
/// Runtime-installed CLI. It does not consume or mark inbox state.
async fn prove_tool_contract(
    container: &str,
    profile: AcpProfile,
    auth: &AgentAuth,
    actor: &str,
) -> Result<String> {
    let mut args = vec![
        "exec".to_string(),
        "-u".to_string(),
        "company".to_string(),
        "-e".to_string(),
        auth.coordination_token_env.clone(),
        "-e".to_string(),
        format!("RESTLESS_ACTOR={actor}"),
    ];
    args.push("-e".to_string());
    args.push(format!("RESTLESS_COORDINATOR={}", runtime_coordinator()?));
    args.extend([
        container.to_string(),
        "restless".to_string(),
        "people".to_string(),
        "-c".to_string(),
        auth.company.clone(),
    ]);
    let output = tokio::process::Command::new("docker")
        .env(&auth.coordination_token_env, &auth.coordination_token)
        .args(args)
        .output()
        .await
        .context("probe actor coordination tool contract")?;
    if !output.status.success() {
        anyhow::bail!(
            "session-specific coordination readiness failed before prompt: {}",
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(500)
                .collect::<String>()
        );
    }
    Ok(format!(
        "{:x}",
        Sha256::digest(
            format!(
                "harness:{}\0native:{}\0coordination:restless-people\0actor:{actor}",
                profile.harness.as_str(),
                profile.native_tools().join(",")
            )
            .as_bytes()
        )
    ))
}

fn claude_agent_settings(session_model: &str) -> Result<String> {
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "availableModels": [session_model],
        "enabledPlugins": {},
        "hooks": {},
        "permissions": {
            "defaultMode": "default",
            "allow": [],
            "deny": [],
            "ask": []
        }
    }))?)
}

/// Install the selected harness's credential-free policy in a Restless-owned
/// directory. This never touches a company's general-purpose harness config
/// and never writes a capability or provider key to the volume.
pub(crate) async fn prepare_agent_runtime(
    container: &str,
    harness: crate::runtime::AgentHarness,
    auth: &AgentAuth,
) -> Result<()> {
    let profile = AcpProfile::new(harness)?;
    profile.session_model(&auth.model)?;
    if harness == crate::runtime::AgentHarness::RestlessManaged {
        let config = crate::model_gateway::models_config(
            &auth.model,
            &auth.gateway_url,
            &auth.gateway_token_env,
        )?;
        write_private_container_file(
            container,
            &format!("{AGENT_CONFIG_DIR}/models.yml"),
            &config,
        )
        .await?;
        write_private_container_file(container, OMP_RUNTIME_CONFIG, RESTLESS_OMP_CONFIG).await?;
    } else {
        let model = profile.session_model(&auth.model)?;
        let settings = claude_agent_settings(&model)?;
        write_private_container_file(
            container,
            &format!("{}/settings.json", profile.config_dir()),
            &settings,
        )
        .await?;
    }
    Ok(())
}

/// What one turn consumed, as the agent reported it (ACP `UsageUpdate`).
/// This is the fuse's input and the health gate's load-bearing signal: the
/// agent knows its own token count and dollar cost, so the daemon no longer
/// has to sit in the HTTP path parsing SSE tails to find out (T2 → the
/// spend spool keeps its ledger, loses its proxy).
#[derive(Debug, Clone, Copy, Default)]
pub struct TurnUsage {
    /// Tokens currently in the session context. This is a snapshot, not a
    /// delta and not a cumulative token bill.
    pub used: u64,
    /// Context window size the agent is working against.
    pub size: u64,
    /// Cost added by this wake relative to its persisted session baseline,
    /// when the provider reports a cumulative session price.
    pub cost_usd: Option<f64>,
}

/// One observed turn: the agent's visible text plus the tool calls it made.
/// This is observability, not a governed record (§4.4).
#[derive(Debug)]
pub struct TurnTranscript {
    pub text: String,
    /// The most recent assistant message in this prompt turn. Agents may emit
    /// several message blocks around tools; conversation persists only the
    /// final one while earlier blocks remain ephemeral activity.
    pub last_message_text: String,
    pub output_tokens: Option<u64>,
    pub tool_calls: Vec<String>,
    last_message_id: Option<String>,
    /// Last usage report of the turn; `None` means the agent never sent one.
    /// A completed, otherwise-empty transcript treats that like zero; text or
    /// tool activity makes it an interrupted, recoverable turn instead.
    pub usage: Option<TurnUsage>,
    /// Provider cumulative price at the start of this process. Kept private:
    /// consumers must use the per-wake delta in `usage`.
    session_cost_baseline_usd: Option<f64>,
    /// When the agent last said anything at all — the liveness signal the
    /// watchdog reads. Thought chunks count: they are not transcript content,
    /// but they are proof the model is running. A 20-minute wall-clock bound
    /// once killed a turn that had been streaming reasoning the whole time
    /// and recorded it as "the model never ran".
    last_activity: std::time::Instant,
    /// Tool calls started but not yet finished. Silence while a tool runs is
    /// a test suite or an install, not a wedge, so it is allowed far longer.
    tools_in_flight: usize,
}

impl Default for TurnTranscript {
    fn default() -> Self {
        Self {
            text: String::new(),
            last_message_text: String::new(),
            output_tokens: None,
            tool_calls: Vec::new(),
            last_message_id: None,
            usage: None,
            session_cost_baseline_usd: None,
            last_activity: std::time::Instant::now(),
            tools_in_flight: 0,
        }
    }
}

fn session_cost_delta(current: Option<f64>, baseline: Option<f64>) -> Option<f64> {
    match (current, baseline) {
        (Some(current), Some(baseline)) if current >= baseline => Some(current - baseline),
        (Some(_), Some(_)) => None,
        (Some(current), None) => Some(current),
        (None, _) => None,
    }
}

impl TurnTranscript {
    /// Evidence that the agent ran even if the ACP process ended before its
    /// final usage update. Thought-only activity deliberately does not qualify:
    /// it is a liveness pulse, but leaves no replayable transcript or durable
    /// operation from which the next wake can recover.
    #[must_use]
    pub fn has_observable_activity(&self) -> bool {
        !self.text.trim().is_empty() || !self.tool_calls.is_empty()
    }

    fn note(&mut self, update: &SessionUpdate) {
        // Every update is liveness, whatever it carries.
        self.last_activity = std::time::Instant::now();
        match update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let ContentBlock::Text(text) = &chunk.content {
                    let message_id = chunk.message_id.as_ref().map(ToString::to_string);
                    if self.last_message_id.is_some()
                        && message_id.is_some()
                        && self.last_message_id != message_id
                    {
                        self.last_message_text.clear();
                        if !self.text.is_empty() && !self.text.ends_with("\n\n") {
                            self.text.push_str("\n\n");
                        }
                    }
                    if message_id.is_some() {
                        self.last_message_id = message_id;
                    }
                    self.text.push_str(&text.text);
                    self.last_message_text.push_str(&text.text);
                }
            }
            SessionUpdate::ToolCall(call) => {
                self.tool_calls
                    .push(format!("{:?}: {}", call.kind, call.title));
                self.tools_in_flight += 1;
            }
            SessionUpdate::ToolCallUpdate(update) => {
                if matches!(
                    update.fields.status,
                    Some(ToolCallStatus::Completed) | Some(ToolCallStatus::Failed)
                ) {
                    self.tools_in_flight = self.tools_in_flight.saturating_sub(1);
                }
            }
            SessionUpdate::UsageUpdate(usage) => {
                let cumulative_cost_usd = usage.cost.as_ref().map(|cost| cost.amount);
                self.usage = Some(TurnUsage {
                    used: usage.used,
                    size: usage.size,
                    cost_usd: session_cost_delta(
                        cumulative_cost_usd,
                        self.session_cost_baseline_usd,
                    ),
                });
            }
            _ => {}
        }
    }
}

fn live_event(update: &SessionUpdate) -> Option<LiveSessionEvent> {
    match update {
        SessionUpdate::AgentMessageChunk(chunk) => match &chunk.content {
            ContentBlock::Text(text) => Some(LiveSessionEvent::ReplyDelta {
                message_id: chunk.message_id.as_ref().map(ToString::to_string),
                text: text.text.clone(),
            }),
            _ => None,
        },
        SessionUpdate::AgentThoughtChunk(chunk) => match &chunk.content {
            ContentBlock::Text(_) => Some(LiveSessionEvent::ThoughtDelta),
            _ => None,
        },
        SessionUpdate::ToolCall(call) => Some(LiveSessionEvent::ToolStarted {
            id: call.tool_call_id.to_string(),
            title: call.title.clone(),
            kind: format!("{:?}", call.kind).to_lowercase(),
        }),
        SessionUpdate::ToolCallUpdate(update) => Some(LiveSessionEvent::ToolUpdated {
            id: update.tool_call_id.to_string(),
            title: update.fields.title.clone(),
            status: update
                .fields
                .status
                .map(|status| format!("{status:?}").to_lowercase())
                .unwrap_or_else(|| "active".into()),
        }),
        // Usage needs the persisted session baseline before it is safe for an
        // owner surface. The notification handler emits the delta after
        // `TurnTranscript::note`; raw cumulative provider cost never escapes.
        SessionUpdate::UsageUpdate(_) => None,
        _ => None,
    }
}

/// A live agent process and connection, mid-turn.
pub struct AgentSession {
    cx: ConnectionTo<Agent>,
    pub session_id: SessionId,
    transcript: Arc<Mutex<TurnTranscript>>,
    observer: Option<SessionObserver>,
    live_observer_enabled: Arc<AtomicBool>,
    pub launch_id: String,
    pub harness: crate::runtime::AgentHarness,
    pub harness_build: String,
    pub transport: String,
    pub model: String,
    pub effort: String,
    pub resumed: bool,
    pub reconstructed: bool,
    pub reconstruction_reason: Option<String>,
    pub tool_contract_digest: String,
    pub capabilities: serde_json::Value,
}

pub(crate) fn required_readiness_text<'a>(
    readiness: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str> {
    readiness
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("agent readiness omitted required {field}"))
}

pub(crate) fn required_readiness_bool(readiness: &serde_json::Value, field: &str) -> Result<bool> {
    readiness
        .get(field)
        .and_then(serde_json::Value::as_bool)
        .with_context(|| format!("agent readiness omitted required {field}"))
}

/// Silence with nothing running: the agent is wedged.
///
/// Raised from 120s after it killed a Kimi turn 50 minutes into real work. The
/// original number was picked against glm-5.2, which streams reasoning almost
/// continuously; a model that pauses longer between outputs is not wedged, and
/// the cost of the two errors is wildly asymmetric — waiting five more minutes
/// on a genuinely dead agent is free, while a false kill destroys an hour of
/// work and reports it as a failure. When in doubt, wait.
const IDLE_SILENT: std::time::Duration = std::time::Duration::from_secs(8 * 60);
/// Silence while a tool call is in flight: a test suite, an install, a build.
/// Bounded, but generously.
const IDLE_TOOL_RUNNING: std::time::Duration = std::time::Duration::from_secs(15 * 60);
/// How often the watchdog looks. Cheap: it reads two in-memory values.
const WATCHDOG_TICK: std::time::Duration = std::time::Duration::from_secs(5);

/// **Every** way a turn can end, as one closed set.
///
/// This type exists because its absence cost us the same bug three times. The
/// old shape was `Result<Option<TurnHalt>>` plus a separate
/// `classify_turn(Option<u64>, Option<&str>)`, and both halves lost the thing
/// that mattered: `None` meant "unknown" in one caller and "zero" in the next,
/// so correct interpretation lived in each call site's head rather than in the
/// types. Three call sites forgot, each in its own way, and each time the
/// generic "the model never ran" verdict overwrote a specific and true one —
/// sending the owner to check a healthy credential while the real cause (a
/// 20-minute boundary, a wedge 50 minutes into real work, an exhausted
/// ceiling) went unreported.
///
/// So: a turn ends by producing one of these, always, and the only way to read
/// one is `health::classify`, which is total over the set. A new way for a turn
/// to end is a new variant, and adding it fails compilation everywhere until
/// someone says what it means. That is the whole point — the discipline moved
/// from memory into the compiler.
///
/// Every variant carries its transcript: work that happened before a halt is
/// still on disk and still worth reporting, and a failure with the agent's own
/// last words is debuggable where a bare error string is not.
#[derive(Debug)]
pub enum TurnEnd {
    /// The agent finished the turn itself. Whether it *did* anything is a
    /// separate question, answered from the usage — see `health::classify`.
    Completed { transcript: TurnTranscript },
    /// No output of any kind for longer than the idle allowance.
    Wedged {
        idle: std::time::Duration,
        transcript: TurnTranscript,
    },
    /// The company reached its ceiling mid-turn.
    OverBudget { transcript: TurnTranscript },
    /// The owner deliberately interrupted this session to send a new
    /// direction. Files already written remain on the persistent Runtime.
    Interrupted { transcript: TurnTranscript },
    /// The session or its transport failed. `error` is the anyhow chain.
    Failed {
        error: String,
        transcript: TurnTranscript,
    },
}

impl TurnEnd {
    /// The transcript, whichever way the turn ended.
    #[must_use]
    pub fn transcript(&self) -> &TurnTranscript {
        match self {
            Self::Completed { transcript }
            | Self::Wedged { transcript, .. }
            | Self::OverBudget { transcript }
            | Self::Interrupted { transcript }
            | Self::Failed { transcript, .. } => transcript,
        }
    }

    /// Take the transcript, consuming the end.
    #[must_use]
    pub fn into_transcript(self) -> TurnTranscript {
        match self {
            Self::Completed { transcript }
            | Self::Wedged { transcript, .. }
            | Self::OverBudget { transcript }
            | Self::Interrupted { transcript }
            | Self::Failed { transcript, .. } => transcript,
        }
    }

    /// What the turn consumed, if the agent reported it. `None` here is
    /// genuinely "the agent never told us" — it is not zero, and only the
    /// `Completed` arm of `health::classify` is entitled to read it as a
    /// failure tell.
    #[must_use]
    pub fn usage(&self) -> Option<TurnUsage> {
        self.transcript().usage
    }
}

impl AgentSession {
    pub fn readiness_observation(&self) -> serde_json::Value {
        serde_json::json!({
            "launch_id": self.launch_id,
            "harness": self.harness,
            "harness_build": self.harness_build,
            "transport": self.transport,
            "model": self.model,
            "configured_effort": self.effort,
            "session_id": self.session_id.to_string(),
            "resumed": self.resumed,
            "reconstructed": self.reconstructed,
            "reconstruction_reason": self.reconstruction_reason,
            "tool_contract_digest": self.tool_contract_digest,
            "capabilities": self.capabilities,
            "fresh_process_capability": true,
        })
    }

    /// Activity is owner-visible only while the agent works the requested
    /// outcome. The deterministic private termination envelope shares the ACP
    /// session but must never be rendered as the agent's public reply.
    pub fn set_live_observer_enabled(&self, enabled: bool) {
        self.live_observer_enabled.store(enabled, Ordering::Release);
    }

    /// Send one prompt and let it run for as long as it is *alive*, rather
    /// than for a fixed wall-clock budget.
    ///
    /// A total timeout cannot distinguish "wedged" from "working hard": it is
    /// simultaneously far too slow to catch a hung tool (20 minutes) and far
    /// too fast for real work. An idle timer is better on both axes — it
    /// catches a wedge in ~2 minutes and lets a legitimately slow turn run as
    /// long as it keeps producing.
    ///
    /// The outer bound on runaway is therefore money, not time: `budget`
    /// returns true when the company has spent its ceiling, checked each tick
    /// against the usage the agent reports as it goes. "$3 with nothing to
    /// show" is a meaningful statement about waste; "20 minutes elapsed" is
    /// not.
    ///
    /// Returns a [`TurnEnd`] and **not** a `Result`: a transport failure is one
    /// of the ways a turn ends, not an exception to a turn ending. Handing back
    /// a `Result` is what let callers `?` past the classifier and reach for
    /// their own reading of what went wrong. There is no such escape now — the
    /// only thing to do with a `TurnEnd` is classify it.
    pub async fn prompt_live(
        &self,
        text: &str,
        budget: impl Fn(&TurnUsage) -> bool + Send,
        cancellation: &CancellationToken,
    ) -> TurnEnd {
        let prompt = self.prompt(text);
        tokio::pin!(prompt);
        loop {
            tokio::select! {
                () = cancellation.cancelled() => {
                    let _ = self.cancel().await;
                    return TurnEnd::Interrupted { transcript: self.take_transcript() };
                }
                finished = &mut prompt => {
                    return match finished {
                        Ok(response) => {
                            let output_tokens = response.usage.map(|usage| usage.output_tokens);
                            if let Some(tokens) = output_tokens {
                                if self.live_observer_enabled.load(Ordering::Acquire) {
                                    if let Some(observer) = &self.observer {
                                        observer(LiveSessionEvent::GeneratedOutputTokens(tokens));
                                    }
                                }
                            }
                            let mut transcript = self.take_transcript();
                            transcript.output_tokens = output_tokens;
                            TurnEnd::Completed { transcript }
                        },
                        Err(error) => TurnEnd::Failed {
                            error: format!("{error:#}"),
                            transcript: self.take_transcript(),
                        },
                    };
                }
                () = tokio::time::sleep(WATCHDOG_TICK) => {
                    // The guard must not cross an await: read three plain
                    // values out of it and let it drop with the block.
                    let Some((idle, allowance, over)) = ({
                        match self.transcript.lock() {
                            Err(_) => None,
                            Ok(transcript) => Some((
                                transcript.last_activity.elapsed(),
                                if transcript.tools_in_flight > 0 {
                                    IDLE_TOOL_RUNNING
                                } else {
                                    IDLE_SILENT
                                },
                                transcript.usage.as_ref().is_some_and(&budget),
                            )),
                        }
                    }) else { continue };
                    if over {
                        let _ = self.cancel().await;
                        return TurnEnd::OverBudget { transcript: self.take_transcript() };
                    }
                    if idle > allowance {
                        tracing::warn!(
                            idle_secs = idle.as_secs(),
                            "agent produced nothing for longer than the idle allowance"
                        );
                        let _ = self.cancel().await;
                        return TurnEnd::Wedged { idle, transcript: self.take_transcript() };
                    }
                }
            }
        }
    }

    /// Send one prompt and wait for the turn to complete.
    pub async fn prompt(&self, text: &str) -> Result<PromptResponse> {
        self.cx
            .send_request(PromptRequest::new(
                self.session_id.clone(),
                vec![ContentBlock::Text(TextContent::new(text.to_string()))],
            ))
            .block_task()
            .await
            .context("acp session/prompt")
    }

    /// Ask the agent to stop mid-turn (best-effort; the process is killed
    /// when the session guard drops regardless).
    pub async fn cancel(&self) -> Result<()> {
        self.cx
            .send_notification(CancelNotification::new(self.session_id.clone()))
            .map_err(|error| anyhow::anyhow!(error.to_string()))
    }

    /// Read the accumulated transcript, leaving it empty.
    pub fn take_transcript(&self) -> TurnTranscript {
        self.transcript
            .lock()
            .map(|mut guard| {
                let baseline = guard.session_cost_baseline_usd;
                let transcript = std::mem::take(&mut *guard);
                guard.session_cost_baseline_usd = baseline;
                transcript
            })
            .unwrap_or_default()
    }
}

/// Spawn OMP's ACP server inside the company container, authenticate against the
/// gateway, open a session rooted at `workdir`, and hand it to `drive`. The
/// process dies when the returned future completes — agents are ordinary
/// processes, not daemons (§5). `actor` becomes RESTLESS_ACTOR in the
/// process env, so the CLI the agent shells out to knows who is reporting
/// (T10); RESTLESS_COMPANY and the coordinator address come from the
/// container's own env.
#[expect(
    clippy::too_many_arguments,
    reason = "the ACP launch boundary keeps identity, responsibility, authority and observation explicit"
)]
pub async fn with_agent<F, T>(
    container: &str,
    harness: crate::runtime::AgentHarness,
    auth: &AgentAuth,
    workdir: &str,
    actor: &str,
    responsibility: &str,
    controls: AgentControls,
    observer: Option<SessionObserver>,
    drive: F,
) -> Result<T>
where
    F: for<'a> FnOnce(
        &'a AgentSession,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<T>> + Send + 'a>,
    >,
{
    if responsibility.trim().is_empty() {
        anyhow::bail!("ACP session responsibility scope must not be empty");
    }
    let profile = AcpProfile::new(harness)?;
    prepare_agent_runtime(container, harness, auth).await?;
    let session_model = profile.session_model(&auth.model)?;
    let session_meta = profile.session_meta(&controls.system_prompt, &session_model);
    let locator_path = session_locator_path(harness, &auth.company, actor, responsibility)?;
    let prior_locator = read_session_locator(container, &locator_path).await?;
    if let Some(locator) = &prior_locator {
        validate_session_locator(locator, profile, &auth.company, actor, responsibility)?;
    }
    // Every docker-exec process is a Linux session leader. Record this turn's
    // session id inside the container so cleanup can reap only its process
    // tree. A before/after PID diff is not ownership: a staff turn may start
    // while the Exec is running, and the Exec must never kill it on exit.
    let launch_id = auth.session_id.clone();
    let session_marker = format!("/tmp/restless-agent-{launch_id}.sid");
    let system_prompt_path = format!("/tmp/restless-agent-{launch_id}.system.md");
    let session_runtime = format!("/company/run/agent-sessions/{launch_id}");
    let runtime_dirs = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            container,
            "mkdir",
            "-p",
            &format!("{session_runtime}/cache"),
            &format!("{session_runtime}/tmp"),
        ])
        .output()
        .await
        .context("prepare external agent cache directories")?;
    if !runtime_dirs.status.success() {
        anyhow::bail!(
            "prepare external agent cache directories: {}",
            String::from_utf8_lossy(&runtime_dirs.stderr)
        );
    }
    write_private_container_file(container, &system_prompt_path, &controls.system_prompt).await?;
    // docker exec takes its -e flags BEFORE the container name; anything after
    // it belongs to the command. Build the vector explicitly rather than
    // chaining .args() and hoping the order is right — appending the base-URL
    // override after the container silently handed it to omp as an argument,
    // which surfaced as "No model selected".
    let mut args = agent_exec_prefix(workdir);
    args.push("-e".to_string());
    args.push(match profile.harness {
        crate::runtime::AgentHarness::RestlessManaged => {
            format!("PI_CODING_AGENT_DIR={AGENT_CONFIG_DIR}")
        }
        crate::runtime::AgentHarness::ClaudeAgent => {
            format!("CLAUDE_CONFIG_DIR={CLAUDE_AGENT_CONFIG_DIR}")
        }
        crate::runtime::AgentHarness::Codex => unreachable!(),
    });
    args.push("-e".to_string());
    args.push(format!("RESTLESS_ACTOR={actor}"));
    if controls.team_coordination_wake {
        args.push("-e".to_string());
        args.push("RESTLESS_COORDINATION_WAKE=1".to_string());
    }
    args.push("-e".to_string());
    args.push(format!("RESTLESS_COORDINATOR={}", runtime_coordinator()?));
    args.push("-e".to_string());
    args.push(format!("XDG_CACHE_HOME={session_runtime}/cache"));
    args.push("-e".to_string());
    args.push(format!("TMPDIR={session_runtime}/tmp"));
    args.push("-e".to_string());
    args.push(auth.coordination_token_env.clone());
    args.push("-e".to_string());
    // `docker exec -e NAME` copies NAME from the docker client's environment.
    // Passing NAME=VALUE in argv would expose the scoped model capability to
    // host process listings during session bootstrap.
    if harness == crate::runtime::AgentHarness::ClaudeAgent {
        args.push("ANTHROPIC_AUTH_TOKEN".to_string());
        for value in [
            format!("ANTHROPIC_BASE_URL={}", auth.gateway_url),
            "ANTHROPIC_API_KEY=".to_string(),
            "CLAUDE_CODE_OAUTH_TOKEN=".to_string(),
        ] {
            args.push("-e".to_string());
            args.push(value);
        }
    } else {
        args.push(auth.gateway_token_env.clone());
    }
    args.extend(
        [
            container,
            "sh",
            "-c",
            // Productive files stay private to the company group, whose only
            // other member is the isolated governed-effect UID. A 077 umask
            // made Git metadata unreadable to the exact CLI process that had
            // been authorised to publish it.
            "umask 007; printf '%s\\n' \"$$\" > \"$1\"; shift; exec \"$@\"",
            "restless-agent",
            session_marker.as_str(),
        ]
        .iter()
        .map(|arg| (*arg).to_string()),
    );
    args.extend(profile.command_args(&auth.model, &auth.effort, &system_prompt_path));
    let spawned = tokio::process::Command::new("docker")
        .env(&auth.coordination_token_env, &auth.coordination_token)
        .env(&auth.gateway_token_env, &auth.gateway_token)
        .env("ANTHROPIC_AUTH_TOKEN", &auth.gateway_token)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            let _ = tokio::process::Command::new("docker")
                .args(["exec", container, "unlink", &system_prompt_path])
                .output()
                .await;
            return Err(error).context("spawn ACP agent in container");
        }
    };
    let stdin = child.stdin.take().expect("piped stdin");
    let stdout = child.stdout.take().expect("piped stdout");
    let transport = ByteStreams::new(stdin.compat_write(), stdout.compat());

    let transcript = Arc::new(Mutex::new(TurnTranscript::default()));
    let sink = Arc::clone(&transcript);
    let capture_notifications = Arc::new(AtomicBool::new(false));
    let notification_capture = Arc::clone(&capture_notifications);
    let live_locator = Arc::new(Mutex::new(None::<SessionLocator>));
    let notification_locator = Arc::clone(&live_locator);
    let notification_container = container.to_string();
    let notification_locator_path = locator_path.clone();
    let event_observer = observer.clone();
    let live_observer_enabled = Arc::new(AtomicBool::new(true));
    let observer_enabled = Arc::clone(&live_observer_enabled);
    let workdir = workdir.to_string();
    let mcp_servers = controls.mcp_servers;
    let mcp_server_count = mcp_servers.len();
    let responsibility = responsibility.to_string();
    let launch_auth = auth.clone();
    let launch_profile = profile;
    let launch_session_model = session_model;
    let launch_session_meta = session_meta;
    let launch_container = container.to_string();
    let launch_actor = actor.to_string();
    let launch_id_for_session = launch_id.clone();
    let session_locator_path = locator_path.clone();
    // connect_with speaks acp::Error; the real anyhow chain is parked here
    // and restored after the connection closes.
    let failure = Arc::new(Mutex::new(None::<anyhow::Error>));
    let failure_slot = Arc::clone(&failure);

    let result = Client
        .builder()
        .on_receive_notification(
            move |notification: SessionNotification, _cx| {
                let sink = Arc::clone(&sink);
                let event_observer = event_observer.clone();
                let observer_enabled = Arc::clone(&observer_enabled);
                let capture_notifications = Arc::clone(&notification_capture);
                let live_locator = Arc::clone(&notification_locator);
                let locator_container = notification_container.clone();
                let locator_path = notification_locator_path.clone();
                async move {
                    // session/load may replay historical notifications to
                    // reconstruct the agent. They are not current work,
                    // usage, owner text, or organisational activity.
                    if !capture_notifications.load(Ordering::Acquire) {
                        return Ok(());
                    }
                    let mut event = live_event(&notification.update);
                    if let Ok(mut transcript) = sink.lock() {
                        transcript.note(&notification.update);
                        if matches!(&notification.update, SessionUpdate::UsageUpdate(_)) {
                            event = transcript.usage.map(|usage| LiveSessionEvent::UsageUpdate {
                                used: usage.used,
                                size: usage.size,
                                cost_usd: usage.cost_usd,
                            });
                        }
                    }
                    if observer_enabled.load(Ordering::Acquire) {
                        if let (Some(observer), Some(event)) = (event_observer.as_ref(), event) {
                            observer(event);
                        }
                    }
                    if let SessionUpdate::UsageUpdate(usage) = &notification.update {
                        let updated = if let Ok(mut current) = live_locator.lock() {
                            current.as_mut().map(|locator| {
                                if let Some(observed) = usage.cost.as_ref().map(|cost| cost.amount) {
                                    if locator
                                        .cumulative_cost_usd
                                        .is_none_or(|previous| observed >= previous)
                                    {
                                        locator.cumulative_cost_usd = Some(observed);
                                    }
                                }
                                locator.clone()
                            })
                        } else {
                            None
                        };
                        if let Some(locator) = updated {
                            if let Err(error) = persist_session_locator(
                                &locator_container,
                                &locator_path,
                                &locator,
                            )
                            .await
                            {
                                tracing::warn!(%error, "could not persist cumulative ACP usage");
                            }
                        }
                    }
                    Ok(())
                }
            },
            acp::on_receive_notification!(),
        )
        .on_receive_request(
            move |request: RequestPermissionRequest,
                  responder: acp::Responder<RequestPermissionResponse>,
                  _cx| async move {
                // Permissive inside the company environment (§2.1): pick the
                // first allow-kind option, else the first option.
                let option = request
                    .options
                    .iter()
                    .find(|option| {
                        matches!(
                            option.kind,
                            PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
                        )
                    })
                    .or_else(|| request.options.first());
                let outcome = match option {
                    Some(option) => RequestPermissionOutcome::Selected(
                        SelectedPermissionOutcome::new(option.option_id.clone()),
                    ),
                    None => RequestPermissionOutcome::Cancelled,
                };
                responder.respond(RequestPermissionResponse::new(outcome))
            },
            acp::on_receive_request!(),
        )
        .connect_with(transport, async move |cx: ConnectionTo<Agent>| {
            let step = async {
                // Client capabilities are left at their defaults, and that is
                // load-bearing: advertising `fs.writeTextFile` tells the agent
                // to route file writes back over ACP to us. Probed live — with
                // fs advertised, omp narrates the write and no file appears,
                // which reads exactly like the model ignoring instructions.
                // Agents here work directly on the company volume, so the
                // client must decline fs. See `declines_client_side_filesystem`.
                let initialized = cx
                    .send_request(InitializeRequest::new(ProtocolVersion::V1))
                    .block_task()
                    .await
                    .context("acp initialize")?;
                if launch_profile.harness == crate::runtime::AgentHarness::ClaudeAgent {
                    let info = initialized.agent_info.as_ref().context(
                        "Claude Agent ACP initialize omitted its adapter build identity",
                    )?;
                    if info.name != "@agentclientprotocol/claude-agent-acp"
                        || info.version != "0.73.0"
                    {
                        anyhow::bail!(
                            "Claude Agent ACP build mismatch: observed {} v{}, required @agentclientprotocol/claude-agent-acp v0.73.0",
                            info.name,
                            info.version
                        );
                    }
                }
                let agent_name = initialized
                    .agent_info
                    .as_ref()
                    .map(|info| format!("{} v{}", info.name, info.version))
                    .unwrap_or_else(|| "unknown".to_string());
                tracing::info!(agent = %agent_name, "acp agent initialized");

                let (
                    session_id,
                    resumed,
                    reconstructed,
                    reconstruction_reason,
                    baseline_cost,
                    config_options,
                ) = match prior_locator {
                        Some(prior)
                            if session_locator_is_reusable(
                                &prior,
                                launch_profile,
                                &workdir,
                                &launch_auth.model,
                                &launch_auth.effort,
                                initialized.agent_capabilities.load_session,
                            ) =>
                        {
                            let prior_id: SessionId = prior.session_id.clone().into();
                            match cx
                                .send_request(
                                    LoadSessionRequest::new(prior_id.clone(), workdir.clone())
                                        .mcp_servers(mcp_servers.clone())
                                        .meta(launch_session_meta.clone()),
                                )
                                .block_task()
                                .await
                            {
                                Ok(loaded) => (
                                    prior_id,
                                    true,
                                    false,
                                    None,
                                    prior.cumulative_cost_usd,
                                    loaded.config_options.unwrap_or_default(),
                                ),
                                Err(error) => {
                                    let session = cx
                                        .send_request(
                                            NewSessionRequest::new(workdir.clone())
                                                .mcp_servers(mcp_servers.clone())
                                                .meta(launch_session_meta.clone()),
                                        )
                                        .block_task()
                                        .await
                                        .context("acp session/new after failed load")?;
                                    (
                                        session.session_id,
                                        false,
                                        true,
                                        Some(format!("session/load failed: {error}")),
                                        None,
                                        session.config_options.unwrap_or_default(),
                                    )
                                }
                            }
                        }
                        Some(prior) => {
                            let reason = if prior.cwd != workdir {
                                format!(
                                    "workspace changed from {} to {}; prior provider session is not reusable",
                                    prior.cwd, workdir
                                )
                            } else if prior.model != launch_auth.model {
                                format!(
                                    "model changed from {} to {}; prior provider session is not reusable",
                                    prior.model, launch_auth.model
                                )
                            } else if prior.effort != launch_auth.effort {
                                format!(
                                    "reasoning effort changed from {} to {}; prior provider session is not reusable",
                                    prior.effort, launch_auth.effort
                                )
                            } else {
                                "ACP agent does not advertise session/load".to_string()
                            };
                            let session = cx
                                .send_request(
                                    NewSessionRequest::new(workdir.clone())
                                        .mcp_servers(mcp_servers.clone())
                                        .meta(launch_session_meta.clone()),
                                )
                                .block_task()
                                .await
                                .context("acp session/new for explicit reconstruction")?;
                            (
                                session.session_id,
                                false,
                                true,
                                Some(reason),
                                None,
                                session.config_options.unwrap_or_default(),
                            )
                        }
                        None => {
                            let session = cx
                                .send_request(
                                    NewSessionRequest::new(workdir.clone())
                                        .mcp_servers(mcp_servers.clone())
                                        .meta(launch_session_meta.clone()),
                                )
                                .block_task()
                                .await
                                .context("acp session/new")?;
                            (
                                session.session_id,
                                false,
                                false,
                                None,
                                None,
                                session.config_options.unwrap_or_default(),
                            )
                        }
                    };

                let (model_config_id, model_value) =
                    exact_model_config_selection(&config_options, &launch_session_model)
                        .context("select exact ACP session model")?;
                let configured = cx
                    .send_request(SetSessionConfigOptionRequest::new(
                        session_id.clone(),
                        model_config_id.clone(),
                        model_value.as_str(),
                    ))
                    .block_task()
                    .await
                    .context("acp session/set_config_option for exact model")?;
                if !model_config_is_selected(
                    &configured.config_options,
                    &model_config_id,
                    &model_value,
                ) {
                    anyhow::bail!(
                        "ACP agent did not confirm exact selected model {}",
                        launch_auth.model
                    );
                }
                let mut confirmed_options = configured.config_options;
                if launch_profile.harness == crate::runtime::AgentHarness::ClaudeAgent {
                    let (effort_id, effort_value) = exact_named_config_selection(
                        &confirmed_options,
                        "effort",
                        &launch_auth.effort,
                    )
                    .context("select exact Claude Agent effort")?;
                    confirmed_options = cx
                        .send_request(SetSessionConfigOptionRequest::new(
                            session_id.clone(),
                            effort_id.clone(),
                            effort_value.as_str(),
                        ))
                        .block_task()
                        .await
                        .context("acp session/set_config_option for exact effort")?
                        .config_options;
                    if !model_config_is_selected(
                        &confirmed_options,
                        &effort_id,
                        &effort_value,
                    ) {
                        anyhow::bail!("Claude Agent did not confirm exact selected effort");
                    }

                    // The adapter consults project settings while constructing a
                    // session. Reasserting the ordinary permission mode before
                    // the first prompt prevents a hostile project default from
                    // turning into ambient bypass or plan-only behavior.
                    let (mode_id, mode_value) =
                        exact_named_config_selection(&confirmed_options, "mode", "default")
                            .context("select isolated Claude Agent permission mode")?;
                    confirmed_options = cx
                        .send_request(SetSessionConfigOptionRequest::new(
                            session_id.clone(),
                            mode_id.clone(),
                            mode_value.as_str(),
                        ))
                        .block_task()
                        .await
                        .context("acp session/set_config_option for isolated mode")?
                        .config_options;
                    if !model_config_is_selected(&confirmed_options, &mode_id, &mode_value) {
                        anyhow::bail!("Claude Agent did not confirm isolated permission mode");
                    }
                }

                let locator = SessionLocator {
                    version: 2,
                    harness: launch_profile.harness,
                    harness_build: launch_profile.build().to_string(),
                    company: launch_auth.company.clone(),
                    actor: launch_actor.clone(),
                    responsibility: responsibility.clone(),
                    cwd: workdir.clone(),
                    model: launch_auth.model.clone(),
                    effort: launch_auth.effort.clone(),
                    session_id: session_id.to_string(),
                    cumulative_cost_usd: baseline_cost,
                };
                persist_session_locator(&launch_container, &session_locator_path, &locator)
                    .await
                    .context("persist ACP session scope before prompt")?;
                if let Ok(mut active) = live_locator.lock() {
                    *active = Some(locator);
                }
                if let Ok(mut current) = transcript.lock() {
                    current.session_cost_baseline_usd = baseline_cost;
                }
                let tool_contract_digest = prove_tool_contract(
                    &launch_container,
                    launch_profile,
                    &launch_auth,
                    &launch_actor,
                )
                .await?;
                let capabilities = serde_json::json!({
                    "native_tools": launch_profile.native_tools(),
                    "native_agent_build": launch_profile.native_agent_build(),
                    "mcp_server_count": mcp_server_count,
                    "session_load": initialized.agent_capabilities.load_session,
                    "model_selection": "exact",
                    "effort_selection": if launch_profile.harness == crate::runtime::AgentHarness::ClaudeAgent { "exact_acp" } else { "exact_process_flag" },
                    "permission_mode": if launch_profile.harness == crate::runtime::AgentHarness::ClaudeAgent { "default_reasserted" } else { "runtime_sandbox" },
                    "tariff_version": launch_profile.tariff_version(),
                });
                capture_notifications.store(true, Ordering::Release);
                tracing::info!(
                    actor = %launch_actor,
                    responsibility = %responsibility,
                    launch_id = %launch_id_for_session,
                    session_id = %session_id,
                    resumed,
                    reconstructed,
                    tool_contract_digest = %tool_contract_digest,
                    "ACP session ready before production prompt"
                );

                let agent = AgentSession {
                    cx,
                    session_id,
                    transcript,
                    observer,
                    live_observer_enabled,
                    launch_id: launch_id_for_session,
                    harness: launch_profile.harness,
                    harness_build: launch_profile.build().to_string(),
                    transport: launch_profile.transport().to_string(),
                    model: launch_auth.model.clone(),
                    effort: launch_auth.effort.clone(),
                    resumed,
                    reconstructed,
                    reconstruction_reason,
                    tool_contract_digest,
                    capabilities,
                };
                drive(&agent).await
            };
            match step.await {
                Ok(outcome) => Ok(outcome),
                Err(error) => {
                    let message = format!("{error:#}");
                    if let Ok(mut slot) = failure_slot.lock() {
                        *slot = Some(error);
                    }
                    Err(acp::Error::internal_error().data(serde_json::json!(message)))
                }
            }
        })
        .await;

    let owned_session = read_session_id(container, &session_marker).await;
    let _ = child.kill().await;
    let process_cleanup = if let Some(session_id) = owned_session {
        let _ = reap_session(container, &session_id).await;
        verify_session_reaped(container, &session_id).await
    } else {
        Err(anyhow::anyhow!(
            "agent session ownership marker {session_marker} is missing; broad process cleanup was refused"
        ))
    };
    let secret_cleanup = async {
        purge_exact_secret_residue(container, profile.config_dir(), &auth.gateway_token).await?;
        purge_exact_secret_residue(container, profile.config_dir(), &auth.coordination_token).await
    }
    .await;
    let artifact_cleanup = remove_and_verify_session_artifacts(
        container,
        &[&session_marker, &system_prompt_path, &session_runtime],
    )
    .await;
    if let Ok(mut slot) = failure.lock() {
        if let Some(error) = slot.take() {
            if let Err(cleanup) = process_cleanup.and(secret_cleanup).and(artifact_cleanup) {
                tracing::error!(
                    container,
                    "agent failed and terminal cleanup also failed: {cleanup:#}"
                );
            }
            return Err(error);
        }
    }
    process_cleanup?;
    secret_cleanup?;
    artifact_cleanup?;
    Ok(result?)
}

/// Docker's process cwd and ACP's `session/new` cwd must agree. Otherwise an
/// agent can be told that it is reviewing a detached copy while its shell
/// tools still start in `/company`, which silently reopens the source
/// worktree mutation path Sprint 14 is closing.
pub(crate) fn agent_exec_prefix(workdir: &str) -> Vec<String> {
    let mut args = [
        "exec".to_string(),
        "-i".to_string(),
        "-u".to_string(),
        "company".to_string(),
        "-w".to_string(),
        workdir.to_string(),
        "-e".to_string(),
        "NO_BROWSER=1".to_string(),
    ]
    .to_vec();
    if workdir.starts_with("/company/reviews/") {
        for value in [
            "GIT_CONFIG_COUNT=1".to_string(),
            "GIT_CONFIG_KEY_0=safe.directory".to_string(),
            format!("GIT_CONFIG_VALUE_0={workdir}"),
        ] {
            args.push("-e".to_string());
            args.push(value);
        }
    }
    args
}

/// Read and validate the Linux session id written by this turn's wrapper.
pub(crate) async fn read_session_id(container: &str, marker: &str) -> Option<String> {
    let Ok(output) = crate::runtime::docker_bounded(
        &["exec", container, "cat", marker],
        std::time::Duration::from_secs(8),
    )
    .await
    else {
        return None;
    };
    let session_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success()
        && session_id != "1"
        && !session_id.is_empty()
        && session_id.bytes().all(|byte| byte.is_ascii_digit()))
    .then_some(session_id)
}

pub(crate) async fn remove_and_verify_session_artifacts(
    container: &str,
    paths: &[&str],
) -> Result<()> {
    let mut args = [
        "exec",
        "-u",
        "root",
        container,
        "sh",
        "-c",
        "set -eu; for path in \"$@\"; do rm -rf \"$path\"; done; for path in \"$@\"; do test ! -e \"$path\"; test ! -L \"$path\"; done",
        "session-artifact-cleanup",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    args.extend(paths.iter().map(|path| (*path).to_string()));
    let output = tokio::process::Command::new("docker")
        .args(args)
        .output()
        .await
        .context("clean exact agent session artifacts")?;
    if !output.status.success() {
        anyhow::bail!(
            "clean exact agent session artifacts: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Scrub any profile file that captured one exact short-lived capability and
/// verify the value is absent before the launch is considered cleaned up.
/// The secret travels in docker-exec's environment, never its argv.
pub(crate) async fn purge_exact_secret_residue(
    container: &str,
    profile_root: &str,
    secret: &str,
) -> Result<()> {
    if secret.is_empty() {
        anyhow::bail!("refusing to purge an empty capability value");
    }
    let output = tokio::process::Command::new("docker")
        .env("RESTLESS_PURGE_SECRET", secret)
        .args([
            "exec",
            "-u",
            "company",
            "-e",
            "RESTLESS_PURGE_SECRET",
            container,
            "sh",
            "-c",
            "set -eu; root=$1; test -d \"$root\" || exit 0; find \"$root\" -type f -size -4194304c -exec sh -c 'for file do if grep -qF -- \"$RESTLESS_PURGE_SECRET\" \"$file\"; then : > \"$file\"; fi; done' sh {} +; ! grep -RIlF -- \"$RESTLESS_PURGE_SECRET\" \"$root\" >/dev/null 2>&1",
            "restless-purge-capability",
            profile_root,
        ])
        .output()
        .await
        .context("purge agent capability residue")?;
    if !output.status.success() {
        anyhow::bail!("agent profile retained a scoped capability after cleanup");
    }
    Ok(())
}

pub(crate) async fn verify_session_reaped(container: &str, session_id: &str) -> Result<()> {
    let output = tokio::process::Command::new("docker")
        .args(["exec", container, "ps", "-eo", "pid=,sid="])
        .output()
        .await
        .context("verify exact agent process session cleanup")?;
    if !output.status.success() {
        anyhow::bail!("could not observe agent process sessions after cleanup");
    }
    let residue = pids_in_session(&String::from_utf8_lossy(&output.stdout), session_id);
    if !residue.is_empty() {
        anyhow::bail!(
            "agent session {session_id} retained {} processes after cleanup",
            residue.len()
        );
    }
    Ok(())
}

/// Kill whatever this turn's Linux session started and left behind.
///
/// The persistent company computer (§5, §17 step 2) is the right model, but
/// the per-turn disposable sandbox it replaced was doing garbage collection
/// for free and nothing took over. Observed live: a Chromium GPU process the
/// Exec launched to verify its game sat at **908% CPU for 2h25m** after the
/// wake ended, alongside two abandoned static servers. It starved every
/// concurrently-running company — which is why three companies had never once
/// run well at the same time.
///
/// The default is deliberately "reap": survival past the wake that started it
/// should be an explicit act, not the consequence of nobody noticing. A
/// company that genuinely needs a durable service will need a way to say so,
/// and that is the tripwire for revisiting this — no company has wanted one
/// yet (§16.1, observe before modelling).
pub(crate) async fn reap_session(container: &str, session_id: &str) -> usize {
    let Ok(output) = crate::runtime::docker_bounded(
        &["exec", container, "ps", "-eo", "pid=,sid="],
        std::time::Duration::from_secs(8),
    )
    .await
    else {
        return 0;
    };
    let leaked = pids_in_session(&String::from_utf8_lossy(&output.stdout), session_id);
    if leaked.is_empty() {
        return 0;
    }
    tracing::info!(
        container,
        session_id,
        count = leaked.len(),
        "reaping processes this turn left behind"
    );
    let mut args = vec!["exec", container, "kill", "-9"];
    args.extend(leaked.iter().map(String::as_str));
    let _ = crate::runtime::docker_bounded(&args, std::time::Duration::from_secs(8)).await;
    leaked.len()
}

/// Reap sessions whose owning daemon disappeared. The marker is the
/// ownership record; a process-name search is incomplete (`omp` replaced
/// `codex-acp`) and can claim unrelated work.
pub(crate) async fn reap_orphan_sessions(container: &str) -> usize {
    let Ok(output) = crate::runtime::docker_bounded(
        &[
            "exec",
            container,
            "find",
            "/tmp",
            "-maxdepth",
            "1",
            "-type",
            "f",
            "-name",
            "restless-agent-*.sid",
            "-print",
        ],
        std::time::Duration::from_secs(8),
    )
    .await
    else {
        return 0;
    };
    if !output.status.success() {
        return 0;
    }

    let mut reaped = 0;
    for marker in String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|path| path.starts_with("/tmp/restless-agent-") && path.ends_with(".sid"))
    {
        if let Some(session_id) = read_session_id(container, marker).await {
            reaped += reap_session(container, &session_id).await;
        }
        let _ = crate::runtime::docker_bounded(
            &["exec", container, "unlink", marker],
            std::time::Duration::from_secs(8),
        )
        .await;
    }
    reaped
}

/// Parse `ps -eo pid=,sid=` without teaching cleanup about process names.
/// Ownership is the kernel's session id, not a substring such as `omp`.
fn pids_in_session(table: &str, session_id: &str) -> Vec<String> {
    table
        .lines()
        .filter_map(|line| {
            let mut columns = line.split_whitespace();
            let pid = columns.next()?;
            let sid = columns.next()?;
            (sid == session_id && pid != "1").then(|| pid.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use agent_client_protocol::schema::v1::{
        ClientCapabilities, SessionConfigOption, SessionConfigOptionCategory,
        SessionConfigSelectOption,
    };

    use super::{
        agent_exec_prefix, claude_agent_settings, exact_model_config_selection,
        exact_named_config_selection, model_config_is_selected, omp_agent_command_args,
        persist_session_locator, pids_in_session, read_session_locator, session_cost_delta,
        session_locator_is_reusable, session_locator_path, validate_session_locator, with_agent,
        AcpProfile, AgentAuth, AgentControls, SessionLocator, DEFAULT_REASONING_EFFORT,
        OMP_AGENT_TOOLS, RESTLESS_OMP_CONFIG,
    };

    #[test]
    fn resumed_usage_is_a_per_wake_delta_and_counter_regression_is_unknown() {
        assert_eq!(session_cost_delta(Some(1.25), Some(1.0)), Some(0.25));
        assert_eq!(session_cost_delta(Some(0.8), Some(1.0)), None);
        assert_eq!(session_cost_delta(None, Some(1.0)), None);
        assert_eq!(session_cost_delta(Some(0.25), None), Some(0.25));
    }

    #[test]
    fn provider_sessions_are_scoped_to_one_actor_and_responsibility() {
        let locator = SessionLocator {
            version: 2,
            harness: crate::runtime::AgentHarness::RestlessManaged,
            harness_build: "omp-18.0.10".into(),
            company: "acme_test".into(),
            actor: "account-reply-writer".into(),
            responsibility: "work:abc".into(),
            cwd: "/company/worktrees/work-abc-r1".into(),
            model: "zai/glm-5.3".into(),
            effort: DEFAULT_REASONING_EFFORT.into(),
            session_id: "session-1".into(),
            cumulative_cost_usd: Some(0.2),
        };
        let profile = AcpProfile::new(crate::runtime::AgentHarness::RestlessManaged).unwrap();
        validate_session_locator(
            &locator,
            profile,
            "acme_test",
            "account-reply-writer",
            "work:abc",
        )
        .unwrap();
        assert!(session_locator_is_reusable(
            &locator,
            profile,
            "/company/worktrees/work-abc-r1",
            "zai/glm-5.3",
            DEFAULT_REASONING_EFFORT,
            true,
        ));
        assert!(
            !session_locator_is_reusable(
                &locator,
                profile,
                "/company/worktrees/work-abc-r2",
                "zai/glm-5.3",
                DEFAULT_REASONING_EFFORT,
                true,
            ),
            "a new revision worktree reconstructs rather than crashing or loading stale context"
        );
        assert!(validate_session_locator(
            &locator,
            profile,
            "acme_test",
            "other-writer",
            "work:abc",
        )
        .is_err());
        assert!(validate_session_locator(
            &locator,
            profile,
            "acme_test",
            "account-reply-writer",
            "work:def",
        )
        .is_err());
        assert_ne!(
            session_locator_path(
                crate::runtime::AgentHarness::RestlessManaged,
                "acme_test",
                "account-reply-writer",
                "work:abc"
            )
            .unwrap(),
            session_locator_path(
                crate::runtime::AgentHarness::RestlessManaged,
                "acme_test",
                "account-reply-writer",
                "work:def"
            )
            .unwrap()
        );
    }

    /// Opt-in product probe. It uses one dedicated `_test` company and the
    /// already-running host gateway/coordination service; no provider secret
    /// enters the test process or Company Runtime.
    #[tokio::test]
    #[ignore = "requires RESTLESS_ACP_SESSION_TEST_COMPANY and a live model gateway"]
    async fn live_process_cold_session_hot_continuity_and_reconstruction() {
        let company = std::env::var("RESTLESS_ACP_SESSION_TEST_COMPANY")
            .expect("set RESTLESS_ACP_SESSION_TEST_COMPANY");
        assert!(company.ends_with("_test"));
        let model = std::env::var("RESTLESS_ACP_SESSION_TEST_MODEL")
            .unwrap_or_else(|_| "zai/glm-5.3".to_string());
        let provider = model
            .split_once('/')
            .map(|(provider, _)| provider)
            .expect("provider-qualified model")
            .to_string();
        let root = std::env::var("RESTLESS_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(std::env::var("HOME").expect("HOME")).join(".restless")
            });
        let capabilities = crate::capability::CapabilityIssuer::open(&root).unwrap();
        let container = crate::runtime::container_name(&company);
        let actor = "exec";
        let run_id = uuid::Uuid::new_v4().simple().to_string();
        let responsibility = format!("evaluation:s17-session-continuity:{run_id}");
        let workdir = "/company";
        let make_auth = || {
            let launch_id = uuid::Uuid::new_v4().simple().to_string();
            AgentAuth {
                model: model.clone(),
                effort: DEFAULT_REASONING_EFFORT.into(),
                company: company.clone(),
                session_id: launch_id.clone(),
                coordination_token_env: "RESTLESS_SESSION_CAPABILITY".into(),
                coordination_token: capabilities
                    .issue_actor_session(&company, actor, &launch_id)
                    .unwrap(),
                gateway_token_env: "RESTLESS_MODEL_CAPABILITY".into(),
                gateway_token: capabilities
                    .issue_model_session(
                        &company,
                        actor,
                        &launch_id,
                        &provider,
                        &model,
                        crate::model_gateway::ModelBilling::Subscription.as_str(),
                        &responsibility,
                        None,
                        None,
                    )
                    .unwrap(),
                gateway_url: "http://host.docker.internal:7790".into(),
                billing: crate::model_gateway::ModelBilling::Subscription,
            }
        };
        let marker = format!("S17-{}", uuid::Uuid::new_v4().simple());
        let first_auth = make_auth();
        let first = with_agent(
            &container,
            crate::runtime::AgentHarness::RestlessManaged,
            &first_auth,
            workdir,
            actor,
            &responsibility,
            AgentControls::company_actor("You are a bounded continuity probe.".into()).unwrap(),
            None,
            {
                let marker = marker.clone();
                move |session| {
                    Box::pin(async move {
                        session
                            .prompt(&format!(
                                "Privately retain this exact continuity marker for my next turn: {marker}. Reply only ACK."
                            ))
                            .await?;
                        let transcript = session.take_transcript();
                        anyhow::Ok((
                            session.launch_id.clone(),
                            session.session_id.to_string(),
                            session.resumed,
                            session.reconstructed,
                            session.tool_contract_digest.clone(),
                            transcript.usage,
                            transcript.last_message_text,
                        ))
                    })
                }
            },
        )
        .await
        .unwrap();
        assert!(!first.2 && !first.3);

        let second_auth = make_auth();
        assert_ne!(first_auth.session_id, second_auth.session_id);
        assert_ne!(
            first_auth.coordination_token,
            second_auth.coordination_token
        );
        let second = with_agent(
            &container,
            crate::runtime::AgentHarness::RestlessManaged,
            &second_auth,
            workdir,
            actor,
            &responsibility,
            AgentControls::company_actor("You are a bounded continuity probe.".into()).unwrap(),
            None,
            move |session| {
                Box::pin(async move {
                    session
                        .prompt("Return only the private continuity marker from my previous turn.")
                        .await?;
                    let transcript = session.take_transcript();
                    anyhow::Ok((
                        session.launch_id.clone(),
                        session.session_id.to_string(),
                        session.resumed,
                        session.reconstructed,
                        session.tool_contract_digest.clone(),
                        transcript.usage,
                        transcript.last_message_text,
                    ))
                })
            },
        )
        .await
        .unwrap();
        assert_ne!(first.0, second.0, "each wake needs a new OS launch");
        assert_eq!(first.1, second.1, "provider session should stay hot");
        assert!(second.2 && !second.3);
        assert_eq!(first.4, second.4);
        assert!(second.6.contains(&marker), "reply was {:?}", second.6);
        assert!(second
            .5
            .and_then(|usage| usage.cost_usd)
            .is_none_or(|cost| cost >= 0.0));

        let path = session_locator_path(
            crate::runtime::AgentHarness::RestlessManaged,
            &company,
            actor,
            &responsibility,
        )
        .unwrap();
        let mut broken = read_session_locator(&container, &path)
            .await
            .unwrap()
            .unwrap();
        broken.session_id = format!("missing-{}", uuid::Uuid::new_v4().simple());
        persist_session_locator(&container, &path, &broken)
            .await
            .unwrap();
        let third_auth = make_auth();
        let third = with_agent(
            &container,
            crate::runtime::AgentHarness::RestlessManaged,
            &third_auth,
            workdir,
            actor,
            &responsibility,
            AgentControls::company_actor("You are a bounded continuity probe.".into()).unwrap(),
            None,
            {
                let marker = marker.clone();
                move |session| {
                    Box::pin(async move {
                        session
                            .prompt(&format!(
                                "Durable factual context reconstruction: the expected marker is {marker}. Return only that marker."
                            ))
                            .await?;
                        let transcript = session.take_transcript();
                        anyhow::Ok((
                            session.session_id.to_string(),
                            session.resumed,
                            session.reconstructed,
                            session.reconstruction_reason.clone(),
                            transcript.last_message_text,
                        ))
                    })
                }
            },
        )
        .await
        .unwrap();
        assert!(!third.1 && third.2);
        assert!(third
            .3
            .as_deref()
            .is_some_and(|reason| reason.contains("session/load failed")));
        assert_ne!(third.0, broken.session_id);
        assert!(third.4.contains(&marker), "reply was {:?}", third.4);

        let readiness_responsibility = format!("evaluation:s17-readiness-repair:{run_id}");
        let mut invalid_auth = make_auth();
        invalid_auth.coordination_token = "invalid-session-capability".into();
        let productive_prompt_reached =
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called = productive_prompt_reached.clone();
        let invalid = with_agent(
            &container,
            crate::runtime::AgentHarness::RestlessManaged,
            &invalid_auth,
            workdir,
            actor,
            &readiness_responsibility,
            AgentControls::company_actor("You are a bounded readiness probe.".into()).unwrap(),
            None,
            move |_session| {
                let called = called.clone();
                Box::pin(async move {
                    called.store(true, std::sync::atomic::Ordering::Release);
                    anyhow::Ok(())
                })
            },
        )
        .await
        .unwrap_err();
        assert!(format!("{invalid:#}").contains("coordination readiness failed before prompt"));
        assert!(!productive_prompt_reached.load(std::sync::atomic::Ordering::Acquire));

        let repaired_auth = make_auth();
        let repaired = with_agent(
            &container,
            crate::runtime::AgentHarness::RestlessManaged,
            &repaired_auth,
            workdir,
            actor,
            &readiness_responsibility,
            AgentControls::company_actor("You are a bounded readiness probe.".into()).unwrap(),
            None,
            move |session| {
                Box::pin(async move {
                    session.prompt("Reply only READY.").await?;
                    let transcript = session.take_transcript();
                    anyhow::Ok((session.resumed, transcript.last_message_text))
                })
            },
        )
        .await
        .unwrap();
        assert!(
            repaired.0,
            "the repaired launch should reuse the unspent scoped session"
        );
        assert!(repaired.1.contains("READY"), "reply was {:?}", repaired.1);
    }

    /// OrgIntel owns Staff identity and handoff evidence. OMP's similarly
    /// named task runtime is deliberately absent so an actor cannot bypass
    /// the Work graph and then claim that private subagents were Staff.
    #[test]
    fn omp_cannot_create_invisible_staff() {
        let tools: Vec<_> = OMP_AGENT_TOOLS.split(',').collect();
        assert_eq!(tools, ["read", "bash", "edit", "write", "grep"]);
        // Vision-capable OMP models ingest images through `read`. OMP hides
        // its delegated `inspect_image` tool in auto mode for those models;
        // forcing it would permit a second model route and weaken exact-model
        // evidence.
        assert!(tools.contains(&"read"));
        assert!(!tools.contains(&"inspect_image"));
        assert!(!tools.contains(&"task"));
    }

    #[test]
    fn every_actor_launch_requires_restless_system_policy() {
        assert!(AgentControls::company_actor("   ".into()).is_err());
        let controls = AgentControls::company_actor("You are a Restless actor.".into()).unwrap();
        assert_eq!(controls.system_prompt, "You are a Restless actor.");
        assert!(controls.mcp_servers.is_empty());
        assert!(!controls.team_coordination_wake);

        let coordination = controls.for_team_coordination();
        assert!(coordination.team_coordination_wake);
    }

    #[test]
    fn agent_shell_and_acp_session_share_the_requested_workspace() {
        let review_copy = "/company/reviews/attempt-0123456789abcdef";
        let prefix = agent_exec_prefix(review_copy);
        assert_eq!(
            prefix.windows(2).find(|window| window[0] == "-w"),
            Some(["-w".to_string(), review_copy.to_string()].as_slice())
        );
        assert!(!prefix.iter().any(|argument| argument == "/company"));
    }

    #[test]
    fn omp_global_launch_flags_precede_the_acp_subcommand() {
        let args = omp_agent_command_args(
            "zai/glm-5.3-flash",
            "medium",
            "/tmp/restless-agent.system.md",
        );
        assert_eq!(args.first().map(String::as_str), Some("omp"));
        assert_eq!(args.last().map(String::as_str), Some("acp"));
        let acp = args.iter().position(|arg| arg == "acp").unwrap();
        for flag in [
            "--model",
            "--thinking",
            "--system-prompt",
            "--config",
            "--no-extensions",
            "--no-rules",
            "--tools",
        ] {
            assert!(
                args.iter().position(|arg| arg == flag).unwrap() < acp,
                "{flag} must be parsed as an OMP global flag before `acp`"
            );
        }
    }

    #[test]
    fn claude_profile_is_isolated_exact_and_policy_bound() {
        let profile = AcpProfile::new(crate::runtime::AgentHarness::ClaudeAgent).unwrap();
        assert_eq!(profile.config_dir(), "/company/home/.restless/claude-agent");
        assert_eq!(profile.build(), "claude-agent-acp-0.73.0");
        assert_eq!(profile.native_agent_build(), Some("claude-code-2.1.257"));
        assert_eq!(profile.transport(), "acp-stdio-v1");
        assert_eq!(
            profile
                .session_model("anthropic/claude-sonnet-4-6")
                .unwrap(),
            "claude-sonnet-4-6"
        );
        assert!(profile.session_model("litellm/gpt-5.6-sol").is_err());
        assert_eq!(
            profile.command_args("ignored", "high", "ignored"),
            ["claude-agent-acp"]
        );
        assert_eq!(
            profile.native_tools(),
            ["Read", "Write", "Edit", "Bash", "Glob", "Grep"]
        );

        let meta = serde_json::Value::Object(
            profile
                .session_meta("Restless policy", "claude-sonnet-4-6")
                .unwrap(),
        );
        assert_eq!(meta["systemPrompt"], "Restless policy");
        assert_eq!(
            meta["claudeCode"]["options"]["settingSources"],
            serde_json::json!([])
        );
        assert_eq!(
            meta["claudeCode"]["options"]["settings"]["availableModels"],
            serde_json::json!(["claude-sonnet-4-6"])
        );
        assert_eq!(
            meta["claudeCode"]["options"]["tools"],
            serde_json::json!(["Read", "Write", "Edit", "Bash", "Glob", "Grep"])
        );
        let settings: serde_json::Value =
            serde_json::from_str(&claude_agent_settings("claude-sonnet-4-6").unwrap()).unwrap();
        assert_eq!(
            settings["availableModels"],
            serde_json::json!(["claude-sonnet-4-6"])
        );
        assert_eq!(settings["enabledPlugins"], serde_json::json!({}));
        assert_eq!(settings["hooks"], serde_json::json!({}));
        assert_eq!(settings["permissions"]["defaultMode"], "default");
        assert_eq!(settings["permissions"]["allow"], serde_json::json!([]));
        let disallowed = meta["claudeCode"]["options"]["disallowedTools"]
            .as_array()
            .unwrap();
        for name in [
            "Agent",
            "Task",
            "Skill",
            "WebSearch",
            "WebFetch",
            "AskUserQuestion",
        ] {
            assert!(disallowed.contains(&serde_json::Value::String(name.into())));
        }
    }

    #[test]
    fn claude_profile_rejects_cross_harness_session_reuse() {
        let locator = SessionLocator {
            version: 2,
            harness: crate::runtime::AgentHarness::RestlessManaged,
            harness_build: "omp-18.0.10".into(),
            company: "acme_test".into(),
            actor: "lead".into(),
            responsibility: "work:abc".into(),
            cwd: "/company/worktrees/work-abc-r1".into(),
            model: "anthropic/claude-sonnet-4-6".into(),
            effort: "high".into(),
            session_id: "session-1".into(),
            cumulative_cost_usd: None,
        };
        let claude = AcpProfile::new(crate::runtime::AgentHarness::ClaudeAgent).unwrap();
        assert!(!session_locator_is_reusable(
            &locator,
            claude,
            "/company/worktrees/work-abc-r1",
            "anthropic/claude-sonnet-4-6",
            "high",
            true,
        ));
    }

    #[test]
    fn claude_required_session_options_are_exact_selects() {
        let options = vec![
            SessionConfigOption::select(
                "effort",
                "Effort",
                "medium",
                vec![
                    SessionConfigSelectOption::new("medium", "Medium"),
                    SessionConfigSelectOption::new("high", "High"),
                ],
            ),
            SessionConfigOption::select(
                "mode",
                "Permission mode",
                "plan",
                vec![
                    SessionConfigSelectOption::new("default", "Default"),
                    SessionConfigSelectOption::new("plan", "Plan"),
                ],
            ),
        ];
        assert_eq!(
            exact_named_config_selection(&options, "effort", "high").unwrap(),
            ("effort".into(), "high".into())
        );
        assert_eq!(
            exact_named_config_selection(&options, "mode", "default").unwrap(),
            ("mode".into(), "default".into())
        );
        assert!(exact_named_config_selection(&options, "effort", "ultra").is_err());
    }

    #[test]
    fn exact_acp_model_selection_is_advertised_and_verified() {
        let options = vec![SessionConfigOption::select(
            "model",
            "Model",
            "zai/glm-5.3",
            vec![
                SessionConfigSelectOption::new("zai/glm-5.3", "GLM 5.3"),
                SessionConfigSelectOption::new("zai/glm-5.3-flash", "GLM 5.3 Flash"),
            ],
        )
        .category(SessionConfigOptionCategory::Model)];
        let selected = exact_model_config_selection(&options, "zai/glm-5.3-flash").unwrap();
        assert_eq!(selected, ("model".into(), "zai/glm-5.3-flash".into()));
        assert!(!model_config_is_selected(
            &options,
            &selected.0,
            &selected.1
        ));

        let confirmed = vec![SessionConfigOption::select(
            "model",
            "Model",
            "zai/glm-5.3-flash",
            vec![SessionConfigSelectOption::new(
                "zai/glm-5.3-flash",
                "GLM 5.3 Flash",
            )],
        )
        .category(SessionConfigOptionCategory::Model)];
        assert!(model_config_is_selected(
            &confirmed,
            &selected.0,
            &selected.1
        ));
        assert!(exact_model_config_selection(&options, "zai/glm-does-not-exist").is_err());
    }

    #[test]
    fn omp_profile_rejects_ambient_agent_capabilities() {
        assert!(RESTLESS_OMP_CONFIG.contains("enableProjectConfig: false"));
        assert!(RESTLESS_OMP_CONFIG.contains("enableAgentsUser: false"));
        assert!(RESTLESS_OMP_CONFIG.contains("enableAgentsProject: true"));
        assert!(RESTLESS_OMP_CONFIG.contains("/opt/restless/skills"));
        assert!(RESTLESS_OMP_CONFIG.contains("/company/skills"));
        assert!(RESTLESS_OMP_CONFIG.contains("retry:\n  enabled: false"));
        assert!(RESTLESS_OMP_CONFIG.contains("modelFallback: false"));
        for provider in ["claude", "codex", "github", "opencode"] {
            assert!(RESTLESS_OMP_CONFIG.contains(&format!("  - {provider}\n")));
        }
    }

    /// Guards the silent failure that cost a probe cycle: if the client ever
    /// advertises filesystem capabilities, the agent stops writing to the
    /// company volume and starts asking us to write for it — and the symptom
    /// is a turn that looks successful and produces no files.
    #[test]
    fn declines_client_side_filesystem() {
        let capabilities = ClientCapabilities::default();
        assert!(!capabilities.fs.read_text_file);
        assert!(!capabilities.fs.write_text_file);
    }

    /// S04-T6 regression: a staff process started during an Exec turn is new
    /// in a PID snapshot but belongs to another Linux session. Cleanup must
    /// select only the caller's session.
    #[test]
    fn cleanup_never_claims_a_concurrent_actor_session() {
        let table = "  101  101\n  102  101\n  201  201\n  202  201\n";
        assert_eq!(pids_in_session(table, "101"), vec!["101", "102"]);
        assert_eq!(pids_in_session(table, "201"), vec!["201", "202"]);
    }
}
