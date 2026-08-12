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
            SessionId, SessionNotification, SessionUpdate, TextContent,
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
    /// Provider-qualified model, e.g. `zai/glm-5.2`.
    pub model: String,
    pub provider_key_env: String,
    pub provider_key: String,
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
#[derive(Debug, Default)]
pub struct TurnTranscript {
    pub text: String,
    pub tool_calls: Vec<String>,
    /// Last usage report of the turn; `None` means the agent never sent one,
    /// which `health::classify_turn` treats exactly like zero.
    pub usage: Option<TurnUsage>,
}

impl TurnTranscript {
    fn note(&mut self, update: SessionUpdate) {
        match update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let ContentBlock::Text(text) = chunk.content {
                    self.text.push_str(&text.text);
                }
            }
            SessionUpdate::ToolCall(call) => {
                self.tool_calls.push(format!("{:?}: {}", call.kind, call.title));
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

impl AgentSession {
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
    let mut child = tokio::process::Command::new("docker")
        .args([
            "exec", "-i", "-u", "company", "-w", "/company",
            "-e", "OMP_HOME=/company/home/.omp",
            "-e", "NO_BROWSER=1",
            "-e", &format!("RESTLESS_ACTOR={actor}"),
            "-e", &format!("{}={}", auth.provider_key_env, auth.provider_key),
            container, "omp", "acp", "--model", &auth.model,
        ])
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
    if let Ok(mut slot) = failure.lock() {
        if let Some(error) = slot.take() {
            return Err(error);
        }
    }
    Ok(result?)
}

#[cfg(test)]
mod tests {
    use super::*;
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
