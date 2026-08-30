//! Full-screen owner conversation over the loopback owner gateway.
//!
//! The terminal is deliberately a client of the same durable transcript and
//! reconnectable activity endpoints as the cockpit. It does not talk ACP or
//! the coordinator socket directly: the daemon remains the policy and
//! persistence boundary.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, IsTerminal};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
};
use crossterm::execute;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use reqwest::blocking::multipart::Form;
use reqwest::blocking::{Client as HttpClient, Response};
use serde::Deserialize;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
use url::Url;

const MAX_MESSAGE_CHARS: usize = 20_000;
const INPUT_VISIBLE_LINES: usize = 5;
const EVENT_POLL: Duration = Duration::from_millis(80);
const CTRL_C_EXIT_WINDOW: Duration = Duration::from_secs(2);
const SSE_RETRY_INITIAL: Duration = Duration::from_millis(350);
const SSE_RETRY_MAX: Duration = Duration::from_secs(4);

const OWNER: Color = Color::LightBlue;
const EXEC: Color = Color::LightCyan;
const MUTED: Color = Color::DarkGray;
const LIVE: Color = Color::Cyan;
const WARN: Color = Color::Yellow;
const ERROR: Color = Color::LightRed;

/// Start a full-screen conversation with one durable company actor.
pub fn run(company: String, actor: String) -> Result<()> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!("`restless chat` needs an interactive terminal");
    }

    let client = ChatClient::from_environment()?;
    let conversation = client
        .conversation(&company, &actor)
        .with_context(|| format!("open {actor} conversation for {company}"))?;
    let initial_message = latest_unread_owner_message(&conversation);

    let (events, receiver) = mpsc::channel();
    let shutdown = Arc::new(AtomicBool::new(false));
    let mut app = App::new(company.clone(), actor.clone(), conversation);
    if let Some(message_id) = initial_message {
        app.following_message_id = Some(message_id);
    }

    let mut session = TerminalSession::enter()?;
    if let Some(message_id) = initial_message {
        spawn_activity_stream(
            client.clone(),
            company,
            actor,
            message_id,
            events.clone(),
            Arc::clone(&shutdown),
        );
    }
    let result = run_loop(
        &mut session.terminal,
        &mut app,
        client,
        events,
        receiver,
        Arc::clone(&shutdown),
    );
    shutdown.store(true, Ordering::Relaxed);
    drop(session);
    result
}

struct TerminalSession {
    terminal: DefaultTerminal,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        let terminal = ratatui::try_init().context("enter full-screen terminal mode")?;
        if let Err(error) = execute!(std::io::stdout(), EnableMouseCapture, EnableBracketedPaste) {
            ratatui::restore();
            return Err(error).context("enable terminal mouse and paste support");
        }
        Ok(Self { terminal })
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = execute!(
            std::io::stdout(),
            DisableBracketedPaste,
            DisableMouseCapture
        );
        ratatui::restore();
    }
}

fn run_loop(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    client: ChatClient,
    events: Sender<UiEvent>,
    receiver: Receiver<UiEvent>,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    loop {
        while let Ok(event) = receiver.try_recv() {
            let action = app.apply(event);
            handle_app_action(action, app, &client, &events, &shutdown);
        }
        app.expire_shortcuts();
        terminal.draw(|frame| render(frame, app))?;

        if event::poll(EVENT_POLL)? {
            let event = event::read()?;
            let action = handle_terminal_event(app, event);
            if action == InputAction::Exit {
                return Ok(());
            }
            handle_input_action(action, app, &client, &events, &shutdown);
        }
    }
}

fn handle_app_action(
    action: AppAction,
    app: &mut App,
    client: &ChatClient,
    events: &Sender<UiEvent>,
    shutdown: &Arc<AtomicBool>,
) {
    match action {
        AppAction::None => {}
        AppAction::RefreshConversation => {
            spawn_conversation_refresh(
                client.clone(),
                app.company.clone(),
                app.actor.clone(),
                events.clone(),
            );
        }
        AppAction::Follow(message_id) => {
            spawn_activity_stream(
                client.clone(),
                app.company.clone(),
                app.actor.clone(),
                message_id,
                events.clone(),
                Arc::clone(shutdown),
            );
        }
    }
}

fn handle_terminal_event(app: &mut App, event: Event) -> InputAction {
    match event {
        Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
            handle_key(app, key)
        }
        Event::Paste(value) => {
            app.editor.insert(&value);
            InputAction::None
        }
        Event::Mouse(mouse) => match mouse.kind {
            MouseEventKind::ScrollUp => {
                app.scroll_up(5);
                InputAction::None
            }
            MouseEventKind::ScrollDown => {
                app.scroll_down(5);
                InputAction::None
            }
            _ => InputAction::None,
        },
        _ => InputAction::None,
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> InputAction {
    let modifiers = key.modifiers;
    if modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('C') => app.ctrl_c(),
            KeyCode::Char('u') | KeyCode::Char('U') => {
                app.editor.delete_to_line_start();
                InputAction::None
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                app.editor.delete_previous_word();
                InputAction::None
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                app.editor.home();
                InputAction::None
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                app.editor.end();
                InputAction::None
            }
            KeyCode::Char('l') | KeyCode::Char('L') => InputAction::Refresh,
            _ => InputAction::None,
        };
    }

    match key.code {
        KeyCode::Enter if modifiers.contains(KeyModifiers::SHIFT) => {
            app.editor.insert("\n");
            InputAction::None
        }
        KeyCode::Enter => app.submit(),
        KeyCode::Backspace => {
            app.editor.backspace();
            InputAction::None
        }
        KeyCode::Delete => {
            app.editor.delete();
            InputAction::None
        }
        KeyCode::Left => {
            app.editor.left();
            InputAction::None
        }
        KeyCode::Right => {
            app.editor.right();
            InputAction::None
        }
        KeyCode::Up => {
            app.editor.up();
            InputAction::None
        }
        KeyCode::Down => {
            app.editor.down();
            InputAction::None
        }
        KeyCode::Home => {
            app.editor.home();
            InputAction::None
        }
        KeyCode::End => {
            app.editor.end();
            InputAction::None
        }
        KeyCode::PageUp => {
            app.scroll_up(12);
            InputAction::None
        }
        KeyCode::PageDown => {
            app.scroll_down(12);
            InputAction::None
        }
        KeyCode::Tab => {
            app.editor.insert("    ");
            InputAction::None
        }
        KeyCode::Esc => {
            if app.editor.is_empty() {
                app.notice(
                    "Use Ctrl-C to leave chat.",
                    NoticeTone::Neutral,
                    Some(Duration::from_secs(2)),
                );
            } else {
                app.editor.clear();
                app.notice(
                    "Draft cleared.",
                    NoticeTone::Neutral,
                    Some(Duration::from_secs(2)),
                );
            }
            InputAction::None
        }
        KeyCode::Char(character)
            if !modifiers.contains(KeyModifiers::ALT)
                && !modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.editor.insert(&character.to_string());
            InputAction::None
        }
        _ => InputAction::None,
    }
}

fn handle_input_action(
    action: InputAction,
    app: &mut App,
    client: &ChatClient,
    events: &Sender<UiEvent>,
    shutdown: &Arc<AtomicBool>,
) {
    match action {
        InputAction::None | InputAction::Exit => {}
        InputAction::Refresh => {
            spawn_conversation_refresh(
                client.clone(),
                app.company.clone(),
                app.actor.clone(),
                events.clone(),
            );
            app.notice(
                "Refreshing durable conversation…",
                NoticeTone::Neutral,
                Some(Duration::from_secs(2)),
            );
        }
        InputAction::Send(body) => {
            spawn_message_send(
                client.clone(),
                app.company.clone(),
                app.actor.clone(),
                body,
                events.clone(),
            );
        }
        InputAction::Interrupt(message_id) => {
            spawn_interrupt(
                client.clone(),
                app.company.clone(),
                app.actor.clone(),
                message_id,
                events.clone(),
            );
            let _ = shutdown;
        }
    }
}

#[derive(Clone)]
struct ChatClient {
    http: HttpClient,
    base: Url,
}

impl ChatClient {
    fn from_environment() -> Result<Self> {
        let base = std::env::var("RESTLESS_OWNER_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:7788".to_string());
        let base = Url::parse(&base).context("parse RESTLESS_OWNER_URL")?;
        if !matches!(base.scheme(), "http" | "https") {
            bail!("RESTLESS_OWNER_URL must use http:// or https://");
        }
        let http = HttpClient::builder()
            .connect_timeout(Duration::from_secs(4))
            // A live SSE stream is expected to idle. A bounded request
            // lifetime lets the worker re-check shutdown and reconnect after
            // a broken local transport instead of pinning the UI to one
            // socket forever. The gateway emits a keepalive while it is open.
            .timeout(Duration::from_secs(35))
            .build()
            .context("create owner gateway client")?;
        Ok(Self { http, base })
    }

    fn conversation(&self, company: &str, actor: &str) -> Result<ConversationResponse> {
        let endpoint = self.endpoint(&["companies", company, "actors", actor, "conversation"])?;
        let response = self
            .http
            .get(endpoint)
            .send()
            .context("request conversation")?;
        success(response, "request conversation")?
            .json()
            .context("decode conversation response")
    }

    fn send_message(&self, company: &str, actor: &str, body: &str) -> Result<SendReceipt> {
        let endpoint = self.endpoint(&["companies", company, "actors", actor, "conversation"])?;
        let form = Form::new().text("body", body.to_string());
        let response = self
            .http
            .post(endpoint)
            .multipart(form)
            .send()
            .context("send conversation message")?;
        success(response, "send conversation message")?
            .json()
            .context("decode conversation send response")
    }

    fn interrupt(&self, company: &str, actor: &str, message_id: i64) -> Result<InterruptReceipt> {
        let endpoint = self.endpoint(&[
            "companies",
            company,
            "actors",
            actor,
            "conversation",
            &message_id.to_string(),
            "interrupt",
        ])?;
        let response = self
            .http
            .post(endpoint)
            .send()
            .context("interrupt conversation")?;
        success(response, "interrupt conversation")?
            .json()
            .context("decode conversation interruption response")
    }

    fn activity(&self, company: &str, actor: &str, message_id: i64) -> Result<Response> {
        let mut endpoint = self.endpoint(&["companies", company, "actors", actor, "activity"])?;
        endpoint
            .query_pairs_mut()
            .append_pair("message_id", &message_id.to_string());
        let response = self
            .http
            .get(endpoint)
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .send()
            .context("open live activity stream")?;
        success(response, "open live activity stream")
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        let mut endpoint = self.base.clone();
        endpoint.set_query(None);
        endpoint.set_fragment(None);
        let mut path = endpoint
            .path_segments_mut()
            .map_err(|_| anyhow!("RESTLESS_OWNER_URL cannot be used as a base URL"))?;
        path.pop_if_empty();
        // `RESTLESS_OWNER_URL` names the owner gateway root, not an API-base
        // URL. Keep this identical to the Cockpit's same-origin `/api/...`
        // calls and to the `doctor` probe.
        path.push("api");
        for segment in segments {
            path.push(segment);
        }
        drop(path);
        Ok(endpoint)
    }
}

fn success(response: Response, operation: &str) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().unwrap_or_default();
    let detail = serde_json::from_str::<GatewayError>(&body)
        .map(|error| format!("[{}] {}", error.error, error.message))
        .unwrap_or_else(|_| compact_server_error(&body));
    bail!("{operation} failed (HTTP {status}): {detail}")
}

fn compact_server_error(value: &str) -> String {
    let safe = terminal_text(value);
    let compact = safe.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        "owner gateway returned no error detail".into()
    } else {
        truncate_width(&compact, 240)
    }
}

#[derive(Debug, Deserialize)]
struct GatewayError {
    error: String,
    message: String,
}

// Keep the complete response shape here even where the calm terminal surface
// elects not to render a field. That makes this a real client of the shared
// owner contract rather than an untyped partial JSON reader.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ConversationResponse {
    actor: ConversationActor,
    focus: Option<ConversationFocus>,
    #[serde(default)]
    messages: Vec<ConversationMessage>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ConversationActor {
    id: String,
    display: String,
    kind: String,
    role: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ConversationFocus {
    after_message_id: i64,
    started_at: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct ConversationMessage {
    id: i64,
    from_actor: String,
    to_actor: Option<String>,
    body: String,
    #[serde(default)]
    attachments: Vec<Attachment>,
    #[serde(default)]
    details: Option<String>,
    #[serde(default)]
    intent: Option<IntentReceipt>,
    #[serde(default)]
    context_path: Option<String>,
    created_at: String,
    #[serde(default)]
    read_at: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Attachment {
    upload_id: String,
    name: String,
    media_type: String,
    size_bytes: usize,
    path: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct IntentReceipt {
    kind: String,
    summary: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct SendReceipt {
    message_id: i64,
    interrupted: bool,
    context_attached: bool,
    context_omitted: bool,
    focus: Option<ConversationFocus>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct InterruptReceipt {
    message_id: i64,
    cancelled: bool,
    interrupted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ActivityPhase {
    Queued,
    Thinking,
    Acting,
    Responding,
    Complete,
    Failed,
}

impl ActivityPhase {
    fn is_live(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Thinking | Self::Acting | Self::Responding
        )
    }

    fn label(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Thinking => "thinking",
            Self::Acting => "working",
            Self::Responding => "replying",
            Self::Complete => "finished",
            Self::Failed => "stopped",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityState {
    stream_id: String,
    sequence: u64,
    company: String,
    actor_id: String,
    trigger_message_id: Option<i64>,
    work_id: Option<String>,
    attempt_id: Option<String>,
    phase: ActivityPhase,
    reply: String,
    generated_output_tokens: Option<u64>,
    context_usage: Option<ContextUsage>,
    #[serde(default)]
    activity: Vec<ActivityItem>,
    started_at: Option<String>,
    updated_at: String,
    completed_message_id: Option<i64>,
    error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContextUsage {
    used: u64,
    size: u64,
    cost_usd: Option<f64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityItem {
    id: String,
    kind: String,
    label: String,
    detail: String,
    status: String,
    reply_offset: usize,
}

enum UiEvent {
    Conversation(std::result::Result<ConversationResponse, String>),
    Sent {
        body: String,
        result: std::result::Result<SendReceipt, String>,
    },
    Activity(ActivityState),
    Transport {
        message_id: i64,
        detail: String,
    },
    Interrupted(std::result::Result<InterruptReceipt, String>),
}

fn spawn_conversation_refresh(
    client: ChatClient,
    company: String,
    actor: String,
    sender: Sender<UiEvent>,
) {
    thread::spawn(move || {
        let result = client
            .conversation(&company, &actor)
            .map_err(|error| format!("{error:#}"));
        let _ = sender.send(UiEvent::Conversation(result));
    });
}

fn spawn_message_send(
    client: ChatClient,
    company: String,
    actor: String,
    body: String,
    sender: Sender<UiEvent>,
) {
    thread::spawn(move || {
        let result = client
            .send_message(&company, &actor, &body)
            .map_err(|error| format!("{error:#}"));
        let _ = sender.send(UiEvent::Sent { body, result });
    });
}

fn spawn_interrupt(
    client: ChatClient,
    company: String,
    actor: String,
    message_id: i64,
    sender: Sender<UiEvent>,
) {
    thread::spawn(move || {
        let result = client
            .interrupt(&company, &actor, message_id)
            .map_err(|error| format!("{error:#}"));
        let _ = sender.send(UiEvent::Interrupted(result));
    });
}

fn spawn_activity_stream(
    client: ChatClient,
    company: String,
    actor: String,
    message_id: i64,
    sender: Sender<UiEvent>,
    shutdown: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let mut retry_after = SSE_RETRY_INITIAL;
        while !shutdown.load(Ordering::Relaxed) {
            match client.activity(&company, &actor, message_id) {
                Ok(response) => {
                    retry_after = SSE_RETRY_INITIAL;
                    match read_sse(response, &sender, message_id, &shutdown) {
                        SseRead::Terminal => return,
                        SseRead::Closed => {
                            let _ = sender.send(UiEvent::Transport {
                                message_id,
                                detail: "Live connection closed; reconnecting…".into(),
                            });
                        }
                        SseRead::Error(error) => {
                            let _ = sender.send(UiEvent::Transport {
                                message_id,
                                detail: format!(
                                    "Live connection interrupted ({error}); reconnecting…"
                                ),
                            });
                        }
                    }
                }
                Err(error) => {
                    let _ = sender.send(UiEvent::Transport {
                        message_id,
                        detail: format!("Live connection unavailable ({error:#}); reconnecting…"),
                    });
                }
            }
            wait_for_retry(retry_after, &shutdown);
            retry_after = (retry_after * 2).min(SSE_RETRY_MAX);
        }
    });
}

enum SseRead {
    Terminal,
    Closed,
    Error(std::io::Error),
}

fn read_sse(
    response: Response,
    sender: &Sender<UiEvent>,
    message_id: i64,
    shutdown: &AtomicBool,
) -> SseRead {
    let mut reader = BufReader::new(response);
    let mut parser = SseParser::default();
    loop {
        if shutdown.load(Ordering::Relaxed) {
            return SseRead::Closed;
        }
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return SseRead::Closed,
            Ok(_) => match parser.push(&line) {
                Some(Ok(state)) => {
                    let terminal = !state.phase.is_live();
                    let _ = sender.send(UiEvent::Activity(state));
                    if terminal {
                        return SseRead::Terminal;
                    }
                }
                Some(Err(error)) => {
                    let _ = sender.send(UiEvent::Transport {
                        message_id,
                        detail: format!("Ignored malformed live update ({error})."),
                    });
                }
                None => {}
            },
            Err(error) => return SseRead::Error(error),
        }
    }
}

fn wait_for_retry(duration: Duration, shutdown: &AtomicBool) {
    let until = Instant::now() + duration;
    while !shutdown.load(Ordering::Relaxed) {
        let remaining = until.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return;
        }
        thread::sleep(remaining.min(Duration::from_millis(100)));
    }
}

#[derive(Default)]
struct SseParser {
    event: Option<String>,
    data: Vec<String>,
}

impl SseParser {
    fn push(&mut self, line: &str) -> Option<std::result::Result<ActivityState, String>> {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            let event = self.event.take();
            let data = std::mem::take(&mut self.data).join("\n");
            if event.as_deref() != Some("activity") || data.is_empty() {
                return None;
            }
            return Some(serde_json::from_str(&data).map_err(|error| error.to_string()));
        }
        if line.starts_with(':') {
            return None;
        }
        if let Some(value) = line.strip_prefix("event:") {
            self.event = Some(value.trim_start().to_string());
        } else if let Some(value) = line.strip_prefix("data:") {
            self.data.push(value.trim_start().to_string());
        }
        None
    }
}

#[derive(Clone)]
struct PendingMessage {
    id: Option<i64>,
    body: String,
}

struct App {
    company: String,
    actor: String,
    conversation: ConversationResponse,
    editor: Editor,
    pending: Option<PendingMessage>,
    live: Option<ActivityState>,
    following_message_id: Option<i64>,
    sending: bool,
    interrupting: bool,
    interrupted_message_ids: BTreeSet<i64>,
    ctrl_c_until: Option<Instant>,
    scroll_from_bottom: u16,
    notice: Option<Notice>,
}

impl App {
    fn new(company: String, actor: String, conversation: ConversationResponse) -> Self {
        Self {
            company,
            actor,
            conversation,
            editor: Editor::default(),
            pending: None,
            live: None,
            following_message_id: None,
            sending: false,
            interrupting: false,
            interrupted_message_ids: BTreeSet::new(),
            ctrl_c_until: None,
            scroll_from_bottom: 0,
            notice: None,
        }
    }

    fn actor_display(&self) -> String {
        if self.conversation.actor.id == "exec" {
            "Exec".into()
        } else {
            terminal_text(&self.conversation.actor.display)
        }
    }

    fn is_live_turn(&self) -> bool {
        self.following_message_id.is_some()
    }

    fn busy(&self) -> bool {
        self.sending || self.is_live_turn()
    }

    fn notice(
        &mut self,
        text: impl Into<String>,
        tone: NoticeTone,
        for_duration: Option<Duration>,
    ) {
        self.notice = Some(Notice {
            text: text.into(),
            tone,
            expires_at: for_duration.map(|duration| Instant::now() + duration),
        });
    }

    fn expire_shortcuts(&mut self) {
        if self
            .ctrl_c_until
            .is_some_and(|until| Instant::now() >= until)
        {
            self.ctrl_c_until = None;
        }
        if self
            .notice
            .as_ref()
            .and_then(|notice| notice.expires_at)
            .is_some_and(|until| Instant::now() >= until)
        {
            self.notice = None;
        }
    }

    fn submit(&mut self) -> InputAction {
        if self.busy() {
            self.notice(
                "Exec is still handling the current turn. Wait for it to settle before sending again.",
                NoticeTone::Warning,
                Some(Duration::from_secs(4)),
            );
            return InputAction::None;
        }
        let body = self.editor.text.trim().to_string();
        if body.is_empty() {
            return InputAction::None;
        }
        self.editor.clear();
        self.pending = Some(PendingMessage {
            id: None,
            body: body.clone(),
        });
        self.sending = true;
        self.notice("Sending to Exec…", NoticeTone::Neutral, None);
        InputAction::Send(body)
    }

    fn ctrl_c(&mut self) -> InputAction {
        if self.interrupting {
            return InputAction::Exit;
        }
        if let Some(until) = self.ctrl_c_until {
            if Instant::now() < until {
                return InputAction::Exit;
            }
        }
        if let Some(message_id) = self.following_message_id {
            self.ctrl_c_until = Some(Instant::now() + CTRL_C_EXIT_WINDOW);
            self.interrupting = true;
            self.notice(
                "Interrupting this turn · Ctrl-C again within 2s to leave chat.",
                NoticeTone::Warning,
                None,
            );
            return InputAction::Interrupt(message_id);
        }
        if self.sending {
            self.ctrl_c_until = Some(Instant::now() + CTRL_C_EXIT_WINDOW);
            self.notice(
                "The message is still being sent · Ctrl-C again within 2s to leave chat.",
                NoticeTone::Warning,
                None,
            );
            return InputAction::None;
        }
        InputAction::Exit
    }

    fn scroll_up(&mut self, amount: u16) {
        self.scroll_from_bottom = self.scroll_from_bottom.saturating_add(amount);
    }

    fn scroll_down(&mut self, amount: u16) {
        self.scroll_from_bottom = self.scroll_from_bottom.saturating_sub(amount);
    }

    fn apply(&mut self, event: UiEvent) -> AppAction {
        match event {
            UiEvent::Conversation(result) => match result {
                Ok(conversation) => {
                    let pending_id = self.pending.as_ref().and_then(|pending| pending.id);
                    self.conversation = conversation;
                    if pending_id.is_some_and(|id| {
                        self.conversation
                            .messages
                            .iter()
                            .any(|message| message.id == id)
                    }) {
                        self.pending = None;
                    }
                    if let Some(message_id) = self.following_message_id {
                        let response_recorded = self.conversation.messages.iter().any(|message| {
                            message.from_actor != "owner" && message.id > message_id
                        });
                        if response_recorded {
                            self.following_message_id = None;
                            self.live = None;
                            self.interrupting = false;
                            self.notice(
                                "Durable reply recorded.",
                                NoticeTone::Success,
                                Some(Duration::from_secs(2)),
                            );
                        }
                    }
                    AppAction::None
                }
                Err(error) => {
                    self.notice(
                        format!(
                            "Could not refresh durable conversation: {}",
                            terminal_text(&error)
                        ),
                        NoticeTone::Error,
                        Some(Duration::from_secs(5)),
                    );
                    AppAction::None
                }
            },
            UiEvent::Sent { body, result } => {
                self.sending = false;
                match result {
                    Ok(receipt) => {
                        let message_id = receipt.message_id;
                        self.pending = Some(PendingMessage {
                            id: Some(message_id),
                            body,
                        });
                        self.following_message_id = Some(message_id);
                        self.live = None;
                        self.notice(
                            "Exec has the message.",
                            NoticeTone::Success,
                            Some(Duration::from_secs(2)),
                        );
                        AppAction::Follow(message_id)
                    }
                    Err(error) => {
                        if self.editor.is_empty() {
                            self.editor.set(&body);
                        }
                        self.pending = None;
                        self.notice(
                            format!("Message was not sent: {}", terminal_text(&error)),
                            NoticeTone::Error,
                            Some(Duration::from_secs(6)),
                        );
                        AppAction::None
                    }
                }
            }
            UiEvent::Activity(state) => {
                let Some(message_id) = self.following_message_id else {
                    return AppAction::None;
                };
                if state.trigger_message_id != Some(message_id) {
                    return AppAction::None;
                }
                if self.live.as_ref().is_some_and(|current| {
                    current.stream_id == state.stream_id && current.sequence >= state.sequence
                }) {
                    return AppAction::None;
                }
                let terminal = !state.phase.is_live();
                let error = state.error.clone();
                self.live = Some(state);
                if terminal {
                    self.following_message_id = None;
                    self.interrupting = false;
                    if let Some(error) = error {
                        self.notice(
                            terminal_text(&error),
                            NoticeTone::Warning,
                            Some(Duration::from_secs(5)),
                        );
                    } else {
                        self.notice(
                            "Exec finished; syncing the durable reply…",
                            NoticeTone::Success,
                            Some(Duration::from_secs(4)),
                        );
                    }
                    AppAction::RefreshConversation
                } else {
                    AppAction::None
                }
            }
            UiEvent::Transport { message_id, detail } => {
                if self.following_message_id == Some(message_id) && !self.interrupting {
                    self.notice(
                        terminal_text(&detail),
                        NoticeTone::Warning,
                        Some(Duration::from_secs(3)),
                    );
                }
                AppAction::None
            }
            UiEvent::Interrupted(result) => {
                self.interrupting = false;
                match result {
                    Ok(receipt) => {
                        self.interrupted_message_ids.insert(receipt.message_id);
                        if self.following_message_id == Some(receipt.message_id) {
                            self.following_message_id = None;
                        }
                        self.live = None;
                        self.notice(
                            if receipt.interrupted {
                                "Turn interrupted."
                            } else {
                                "Queued turn cancelled before it started."
                            },
                            NoticeTone::Warning,
                            Some(Duration::from_secs(4)),
                        );
                        AppAction::RefreshConversation
                    }
                    Err(error) => {
                        self.notice(
                            format!("Could not interrupt this turn: {}", terminal_text(&error)),
                            NoticeTone::Error,
                            Some(Duration::from_secs(6)),
                        );
                        AppAction::None
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InputAction {
    None,
    Exit,
    Refresh,
    Send(String),
    Interrupt(i64),
}

enum AppAction {
    None,
    RefreshConversation,
    Follow(i64),
}

struct Notice {
    text: String,
    tone: NoticeTone,
    expires_at: Option<Instant>,
}

#[derive(Clone, Copy)]
enum NoticeTone {
    Neutral,
    Success,
    Warning,
    Error,
}

#[derive(Default)]
struct Editor {
    text: String,
    cursor: usize,
}

impl Editor {
    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    fn set(&mut self, value: &str) {
        self.text = editor_text(value, MAX_MESSAGE_CHARS);
        self.cursor = self.text.len();
    }

    fn insert(&mut self, value: &str) {
        let remaining = MAX_MESSAGE_CHARS.saturating_sub(self.text.chars().count());
        if remaining == 0 {
            return;
        }
        let value = editor_text(value, remaining);
        self.text.insert_str(self.cursor, &value);
        self.cursor += value.len();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let previous = previous_grapheme_boundary(&self.text, self.cursor);
        self.text.replace_range(previous..self.cursor, "");
        self.cursor = previous;
    }

    fn delete(&mut self) {
        if self.cursor >= self.text.len() {
            return;
        }
        let next = next_grapheme_boundary(&self.text, self.cursor);
        self.text.replace_range(self.cursor..next, "");
    }

    fn left(&mut self) {
        self.cursor = previous_grapheme_boundary(&self.text, self.cursor);
    }

    fn right(&mut self) {
        self.cursor = next_grapheme_boundary(&self.text, self.cursor);
    }

    fn home(&mut self) {
        self.cursor = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
    }

    fn end(&mut self) {
        self.cursor = self.text[self.cursor..]
            .find('\n')
            .map_or(self.text.len(), |offset| self.cursor + offset);
    }

    fn up(&mut self) {
        let line_start = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if line_start == 0 {
            return;
        }
        let target_end = line_start - 1;
        let target_start = self.text[..target_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let column = display_width(&self.text[line_start..self.cursor]);
        self.cursor = cursor_at_column(&self.text, target_start, target_end, column);
    }

    fn down(&mut self) {
        let line_start = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let line_end = self.text[self.cursor..]
            .find('\n')
            .map_or(self.text.len(), |offset| self.cursor + offset);
        if line_end == self.text.len() {
            return;
        }
        let target_start = line_end + 1;
        let target_end = self.text[target_start..]
            .find('\n')
            .map_or(self.text.len(), |offset| target_start + offset);
        let column = display_width(&self.text[line_start..self.cursor]);
        self.cursor = cursor_at_column(&self.text, target_start, target_end, column);
    }

    fn delete_to_line_start(&mut self) {
        let start = self.text[..self.cursor]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        self.text.replace_range(start..self.cursor, "");
        self.cursor = start;
    }

    fn delete_previous_word(&mut self) {
        let mut start = self.cursor;
        while start > 0 && grapheme_before(&self.text, start).is_some_and(is_space) {
            start = previous_grapheme_boundary(&self.text, start);
        }
        while start > 0 && grapheme_before(&self.text, start).is_some_and(|value| !is_space(value))
        {
            start = previous_grapheme_boundary(&self.text, start);
        }
        self.text.replace_range(start..self.cursor, "");
        self.cursor = start;
    }
}

fn editor_text(value: &str, max_chars: usize) -> String {
    let mut result = String::new();
    let mut count = 0;
    for character in value.replace("\r\n", "\n").replace('\r', "\n").chars() {
        let emitted = match character {
            '\n' => Some("\n"),
            '\t' => Some("    "),
            character if character.is_control() => None,
            character if is_bidi_control(character) => None,
            character => {
                if count >= max_chars {
                    break;
                }
                result.push(character);
                count += 1;
                continue;
            }
        };
        if let Some(emitted) = emitted {
            for character in emitted.chars() {
                if count >= max_chars {
                    return result;
                }
                result.push(character);
                count += 1;
            }
        }
    }
    result
}

fn previous_grapheme_boundary(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .grapheme_indices(true)
        .next_back()
        .map_or(0, |(index, _)| index)
}

fn next_grapheme_boundary(value: &str, cursor: usize) -> usize {
    value[cursor..]
        .graphemes(true)
        .next()
        .map_or(cursor, |grapheme| cursor + grapheme.len())
}

fn grapheme_before(value: &str, cursor: usize) -> Option<&str> {
    let start = previous_grapheme_boundary(value, cursor);
    (start < cursor).then_some(&value[start..cursor])
}

fn is_space(value: &str) -> bool {
    value.chars().all(char::is_whitespace)
}

fn cursor_at_column(value: &str, start: usize, end: usize, target: usize) -> usize {
    let mut cursor = start;
    let mut column = 0;
    for (offset, grapheme) in value[start..end].grapheme_indices(true) {
        let width = grapheme_width(grapheme);
        if column + width > target {
            break;
        }
        cursor = start + offset + grapheme.len();
        column += width;
    }
    cursor
}

fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    if area.width < 32 || area.height < 9 {
        render_too_small(frame, area);
        return;
    }

    let composer_width = area.width.saturating_sub(4).max(1) as usize;
    let composer = ComposerView::new(&app.editor, composer_width);
    let composer_height = (composer.lines.len().min(INPUT_VISIBLE_LINES) as u16 + 2).clamp(3, 7);
    let status_height = u16::from(app.status_text().is_some());
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(status_height),
            Constraint::Length(composer_height),
            Constraint::Length(1),
        ])
        .split(area);
    render_header(frame, vertical[0], app);
    render_transcript(frame, vertical[1], app);
    if status_height > 0 {
        render_status(frame, vertical[2], app);
    }
    render_composer(frame, vertical[3], app, composer);
    render_help(frame, vertical[4], app);
}

fn render_too_small(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    let text = Text::from(vec![
        Line::from(Span::styled(
            "restless chat",
            Style::default().fg(OWNER).add_modifier(Modifier::BOLD),
        )),
        Line::from("Terminal is too small. Resize to at least 32 × 9."),
        Line::from(Span::styled("Ctrl-C exits.", Style::default().fg(MUTED))),
    ]);
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL)),
        area,
    );
    frame.set_cursor_position((area.x, area.y));
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = format!(
        " restless chat  /  {}  /  {} ",
        app.company,
        app.actor_display()
    );
    let role = terminal_text(&app.conversation.actor.role);
    let right = if role.is_empty() {
        String::new()
    } else {
        format!(" {role} ")
    };
    let available = area.width as usize;
    let title = truncate_width(&title, available.saturating_sub(display_width(&right)));
    let gap = available.saturating_sub(display_width(&title) + display_width(&right));
    let line = Line::from(vec![
        Span::styled(
            title,
            Style::default().fg(OWNER).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, Style::default().fg(MUTED)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
    if area.height > 1 {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(area.width as usize),
                Style::default().fg(MUTED),
            ))),
            Rect {
                y: area.y + 1,
                height: 1,
                ..area
            },
        );
    }
}

fn render_transcript(frame: &mut Frame, area: Rect, app: &mut App) {
    let lines = transcript_lines(app, area.width.saturating_sub(2).max(1) as usize);
    let total_lines = lines.len() as u16;
    let max_scroll = total_lines.saturating_sub(area.height);
    app.scroll_from_bottom = app.scroll_from_bottom.min(max_scroll);
    let scroll = max_scroll.saturating_sub(app.scroll_from_bottom);
    let paragraph = Paragraph::new(Text::from(lines)).scroll((scroll, 0));
    frame.render_widget(paragraph, area);
}

fn transcript_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for message in &app.conversation.messages {
        let author = if message.from_actor == "owner" {
            "You".to_string()
        } else {
            app.actor_display()
        };
        let style = if message.from_actor == "owner" {
            Style::default().fg(OWNER)
        } else {
            Style::default().fg(EXEC)
        };
        append_message(
            &mut lines,
            TranscriptMessage {
                author: &author,
                timestamp: &message.created_at,
                body: &message.body,
                attachments: &message.attachments,
                author_style: style,
                interrupted: app.interrupted_message_ids.contains(&message.id),
            },
            width,
        );
    }

    if let Some(pending) = &app.pending {
        let state = if pending.id.is_some() {
            "sending"
        } else {
            "waiting for gateway"
        };
        append_message(
            &mut lines,
            TranscriptMessage {
                author: "You",
                timestamp: state,
                body: &pending.body,
                attachments: &[],
                author_style: Style::default().fg(OWNER),
                interrupted: false,
            },
            width,
        );
    }

    if let Some(live) = &app.live {
        if live.phase.is_live() && !live.reply.trim().is_empty() {
            append_message(
                &mut lines,
                TranscriptMessage {
                    author: &app.actor_display(),
                    timestamp: "writing",
                    body: &live.reply,
                    attachments: &[],
                    author_style: Style::default().fg(EXEC),
                    interrupted: false,
                },
                width,
            );
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No conversation yet. Write a direction below to start.",
            Style::default().fg(MUTED),
        )));
    }
    lines
}

struct TranscriptMessage<'a> {
    author: &'a str,
    timestamp: &'a str,
    body: &'a str,
    attachments: &'a [Attachment],
    author_style: Style,
    interrupted: bool,
}

fn append_message(lines: &mut Vec<Line<'static>>, message: TranscriptMessage<'_>, width: usize) {
    let label = if message.interrupted {
        format!(
            "{}  {}  interrupted",
            terminal_text(message.author),
            compact_timestamp(message.timestamp)
        )
    } else {
        format!(
            "{}  {}",
            terminal_text(message.author),
            compact_timestamp(message.timestamp)
        )
    };
    lines.push(Line::from(vec![
        Span::styled("▎ ", message.author_style.add_modifier(Modifier::BOLD)),
        Span::styled(label, message.author_style.add_modifier(Modifier::BOLD)),
    ]));
    let body_style = Style::default();
    for line in soft_wrap(&terminal_text(message.body), width.saturating_sub(3).max(1)) {
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(line, body_style),
        ]));
    }
    for attachment in message.attachments {
        let label = format!(
            "   ↳ {} · {}",
            terminal_text(&attachment.name),
            format_size(attachment.size_bytes)
        );
        lines.push(Line::from(Span::styled(label, Style::default().fg(MUTED))));
    }
    lines.push(Line::from(""));
}

fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let Some((text, style)) = app.status_text() else {
        return;
    };
    let text = truncate_width(&terminal_text(&text), area.width.saturating_sub(2) as usize);
    let line = Line::from(vec![
        Span::styled("▎ ", style.add_modifier(Modifier::BOLD)),
        Span::styled(text, style),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_composer(frame: &mut Frame, area: Rect, app: &App, composer: ComposerView) {
    let border_style = if app.busy() {
        Style::default().fg(MUTED)
    } else {
        Style::default().fg(OWNER)
    };
    let title = if app.busy() {
        " Message · waiting "
    } else {
        " Message "
    };
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(border_style)
        .title(Span::styled(
            title,
            border_style.add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    let visible = composer.visible_lines();
    let text = Text::from(
        visible
            .iter()
            .map(|line| Line::from(Span::raw(line.text.clone())))
            .collect::<Vec<_>>(),
    );
    frame.render_widget(Paragraph::new(text).block(block), area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    let (line, column) = composer.cursor_visible_position();
    let x = inner.x.saturating_add(
        column
            .min(inner.width.saturating_sub(1) as usize)
            .try_into()
            .unwrap_or(u16::MAX),
    );
    let y = inner
        .y
        .saturating_add(line.min(inner.height.saturating_sub(1) as usize) as u16);
    frame.set_cursor_position((x, y));
}

fn render_help(frame: &mut Frame, area: Rect, app: &App) {
    let ctrl_c = if app.is_live_turn() || app.sending {
        "Ctrl-C interrupt · again exit"
    } else {
        "Ctrl-C exit"
    };
    let help = format!("Enter send  ·  Shift+Enter newline  ·  {ctrl_c}  ·  PgUp/PgDn scroll");
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            truncate_width(&help, area.width as usize),
            Style::default().fg(MUTED),
        ))),
        area,
    );
}

impl App {
    fn status_text(&self) -> Option<(String, Style)> {
        if let Some(notice) = &self.notice {
            return Some((notice.text.clone(), notice_style(notice.tone)));
        }
        if self.interrupting {
            return Some((
                "Interrupting this turn · Ctrl-C again within 2s to leave chat.".into(),
                Style::default().fg(WARN),
            ));
        }
        if self.sending {
            return Some(("Sending owner message…".into(), Style::default().fg(LIVE)));
        }
        if self.is_live_turn() {
            let status = self.live.as_ref().map_or_else(
                || "Exec is queued…".into(),
                |state| live_status(state, &self.actor_display()),
            );
            return Some((status, Style::default().fg(LIVE)));
        }
        None
    }
}

fn notice_style(tone: NoticeTone) -> Style {
    match tone {
        NoticeTone::Neutral => Style::default().fg(MUTED),
        NoticeTone::Success => Style::default().fg(LIVE),
        NoticeTone::Warning => Style::default().fg(WARN),
        NoticeTone::Error => Style::default().fg(ERROR),
    }
}

fn live_status(state: &ActivityState, actor: &str) -> String {
    let base = format!("{actor} is {}", state.phase.label());
    let activity = state
        .activity
        .iter()
        .rev()
        .find(|activity| activity.status == "active")
        .or_else(|| state.activity.last());
    match activity {
        Some(activity) if !activity.label.trim().is_empty() => {
            format!("{base} · {}", terminal_text(&activity.label))
        }
        _ => base,
    }
}

fn latest_unread_owner_message(conversation: &ConversationResponse) -> Option<i64> {
    conversation
        .messages
        .iter()
        .rev()
        .find(|message| message.from_actor == "owner" && message.read_at.is_none())
        .map(|message| message.id)
}

fn soft_wrap(value: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    for logical_line in value.split('\n') {
        if logical_line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut used = 0;
        for grapheme in logical_line.graphemes(true) {
            let grapheme_width = grapheme_width(grapheme).max(1);
            if used > 0 && used + grapheme_width > width {
                lines.push(line);
                line = String::new();
                used = 0;
            }
            line.push_str(grapheme);
            used += grapheme_width;
        }
        lines.push(line);
    }
    lines
}

struct ComposerLine {
    start: usize,
    end: usize,
    text: String,
}

struct ComposerView {
    lines: Vec<ComposerLine>,
    cursor_line: usize,
    cursor_column: usize,
}

impl ComposerView {
    fn new(editor: &Editor, width: usize) -> Self {
        let width = width.max(1);
        let mut lines = Vec::new();
        let mut start = 0;
        let mut used = 0;
        for (index, grapheme) in editor.text.grapheme_indices(true) {
            if grapheme == "\n" {
                lines.push(ComposerLine {
                    start,
                    end: index,
                    text: editor.text[start..index].to_string(),
                });
                start = index + grapheme.len();
                used = 0;
                continue;
            }
            let grapheme_width = grapheme_width(grapheme).max(1);
            if used > 0 && used + grapheme_width > width {
                lines.push(ComposerLine {
                    start,
                    end: index,
                    text: editor.text[start..index].to_string(),
                });
                start = index;
                used = 0;
            }
            used += grapheme_width;
        }
        lines.push(ComposerLine {
            start,
            end: editor.text.len(),
            text: editor.text[start..].to_string(),
        });

        let mut cursor_line = lines.len().saturating_sub(1);
        let mut cursor_column = 0;
        for (index, line) in lines.iter().enumerate() {
            let cursor_is_inside = editor.cursor >= line.start && editor.cursor <= line.end;
            let starts_wrapped_line = index > 0 && editor.cursor == line.start;
            let ends_before_wrapped_line = lines
                .get(index + 1)
                .is_some_and(|next| editor.cursor == next.start);
            if cursor_is_inside && !starts_wrapped_line && !ends_before_wrapped_line {
                cursor_line = index;
                cursor_column = display_width(&editor.text[line.start..editor.cursor]);
                break;
            }
            if starts_wrapped_line {
                cursor_line = index;
                cursor_column = 0;
                break;
            }
        }
        Self {
            lines,
            cursor_line,
            cursor_column,
        }
    }

    fn visible_start(&self) -> usize {
        self.cursor_line
            .saturating_add(1)
            .saturating_sub(INPUT_VISIBLE_LINES)
    }

    fn visible_lines(&self) -> &[ComposerLine] {
        let start = self.visible_start();
        let end = (start + INPUT_VISIBLE_LINES).min(self.lines.len());
        &self.lines[start..end]
    }

    fn cursor_visible_position(&self) -> (usize, usize) {
        (
            self.cursor_line.saturating_sub(self.visible_start()),
            self.cursor_column,
        )
    }
}

fn terminal_text(value: &str) -> String {
    value
        .chars()
        .filter_map(|character| match character {
            '\n' => Some('\n'),
            '\t' => Some(' '),
            character if character.is_control() || is_bidi_control(character) => None,
            character => Some(character),
        })
        .collect()
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

fn grapheme_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

fn truncate_width(value: &str, width: usize) -> String {
    if display_width(value) <= width {
        return value.to_string();
    }
    if width <= 1 {
        return "…".chars().take(width).collect();
    }
    let mut result = String::new();
    let mut used = 0;
    for grapheme in value.graphemes(true) {
        let grapheme_width = grapheme_width(grapheme);
        if used + grapheme_width + 1 > width {
            break;
        }
        result.push_str(grapheme);
        used += grapheme_width;
    }
    result.push('…');
    result
}

fn compact_timestamp(value: &str) -> String {
    let value = terminal_text(value);
    if value.len() >= 16 && value.as_bytes().get(10) == Some(&b'T') {
        return value[11..16].to_string();
    }
    if value == "writing" || value == "sending" || value == "waiting for gateway" {
        return value;
    }
    truncate_width(&value, 16)
}

fn format_size(bytes: usize) -> String {
    const KIB: usize = 1024;
    const MIB: usize = KIB * KIB;
    if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{} KiB", bytes.div_ceil(KIB))
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn fixture() -> ConversationResponse {
        ConversationResponse {
            actor: ConversationActor {
                id: "exec".into(),
                display: "Executive".into(),
                kind: "exec".into(),
                role: "Company executive".into(),
            },
            focus: None,
            messages: vec![ConversationMessage {
                id: 41,
                from_actor: "owner".into(),
                to_actor: Some("exec".into()),
                body: "Please check the launch plan.".into(),
                attachments: Vec::new(),
                details: None,
                intent: None,
                context_path: None,
                created_at: "2026-08-28T12:34:56Z".into(),
                read_at: None,
            }],
        }
    }

    fn live(message_id: i64) -> ActivityState {
        ActivityState {
            stream_id: "stream-1".into(),
            sequence: 3,
            company: "demo_test".into(),
            actor_id: "exec".into(),
            trigger_message_id: Some(message_id),
            work_id: None,
            attempt_id: None,
            phase: ActivityPhase::Responding,
            reply: "I found the remaining risk and am preparing the next step.".into(),
            generated_output_tokens: None,
            context_usage: None,
            activity: vec![ActivityItem {
                id: "tool-1".into(),
                kind: "tool".into(),
                label: "Review launch checklist".into(),
                detail: "shell".into(),
                status: "active".into(),
                reply_offset: 0,
            }],
            started_at: Some("2026-08-28T12:35:00Z".into()),
            updated_at: "2026-08-28T12:35:01Z".into(),
            completed_message_id: None,
            error: None,
        }
    }

    #[test]
    fn first_ctrl_c_interrupts_and_second_leaves() {
        let mut app = App::new("demo_test".into(), "exec".into(), fixture());
        app.following_message_id = Some(41);
        assert_eq!(app.ctrl_c(), InputAction::Interrupt(41));
        assert_eq!(app.ctrl_c(), InputAction::Exit);
    }

    #[test]
    fn ctrl_c_leaves_an_idle_chat_immediately() {
        let mut app = App::new("demo_test".into(), "exec".into(), fixture());
        assert_eq!(app.ctrl_c(), InputAction::Exit);
    }

    #[test]
    fn sse_parser_accepts_complete_activity_snapshots() {
        let mut parser = SseParser::default();
        assert!(parser.push("event: activity\n").is_none());
        assert!(parser
            .push("data: {\"streamId\":\"one\",\"sequence\":1,\"company\":\"demo\",\"actorId\":\"exec\",\"triggerMessageId\":41,\"workId\":null,\"attemptId\":null,\"phase\":\"thinking\",\"reply\":\"\",\"generatedOutputTokens\":null,\"contextUsage\":null,\"activity\":[],\"startedAt\":null,\"updatedAt\":\"now\",\"completedMessageId\":null,\"error\":null}\n")
            .is_none());
        let state = parser.push("\n").expect("one event").expect("valid state");
        assert_eq!(state.phase, ActivityPhase::Thinking);
        assert_eq!(state.trigger_message_id, Some(41));
    }

    #[test]
    fn chat_uses_the_cockpits_api_gateway_path() {
        let client = ChatClient {
            http: HttpClient::builder().build().expect("test client"),
            base: Url::parse("http://127.0.0.1:7788").expect("gateway URL"),
        };
        let endpoint = client
            .endpoint(&["companies", "demo test", "actors", "exec", "conversation"])
            .expect("conversation endpoint");
        assert_eq!(
            endpoint.as_str(),
            "http://127.0.0.1:7788/api/companies/demo%20test/actors/exec/conversation"
        );
    }

    #[test]
    fn terminal_text_removes_escape_and_bidi_controls() {
        assert_eq!(terminal_text("safe\u{1b}[2J\u{202e}text"), "safe[2Jtext");
    }

    #[test]
    fn editor_treats_emoji_as_one_editing_unit_and_places_wrapped_cursor_on_next_line() {
        let mut editor = Editor::default();
        editor.set("a👩‍🚀");
        editor.backspace();
        assert_eq!(editor.text, "a");

        editor.set("abcd");
        editor.cursor = 2;
        let composer = ComposerView::new(&editor, 2);
        assert_eq!(composer.cursor_visible_position(), (1, 0));
    }

    #[test]
    fn full_screen_layout_keeps_composer_and_live_turn_visible() {
        let mut app = App::new("demo_test".into(), "exec".into(), fixture());
        app.following_message_id = Some(41);
        app.live = Some(live(41));
        app.editor.set("Give me the short version");
        let backend = TestBackend::new(84, 24);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render");
        let buffer = terminal.backend().buffer();
        let text = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("restless chat"));
        assert!(text.contains("Exec is replying"));
        assert!(text.contains("Give me the short version"));
        assert!(buffer_line(buffer, 0).contains("restless chat"));
        assert!(buffer_line(buffer, 19).contains("Exec is replying"));
        assert!(buffer_line(buffer, 20).contains("Message"));
        assert!(buffer_line(buffer, 23).contains("Enter send"));
        assert!(terminal.backend().cursor_visible());
    }

    #[test]
    fn compact_layout_does_not_panic() {
        let mut app = App::new("demo_test".into(), "exec".into(), fixture());
        let backend = TestBackend::new(32, 9);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render");
    }

    fn buffer_line(buffer: &ratatui::buffer::Buffer, row: usize) -> String {
        let width = buffer.area().width as usize;
        buffer.content()[row * width..(row + 1) * width]
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }
}
