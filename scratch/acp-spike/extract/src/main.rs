//! Branch (a): extraction spike. The pure ACP session lifecycle lifted out of
//! legacy contained.rs, with the envelope/fence/tunnel scaffolding cut away.
//! Functions below marked LIFT are verbatim (modulo type adjustments) from
//! helm crates/company-runtime/src/contained.rs.

use std::collections::BTreeMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use agent_client_protocol::{
    self as acp, Agent, ByteStreams, Client, ConnectionTo,
    schema::{
        ProtocolVersion,
        v1::{
            AuthenticateRequest, CancelNotification, ClientCapabilities, ContentBlock,
            InitializeRequest, NewSessionRequest, PromptRequest, RequestPermissionOutcome,
            RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
            SessionConfigKind, SessionConfigOption, SessionConfigSelectOptions,
            SessionNotification, SessionUpdate, SetSessionConfigOptionRequest, TextContent,
            ToolCall,
        },
    },
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

// --- LIFT: contained.rs initialize_request (gateway arm dropped) -------------
fn initialize_request(version: ProtocolVersion) -> InitializeRequest {
    InitializeRequest::new(version).client_capabilities(ClientCapabilities::new())
}

// --- LIFT: contained.rs require_protocol_v1 (verbatim) -----------------------
fn require_protocol_v1(
    initialized: &agent_client_protocol::schema::v1::InitializeResponse,
) -> Result<(), agent_client_protocol::Error> {
    if initialized.protocol_version == ProtocolVersion::V1 {
        Ok(())
    } else {
        Err(agent_client_protocol::util::internal_error(format!(
            "adapter negotiated unsupported ACP version {:?}",
            initialized.protocol_version
        )))
    }
}

// --- LIFT: contained.rs Capture + capture_notification (verbatim) ------------
#[derive(Default)]
struct Capture {
    output: String,
    tool_updates: u64,
    allowed_permissions: u64,
    rejected_permissions: u64,
    cancelled_permissions: u64,
    tool_calls: BTreeMap<String, ToolCall>,
}

fn capture_notification(captured: &mut Capture, notification: SessionNotification) {
    match notification.update {
        SessionUpdate::AgentMessageChunk(chunk) => {
            if let ContentBlock::Text(text) = chunk.content {
                captured.output.push_str(&text.text);
            }
        }
        SessionUpdate::ToolCall(call) => {
            captured.tool_updates = captured.tool_updates.saturating_add(1);
            let _ = captured
                .tool_calls
                .insert(call.tool_call_id.to_string(), call);
        }
        SessionUpdate::ToolCallUpdate(update) => {
            captured.tool_updates = captured.tool_updates.saturating_add(1);
            let key = update.tool_call_id.to_string();
            if let Some(call) = captured.tool_calls.get_mut(&key) {
                call.update(update.fields);
            } else if let Ok(call) = ToolCall::try_from(update) {
                let _ = captured.tool_calls.insert(key, call);
            }
        }
        _ => {}
    }
}

// --- LIFT: contained.rs select_session_model + require_selected_model_option -
async fn select_session_model(
    connection: &ConnectionTo<Agent>,
    session: &agent_client_protocol::schema::v1::NewSessionResponse,
    selected_model: &str,
) -> Result<(), agent_client_protocol::Error> {
    require_selected_model_option(session.config_options.as_deref().unwrap_or_default(), selected_model, false)?;
    let configured = connection
        .send_request(SetSessionConfigOptionRequest::new(
            session.session_id.clone(),
            "model",
            selected_model,
        ))
        .block_task()
        .await?;
    require_selected_model_option(&configured.config_options, selected_model, true)
}

fn require_selected_model_option(
    options: &[SessionConfigOption],
    selected_model: &str,
    require_current: bool,
) -> Result<(), agent_client_protocol::Error> {
    let model = options
        .iter()
        .find(|option| option.id.0.as_ref() == "model")
        .ok_or_else(|| {
            agent_client_protocol::util::internal_error(
                "ACP session did not advertise the standard selected model configuration",
            )
        })?;
    let SessionConfigKind::Select(select) = &model.kind else {
        return Err(agent_client_protocol::util::internal_error(
            "ACP selected model configuration was not a select option",
        ));
    };
    let offered = match &select.options {
        SessionConfigSelectOptions::Ungrouped(options) => options
            .iter()
            .any(|option| option.value.0.as_ref() == selected_model),
        SessionConfigSelectOptions::Grouped(groups) => groups.iter().any(|group| {
            group
                .options
                .iter()
                .any(|option| option.value.0.as_ref() == selected_model)
        }),
        _ => false,
    };
    if !offered {
        return Err(agent_client_protocol::util::internal_error(format!(
            "ACP session did not offer the exact selected model {selected_model}",
        )));
    }
    if require_current && select.current_value.0.as_ref() != selected_model {
        return Err(agent_client_protocol::util::internal_error(format!(
            "ACP selected model verification returned a different current value than {selected_model}",
        )));
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let agent_js = std::env::var("ACP_AGENT_JS").expect("ACP_AGENT_JS");
    let codex_home = std::env::var("CODEX_HOME").expect("CODEX_HOME");
    let api_key = std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY");
    let workdir = std::env::var("ACP_WORKDIR").expect("ACP_WORKDIR");
    let model = std::env::var("ACP_MODEL").unwrap_or_else(|_| "anthropic/claude-sonnet-4".into());

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

    let capture = Arc::new(Mutex::new(Capture::default()));
    let notification_capture = Arc::clone(&capture);

    let result = Client
        .builder()
        .on_receive_notification(
            move |notification: SessionNotification, _cx| {
                let capture = Arc::clone(&notification_capture);
                async move {
                    if let Ok(mut captured) = capture.lock() {
                        capture_notification(&mut captured, notification);
                    }
                    Ok(())
                }
            },
            acp::on_receive_notification!(),
        )
        .on_receive_request(
            move |_request: RequestPermissionRequest,
                  responder: acp::Responder<RequestPermissionResponse>,
                  _cx| async move {
                let outcome = RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                    _request.options[0].option_id.clone(),
                ));
                responder.respond(RequestPermissionResponse::new(outcome))
            },
            acp::on_receive_request!(),
        )
        .connect_with(transport, async move |cx: ConnectionTo<Agent>| {
            let initialized = cx
                .send_request(initialize_request(ProtocolVersion::V1))
                .block_task()
                .await?;
            require_protocol_v1(&initialized)?;
            println!("[probe] initialized + v1 required: ok");

            // api-key authenticate (gateway arm exists in the legacy code; not
            // lifted here because the gateway does not exist yet this sprint).
            let mut auth_meta = serde_json::Map::new();
            auth_meta.insert("api-key".to_string(), serde_json::json!({ "apiKey": api_key }));
            cx.send_request(AuthenticateRequest::new("api-key").meta(auth_meta))
                .block_task()
                .await?;
            println!("[probe] authenticated");

            let session = cx
                .send_request(NewSessionRequest::new(workdir.clone()).mcp_servers(vec![]))
                .block_task()
                .await?;
            println!("[probe] session: {}", session.session_id);

            // LIFT: live-verify the session model (probe, never guess).
            select_session_model(&cx, &session, &model).await?;
            println!("[probe] model verified: {model}");

            let response = cx
                .send_request(PromptRequest::new(
                    session.session_id.clone(),
                    vec![ContentBlock::Text(TextContent::new(
                        "Create a file named hello-extract.txt containing exactly: extract-ok\nThen reply DONE.",
                    ))],
                ))
                .block_task()
                .await?;

            let captured = capture.lock().map(|c| c.output.len()).unwrap_or(0);
            println!(
                "[probe] stop: {:?}, captured output bytes: {captured}, tool updates: {}",
                response.stop_reason,
                capture.lock().map(|c| c.tool_updates).unwrap_or(0)
            );
            Ok(())
        })
        .await;

    let _ = child.kill().await;
    result.map_err(|e| anyhow::anyhow!(e.to_string()))
}
