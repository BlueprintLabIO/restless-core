//! Ephemeral, reconnectable activity for any supervised agent turn.
//!
//! OrgIntel remains the writer of completed messages, Work and Attempts. This
//! module has one narrower job: project the current ACP turn to the owner so
//! a conversation and a claimed Work attempt do not need two competing live
//! transports. It is intentionally bounded and may disappear on daemon
//! restart without changing durable company truth.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::watch;
use uuid::Uuid;

use crate::acp::{LiveSessionEvent, SessionObserver};

const MAX_ACTIVITY_ITEMS: usize = 64;
const MAX_ACTIVITY_LABEL_CHARS: usize = 500;
const MAX_ACTIVITY_DETAIL_CHARS: usize = 160;
const MAX_ACTIVITY_STATUS_CHARS: usize = 80;
const MAX_REPLY_CHARS: usize = 40_000;
/// Snapshots deliberately replace fragile client-side token delta assembly.
/// Keep them at a paint-friendly rate so a fast provider does not turn one
/// response into hundreds of Svelte renders and megabytes of repeated text.
const MIN_INCREMENTAL_EMIT_INTERVAL: Duration = Duration::from_millis(75);
const INTENT_MARKER: &str = "<!--restless-intent:";
const DETAILS_MARKER: &str = "<!--restless-details:";
const HIDDEN_MARKERS: [&str; 2] = [DETAILS_MARKER, INTENT_MARKER];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ActivityScope {
    Message(i64),
    Work(Uuid),
}

type StreamKey = (String, String, ActivityScope);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActivityPhase {
    Queued,
    Thinking,
    Acting,
    Responding,
    Complete,
    Failed,
}

impl AgentActivityPhase {
    fn is_live(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Thinking | Self::Acting | Self::Responding
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentActivityItem {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub detail: String,
    pub status: String,
    /// Unicode-scalar offset into `reply` at which this activity began. The
    /// browser uses it to keep visible response/tool chronology intact.
    pub reply_offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentContextUsage {
    pub used: u64,
    pub size: u64,
    pub cost_usd: Option<f64>,
}

/// A complete snapshot sent on each SSE event. Snapshot delivery lets an
/// EventSource reconnect without rebuilding text from a potentially dropped
/// delta.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentActivityState {
    pub stream_id: Uuid,
    pub sequence: u64,
    pub company: String,
    pub actor_id: String,
    pub trigger_message_id: Option<i64>,
    pub work_id: Option<Uuid>,
    pub attempt_id: Option<Uuid>,
    pub phase: AgentActivityPhase,
    pub reply: String,
    /// Final generated-output total only when ACP reports it. Context usage
    /// below is a separate, live session snapshot.
    pub generated_output_tokens: Option<u64>,
    pub context_usage: Option<AgentContextUsage>,
    pub activity: Vec<AgentActivityItem>,
    pub started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub completed_message_id: Option<i64>,
    pub error: Option<String>,
    #[serde(skip)]
    reply_message_id: Option<String>,
    #[serde(skip)]
    reply_pending: String,
    #[serde(skip)]
    suppress_reply_tail: bool,
    #[serde(skip)]
    last_incremental_emit: Instant,
}

impl AgentActivityState {
    fn queued(
        company: &str,
        actor: &str,
        trigger_message_id: Option<i64>,
        work_id: Option<Uuid>,
        attempt_id: Option<Uuid>,
    ) -> Self {
        Self {
            stream_id: Uuid::new_v4(),
            sequence: 1,
            company: company.to_string(),
            actor_id: actor.to_string(),
            trigger_message_id,
            work_id,
            attempt_id,
            phase: AgentActivityPhase::Queued,
            reply: String::new(),
            generated_output_tokens: None,
            context_usage: None,
            activity: Vec::new(),
            started_at: None,
            updated_at: Utc::now(),
            completed_message_id: None,
            error: None,
            reply_message_id: None,
            reply_pending: String::new(),
            suppress_reply_tail: false,
            last_incremental_emit: Instant::now(),
        }
    }

    fn absent(
        company: &str,
        actor: &str,
        trigger_message_id: Option<i64>,
        work_id: Option<Uuid>,
    ) -> Self {
        let mut state = Self::queued(company, actor, trigger_message_id, work_id, None);
        state.fail("No live agent session is available to reconnect to.");
        state
    }

    fn touch(&mut self) {
        self.sequence = self.sequence.saturating_add(1);
        self.updated_at = Utc::now();
    }

    fn publish_incremental(&mut self) -> bool {
        if self.last_incremental_emit.elapsed() < MIN_INCREMENTAL_EMIT_INTERVAL {
            return false;
        }
        self.last_incremental_emit = Instant::now();
        self.touch();
        true
    }

    fn publish_now(&mut self) {
        self.last_incremental_emit = Instant::now();
        self.touch();
    }

    fn begin(&mut self) {
        self.phase = AgentActivityPhase::Thinking;
        self.reply.clear();
        self.generated_output_tokens = None;
        self.context_usage = None;
        self.activity.clear();
        self.started_at = Some(Utc::now());
        self.completed_message_id = None;
        self.error = None;
        self.reply_message_id = None;
        self.reply_pending.clear();
        self.suppress_reply_tail = false;
        self.publish_now();
    }

    /// Apply one ACP notification and return whether it merits an SSE
    /// snapshot. State can advance between snapshots, but every emitted event
    /// is complete and the final state is always forced out on completion.
    fn apply(&mut self, event: &LiveSessionEvent) -> bool {
        let mut changed = false;
        let mut incremental = false;
        match event {
            LiveSessionEvent::ReplyDelta { message_id, text } => {
                let starts_new_message = self.reply_message_id.is_some()
                    && message_id.is_some()
                    && self.reply_message_id != *message_id;
                if starts_new_message {
                    self.begin_reply_message();
                }
                if message_id.is_some() {
                    self.reply_message_id = message_id.clone();
                }
                let phase_changed = self.phase != AgentActivityPhase::Responding;
                changed |= phase_changed;
                self.phase = AgentActivityPhase::Responding;
                let visible_before = self.reply.len();
                if !self.suppress_reply_tail {
                    self.append_reply_delta(text);
                }
                changed |= self.reply.len() != visible_before;
                incremental = !phase_changed && !starts_new_message;
            }
            LiveSessionEvent::ThoughtDelta => {
                changed = self.phase != AgentActivityPhase::Thinking;
                self.phase = AgentActivityPhase::Thinking;
                // Thought text is liveness, not owner-visible content.
            }
            LiveSessionEvent::ToolStarted { id, title, kind } => {
                changed = true;
                self.phase = AgentActivityPhase::Acting;
                self.activity.push(AgentActivityItem {
                    id: format!("tool-{id}"),
                    kind: "tool".into(),
                    label: owner_visible_activity_text(title, MAX_ACTIVITY_LABEL_CHARS),
                    detail: owner_visible_activity_text(kind, MAX_ACTIVITY_DETAIL_CHARS),
                    status: "active".into(),
                    reply_offset: self.reply.chars().count(),
                });
            }
            LiveSessionEvent::ToolUpdated { id, title, status } => {
                changed = self.phase != AgentActivityPhase::Acting;
                self.phase = AgentActivityPhase::Acting;
                let activity_id = format!("tool-{id}");
                if let Some(existing) = self
                    .activity
                    .iter_mut()
                    .rev()
                    .find(|activity| activity.id == activity_id)
                {
                    if let Some(title) = title {
                        let label = owner_visible_activity_text(title, MAX_ACTIVITY_LABEL_CHARS);
                        changed |= existing.label != label;
                        existing.label = label;
                    }
                    let visible_status =
                        owner_visible_activity_text(status, MAX_ACTIVITY_STATUS_CHARS);
                    changed |= existing.status != visible_status;
                    existing.status = visible_status;
                }
            }
            LiveSessionEvent::GeneratedOutputTokens(tokens) => {
                changed = self.generated_output_tokens != Some(*tokens);
                self.generated_output_tokens = Some(*tokens);
            }
            LiveSessionEvent::UsageUpdate {
                used,
                size,
                cost_usd,
            } => {
                let next = AgentContextUsage {
                    used: *used,
                    size: *size,
                    cost_usd: *cost_usd,
                };
                changed = self.context_usage != Some(next);
                self.context_usage = Some(next);
                incremental = true;
            }
        }
        if !changed {
            return false;
        }
        if self.activity.len() > MAX_ACTIVITY_ITEMS {
            self.activity
                .drain(0..self.activity.len() - MAX_ACTIVITY_ITEMS);
        }
        if incremental {
            self.publish_incremental()
        } else {
            self.publish_now();
            true
        }
    }

    fn complete(&mut self, message_id: Option<i64>, output_tokens: Option<u64>) {
        if !self.suppress_reply_tail && !self.reply_pending.is_empty() {
            let pending = std::mem::take(&mut self.reply_pending);
            self.append_visible_reply(&pending);
        }
        self.phase = AgentActivityPhase::Complete;
        self.completed_message_id = message_id;
        if output_tokens.is_some() {
            self.generated_output_tokens = output_tokens;
        }
        for activity in &mut self.activity {
            if activity.status == "active" {
                activity.status = "complete".into();
            }
        }
        self.publish_now();
    }

    fn fail(&mut self, message: &str) {
        self.phase = AgentActivityPhase::Failed;
        self.error = Some(bounded(message, 500));
        self.publish_now();
    }

    fn begin_reply_message(&mut self) {
        if !self.suppress_reply_tail && !self.reply_pending.is_empty() {
            let pending = std::mem::take(&mut self.reply_pending);
            self.append_visible_reply(&pending);
        }
        self.reply_pending.clear();
        self.suppress_reply_tail = false;
        if !self.reply.is_empty() && !self.reply.ends_with("\n\n") {
            self.append_visible_reply("\n\n");
        }
    }

    /// Keep progressively streamed metadata markers out of public output.
    fn append_reply_delta(&mut self, text: &str) {
        self.reply_pending.push_str(text);
        if let Some(marker) = HIDDEN_MARKERS
            .iter()
            .filter_map(|marker| self.reply_pending.find(marker))
            .min()
        {
            let visible = self.reply_pending[..marker].to_string();
            self.append_visible_reply(&visible);
            while self.reply.ends_with(['\n', ' ', '\t']) {
                self.reply.pop();
            }
            self.reply_pending.clear();
            self.suppress_reply_tail = true;
            return;
        }

        let held = HIDDEN_MARKERS
            .iter()
            .map(|marker| {
                (1..marker.len())
                    .rev()
                    .find(|length| self.reply_pending.ends_with(&marker[..*length]))
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);
        let ready = self.reply_pending.len().saturating_sub(held);
        if ready > 0 {
            let visible = self.reply_pending[..ready].to_string();
            self.append_visible_reply(&visible);
            self.reply_pending.drain(..ready);
        }
    }

    fn append_visible_reply(&mut self, addition: &str) {
        self.reply.push_str(addition);
        let length = self.reply.chars().count();
        if length <= MAX_REPLY_CHARS {
            return;
        }
        let removed = length - MAX_REPLY_CHARS;
        self.reply = self.reply.chars().skip(removed).collect();
        for activity in &mut self.activity {
            activity.reply_offset = activity.reply_offset.saturating_sub(removed);
        }
    }
}

#[derive(Clone, Default)]
pub struct AgentActivityStreams {
    streams: Arc<Mutex<HashMap<StreamKey, watch::Sender<AgentActivityState>>>>,
}

impl AgentActivityStreams {
    pub fn expect_message(
        &self,
        company: &str,
        actor: &str,
        message_id: i64,
        work_id: Option<Uuid>,
    ) {
        let key = (
            company.to_string(),
            actor.to_string(),
            ActivityScope::Message(message_id),
        );
        if let Ok(mut streams) = self.streams.lock() {
            streams.retain(|(stream_company, stream_actor, scope), sender| {
                stream_company != company
                    || stream_actor != actor
                    || matches!(scope, ActivityScope::Message(id) if *id == message_id)
                    || sender.borrow().phase.is_live()
            });
            streams.entry(key).or_insert_with(|| {
                watch::channel(AgentActivityState::queued(
                    company,
                    actor,
                    Some(message_id),
                    work_id,
                    None,
                ))
                .0
            });
        }
    }

    pub fn start_messages(
        &self,
        company: &str,
        actor: &str,
        message_ids: &[i64],
    ) -> AgentActivityTurn {
        let mut senders = Vec::new();
        if let Ok(mut streams) = self.streams.lock() {
            for message_id in message_ids {
                let key = (
                    company.to_string(),
                    actor.to_string(),
                    ActivityScope::Message(*message_id),
                );
                let sender = streams
                    .entry(key)
                    .or_insert_with(|| {
                        watch::channel(AgentActivityState::queued(
                            company,
                            actor,
                            Some(*message_id),
                            None,
                            None,
                        ))
                        .0
                    })
                    .clone();
                sender.send_modify(AgentActivityState::begin);
                senders.push(sender);
            }
        }
        AgentActivityTurn { senders }
    }

    pub fn start_work(
        &self,
        company: &str,
        actor: &str,
        work_id: Uuid,
        attempt_id: Uuid,
    ) -> AgentActivityTurn {
        let key = (
            company.to_string(),
            actor.to_string(),
            ActivityScope::Work(work_id),
        );
        let sender = if let Ok(mut streams) = self.streams.lock() {
            streams
                .entry(key)
                .or_insert_with(|| {
                    watch::channel(AgentActivityState::queued(
                        company,
                        actor,
                        None,
                        Some(work_id),
                        Some(attempt_id),
                    ))
                    .0
                })
                .clone()
        } else {
            let (sender, _) = watch::channel(AgentActivityState::queued(
                company,
                actor,
                None,
                Some(work_id),
                Some(attempt_id),
            ));
            sender
        };
        sender.send_modify(|state| {
            state.work_id = Some(work_id);
            state.attempt_id = Some(attempt_id);
            state.begin();
        });
        AgentActivityTurn {
            senders: vec![sender],
        }
    }

    pub fn subscribe(
        &self,
        company: &str,
        actor: &str,
        message_id: Option<i64>,
        work_id: Option<Uuid>,
    ) -> watch::Receiver<AgentActivityState> {
        let scope = match (message_id, work_id) {
            (Some(message_id), None) => Some(ActivityScope::Message(message_id)),
            (None, Some(work_id)) => Some(ActivityScope::Work(work_id)),
            (None, None) => None,
            (Some(_), Some(_)) => {
                return watch::channel(AgentActivityState::absent(
                    company, actor, message_id, work_id,
                ))
                .1
            }
        };
        let streams = self.streams.lock().expect("agent activity stream registry");
        let sender = match scope {
            Some(scope) => streams.get(&(company.to_string(), actor.to_string(), scope)),
            None => streams
                .iter()
                .filter(|((stream_company, stream_actor, _), sender)| {
                    stream_company == company
                        && stream_actor == actor
                        && sender.borrow().phase.is_live()
                })
                .max_by_key(|(_, sender)| sender.borrow().updated_at)
                .map(|(_, sender)| sender),
        };
        sender.map(watch::Sender::subscribe).unwrap_or_else(|| {
            watch::channel(AgentActivityState::absent(
                company, actor, message_id, work_id,
            ))
            .1
        })
    }
}

#[derive(Clone)]
pub struct AgentActivityTurn {
    senders: Vec<watch::Sender<AgentActivityState>>,
}

impl AgentActivityTurn {
    pub fn observer(&self) -> SessionObserver {
        let turn = self.clone();
        Arc::new(move |event| turn.apply(event))
    }

    pub fn apply(&self, event: LiveSessionEvent) {
        for sender in &self.senders {
            sender.send_if_modified(|state| state.apply(&event));
        }
    }

    pub fn complete(&self, message_id: Option<i64>, output_tokens: Option<u64>) {
        for sender in &self.senders {
            sender.send_modify(|state| state.complete(message_id, output_tokens));
        }
    }

    pub fn fail(&self, message: &str) {
        for sender in &self.senders {
            sender.send_modify(|state| state.fail(message));
        }
    }
}

fn bounded(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

/// ACP tool titles are model/runtime text, not a trusted owner-facing schema.
/// Give them the same metadata boundary as streamed replies before they enter
/// the activity projection. A marker ends the visible portion rather than
/// asking the browser to understand internal coordination syntax.
fn owner_visible_activity_text(value: &str, max_chars: usize) -> String {
    let visible = HIDDEN_MARKERS
        .iter()
        .filter_map(|marker| value.find(marker))
        .min()
        .map_or(value, |marker| &value[..marker]);
    bounded(visible.trim(), max_chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_is_reconnectable_and_never_leaks_message_metadata() {
        let streams = AgentActivityStreams::default();
        streams.expect_message("company_test", "lead", 42, None);
        let receiver = streams.subscribe("company_test", "lead", Some(42), None);
        let turn = streams.start_messages("company_test", "lead", &[42]);
        turn.apply(LiveSessionEvent::UsageUpdate {
            used: 31_000,
            size: 128_000,
            cost_usd: Some(0.12),
        });
        turn.apply(LiveSessionEvent::ToolStarted {
            id: "reply".into(),
            title: "Prepare visible reply <!--restless-intent:{\"kind\":\"conversation\"}-->"
                .into(),
            kind: "shell<!--restless-details:{\"private\":true}-->".into(),
        });
        turn.apply(LiveSessionEvent::ReplyDelta {
            message_id: Some("reply-1".into()),
            text: "Visible response\n<!--restless-intent:".into(),
        });
        turn.complete(Some(99), Some(137));

        let state = receiver.borrow().clone();
        assert_eq!(state.phase, AgentActivityPhase::Complete);
        assert_eq!(state.reply, "Visible response");
        assert_eq!(state.generated_output_tokens, Some(137));
        assert_eq!(state.context_usage.unwrap().used, 31_000);
        assert_eq!(state.completed_message_id, Some(99));
        assert_eq!(state.activity[0].label, "Prepare visible reply");
        assert_eq!(state.activity[0].detail, "shell");
    }

    #[test]
    fn a_work_attempt_has_the_same_live_projection_as_a_message_turn() {
        let streams = AgentActivityStreams::default();
        let work_id = Uuid::new_v4();
        let attempt_id = Uuid::new_v4();
        let turn = streams.start_work("company_test", "researcher", work_id, attempt_id);
        turn.apply(LiveSessionEvent::ToolStarted {
            id: "probe".into(),
            title: "Probe source".into(),
            kind: "browser".into(),
        });
        let state = streams
            .subscribe("company_test", "researcher", None, Some(work_id))
            .borrow()
            .clone();
        assert_eq!(state.work_id, Some(work_id));
        assert_eq!(state.attempt_id, Some(attempt_id));
        assert_eq!(state.phase, AgentActivityPhase::Acting);

        let absent = streams.subscribe("company_test", "other", None, None);
        assert_eq!(absent.borrow().phase, AgentActivityPhase::Failed);
    }

    #[test]
    fn streaming_reply_snapshots_are_coalesced_without_losing_text() {
        let mut state = AgentActivityState::queued("company_test", "exec", Some(7), None, None);
        state.begin();
        assert!(!state.apply(&LiveSessionEvent::ThoughtDelta));

        assert!(state.apply(&LiveSessionEvent::ReplyDelta {
            message_id: Some("reply-1".into()),
            text: "SSE ".into(),
        }));
        assert!(!state.apply(&LiveSessionEvent::ReplyDelta {
            message_id: Some("reply-1".into()),
            text: "smoke".into(),
        }));
        assert_eq!(state.reply, "SSE smoke");

        state.complete(Some(8), Some(2));
        assert_eq!(state.phase, AgentActivityPhase::Complete);
        assert_eq!(state.reply, "SSE smoke");
    }
}
