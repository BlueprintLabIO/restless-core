use std::fs::OpenOptions;
use std::io::Write;
use std::net::TcpStream;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use agent_client_protocol::{
    self as acp,
    schema::{
        v1::{
            ContentBlock, EnvVariable, InitializeRequest, McpServer, McpServerStdio,
            NewSessionRequest, PermissionOptionKind, PromptRequest, RequestPermissionOutcome,
            RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
            SessionNotification, SessionUpdate, TextContent, ToolCallStatus,
        },
        ProtocolVersion,
    },
    Agent, ByteStreams, Client, ConnectionTo,
};
use anyhow::{Context, Result};
use serde::Serialize;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(Default, Serialize)]
struct Transcript {
    text: String,
    tool_calls: Vec<String>,
    used_tokens: Option<u64>,
    context_size: Option<u64>,
    cost_usd: Option<f64>,
    output_tokens: Option<u64>,
    stop_reason: Option<String>,
}

fn required(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("missing {name}"))
}

#[derive(Clone)]
enum EventSink {
    File(String),
    Coordinator { endpoint: String, turn_id: String },
}

fn append_event(sink: &EventSink, actor: &str, kind: &str, payload: serde_json::Value) {
    let line = serde_json::json!({
        "type": "trace",
        "at": chrono_like_now(),
        "actor": actor,
        "kind": kind,
        "payload": payload,
        "turn_id": match sink {
            EventSink::Coordinator { turn_id, .. } => Some(turn_id.as_str()),
            EventSink::File(_) => None,
        },
    });
    match sink {
        EventSink::File(path) => {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(file, "{line}");
            }
        }
        EventSink::Coordinator { endpoint, .. } => {
            if let Ok(mut stream) = TcpStream::connect(endpoint) {
                let _ = writeln!(stream, "{line}");
            }
        }
    }
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    ms.to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--capabilities") {
        println!(
            "{}",
            serde_json::json!({
                "protocol": 24,
                "optional_actor_max_time": true,
                "completion": "actor_callback_or_process_exit"
            })
        );
        return Ok(());
    }
    let actor = required("COORD_ACTOR")?;
    let model = required("COORD_MODEL")?;
    let prompt_path = required("COORD_PROMPT_PATH")?;
    let system_path = required("COORD_SYSTEM_PATH")?;
    let workdir = required("COORD_WORKDIR")?;
    let attempt = std::env::var("COORD_ATTEMPT").unwrap_or_default();
    let lease_token = std::env::var("COORD_LEASE_TOKEN").unwrap_or_default();
    let run_id = required("COORD_RUN_ID")?;
    let container = required("COORD_CONTAINER")?;
    let turn_id = std::env::var("COORD_TURN_ID").unwrap_or_default();
    let coordinator_endpoint = std::env::var("COORD_ENDPOINT").ok();
    let event_sink = if let Some(endpoint) = std::env::var("COORD_EVENT_ENDPOINT").ok() {
        EventSink::Coordinator { endpoint, turn_id }
    } else {
        EventSink::File(required("COORD_EVENTS_PATH")?)
    };
    let gateway_token = required("RESTLESS_MODEL_GATEWAY_TOKEN")?;
    let read_only = std::env::var("COORD_READ_ONLY").as_deref() == Ok("1");
    let native_tools = if read_only {
        "read,grep"
    } else {
        "read,bash,edit,write,grep"
    };
    let prompt = std::fs::read_to_string(&prompt_path)
        .with_context(|| format!("read prompt {prompt_path}"))?;

    let agent_home = std::env::var("COORD_AGENT_HOME")
        .unwrap_or_else(|_| format!("/lab/{run_id}/agent-home/{actor}"));
    let runtime_bin =
        std::env::var("COORD_RUNTIME_BIN").unwrap_or_else(|_| format!("/lab/{run_id}/runtime-bin"));
    let max_time = std::env::var("COORD_MAX_TIME").unwrap_or_else(|_| "none".to_string());
    let mcp_server_path =
        std::env::var("COORD_MCP_SERVER").unwrap_or_else(|_| "/harness/mcp_server.py".to_string());
    let mut args = vec![
        "exec".to_string(),
        "-i".to_string(),
        "-u".to_string(),
        "company".to_string(),
        "-w".to_string(),
        workdir.clone(),
        "-e".to_string(),
        format!("PI_CODING_AGENT_DIR={agent_home}"),
        "-e".to_string(),
        "NO_BROWSER=1".to_string(),
        "-e".to_string(),
        "RESTLESS_MODEL_GATEWAY_TOKEN".to_string(),
        "-e".to_string(),
        "COORD_GATEWAY_PORT".to_string(),
        "-e".to_string(),
        format!("PATH={runtime_bin}:/usr/local/bin:/usr/bin:/bin"),
        container,
        "/usr/local/bin/omp".to_string(),
        "acp".to_string(),
        "--model".to_string(),
        model,
        "--system-prompt".to_string(),
        system_path,
        "--config".to_string(),
        "/harness/omp-runtime.yml".to_string(),
        "--extension".to_string(),
        "/harness/v2/openrouter-live-free-models.ts".to_string(),
        "--no-extensions".to_string(),
        "--no-rules".to_string(),
        "--tools".to_string(),
        native_tools.to_string(),
    ];
    if max_time != "none" {
        args.push("--max-time".to_string());
        args.push(max_time);
    }

    let mut child = tokio::process::Command::new("docker")
        .env("RESTLESS_MODEL_GATEWAY_TOKEN", gateway_token)
        .args(args.drain(..))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .context("spawn OMP ACP agent")?;
    let stdin = child.stdin.take().context("ACP stdin")?;
    let stdout = child.stdout.take().context("ACP stdout")?;
    let transport = ByteStreams::new(stdin.compat_write(), stdout.compat());
    let transcript = Arc::new(Mutex::new(Transcript::default()));
    let sink = Arc::clone(&transcript);
    let response_transcript = Arc::clone(&transcript);
    let event_actor = actor.clone();
    let notification_sink = event_sink.clone();

    let result = Client
        .builder()
        .on_receive_notification(
            move |notification: SessionNotification, _cx| {
                let sink = Arc::clone(&sink);
                let actor = event_actor.clone();
                let event_sink = notification_sink.clone();
                async move {
                    if let Ok(mut transcript) = sink.lock() {
                        match notification.update {
                            SessionUpdate::AgentMessageChunk(chunk) => {
                                if let ContentBlock::Text(text) = chunk.content {
                                    transcript.text.push_str(&text.text);
                                    append_event(
                                        &event_sink,
                                        &actor,
                                        "agent_text",
                                        serde_json::json!({"text": text.text}),
                                    );
                                }
                            }
                            SessionUpdate::AgentThoughtChunk(_) => {
                                append_event(
                                    &event_sink,
                                    &actor,
                                    "agent_thinking",
                                    serde_json::json!({}),
                                );
                            }
                            SessionUpdate::ToolCall(call) => {
                                transcript.tool_calls.push(call.title.clone());
                                append_event(
                                    &event_sink,
                                    &actor,
                                    "tool_started",
                                    serde_json::json!({
                                        "id": call.tool_call_id.to_string(), "title": call.title,
                                        "tool_kind": format!("{:?}", call.kind).to_lowercase()
                                    }),
                                );
                            }
                            SessionUpdate::ToolCallUpdate(update) => {
                                let status = update
                                    .fields
                                    .status
                                    .map(|s| format!("{s:?}").to_lowercase());
                                if matches!(
                                    update.fields.status,
                                    Some(ToolCallStatus::Completed | ToolCallStatus::Failed)
                                ) {
                                    append_event(
                                        &event_sink,
                                        &actor,
                                        "tool_terminal",
                                        serde_json::json!({
                                            "id": update.tool_call_id.to_string(), "status": status,
                                            "title": update.fields.title
                                        }),
                                    );
                                }
                            }
                            SessionUpdate::UsageUpdate(usage) => {
                                let cost_usd = usage.cost.as_ref().map(|cost| cost.amount);
                                transcript.used_tokens = Some(usage.used);
                                transcript.context_size = Some(usage.size);
                                transcript.cost_usd = cost_usd;
                                append_event(
                                    &event_sink,
                                    &actor,
                                    "model_usage",
                                    serde_json::json!({
                                        "used_tokens": usage.used,
                                        "context_size": usage.size,
                                        "cost_usd": cost_usd,
                                    }),
                                );
                            }
                            _ => {}
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
                let title = request
                    .tool_call
                    .fields
                    .title
                    .as_deref()
                    .unwrap_or_default();
                let raw_input = request
                    .tool_call
                    .fields
                    .raw_input
                    .as_ref()
                    .map(serde_json::Value::to_string)
                    .unwrap_or_default();
                let proposed = format!("{title}\n{raw_input}").to_lowercase();
                let forbidden_nested_model = [
                    "$ omp ",
                    "\"omp ",
                    "/omp ",
                    "$ claude ",
                    "\"claude ",
                    "$ codex ",
                    "\"codex ",
                ]
                .iter()
                .any(|needle| proposed.contains(needle));
                let option = if forbidden_nested_model {
                    request.options.iter().find(|option| {
                        matches!(
                            option.kind,
                            PermissionOptionKind::RejectOnce | PermissionOptionKind::RejectAlways
                        )
                    })
                } else {
                    request
                        .options
                        .iter()
                        .find(|option| {
                            matches!(
                                option.kind,
                                PermissionOptionKind::AllowOnce | PermissionOptionKind::AllowAlways
                            )
                        })
                        .or_else(|| request.options.first())
                };
                let outcome = option
                    .map(|option| {
                        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                            option.option_id.clone(),
                        ))
                    })
                    .unwrap_or(RequestPermissionOutcome::Cancelled);
                responder.respond(RequestPermissionResponse::new(outcome))
            },
            acp::on_receive_request!(),
        )
        .connect_with(transport, async move |cx: ConnectionTo<Agent>| {
            cx.send_request(InitializeRequest::new(ProtocolVersion::V1))
                .block_task()
                .await
                .context("ACP initialize")?;

            let mut mcp_env = vec![
                EnvVariable::new("COORD_ACTOR", actor.clone()),
                EnvVariable::new("COORD_ATTEMPT", attempt),
                EnvVariable::new("COORD_LEASE_TOKEN", lease_token),
                EnvVariable::new("COORD_RUN_ID", run_id.clone()),
            ];
            if let Some(endpoint) = coordinator_endpoint {
                mcp_env.push(EnvVariable::new("COORD_ENDPOINT", endpoint));
            } else {
                let database = required("COORD_DB_CONTAINER")?;
                let notify_port = required("COORD_NOTIFY_PORT")?;
                mcp_env.extend([
                    EnvVariable::new("COORD_DB", database),
                    EnvVariable::new("COORD_NOTIFY_HOST", "host.docker.internal"),
                    EnvVariable::new("COORD_NOTIFY_PORT", notify_port),
                    EnvVariable::new("COORD_REPO", format!("/lab/{run_id}/repo")),
                ]);
            }
            let mcp = McpServer::Stdio(
                McpServerStdio::new("orgintel-coordination", "/usr/bin/python3")
                    .args(vec![mcp_server_path])
                    .env(mcp_env),
            );
            let session = cx
                .send_request(NewSessionRequest::new(workdir).mcp_servers(vec![mcp]))
                .block_task()
                .await
                .context("ACP session/new")?;
            let response = cx
                .send_request(PromptRequest::new(
                    session.session_id,
                    vec![ContentBlock::Text(TextContent::new(prompt))],
                ))
                .block_task()
                .await
                .context("ACP prompt")?;
            let mut transcript = response_transcript
                .lock()
                .map_err(|_| anyhow::anyhow!("transcript lock"))?;
            transcript.stop_reason = Some(format!("{:?}", response.stop_reason).to_lowercase());
            transcript.output_tokens = response.usage.map(|usage| usage.output_tokens);
            Ok(())
        })
        .await;

    let _ = child.kill().await;
    result.map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let final_transcript = transcript
        .lock()
        .map_err(|_| anyhow::anyhow!("transcript lock"))?;
    println!("{}", serde_json::to_string(&*final_transcript)?);
    Ok(())
}
