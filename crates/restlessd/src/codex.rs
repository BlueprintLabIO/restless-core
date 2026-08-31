//! First-party Codex app-server transport for productive Staff Attempts.
//!
//! The byte-identical JSONL runner is also used by EXP-17's neutral solo
//! controller. This adapter contributes only Restless process ownership,
//! scoped capabilities, hot-session custody, observations and cancellation;
//! it does not add planning or semantic rescue.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{mpsc, Mutex as AsyncMutex};
use tokio_util::sync::CancellationToken;

use crate::acp::{
    AgentAuth, LiveSessionEvent, SessionObserver, TurnEnd, TurnTranscript, TurnUsage,
};

const CODEX_ROOT: &str = "/company/home/.restless/codex-agent";
const RUNNER: &str = "/usr/local/bin/restless-codex-runner";
const MODEL_CAPABILITY_ENV: &str = "RESTLESS_MODEL_CAPABILITY";
const IDLE_SILENT: Duration = Duration::from_secs(8 * 60);
const IDLE_TOOL_RUNNING: Duration = Duration::from_secs(15 * 60);
const WATCHDOG_TICK: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionLocator {
    version: u8,
    company: String,
    actor: String,
    responsibility: String,
    cwd: String,
    model: String,
    effort: String,
    thread_id: String,
    runner_digest: String,
}

fn scope_digest(company: &str, actor: &str, responsibility: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(format!("{company}\0{actor}\0{responsibility}").as_bytes())
    )
}

fn locator_path(company: &str, actor: &str, responsibility: &str) -> String {
    format!(
        "{CODEX_ROOT}/sessions/{}.json",
        scope_digest(company, actor, responsibility)
    )
}

fn home_path(company: &str, actor: &str, responsibility: &str) -> String {
    format!(
        "{CODEX_ROOT}/homes/{}",
        scope_digest(company, actor, responsibility)
    )
}

async fn read_locator(container: &str, path: &str) -> Result<Option<SessionLocator>> {
    let output = Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            container,
            "sh",
            "-c",
            "test ! -e \"$1\" || cat \"$1\"",
            "restless-read-codex-session",
            path,
        ])
        .output()
        .await
        .context("read Codex session locator")?;
    if !output.status.success() {
        bail!("read Codex session locator failed");
    }
    let body = String::from_utf8_lossy(&output.stdout);
    if body.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(
        serde_json::from_str(body.trim()).context("parse Codex session locator")?,
    ))
}

async fn persist_locator(container: &str, path: &str, locator: &SessionLocator) -> Result<()> {
    crate::acp::write_private_container_file(
        container,
        path,
        &serde_json::to_string(locator).context("encode Codex session locator")?,
    )
    .await
}

pub(crate) async fn discard_session_locator(
    container: &str,
    company: &str,
    actor: &str,
    responsibility: &str,
) -> Result<()> {
    let path = locator_path(company, actor, responsibility);
    let output = Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            container,
            "sh",
            "-c",
            "test ! -e \"$1\" || unlink \"$1\"",
            "restless-discard-codex-session",
            &path,
        ])
        .output()
        .await
        .context("discard Codex session locator")?;
    if !output.status.success() {
        bail!("discard Codex session locator failed");
    }
    Ok(())
}

async fn prove_tool_contract(container: &str, auth: &AgentAuth, actor: &str) -> Result<String> {
    let mut args = vec![
        "exec".to_string(),
        "-u".to_string(),
        "company".to_string(),
        "-e".to_string(),
        auth.coordination_token_env.clone(),
        "-e".to_string(),
        format!("RESTLESS_ACTOR={actor}"),
        "-e".to_string(),
        format!(
            "RESTLESS_COORDINATOR={}",
            crate::acp::runtime_coordinator()?
        ),
        container.to_string(),
        "restless".to_string(),
        "people".to_string(),
        "-c".to_string(),
        auth.company.clone(),
    ];
    let output = Command::new("docker")
        .env(&auth.coordination_token_env, &auth.coordination_token)
        .args(&args)
        .output()
        .await
        .context("probe Codex coordination tool contract")?;
    args.clear();
    if !output.status.success() {
        bail!("Codex coordination readiness failed before prompt");
    }
    Ok(format!(
        "{:x}",
        Sha256::digest(
            format!("codex-native:shell,apply_patch\0coordination:restless-people\0actor:{actor}")
                .as_bytes()
        )
    ))
}

fn event_type(value: &serde_json::Value) -> &str {
    value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
}

fn event_string<'a>(value: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(serde_json::Value::as_str)
}

async fn send_operation(
    stdin: &Arc<AsyncMutex<ChildStdin>>,
    value: serde_json::Value,
) -> Result<()> {
    let mut stdin = stdin.lock().await;
    stdin
        .write_all(serde_json::to_string(&value)?.as_bytes())
        .await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;
    Ok(())
}

pub(crate) struct CodexSession {
    stdin: Arc<AsyncMutex<ChildStdin>>,
    events: AsyncMutex<mpsc::UnboundedReceiver<serde_json::Value>>,
    observer: Option<SessionObserver>,
    live_observer_enabled: Arc<AtomicBool>,
    pub(crate) launch_id: String,
    pub(crate) thread_id: String,
    pub(crate) model: String,
    pub(crate) effort: String,
    pub(crate) resumed: bool,
    pub(crate) reconstructed: bool,
    pub(crate) reconstruction_reason: Option<String>,
    pub(crate) runner_digest: String,
    pub(crate) tool_contract_digest: String,
    pub(crate) observed: serde_json::Value,
}

impl CodexSession {
    pub(crate) fn readiness_observation(&self) -> serde_json::Value {
        serde_json::json!({
            "transport": "codex_app_server",
            "launch_id": self.launch_id,
            "model": self.model,
            "configured_effort": self.effort,
            "session_id": self.thread_id,
            "resumed": self.resumed,
            "reconstructed": self.reconstructed,
            "reconstruction_reason": self.reconstruction_reason,
            "runner_digest": self.runner_digest,
            "tool_contract_digest": self.tool_contract_digest,
            "responses_tariff_version": crate::model_gateway::RESPONSES_TARIFF_VERSION,
            "observed": self.observed,
            "fresh_process_capability": true,
        })
    }

    pub(crate) fn set_live_observer_enabled(&self, enabled: bool) {
        self.live_observer_enabled.store(enabled, Ordering::Release);
    }

    fn observe(&self, event: LiveSessionEvent) {
        if self.live_observer_enabled.load(Ordering::Acquire) {
            if let Some(observer) = &self.observer {
                observer(event);
            }
        }
    }

    pub(crate) async fn prompt_live(
        &self,
        text: &str,
        enforce_spend_budget: bool,
        remaining_budget_usd: f64,
        cancellation: &CancellationToken,
    ) -> TurnEnd {
        if let Err(error) = send_operation(
            &self.stdin,
            serde_json::json!({
                "op": "turn",
                "request_id": uuid::Uuid::new_v4().simple().to_string(),
                "text": text,
            }),
        )
        .await
        {
            return TurnEnd::Failed {
                error: format!("start Codex turn: {error:#}"),
                transcript: TurnTranscript::default(),
            };
        }

        let mut transcript = TurnTranscript::default();
        let mut last_message_id: Option<String> = None;
        let mut last_activity = Instant::now();
        let mut tools_in_flight = 0usize;
        let mut events = self.events.lock().await;
        loop {
            tokio::select! {
                () = cancellation.cancelled() => {
                    let _ = send_operation(
                        &self.stdin,
                        serde_json::json!({"op":"interrupt","request_id":uuid::Uuid::new_v4().simple().to_string()}),
                    ).await;
                    return TurnEnd::Interrupted { transcript };
                }
                () = tokio::time::sleep(WATCHDOG_TICK) => {
                    if enforce_spend_budget && transcript.usage.as_ref().is_some_and(|usage| {
                        usage.cost_usd.is_some_and(|cost| cost >= remaining_budget_usd)
                    }) {
                        let _ = send_operation(&self.stdin, serde_json::json!({"op":"interrupt"})).await;
                        return TurnEnd::OverBudget { transcript };
                    }
                    let allowance = if tools_in_flight > 0 { IDLE_TOOL_RUNNING } else { IDLE_SILENT };
                    if last_activity.elapsed() > allowance {
                        let idle = last_activity.elapsed();
                        let _ = send_operation(&self.stdin, serde_json::json!({"op":"interrupt"})).await;
                        return TurnEnd::Wedged { idle, transcript };
                    }
                }
                event = events.recv() => {
                    let Some(event) = event else {
                        return TurnEnd::Failed {
                            error: "Codex runner event stream closed".into(),
                            transcript,
                        };
                    };
                    let kind = event_type(&event);
                    if matches!(kind, "agent_text_delta" | "reasoning_activity" | "item_started" | "item_completed" | "usage" | "turn_started") {
                        last_activity = Instant::now();
                    }
                    match kind {
                        "agent_text_delta" => {
                            let text = event_string(&event, "text").unwrap_or_default();
                            let message_id = event_string(&event, "item_id").map(str::to_string);
                            if last_message_id.is_some() && message_id.is_some() && last_message_id != message_id {
                                transcript.last_message_text.clear();
                                if !transcript.text.is_empty() && !transcript.text.ends_with("\n\n") {
                                    transcript.text.push_str("\n\n");
                                }
                            }
                            if message_id.is_some() {
                                last_message_id = message_id.clone();
                            }
                            transcript.text.push_str(text);
                            transcript.last_message_text.push_str(text);
                            self.observe(LiveSessionEvent::ReplyDelta { message_id, text: text.to_string() });
                        }
                        "reasoning_activity" => self.observe(LiveSessionEvent::ThoughtDelta),
                        "item_started" => {
                            let item = event.get("item").cloned().unwrap_or_default();
                            let item_kind = event_string(&item, "type").unwrap_or("tool");
                            if !matches!(item_kind, "agentMessage" | "reasoning" | "userMessage") {
                                tools_in_flight = tools_in_flight.saturating_add(1);
                                let id = event_string(&item, "id").unwrap_or("codex-tool").to_string();
                                let title = event_string(&item, "command")
                                    .or_else(|| event_string(&item, "path"))
                                    .or_else(|| event_string(&item, "tool"))
                                    .unwrap_or(item_kind)
                                    .to_string();
                                transcript.tool_calls.push(format!("{item_kind}: {title}"));
                                self.observe(LiveSessionEvent::ToolStarted { id, title, kind: item_kind.to_string() });
                            }
                        }
                        "item_completed" => {
                            let item = event.get("item").cloned().unwrap_or_default();
                            let item_kind = event_string(&item, "type").unwrap_or("tool");
                            if !matches!(item_kind, "agentMessage" | "reasoning" | "userMessage") {
                                tools_in_flight = tools_in_flight.saturating_sub(1);
                                self.observe(LiveSessionEvent::ToolUpdated {
                                    id: event_string(&item, "id").unwrap_or("codex-tool").to_string(),
                                    title: None,
                                    status: event_string(&item, "status").unwrap_or("completed").to_string(),
                                });
                            }
                        }
                        "usage" => {
                            let usage = event.get("token_usage").cloned().unwrap_or_default();
                            let last = usage.get("last").cloned().unwrap_or_default();
                            let used = last.get("totalTokens").and_then(serde_json::Value::as_u64).unwrap_or_default();
                            let size = usage.get("modelContextWindow").and_then(serde_json::Value::as_u64).unwrap_or_default();
                            let output = last.get("outputTokens").and_then(serde_json::Value::as_u64);
                            transcript.usage = Some(TurnUsage { used, size, cost_usd: None });
                            transcript.output_tokens = output;
                            if let Some(output) = output {
                                self.observe(LiveSessionEvent::GeneratedOutputTokens(output));
                            }
                            self.observe(LiveSessionEvent::UsageUpdate { used, size, cost_usd: None });
                        }
                        "turn_completed" => {
                            let status = event_string(&event, "status").unwrap_or("unknown");
                            return match status {
                                "completed" => TurnEnd::Completed { transcript },
                                "interrupted" | "cancelled" => TurnEnd::Interrupted { transcript },
                                _ => TurnEnd::Failed {
                                    error: format!("Codex turn ended {status}: {}", event.get("error").unwrap_or(&serde_json::Value::Null)),
                                    transcript,
                                },
                            };
                        }
                        "runner_error" | "app_server_exited" | "runner_process_closed" => {
                            return TurnEnd::Failed {
                                error: format!("Codex runner failed: {}", event),
                                transcript,
                            };
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the Codex launch boundary keeps identity, responsibility, authority and observation explicit"
)]
pub(crate) async fn with_agent<F, T>(
    container: &str,
    auth: &AgentAuth,
    workdir: &str,
    actor: &str,
    responsibility: &str,
    system_prompt: &str,
    observer: Option<SessionObserver>,
    drive: F,
) -> Result<T>
where
    F: for<'a> FnOnce(
        &'a CodexSession,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<T>> + Send + 'a>,
    >,
{
    if responsibility.trim().is_empty() || system_prompt.trim().is_empty() {
        bail!("Codex session needs responsibility and developer instructions");
    }
    if auth.gateway_token_env != MODEL_CAPABILITY_ENV {
        bail!("Codex runner requires the scoped Restless model capability");
    }
    let locator_path = locator_path(&auth.company, actor, responsibility);
    let codex_home = home_path(&auth.company, actor, responsibility);
    let prior = read_locator(container, &locator_path).await?;
    if let Some(locator) = &prior {
        if locator.version != 1
            || locator.company != auth.company
            || locator.actor != actor
            || locator.responsibility != responsibility
        {
            bail!("refusing Codex locator outside its company/actor/responsibility scope");
        }
    }
    let reusable = prior.as_ref().is_some_and(|locator| {
        locator.cwd == workdir && locator.model == auth.model && locator.effort == auth.effort
    });
    let reconstructed = prior.is_some() && !reusable;
    let reconstruction_reason = reconstructed.then(|| {
        "saved Codex thread did not match the exact workspace/model/effort launch".to_string()
    });
    let prior_thread = reusable
        .then(|| prior.as_ref().map(|locator| locator.thread_id.clone()))
        .flatten();

    let launch_id = auth.session_id.clone();
    let session_marker = format!("/tmp/restless-agent-{launch_id}.sid");
    let session_runtime = format!("/company/run/agent-sessions/{launch_id}");
    let dirs = Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            container,
            "mkdir",
            "-p",
            &codex_home,
            &format!("{CODEX_ROOT}/sessions"),
            &format!("{session_runtime}/cache"),
            &format!("{session_runtime}/tmp"),
        ])
        .output()
        .await
        .context("prepare Codex session directories")?;
    if !dirs.status.success() {
        bail!("prepare Codex session directories failed");
    }

    let mut args = crate::acp::agent_exec_prefix(workdir);
    for value in [
        format!("RESTLESS_ACTOR={actor}"),
        format!(
            "RESTLESS_COORDINATOR={}",
            crate::acp::runtime_coordinator()?
        ),
        format!("CODEX_HOME={codex_home}"),
        format!("XDG_CACHE_HOME={session_runtime}/cache"),
        format!("TMPDIR={session_runtime}/tmp"),
    ] {
        args.push("-e".to_string());
        args.push(value);
    }
    args.push("-e".to_string());
    args.push(auth.coordination_token_env.clone());
    args.push("-e".to_string());
    args.push(auth.gateway_token_env.clone());
    args.extend([
        container.to_string(),
        "sh".to_string(),
        "-c".to_string(),
        "umask 007; printf '%s\\n' \"$$\" > \"$1\"; shift; exec \"$@\"".to_string(),
        "restless-codex".to_string(),
        session_marker.clone(),
        RUNNER.to_string(),
    ]);
    let mut child = Command::new("docker")
        .env(&auth.coordination_token_env, &auth.coordination_token)
        .env(&auth.gateway_token_env, &auth.gateway_token)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("spawn first-party Codex runner in company Runtime")?;
    let stdin = Arc::new(AsyncMutex::new(
        child.stdin.take().context("Codex runner stdin")?,
    ));
    let stdout = child.stdout.take().context("Codex runner stdout")?;
    let mut stderr = child.stderr.take().context("Codex runner stderr")?;
    let (events_tx, events_rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let value = serde_json::from_str(&line).unwrap_or_else(|error| {
                serde_json::json!({"type":"runner_error","message":format!("invalid runner JSON: {error}")})
            });
            if events_tx.send(value).is_err() {
                return;
            }
        }
        let _ = events_tx.send(serde_json::json!({"type":"runner_process_closed"}));
    });
    tokio::spawn(async move {
        let mut sink = Vec::new();
        let _ = stderr.read_to_end(&mut sink).await;
    });
    let events = AsyncMutex::new(events_rx);
    send_operation(
        &stdin,
        serde_json::json!({
            "op": "launch",
            "cwd": workdir,
            "model": auth.model,
            "effort": auth.effort,
            "provider_base_url": auth.gateway_url,
            "developer_instructions": system_prompt,
            "thread_id": prior_thread,
        }),
    )
    .await?;

    let mut receiver = events.lock().await;
    let ready = loop {
        let event = receiver
            .recv()
            .await
            .context("Codex runner closed before readiness")?;
        match event_type(&event) {
            "session_ready" => break event,
            "runner_error" | "app_server_exited" | "runner_process_closed" => {
                bail!("Codex runner failed before readiness: {event}")
            }
            _ => {}
        }
    };
    drop(receiver);
    let thread_id = event_string(&ready, "thread_id")
        .context("Codex readiness omitted thread id")?
        .to_string();
    let observed = ready.get("observed").cloned().unwrap_or_default();
    let runner_digest = event_string(&observed, "runner_digest")
        .context("Codex readiness omitted runner digest")?
        .to_string();
    let locator = SessionLocator {
        version: 1,
        company: auth.company.clone(),
        actor: actor.to_string(),
        responsibility: responsibility.to_string(),
        cwd: workdir.to_string(),
        model: auth.model.clone(),
        effort: auth.effort.clone(),
        thread_id: thread_id.clone(),
        runner_digest: runner_digest.clone(),
    };
    persist_locator(container, &locator_path, &locator).await?;
    let tool_contract_digest = prove_tool_contract(container, auth, actor).await?;
    let session = CodexSession {
        stdin: Arc::clone(&stdin),
        events,
        observer,
        live_observer_enabled: Arc::new(AtomicBool::new(true)),
        launch_id,
        thread_id,
        model: auth.model.clone(),
        effort: auth.effort.clone(),
        resumed: ready
            .get("resumed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        reconstructed,
        reconstruction_reason,
        runner_digest,
        tool_contract_digest,
        observed,
    };
    let result = drive(&session).await;
    let _ = send_operation(&stdin, serde_json::json!({"op":"shutdown"})).await;
    let owned_session = crate::acp::read_session_id(container, &session_marker).await;
    let _ = child.kill().await;
    if let Some(session_id) = owned_session {
        let _ = crate::acp::reap_session(container, &session_id).await;
    } else {
        tracing::warn!(container, marker = %session_marker, "Codex session marker missing; declining broad cleanup");
    }
    for path in [&session_marker, &session_runtime] {
        let _ = Command::new("docker")
            .args(["exec", "-u", "root", container, "rm", "-rf", path])
            .output()
            .await;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responsibility_scope_is_stable_and_distinct() {
        assert_eq!(
            scope_digest("acme", "game-builder", "work:one"),
            scope_digest("acme", "game-builder", "work:one")
        );
        assert_ne!(
            scope_digest("acme", "game-builder", "work:one"),
            scope_digest("acme", "game-builder", "work:two")
        );
    }

    #[test]
    fn event_projection_never_treats_missing_type_as_success() {
        assert_eq!(
            event_type(&serde_json::json!({"status":"completed"})),
            "unknown"
        );
        assert_eq!(
            event_type(&serde_json::json!({"type":"turn_completed"})),
            "turn_completed"
        );
    }
}
