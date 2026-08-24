//! ACP client (sprint 01 T3 canon, grown into the daemon): start the agent
//! binary as an ordinary supervised process inside the company's persistent container
//! (`docker exec`, §5), speak JSON-RPC over stdio, stream turn updates,
//! cancel on demand. The session is disposable; the company is not.
//!
//! The agent is `omp`, which speaks ACP natively and — unlike the codex-acp
//! binary it replaces — reports per-turn token and dollar usage on the
//! session stream. That single fact moved the spend fuse out of the HTTP path
//! (T2's proxy) and up to here, where the daemon already knows which company
//! the session belongs to.

use std::process::Stdio;
use std::sync::{Arc, Mutex};

use agent_client_protocol::{
    self as acp,
    schema::{
        v1::{
            CancelNotification, ContentBlock, InitializeRequest, McpServer, NewSessionRequest,
            PermissionOptionKind, PromptRequest, PromptResponse, RequestPermissionOutcome,
            RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
            SessionId, SessionNotification, SessionUpdate, TextContent, ToolCallStatus,
        },
        ProtocolVersion,
    },
    Agent, ByteStreams, Client, ConnectionTo,
};
use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt as _;
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

/// Everything a wake needs to start an agent against a provider.
///
/// The agent binary is `omp` (Oh My Pi), which speaks ACP natively and
/// reports its own token and dollar usage per turn. The process receives a
/// narrow OMP auth-gateway bearer, never the provider credential or Infisical
/// machine identity.
pub struct AgentAuth {
    /// Provider-qualified model, e.g. `moonshot/k3-256k`.
    pub model: String,
    pub provider: String,
    pub gateway_token_env: String,
    pub gateway_token: String,
    pub gateway_url: String,
    /// Whether the provider reports a charged API cost or only a catalogue
    /// estimate for subscription access. The Runtime still receives no
    /// provider credential either way.
    pub billing: crate::model_gateway::ModelBilling,
}

pub(crate) const AGENT_CONFIG_DIR: &str = "/company/home/.restless/omp-agent";
const OMP_RUNTIME_CONFIG: &str = "/company/home/.restless/omp-agent/restless-runtime.yml";

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
    #[allow(dead_code)]
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

async fn write_private_container_file(container: &str, path: &str, contents: &str) -> Result<()> {
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

/// Install the provider's credential-free OMP route in a Restless-owned agent
/// directory. This never touches the company's general-purpose ~/.omp config
/// and never writes a bearer or provider key to the volume.
pub(crate) async fn prepare_agent_runtime(container: &str, auth: &AgentAuth) -> Result<()> {
    let config = crate::model_gateway::models_config(
        &auth.provider,
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
    /// Cumulative session cost, when the provider priced the session.
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
    /// Last usage report of the turn; `None` means the agent never sent one,
    /// which `health::classify_turn` treats exactly like zero.
    pub usage: Option<TurnUsage>,
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
            last_activity: std::time::Instant::now(),
            tools_in_flight: 0,
        }
    }
}

impl TurnTranscript {
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
                self.usage = Some(TurnUsage {
                    used: usage.used,
                    size: usage.size,
                    cost_usd: usage.cost.as_ref().map(|cost| cost.amount),
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
        _ => None,
    }
}

/// A live agent process and connection, mid-turn.
pub struct AgentSession {
    cx: ConnectionTo<Agent>,
    pub session_id: SessionId,
    transcript: Arc<Mutex<TurnTranscript>>,
    observer: Option<SessionObserver>,
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
    ) -> TurnEnd {
        let prompt = self.prompt(text);
        tokio::pin!(prompt);
        loop {
            tokio::select! {
                finished = &mut prompt => {
                    return match finished {
                        Ok(response) => {
                            let output_tokens = response.usage.map(|usage| usage.output_tokens);
                            if let Some(tokens) = output_tokens {
                                if let Some(observer) = &self.observer {
                                    observer(LiveSessionEvent::GeneratedOutputTokens(tokens));
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
            .map(|mut guard| std::mem::take(&mut *guard))
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
pub async fn with_agent<F, T>(
    container: &str,
    auth: &AgentAuth,
    workdir: &str,
    actor: &str,
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
    prepare_agent_runtime(container, auth).await?;
    // Every docker-exec process is a Linux session leader. Record this turn's
    // session id inside the container so cleanup can reap only its process
    // tree. A before/after PID diff is not ownership: a staff turn may start
    // while the Exec is running, and the Exec must never kill it on exit.
    let launch_id = uuid::Uuid::new_v4().simple().to_string();
    let session_marker = format!("/tmp/restless-agent-{launch_id}.sid");
    let system_prompt_path = format!("/tmp/restless-agent-{launch_id}.system.md");
    write_private_container_file(container, &system_prompt_path, &controls.system_prompt).await?;
    // docker exec takes its -e flags BEFORE the container name; anything after
    // it belongs to the command. Build the vector explicitly rather than
    // chaining .args() and hoping the order is right — appending the base-URL
    // override after the container silently handed it to omp as an argument,
    // which surfaced as "No model selected".
    let mut args = agent_exec_prefix(workdir);
    args.push("-e".to_string());
    args.push(format!("RESTLESS_ACTOR={actor}"));
    if controls.team_coordination_wake {
        args.push("-e".to_string());
        args.push("RESTLESS_COORDINATION_WAKE=1".to_string());
    }
    args.push("-e".to_string());
    // `docker exec -e NAME` copies NAME from the docker client's environment.
    // Passing NAME=VALUE in argv would expose even the narrow gateway bearer
    // to host process listings during session bootstrap.
    args.push(auth.gateway_token_env.clone());
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
            "omp",
            "acp",
            "--model",
            auth.model.as_str(),
            "--system-prompt",
            system_prompt_path.as_str(),
            "--config",
            OMP_RUNTIME_CONFIG,
            "--no-extensions",
            "--no-rules",
            "--tools",
            OMP_AGENT_TOOLS,
        ]
        .iter()
        .map(|arg| (*arg).to_string()),
    );
    let spawned = tokio::process::Command::new("docker")
        .env(&auth.gateway_token_env, &auth.gateway_token)
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
    let event_observer = observer.clone();
    let workdir = workdir.to_string();
    let mcp_servers = controls.mcp_servers;
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
                async move {
                    if let (Some(observer), Some(event)) =
                        (event_observer.as_ref(), live_event(&notification.update))
                    {
                        observer(event);
                    }
                    if let Ok(mut transcript) = sink.lock() {
                        transcript.note(&notification.update);
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
                let agent_name = initialized
                    .agent_info
                    .as_ref()
                    .map(|info| format!("{} v{}", info.name, info.version))
                    .unwrap_or_else(|| "unknown".to_string());
                tracing::info!(agent = %agent_name, "acp agent initialized");

                let session = cx
                    .send_request(NewSessionRequest::new(workdir).mcp_servers(mcp_servers))
                    .block_task()
                    .await
                    .context("acp session/new")?;

                let agent = AgentSession {
                    cx,
                    session_id: session.session_id,
                    transcript,
                    observer,
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
    if let Some(session_id) = owned_session {
        let _ = reap_session(container, &session_id).await;
    } else {
        tracing::warn!(container, marker = %session_marker, "agent session ownership marker missing; declining broad process cleanup");
    }
    let _ = tokio::process::Command::new("docker")
        .args(["exec", container, "unlink", &session_marker])
        .output()
        .await;
    let _ = tokio::process::Command::new("docker")
        .args(["exec", container, "unlink", &system_prompt_path])
        .output()
        .await;
    if let Ok(mut slot) = failure.lock() {
        if let Some(error) = slot.take() {
            return Err(error);
        }
    }
    Ok(result?)
}

/// Docker's process cwd and ACP's `session/new` cwd must agree. Otherwise an
/// agent can be told that it is reviewing a detached copy while its shell
/// tools still start in `/company`, which silently reopens the source
/// worktree mutation path Sprint 14 is closing.
fn agent_exec_prefix(workdir: &str) -> Vec<String> {
    [
        "exec".to_string(),
        "-i".to_string(),
        "-u".to_string(),
        "company".to_string(),
        "-w".to_string(),
        workdir.to_string(),
        "-e".to_string(),
        format!("PI_CODING_AGENT_DIR={AGENT_CONFIG_DIR}"),
        "-e".to_string(),
        "NO_BROWSER=1".to_string(),
    ]
    .to_vec()
}

/// Read and validate the Linux session id written by this turn's wrapper.
async fn read_session_id(container: &str, marker: &str) -> Option<String> {
    let Ok(output) = tokio::process::Command::new("docker")
        .args(["exec", container, "cat", marker])
        .output()
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
async fn reap_session(container: &str, session_id: &str) -> usize {
    let Ok(output) = tokio::process::Command::new("docker")
        .args(["exec", container, "ps", "-eo", "pid=,sid="])
        .output()
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
    let _ = tokio::process::Command::new("docker")
        .args(&args)
        .output()
        .await;
    leaked.len()
}

/// Reap sessions whose owning daemon disappeared. The marker is the
/// ownership record; a process-name search is incomplete (`omp` replaced
/// `codex-acp`) and can claim unrelated work.
pub(crate) async fn reap_orphan_sessions(container: &str) -> usize {
    let Ok(output) = tokio::process::Command::new("docker")
        .args([
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
        ])
        .output()
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
        let _ = tokio::process::Command::new("docker")
            .args(["exec", container, "unlink", marker])
            .output()
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
    use agent_client_protocol::schema::v1::ClientCapabilities;

    use super::{
        agent_exec_prefix, pids_in_session, AgentControls, OMP_AGENT_TOOLS, RESTLESS_OMP_CONFIG,
    };

    /// OrgIntel owns Staff identity and handoff evidence. OMP's similarly
    /// named task runtime is deliberately absent so an actor cannot bypass
    /// the Work graph and then claim that private subagents were Staff.
    #[test]
    fn omp_cannot_create_invisible_staff() {
        let tools: Vec<_> = OMP_AGENT_TOOLS.split(',').collect();
        assert_eq!(tools, ["read", "bash", "edit", "write", "grep"]);
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
    fn omp_profile_rejects_ambient_agent_capabilities() {
        assert!(RESTLESS_OMP_CONFIG.contains("enableProjectConfig: false"));
        assert!(RESTLESS_OMP_CONFIG.contains("enableAgentsUser: false"));
        assert!(RESTLESS_OMP_CONFIG.contains("enableAgentsProject: true"));
        assert!(RESTLESS_OMP_CONFIG.contains("/opt/restless/skills"));
        assert!(RESTLESS_OMP_CONFIG.contains("/company/skills"));
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
