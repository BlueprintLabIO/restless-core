//! Branch (b): fresh thin ACP client. The persistent container means the agent
//! does its own file IO; the client is only: spawn over stdio → initialize →
//! session/new → prompt → stream updates → cancel. Permissions auto-allow
//! (permissive inside the company environment, ARCHITECTURE.md §2.1).

use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use agent_client_protocol::{
    self as acp, Agent, ByteStreams, Client, ConnectionTo, Error,
    schema::{
        ProtocolVersion,
        v1::{
            CancelNotification, ContentBlock, InitializeRequest, NewSessionRequest,
            PermissionOptionKind, PromptRequest, RequestPermissionOutcome,
            RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
            SessionNotification, SessionUpdate, TextContent,
        },
    },
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let agent_js = std::env::var("ACP_AGENT_JS").expect("ACP_AGENT_JS: path to codex-acp index.js");
    let codex_home = std::env::var("CODEX_HOME").expect("CODEX_HOME");
    let api_key = std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY");
    let workdir = std::env::var("ACP_WORKDIR").expect("ACP_WORKDIR");
    let cancel_test = std::env::args().any(|a| a == "--cancel-test");

    let mut child = tokio::process::Command::new("node")
        .arg(agent_js)
        .env("CODEX_HOME", &codex_home)
        .env("OPENROUTER_API_KEY", &api_key)
        .env("NO_BROWSER", "1")
        .current_dir(&workdir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()?;
    let stdin = child.stdin.take().expect("piped stdin");
    let stdout = child.stdout.take().expect("piped stdout");
    let transport = ByteStreams::new(stdin.compat_write(), stdout.compat());

    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_in_handler = Arc::clone(&cancelled);

    let result = Client
        .builder()
        .on_receive_notification(
            move |notification: SessionNotification, _cx| {
                let cancelled = Arc::clone(&cancelled_in_handler);
                async move {
                    match notification.update {
                        SessionUpdate::AgentMessageChunk(chunk) => {
                            if let ContentBlock::Text(t) = chunk.content {
                                print!("{}", t.text);
                                use std::io::Write;
                                let _ = std::io::stdout().flush();
                            }
                        }
                        SessionUpdate::ToolCall(call) => {
                            println!("\n[tool:{:?}] {}", call.kind, call.title);
                            cancelled.store(true, Ordering::SeqCst);
                        }
                        _ => {}
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
                // Auto-allow: pick the first allow-kind option, else the first option.
                let option = request
                    .options
                    .iter()
                    .find(|o| matches!(o.kind, PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways))
                    .or_else(|| request.options.first());
                let outcome = match option {
                    Some(o) => RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                        o.option_id.clone(),
                    )),
                    None => RequestPermissionOutcome::Cancelled,
                };
                responder.respond(RequestPermissionResponse::new(outcome))
            },
            acp::on_receive_request!(),
        )
        .connect_with(transport, async move |cx: ConnectionTo<Agent>| {
            let initialized = cx
                .send_request(InitializeRequest::new(ProtocolVersion::V1))
                .block_task()
                .await?;
            let info = initialized
                .agent_info
                .as_ref()
                .map(|i| format!("{} v{}", i.name, i.version))
                .unwrap_or_else(|| "unknown".to_string());
            println!("[probe] initialized: {info}");

            let mut auth_meta = serde_json::Map::new();
            auth_meta.insert(
                "api-key".to_string(),
                serde_json::json!({ "apiKey": api_key }),
            );
            cx.send_request(
                agent_client_protocol::schema::v1::AuthenticateRequest::new("api-key")
                    .meta(auth_meta),
            )
            .block_task()
            .await?;
            println!("[probe] authenticated (api-key)");

            let session = cx
                .send_request(NewSessionRequest::new(workdir.clone()).mcp_servers(vec![]))
                .block_task()
                .await?;
            println!("[probe] session: {}", session.session_id);

            let prompt_text = if cancel_test {
                "Write a long detailed essay about the history of computing to essay.md. Take your time."
            } else {
                "Create a file named hello-acp.txt containing exactly the text: acp-ok\nThen reply with DONE."
            };
            println!("[probe] prompt: {prompt_text}");

            let prompt = cx
                .send_request(PromptRequest::new(
                    session.session_id.clone(),
                    vec![ContentBlock::Text(TextContent::new(prompt_text))],
                ))
                .block_task();
            tokio::pin!(prompt);

            let response = if cancel_test {
                let cancelled = Arc::clone(&cancelled);
                let first_tool_call = async {
                    while !cancelled.load(Ordering::SeqCst) {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                };
                tokio::select! {
                    r = &mut prompt => r?,
                    _ = first_tool_call => {
                        cx.send_notification(CancelNotification::new(session.session_id.clone()))?;
                        println!("\n[probe] cancel sent after first tool call");
                        prompt.await?
                    }
                }
            } else {
                prompt.await?
            };

            println!("\n[probe] stop reason: {:?}", response.stop_reason);
            Ok(info)
        })
        .await;

    let _ = child.kill().await;
    match result {
        Ok(info) => {
            println!("[probe] OK — agent was: {info}");
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!(e.to_string())),
    }
}
