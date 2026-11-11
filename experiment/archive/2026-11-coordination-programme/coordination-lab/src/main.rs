use std::fs::{create_dir_all, read_to_string, rename, write, OpenOptions};
use std::io::Write;
use std::net::TcpStream;
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use agent_client_protocol::{
    self as acp,
    schema::{
        v1::{
            ContentBlock, EnvVariable, InitializeRequest, LoadSessionRequest, McpServer,
            McpServerStdio, NewSessionRequest, PermissionOptionKind, PromptRequest,
            RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
            SelectedPermissionOutcome, SessionNotification, SessionUpdate, TextContent,
            ToolCallStatus,
        },
        ProtocolVersion,
    },
    Agent, ByteStreams, Client, ConnectionTo,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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
    session_id: Option<String>,
    session_resumed: bool,
    session_reconstructed: bool,
    configured_effort: Option<String>,
    cumulative_used_tokens: Option<u64>,
    cumulative_cost_usd: Option<f64>,
}

#[derive(Clone, Default, Deserialize, Serialize)]
struct SessionState {
    session_id: String,
    used_tokens: Option<u64>,
    cost_usd: Option<f64>,
}

fn required(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("missing {name}"))
}

#[derive(Clone)]
enum EventSink {
    File(String),
    Coordinator {
        endpoint: String,
        turn_id: String,
        stream: Arc<Mutex<Option<TcpStream>>>,
    },
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
        // ACP notifications are telemetry, not request/response commands. The
        // coordinator keeps this connection open and deliberately sends no
        // acknowledgement for these records.
        "one_way": matches!(sink, EventSink::Coordinator { .. }),
    });
    match sink {
        EventSink::File(path) => {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(file, "{line}");
            }
        }
        EventSink::Coordinator {
            endpoint, stream, ..
        } => {
            let bytes = format!("{line}\n");
            let Ok(mut stream) = stream.lock() else {
                return;
            };
            // Keep one hot trace stream per model turn. If the coordinator was
            // restarted between notifications, reconnect once and retry.
            for _ in 0..2 {
                if stream.is_none() {
                    match TcpStream::connect(endpoint) {
                        Ok(connection) => {
                            let _ = connection.set_nodelay(true);
                            *stream = Some(connection);
                        }
                        Err(_) => return,
                    }
                }
                if stream
                    .as_mut()
                    .is_some_and(|connection| connection.write_all(bytes.as_bytes()).is_ok())
                {
                    return;
                }
                *stream = None;
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

fn load_session_state(path: &str) -> Option<SessionState> {
    let body = read_to_string(path).ok()?;
    serde_json::from_str(&body).ok().or_else(|| {
        let session_id = body.trim();
        (!session_id.is_empty()).then(|| SessionState {
            session_id: session_id.to_string(),
            ..SessionState::default()
        })
    })
}

fn persist_session_state(path: &str, state: &SessionState) -> Result<()> {
    let state_path = std::path::Path::new(path);
    if let Some(parent) = state_path.parent() {
        create_dir_all(parent)
            .with_context(|| format!("create session-state directory {}", parent.display()))?;
    }
    let temporary = state_path.with_extension(format!("tmp-{}", std::process::id()));
    write(&temporary, format!("{}\n", serde_json::to_string(state)?))
        .with_context(|| format!("write session state {}", temporary.display()))?;
    rename(&temporary, state_path)
        .with_context(|| format!("publish session state {}", state_path.display()))?;
    Ok(())
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
        EventSink::Coordinator {
            endpoint,
            turn_id: turn_id.clone(),
            stream: Arc::new(Mutex::new(None)),
        }
    } else {
        EventSink::File(required("COORD_EVENTS_PATH")?)
    };
    let gateway_token = required("RESTLESS_MODEL_GATEWAY_TOKEN")?;
    let read_only = std::env::var("COORD_READ_ONLY").as_deref() == Ok("1");
    let reasoning_effort = std::env::var("COORD_REASONING_EFFORT")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let session_state_path = std::env::var("COORD_SESSION_STATE_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let prior_session_state = session_state_path.as_deref().and_then(load_session_state);
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
    if let Some(effort) = reasoning_effort.as_ref() {
        args.push("--thinking".to_string());
        args.push(effort.clone());
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
    if let Ok(mut current) = transcript.lock() {
        current.configured_effort = reasoning_effort.clone();
    }
    let sink = Arc::clone(&transcript);
    let response_transcript = Arc::clone(&transcript);
    let capture_updates = Arc::new(AtomicBool::new(false));
    let notification_capture = Arc::clone(&capture_updates);
    let thinking_observed = Arc::new(AtomicBool::new(false));
    let notification_thinking = Arc::clone(&thinking_observed);
    let turn_baseline = Arc::new(Mutex::new(SessionState::default()));
    let notification_baseline = Arc::clone(&turn_baseline);
    let active_session_state = Arc::new(Mutex::new(None::<SessionState>));
    let notification_session_state = Arc::clone(&active_session_state);
    let notification_state_path = session_state_path.clone();
    let event_actor = actor.clone();
    let notification_sink = event_sink.clone();

    let result = Client
        .builder()
        .on_receive_notification(
            move |notification: SessionNotification, _cx| {
                let sink = Arc::clone(&sink);
                let actor = event_actor.clone();
                let event_sink = notification_sink.clone();
                let capture = Arc::clone(&notification_capture);
                let thinking = Arc::clone(&notification_thinking);
                let baseline = Arc::clone(&notification_baseline);
                let session_state = Arc::clone(&notification_session_state);
                let state_path = notification_state_path.clone();
                async move {
                    // ACP session/load replays historical notifications. They reconstruct the
                    // agent, but they are not current-turn output or fresh telemetry.
                    if !capture.load(Ordering::Acquire) {
                        return Ok(());
                    }
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
                                if !thinking.swap(true, Ordering::AcqRel) {
                                    append_event(
                                        &event_sink,
                                        &actor,
                                        "agent_thinking",
                                        serde_json::json!({"state": "active"}),
                                    );
                                }
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
                                let cumulative_cost = usage.cost.as_ref().map(|cost| cost.amount);
                                let observed_baseline = baseline
                                    .lock()
                                    .map(|state| state.clone())
                                    .unwrap_or_default();
                                let used_tokens = usage
                                    .used
                                    .saturating_sub(observed_baseline.used_tokens.unwrap_or(0));
                                let cost_usd = cumulative_cost.map(|cost| {
                                    (cost - observed_baseline.cost_usd.unwrap_or(0.0)).max(0.0)
                                });
                                transcript.used_tokens = Some(used_tokens);
                                transcript.context_size = Some(usage.size);
                                transcript.cost_usd = cost_usd;
                                transcript.cumulative_used_tokens = Some(usage.used);
                                transcript.cumulative_cost_usd = cumulative_cost;
                                if let Ok(mut current) = session_state.lock() {
                                    if let Some(current) = current.as_mut() {
                                        current.used_tokens = Some(usage.used);
                                        current.cost_usd = cumulative_cost;
                                        if let Some(path) = state_path.as_deref() {
                                            let _ = persist_session_state(path, current);
                                        }
                                    }
                                }
                                append_event(
                                    &event_sink,
                                    &actor,
                                    "model_usage",
                                    serde_json::json!({
                                        "used_tokens": usage.used,
                                        "turn_used_tokens": used_tokens,
                                        "context_size": usage.size,
                                        "cost_usd": cost_usd,
                                        "cumulative_cost_usd": cumulative_cost,
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
                EnvVariable::new("COORD_TURN_ID", turn_id.clone()),
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
            let (session_id, resumed, reconstructed, reconstruction_error, baseline) =
                if let Some(prior) = prior_session_state {
                    let prior_id = prior.session_id.clone();
                    match cx
                        .send_request(
                            LoadSessionRequest::new(prior_id.clone(), workdir.clone())
                                .mcp_servers(vec![mcp.clone()]),
                        )
                        .block_task()
                        .await
                    {
                        Ok(_) => (prior_id.into(), true, false, None, prior),
                        Err(error) => {
                            let session = cx
                                .send_request(
                                    NewSessionRequest::new(workdir.clone())
                                        .mcp_servers(vec![mcp.clone()]),
                                )
                                .block_task()
                                .await
                                .context("ACP session/new after failed load")?;
                            (
                                session.session_id,
                                false,
                                true,
                                Some(error.to_string()),
                                SessionState::default(),
                            )
                        }
                    }
                } else {
                    let session = cx
                        .send_request(
                            NewSessionRequest::new(workdir.clone()).mcp_servers(vec![mcp]),
                        )
                        .block_task()
                        .await
                        .context("ACP session/new")?;
                    (
                        session.session_id,
                        false,
                        false,
                        None,
                        SessionState::default(),
                    )
                };
            let session_id_text = session_id.to_string();
            if let Ok(mut current_baseline) = turn_baseline.lock() {
                *current_baseline = baseline.clone();
            }
            let current_session_state = SessionState {
                session_id: session_id_text.clone(),
                used_tokens: baseline.used_tokens,
                cost_usd: baseline.cost_usd,
            };
            if let Ok(mut current) = active_session_state.lock() {
                *current = Some(current_session_state.clone());
            }
            if let Some(path) = session_state_path.as_deref() {
                persist_session_state(path, &current_session_state)?;
            }
            {
                let mut transcript = response_transcript
                    .lock()
                    .map_err(|_| anyhow::anyhow!("transcript lock"))?;
                transcript.session_id = Some(session_id_text.clone());
                transcript.session_resumed = resumed;
                transcript.session_reconstructed = reconstructed;
            }
            append_event(
                &event_sink,
                &actor,
                "model_session",
                serde_json::json!({
                    "session_id": session_id_text,
                    "resumed": resumed,
                    "reconstructed": reconstructed,
                    "reconstruction_error": reconstruction_error,
                }),
            );
            capture_updates.store(true, Ordering::Release);
            let response = cx
                .send_request(PromptRequest::new(
                    session_id,
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
