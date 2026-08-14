//! ACP client (sprint 01 T3 canon, grown into the daemon): spawn the agent
//! binary as an ordinary process inside the company's persistent container
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
    self as acp, Agent, ByteStreams, Client, ConnectionTo,
    schema::{
        ProtocolVersion,
        v1::{
            CancelNotification, ContentBlock, InitializeRequest,
            NewSessionRequest, PermissionOptionKind, PromptRequest, RequestPermissionOutcome,
            RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
            SessionId, SessionNotification, SessionUpdate, TextContent, ToolCallStatus,
        },
    },
};
use anyhow::{Context, Result};
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

/// Everything a wake needs to start an agent against a provider.
///
/// The agent binary is `omp` (Oh My Pi), which speaks ACP natively and
/// reports its own token and dollar usage per turn. It resolves credentials
/// from its process environment, so `provider_key_env` names the variable and
/// `provider_key` carries the value into `docker exec -e` — the value lives
/// in the agent process's environment for the life of the turn, not in the
/// image, the container's persistent env, or any file on the volume.
pub struct AgentAuth {
    /// Provider-qualified model, e.g. `moonshot/k3-256k`.
    pub model: String,
    pub provider_key_env: String,
    pub provider_key: String,
    /// Optional `<PROVIDER>_BASE_URL` override, forwarded when the daemon has
    /// one. A provider's default host is not always the one a given plan is
    /// served from — a Kimi For Coding key authenticates against
    /// `api.kimi.com/coding/v1` and 401s against `api.moonshot.ai`, which is
    /// indistinguishable from a dead key unless the override exists.
    pub provider_base_url: Option<(String, String)>,
}

/// What one turn consumed, as the agent reported it (ACP `UsageUpdate`).
/// This is the fuse's input and the health gate's load-bearing signal: the
/// agent knows its own token count and dollar cost, so the daemon no longer
/// has to sit in the HTTP path parsing SSE tails to find out (T2 → the
/// spend spool keeps its ledger, loses its proxy).
#[derive(Debug, Clone, Copy, Default)]
pub struct TurnUsage {
    /// Tokens consumed this turn. Zero is the universal failure tell.
    pub used: u64,
    /// Context window size the agent is working against.
    pub size: u64,
    /// Dollar cost, when the provider priced the turn.
    pub cost_usd: Option<f64>,
}

/// One observed turn: the agent's visible text plus the tool calls it made.
/// This is observability, not a governed record (§4.4).
#[derive(Debug)]
pub struct TurnTranscript {
    pub text: String,
    pub tool_calls: Vec<String>,
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
            tool_calls: Vec::new(),
            usage: None,
            last_activity: std::time::Instant::now(),
            tools_in_flight: 0,
        }
    }
}

impl TurnTranscript {
    fn note(&mut self, update: SessionUpdate) {
        // Every update is liveness, whatever it carries.
        self.last_activity = std::time::Instant::now();
        match update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let ContentBlock::Text(text) = chunk.content {
                    self.text.push_str(&text.text);
                }
            }
            SessionUpdate::ToolCall(call) => {
                self.tool_calls.push(format!("{:?}: {}", call.kind, call.title));
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

/// A live agent process and connection, mid-turn.
pub struct AgentSession {
    cx: ConnectionTo<Agent>,
    pub session_id: SessionId,
    transcript: Arc<Mutex<TurnTranscript>>,
}

/// Silence with nothing running: the agent is wedged. Deliberately loose —
/// waiting is cheap, and a false kill destroys a turn's work.
const IDLE_SILENT: std::time::Duration = std::time::Duration::from_secs(120);
/// Silence while a tool call is in flight: a test suite, an install, a build.
/// Bounded, but generously.
const IDLE_TOOL_RUNNING: std::time::Duration = std::time::Duration::from_secs(15 * 60);
/// How often the watchdog looks. Cheap: it reads two in-memory values.
const WATCHDOG_TICK: std::time::Duration = std::time::Duration::from_secs(5);

/// Why a turn stopped early. Both are deterministic observations, never
/// inferences from what the model wrote (LLM_CURE.md frame 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnHalt {
    /// No output of any kind for longer than the idle allowance.
    Wedged,
    /// The company spent its ceiling mid-turn.
    OverBudget,
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
    pub async fn prompt_live(
        &self,
        text: &str,
        budget: impl Fn(&TurnUsage) -> bool + Send,
    ) -> Result<Option<TurnHalt>> {
        let prompt = self.prompt(text);
        tokio::pin!(prompt);
        loop {
            tokio::select! {
                finished = &mut prompt => {
                    finished?;
                    return Ok(None);
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
                        return Ok(Some(TurnHalt::OverBudget));
                    }
                    if idle > allowance {
                        tracing::warn!(
                            idle_secs = idle.as_secs(),
                            "agent produced nothing for longer than the idle allowance"
                        );
                        let _ = self.cancel().await;
                        return Ok(Some(TurnHalt::Wedged));
                    }
                }
            }
        }
    }

    /// Send one prompt and wait for the turn to complete.
    pub async fn prompt(&self, text: &str) -> Result<()> {
        self.cx
            .send_request(PromptRequest::new(
                self.session_id.clone(),
                vec![ContentBlock::Text(TextContent::new(text.to_string()))],
            ))
            .block_task()
            .await
            .context("acp session/prompt")?;
        Ok(())
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

/// Spawn codex-acp inside the company container, authenticate against the
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
    drive: F,
) -> Result<T>
where
    F: for<'a> FnOnce(
        &'a AgentSession,
    )
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send + 'a>>,
{
    // Snapshot before the agent starts: anything new and still alive when the
    // turn ends was started by this turn and is ours to clean up.
    let before = pids(container).await;
    // docker exec takes its -e flags BEFORE the container name; anything after
    // it belongs to the command. Build the vector explicitly rather than
    // chaining .args() and hoping the order is right — appending the base-URL
    // override after the container silently handed it to omp as an argument,
    // which surfaced as "No model selected".
    let mut args: Vec<String> = [
        "exec", "-i", "-u", "company", "-w", "/company",
        "-e", "OMP_HOME=/company/home/.omp",
        "-e", "NO_BROWSER=1",
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect();
    args.push("-e".to_string());
    args.push(format!("RESTLESS_ACTOR={actor}"));
    args.push("-e".to_string());
    args.push(format!("{}={}", auth.provider_key_env, auth.provider_key));
    if let Some((name, value)) = &auth.provider_base_url {
        args.push("-e".to_string());
        args.push(format!("{name}={value}"));
    }
    args.extend(
        [container, "omp", "acp", "--model", auth.model.as_str()]
            .iter()
            .map(|arg| (*arg).to_string()),
    );
    let mut child = tokio::process::Command::new("docker")
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("spawn codex-acp in container")?;
    let stdin = child.stdin.take().expect("piped stdin");
    let stdout = child.stdout.take().expect("piped stdout");
    let transport = ByteStreams::new(stdin.compat_write(), stdout.compat());

    let transcript = Arc::new(Mutex::new(TurnTranscript::default()));
    let sink = Arc::clone(&transcript);
    let workdir = workdir.to_string();
    // connect_with speaks acp::Error; the real anyhow chain is parked here
    // and restored after the connection closes.
    let failure = Arc::new(Mutex::new(None::<anyhow::Error>));
    let failure_slot = Arc::clone(&failure);

    let result = Client
        .builder()
        .on_receive_notification(
            move |notification: SessionNotification, _cx| {
                let sink = Arc::clone(&sink);
                async move {
                    if let Ok(mut transcript) = sink.lock() {
                        transcript.note(notification.update);
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
                    .send_request(NewSessionRequest::new(workdir).mcp_servers(vec![]))
                    .block_task()
                    .await
                    .context("acp session/new")?;

                let agent = AgentSession { cx, session_id: session.session_id, transcript };
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

    let _ = child.kill().await;
    reap(container, &before).await;
    if let Ok(mut slot) = failure.lock() {
        if let Some(error) = slot.take() {
            return Err(error);
        }
    }
    Ok(result?)
}

/// Every process id currently alive in the container.
async fn pids(container: &str) -> std::collections::HashSet<String> {
    let Ok(output) = tokio::process::Command::new("docker")
        .args(["exec", container, "ps", "-eo", "pid="])
        .output()
        .await
    else {
        return std::collections::HashSet::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Kill whatever the turn started and left behind.
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
async fn reap(container: &str, before: &std::collections::HashSet<String>) {
    let after = pids(container).await;
    let leaked: Vec<&str> = after
        .difference(before)
        .map(String::as_str)
        // PID 1 is the container's init and never ours to kill.
        .filter(|pid| *pid != "1")
        .collect();
    if leaked.is_empty() {
        return;
    }
    tracing::info!(container, count = leaked.len(), "reaping processes the turn left behind");
    let mut args = vec!["exec", container, "kill", "-9"];
    args.extend(leaked.iter().copied());
    let _ = tokio::process::Command::new("docker").args(&args).output().await;
}

#[cfg(test)]
mod tests {
    use agent_client_protocol::schema::v1::ClientCapabilities;

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
}
