//! ACP client (sprint 01 T3 canon, grown into the daemon): spawn the pinned
//! codex-acp binary as an ordinary process inside the company's persistent
//! container (`docker exec`, §5), speak JSON-RPC over stdio, stream turn
//! updates, cancel on demand. The session is disposable; the company is not.

use std::process::Stdio;
use std::sync::{Arc, Mutex};

use agent_client_protocol::{
    self as acp, Agent, ByteStreams, Client, ConnectionTo,
    schema::{
        ProtocolVersion,
        v1::{
            AuthenticateRequest, CancelNotification, ContentBlock, InitializeRequest,
            NewSessionRequest, PermissionOptionKind, PromptRequest, RequestPermissionOutcome,
            RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
            SessionId, SessionNotification, SessionUpdate, TextContent,
        },
    },
};
use anyhow::{Context, Result};
use tokio_util::compat::{TokioAsyncReadCompatExt as _, TokioAsyncWriteCompatExt as _};

/// Everything a wake needs to point an agent at the gateway (T2): the token
/// travels in the authenticate `_meta` over stdio — never container env,
/// never on disk.
pub struct GatewayAuth {
    pub base_url: String,
    pub token: String,
}

/// One observed turn: the agent's visible text plus the tool calls it made.
/// This is observability, not a governed record (§4.4).
#[derive(Debug, Default)]
pub struct TurnTranscript {
    pub text: String,
    pub tool_calls: Vec<String>,
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
    auth: &GatewayAuth,
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
            "-e", "CODEX_HOME=/company/home/.codex",
            "-e", "NO_BROWSER=1",
            "-e", &format!("RESTLESS_ACTOR={actor}"),
            container, "codex-acp",
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
    let auth_meta = gateway_meta(auth);
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

                cx.send_request(AuthenticateRequest::new("gateway").meta(auth_meta))
                    .block_task()
                    .await
                    .context("acp authenticate (gateway)")?;

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

/// The exact `_meta` codex-acp's `gateway` auth method consumes (probed from
/// the installed 1.1.4 source): baseUrl, headers, providerName; the wire
/// protocol is fixed to openai/responses by the binary.
fn gateway_meta(auth: &GatewayAuth) -> serde_json::Map<String, serde_json::Value> {
    let mut meta = serde_json::Map::new();
    meta.insert(
        "gateway".to_string(),
        serde_json::json!({
            "baseUrl": auth.base_url,
            "providerName": "restless-gateway",
            "headers": { "Authorization": format!("Bearer {}", auth.token) },
        }),
    );
    meta
}
